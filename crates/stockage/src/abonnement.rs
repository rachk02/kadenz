//! Abonnement de l'entreprise (phase 2, jalon F3 ; décisions 6, 20, 21, 25,
//! 26 et 29 ; spécification de synchronisation, section 7).
//!
//! Côté serveur : la licence d'abonnement est gardée dans les paramètres
//! système et vérifiée avec les clés du portail. La date retenue ne recule
//! jamais : c'est la plus récente de l'horloge du serveur, de la dernière date
//! vue et de la date d'émission de la licence. Reculer l'horloge est donc sans
//! effet ; une horloge avancée par erreur se rattrape en installant une
//! nouvelle licence. L'état (lecture seule, modules) est tenu dans des tables
//! temporaires de la connexion, comme les modules de la V1.
//!
//! Côté poste : le dernier état reçu du serveur est gardé dans la copie
//! locale, pour l'afficher hors connexion.

use chrono::NaiveDate;
use kadenz_domaine::message::MessageCompose;
use kadenz_domaine::module::Module;
use kadenz_domaine::role::Droit;
use kadenz_licence::VerifyingKey;
use kadenz_licence::entreprise::{self as lic, LicenceEntreprise};
use kadenz_protocole::abonnement::{
    self as proto, Abonnement, Avertissement, LicenceAbonnement, Refus, Statut,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::json;

use crate::audit::{self, Entree};
use crate::comptes::Utilisateur;
use crate::droits::exiger as exiger_droit;
use crate::{Erreur, Resultat, licence, maintenant, postes, synchro};

/// Message des opérations refusées pendant la lecture seule.
pub const MODELE_LECTURE_SEULE: &str = "abonnement échu : Kadenz est en lecture seule";

/// Refus d'une licence qui reculerait l'échéance de l'abonnement en cours
/// (décision du 2026-11-19) : date reçue, puis échéance actuelle.
pub const MODELE_RECUL_ECHEANCE: &str =
    "cette licence mènerait l'abonnement au {}, avant son échéance actuelle du {} : licence gardée";

const CLE: &str = "abonnement.cle";
/// Dernière date retenue (ne recule jamais).
const DATE_VUE: &str = "abonnement.date_vue";
/// Date d'émission de la licence la plus récente installée.
const EMISSION_VUE: &str = "abonnement.emission_vue";
/// Dernier état reçu du serveur, sur un poste.
const RECU: &str = "abonnement.recu";
/// Date de début de l'essai gratuit (installation sans licence).
const ESSAI_DEBUT: &str = "abonnement.essai_debut";
/// Marque d'un essai pas encore confirmé par le portail (décision 78) :
/// présente tant que le portail n'a pas rendu la date de début de ce poste.
/// Aucune connexion n'est exigée : l'essai continue sans confirmation, la
/// demande repart à chaque lancement (décision de l'utilisateur du 2026-11-26).
const ESSAI_A_CONFIRMER: &str = "abonnement.essai_a_confirmer";

/// Dernier bulletin du portail appliqué (décision 80) : texte, date
/// d'émission (UTC, à la seconde) et date retenue de son application.
const BULLETIN: &str = "abonnement.bulletin";
const BULLETIN_EMIS: &str = "abonnement.bulletin_emis";
const BULLETIN_VU: &str = "abonnement.bulletin_vu";
/// Licence révoquée par le dernier bulletin (« oui ») ; absente sinon.
const REVOQUEE: &str = "abonnement.revoquee";
/// Ordinateurs bloqués par le dernier bulletin, liste JSON.
const BLOQUES: &str = "abonnement.ordinateurs_bloques";
/// Départ du compte des jours sans contact avant le premier bulletin : la
/// première évaluation d'une licence par une version qui sait appliquer les
/// bulletins (mise à jour du serveur).
const CONTACT_DEPART: &str = "abonnement.contact_depart";

/// Jours sans bulletin du portail avant la lecture seule d'une licence
/// (décision de l'utilisateur du 2026-11-27).
pub const JOURS_SANS_CONTACT: i64 = 30;

/// Raison de la lecture seule d'une licence révoquée au portail.
pub const MODELE_REVOQUEE: &str = "licence révoquée par l'éditeur : contactez votre revendeur";
/// Raison de la lecture seule d'une licence sans bulletin depuis 30 jours.
pub const MODELE_NON_CONFIRMEE: &str = "licence non confirmée depuis 30 jours : reliez à Internet un poste à jour, puis relancez Kadenz";
/// Refus d'inscription d'un poste sur un ordinateur bloqué au portail.
pub const MODELE_ORDINATEUR_BLOQUE: &str =
    "cet ordinateur a été retiré de la licence : contactez votre revendeur";

/// Durée de l'essai gratuit, en jours (décision de l'utilisateur du 2026-11-25).
pub const DUREE_ESSAI_JOURS: i64 = 30;
/// Postes permis pendant l'essai : l'édition à plusieurs postes peut s'essayer à
/// plusieurs ; l'édition à un poste n'en inscrit de toute façon qu'un.
pub const POSTES_ESSAI: u32 = 5;

/// Démarre l'essai gratuit à l'installation sans licence. Sans effet si un essai a
/// déjà commencé : il ne se relance pas.
pub fn demarrer_essai(conn: &Connection, horloge: NaiveDate) -> Resultat<()> {
    if lire(conn, ESSAI_DEBUT)?.is_none() {
        ecrire(conn, ESSAI_DEBUT, &en_texte(horloge))?;
    }
    Ok(())
}

/// Note l'essai comme à confirmer par le portail (installation sans licence,
/// portail pas encore joint).
pub fn noter_essai_a_confirmer(conn: &Connection) -> Resultat<()> {
    ecrire(conn, ESSAI_A_CONFIRMER, "oui")
}

/// Vrai si l'essai attend encore la confirmation du portail.
pub fn essai_a_confirmer(conn: &Connection) -> Resultat<bool> {
    Ok(lire(conn, ESSAI_A_CONFIRMER)?.is_some_and(|e| !e.is_empty()))
}

/// Essai confirmé par le portail : la date de début du portail remplace celle
/// du poste si elle est plus ancienne (réinstallation), puis l'essai n'est plus
/// à confirmer.
pub fn confirmer_essai(conn: &Connection, debut_portail: NaiveDate) -> Resultat<()> {
    let local = lire(conn, ESSAI_DEBUT)?.as_deref().and_then(date);
    let debut = local.map_or(debut_portail, |l| l.min(debut_portail));
    ecrire(conn, ESSAI_DEBUT, &en_texte(debut))?;
    conn.execute(
        "DELETE FROM parametre_systeme WHERE cle = ?1",
        [ESSAI_A_CONFIRMER],
    )?;
    Ok(())
}

/// Table temporaire de l'état de la connexion ; vide, rien n'est bloqué (base
/// d'un poste, tests).
pub(crate) fn preparer(conn: &Connection) -> Resultat<()> {
    conn.execute(
        "CREATE TEMP TABLE IF NOT EXISTS abonnement_etat (
             id INTEGER PRIMARY KEY CHECK (id = 1),
             lecture_seule INTEGER NOT NULL,
             echeance TEXT
         )",
        [],
    )?;
    Ok(())
}

fn lire(conn: &Connection, cle: &str) -> Resultat<Option<String>> {
    let valeur: Option<String> = conn
        .query_row(
            "SELECT valeur FROM parametre_systeme WHERE cle = ?1",
            [cle],
            |r| r.get(0),
        )
        .optional()?;
    Ok(valeur.and_then(|v| match serde_json::from_str(&v) {
        Ok(serde_json::Value::String(s)) => Some(s),
        _ => None,
    }))
}

/// Licence d'entreprise en place sur le serveur (présentée au relais, F8).
pub fn licence(conn: &Connection) -> Resultat<Option<String>> {
    lire(conn, CLE)
}

fn ecrire(conn: &Connection, cle: &str, valeur: &str) -> Resultat<()> {
    conn.execute(
        "INSERT INTO parametre_systeme (cle, valeur) VALUES (?1, ?2)
         ON CONFLICT (cle) DO UPDATE SET valeur = excluded.valeur",
        params![cle, serde_json::Value::String(valeur.into()).to_string()],
    )?;
    Ok(())
}

fn effacer(conn: &Connection, cle: &str) -> Resultat<()> {
    conn.execute("DELETE FROM parametre_systeme WHERE cle = ?1", [cle])?;
    Ok(())
}

fn date(texte: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(texte, "%Y-%m-%d").ok()
}

fn en_texte(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

/// Restauration du serveur (F7) : reporte dans la base restaurée `cible` ce
/// qui ne doit pas revenir en arrière depuis la base courante, de la même
/// entreprise : la date retenue (la plus récente des deux) et la licence
/// installée la plus récente.
pub fn reporter(courante: &Connection, cible: &Connection) -> Resultat<()> {
    let vue_courante = lire(courante, DATE_VUE)?.as_deref().and_then(date);
    let vue_cible = lire(cible, DATE_VUE)?.as_deref().and_then(date);
    if let Some(vue) = vue_courante
        && vue_cible.is_none_or(|c| c < vue)
    {
        ecrire(cible, DATE_VUE, &en_texte(vue))?;
    }
    // Révocation (décision 80) : une sauvegarde plus ancienne ne lève ni une
    // révocation ni un blocage, et ne remet pas à zéro le compte des jours
    // sans contact.
    let bulletin_courant = lire(courante, BULLETIN_EMIS)?;
    let bulletin_cible = lire(cible, BULLETIN_EMIS)?;
    if let Some(emis) = &bulletin_courant
        && bulletin_cible.as_ref().is_none_or(|c| c < emis)
    {
        for cle in [BULLETIN, BULLETIN_EMIS, BULLETIN_VU, BLOQUES] {
            match lire(courante, cle)? {
                Some(v) => ecrire(cible, cle, &v)?,
                None => effacer(cible, cle)?,
            }
        }
        match lire(courante, REVOQUEE)? {
            Some(v) => ecrire(cible, REVOQUEE, &v)?,
            None => effacer(cible, REVOQUEE)?,
        }
    }
    let depart_courant = lire(courante, CONTACT_DEPART)?.as_deref().and_then(date);
    let depart_cible = lire(cible, CONTACT_DEPART)?.as_deref().and_then(date);
    if let Some(d) = depart_courant
        && depart_cible.is_none_or(|c| c > d)
    {
        ecrire(cible, CONTACT_DEPART, &en_texte(d))?;
    }
    let emission_courante = lire(courante, EMISSION_VUE)?.as_deref().and_then(date);
    let emission_cible = lire(cible, EMISSION_VUE)?.as_deref().and_then(date);
    if let (Some(emission), Some(cle)) = (emission_courante, lire(courante, CLE)?)
        && emission_cible.is_none_or(|c| c < emission)
    {
        ecrire(cible, CLE, &cle)?;
        ecrire(cible, EMISSION_VUE, &en_texte(emission))?;
    }
    Ok(())
}

/// Date du jour de l'horloge locale de la machine (fuseau de l'entreprise,
/// décision 37).
pub fn horloge() -> NaiveDate {
    chrono::Local::now().date_naive()
}

/// Vérifie une licence pour l'entreprise de cette base, dates du calendrier
/// comprises.
fn verifier(
    conn: &Connection,
    cle: &str,
    cles_portail: &[VerifyingKey],
) -> Resultat<Result<LicenceEntreprise, MessageCompose>> {
    let entreprise = synchro::identifiant_entreprise(conn)?;
    Ok(lic::verifier(cle, &entreprise, cles_portail).and_then(|l| {
        if [&l.debut, &l.echeance, &l.emise_le]
            .iter()
            .all(|d| date(d).is_some())
        {
            Ok(l)
        } else {
            Err(MessageCompose::new("clé de licence illisible", &[]))
        }
    }))
}

/// Date retenue, gardée : jamais avant la dernière date vue ni avant
/// l'émission de la licence en place.
fn date_retenue(
    conn: &Connection,
    horloge: NaiveDate,
    emission: Option<NaiveDate>,
) -> Resultat<NaiveDate> {
    let vue = lire(conn, DATE_VUE)?.as_deref().and_then(date);
    let retenue = [Some(horloge), vue, emission]
        .into_iter()
        .flatten()
        .max()
        .unwrap_or(horloge);
    if vue != Some(retenue) {
        ecrire(conn, DATE_VUE, &en_texte(retenue))?;
    }
    Ok(retenue)
}

fn vers_proto(l: &LicenceEntreprise) -> LicenceAbonnement {
    LicenceAbonnement {
        entreprise: l.entreprise.clone(),
        client: l.client.clone(),
        formule: match l.formule {
            lic::Formule::Mensuelle => proto::Formule::Mensuelle,
            lic::Formule::Annuelle => proto::Formule::Annuelle,
        },
        postes: l.postes,
        modules: l.modules.iter().map(|m| m.code().to_string()).collect(),
        debut: l.debut.clone(),
        echeance: l.echeance.clone(),
        emetteur: match &l.emetteur {
            lic::Emetteur::Editeur => proto::Emetteur::Editeur,
            lic::Emetteur::Revendeur { identifiant, nom } => proto::Emetteur::Revendeur {
                identifiant: identifiant.clone(),
                nom: nom.clone(),
            },
        },
        emise_le: l.emise_le.clone(),
    }
}

/// État de l'abonnement et modules de la licence en place.
fn evaluer(
    conn: &Connection,
    horloge: NaiveDate,
    cles_portail: &[VerifyingKey],
) -> Resultat<(Abonnement, Vec<Module>)> {
    let postes_utilises = postes::nombre_inscrits(conn)?;
    let verifiee = match lire(conn, CLE)?.filter(|c| !c.trim().is_empty()) {
        None => None,
        Some(cle) => Some(verifier(conn, &cle, cles_portail)?),
    };
    let emission = match &verifiee {
        Some(Ok(l)) => date(&l.emise_le),
        _ => None,
    };
    let aujourdhui = date_retenue(conn, horloge, emission)?;
    let mut abonnement = Abonnement {
        statut: Statut::Absent,
        aujourdhui: en_texte(aujourdhui),
        jours_restants: None,
        avertissement: None,
        postes_utilises,
        licence: None,
        erreur: None,
        licences: Vec::new(),
    };
    let modules = match verifiee {
        None => {
            // Sans licence : essai gratuit tant que ses 30 jours ne sont pas passés ;
            // ensuite l'état reste « absent » (lecture seule).
            let fin = lire(conn, ESSAI_DEBUT)?
                .as_deref()
                .and_then(date)
                .map(|d| d + chrono::Duration::days(DUREE_ESSAI_JOURS));
            match fin {
                Some(fin) if (fin - aujourdhui).num_days() >= 0 => {
                    let jours = (fin - aujourdhui).num_days();
                    abonnement.statut = Statut::Essai;
                    abonnement.jours_restants = Some(jours);
                    abonnement.avertissement = Avertissement::pour(jours);
                    Module::TOUS.to_vec()
                }
                _ => Vec::new(),
            }
        }
        Some(Err(m)) => {
            abonnement.statut = Statut::Invalide;
            abonnement.erreur = Some(Refus {
                modele: m.modele.to_string(),
                valeurs: m.valeurs,
            });
            Vec::new()
        }
        Some(Ok(l)) => {
            // Seule l'échéance compte (décision 6) ; le début est indicatif.
            let echeance = date(&l.echeance).unwrap_or(aujourdhui);
            let jours = (echeance - aujourdhui).num_days();
            abonnement.statut = if jours < 0 {
                Statut::Echu
            } else {
                Statut::Actif
            };
            abonnement.jours_restants = Some(jours);
            abonnement.avertissement = Avertissement::pour(jours);
            abonnement.licence = Some(vers_proto(&l));
            l.modules
        }
    };
    // Révocation au portail (décision 80) : lecture seule, avec sa raison,
    // quelle que soit la licence installée.
    let revoquee = lire(conn, REVOQUEE)?.is_some_and(|v| v == "oui");
    let refus = |modele: &str| Refus {
        modele: modele.to_string(),
        valeurs: Vec::new(),
    };
    if revoquee
        && matches!(
            abonnement.statut,
            Statut::Actif | Statut::Essai | Statut::Echu
        )
    {
        abonnement.statut = Statut::Invalide;
        abonnement.avertissement = None;
        abonnement.erreur = Some(refus(MODELE_REVOQUEE));
    } else if abonnement.statut == Statut::Actif {
        // Licence active : au plus 30 jours sans bulletin du portail. Le
        // compte part du dernier bulletin appliqué, ou de la mise à jour du
        // serveur avant le premier.
        let vu = lire(conn, BULLETIN_VU)?.as_deref().and_then(date);
        let depart = match lire(conn, CONTACT_DEPART)?.as_deref().and_then(date) {
            Some(d) => d,
            None => {
                ecrire(conn, CONTACT_DEPART, &en_texte(aujourdhui))?;
                aujourdhui
            }
        };
        let depuis = vu.map_or(depart, |v| v.max(depart));
        if (aujourdhui - depuis).num_days() > JOURS_SANS_CONTACT {
            abonnement.statut = Statut::Invalide;
            abonnement.avertissement = None;
            abonnement.erreur = Some(refus(MODELE_NON_CONFIRMEE));
        }
    }
    Ok((abonnement, modules))
}

/// Ajoute à l'état les licences installées sur ce serveur, lues au journal
/// d'audit (de la plus ancienne à la plus récente ; une même licence
/// réinstallée n'y figure qu'une fois). Seulement pour les réponses aux postes :
/// l'évaluation faite avant chaque opération ne lit pas le journal.
pub fn avec_licences(conn: &Connection, mut abonnement: Abonnement) -> Resultat<Abonnement> {
    let mut requete = conn.prepare(
        "SELECT nouvelle_valeur ->> '$.formule', nouvelle_valeur ->> '$.echeance', date_action
         FROM audit_log
         WHERE entite = 'parametres' AND action = 'modification_abonnement'
         ORDER BY id",
    )?;
    let lignes = requete
        .query_map([], |r| {
            Ok((
                r.get::<_, Option<String>>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut licences: Vec<proto::LicenceInstallee> = Vec::new();
    for (formule, echeance, installee_le) in lignes {
        let formule = match formule.as_deref() {
            Some("mensuelle") => proto::Formule::Mensuelle,
            Some("annuelle") => proto::Formule::Annuelle,
            _ => continue,
        };
        let Some(echeance) = echeance else { continue };
        if licences
            .iter()
            .any(|l| l.formule == formule && l.echeance == echeance)
        {
            continue;
        }
        licences.push(proto::LicenceInstallee {
            formule,
            echeance,
            installee_le,
        });
    }
    abonnement.licences = licences;
    Ok(abonnement)
}

/// Vrai si l'ordinateur est bloqué par le dernier bulletin appliqué.
pub fn ordinateur_bloque(conn: &Connection, ordinateur: &str) -> Resultat<bool> {
    let ordinateur = ordinateur.trim();
    if ordinateur.is_empty() {
        return Ok(false);
    }
    Ok(ordinateurs_bloques(conn)?
        .iter()
        .any(|o| o.eq_ignore_ascii_case(ordinateur)))
}

fn ordinateurs_bloques(conn: &Connection) -> Resultat<Vec<String>> {
    Ok(lire(conn, BLOQUES)?
        .and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or_default())
}

/// Date d'émission du dernier bulletin appliqué.
pub fn dernier_bulletin(conn: &Connection) -> Resultat<Option<String>> {
    lire(conn, BULLETIN_EMIS)
}

/// Applique un bulletin du portail (décision 80,
/// `docs/spec-revocation-licences.md`, section 4), relayé par un poste. Un
/// bulletin refusé (signature, autre entreprise) n'écrit rien ; un bulletin qui
/// n'est pas plus récent que le dernier appliqué est ignoré sans erreur.
/// Applique la révocation, retire les postes demandés et ceux des ordinateurs
/// bloqués, et remet à zéro le compte des jours sans contact.
pub fn appliquer_bulletin(
    conn: &Connection,
    texte: &str,
    horloge: NaiveDate,
    cles_portail: &[VerifyingKey],
) -> Resultat<Result<(), MessageCompose>> {
    let entreprise = synchro::identifiant_entreprise(conn)?;
    let bulletin = match kadenz_licence::bulletin::verifier(texte, &entreprise, cles_portail) {
        Ok(b) => b,
        Err(m) => return Ok(Err(m)),
    };
    if lire(conn, BULLETIN_EMIS)?.is_some_and(|e| bulletin.emis_le <= e) {
        return Ok(Ok(()));
    }
    let emission = lire(conn, EMISSION_VUE)?.as_deref().and_then(date);
    let aujourdhui = date_retenue(conn, horloge, emission)?;
    let tx = conn.unchecked_transaction()?;
    let texte: String = texte.chars().filter(|c| !c.is_whitespace()).collect();
    ecrire(&tx, BULLETIN, &texte)?;
    ecrire(&tx, BULLETIN_EMIS, &bulletin.emis_le)?;
    ecrire(&tx, BULLETIN_VU, &en_texte(aujourdhui))?;
    if bulletin.revoquee {
        ecrire(&tx, REVOQUEE, "oui")?;
    } else {
        effacer(&tx, REVOQUEE)?;
    }
    ecrire(
        &tx,
        BLOQUES,
        &serde_json::to_string(&bulletin.bloques).unwrap_or_else(|_| "[]".into()),
    )?;
    for empreinte in &bulletin.retires {
        postes::retirer(&tx, empreinte)?;
    }
    postes::retirer_ordinateurs(&tx, &bulletin.bloques)?;
    tx.commit()?;
    Ok(Ok(()))
}

/// État de l'abonnement, avec les clés du portail données.
pub fn etat_avec(
    conn: &Connection,
    horloge: NaiveDate,
    cles_portail: &[VerifyingKey],
) -> Resultat<Abonnement> {
    Ok(evaluer(conn, horloge, cles_portail)?.0)
}

/// État de l'abonnement, avec les clés du portail embarquées.
pub fn etat(conn: &Connection, horloge: NaiveDate) -> Resultat<Abonnement> {
    etat_avec(conn, horloge, &lic::cles_portail())
}

/// Évalue l'abonnement et l'applique à la connexion : lecture seule et
/// modules actifs. Le serveur l'appelle avant chaque opération, pour suivre le
/// changement de jour.
pub fn appliquer_avec(
    conn: &Connection,
    horloge: NaiveDate,
    cles_portail: &[VerifyingKey],
) -> Resultat<Abonnement> {
    let (abonnement, modules) = evaluer(conn, horloge, cles_portail)?;
    conn.execute("DELETE FROM temp.abonnement_etat", [])?;
    conn.execute(
        "INSERT INTO temp.abonnement_etat (id, lecture_seule, echeance) VALUES (1, ?1, ?2)",
        params![
            abonnement.lecture_seule(),
            abonnement.licence.as_ref().map(|l| l.echeance.clone())
        ],
    )?;
    licence::remplir(conn, &modules)?;
    Ok(abonnement)
}

/// Postes permis sans abonnement actif (sans licence, licence refusée ou
/// échue) : un seul, le temps d'installer la licence depuis ce poste
/// (décision 41).
pub const POSTES_SANS_ABONNEMENT_ACTIF: u32 = 1;

/// Nombre de postes que l'abonnement permet d'inscrire. Une licence qui
/// descend sous le nombre de postes déjà inscrits ne retire aucun poste : elle
/// bloque les nouvelles inscriptions (décision 42).
pub fn postes_permis(abonnement: &Abonnement) -> u32 {
    match (abonnement.statut, &abonnement.licence) {
        (Statut::Actif, Some(licence)) => licence.postes,
        (Statut::Essai, _) => POSTES_ESSAI,
        _ => POSTES_SANS_ABONNEMENT_ACTIF,
    }
}

/// Refuse une saisie ou une validation pendant la lecture seule.
pub(crate) fn exiger_ecriture(conn: &Connection) -> Resultat<()> {
    let etat: Option<(bool, Option<String>)> = conn
        .query_row(
            "SELECT lecture_seule, echeance FROM temp.abonnement_etat WHERE id = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    match etat {
        Some((true, echeance)) => Err(Erreur::LectureSeule { echeance }),
        _ => Ok(()),
    }
}

/// Installe une nouvelle licence d'abonnement, au droit « Gérer les
/// paramètres ». Une licence refusée n'est pas enregistrée : celle en place
/// reste. Une licence plus récente que toutes les précédentes ramène la date
/// retenue à l'horloge (horloge avancée par erreur) ; une licence plus
/// ancienne ne le fait pas, pour qu'on ne puisse pas revenir en arrière en
/// réinstallant une ancienne licence. Une licence dont l'échéance tombe avant
/// celle de l'abonnement en cours encore actif est refusée (décision du
/// 2026-11-19 : un ancien code mensuel ne remplace plus sans avertir une
/// licence annuelle) ; un abonnement échu, absent ou invalide n'est jamais
/// protégé.
pub fn enregistrer_avec(
    conn: &Connection,
    acteur: &Utilisateur,
    cle: &str,
    horloge: NaiveDate,
    cles_portail: &[VerifyingKey],
) -> Resultat<Abonnement> {
    exiger_droit(conn, acteur, Droit::GererParametres)?;
    let cle: String = cle.chars().filter(|c| !c.is_whitespace()).collect();
    let licence = verifier(conn, &cle, cles_portail)??;
    let (en_cours, _) = evaluer(conn, horloge, cles_portail)?;
    if let (Statut::Actif, Some(actuelle)) = (en_cours.statut, &en_cours.licence)
        && let (Some(actuelle), Some(recue)) = (date(&actuelle.echeance), date(&licence.echeance))
        && recue < actuelle
    {
        return Err(crate::refus(
            MODELE_RECUL_ECHEANCE,
            &[&recue.format("%d/%m/%Y"), &actuelle.format("%d/%m/%Y")],
        ));
    }
    let emission = date(&licence.emise_le).unwrap_or(horloge);
    let derniere = lire(conn, EMISSION_VUE)?.as_deref().and_then(date);
    let en_place = lire(conn, CLE)?;
    if derniere.is_none_or(|d| emission > d || (emission == d && en_place.as_deref() != Some(&cle)))
    {
        ecrire(conn, DATE_VUE, &en_texte(horloge.max(emission)))?;
        ecrire(conn, EMISSION_VUE, &licence.emise_le)?;
    }
    ecrire(conn, CLE, &cle)?;
    audit::journaliser(
        conn,
        Entree {
            entite: "parametres",
            entite_id: 0,
            action: "modification_abonnement",
            ancienne_valeur: None,
            nouvelle_valeur: Some(json!({
                "client": licence.client,
                "formule": licence.formule.code(),
                "postes": licence.postes,
                "debut": licence.debut,
                "echeance": licence.echeance,
                "emise_le": licence.emise_le,
            })),
            utilisateur_id: Some(acteur.id),
        },
    )?;
    appliquer_avec(conn, horloge, cles_portail)
}

/// Comme [`enregistrer_avec`], avec les clés du portail embarquées.
pub fn enregistrer(
    conn: &Connection,
    acteur: &Utilisateur,
    cle: &str,
    horloge: NaiveDate,
) -> Resultat<Abonnement> {
    enregistrer_avec(conn, acteur, cle, horloge, &lic::cles_portail())
}

/// Sur un poste : garde le dernier état reçu du serveur.
pub fn garder(conn: &Connection, abonnement: &Abonnement) -> Resultat<()> {
    let valeur = json!({ "abonnement": abonnement, "recu_le": maintenant() });
    ecrire(conn, RECU, &valeur.to_string())
}

/// Sur un poste : dernier état reçu du serveur et sa date de réception.
pub fn dernier_recu(conn: &Connection) -> Resultat<Option<(Abonnement, String)>> {
    let Some(texte) = lire(conn, RECU)? else {
        return Ok(None);
    };
    let valeur: serde_json::Value = match serde_json::from_str(&texte) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    let abonnement = serde_json::from_value(valeur["abonnement"].clone()).ok();
    let recu_le = valeur["recu_le"].as_str().map(str::to_owned);
    Ok(abonnement.zip(recu_le))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::droits::tests::compte;
    use crate::ouvrir_en_memoire;
    use kadenz_domaine::role::Role;
    use kadenz_licence::SigningKey;
    use kadenz_protocole::abonnement::Palier;

    fn cle_portail() -> SigningKey {
        SigningKey::from_bytes(&[3u8; 32])
    }

    fn j(texte: &str) -> NaiveDate {
        date(texte).unwrap()
    }

    fn licence(conn: &Connection, echeance: &str, emise_le: &str) -> String {
        lic::emettre(
            &LicenceEntreprise {
                produit: kadenz_licence::entreprise::Produit::Entreprise,
                entreprise: synchro::identifiant_entreprise(conn).unwrap(),
                client: "Entreprise Essai".into(),
                formule: lic::Formule::Mensuelle,
                postes: 3,
                modules: vec![Module::Devis],
                debut: emise_le.into(),
                echeance: echeance.into(),
                emetteur: lic::Emetteur::Editeur,
                emise_le: emise_le.into(),
            },
            &cle_portail(),
        )
        .unwrap()
    }

    /// Bulletin du portail pour l'entreprise de cette base (décision 80).
    fn bulletin(
        conn: &Connection,
        emis_le: &str,
        revoquee: bool,
        retires: &[&str],
        bloques: &[&str],
    ) -> String {
        kadenz_licence::bulletin::emettre(
            &kadenz_licence::bulletin::Bulletin {
                entreprise: synchro::identifiant_entreprise(conn).unwrap(),
                emis_le: emis_le.into(),
                revoquee,
                retires: retires.iter().map(|e| e.to_string()).collect(),
                bloques: bloques.iter().map(|e| e.to_string()).collect(),
            },
            &cle_portail(),
        )
        .unwrap()
    }

    /// Bulletin ordinaire appliqué ce jour-là : la licence est confirmée.
    fn confirmer(conn: &Connection, jour: &str) {
        let texte = bulletin(conn, &format!("{jour} 12:00:00"), false, &[], &[]);
        appliquer_bulletin(conn, &texte, j(jour), &[cle_portail().verifying_key()])
            .unwrap()
            .unwrap();
    }

    #[test]
    fn revocation_et_ordinateurs_bloques() {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        let cles = [cle_portail().verifying_key()];
        let cle = licence(&conn, "2027-11-30", "2026-11-27");
        enregistrer_avec(&conn, &admin, &cle, j("2026-11-27"), &cles).unwrap();
        postes::inscrire_sur(&conn, "aa", "Caisse", "ABCD-EFGH-JKMN-PQRS").unwrap();
        postes::inscrire_sur(&conn, "bb", "Bureau", "").unwrap();
        postes::inscrire_sur(&conn, "cc", "Stock", "0123-4567-89AB-CDEF").unwrap();

        // Bulletin d'une autre clé : refusé, rien n'est écrit.
        let autre = SigningKey::from_bytes(&[9u8; 32]);
        let faux = kadenz_licence::bulletin::emettre(
            &kadenz_licence::bulletin::verifier(
                &bulletin(&conn, "2026-11-27 10:00:00", true, &[], &[]),
                &synchro::identifiant_entreprise(&conn).unwrap(),
                &cles,
            )
            .unwrap(),
            &autre,
        )
        .unwrap();
        assert!(
            appliquer_bulletin(&conn, &faux, j("2026-11-27"), &cles)
                .unwrap()
                .is_err()
        );
        assert_eq!(dernier_bulletin(&conn).unwrap(), None);

        // Révocation, retrait du bureau, ordinateur de la caisse bloqué.
        let b = bulletin(
            &conn,
            "2026-11-27 10:00:00",
            true,
            &["bb"],
            &["ABCD-EFGH-JKMN-PQRS"],
        );
        appliquer_bulletin(&conn, &b, j("2026-11-27"), &cles)
            .unwrap()
            .unwrap();
        let a = appliquer_avec(&conn, j("2026-11-27"), &cles).unwrap();
        assert_eq!(a.statut, Statut::Invalide);
        assert!(a.lecture_seule());
        assert_eq!(a.erreur.unwrap().modele, MODELE_REVOQUEE);
        assert!(exiger_ecriture(&conn).is_err());
        let restants: Vec<String> = postes::liste(&conn)
            .unwrap()
            .into_iter()
            .map(|p| p.nom)
            .collect();
        assert_eq!(restants, ["Stock"]);
        assert!(ordinateur_bloque(&conn, "abcd-efgh-jkmn-pqrs").unwrap());
        assert!(!ordinateur_bloque(&conn, "").unwrap());

        // Réinstaller la licence ne lève pas la révocation.
        let a = enregistrer_avec(&conn, &admin, &cle, j("2026-11-28"), &cles).unwrap();
        assert_eq!(a.statut, Statut::Invalide);
        // Rejouer un bulletin plus ancien non plus : il est ignoré.
        let ancien = bulletin(&conn, "2026-11-27 09:00:00", false, &[], &[]);
        appliquer_bulletin(&conn, &ancien, j("2026-11-28"), &cles)
            .unwrap()
            .unwrap();
        assert_eq!(
            etat_avec(&conn, j("2026-11-28"), &cles).unwrap().statut,
            Statut::Invalide
        );
        // Un bulletin plus récent sans révocation la lève, et débloque.
        confirmer(&conn, "2026-11-28");
        let a = appliquer_avec(&conn, j("2026-11-28"), &cles).unwrap();
        assert_eq!(a.statut, Statut::Actif);
        assert!(!ordinateur_bloque(&conn, "ABCD-EFGH-JKMN-PQRS").unwrap());
    }

    #[test]
    fn trente_jours_sans_bulletin() {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        let cles = [cle_portail().verifying_key()];
        let cle = licence(&conn, "2027-11-30", "2026-11-27");
        // Le compte part de la première évaluation de la licence.
        enregistrer_avec(&conn, &admin, &cle, j("2026-11-27"), &cles).unwrap();
        assert_eq!(
            etat_avec(&conn, j("2026-12-27"), &cles).unwrap().statut,
            Statut::Actif
        );
        let a = etat_avec(&conn, j("2026-12-28"), &cles).unwrap();
        assert_eq!(a.statut, Statut::Invalide);
        assert_eq!(a.erreur.unwrap().modele, MODELE_NON_CONFIRMEE);
        // Un bulletin lève la lecture seule et relance le compte.
        confirmer(&conn, "2026-12-28");
        assert_eq!(
            etat_avec(&conn, j("2027-01-27"), &cles).unwrap().statut,
            Statut::Actif
        );
        assert_eq!(
            etat_avec(&conn, j("2027-01-28"), &cles).unwrap().statut,
            Statut::Invalide
        );

        // Sans licence, l'essai gratuit n'exige aucun contact (décision 78).
        let conn = ouvrir_en_memoire().unwrap();
        demarrer_essai(&conn, j("2026-11-01")).unwrap();
        assert_eq!(
            etat_avec(&conn, j("2026-11-29"), &cles).unwrap().statut,
            Statut::Essai
        );
    }

    #[test]
    fn restauration_garde_la_revocation() {
        let courante = ouvrir_en_memoire().unwrap();
        let cles = [cle_portail().verifying_key()];
        let admin = compte(&courante, "admin", Role::Admin);
        let cle = licence(&courante, "2027-11-30", "2026-11-27");
        enregistrer_avec(&courante, &admin, &cle, j("2026-11-27"), &cles).unwrap();
        // Sauvegarde ancienne, sans bulletin (le report suppose la même
        // entreprise, vérifiée par la restauration).
        let cible = ouvrir_en_memoire().unwrap();
        let b = bulletin(
            &courante,
            "2026-11-27 10:00:00",
            true,
            &[],
            &["ABCD-EFGH-JKMN-PQRS"],
        );
        appliquer_bulletin(&courante, &b, j("2026-11-27"), &cles)
            .unwrap()
            .unwrap();
        reporter(&courante, &cible).unwrap();
        assert_eq!(
            dernier_bulletin(&cible).unwrap().as_deref(),
            Some("2026-11-27 10:00:00")
        );
        assert!(ordinateur_bloque(&cible, "ABCD-EFGH-JKMN-PQRS").unwrap());
        assert_eq!(lire(&cible, REVOQUEE).unwrap().as_deref(), Some("oui"));
    }

    #[test]
    fn essai_a_confirmer_par_le_portail() {
        let conn = ouvrir_en_memoire().unwrap();
        demarrer_essai(&conn, j("2026-11-25")).unwrap();
        noter_essai_a_confirmer(&conn).unwrap();
        assert!(essai_a_confirmer(&conn).unwrap());
        // Sans confirmation, l'essai continue : aucune connexion n'est exigée.
        let a = etat_avec(&conn, j("2026-12-03"), &[]).unwrap();
        assert_eq!(a.statut, Statut::Essai);
        assert_eq!(a.jours_restants, Some(22));
        // Le portail confirme avec une date plus ancienne (réinstallation) :
        // elle est retenue, l'essai n'est plus à confirmer.
        confirmer_essai(&conn, j("2026-11-20")).unwrap();
        assert!(!essai_a_confirmer(&conn).unwrap());
        let a = etat_avec(&conn, j("2026-12-03"), &[]).unwrap();
        assert_eq!(a.statut, Statut::Essai);
        assert_eq!(a.jours_restants, Some(17));
        // Une date du portail plus récente ne rallonge pas l'essai.
        confirmer_essai(&conn, j("2026-12-01")).unwrap();
        assert_eq!(
            etat_avec(&conn, j("2026-12-03"), &[])
                .unwrap()
                .jours_restants,
            Some(17)
        );
    }

    #[test]
    fn essai_gratuit_de_trente_jours_sans_licence() {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        let cles = [cle_portail().verifying_key()];
        // Sans essai démarré : lecture seule, un poste.
        let a = appliquer_avec(&conn, j("2026-11-25"), &cles).unwrap();
        assert_eq!(a.statut, Statut::Absent);
        assert!(a.lecture_seule());
        // Essai démarré le 25 novembre : 30 jours, tous les modules, saisie permise.
        demarrer_essai(&conn, j("2026-11-25")).unwrap();
        let a = appliquer_avec(&conn, j("2026-11-25"), &cles).unwrap();
        assert_eq!(a.statut, Statut::Essai);
        assert_eq!(a.jours_restants, Some(30));
        assert!(!a.lecture_seule());
        assert_eq!(postes_permis(&a), POSTES_ESSAI);
        assert!(exiger_ecriture(&conn).is_ok());
        // Un second démarrage ne relance pas l'essai.
        demarrer_essai(&conn, j("2026-12-20")).unwrap();
        let a = appliquer_avec(&conn, j("2026-12-24"), &cles).unwrap();
        assert_eq!((a.statut, a.jours_restants), (Statut::Essai, Some(1)));
        // Dernier jour (le 25 décembre) encore permis, puis lecture seule.
        let a = appliquer_avec(&conn, j("2026-12-25"), &cles).unwrap();
        assert_eq!((a.statut, a.jours_restants), (Statut::Essai, Some(0)));
        let a = appliquer_avec(&conn, j("2026-12-26"), &cles).unwrap();
        assert_eq!(a.statut, Statut::Absent);
        assert!(a.lecture_seule());
        assert!(matches!(
            exiger_ecriture(&conn),
            Err(Erreur::LectureSeule { .. })
        ));
        // Reculer l'horloge ne rend pas l'essai : la date retenue ne recule pas.
        let a = appliquer_avec(&conn, j("2026-11-30"), &cles).unwrap();
        assert_eq!(a.statut, Statut::Absent);
        // Une licence prend le relais de l'essai.
        let cle = licence(&conn, "2027-12-31", "2026-12-26");
        let a = enregistrer_avec(&conn, &admin, &cle, j("2026-12-26"), &cles).unwrap();
        assert_eq!(a.statut, Statut::Actif);
    }

    #[test]
    fn echeance_avertissements_et_lecture_seule() {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        let cles = [cle_portail().verifying_key()];
        let a = appliquer_avec(&conn, j("2026-11-12"), &cles).unwrap();
        assert_eq!(a.statut, Statut::Absent);
        assert!(exiger_ecriture(&conn).is_err());

        let cle = licence(&conn, "2026-12-31", "2026-11-12");
        let a = enregistrer_avec(&conn, &admin, &cle, j("2026-11-12"), &cles).unwrap();
        assert_eq!(a.statut, Statut::Actif);
        assert_eq!(a.jours_restants, Some(49));
        assert_eq!(a.avertissement, None);
        assert!(exiger_ecriture(&conn).is_ok());
        assert!(licence::module_actif(&conn, Module::Devis).unwrap());
        assert!(!licence::module_actif(&conn, Module::Paiements).unwrap());

        let palier = |jour: &str| {
            appliquer_avec(&conn, j(jour), &cles)
                .unwrap()
                .avertissement
                .map(|a| (a.palier, a.tous_les_comptes))
        };
        assert_eq!(palier("2026-12-01"), Some((Palier::Trente, false)));
        confirmer(&conn, "2026-12-01");
        assert_eq!(palier("2026-12-24"), Some((Palier::Sept, true)));
        assert_eq!(palier("2026-12-30"), Some((Palier::Dernier, true)));
        assert_eq!(palier("2026-12-31"), Some((Palier::Dernier, true)));
        assert!(exiger_ecriture(&conn).is_ok());
        let a = appliquer_avec(&conn, j("2027-01-01"), &cles).unwrap();
        assert_eq!(a.statut, Statut::Echu);
        assert!(a.lecture_seule());
        assert!(matches!(
            exiger_ecriture(&conn),
            Err(Erreur::LectureSeule { echeance: Some(e) }) if e == "2026-12-31"
        ));
        // Modules gardés pour la consultation.
        assert!(licence::module_actif(&conn, Module::Devis).unwrap());
    }

    #[test]
    fn horloge_reculee_sans_effet_puis_rattrapage() {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        let cles = [cle_portail().verifying_key()];
        let ancienne = licence(&conn, "2026-12-31", "2026-11-12");
        enregistrer_avec(&conn, &admin, &ancienne, j("2026-11-12"), &cles).unwrap();
        assert_eq!(
            etat_avec(&conn, j("2027-01-05"), &cles).unwrap().statut,
            Statut::Echu
        );
        // Horloge reculée : la date retenue reste le 5 janvier.
        let a = etat_avec(&conn, j("2026-12-01"), &cles).unwrap();
        assert_eq!(a.statut, Statut::Echu);
        assert_eq!(a.aujourdhui, "2027-01-05");
        // Réinstaller l'ancienne licence ne fait pas revenir en arrière.
        let a = enregistrer_avec(&conn, &admin, &ancienne, j("2026-12-01"), &cles).unwrap();
        assert_eq!(a.statut, Statut::Echu);

        // Horloge avancée par erreur jusqu'en 2030...
        let nouvelle = licence(&conn, "2027-12-31", "2027-01-06");
        enregistrer_avec(&conn, &admin, &nouvelle, j("2027-01-06"), &cles).unwrap();
        assert_eq!(
            etat_avec(&conn, j("2030-01-01"), &cles).unwrap().statut,
            Statut::Echu
        );
        assert_eq!(
            etat_avec(&conn, j("2027-01-07"), &cles).unwrap().statut,
            Statut::Echu
        );
        // ... rattrapée par une licence plus récente, émise par l'éditeur,
        // la licence étant confirmée par le portail.
        confirmer(&conn, "2027-01-07");
        let rattrapage = licence(&conn, "2027-12-31", "2027-01-07");
        let a = enregistrer_avec(&conn, &admin, &rattrapage, j("2027-01-07"), &cles).unwrap();
        assert_eq!(a.statut, Statut::Actif);
        assert_eq!(a.aujourdhui, "2027-01-07");
    }

    /// Une licence reçue ne recule pas l'échéance d'un abonnement actif ;
    /// un abonnement échu n'est pas protégé.
    #[test]
    fn licence_plus_courte_refusee() {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        let cles = [cle_portail().verifying_key()];
        let annuelle = licence(&conn, "2027-11-18", "2026-11-19");
        enregistrer_avec(&conn, &admin, &annuelle, j("2026-11-19"), &cles).unwrap();
        let mensuelle = licence(&conn, "2026-12-18", "2026-11-20");
        match enregistrer_avec(&conn, &admin, &mensuelle, j("2026-11-20"), &cles) {
            Err(Erreur::ValidationComposee { modele, valeurs }) => {
                assert_eq!(modele, MODELE_RECUL_ECHEANCE);
                assert_eq!(valeurs, ["18/12/2026", "18/11/2027"]);
            }
            autre => panic!("{autre:?}"),
        }
        // Rien n'a changé.
        let a = etat_avec(&conn, j("2026-11-20"), &cles).unwrap();
        assert_eq!(a.licence.unwrap().echeance, "2027-11-18");
        // Même échéance (réinstallation) ou plus tardive : acceptée.
        enregistrer_avec(&conn, &admin, &annuelle, j("2026-11-20"), &cles).unwrap();
        let suivante = licence(&conn, "2028-11-18", "2026-11-21");
        let a = enregistrer_avec(&conn, &admin, &suivante, j("2026-11-21"), &cles).unwrap();
        assert_eq!(a.licence.unwrap().echeance, "2028-11-18");

        // Abonnement échu : jamais protégé.
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        let courte = licence(&conn, "2026-12-31", "2026-11-12");
        enregistrer_avec(&conn, &admin, &courte, j("2026-11-12"), &cles).unwrap();
        let plus_courte = licence(&conn, "2026-12-20", "2026-12-01");
        let a = enregistrer_avec(&conn, &admin, &plus_courte, j("2027-01-05"), &cles).unwrap();
        assert_eq!(a.licence.unwrap().echeance, "2026-12-20");
        assert_eq!(a.statut, Statut::Echu);
    }

    #[test]
    fn licence_refusee_et_droit() {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        let lecteur = compte(&conn, "lecture", Role::Consultation);
        let cles = [cle_portail().verifying_key()];
        let bonne = licence(&conn, "2026-12-31", "2026-11-12");
        assert!(matches!(
            enregistrer_avec(&conn, &lecteur, &bonne, j("2026-11-12"), &cles),
            Err(Erreur::Interdit(_))
        ));
        // Autre entreprise : refusée, rien d'enregistré.
        let autre = lic::emettre(
            &LicenceEntreprise {
                produit: kadenz_licence::entreprise::Produit::Entreprise,
                entreprise: "autre".into(),
                ..lic::verifier(
                    &bonne,
                    &synchro::identifiant_entreprise(&conn).unwrap(),
                    &cles,
                )
                .unwrap()
            },
            &cle_portail(),
        )
        .unwrap();
        match enregistrer_avec(&conn, &admin, &autre, j("2026-11-12"), &cles) {
            Err(Erreur::ValidationComposee { modele, .. }) => assert_eq!(
                modele,
                "licence d'abonnement émise pour une autre entreprise ({})"
            ),
            autre => panic!("{autre:?}"),
        }
        assert_eq!(
            etat_avec(&conn, j("2026-11-12"), &cles).unwrap().statut,
            Statut::Absent
        );
        // Licence enregistrée puis clés du portail changées : invalide.
        enregistrer_avec(&conn, &admin, &bonne, j("2026-11-12"), &cles).unwrap();
        let autre_cle = [SigningKey::from_bytes(&[4u8; 32]).verifying_key()];
        let a = etat_avec(&conn, j("2026-11-12"), &autre_cle).unwrap();
        assert_eq!(a.statut, Statut::Invalide);
        assert_eq!(
            a.erreur.unwrap().modele,
            "clé de signature de la licence inconnue : {}"
        );
    }

    #[test]
    fn dernier_etat_garde_sur_le_poste() {
        let conn = ouvrir_en_memoire().unwrap();
        assert!(dernier_recu(&conn).unwrap().is_none());
        let a = etat_avec(&conn, j("2026-11-12"), &[]).unwrap();
        garder(&conn, &a).unwrap();
        let (lu, recu_le) = dernier_recu(&conn).unwrap().unwrap();
        assert_eq!(lu, a);
        assert!(!recu_le.is_empty());
    }
}
