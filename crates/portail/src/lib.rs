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
mod chaine;
mod coffre;
mod frein;
pub mod pages;
mod sessions;
pub mod telechargement;
pub mod totp;
pub mod web;

use std::path::Path;
use std::sync::{Arc, Condvar, LazyLock, Mutex, MutexGuard};

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

pub use chaine::{Archivage, EntreeChainee, EtatChaine, lire_export};

use sessions::Sessions;

/// Longueur minimale d'un mot de passe du portail (lignes directrices du
/// NIST, SP 800-63B : 15 caractères pour un mot de passe seul facteur).
pub const LONGUEUR_MIN_MOT_DE_PASSE: usize = 15;

/// Mots de passe compromis refusés, un par ligne : ceux de 15 caractères ou
/// plus parmi les 100 000 plus courants de la liste « ten million passwords »
/// (SecLists, licence MIT ; provenance et licence dans
/// `donnees/mots-de-passe-compromis-SOURCE.txt`).
const MOTS_DE_PASSE_COMPROMIS: &str = include_str!("../donnees/mots-de-passe-compromis.txt");

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
    /// Paiement d'origine ; aucun pour un code à vendre.
    pub paiement: Option<i64>,
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

/// Gamme d'un code (décision 75) : abonnement d'entreprise (serveur et
/// postes 1.x) ou ligne V1 (un poste de la 0.2.x).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gamme {
    Entreprise,
    V1,
}

impl Gamme {
    pub fn code(self) -> &'static str {
        match self {
            Gamme::Entreprise => "entreprise",
            Gamme::V1 => "v1",
        }
    }

    pub fn libelle(self) -> &'static str {
        match self {
            Gamme::Entreprise => "Entreprise (serveur et postes)",
            Gamme::V1 => "Ligne V1 (un poste)",
        }
    }

    fn depuis_code(code: &str) -> Option<Gamme> {
        match code {
            "entreprise" => Some(Gamme::Entreprise),
            "v1" => Some(Gamme::V1),
            _ => None,
        }
    }
}

/// Code d'activation, sans le code en clair (jamais gardé).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeActivation {
    pub id: i64,
    pub gamme: Gamme,
    /// Édition à laquelle le code est destiné (décision du 2026-11-26).
    pub produit: lic::Produit,
    /// Poste lié en ce moment (code V1 activé, pas libéré).
    pub poste: Option<String>,
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
    /// Identifiant du poste (ligne V1) ; vide pour une demande d'entreprise.
    pub poste: String,
    /// Édition du programme qui active (« entreprise » ou « monoposte ») ; vide
    /// pour un programme antérieur, qui est de l'édition Entreprise.
    pub produit: String,
}

/// Demande d'essai envoyée par le programme (route `/api/essai`, décision 78).
#[derive(Debug, Clone, Default)]
pub struct DemandeEssai {
    /// `entreprise` ou `monoposte`.
    pub produit: String,
    /// Identifiant du poste (`identifiant_poste`).
    pub poste: String,
    /// Version du programme, notée au journal.
    pub version: String,
}

/// Réponse à une demande d'essai : l'attestation signée et la date de début.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EssaiAtteste {
    pub attestation: String,
    pub debut: String,
}

/// Modèles des refus d'une demande d'essai, traduits par le programme.
pub const MODELES_ESSAI: &[&str] = &["demande d'essai invalide"];

/// Poste rapporté par un état de licence (décision 80) : ce que le serveur de
/// l'entreprise sait de lui.
#[derive(Debug, Clone, Default)]
pub struct PosteRapporte {
    /// Empreinte SHA-256 du certificat du poste, hexadécimal minuscule.
    pub empreinte: String,
    /// Identifiant de l'ordinateur (`identifiant_poste`) ; vide pour un poste
    /// qui ne l'a pas donné à son inscription.
    pub ordinateur: String,
    pub nom: String,
    /// Date d'inscription et dernier contact, tels que le serveur les note.
    pub inscrit_le: String,
    pub vu_le: String,
}

/// État de licence envoyé par un poste (`POST /api/licence/etat`).
#[derive(Debug, Clone, Default)]
pub struct DemandeEtat {
    /// Identifiant de l'entreprise.
    pub entreprise: String,
    /// `entreprise` ou `monoposte`.
    pub produit: String,
    /// Version du programme qui relaie.
    pub version: String,
    pub postes: Vec<PosteRapporte>,
}

/// Refus de la route d'état : le programme les note sans les afficher.
pub const MODELES_ETAT: &[&str] = &[
    "demande d'état de licence invalide",
    "entreprise inconnue du portail",
];

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
/// qui les traduit (partagés avec lui par `kadenz-licence`).
pub use kadenz_licence::code_court::MODELES_ACTIVATION;

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
    /// Appareils rapportés par les postes (décision 80), actifs d'abord.
    pub appareils: Vec<Appareil>,
    /// Ordinateurs bloqués, du plus récent au plus ancien.
    pub bloques: Vec<OrdinateurBloque>,
    /// Révocation en cours, s'il y en a une.
    pub revocation: Option<Revocation>,
}

/// Appareil d'une licence, tel que les postes l'ont rapporté (décision 80).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Appareil {
    pub id: i64,
    /// Identifiant de l'entreprise.
    pub entreprise: String,
    pub empreinte: String,
    /// Identifiant de l'ordinateur ; vide pour un poste ancien.
    pub ordinateur: String,
    pub nom: String,
    pub inscrit_le: String,
    /// Dernier contact du poste avec son serveur.
    pub contact_le: String,
    /// Dernier état reçu par le portail qui le contenait.
    pub vu_le: String,
    /// Déconnexion demandée au portail.
    pub a_retirer_le: Option<String>,
    /// Absent du dernier état reçu : retiré du serveur de l'entreprise.
    pub retire_le: Option<String>,
}

/// Ordinateur bloqué : le serveur de l'entreprise refuse de l'inscrire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrdinateurBloque {
    pub ordinateur: String,
    pub bloque_le: String,
    /// Nom du compte qui l'a bloqué.
    pub par: String,
}

/// Révocation en cours de la licence d'une entreprise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revocation {
    pub revoquee_le: String,
    /// Nom du compte qui l'a révoquée.
    pub par: String,
    pub raison: String,
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
    /// Mot de passe accepté, code de double authentification demandé : le
    /// jeton de l'étape, à présenter avec le code (`confirmer_code`).
    DeuxiemeFacteur(String),
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
            ErreurPortail::DeuxiemeFacteur(_) => {
                write!(f, "code de double authentification demandé")
            }
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

/// Ordre des verrous, partout : la base d'abord, puis les sessions ; jamais
/// la base pendant que les sessions sont tenues. Le hachage des mots de passe
/// (Argon2, environ 19 Mio et une fraction de seconde chacun) ne se fait
/// jamais sous le verrou de la base : il passe par la porte `hachage`, qui en
/// limite le nombre simultané.
struct Interieur {
    conn: Mutex<Connection>,
    cle: SigningKey,
    /// Chiffrement des secrets de double authentification (clé dérivée de
    /// `cle`, module `coffre`). Absent quand la base est ouverte sans la clé
    /// du portail (commandes d'administration).
    coffre: Option<coffre::Coffre>,
    horloge: Horloge,
    sessions: Mutex<Sessions>,
    hachage: Porte,
    /// Connexions dont le mot de passe est accepté et qui attendent le code
    /// de double authentification, par jeton d'étape. Ordre des verrous :
    /// la base, puis les étapes, jamais l'inverse.
    etapes: Mutex<std::collections::HashMap<String, Etape>>,
}

/// Connexion en attente du code de double authentification.
struct Etape {
    compte: i64,
    identifiant: String,
    debut: std::time::Instant,
    essais: u32,
}

/// Durée d'une étape : passé ce délai, il faut redonner le mot de passe.
const DUREE_ETAPE: std::time::Duration = std::time::Duration::from_secs(300);
/// Codes faux permis par étape, avant de redonner le mot de passe.
const ESSAIS_ETAPE: u32 = 5;
/// Codes de secours donnés à l'activation de la double authentification.
pub const CODES_SECOURS: usize = 10;

/// Hachages Argon2 simultanés au plus : le serveur maison a 2 cœurs et
/// 3,2 Go ; sans limite, des connexions en rafale épuiseraient la mémoire.
const HACHAGES_SIMULTANES: usize = 2;

/// Porte à places comptées : `passer` attend une place libre, le temps de la
/// fonction donnée.
struct Porte {
    libres: Mutex<usize>,
    signal: Condvar,
}

impl Porte {
    fn new(places: usize) -> Porte {
        Porte {
            libres: Mutex::new(places),
            signal: Condvar::new(),
        }
    }

    fn passer<T>(&self, f: impl FnOnce() -> T) -> T {
        {
            let mut libres = verrou(&self.libres);
            while *libres == 0 {
                libres = self.signal.wait(libres).unwrap_or_else(|e| e.into_inner());
            }
            *libres -= 1;
        }
        // La place est rendue même si `f` panique.
        struct Rendre<'a>(&'a Porte);
        impl Drop for Rendre<'_> {
            fn drop(&mut self) {
                *verrou(&self.0.libres) += 1;
                self.0.signal.notify_one();
            }
        }
        let _rendre = Rendre(self);
        f()
    }
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

/// Empreinte d'un mot de passe aléatoire, mêmes réglages qu'un vrai compte :
/// vérifiée quand le compte n'existe pas ou que son empreinte est illisible,
/// pour que le temps de réponse ne dise pas si un identifiant existe.
static EMPREINTE_FACTICE: LazyLock<String> = LazyLock::new(|| {
    hacher(&sessions::alea()).expect("hachage Argon2 d'un mot de passe aléatoire")
});

/// Vérifie un mot de passe ; sans empreinte (compte inconnu), vérifie contre
/// l'empreinte factice et répond non, en prenant le même temps.
fn mot_de_passe_correct(mot_de_passe: &str, hache: Option<&str>) -> bool {
    let verifier = |empreinte: &str| {
        PasswordHash::new(empreinte)
            .map(|h| {
                Argon2::default()
                    .verify_password(mot_de_passe.as_bytes(), &h)
                    .is_ok()
            })
            .ok()
    };
    match hache.and_then(verifier) {
        Some(correct) => correct,
        None => {
            let _ = verifier(&EMPREINTE_FACTICE);
            false
        }
    }
}

/// Secondes depuis 1970 (UTC), pour le frein.
/// Horloge du portail en secondes, pour les liens de téléchargement.
pub(crate) fn secondes_publiques() -> i64 {
    secondes()
}

fn secondes() -> i64 {
    Utc::now().timestamp()
}

fn valider_mot_de_passe(mot_de_passe: &str) -> Resultat<()> {
    if mot_de_passe.chars().count() < LONGUEUR_MIN_MOT_DE_PASSE {
        return Err(ErreurPortail::Validation(format!(
            "mot de passe trop court : {LONGUEUR_MIN_MOT_DE_PASSE} caractères au moins"
        )));
    }
    let minuscules = mot_de_passe.to_lowercase();
    if MOTS_DE_PASSE_COMPROMIS
        .lines()
        .any(|l| l.trim().to_lowercase() == minuscules)
    {
        return refus(
            "mot de passe trop courant, déjà publié dans des listes de mots de passe volés : choisissez-en un autre",
        );
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

/// Longueurs gardées au journal pour des textes du programme client
/// (entreprise refusée, version) : de quoi lire une erreur, jamais plus.
const MAX_ENTREPRISE_JOURNAL: usize = 64;
const MAX_VERSION_JOURNAL: usize = 32;

/// Texte reçu d'un client et noté au journal en ajout seul : sans caractère
/// de contrôle, borné à `max` caractères (« … » s'il est coupé). Le journal
/// ne s'efface pas : un texte non borné y resterait.
fn pour_journal(texte: &str, max: usize) -> String {
    let propre: String = texte.trim().chars().filter(|c| !c.is_control()).collect();
    if propre.chars().count() <= max {
        propre
    } else {
        let mut coupe: String = propre.chars().take(max).collect();
        coupe.push('…');
        coupe
    }
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
/// Identifiant de poste (`kadenz_licence::identifiant_poste`) : quatre
/// groupes de quatre caractères en base 32 de Crockford, majuscules.
fn valider_poste(poste: &str) -> Resultat<String> {
    let poste = poste.trim().to_ascii_uppercase();
    let alphabet = |c: char| c.is_ascii_digit() || (c.is_ascii_uppercase() && !"ILOU".contains(c));
    let forme = poste.len() == 19
        && poste.char_indices().all(|(i, c)| match i {
            4 | 9 | 14 => c == '-',
            _ => alphabet(c),
        });
    if !forme {
        return refus("identifiant de poste invalide");
    }
    Ok(poste)
}

/// Refus pendant une attente du frein : une entrée au journal au plus par
/// clé et par attente (`frein::refus_a_noter`). Un journal en échec ne
/// change pas la réponse : le refus reste un refus.
fn noter_refus_freine(
    conn: &Connection,
    cles: &[frein::Cle],
    action: &str,
    qui: &str,
    attente: u64,
) {
    if let Ok(true) = frein::refus_a_noter(conn, cles, secondes()) {
        let details = format!(
            "{qui}, attente de {attente} s ; les refus suivants de cette attente ne sont pas notés"
        );
        let _ = noter(conn, None, action, None, &details);
    }
}

/// Double authentification active pour ce compte.
fn double_active(conn: &Connection, compte: i64) -> Resultat<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM double_authentification
                        WHERE compte = ?1 AND active_le IS NOT NULL)",
        [compte],
        |r| r.get(0),
    )?)
}

/// Code saisi sans espaces ni tirets, en majuscules.
fn normaliser_code(code: &str) -> String {
    code.chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect::<String>()
        .to_ascii_uppercase()
}

/// Code de secours : 10 caractères Base32 (50 bits d'aléa), `XXXXX-XXXXX`.
fn code_de_secours() -> Resultat<String> {
    let mut octets = [0u8; 7];
    getrandom::fill(&mut octets).map_err(|e| ErreurPortail::Interne(e.to_string()))?;
    let texte = totp::encoder(&octets);
    Ok(format!("{}-{}", &texte[..5], &texte[5..10]))
}

/// Second facteur vérifié pour un compte à la double authentification
/// active : 6 chiffres, code de l'application (un pas ne sert qu'une
/// fois) ; sinon, code de secours encore inutilisé. Rend le moyen employé,
/// pour le journal.
fn verifier_second_facteur(
    conn: &Connection,
    coffre: &coffre::Coffre,
    compte: i64,
    saisi: &str,
) -> Resultat<Option<&'static str>> {
    let ligne: Option<(String, i64)> = conn
        .query_row(
            "SELECT secret, dernier_pas FROM double_authentification
             WHERE compte = ?1 AND active_le IS NOT NULL",
            [compte],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((secret, dernier_pas)) = ligne else {
        return Ok(None);
    };
    if saisi.len() == totp::CHIFFRES as usize && saisi.bytes().all(|o| o.is_ascii_digit()) {
        let octets = octets_du_secret(coffre, compte, &secret)?;
        let Some(pas) = totp::verifier(&octets, saisi, secondes() as u64, dernier_pas) else {
            return Ok(None);
        };
        let pris = conn.execute(
            "UPDATE double_authentification SET dernier_pas = ?2
             WHERE compte = ?1 AND dernier_pas < ?2",
            params![compte, pas],
        )?;
        return Ok((pris == 1).then_some("application d'authentification"));
    }
    let pris = conn.execute(
        "UPDATE code_secours SET utilise_le = ?3
         WHERE compte = ?1 AND empreinte = ?2 AND utilise_le IS NULL",
        params![compte, empreinte_code(saisi), maintenant()],
    )?;
    Ok((pris >= 1).then_some("code de secours"))
}

/// Secret (Base32) d'un compte, déchiffré. Illisible : clé du portail
/// changée ou valeur altérée ; le compte passe par
/// `retirer-double-authentification`.
fn secret_du_compte(coffre: &coffre::Coffre, compte: i64, stocke: &str) -> Resultat<String> {
    coffre.dechiffrer(compte, stocke).ok_or_else(|| {
        ErreurPortail::Interne(format!(
            "secret de double authentification du compte {compte} illisible (clé du portail changée ?)"
        ))
    })
}

fn octets_du_secret(coffre: &coffre::Coffre, compte: i64, stocke: &str) -> Resultat<Vec<u8>> {
    totp::decoder(&secret_du_compte(coffre, compte, stocke)?)
        .ok_or_else(|| ErreurPortail::Interne("secret de double authentification illisible".into()))
}

/// Au démarrage : chiffre les secrets encore en clair (bases d'avant le
/// schéma 10), en une transaction. Sans effet s'ils le sont tous ; un échec
/// ne change rien et arrête le démarrage.
fn chiffrer_secrets(conn: &Connection, coffre: &coffre::Coffre) -> Resultat<()> {
    let en_clair: Vec<(i64, String)> = conn
        .prepare("SELECT compte, secret FROM double_authentification")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<Vec<(i64, String)>>>()?
        .into_iter()
        .filter(|(_, secret)| !coffre::chiffre(secret))
        .collect();
    if en_clair.is_empty() {
        return Ok(());
    }
    let transaction = conn.unchecked_transaction()?;
    for (compte, secret) in &en_clair {
        let stocke = coffre
            .chiffrer(*compte, secret)
            .map_err(ErreurPortail::Interne)?;
        transaction.execute(
            "UPDATE double_authentification SET secret = ?2 WHERE compte = ?1",
            params![compte, stocke],
        )?;
    }
    transaction.commit()?;
    Ok(())
}

/// Appareils d'une entreprise : actifs d'abord (par nom), puis retirés.
fn appareils_de(conn: &Connection, entreprise: &str) -> rusqlite::Result<Vec<Appareil>> {
    let mut req = conn.prepare(
        "SELECT id, empreinte, ordinateur, nom, inscrit_le, contact_le, vu_le, a_retirer_le, retire_le
         FROM appareil WHERE entreprise = ?1
         ORDER BY retire_le IS NOT NULL, nom COLLATE NOCASE, id",
    )?;
    req.query_map([entreprise], |r| {
        Ok(Appareil {
            id: r.get(0)?,
            entreprise: entreprise.to_string(),
            empreinte: r.get(1)?,
            ordinateur: r.get(2)?,
            nom: r.get(3)?,
            inscrit_le: r.get(4)?,
            contact_le: r.get(5)?,
            vu_le: r.get(6)?,
            a_retirer_le: r.get(7)?,
            retire_le: r.get(8)?,
        })
    })?
    .collect()
}

fn bloques_de(conn: &Connection, entreprise: &str) -> rusqlite::Result<Vec<OrdinateurBloque>> {
    let mut req = conn.prepare(
        "SELECT b.ordinateur, b.bloque_le, c.nom FROM ordinateur_bloque b
         JOIN compte c ON c.id = b.par WHERE b.entreprise = ?1 ORDER BY b.bloque_le DESC",
    )?;
    req.query_map([entreprise], |r| {
        Ok(OrdinateurBloque {
            ordinateur: r.get(0)?,
            bloque_le: r.get(1)?,
            par: r.get(2)?,
        })
    })?
    .collect()
}

fn revocation_de(conn: &Connection, entreprise: &str) -> rusqlite::Result<Option<Revocation>> {
    conn.query_row(
        "SELECT r.revoquee_le, c.nom, r.raison FROM revocation r JOIN compte c ON c.id = r.par
         WHERE r.entreprise = ?1 AND r.retablie_le IS NULL",
        [entreprise],
        |r| {
            Ok(Revocation {
                revoquee_le: r.get(0)?,
                par: r.get(1)?,
                raison: r.get(2)?,
            })
        },
    )
    .optional()
}

/// Vrai si la licence de l'entreprise est révoquée (décision 80).
fn revoquee(conn: &Connection, entreprise: &str) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM revocation WHERE entreprise = ?1 AND retablie_le IS NULL)",
        [entreprise],
        |r| r.get(0),
    )
}

/// Bulletin à jour d'une entreprise : révocation, postes dont la déconnexion
/// est demandée et pas encore constatée, ordinateurs bloqués.
fn bulletin_de(
    conn: &Connection,
    entreprise: &str,
    emis_le: &str,
) -> rusqlite::Result<kadenz_licence::bulletin::Bulletin> {
    let liste = |sql: &str| -> rusqlite::Result<Vec<String>> {
        let mut req = conn.prepare(sql)?;
        req.query_map([entreprise], |r| r.get(0))?.collect()
    };
    Ok(kadenz_licence::bulletin::Bulletin {
        entreprise: entreprise.to_string(),
        emis_le: emis_le.to_string(),
        revoquee: revoquee(conn, entreprise)?,
        retires: liste(
            "SELECT empreinte FROM appareil
             WHERE entreprise = ?1 AND a_retirer_le IS NOT NULL AND retire_le IS NULL
             ORDER BY empreinte",
        )?,
        bloques: liste(
            "SELECT ordinateur FROM ordinateur_bloque WHERE entreprise = ?1 ORDER BY ordinateur",
        )?,
    })
}

/// Poste rapporté nettoyé, ou `None` s'il est mal formé : empreinte en
/// hexadécimal de 64 caractères, identifiant d'ordinateur vide ou valide, nom
/// et dates courts, sans caractère de contrôle.
fn valider_poste_rapporte(p: &PosteRapporte) -> Option<PosteRapporte> {
    let empreinte = p.empreinte.trim().to_ascii_lowercase();
    if empreinte.len() != 64 || !empreinte.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let ordinateur = if p.ordinateur.trim().is_empty() {
        String::new()
    } else {
        valider_poste(&p.ordinateur).ok()?
    };
    let nom = valider_texte(&p.nom, "nom du poste", 100).ok()?;
    let date = |d: &str| -> Option<String> {
        let d = d.trim();
        (d.len() <= 32 && d.bytes().all(|b| b.is_ascii_graphic() || b == b' '))
            .then(|| d.to_string())
    };
    Some(PosteRapporte {
        empreinte,
        ordinateur,
        nom,
        inscrit_le: date(&p.inscrit_le)?,
        vu_le: date(&p.vu_le)?,
    })
}

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

fn valider_postes(texte: &str) -> Resultat<u32> {
    texte
        .trim()
        .parse()
        .ok()
        .filter(|p| (1..=1000).contains(p))
        .ok_or_else(|| ErreurPortail::Validation("nombre de postes invalide : de 1 à 1000".into()))
}

fn valider_formule(texte: &str) -> Resultat<Formule> {
    Formule::depuis_code(texte.trim())
        .ok_or_else(|| ErreurPortail::Validation("formule invalide : mensuelle ou annuelle".into()))
}

/// Codes à vendre tirés en une fois, au plus.
pub const CODES_PAR_TIRAGE: u32 = 50;

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
            k.entreprise, k.etat, c.nom, k.cree_le, k.active_le, l.id, k.cree_par, k.gamme,
            (SELECT v.poste FROM licence_v1 v WHERE v.code = k.id AND v.liberee_le IS NULL),
            k.produit
     FROM code_activation k JOIN compte c ON c.id = k.cree_par
     LEFT JOIN licence_emise l ON l.code = k.id";

fn code_depuis(r: &rusqlite::Row) -> rusqlite::Result<CodeActivation> {
    Ok(CodeActivation {
        id: r.get(0)?,
        gamme: Gamme::depuis_code(&r.get::<_, String>(13)?).unwrap_or(Gamme::Entreprise),
        produit: lic::Produit::depuis_code(&r.get::<_, String>(15)?)
            .unwrap_or(lic::Produit::Entreprise),
        poste: r.get(14)?,
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
    produit: lic::Produit,
    entreprise: &'a Entreprise,
    paiement: Option<i64>,
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
        produit: a.produit,
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
    let texte = lic::emettre(&licence, cle).map_err(|e| ErreurPortail::Interne(e.to_string()))?;
    lic::verifier_pour(
        &texte,
        &a.entreprise.identifiant,
        a.produit,
        &[cle.verifying_key()],
    )
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
        modules
            .iter()
            .map(|m| m.libelle())
            .collect::<Vec<_>>()
            .join(", ")
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
    // L'entrée et son maillon ensemble, ou rien : un point de sauvegarde,
    // valable dans une transaction en cours comme hors transaction.
    conn.execute_batch("SAVEPOINT noter")?;
    let ecrit = (|| -> rusqlite::Result<()> {
        conn.execute(
            "INSERT INTO journal (heure, compte, action, cible, details) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![maintenant(), compte, action, cible, details],
        )?;
        chaine::chainer(conn, conn.last_insert_rowid())
    })();
    match ecrit {
        Ok(()) => {
            conn.execute_batch("RELEASE noter")?;
            Ok(())
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK TO noter; RELEASE noter");
            Err(e.into())
        }
    }
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
        let coffre = coffre::Coffre::depuis_cle(&cle);
        Portail::construire(dossier, cle, Some(coffre), horloge)
    }

    /// Ouvre la base pour une commande d'administration qui ne signe rien et
    /// ne lit aucun secret de double authentification (création d'un compte,
    /// retrait de la double authentification, journal) : la clé du portail
    /// n'est pas lue, une clé jetable la remplace. Les secrets restent tels
    /// quels : ceux encore en clair ne sont chiffrés qu'à l'ouverture avec la
    /// clé du portail, jamais avec la clé jetable.
    pub fn ouvrir_sans_cle(dossier: &Path) -> Resultat<Portail> {
        let mut graine = zeroize::Zeroizing::new([0u8; 32]);
        getrandom::fill(&mut graine[..]).map_err(|e| ErreurPortail::Interne(e.to_string()))?;
        let cle = SigningKey::from_bytes(&graine);
        Portail::construire(dossier, cle, None, Arc::new(|| Utc::now().date_naive()))
    }

    fn construire(
        dossier: &Path,
        cle: SigningKey,
        coffre: Option<coffre::Coffre>,
        horloge: Horloge,
    ) -> Resultat<Portail> {
        let conn = base::ouvrir(dossier)?;
        if let Some(coffre) = &coffre {
            chiffrer_secrets(&conn, coffre)?;
        }
        Ok(Portail(Arc::new(Interieur {
            conn: Mutex::new(conn),
            cle,
            coffre,
            horloge,
            sessions: Mutex::new(Sessions::default()),
            hachage: Porte::new(HACHAGES_SIMULTANES),
            etapes: Mutex::new(std::collections::HashMap::new()),
        })))
    }

    /// Chiffrement des secrets de double authentification ; refusé quand la
    /// base est ouverte sans la clé du portail.
    fn coffre(&self) -> Resultat<&coffre::Coffre> {
        self.0.coffre.as_ref().ok_or_else(|| {
            ErreurPortail::Interne(
                "portail ouvert sans sa clé : secrets de double authentification indisponibles"
                    .into(),
            )
        })
    }

    /// Hache un mot de passe hors du verrou de la base, par la porte.
    fn hacher(&self, mot_de_passe: &str) -> Resultat<String> {
        self.0.hachage.passer(|| hacher(mot_de_passe))
    }

    /// Vérifie un mot de passe hors du verrou de la base, par la porte.
    fn verifier(&self, mot_de_passe: &str, hache: Option<&str>) -> bool {
        self.0
            .hachage
            .passer(|| mot_de_passe_correct(mot_de_passe, hache))
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
        valider_mot_de_passe(mot_de_passe)?;
        let hache = self.hacher(mot_de_passe)?;
        let conn = self.conn();
        let compte = creer_compte(&conn, Role::Editeur, identifiant, nom, &hache)?;
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
        // Droit contrôlé avant le hachage (aucun calcul pour un non-éditeur),
        // puis de nouveau sous le verrou.
        exiger_editeur(&self.conn(), acteur)?;
        valider_mot_de_passe(mot_de_passe)?;
        let hache = self.hacher(mot_de_passe)?;
        let conn = self.conn();
        let acteur = exiger_editeur(&conn, acteur)?;
        let compte = creer_compte(&conn, Role::Revendeur, identifiant, nom, &hache)?;
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

    /// Change le mot de passe du compte connecté. Toutes les sessions du
    /// compte finissent (une session volée ne survit pas au changement) ; le
    /// jeton d'une nouvelle session est rendu pour la personne qui l'a changé.
    pub fn changer_mot_de_passe(
        &self,
        acteur: &Compte,
        ancien: &str,
        nouveau: &str,
        confirmation: &str,
    ) -> Resultat<String> {
        let (acteur, hache) = {
            let conn = self.conn();
            let acteur = relire(&conn, acteur)?;
            let hache: String = conn.query_row(
                "SELECT mot_de_passe FROM compte WHERE id = ?1",
                [acteur.id],
                |r| r.get(0),
            )?;
            (acteur, hache)
        };
        if !self.verifier(ancien, Some(&hache)) {
            return refus("mot de passe actuel incorrect");
        }
        if nouveau != confirmation {
            return refus("les deux nouveaux mots de passe diffèrent");
        }
        valider_mot_de_passe(nouveau)?;
        let nouvelle = self.hacher(nouveau)?;
        let conn = self.conn();
        // Empreinte changée entre la vérification et l'écriture (autre
        // changement en même temps) : refus, rien n'est écrit.
        let ecrites = conn.execute(
            "UPDATE compte SET mot_de_passe = ?1 WHERE id = ?2 AND mot_de_passe = ?3 AND actif = 1",
            params![nouvelle, acteur.id, hache],
        )?;
        if ecrites != 1 {
            return refus("mot de passe changé entre-temps : recommencez");
        }
        noter(
            &conn,
            Some(acteur.id),
            "mot_de_passe_change",
            Some(&acteur.identifiant),
            "",
        )?;
        let mut sessions = verrou(&self.0.sessions);
        sessions.fermer_compte(acteur.id);
        Ok(sessions.ouvrir(acteur.id))
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
        // Identifiant qu'aucun compte ne peut porter (règle de la création) :
        // refusé tout de suite, sans toucher au frein ni au journal, pour
        // qu'un texte quelconque, de n'importe quelle longueur, n'y soit
        // jamais écrit (audit d'OpenCode, 2026-11-19). La règle étant
        // publique, la réponse rapide ne dit rien d'un compte.
        let Ok(identifiant) = valider_identifiant(identifiant) else {
            return Err(ErreurPortail::Identifiants);
        };
        let identifiant = identifiant.to_ascii_lowercase();
        let cles = frein::cles_connexion(&identifiant, adresse);
        // Frein, puis lecture du compte, sous le verrou ; vérification du mot
        // de passe hors du verrou ; puis, de nouveau sous le verrou, le compte
        // doit être encore actif avec la même empreinte (un changement de mot
        // de passe ou une désactivation pendant la vérification l'emporte).
        let trouve = {
            let conn = self.conn();
            if let Some(attente) = frein::attente(&conn, &cles, secondes())? {
                noter_refus_freine(
                    &conn,
                    &cles,
                    "connexion_freinee",
                    &format!("identifiant {identifiant}, adresse {adresse}"),
                    attente,
                );
                return Err(ErreurPortail::Freine(attente));
            }
            conn.query_row(
                &format!(
                    "SELECT {COLONNES_COMPTE}, mot_de_passe FROM compte WHERE identifiant = ?1"
                ),
                [&identifiant],
                |r| Ok((compte_depuis(r)?, r.get::<_, String>(5)?)),
            )
            .optional()?
        };
        let correct = self.verifier(mot_de_passe, trouve.as_ref().map(|t| t.1.as_str()));
        let conn = self.conn();
        let encore_valable = |compte: &Compte, hache: &str| -> Resultat<bool> {
            Ok(conn
                .query_row(
                    "SELECT actif = 1 AND mot_de_passe = ?2 FROM compte WHERE id = ?1",
                    params![compte.id, hache],
                    |r| r.get::<_, bool>(0),
                )
                .optional()?
                .unwrap_or(false))
        };
        match trouve {
            Some((compte, hache))
                if correct && compte.actif && encore_valable(&compte, &hache)? =>
            {
                // Double authentification active : pas encore de session, ni
                // d'effacement du frein (sinon le mot de passe seul relancerait
                // les essais de code) ; une étape attend le code.
                if double_active(&conn, compte.id)? {
                    noter(
                        &conn,
                        Some(compte.id),
                        "connexion_etape",
                        None,
                        &format!("adresse {adresse}, code demandé"),
                    )?;
                    let etape = sessions::alea();
                    let mut etapes = verrou(&self.0.etapes);
                    etapes.retain(|_, e| e.debut.elapsed() <= DUREE_ETAPE);
                    etapes.insert(
                        etape.clone(),
                        Etape {
                            compte: compte.id,
                            identifiant: identifiant.clone(),
                            debut: std::time::Instant::now(),
                            essais: 0,
                        },
                    );
                    return Err(ErreurPortail::DeuxiemeFacteur(etape));
                }
                frein::reussite(&conn, &cles)?;
                noter(&conn, Some(compte.id), "connexion", None, adresse)?;
                let jeton = verrou(&self.0.sessions).ouvrir(compte.id);
                Ok((jeton, compte))
            }
            trouve => {
                frein::echec(&conn, &cles, secondes())?;
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
    /// Étape de connexion encore valable (page du code).
    pub fn etape_valable(&self, etape: &str) -> bool {
        let mut etapes = verrou(&self.0.etapes);
        etapes.retain(|_, e| e.debut.elapsed() <= DUREE_ETAPE);
        etapes.contains_key(etape)
    }

    /// Seconde étape de la connexion : code de l'application
    /// d'authentification (6 chiffres) ou code de secours. Étape inconnue ou
    /// expirée : `Identifiants` (le mot de passe est à redonner). Code faux :
    /// refus freiné comme un mot de passe faux ; après 5 codes faux, l'étape
    /// est close.
    pub fn confirmer_code(
        &self,
        etape: &str,
        code: &str,
        adresse: &str,
    ) -> Resultat<(String, Compte)> {
        let trouvee = {
            let mut etapes = verrou(&self.0.etapes);
            etapes.retain(|_, e| e.debut.elapsed() <= DUREE_ETAPE);
            etapes.get(etape).map(|e| (e.compte, e.identifiant.clone()))
        };
        let Some((compte_id, identifiant)) = trouvee else {
            return Err(ErreurPortail::Identifiants);
        };
        let cles = frein::cles_connexion(&identifiant, adresse);
        let conn = self.conn();
        if let Some(attente) = frein::attente(&conn, &cles, secondes())? {
            noter_refus_freine(
                &conn,
                &cles,
                "code_freine",
                &format!("identifiant {identifiant}, adresse {adresse}"),
                attente,
            );
            return Err(ErreurPortail::Freine(attente));
        }
        let saisi = normaliser_code(code);
        let Some(moyen) = verifier_second_facteur(&conn, self.coffre()?, compte_id, &saisi)? else {
            frein::echec(&conn, &cles, secondes())?;
            noter(
                &conn,
                Some(compte_id),
                "code_refuse",
                None,
                &format!("adresse {adresse}"),
            )?;
            let mut etapes = verrou(&self.0.etapes);
            let epuisee = etapes
                .get_mut(etape)
                .map(|e| {
                    e.essais += 1;
                    e.essais >= ESSAIS_ETAPE
                })
                .unwrap_or(false);
            if epuisee {
                etapes.remove(etape);
            }
            return refus("code incorrect ou déjà utilisé");
        };
        verrou(&self.0.etapes).remove(etape);
        let compte = conn
            .query_row(
                &format!("SELECT {COLONNES_COMPTE} FROM compte WHERE id = ?1"),
                [compte_id],
                compte_depuis,
            )
            .optional()?
            .filter(|c| c.actif)
            .ok_or(ErreurPortail::Identifiants)?;
        frein::reussite(&conn, &cles)?;
        noter(
            &conn,
            Some(compte.id),
            "connexion",
            None,
            &format!("{adresse}, {moyen}"),
        )?;
        let jeton = verrou(&self.0.sessions).ouvrir(compte.id);
        Ok((jeton, compte))
    }

    /// Un éditeur sans double authentification active n'a droit qu'à sa
    /// mise en place (obligatoire avant la mise en service). En cas
    /// d'erreur de lecture, la mise en place est exigée.
    pub fn mise_en_place_exigee(&self, compte: &Compte) -> bool {
        compte.role == Role::Editeur && !double_active(&self.conn(), compte.id).unwrap_or(false)
    }

    /// Mise en place de la double authentification d'un éditeur : secret en
    /// Base32, à saisir dans l'application. Une mise en place en cours garde
    /// son secret (page rechargée) ; une double authentification active est
    /// refusée.
    pub fn preparer_double_authentification(&self, acteur: &Compte) -> Resultat<String> {
        let conn = self.conn();
        let acteur = exiger_editeur(&conn, acteur)?;
        let ligne: Option<(String, Option<String>)> = conn
            .query_row(
                "SELECT secret, active_le FROM double_authentification WHERE compte = ?1",
                [acteur.id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        match ligne {
            Some((_, Some(_))) => refus("la double authentification est déjà active"),
            Some((stocke, None)) => secret_du_compte(self.coffre()?, acteur.id, &stocke),
            None => {
                let mut octets = [0u8; totp::OCTETS_SECRET];
                getrandom::fill(&mut octets).map_err(|e| ErreurPortail::Interne(e.to_string()))?;
                let secret = totp::encoder(&octets);
                let stocke = self
                    .coffre()?
                    .chiffrer(acteur.id, &secret)
                    .map_err(ErreurPortail::Interne)?;
                conn.execute(
                    "INSERT INTO double_authentification (compte, secret) VALUES (?1, ?2)",
                    params![acteur.id, stocke],
                )?;
                Ok(secret)
            }
        }
    }

    /// Active la double authentification avec un premier code de
    /// l'application ; rend les codes de secours en clair, seul moment où
    /// ils existent (le portail n'en garde que l'empreinte).
    pub fn activer_double_authentification(
        &self,
        acteur: &Compte,
        code: &str,
    ) -> Resultat<Vec<String>> {
        let conn = self.conn();
        let acteur = exiger_editeur(&conn, acteur)?;
        let secret: Option<String> = conn
            .query_row(
                "SELECT secret FROM double_authentification WHERE compte = ?1 AND active_le IS NULL",
                [acteur.id],
                |r| r.get(0),
            )
            .optional()?;
        let Some(secret) = secret else {
            return refus("aucune mise en place en cours : rechargez la page");
        };
        let octets = octets_du_secret(self.coffre()?, acteur.id, &secret)?;
        let Some(pas) = totp::verifier(&octets, &normaliser_code(code), secondes() as u64, 0)
        else {
            return refus(
                "code incorrect : vérifiez la clé saisie dans l'application et l'heure du téléphone",
            );
        };
        let codes = (0..CODES_SECOURS)
            .map(|_| code_de_secours())
            .collect::<Resultat<Vec<_>>>()?;
        let transaction = conn.unchecked_transaction()?;
        transaction.execute(
            "UPDATE double_authentification SET active_le = ?2, dernier_pas = ?3 WHERE compte = ?1",
            params![acteur.id, maintenant(), pas],
        )?;
        transaction.execute("DELETE FROM code_secours WHERE compte = ?1", [acteur.id])?;
        for c in &codes {
            transaction.execute(
                "INSERT INTO code_secours (compte, empreinte) VALUES (?1, ?2)",
                params![acteur.id, empreinte_code(&normaliser_code(c))],
            )?;
        }
        noter(
            &transaction,
            Some(acteur.id),
            "double_authentification_activee",
            None,
            &format!(
                "compte {}, {CODES_SECOURS} codes de secours",
                acteur.identifiant
            ),
        )?;
        transaction.commit()?;
        Ok(codes)
    }

    /// Retire la double authentification d'un compte (programme, sur le
    /// serveur : téléphone perdu sans code de secours). Journalisé ; le
    /// compte devra la remettre en place à sa prochaine connexion.
    pub fn retirer_double_authentification(&self, identifiant: &str) -> Resultat<()> {
        let identifiant = identifiant.trim().to_ascii_lowercase();
        let conn = self.conn();
        let compte: Option<i64> = conn
            .query_row(
                "SELECT id FROM compte WHERE identifiant = ?1",
                [&identifiant],
                |r| r.get(0),
            )
            .optional()?;
        let Some(compte) = compte else {
            return Err(ErreurPortail::Introuvable);
        };
        let transaction = conn.unchecked_transaction()?;
        let retiree = transaction.execute(
            "DELETE FROM double_authentification WHERE compte = ?1",
            [compte],
        )?;
        transaction.execute("DELETE FROM code_secours WHERE compte = ?1", [compte])?;
        if retiree == 0 {
            return refus("aucune double authentification pour ce compte");
        }
        noter(
            &transaction,
            None,
            "double_authentification_retiree",
            None,
            &format!("compte {identifiant}, par le programme sur le serveur"),
        )?;
        transaction.commit()?;
        verrou(&self.0.sessions).fermer_compte(compte);
        Ok(())
    }

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
        // verrous : base, puis sessions).
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
        let appareils = appareils_de(&conn, &entreprise.identifiant)?;
        let bloques = bloques_de(&conn, &entreprise.identifiant)?;
        let revocation = revocation_de(&conn, &entreprise.identifiant)?;
        Ok(FicheEntreprise {
            entreprise,
            paiements,
            licences,
            codes,
            appareils,
            bloques,
            revocation,
        })
    }

    /// Déconnecte un appareil (décision 80) : le prochain bulletin demande
    /// son retrait au serveur de l'entreprise, et son ordinateur, s'il est
    /// connu, est bloqué jusqu'à ce qu'on le réautorise. L'éditeur, ou le
    /// revendeur de l'entreprise.
    pub fn deconnecter_appareil(&self, acteur: &Compte, appareil: i64) -> Resultat<Appareil> {
        let conn = self.conn();
        let acteur = relire(&conn, acteur)?;
        let entreprise: String = conn
            .query_row(
                "SELECT entreprise FROM appareil WHERE id = ?1",
                [appareil],
                |r| r.get(0),
            )
            .optional()?
            .ok_or(ErreurPortail::Introuvable)?;
        entreprise_permise(&conn, &acteur, &entreprise)?;
        let transaction = conn.unchecked_transaction()?;
        let instant = maintenant();
        transaction.execute(
            "UPDATE appareil SET a_retirer_le = COALESCE(a_retirer_le, ?2) WHERE id = ?1",
            params![appareil, instant],
        )?;
        let (nom, ordinateur): (String, String) = transaction.query_row(
            "SELECT nom, ordinateur FROM appareil WHERE id = ?1",
            [appareil],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if !ordinateur.is_empty() {
            transaction.execute(
                "INSERT OR IGNORE INTO ordinateur_bloque (entreprise, ordinateur, bloque_le, par)
                 VALUES (?1, ?2, ?3, ?4)",
                params![entreprise, ordinateur, instant, acteur.id],
            )?;
        }
        noter(
            &transaction,
            Some(acteur.id),
            "appareil_deconnexion",
            Some(&entreprise),
            &format!(
                "{}, ordinateur {}",
                pour_journal(&nom, 100),
                if ordinateur.is_empty() {
                    "non donné"
                } else {
                    &ordinateur
                }
            ),
        )?;
        transaction.commit()?;
        appareils_de(&conn, &entreprise)?
            .into_iter()
            .find(|a| a.id == appareil)
            .ok_or(ErreurPortail::Introuvable)
    }

    /// Réautorise un ordinateur bloqué : il peut de nouveau s'inscrire. Une
    /// déconnexion pas encore constatée sur ses appareils est annulée.
    pub fn reautoriser_ordinateur(
        &self,
        acteur: &Compte,
        entreprise: &str,
        ordinateur: &str,
    ) -> Resultat<()> {
        let conn = self.conn();
        let acteur = relire(&conn, acteur)?;
        let entreprise = entreprise_permise(&conn, &acteur, entreprise)?.identifiant;
        let ordinateur = valider_poste(ordinateur)?;
        let transaction = conn.unchecked_transaction()?;
        let retires = transaction.execute(
            "DELETE FROM ordinateur_bloque WHERE entreprise = ?1 AND ordinateur = ?2",
            params![entreprise, ordinateur],
        )?;
        if retires == 0 {
            return Err(ErreurPortail::Introuvable);
        }
        transaction.execute(
            "UPDATE appareil SET a_retirer_le = NULL
             WHERE entreprise = ?1 AND ordinateur = ?2 AND retire_le IS NULL",
            params![entreprise, ordinateur],
        )?;
        noter(
            &transaction,
            Some(acteur.id),
            "ordinateur_reautorise",
            Some(&entreprise),
            &ordinateur,
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// Révoque la licence d'une entreprise (décision 80) : le prochain
    /// bulletin la met en lecture seule, et aucun code ne s'active plus pour
    /// elle tant qu'elle n'est pas rétablie.
    pub fn revoquer_licence(
        &self,
        acteur: &Compte,
        entreprise: &str,
        raison: &str,
    ) -> Resultat<()> {
        let conn = self.conn();
        let acteur = relire(&conn, acteur)?;
        let entreprise = entreprise_permise(&conn, &acteur, entreprise)?.identifiant;
        let raison = valider_texte(raison, "raison", 200)?;
        if revoquee(&conn, &entreprise)? {
            return refus("licence déjà révoquée");
        }
        let transaction = conn.unchecked_transaction()?;
        transaction.execute(
            "INSERT INTO revocation (entreprise, revoquee_le, par, raison) VALUES (?1, ?2, ?3, ?4)",
            params![entreprise, maintenant(), acteur.id, raison],
        )?;
        noter(
            &transaction,
            Some(acteur.id),
            "licence_revoquee",
            Some(&entreprise),
            &raison,
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// Rétablit une licence révoquée.
    pub fn retablir_licence(&self, acteur: &Compte, entreprise: &str) -> Resultat<()> {
        let conn = self.conn();
        let acteur = relire(&conn, acteur)?;
        let entreprise = entreprise_permise(&conn, &acteur, entreprise)?.identifiant;
        let transaction = conn.unchecked_transaction()?;
        let n = transaction.execute(
            "UPDATE revocation SET retablie_le = ?2, retablie_par = ?3
             WHERE entreprise = ?1 AND retablie_le IS NULL",
            params![entreprise, maintenant(), acteur.id],
        )?;
        if n == 0 {
            return refus("licence non révoquée");
        }
        noter(
            &transaction,
            Some(acteur.id),
            "licence_retablie",
            Some(&entreprise),
            "",
        )?;
        transaction.commit()?;
        Ok(())
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
        let postes = valider_postes(&saisie.postes)?;
        let formule = valider_formule(&saisie.formule)?;
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
        let echeance = echeance_proposee(
            &conn,
            &paiement.entreprise,
            paiement.formule,
            self.aujourdhui(),
        )?;
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
                produit: lic::Produit::Entreprise,
                entreprise: &entreprise,
                paiement: Some(paiement.id),
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
        let (id, code, fin) = inserer_code(
            &conn,
            &acteur,
            paiement.postes,
            paiement.formule,
            &modules,
            Some((paiement.id, &paiement.entreprise)),
            Gamme::Entreprise,
            lic::Produit::Entreprise,
        )?;
        let details = format!(
            "code …{fin}, paiement {}, {} postes, formule {}, modules : {}",
            paiement.id,
            paiement.postes,
            paiement.formule.code(),
            liste_modules(&modules)
        );
        noter(
            &conn,
            Some(acteur.id),
            "code_cree",
            Some(&paiement.entreprise),
            &details,
        )?;
        Ok(CodeCree {
            code,
            fiche: code_permis(&conn, &acteur, id)?,
        })
    }

    /// Tire des codes à vendre (décision 74) : sans paiement ni entreprise,
    /// pour un nombre de postes et une formule ; l'entreprise se lie à
    /// l'activation, rattachée au revendeur qui a tiré le code. Les codes en
    /// clair ne sont rendus qu'ici.
    pub fn creer_codes(
        &self,
        acteur: &Compte,
        nombre: &str,
        postes: &str,
        formule: &str,
        modules: &[Module],
    ) -> Resultat<Vec<CodeCree>> {
        self.creer_codes_gamme(acteur, "entreprise", nombre, postes, formule, modules)
    }

    /// Comme [`Portail::creer_codes`], pour une gamme choisie (décision 75) :
    /// un code V1 vaut un poste.
    pub fn creer_codes_gamme(
        &self,
        acteur: &Compte,
        gamme: &str,
        nombre: &str,
        postes: &str,
        formule: &str,
        modules: &[Module],
    ) -> Resultat<Vec<CodeCree>> {
        // L'édition un poste est une gamme à part pour l'opérateur (décision du
        // 2026-11-26) : un code Entreprise sous le capot, d'un poste, et
        // destiné à l'édition un poste seulement.
        let monoposte = gamme.trim() == "monoposte";
        let produit = if monoposte {
            lic::Produit::Monoposte
        } else {
            lic::Produit::Entreprise
        };
        let gamme = if monoposte {
            Gamme::Entreprise
        } else {
            Gamme::depuis_code(gamme.trim())
                .ok_or_else(|| ErreurPortail::Validation("gamme invalide".into()))?
        };
        let conn = self.conn();
        let acteur = relire(&conn, acteur)?;
        let nombre: u32 = nombre
            .trim()
            .parse()
            .ok()
            .filter(|n| (1..=CODES_PAR_TIRAGE).contains(n))
            .ok_or_else(|| {
                ErreurPortail::Validation(format!(
                    "nombre de codes invalide : de 1 à {CODES_PAR_TIRAGE}"
                ))
            })?;
        let postes = valider_postes(postes)?;
        if gamme == Gamme::V1 && postes != 1 {
            return refus("un code de la ligne V1 vaut un poste");
        }
        if monoposte && postes != 1 {
            return refus("un code de l'édition un poste vaut un poste");
        }
        let formule = valider_formule(formule)?;
        let modules: Vec<Module> = Module::TOUS
            .into_iter()
            .filter(|m| modules.contains(m))
            .collect();
        let transaction = conn.unchecked_transaction()?;
        let mut tires = Vec::new();
        for _ in 0..nombre {
            let (id, code, fin) = inserer_code(
                &transaction,
                &acteur,
                postes,
                formule,
                &modules,
                None,
                gamme,
                produit,
            )?;
            tires.push((id, code, fin));
        }
        let fins: Vec<&str> = tires.iter().map(|t| t.2.as_str()).collect();
        let details = format!(
            "{nombre} codes à vendre (…{}), gamme {}, édition {}, {postes} postes, formule {}, modules : {}",
            fins.join(", …"),
            gamme.code(),
            produit.code(),
            formule.code(),
            liste_modules(&modules)
        );
        noter(&transaction, Some(acteur.id), "codes_crees", None, &details)?;
        transaction.commit()?;
        tires
            .into_iter()
            .map(|(id, code, _)| {
                Ok(CodeCree {
                    code,
                    fiche: code_permis(&conn, &acteur, id)?,
                })
            })
            .collect()
    }

    /// Codes à vendre (sans paiement), du plus récent au plus ancien (500 au
    /// plus) : tous pour l'éditeur, les siens pour un revendeur.
    pub fn codes_a_vendre(&self, acteur: &Compte) -> Resultat<Vec<CodeActivation>> {
        let conn = self.conn();
        let acteur = relire(&conn, acteur)?;
        let filtre = match acteur.role {
            Role::Editeur => String::new(),
            Role::Revendeur => format!(" AND k.cree_par = {}", acteur.id),
        };
        let mut req = conn.prepare(&format!(
            "{SELECT_CODE} WHERE k.paiement IS NULL{filtre} ORDER BY k.id DESC LIMIT 500"
        ))?;
        let codes = req.query_map([], code_depuis)?.collect::<Result<_, _>>()?;
        Ok(codes)
    }

    /// Annule un code pas encore activé : l'éditeur, ou le compte qui l'a
    /// tiré. Le paiement d'un code annulé peut servir de nouveau.
    pub fn annuler_code(&self, acteur: &Compte, id: i64) -> Resultat<CodeActivation> {
        let conn = self.conn();
        let acteur = relire(&conn, acteur)?;
        let code = code_permis(&conn, &acteur, id)?;
        let createur: i64 = conn.query_row(
            "SELECT cree_par FROM code_activation WHERE id = ?1",
            [id],
            |r| r.get(0),
        )?;
        if acteur.role != Role::Editeur && createur != acteur.id {
            return Err(ErreurPortail::Interdit);
        }
        if code.etat != EtatCode::Libre {
            return refus("seul un code pas encore activé peut être annulé");
        }
        conn.execute(
            "UPDATE code_activation SET etat = 'annule' WHERE id = ?1",
            [id],
        )?;
        let details = format!(
            "code …{}{}",
            code.fin,
            code.paiement
                .map(|p| format!(", paiement {p} de nouveau libre"))
                .unwrap_or_default()
        );
        noter(
            &conn,
            Some(acteur.id),
            "code_annule",
            code.entreprise.as_deref(),
            &details,
        )?;
        code_permis(&conn, &acteur, id)
    }

    /// Code d'activation, dans la portée du compte.
    pub fn code(&self, acteur: &Compte, id: i64) -> Resultat<CodeActivation> {
        let conn = self.conn();
        let acteur = relire(&conn, acteur)?;
        code_permis(&conn, &acteur, id)
    }

    /// Libère le poste lié à un code V1 (« Libérer le poste », décision 75) :
    /// l'éditeur, ou le compte qui a tiré le code. Le code redevient
    /// activable sur un autre poste, avec la même échéance.
    pub fn liberer_poste(&self, acteur: &Compte, id: i64) -> Resultat<CodeActivation> {
        let conn = self.conn();
        let acteur = relire(&conn, acteur)?;
        let code = code_permis(&conn, &acteur, id)?;
        let createur: i64 = conn.query_row(
            "SELECT cree_par FROM code_activation WHERE id = ?1",
            [id],
            |r| r.get(0),
        )?;
        if acteur.role != Role::Editeur && createur != acteur.id {
            return Err(ErreurPortail::Interdit);
        }
        if code.gamme != Gamme::V1 {
            return refus("seul un code de la ligne V1 est lié à un poste");
        }
        let Some(poste) = &code.poste else {
            return refus("aucun poste n'est lié à ce code");
        };
        conn.execute(
            "UPDATE licence_v1 SET liberee_le = ?2, liberee_par = ?3
             WHERE code = ?1 AND liberee_le IS NULL",
            params![id, maintenant(), acteur.id],
        )?;
        noter(
            &conn,
            Some(acteur.id),
            "poste_libere",
            None,
            &format!("code …{}, poste {poste}", code.fin),
        )?;
        code_permis(&conn, &acteur, id)
    }

    /// Activation d'un code V1 pour un poste (décision 75) : même poste,
    /// même licence ; autre poste pendant que le code est lié, refus [6] ;
    /// code libre ou libéré, licence V1 datée signée par la clé du portail.
    /// Après une libération, l'échéance reste celle de la première activation
    /// (un changement de poste ne prolonge rien).
    fn activer_v1(
        &self,
        conn: &Connection,
        trouve: &CodeActivation,
        poste: &str,
        client: &str,
        demande: &DemandeActivation,
        adresse: &str,
    ) -> Resultat<Activation> {
        let en_cours: Option<(String, String, String, String)> = conn
            .query_row(
                "SELECT poste, texte, debut, echeance FROM licence_v1
                 WHERE code = ?1 AND liberee_le IS NULL",
                [trouve.id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        if let Some((lie, texte, debut, echeance)) = en_cours {
            if lie != poste {
                return Err(ErreurPortail::Validation(MODELES_ACTIVATION[6].into()));
            }
            noter(
                conn,
                None,
                "code_reactive",
                None,
                &format!("code …{}, poste {poste}, adresse {adresse}", trouve.fin),
            )?;
            return Ok(Activation {
                licence: texte,
                postes: 1,
                formule: trouve.formule,
                debut,
                echeance,
            });
        }
        let aujourdhui = self.aujourdhui();
        let premiere: Option<(String, String)> = conn
            .query_row(
                "SELECT debut, echeance FROM licence_v1 WHERE code = ?1 ORDER BY id LIMIT 1",
                [trouve.id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let (debut, echeance) = match premiere {
            Some(p) => p,
            None => {
                let veille = aujourdhui.pred_opt().unwrap_or(aujourdhui);
                (
                    aujourdhui.format("%Y-%m-%d").to_string(),
                    ajouter_periode(veille, trouve.formule)
                        .format("%Y-%m-%d")
                        .to_string(),
                )
            }
        };
        let licence = kadenz_licence::Licence {
            client: client.to_string(),
            poste: poste.to_string(),
            modules: trouve.modules.clone(),
            emise_le: aujourdhui.format("%Y-%m-%d").to_string(),
            echeance: Some(echeance.clone()),
            postes: 1,
        };
        let texte = kadenz_licence::emettre(&licence, &self.0.cle)
            .map_err(|e| ErreurPortail::Interne(e.modele.to_string()))?;
        let transaction = conn.unchecked_transaction()?;
        transaction.execute(
            "INSERT INTO licence_v1 (code, poste, client, texte, debut, echeance, emise_le)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                trouve.id,
                poste,
                client,
                texte,
                debut,
                echeance,
                maintenant()
            ],
        )?;
        transaction.execute(
            "UPDATE code_activation SET etat = 'active', active_le = coalesce(active_le, ?2)
             WHERE id = ?1",
            params![trouve.id, maintenant()],
        )?;
        noter(
            &transaction,
            None,
            "code_v1_active",
            None,
            &format!(
                "code …{}, poste {poste}, client {client}, échéance {echeance}, version {}, adresse {adresse}",
                trouve.fin,
                pour_journal(&demande.version, MAX_VERSION_JOURNAL)
            ),
        )?;
        transaction.commit()?;
        Ok(Activation {
            licence: texte,
            postes: 1,
            formule: trouve.formule,
            debut,
            echeance,
        })
    }

    /// Lien de téléchargement demandé depuis `adresse` (décision 77) : chaque
    /// lien compte dans le frein de l'adresse ; au-delà, l'attente restante en
    /// secondes. Le lien donné est noté au journal.
    pub fn demander_telechargement(&self, nom: &str, adresse: &str) -> Resultat<Option<u64>> {
        let conn = self.conn();
        let cles = frein::cles_telechargement(adresse);
        let maintenant = secondes();
        if let Some(attente) = frein::attente(&conn, &cles, maintenant)? {
            return Ok(Some(attente));
        }
        frein::echec(&conn, &cles, maintenant)?;
        noter(
            &conn,
            None,
            "telechargement",
            None,
            &format!("{nom}, adresse {adresse}"),
        )?;
        Ok(None)
    }

    /// Essai gratuit demandé par le programme (décision 78) : poste inconnu,
    /// l'essai commence aujourd'hui ; poste déjà connu pour cette édition, la
    /// date de début d'origine est rendue. Seul l'identifiant du poste sert,
    /// aucune donnée personnelle (décision de l'utilisateur du 2026-11-26).
    /// Chaque demande compte dans le frein de l'adresse.
    pub fn demander_essai(&self, demande: &DemandeEssai, adresse: &str) -> Resultat<EssaiAtteste> {
        let invalide = || ErreurPortail::Validation(MODELES_ESSAI[0].into());
        let produit = lic::Produit::depuis_code(demande.produit.trim()).ok_or_else(invalide)?;
        let poste = valider_poste(&demande.poste).map_err(|_| invalide())?;
        let conn = self.conn();
        let cles = frein::cles_essai(adresse);
        let instant = secondes();
        if let Some(attente) = frein::attente(&conn, &cles, instant)? {
            return Err(ErreurPortail::Freine(attente));
        }
        frein::echec(&conn, &cles, instant)?;
        let aujourdhui = self.aujourdhui().format("%Y-%m-%d").to_string();
        let transaction = conn.unchecked_transaction()?;
        let connu: Option<(i64, String)> = transaction
            .query_row(
                "SELECT id, debut FROM essai WHERE poste = ?1 AND produit = ?2",
                params![poste, produit.code()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let version = pour_journal(&demande.version, MAX_VERSION_JOURNAL);
        let debut = match connu {
            Some((id, debut)) => {
                transaction.execute(
                    "UPDATE essai SET vu_le = ?2 WHERE id = ?1",
                    params![id, maintenant()],
                )?;
                debut
            }
            None => {
                transaction.execute(
                    "INSERT INTO essai (poste, produit, debut, cree_le, vu_le)
                     VALUES (?1, ?2, ?3, ?4, ?4)",
                    params![poste, produit.code(), aujourdhui, maintenant()],
                )?;
                noter(
                    &transaction,
                    None,
                    "essai_demarre",
                    Some(&poste),
                    &format!(
                        "édition {}, version {version}, adresse {adresse}",
                        produit.code()
                    ),
                )?;
                aujourdhui.clone()
            }
        };
        let attestation = kadenz_licence::essai::emettre(
            &kadenz_licence::essai::Essai {
                poste,
                produit,
                debut: debut.clone(),
                emise_le: aujourdhui,
            },
            &self.0.cle,
        )
        .map_err(|e| ErreurPortail::Interne(e.to_string()))?;
        transaction.commit()?;
        Ok(EssaiAtteste { attestation, debut })
    }

    /// Activation demandée par le programme du client, sans compte connecté
    /// (route `/api/activation`). Code libre : la licence est émise, signée et
    /// le code passe à « activé » ; code déjà activé par la même entreprise :
    /// la même licence est rendue. Les refus sont freinés comme les connexions.
    /// État de licence relayé par un poste (décision 80,
    /// `docs/spec-revocation-licences.md`, section 5.2) : les postes reçus
    /// mettent à jour les appareils de l'entreprise, puis le portail rend le
    /// bulletin signé (révocation, postes à retirer, ordinateurs bloqués). Un
    /// appareil absent de l'état est noté retiré. Seuls les changements
    /// (appareil nouveau, appareil retiré) vont au journal, qui ne s'efface
    /// pas. Les échecs comptent dans le frein de l'adresse et de l'entreprise.
    pub fn etat_licence(&self, demande: &DemandeEtat, adresse: &str) -> Resultat<String> {
        let entreprise_valide = valider_uuid(&demande.entreprise).ok();
        let titulaire = entreprise_valide
            .as_deref()
            .map(frein::Titulaire::Entreprise);
        let cles = frein::cles_etat(titulaire, adresse);
        let conn = self.conn();
        let instant = secondes();
        if let Some(attente) = frein::attente(&conn, &cles, instant)? {
            return Err(ErreurPortail::Freine(attente));
        }
        let resultat = self.etat_sans_frein(&conn, demande, entreprise_valide, adresse);
        if matches!(resultat, Err(ErreurPortail::Validation(_))) {
            frein::echec(&conn, &cles, instant)?;
        }
        resultat
    }

    fn etat_sans_frein(
        &self,
        conn: &Connection,
        demande: &DemandeEtat,
        entreprise: Option<String>,
        adresse: &str,
    ) -> Resultat<String> {
        let invalide = || ErreurPortail::Validation(MODELES_ETAT[0].into());
        let entreprise = entreprise.ok_or_else(invalide)?;
        lic::Produit::depuis_code(demande.produit.trim()).ok_or_else(invalide)?;
        if demande.postes.len() > kadenz_licence::bulletin::MAX_LISTE {
            return Err(invalide());
        }
        let mut postes = Vec::with_capacity(demande.postes.len());
        for p in &demande.postes {
            postes.push(valider_poste_rapporte(p).ok_or_else(invalide)?);
        }
        let existe: bool = conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM entreprise WHERE identifiant = ?1)",
            [&entreprise],
            |r| r.get(0),
        )?;
        if !existe {
            return Err(ErreurPortail::Validation(MODELES_ETAT[1].into()));
        }
        let version = pour_journal(&demande.version, MAX_VERSION_JOURNAL);
        let instant = maintenant();
        let transaction = conn.unchecked_transaction()?;
        for p in &postes {
            let connu: bool = transaction.query_row(
                "SELECT EXISTS (SELECT 1 FROM appareil WHERE entreprise = ?1 AND empreinte = ?2)",
                params![entreprise, p.empreinte],
                |r| r.get(0),
            )?;
            if connu {
                transaction.execute(
                    "UPDATE appareil SET ordinateur = ?3, nom = ?4, inscrit_le = ?5, contact_le = ?6,
                         vu_le = ?7, retire_le = NULL
                     WHERE entreprise = ?1 AND empreinte = ?2",
                    params![
                        entreprise,
                        p.empreinte,
                        p.ordinateur,
                        p.nom,
                        p.inscrit_le,
                        p.vu_le,
                        instant
                    ],
                )?;
            } else {
                transaction.execute(
                    "INSERT INTO appareil (entreprise, empreinte, ordinateur, nom, inscrit_le, contact_le, vu_le)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        entreprise,
                        p.empreinte,
                        p.ordinateur,
                        p.nom,
                        p.inscrit_le,
                        p.vu_le,
                        instant
                    ],
                )?;
                noter(
                    &transaction,
                    None,
                    "appareil_nouveau",
                    Some(&entreprise),
                    &format!(
                        "{}, ordinateur {}, version {version}, adresse {adresse}",
                        pour_journal(&p.nom, 100),
                        if p.ordinateur.is_empty() {
                            "non donné"
                        } else {
                            &p.ordinateur
                        }
                    ),
                )?;
            }
        }
        // Appareils absents de l'état : retirés du serveur de l'entreprise.
        let rapportes: std::collections::HashSet<&str> =
            postes.iter().map(|p| p.empreinte.as_str()).collect();
        let presents: Vec<(i64, String, String)> = {
            let mut req = transaction.prepare(
                "SELECT id, empreinte, nom FROM appareil WHERE entreprise = ?1 AND retire_le IS NULL",
            )?;
            req.query_map([&entreprise], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
                .collect::<rusqlite::Result<_>>()?
        };
        for (id, empreinte, nom) in presents {
            if !rapportes.contains(empreinte.as_str()) {
                transaction.execute(
                    "UPDATE appareil SET retire_le = ?2 WHERE id = ?1",
                    params![id, instant],
                )?;
                noter(
                    &transaction,
                    None,
                    "appareil_retire",
                    Some(&entreprise),
                    &pour_journal(&nom, 100),
                )?;
            }
        }
        let bulletin = bulletin_de(&transaction, &entreprise, &instant)?;
        let texte = kadenz_licence::bulletin::emettre(&bulletin, &self.0.cle)
            .map_err(|e| ErreurPortail::Interne(e.modele.to_string()))?;
        transaction.commit()?;
        Ok(texte)
    }

    pub fn activer(&self, demande: &DemandeActivation, adresse: &str) -> Resultat<Activation> {
        let entreprise_demandee =
            pour_journal(&demande.entreprise, MAX_ENTREPRISE_JOURNAL).to_ascii_lowercase();
        let entreprise_valide = valider_uuid(&demande.entreprise.trim().to_ascii_lowercase()).ok();
        let poste_valide = valider_poste(&demande.poste).ok();
        let titulaire = match (&entreprise_valide, &poste_valide) {
            (Some(e), None) => Some(frein::Titulaire::Entreprise(e)),
            (None, Some(p)) => Some(frein::Titulaire::Poste(p)),
            _ => None,
        };
        let cles = frein::cles_activation(titulaire, adresse);
        let conn = self.conn();
        if let Some(attente) = frein::attente(&conn, &cles, secondes())? {
            let titulaire = match &poste_valide {
                Some(p) => format!("poste {p}"),
                None => format!("entreprise {entreprise_demandee}"),
            };
            noter_refus_freine(
                &conn,
                &cles,
                "activation_freinee",
                &format!("{titulaire}, adresse {adresse}"),
                attente,
            );
            return Err(ErreurPortail::Freine(attente));
        }
        let resultat = self.activer_sans_frein(&conn, demande, adresse);
        match &resultat {
            Ok(_) => frein::reussite(&conn, &cles)?,
            Err(ErreurPortail::Validation(modele)) => {
                frein::echec(&conn, &cles, secondes())?;
                let titulaire = match &poste_valide {
                    Some(p) => format!("poste {p}"),
                    None => format!("entreprise {entreprise_demandee}"),
                };
                let details = format!(
                    "{titulaire}, adresse {adresse}, version {} : {modele}",
                    pour_journal(&demande.version, MAX_VERSION_JOURNAL)
                );
                // Un journal en échec ne change pas la réponse : le refus reste un refus.
                let _ = noter(&conn, None, "activation_refusee", None, &details);
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
        // Refus du portail traduisibles par le client : un modèle fixe.
        let invalide = |_| ErreurPortail::Validation(MODELES_ACTIVATION[4].into());
        // Une demande porte une entreprise (gamme entreprise) ou un poste
        // (ligne V1), jamais les deux.
        let par_poste = !demande.poste.trim().is_empty();
        if par_poste == !demande.entreprise.trim().is_empty() {
            return Err(ErreurPortail::Validation(MODELES_ACTIVATION[4].into()));
        }
        let client = valider_texte(&demande.client, "nom du client", 200).map_err(invalide)?;
        let trouve = conn
            .query_row(
                &format!("{SELECT_CODE} WHERE k.empreinte = ?1"),
                [empreinte_code(&utiles)],
                code_depuis,
            )
            .optional()?
            .filter(|c| c.etat != EtatCode::Annule)
            .ok_or_else(inconnu)?;
        // Code d'une gamme présenté par un logiciel de l'autre : refus [5].
        let gamme_demandee = if par_poste {
            Gamme::V1
        } else {
            Gamme::Entreprise
        };
        if trouve.gamme != gamme_demandee {
            return Err(ErreurPortail::Validation(MODELES_ACTIVATION[5].into()));
        }
        // Édition du programme : l'édition un poste et l'édition Entreprise ne
        // s'activent jamais l'une avec le code de l'autre (décision du
        // 2026-11-26). Un programme antérieur n'envoie rien : c'est Entreprise.
        let produit_demande = match demande.produit.trim() {
            "" => lic::Produit::Entreprise,
            autre => lic::Produit::depuis_code(autre)
                .ok_or_else(|| ErreurPortail::Validation(MODELES_ACTIVATION[4].into()))?,
        };
        if !par_poste && trouve.produit != produit_demande {
            return Err(ErreurPortail::Validation(MODELES_ACTIVATION[5].into()));
        }
        if par_poste {
            let poste = valider_poste(&demande.poste).map_err(invalide)?;
            return self.activer_v1(conn, &trouve, &poste, &client, demande, adresse);
        }
        let identifiant = valider_uuid(&demande.entreprise).map_err(invalide)?;
        if revoquee(conn, &identifiant)? {
            return Err(ErreurPortail::Validation(MODELES_ACTIVATION[7].into()));
        }
        if trouve
            .entreprise
            .as_deref()
            .is_some_and(|e| e != identifiant)
        {
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
            noter(
                &transaction,
                None,
                "entreprise_creee",
                Some(&identifiant),
                &client,
            )?;
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
                produit: trouve.produit,
                entreprise: &entreprise,
                paiement: trouve.paiement,
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
            pour_journal(&demande.version, MAX_VERSION_JOURNAL)
        );
        noter(
            &transaction,
            None,
            "code_active",
            Some(&identifiant),
            &details,
        )?;
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

    /// Chaîne du journal vérifiée, pour l'éditeur (aucune pour un
    /// revendeur, qui ne voit qu'une partie du journal).
    pub fn chaine_du_journal(&self, acteur: &Compte) -> Resultat<Option<EtatChaine>> {
        let conn = self.conn();
        let acteur = relire(&conn, acteur)?;
        if acteur.role != Role::Editeur {
            return Ok(None);
        }
        Ok(Some(chaine::verifier(&conn)?))
    }

    /// Chaîne du journal vérifiée, sans compte (programme, en local sur le
    /// serveur).
    pub fn verifier_journal(&self) -> Resultat<EtatChaine> {
        Ok(chaine::verifier(&self.conn())?)
    }

    /// Export du journal (depuis la première entrée) et base vérifiés
    /// ensemble, avant un archivage (programme).
    pub fn verifier_journal_avec_export(&self, export: &[EntreeChainee]) -> Resultat<EtatChaine> {
        let conn = self.conn();
        // Une lecture cohérente de la base, même si le portail écrit.
        let transaction = conn.unchecked_transaction()?;
        Ok(chaine::verifier_avec_export(&transaction, export)?)
    }

    /// Retire de la base les entrées du journal jusqu'à `jusqua` (sceau :
    /// son empreinte), si toutes ont plus de `jours` jours (programme ; voir
    /// `chaine::archiver`). L'export doit avoir été vérifié avec la base
    /// juste avant.
    pub fn archiver_journal(&self, jusqua: i64, sceau: &str, jours: u32) -> Resultat<Archivage> {
        if jours == 0 {
            return Err(ErreurPortail::Validation(
                "durée de conservation du journal : un jour au moins".into(),
            ));
        }
        let limite = (Utc::now() - chrono::Duration::days(i64::from(jours)))
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        chaine::archiver(&self.conn(), jusqua, sceau, &limite)
    }

    /// Entrées du journal après l'identifiant `depuis`, avec leurs
    /// empreintes : export à copier hors du serveur (programme).
    pub fn exporter_journal(&self, depuis: i64) -> Resultat<Vec<EntreeChainee>> {
        Ok(chaine::entrees(&self.conn(), depuis)?)
    }

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
    hache: &str,
) -> Resultat<Compte> {
    let identifiant = valider_identifiant(identifiant)?.to_ascii_lowercase();
    let nom = valider_texte(nom, "nom", 120)?;
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
        params![identifiant, nom, role.code(), hache, maintenant()],
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

/// Tire un code et l'enregistre ; `lien` : paiement et entreprise d'un code
/// tiré pour un paiement, aucun pour un code à vendre. Rend l'identifiant, le
/// code en clair et sa fin.
#[allow(clippy::too_many_arguments)]
fn inserer_code(
    conn: &Connection,
    acteur: &Compte,
    postes: u32,
    formule: Formule,
    modules: &[Module],
    lien: Option<(i64, &str)>,
    gamme: Gamme,
    produit: lic::Produit,
) -> Resultat<(i64, String, String)> {
    let code = code_court::tirer().map_err(ErreurPortail::Interne)?;
    let utiles = code_court::normaliser(&code);
    let fin: String = utiles.chars().skip(code_court::LONGUEUR - 5).collect();
    conn.execute(
        "INSERT INTO code_activation (empreinte, fin, postes, formule, modules, paiement,
                                      entreprise, cree_par, cree_le, gamme, produit)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            empreinte_code(&utiles),
            fin,
            postes,
            formule.code(),
            modules_en_texte(modules),
            lien.map(|l| l.0),
            lien.map(|l| l.1),
            acteur.id,
            maintenant(),
            gamme.code(),
            produit.code(),
        ],
    )?;
    Ok((conn.last_insert_rowid(), code, fin))
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
