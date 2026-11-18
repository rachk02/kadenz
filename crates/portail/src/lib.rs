//! Portail des licences de Kadenz (phase 2, jalon F9 ; spécification du
//! portail) : comptes éditeur et revendeur, entreprises clientes, paiements
//! enregistrés par l'éditeur ou un revendeur, émission des licences
//! d'abonnement avec la clé du portail, journal des actions en ajout seul.
//! Codes de licence courts (décision 74) : un paiement donne un code
//! d'activation ; le programme du client l'envoie avec l'identifiant de son
//! entreprise et reçoit la licence signée (`activer`).
//!
//! Les droits sont contrôlés ici, à chaque fonction (décision 27) : un
//! revendeur ne voit et ne touche que les entreprises qui lui sont
//! rattachées ; une entreprise d'un autre lui est « introuvable ».

mod base;
mod frein;
pub mod pages;
mod sessions;
pub mod web;

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use argon2::Argon2;
use argon2::password_hash::phc::PasswordHash;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use chrono::{Months, NaiveDate, Utc};
use ed25519_dalek::SigningKey;
use kadenz_domaine::module::Module;
use kadenz_licence::code_court;
use kadenz_licence::entreprise::{self as lic, Emetteur, Formule, LicenceEntreprise};
use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest, Sha256};

use frein::Frein;
use sessions::Sessions;

/// Longueur minimale d'un mot de passe du portail.
pub const LONGUEUR_MIN_MOT_DE_PASSE: usize = 10;

/// Horloge du portail (date UTC du jour), remplaçable dans les essais.
pub type Horloge = Arc<dyn Fn() -> NaiveDate + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Editeur,
    Revendeur,
}

impl Role {
    pub fn code(self) -> &'static str {
        match self {
            Role::Editeur => "editeur",
            Role::Revendeur => "revendeur",
        }
    }

    pub fn libelle(self) -> &'static str {
        match self {
            Role::Editeur => "Éditeur",
            Role::Revendeur => "Revendeur",
        }
    }

    fn depuis_code(code: &str) -> Role {
        if code == "editeur" {
            Role::Editeur
        } else {
            Role::Revendeur
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compte {
    pub id: i64,
    pub identifiant: String,
    pub nom: String,
    pub role: Role,
    pub actif: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entreprise {
    /// Identifiant d'entreprise lu dans Kadenz (« Identifiant de
    /// l'entreprise »).
    pub identifiant: String,
    /// Raison sociale du client, reprise dans les licences.
    pub client: String,
    /// Revendeur rattaché (identifiant interne, nom) ; aucun : client direct
    /// de l'éditeur.
    pub revendeur: Option<(i64, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Virement,
    Cheque,
    Especes,
}

impl Mode {
    pub const TOUS: [Mode; 3] = [Mode::Virement, Mode::Cheque, Mode::Especes];

    pub fn code(self) -> &'static str {
        match self {
            Mode::Virement => "virement",
            Mode::Cheque => "cheque",
            Mode::Especes => "especes",
        }
    }

    pub fn libelle(self) -> &'static str {
        match self {
            Mode::Virement => "Virement",
            Mode::Cheque => "Chèque",
            Mode::Especes => "Espèces",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Mode> {
        Mode::TOUS.into_iter().find(|m| m.code() == code)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paiement {
    pub id: i64,
    pub entreprise: String,
    /// Date de réception, AAAA-MM-JJ.
    pub recu_le: String,
    pub mode: Mode,
    /// Montant entier, dans la devise saisie.
    pub montant: i64,
    pub devise: String,
    /// Numéro de chèque ou de virement.
    pub reference: Option<String>,
    pub postes: u32,
    pub formule: Formule,
    /// Nom du compte qui l'a saisi.
    pub saisi_par: String,
    /// Licence émise avec ce paiement, s'il a déjà servi.
    pub licence: Option<i64>,
    /// Code d'activation tiré pour ce paiement, s'il n'a pas été annulé.
    pub code: Option<i64>,
}

impl Paiement {
    /// Un paiement sert une fois : à une licence, ou à un code d'activation.
    pub fn a_servi(&self) -> bool {
        self.licence.is_some() || self.code.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LicenceEmise {
    pub id: i64,
    pub entreprise: String,
    pub paiement: i64,
    /// Texte complet de la licence (`KADENZ-E-...`), à copier dans la
    /// rubrique « Abonnement ».
    pub texte: String,
    pub debut: String,
    pub echeance: String,
    pub postes: u32,
    pub modules: Vec<Module>,
    pub formule: Formule,
    /// « Éditeur », ou le nom du revendeur.
    pub emetteur: String,
    pub emise_le: String,
    /// Nom du compte qui l'a émise.
    pub par: String,
}

/// État d'un code d'activation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtatCode {
    Libre,
    Active,
    Annule,
}

impl EtatCode {
    pub fn code(self) -> &'static str {
        match self {
            EtatCode::Libre => "libre",
            EtatCode::Active => "active",
            EtatCode::Annule => "annule",
        }
    }

    pub fn libelle(self) -> &'static str {
        match self {
            EtatCode::Libre => "à activer",
            EtatCode::Active => "activé",
            EtatCode::Annule => "annulé",
        }
    }

    fn depuis_code(code: &str) -> EtatCode {
        match code {
            "active" => EtatCode::Active,
            "annule" => EtatCode::Annule,
            _ => EtatCode::Libre,
        }
    }
}

/// Code d'activation, sans le code en clair (jamais gardé).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeActivation {
    pub id: i64,
    /// Cinq derniers caractères, pour le reconnaître.
    pub fin: String,
    pub postes: u32,
    pub formule: Formule,
    pub modules: Vec<Module>,
    pub paiement: Option<i64>,
    /// Entreprise liée : d'avance (code tiré pour un paiement) ou à l'activation.
    pub entreprise: Option<String>,
    pub etat: EtatCode,
    /// Nom du compte qui l'a tiré.
    pub cree_par: String,
    pub cree_le: String,
    pub active_le: Option<String>,
    /// Licence émise à l'activation.
    pub licence: Option<i64>,
}

/// Code tout juste tiré : le seul moment où le code en clair existe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeCree {
    /// `XXXXX-XXXXX-XXXXX-XXXXX-XXXXX`, à remettre au client.
    pub code: String,
    pub fiche: CodeActivation,
}

/// Demande d'activation envoyée par le programme du client.
#[derive(Debug, Clone, Default)]
pub struct DemandeActivation {
    pub code: String,
    /// Identifiant de l'entreprise (UUID de la base de Kadenz).
    pub entreprise: String,
    /// Raison sociale, pour une entreprise encore inconnue du portail.
    pub client: String,
    /// Version du programme, notée au journal.
    pub version: String,
}

/// Réponse d'activation : la licence signée et son résumé pour l'écran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Activation {
    pub licence: String,
    pub postes: u32,
    pub formule: Formule,
    pub debut: String,
    pub echeance: String,
}

/// Modèles des refus d'activation, envoyés tels quels au programme du client
/// qui les traduit ; `{}` reçoit les valeurs jointes.
pub const MODELES_ACTIVATION: &[&str] = &[
    "code d'activation inconnu : vérifiez la saisie",
    "ce code d'activation appartient à une autre entreprise",
    "trop d'essais d'activation : réessayez dans {} secondes",
    "activation impossible pour le moment : réessayez plus tard",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    /// Heure UTC, `AAAA-MM-JJ HH:MM:SS`.
    pub heure: String,
    /// Nom du compte (aucun : connexion refusée sans compte connu).
    pub compte: Option<String>,
    /// Code de l'action, par exemple `licence_emise`.
    pub action: String,
    pub cible: Option<String>,
    pub details: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VueEntreprise {
    pub entreprise: Entreprise,
    pub derniere: Option<LicenceEmise>,
    /// Paiements enregistrés qui n'ont pas encore servi à une licence.
    pub paiements_libres: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FicheEntreprise {
    pub entreprise: Entreprise,
    /// Du plus récent au plus ancien.
    pub paiements: Vec<Paiement>,
    /// De la plus récente à la plus ancienne.
    pub licences: Vec<LicenceEmise>,
    /// Codes d'activation liés, du plus récent au plus ancien.
    pub codes: Vec<CodeActivation>,
}

/// Ce que la page d'émission propose de saisir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposition {
    pub modules: Vec<Module>,
    /// Échéance pré-remplie (proposé, spécification du portail, section 5).
    pub echeance: String,
}

/// Message affiché en tête d'une page après une action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub ok: bool,
    pub texte: String,
}

/// Saisie d'un paiement.
#[derive(Debug, Clone, Default)]
pub struct NouveauPaiement {
    pub recu_le: String,
    pub mode: String,
    pub montant: String,
    pub devise: String,
    pub reference: String,
    pub postes: String,
    pub formule: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErreurPortail {
    /// Saisie refusée, message pour la personne.
    Validation(String),
    /// Identifiant ou mot de passe faux, ou compte désactivé.
    Identifiants,
    /// Trop d'échecs de connexion : attente en secondes.
    Freine(u64),
    /// Action réservée à l'éditeur.
    Interdit,
    /// Introuvable, ou hors de la portée du compte.
    Introuvable,
    /// Erreur interne (base, signature).
    Interne(String),
}

impl std::fmt::Display for ErreurPortail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErreurPortail::Validation(m) => write!(f, "{m}"),
            ErreurPortail::Identifiants => write!(f, "identifiant ou mot de passe incorrect"),
            ErreurPortail::Freine(s) => {
                write!(
                    f,
                    "trop d'essais de connexion : réessayez dans {s} secondes"
                )
            }
            ErreurPortail::Interdit => write!(f, "action réservée à l'éditeur"),
            ErreurPortail::Introuvable => write!(f, "introuvable"),
            ErreurPortail::Interne(m) => write!(f, "erreur interne du portail : {m}"),
        }
    }
}

impl std::error::Error for ErreurPortail {}

impl From<rusqlite::Error> for ErreurPortail {
    fn from(e: rusqlite::Error) -> Self {
        ErreurPortail::Interne(e.to_string())
    }
}

pub type Resultat<T> = Result<T, ErreurPortail>;

fn refus<T>(message: &str) -> Resultat<T> {
    Err(ErreurPortail::Validation(message.into()))
}

/// Ordre des verrous, partout : la base d'abord, puis le frein ou les
/// sessions ; jamais la base pendant que le frein ou les sessions sont tenus.
struct Interieur {
    conn: Mutex<Connection>,
    cle: SigningKey,
    horloge: Horloge,
    frein: Mutex<Frein>,
    sessions: Mutex<Sessions>,
}

/// Le portail : base, clé de signature, sessions. Clonable à bas coût.
#[derive(Clone)]
pub struct Portail(Arc<Interieur>);

fn verrou<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn maintenant() -> String {
    Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn hacher(mot_de_passe: &str) -> Resultat<String> {
    Argon2::default()
        .hash_password(mot_de_passe.as_bytes())
        .map(|h| h.to_string())
        .map_err(|e| ErreurPortail::Interne(e.to_string()))
}

fn mot_de_passe_correct(mot_de_passe: &str, hache: &str) -> bool {
    PasswordHash::new(hache)
        .map(|h| {
            Argon2::default()
                .verify_password(mot_de_passe.as_bytes(), &h)
                .is_ok()
        })
        .unwrap_or(false)
}

fn valider_mot_de_passe(mot_de_passe: &str) -> Resultat<()> {
    if mot_de_passe.chars().count() < LONGUEUR_MIN_MOT_DE_PASSE {
        return Err(ErreurPortail::Validation(format!(
            "mot de passe trop court : {LONGUEUR_MIN_MOT_DE_PASSE} caractères au moins"
        )));
    }
    Ok(())
}

fn valider_identifiant(identifiant: &str) -> Resultat<String> {
    let identifiant = identifiant.trim();
    let permis = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_');
    if identifiant.is_empty() || identifiant.len() > 64 || !identifiant.chars().all(permis) {
        return refus(
            "identifiant invalide : lettres, chiffres, point, tiret ou souligné, 64 caractères au plus",
        );
    }
    Ok(identifiant.to_string())
}

fn valider_texte(texte: &str, champ: &str, max: usize) -> Resultat<String> {
    let texte = texte.trim();
    if texte.is_empty() {
        return Err(ErreurPortail::Validation(format!("{champ} manquant")));
    }
    if texte.chars().count() > max || texte.chars().any(char::is_control) {
        return Err(ErreurPortail::Validation(format!(
            "{champ} invalide : {max} caractères au plus, sans caractère de contrôle"
        )));
    }
    Ok(texte.to_string())
}

/// Identifiant d'entreprise : un UUID, comme celui tiré à la création de la
/// base de Kadenz.
fn valider_uuid(identifiant: &str) -> Resultat<String> {
    let id = identifiant.trim().to_ascii_lowercase();
    let forme = id.len() == 36
        && id.char_indices().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => c == '-',
            _ => c.is_ascii_hexdigit(),
        });
    if !forme {
        return refus(
            "identifiant d'entreprise invalide : copiez celui affiché par Kadenz (« Identifiant de l'entreprise »)",
        );
    }
    Ok(id)
}

fn date(texte: &str, champ: &str) -> Resultat<NaiveDate> {
    NaiveDate::parse_from_str(texte.trim(), "%Y-%m-%d")
        .map_err(|_| ErreurPortail::Validation(format!("{champ} illisible : AAAA-MM-JJ attendu")))
}

fn ajouter_periode(jour: NaiveDate, formule: Formule) -> NaiveDate {
    let mois = match formule {
        Formule::Mensuelle => 1,
        Formule::Annuelle => 12,
    };
    jour.checked_add_months(Months::new(mois)).unwrap_or(jour)
}

/// Montant entier avec une espace fine insécable entre les milliers, pour
/// le journal (150000 : « 150 000 »).
fn milliers(montant: i64) -> String {
    let chiffres = montant.unsigned_abs().to_string();
    let mut texte = String::new();
    for (i, c) in chiffres.chars().enumerate() {
        if i > 0 && (chiffres.len() - i).is_multiple_of(3) {
            texte.push('\u{202f}');
        }
        texte.push(c);
    }
    if montant < 0 {
        texte.insert(0, '-');
    }
    texte
}

fn modules_en_texte(modules: &[Module]) -> String {
    modules
        .iter()
        .map(|m| m.code())
        .collect::<Vec<_>>()
        .join(",")
}

fn modules_depuis_texte(texte: &str) -> Vec<Module> {
    texte.split(',').filter_map(Module::depuis_code).collect()
}

/// Identifiant public d'un revendeur dans les licences (décision 29).
fn identifiant_public(revendeur: i64) -> String {
    format!("revendeur-{revendeur}")
}

fn compte_depuis(r: &rusqlite::Row) -> rusqlite::Result<Compte> {
    Ok(Compte {
        id: r.get(0)?,
        identifiant: r.get(1)?,
        nom: r.get(2)?,
        role: Role::depuis_code(&r.get::<_, String>(3)?),
        actif: r.get(4)?,
    })
}

const COLONNES_COMPTE: &str = "id, identifiant, nom, role, actif";

const SELECT_ENTREPRISE: &str = "SELECT e.identifiant, e.client, e.revendeur, c.nom
     FROM entreprise e LEFT JOIN compte c ON c.id = e.revendeur";

fn entreprise_depuis(r: &rusqlite::Row) -> rusqlite::Result<Entreprise> {
    let revendeur: Option<i64> = r.get(2)?;
    let nom: Option<String> = r.get(3)?;
    Ok(Entreprise {
        identifiant: r.get(0)?,
        client: r.get(1)?,
        revendeur: revendeur.map(|id| (id, nom.unwrap_or_default())),
    })
}

const SELECT_PAIEMENT: &str = "SELECT p.id, p.entreprise, p.recu_le, p.mode, p.montant, p.devise,
            p.reference, p.postes, p.formule, c.nom, l.id,
            (SELECT k.id FROM code_activation k
              WHERE k.paiement = p.id AND k.etat <> 'annule' ORDER BY k.id DESC LIMIT 1)
     FROM paiement p JOIN compte c ON c.id = p.saisi_par
     LEFT JOIN licence_emise l ON l.paiement = p.id";

fn paiement_depuis(r: &rusqlite::Row) -> rusqlite::Result<Paiement> {
    Ok(Paiement {
        id: r.get(0)?,
        entreprise: r.get(1)?,
        recu_le: r.get(2)?,
        mode: Mode::depuis_code(&r.get::<_, String>(3)?).unwrap_or(Mode::Virement),
        montant: r.get(4)?,
        devise: r.get(5)?,
        reference: r.get(6)?,
        postes: r.get(7)?,
        formule: Formule::depuis_code(&r.get::<_, String>(8)?).unwrap_or(Formule::Annuelle),
        saisi_par: r.get(9)?,
        licence: r.get(10)?,
        code: r.get(11)?,
    })
}

const SELECT_CODE: &str = "SELECT k.id, k.fin, k.postes, k.formule, k.modules, k.paiement,
            k.entreprise, k.etat, c.nom, k.cree_le, k.active_le, l.id, k.cree_par
     FROM code_activation k JOIN compte c ON c.id = k.cree_par
     LEFT JOIN licence_emise l ON l.code = k.id";

fn code_depuis(r: &rusqlite::Row) -> rusqlite::Result<CodeActivation> {
    Ok(CodeActivation {
        id: r.get(0)?,
        fin: r.get(1)?,
        postes: r.get(2)?,
        formule: Formule::depuis_code(&r.get::<_, String>(3)?).unwrap_or(Formule::Annuelle),
        modules: modules_depuis_texte(&r.get::<_, String>(4)?),
        paiement: r.get(5)?,
        entreprise: r.get(6)?,
        etat: EtatCode::depuis_code(&r.get::<_, String>(7)?),
        cree_par: r.get(8)?,
        cree_le: r.get(9)?,
        active_le: r.get(10)?,
        licence: r.get(11)?,
    })
}

/// Empreinte d'un code normalisé (25 caractères, sans tirets).
fn empreinte_code(utiles: &str) -> String {
    Sha256::digest(utiles.as_bytes())
        .iter()
        .map(|o| format!("{o:02x}"))
        .collect()
}

/// Émetteur porté par la licence, et son libellé pour les listes.
fn emetteur_de(compte: &Compte) -> (Emetteur, String) {
    match compte.role {
        Role::Editeur => (Emetteur::Editeur, "Éditeur".to_string()),
        Role::Revendeur => (
            Emetteur::Revendeur {
                identifiant: identifiant_public(compte.id),
                nom: compte.nom.clone(),
            },
            compte.nom.clone(),
        ),
    }
}

/// Échéance d'une nouvelle licence (proposé, spécification du portail,
/// section 5) : l'échéance de la dernière licence si elle court encore,
/// sinon la veille, plus la période de la formule.
fn echeance_proposee(
    conn: &Connection,
    entreprise: &str,
    formule: Formule,
    aujourdhui: NaiveDate,
) -> Resultat<NaiveDate> {
    let veille = aujourdhui.pred_opt().unwrap_or(aujourdhui);
    let depart = derniere_licence(conn, entreprise)?
        .and_then(|l| NaiveDate::parse_from_str(&l.echeance, "%Y-%m-%d").ok())
        .filter(|e| *e >= aujourdhui)
        .unwrap_or(veille);
    Ok(ajouter_periode(depart, formule))
}

/// Ce qu'une licence à émettre reprend de son paiement ou de son code.
struct ALicence<'a> {
    entreprise: &'a Entreprise,
    paiement: i64,
    code: Option<i64>,
    formule: Formule,
    postes: u32,
    modules: &'a [Module],
    echeance: NaiveDate,
    /// Compte émetteur : l'opérateur, ou celui qui a tiré le code.
    emetteur: &'a Compte,
}

/// Signe la licence avec la clé du portail, la vérifie aussitôt et
/// l'enregistre ; début et émission : la date du jour.
fn signer_et_enregistrer(
    conn: &Connection,
    cle: &SigningKey,
    aujourdhui: NaiveDate,
    a: ALicence<'_>,
) -> Resultat<(i64, LicenceEntreprise, String)> {
    let modules: Vec<Module> = Module::TOUS
        .into_iter()
        .filter(|m| a.modules.contains(m))
        .collect();
    let (emetteur, libelle_emetteur) = emetteur_de(a.emetteur);
    let jour = aujourdhui.format("%Y-%m-%d").to_string();
    let licence = LicenceEntreprise {
        entreprise: a.entreprise.identifiant.clone(),
        client: a.entreprise.client.clone(),
        formule: a.formule,
        postes: a.postes,
        modules: modules.clone(),
        debut: jour.clone(),
        echeance: a.echeance.format("%Y-%m-%d").to_string(),
        emetteur,
        emise_le: jour,
    };
    let texte =
        lic::emettre(&licence, cle).map_err(|e| ErreurPortail::Interne(e.to_string()))?;
    lic::verifier(&texte, &a.entreprise.identifiant, &[cle.verifying_key()])
        .map_err(|e| ErreurPortail::Interne(format!("licence émise illisible : {e}")))?;
    conn.execute(
        "INSERT INTO licence_emise (entreprise, paiement, texte, debut, echeance, postes,
                                    modules, formule, emetteur, emise_le, par, code)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            a.entreprise.identifiant,
            a.paiement,
            texte,
            licence.debut,
            licence.echeance,
            licence.postes,
            modules_en_texte(&modules),
            licence.formule.code(),
            libelle_emetteur,
            licence.emise_le,
            a.emetteur.id,
            a.code,
        ],
    )?;
    Ok((conn.last_insert_rowid(), licence, texte))
}

fn liste_modules(modules: &[Module]) -> String {
    if modules.is_empty() {
        "aucun".to_string()
    } else {
        modules.iter().map(|m| m.libelle()).collect::<Vec<_>>().join(", ")
    }
}

const SELECT_LICENCE: &str = "SELECT l.id, l.entreprise, l.paiement, l.texte, l.debut, l.echeance,
            l.postes, l.modules, l.formule, l.emetteur, l.emise_le, c.nom
     FROM licence_emise l JOIN compte c ON c.id = l.par";

fn licence_depuis(r: &rusqlite::Row) -> rusqlite::Result<LicenceEmise> {
    Ok(LicenceEmise {
        id: r.get(0)?,
        entreprise: r.get(1)?,
        paiement: r.get(2)?,
        texte: r.get(3)?,
        debut: r.get(4)?,
        echeance: r.get(5)?,
        postes: r.get(6)?,
        modules: modules_depuis_texte(&r.get::<_, String>(7)?),
        formule: Formule::depuis_code(&r.get::<_, String>(8)?).unwrap_or(Formule::Annuelle),
        emetteur: r.get(9)?,
        emise_le: r.get(10)?,
        par: r.get(11)?,
    })
}

fn noter(
    conn: &Connection,
    compte: Option<i64>,
    action: &str,
    cible: Option<&str>,
    details: &str,
) -> Resultat<()> {
    conn.execute(
        "INSERT INTO journal (heure, compte, action, cible, details) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![maintenant(), compte, action, cible, details],
    )?;
    Ok(())
}

/// Compte connu et actif, relu de la base (un compte désactivé perd ses
/// droits tout de suite).
fn relire(conn: &Connection, acteur: &Compte) -> Resultat<Compte> {
    conn.query_row(
        &format!("SELECT {COLONNES_COMPTE} FROM compte WHERE id = ?1"),
        [acteur.id],
        compte_depuis,
    )
    .optional()?
    .filter(|c| c.actif)
    .ok_or(ErreurPortail::Interdit)
}

fn exiger_editeur(conn: &Connection, acteur: &Compte) -> Resultat<Compte> {
    let compte = relire(conn, acteur)?;
    if compte.role != Role::Editeur {
        return Err(ErreurPortail::Interdit);
    }
    Ok(compte)
}

/// Entreprise dans la portée du compte : toutes pour l'éditeur, les siennes
/// pour un revendeur ; sinon « introuvable ».
fn entreprise_permise(
    conn: &Connection,
    compte: &Compte,
    identifiant: &str,
) -> Resultat<Entreprise> {
    let entreprise = conn
        .query_row(
            &format!("{SELECT_ENTREPRISE} WHERE e.identifiant = ?1"),
            [identifiant.trim().to_ascii_lowercase()],
            entreprise_depuis,
        )
        .optional()?
        .ok_or(ErreurPortail::Introuvable)?;
    match compte.role {
        Role::Editeur => Ok(entreprise),
        Role::Revendeur if entreprise.revendeur.as_ref().map(|r| r.0) == Some(compte.id) => {
            Ok(entreprise)
        }
        Role::Revendeur => Err(ErreurPortail::Introuvable),
    }
}

fn derniere_licence(conn: &Connection, entreprise: &str) -> Resultat<Option<LicenceEmise>> {
    Ok(conn
        .query_row(
            &format!("{SELECT_LICENCE} WHERE l.entreprise = ?1 ORDER BY l.id DESC LIMIT 1"),
            [entreprise],
            licence_depuis,
        )
        .optional()?)
}

impl Portail {
    /// Ouvre le portail sur son dossier, avec la clé de signature du portail.
    pub fn ouvrir(dossier: &Path, cle: SigningKey) -> Resultat<Portail> {
        Portail::ouvrir_avec(dossier, cle, Arc::new(|| Utc::now().date_naive()))
    }

    /// Comme [`Portail::ouvrir`], avec une horloge choisie (essais).
    pub fn ouvrir_avec(dossier: &Path, cle: SigningKey, horloge: Horloge) -> Resultat<Portail> {
        let conn = base::ouvrir(dossier)?;
        Ok(Portail(Arc::new(Interieur {
            conn: Mutex::new(conn),
            cle,
            horloge,
            frein: Mutex::new(Frein::default()),
            sessions: Mutex::new(Sessions::default()),
        })))
    }

    fn conn(&self) -> MutexGuard<'_, Connection> {
        verrou(&self.0.conn)
    }

    fn aujourdhui(&self) -> NaiveDate {
        (self.0.horloge)()
    }

    /// Clé publique du portail, qui vérifie les licences émises.
    pub fn cle_publique(&self) -> ed25519_dalek::VerifyingKey {
        self.0.cle.verifying_key()
    }

    // ----- Comptes -----

    /// Crée un compte éditeur (ligne de commande seulement, jamais par une
    /// page).
    pub fn creer_editeur(
        &self,
        identifiant: &str,
        nom: &str,
        mot_de_passe: &str,
    ) -> Resultat<Compte> {
        let conn = self.conn();
        let compte = creer_compte(&conn, Role::Editeur, identifiant, nom, mot_de_passe)?;
        noter(
            &conn,
            Some(compte.id),
            "editeur_cree",
            Some(&compte.identifiant),
            "",
        )?;
        Ok(compte)
    }

    /// Crée un compte revendeur, par l'éditeur.
    pub fn creer_revendeur(
        &self,
        acteur: &Compte,
        identifiant: &str,
        nom: &str,
        mot_de_passe: &str,
    ) -> Resultat<Compte> {
        let conn = self.conn();
        let acteur = exiger_editeur(&conn, acteur)?;
        let compte = creer_compte(&conn, Role::Revendeur, identifiant, nom, mot_de_passe)?;
        noter(
            &conn,
            Some(acteur.id),
            "compte_cree",
            Some(&compte.identifiant),
            &compte.nom,
        )?;
        Ok(compte)
    }

    /// Tous les comptes, pour l'éditeur.
    pub fn comptes(&self, acteur: &Compte) -> Resultat<Vec<Compte>> {
        let conn = self.conn();
        exiger_editeur(&conn, acteur)?;
        let mut req = conn.prepare(&format!(
            "SELECT {COLONNES_COMPTE} FROM compte ORDER BY role, nom COLLATE NOCASE"
        ))?;
        let comptes = req
            .query_map([], compte_depuis)?
            .collect::<Result<_, _>>()?;
        Ok(comptes)
    }

    /// Revendeurs actifs, pour rattacher une entreprise (éditeur seulement).
    pub fn revendeurs(&self, acteur: &Compte) -> Resultat<Vec<Compte>> {
        Ok(self
            .comptes(acteur)?
            .into_iter()
            .filter(|c| c.role == Role::Revendeur && c.actif)
            .collect())
    }

    /// Active ou désactive un compte revendeur ; ses sessions finissent.
    pub fn regler_actif(&self, acteur: &Compte, compte: i64, actif: bool) -> Resultat<Compte> {
        let conn = self.conn();
        let acteur = exiger_editeur(&conn, acteur)?;
        let cible = conn
            .query_row(
                &format!("SELECT {COLONNES_COMPTE} FROM compte WHERE id = ?1"),
                [compte],
                compte_depuis,
            )
            .optional()?
            .ok_or(ErreurPortail::Introuvable)?;
        if cible.role == Role::Editeur {
            return refus("un compte éditeur ne se désactive pas depuis le portail");
        }
        conn.execute(
            "UPDATE compte SET actif = ?1 WHERE id = ?2",
            params![actif, compte],
        )?;
        let action = if actif {
            "compte_active"
        } else {
            "compte_desactive"
        };
        noter(&conn, Some(acteur.id), action, Some(&cible.identifiant), "")?;
        if !actif {
            verrou(&self.0.sessions).fermer_compte(compte);
        }
        Ok(Compte { actif, ..cible })
    }

    /// Change le mot de passe du compte connecté.
    pub fn changer_mot_de_passe(
        &self,
        acteur: &Compte,
        ancien: &str,
        nouveau: &str,
        confirmation: &str,
    ) -> Resultat<()> {
        let conn = self.conn();
        let acteur = relire(&conn, acteur)?;
        let hache: String = conn.query_row(
            "SELECT mot_de_passe FROM compte WHERE id = ?1",
            [acteur.id],
            |r| r.get(0),
        )?;
        if !mot_de_passe_correct(ancien, &hache) {
            return refus("mot de passe actuel incorrect");
        }
        if nouveau != confirmation {
            return refus("les deux nouveaux mots de passe diffèrent");
        }
        valider_mot_de_passe(nouveau)?;
        conn.execute(
            "UPDATE compte SET mot_de_passe = ?1 WHERE id = ?2",
            params![hacher(nouveau)?, acteur.id],
        )?;
        noter(
            &conn,
            Some(acteur.id),
            "mot_de_passe_change",
            Some(&acteur.identifiant),
            "",
        )?;
        Ok(())
    }

    // ----- Connexion et sessions -----

    /// Connexion : renvoie le jeton de session et le compte. Freinée par
    /// identifiant et par adresse ; chaque refus est noté au journal.
    pub fn connecter(
        &self,
        identifiant: &str,
        mot_de_passe: &str,
        adresse: &str,
    ) -> Resultat<(String, Compte)> {
        let identifiant = identifiant.trim().to_ascii_lowercase();
        let cles = vec![format!("id:{identifiant}"), format!("ip:{adresse}")];
        if let Some(attente) = verrou(&self.0.frein).attente(&cles) {
            return Err(ErreurPortail::Freine(attente));
        }
        let conn = self.conn();
        let trouve = conn
            .query_row(
                &format!(
                    "SELECT {COLONNES_COMPTE}, mot_de_passe FROM compte WHERE identifiant = ?1"
                ),
                [&identifiant],
                |r| Ok((compte_depuis(r)?, r.get::<_, String>(5)?)),
            )
            .optional()?;
        match trouve {
            Some((compte, hache)) if compte.actif && mot_de_passe_correct(mot_de_passe, &hache) => {
                verrou(&self.0.frein).reussite(&cles);
                noter(&conn, Some(compte.id), "connexion", None, adresse)?;
                let jeton = verrou(&self.0.sessions).ouvrir(compte.id);
                Ok((jeton, compte))
            }
            trouve => {
                verrou(&self.0.frein).echec(&cles);
                let details = format!("identifiant {identifiant}, adresse {adresse}");
                noter(
                    &conn,
                    trouve.map(|t| t.0.id),
                    "connexion_refusee",
                    None,
                    &details,
                )?;
                Err(ErreurPortail::Identifiants)
            }
        }
    }

    /// Session ouverte pour ce jeton : compte actif et jeton anti-falsification
    /// de la session. Prolonge la session.
    pub fn session(&self, jeton: &str) -> Option<(Compte, String)> {
        let (compte, csrf) = verrou(&self.0.sessions).toucher(jeton)?;
        let conn = self.conn();
        match conn
            .query_row(
                &format!("SELECT {COLONNES_COMPTE} FROM compte WHERE id = ?1"),
                [compte],
                compte_depuis,
            )
            .optional()
        {
            Ok(Some(c)) if c.actif => Some((c, csrf)),
            _ => {
                verrou(&self.0.sessions).fermer(jeton);
                None
            }
        }
    }

    pub fn deconnecter(&self, jeton: &str) {
        // Verrou des sessions relâché avant de prendre la base (ordre des
        // verrous : base, puis frein ou sessions).
        let fermee = verrou(&self.0.sessions).fermer(jeton);
        if let Some((compte, _)) = fermee {
            let _ = noter(&self.conn(), Some(compte), "deconnexion", None, "");
        }
    }

    // ----- Entreprises -----

    /// Enregistre une entreprise cliente. Un revendeur la rattache toujours à
    /// lui ; l'éditeur choisit le revendeur, ou aucun.
    pub fn creer_entreprise(
        &self,
        acteur: &Compte,
        identifiant: &str,
        client: &str,
        revendeur: Option<i64>,
    ) -> Resultat<Entreprise> {
        let conn = self.conn();
        let acteur = relire(&conn, acteur)?;
        let identifiant = valider_uuid(identifiant)?;
        let client = valider_texte(client, "nom du client", 200)?;
        let revendeur = match acteur.role {
            Role::Revendeur => Some(acteur.id),
            Role::Editeur => match revendeur {
                None => None,
                Some(id) => {
                    let est_revendeur: bool = conn
                        .query_row(
                            "SELECT role = 'revendeur' AND actif = 1 FROM compte WHERE id = ?1",
                            [id],
                            |r| r.get(0),
                        )
                        .optional()?
                        .unwrap_or(false);
                    if !est_revendeur {
                        return refus("revendeur inconnu ou désactivé");
                    }
                    Some(id)
                }
            },
        };
        let existe: bool = conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM entreprise WHERE identifiant = ?1)",
            [&identifiant],
            |r| r.get(0),
        )?;
        if existe {
            return refus("cet identifiant d'entreprise est déjà enregistré sur le portail");
        }
        conn.execute(
            "INSERT INTO entreprise (identifiant, client, revendeur, cree_le) VALUES (?1, ?2, ?3, ?4)",
            params![identifiant, client, revendeur, maintenant()],
        )?;
        noter(
            &conn,
            Some(acteur.id),
            "entreprise_creee",
            Some(&identifiant),
            &client,
        )?;
        entreprise_permise(&conn, &acteur, &identifiant)
    }

    /// Entreprises dans la portée du compte, avec leur dernière licence.
    pub fn entreprises(&self, acteur: &Compte) -> Resultat<Vec<VueEntreprise>> {
        let conn = self.conn();
        let acteur = relire(&conn, acteur)?;
        let filtre = match acteur.role {
            Role::Editeur => String::new(),
            Role::Revendeur => format!(" WHERE e.revendeur = {}", acteur.id),
        };
        let mut req = conn.prepare(&format!(
            "{SELECT_ENTREPRISE}{filtre} ORDER BY e.client COLLATE NOCASE"
        ))?;
        let entreprises: Vec<Entreprise> = req
            .query_map([], entreprise_depuis)?
            .collect::<Result<_, _>>()?;
        entreprises
            .into_iter()
            .map(|entreprise| {
                let derniere = derniere_licence(&conn, &entreprise.identifiant)?;
                let paiements_libres: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM paiement p
                     WHERE p.entreprise = ?1
                       AND NOT EXISTS (SELECT 1 FROM licence_emise l WHERE l.paiement = p.id)
                       AND NOT EXISTS (SELECT 1 FROM code_activation k
                                        WHERE k.paiement = p.id AND k.etat <> 'annule')",
                    [&entreprise.identifiant],
                    |r| r.get(0),
                )?;
                Ok(VueEntreprise {
                    entreprise,
                    derniere,
                    paiements_libres: paiements_libres as usize,
                })
            })
            .collect()
    }

    /// Fiche d'une entreprise : paiements et licences.
    pub fn fiche(&self, acteur: &Compte, identifiant: &str) -> Resultat<FicheEntreprise> {
        let conn = self.conn();
        let acteur = relire(&conn, acteur)?;
        let entreprise = entreprise_permise(&conn, &acteur, identifiant)?;
        let mut req = conn.prepare(&format!(
            "{SELECT_PAIEMENT} WHERE p.entreprise = ?1 ORDER BY p.id DESC"
        ))?;
        let paiements = req
            .query_map([&entreprise.identifiant], paiement_depuis)?
            .collect::<Result<_, _>>()?;
        let mut req = conn.prepare(&format!(
            "{SELECT_LICENCE} WHERE l.entreprise = ?1 ORDER BY l.id DESC"
        ))?;
        let licences = req
            .query_map([&entreprise.identifiant], licence_depuis)?
            .collect::<Result<_, _>>()?;
        let mut req = conn.prepare(&format!(
            "{SELECT_CODE} WHERE k.entreprise = ?1 ORDER BY k.id DESC"
        ))?;
        let codes = req
            .query_map([&entreprise.identifiant], code_depuis)?
            .collect::<Result<_, _>>()?;
        Ok(FicheEntreprise {
            entreprise,
            paiements,
            licences,
            codes,
        })
    }

    /// Entreprise seule, dans la portée du compte.
    pub fn entreprise(&self, acteur: &Compte, identifiant: &str) -> Resultat<Entreprise> {
        let conn = self.conn();
        let acteur = relire(&conn, acteur)?;
        entreprise_permise(&conn, &acteur, identifiant)
    }

    // ----- Paiements -----

    /// Enregistre un paiement reçu (virement, chèque ou espèces ; décision 60).
    pub fn enregistrer_paiement(
        &self,
        acteur: &Compte,
        entreprise: &str,
        saisie: &NouveauPaiement,
    ) -> Resultat<Paiement> {
        let conn = self.conn();
        let acteur = relire(&conn, acteur)?;
        let entreprise = entreprise_permise(&conn, &acteur, entreprise)?;
        let recu_le = date(&saisie.recu_le, "date de réception")?;
        if recu_le > self.aujourdhui() {
            return refus("date de réception dans le futur");
        }
        let mode = Mode::depuis_code(saisie.mode.trim()).ok_or_else(|| {
            ErreurPortail::Validation(
                "mode de paiement invalide : virement, chèque ou espèces".into(),
            )
        })?;
        let montant: i64 = saisie
            .montant
            .trim()
            .replace([' ', '\u{a0}', '\u{202f}'], "")
            .parse()
            .ok()
            .filter(|m| *m > 0)
            .ok_or_else(|| {
                ErreurPortail::Validation("montant invalide : nombre entier positif".into())
            })?;
        let devise = saisie.devise.trim().to_ascii_uppercase();
        if devise.len() != 3 || !devise.chars().all(|c| c.is_ascii_uppercase()) {
            return refus("devise invalide : code de trois lettres (par exemple XOF)");
        }
        let reference = match saisie.reference.trim() {
            "" => None,
            r => Some(valider_texte(r, "référence", 100)?),
        };
        let postes: u32 = saisie
            .postes
            .trim()
            .parse()
            .ok()
            .filter(|p| (1..=1000).contains(p))
            .ok_or_else(|| {
                ErreurPortail::Validation("nombre de postes invalide : de 1 à 1000".into())
            })?;
        let formule = Formule::depuis_code(saisie.formule.trim()).ok_or_else(|| {
            ErreurPortail::Validation("formule invalide : mensuelle ou annuelle".into())
        })?;
        conn.execute(
            "INSERT INTO paiement (entreprise, recu_le, mode, montant, devise, reference, postes,
                                   formule, saisi_par, saisi_le)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                entreprise.identifiant,
                recu_le.format("%Y-%m-%d").to_string(),
                mode.code(),
                montant,
                devise,
                reference,
                postes,
                formule.code(),
                acteur.id,
                maintenant(),
            ],
        )?;
        let id = conn.last_insert_rowid();
        let details = format!(
            "{} {} {devise}, {postes} postes, {}{}",
            mode.libelle(),
            milliers(montant),
            formule.code(),
            reference
                .map(|r| format!(", référence {r}"))
                .unwrap_or_default()
        );
        noter(
            &conn,
            Some(acteur.id),
            "paiement_enregistre",
            Some(&entreprise.identifiant),
            &details,
        )?;
        paiement_permis(&conn, &acteur, id)
    }

    /// Paiement dans la portée du compte.
    pub fn paiement(&self, acteur: &Compte, id: i64) -> Resultat<Paiement> {
        let conn = self.conn();
        let acteur = relire(&conn, acteur)?;
        paiement_permis(&conn, &acteur, id)
    }

    // ----- Émission -----

    /// Ce que la page d'émission pré-remplit pour ce paiement : modules de la
    /// dernière licence (tous pour une première licence) ; échéance de la
    /// dernière licence, ou veille du jour si elle est passée ou absente,
    /// plus la période de la formule (proposé).
    pub fn proposition(&self, acteur: &Compte, paiement: i64) -> Resultat<Proposition> {
        let conn = self.conn();
        let acteur = relire(&conn, acteur)?;
        let paiement = paiement_permis(&conn, &acteur, paiement)?;
        let echeance =
            echeance_proposee(&conn, &paiement.entreprise, paiement.formule, self.aujourdhui())?;
        Ok(Proposition {
            modules: derniere_licence(&conn, &paiement.entreprise)?
                .map(|l| l.modules)
                .unwrap_or_else(|| Module::TOUS.to_vec()),
            echeance: echeance.format("%Y-%m-%d").to_string(),
        })
    }

    /// Émet la licence d'un paiement non encore utilisé, signée par la clé du
    /// portail et vérifiée aussitôt.
    pub fn emettre(
        &self,
        acteur: &Compte,
        paiement: i64,
        modules: &[Module],
        echeance: &str,
    ) -> Resultat<LicenceEmise> {
        let conn = self.conn();
        let acteur = relire(&conn, acteur)?;
        let paiement = paiement_permis(&conn, &acteur, paiement)?;
        if paiement.a_servi() {
            return refus("ce paiement a déjà servi à une licence");
        }
        let entreprise = entreprise_permise(&conn, &acteur, &paiement.entreprise)?;
        let aujourdhui = self.aujourdhui();
        let echeance = date(echeance, "échéance")?;
        if echeance < aujourdhui {
            return refus("échéance déjà passée");
        }
        let (id, licence, _) = signer_et_enregistrer(
            &conn,
            &self.0.cle,
            aujourdhui,
            ALicence {
                entreprise: &entreprise,
                paiement: paiement.id,
                code: None,
                formule: paiement.formule,
                postes: paiement.postes,
                modules,
                echeance,
                emetteur: &acteur,
            },
        )?;
        let details = format!(
            "paiement {}, {} postes, échéance {}, modules : {}",
            paiement.id,
            licence.postes,
            licence.echeance,
            liste_modules(&licence.modules)
        );
        noter(
            &conn,
            Some(acteur.id),
            "licence_emise",
            Some(&entreprise.identifiant),
            &details,
        )?;
        licence_permise(&conn, &acteur, id)
    }

    // ----- Codes d'activation (décision 74) -----

    /// Tire le code d'activation d'un paiement non encore utilisé : postes et
    /// formule du paiement, modules choisis, code lié d'avance à l'entreprise
    /// du paiement. L'échéance est calculée à l'activation. Le code en clair
    /// n'est rendu qu'ici.
    pub fn creer_code(
        &self,
        acteur: &Compte,
        paiement: i64,
        modules: &[Module],
    ) -> Resultat<CodeCree> {
        let conn = self.conn();
        let acteur = relire(&conn, acteur)?;
        let paiement = paiement_permis(&conn, &acteur, paiement)?;
        if paiement.a_servi() {
            return refus("ce paiement a déjà servi à une licence");
        }
        let modules: Vec<Module> = Module::TOUS
            .into_iter()
            .filter(|m| modules.contains(m))
            .collect();
        let code = code_court::tirer().map_err(ErreurPortail::Interne)?;
        let utiles = code_court::normaliser(&code);
        let fin: String = utiles.chars().skip(code_court::LONGUEUR - 5).collect();
        conn.execute(
            "INSERT INTO code_activation (empreinte, fin, postes, formule, modules, paiement,
                                          entreprise, cree_par, cree_le)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                empreinte_code(&utiles),
                fin,
                paiement.postes,
                paiement.formule.code(),
                modules_en_texte(&modules),
                paiement.id,
                paiement.entreprise,
                acteur.id,
                maintenant(),
            ],
        )?;
        let id = conn.last_insert_rowid();
        let details = format!(
            "code …{fin}, paiement {}, {} postes, formule {}, modules : {}",
            paiement.id,
            paiement.postes,
            paiement.formule.code(),
            liste_modules(&modules)
        );
        noter(&conn, Some(acteur.id), "code_cree", Some(&paiement.entreprise), &details)?;
        Ok(CodeCree {
            code,
            fiche: code_permis(&conn, &acteur, id)?,
        })
    }

    /// Code d'activation, dans la portée du compte.
    pub fn code(&self, acteur: &Compte, id: i64) -> Resultat<CodeActivation> {
        let conn = self.conn();
        let acteur = relire(&conn, acteur)?;
        code_permis(&conn, &acteur, id)
    }

    /// Activation demandée par le programme du client, sans compte connecté
    /// (route `/api/activation`). Code libre : la licence est émise, signée et
    /// le code passe à « activé » ; code déjà activé par la même entreprise :
    /// la même licence est rendue. Les refus sont freinés comme les connexions.
    pub fn activer(&self, demande: &DemandeActivation, adresse: &str) -> Resultat<Activation> {
        let entreprise_demandee = demande.entreprise.trim().to_ascii_lowercase();
        let cles = vec![
            format!("activation:ip:{adresse}"),
            format!("activation:ent:{entreprise_demandee}"),
        ];
        let conn = self.conn();
        if let Some(attente) = verrou(&self.0.frein).attente(&cles) {
            return Err(ErreurPortail::Freine(attente));
        }
        let resultat = self.activer_sans_frein(&conn, demande, adresse);
        match &resultat {
            Ok(_) => verrou(&self.0.frein).reussite(&cles),
            Err(ErreurPortail::Validation(modele)) => {
                verrou(&self.0.frein).echec(&cles);
                let details = format!(
                    "entreprise {entreprise_demandee}, adresse {adresse}, version {} : {modele}",
                    demande.version.trim()
                );
                noter(&conn, None, "activation_refusee", None, &details)?;
            }
            Err(_) => {}
        }
        resultat
    }

    fn activer_sans_frein(
        &self,
        conn: &Connection,
        demande: &DemandeActivation,
        adresse: &str,
    ) -> Resultat<Activation> {
        let inconnu = || ErreurPortail::Validation(MODELES_ACTIVATION[0].into());
        let code = code_court::verifier(&demande.code).map_err(|_| inconnu())?;
        let utiles = code_court::normaliser(&code);
        let identifiant = valider_uuid(&demande.entreprise)?;
        let client = valider_texte(&demande.client, "nom du client", 200)?;
        let trouve = conn
            .query_row(
                &format!("{SELECT_CODE} WHERE k.empreinte = ?1"),
                [empreinte_code(&utiles)],
                code_depuis,
            )
            .optional()?
            .filter(|c| c.etat != EtatCode::Annule)
            .ok_or_else(inconnu)?;
        if trouve.entreprise.as_deref().is_some_and(|e| e != identifiant) {
            return Err(ErreurPortail::Validation(MODELES_ACTIVATION[1].into()));
        }
        // Déjà activé par cette entreprise : réinstallation, même licence.
        if trouve.etat == EtatCode::Active {
            let licence = trouve
                .licence
                .and_then(|id| {
                    conn.query_row(
                        &format!("{SELECT_LICENCE} WHERE l.id = ?1"),
                        [id],
                        licence_depuis,
                    )
                    .optional()
                    .transpose()
                })
                .transpose()?
                .ok_or_else(|| ErreurPortail::Interne("code activé sans licence".into()))?;
            noter(
                conn,
                None,
                "code_reactive",
                Some(&identifiant),
                &format!("code …{}, adresse {adresse}", trouve.fin),
            )?;
            return Ok(Activation {
                licence: licence.texte,
                postes: licence.postes,
                formule: licence.formule,
                debut: licence.debut,
                echeance: licence.echeance,
            });
        }
        let paiement = trouve.paiement.ok_or_else(|| {
            ErreurPortail::Interne("code sans paiement : codes à vendre pas encore pris en charge".into())
        })?;
        let createur: Compte = conn.query_row(
            &format!(
                "SELECT {COLONNES_COMPTE} FROM compte WHERE id = (SELECT cree_par FROM code_activation WHERE id = ?1)"
            ),
            [trouve.id],
            compte_depuis,
        )?;
        let aujourdhui = self.aujourdhui();
        let transaction = conn.unchecked_transaction()?;
        // Entreprise encore inconnue du portail : créée avec la raison sociale
        // reçue, rattachée au revendeur qui a tiré le code.
        let existe: bool = transaction.query_row(
            "SELECT EXISTS (SELECT 1 FROM entreprise WHERE identifiant = ?1)",
            [&identifiant],
            |r| r.get(0),
        )?;
        if !existe {
            let revendeur = (createur.role == Role::Revendeur).then_some(createur.id);
            transaction.execute(
                "INSERT INTO entreprise (identifiant, client, revendeur, cree_le) VALUES (?1, ?2, ?3, ?4)",
                params![identifiant, client, revendeur, maintenant()],
            )?;
            noter(&transaction, None, "entreprise_creee", Some(&identifiant), &client)?;
        }
        let entreprise = transaction.query_row(
            &format!("{SELECT_ENTREPRISE} WHERE e.identifiant = ?1"),
            [&identifiant],
            entreprise_depuis,
        )?;
        let echeance = echeance_proposee(&transaction, &identifiant, trouve.formule, aujourdhui)?;
        let (_, licence, texte) = signer_et_enregistrer(
            &transaction,
            &self.0.cle,
            aujourdhui,
            ALicence {
                entreprise: &entreprise,
                paiement,
                code: Some(trouve.id),
                formule: trouve.formule,
                postes: trouve.postes,
                modules: &trouve.modules,
                echeance,
                emetteur: &createur,
            },
        )?;
        transaction.execute(
            "UPDATE code_activation SET etat = 'active', entreprise = ?2, active_le = ?3 WHERE id = ?1",
            params![trouve.id, identifiant, maintenant()],
        )?;
        let details = format!(
            "code …{}, {} postes, échéance {}, version {}, adresse {adresse}",
            trouve.fin,
            licence.postes,
            licence.echeance,
            demande.version.trim()
        );
        noter(&transaction, None, "code_active", Some(&identifiant), &details)?;
        transaction.commit()?;
        Ok(Activation {
            licence: texte,
            postes: licence.postes,
            formule: licence.formule,
            debut: licence.debut,
            echeance: licence.echeance,
        })
    }

    /// Licence émise, dans la portée du compte.
    pub fn licence(&self, acteur: &Compte, id: i64) -> Resultat<LicenceEmise> {
        let conn = self.conn();
        let acteur = relire(&conn, acteur)?;
        licence_permise(&conn, &acteur, id)
    }

    // ----- Journal -----

    /// Journal des actions, du plus récent au plus ancien (500 au plus).
    /// L'éditeur voit tout ; un revendeur, ses propres actions.
    pub fn journal(&self, acteur: &Compte) -> Resultat<Vec<Action>> {
        let conn = self.conn();
        let acteur = relire(&conn, acteur)?;
        let filtre = match acteur.role {
            Role::Editeur => String::new(),
            Role::Revendeur => format!(" WHERE j.compte = {}", acteur.id),
        };
        let mut req = conn.prepare(&format!(
            "SELECT j.heure, c.nom, j.action, j.cible, j.details
             FROM journal j LEFT JOIN compte c ON c.id = j.compte{filtre}
             ORDER BY j.id DESC LIMIT 500"
        ))?;
        let actions = req
            .query_map([], |r| {
                Ok(Action {
                    heure: r.get(0)?,
                    compte: r.get(1)?,
                    action: r.get(2)?,
                    cible: r.get(3)?,
                    details: r.get(4)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(actions)
    }
}

fn creer_compte(
    conn: &Connection,
    role: Role,
    identifiant: &str,
    nom: &str,
    mot_de_passe: &str,
) -> Resultat<Compte> {
    let identifiant = valider_identifiant(identifiant)?.to_ascii_lowercase();
    let nom = valider_texte(nom, "nom", 120)?;
    valider_mot_de_passe(mot_de_passe)?;
    let existe: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM compte WHERE identifiant = ?1)",
        [&identifiant],
        |r| r.get(0),
    )?;
    if existe {
        return refus("identifiant déjà pris");
    }
    conn.execute(
        "INSERT INTO compte (identifiant, nom, role, mot_de_passe, cree_le) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![identifiant, nom, role.code(), hacher(mot_de_passe)?, maintenant()],
    )?;
    Ok(Compte {
        id: conn.last_insert_rowid(),
        identifiant,
        nom,
        role,
        actif: true,
    })
}

fn paiement_permis(conn: &Connection, acteur: &Compte, id: i64) -> Resultat<Paiement> {
    let paiement = conn
        .query_row(
            &format!("{SELECT_PAIEMENT} WHERE p.id = ?1"),
            [id],
            paiement_depuis,
        )
        .optional()?
        .ok_or(ErreurPortail::Introuvable)?;
    entreprise_permise(conn, acteur, &paiement.entreprise)?;
    Ok(paiement)
}

/// Code dans la portée du compte : celui de son entreprise, ou un code qu'il
/// a tiré ; l'éditeur les voit tous.
fn code_permis(conn: &Connection, acteur: &Compte, id: i64) -> Resultat<CodeActivation> {
    let (code, createur) = conn
        .query_row(&format!("{SELECT_CODE} WHERE k.id = ?1"), [id], |r| {
            Ok((code_depuis(r)?, r.get::<_, i64>(12)?))
        })
        .optional()?
        .ok_or(ErreurPortail::Introuvable)?;
    if acteur.role == Role::Editeur || createur == acteur.id {
        return Ok(code);
    }
    match &code.entreprise {
        Some(e) => entreprise_permise(conn, acteur, e).map(|_| code),
        None => Err(ErreurPortail::Introuvable),
    }
}

fn licence_permise(conn: &Connection, acteur: &Compte, id: i64) -> Resultat<LicenceEmise> {
    let licence = conn
        .query_row(
            &format!("{SELECT_LICENCE} WHERE l.id = ?1"),
            [id],
            licence_depuis,
        )
        .optional()?
        .ok_or(ErreurPortail::Introuvable)?;
    entreprise_permise(conn, acteur, &licence.entreprise)?;
    Ok(licence)
}

#[cfg(test)]
mod tests {
    use super::milliers;

    #[test]
    fn montants_groupes_par_milliers() {
        assert_eq!(milliers(12500), "12\u{202f}500");
        assert_eq!(milliers(150_000), "150\u{202f}000");
        assert_eq!(milliers(999), "999");
        assert_eq!(milliers(1_000_000), "1\u{202f}000\u{202f}000");
    }
}
