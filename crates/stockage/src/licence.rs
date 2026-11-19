//! Licence du poste et modules activés (cahier des charges, sections 3.2 et 15).
//!
//! La clé saisie est gardée dans les paramètres système. Les modules qu'elle
//! active, une fois sa signature et son poste vérifiés, sont tenus dans une
//! table temporaire de la connexion : jamais écrits dans le fichier de la base,
//! ils ne peuvent pas être forcés en modifiant ce fichier. Sans clé valide,
//! seul le socle est actif (décision de l'utilisateur du 2026-11-11) ; les
//! données des modules restent en base.
//!
//! Licences datées (décision 75, version 2, émises par le portail) : à
//! l'échéance, retour au socle comme sans licence, après des avertissements à
//! 30 jours, 7 jours et la veille. La date du jour retenue ne recule jamais
//! (dernière date vue, gardée dans les paramètres système), comme pour
//! l'abonnement du serveur dans la phase 2.

use kadenz_domaine::message::MessageCompose;
use kadenz_domaine::module::Module;
use kadenz_domaine::role::Droit;
use chrono::NaiveDate;
use kadenz_licence::Licence;
use kadenz_licence::VerifyingKey;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::json;

use crate::audit::{self, Entree};
use crate::comptes::Utilisateur;
use crate::droits::exiger as exiger_droit;
use crate::{Erreur, Resultat};

const CLE: &str = "licence.cle";
/// Dernière date retenue pour l'échéance (ne recule jamais).
const DATE_VUE: &str = "licence.date_vue";

/// État de la licence du poste.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EtatLicence {
    /// Aucune clé saisie : socle seul.
    Absente,
    /// Clé refusée (signature, autre poste...) : socle seul.
    Invalide(MessageCompose),
    /// Licence en cours, avec les jours restants jusqu'à l'échéance comprise
    /// (0 le dernier jour valable) ; `None` pour une licence perpétuelle.
    Valide(Licence, Option<i64>),
    /// Licence datée dont l'échéance est passée : socle seul, la licence
    /// (client, modules, échéance) reste lisible pour l'écran.
    Expiree(Licence),
}

/// Palier d'avertissement avant l'échéance d'une licence datée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Avertissement {
    Aucun,
    /// De 30 à 8 jours restants.
    Trente,
    /// De 7 à 2 jours restants.
    Sept,
    /// La veille et le dernier jour valable.
    Veille,
    /// Échéance passée.
    Expiree,
}

impl EtatLicence {
    /// Modules activés ; vide sans licence en cours.
    pub fn modules(&self) -> &[Module] {
        match self {
            EtatLicence::Valide(l, _) => &l.modules,
            _ => &[],
        }
    }

    /// Jours restants d'une licence datée en cours.
    pub fn jours_restants(&self) -> Option<i64> {
        match self {
            EtatLicence::Valide(_, jours) => *jours,
            _ => None,
        }
    }

    pub fn avertissement(&self) -> Avertissement {
        match self {
            EtatLicence::Expiree(_) => Avertissement::Expiree,
            EtatLicence::Valide(_, Some(jours)) => match jours {
                0..=1 => Avertissement::Veille,
                2..=7 => Avertissement::Sept,
                8..=30 => Avertissement::Trente,
                _ => Avertissement::Aucun,
            },
            _ => Avertissement::Aucun,
        }
    }
}

/// Date du jour de l'horloge de la machine.
pub fn horloge() -> NaiveDate {
    chrono::Local::now().date_naive()
}

fn date(texte: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(texte, "%Y-%m-%d").ok()
}

/// Date retenue, gardée : jamais avant la dernière date vue ni avant
/// l'émission de la licence en place.
fn date_retenue(
    conn: &Connection,
    horloge: NaiveDate,
    emission: Option<NaiveDate>,
) -> Resultat<NaiveDate> {
    let vue = lire_parametre(conn, DATE_VUE)?.as_deref().and_then(date);
    let retenue = [Some(horloge), vue, emission]
        .into_iter()
        .flatten()
        .max()
        .unwrap_or(horloge);
    if vue != Some(retenue) {
        ecrire_parametre(conn, DATE_VUE, &retenue.format("%Y-%m-%d").to_string())?;
    }
    Ok(retenue)
}

/// Licence vérifiée et datée : en cours ou expirée selon la date retenue.
fn situer(conn: &Connection, licence: Licence, horloge: NaiveDate) -> Resultat<EtatLicence> {
    let Some(echeance) = licence.echeance.as_deref().map(date) else {
        return Ok(EtatLicence::Valide(licence, None));
    };
    let Some(echeance) = echeance else {
        return Ok(EtatLicence::Invalide(MessageCompose::new(
            "clé de licence illisible",
            &[],
        )));
    };
    let aujourdhui = date_retenue(conn, horloge, date(&licence.emise_le))?;
    let jours = (echeance - aujourdhui).num_days();
    Ok(if jours < 0 {
        EtatLicence::Expiree(licence)
    } else {
        EtatLicence::Valide(licence, Some(jours))
    })
}

/// Clés permises : la clé V1 pour une licence perpétuelle, la clé du portail
/// pour une licence datée.
pub struct Cles {
    pub perpetuelles: Vec<VerifyingKey>,
    pub datees: Vec<VerifyingKey>,
}

impl Cles {
    /// Clés embarquées dans l'application.
    pub fn embarquees() -> Result<Cles, MessageCompose> {
        Ok(Cles {
            perpetuelles: vec![kadenz_licence::cle_publique()?],
            datees: kadenz_licence::cles_datees(),
        })
    }

    fn verifier(&self, cle: &str, poste: &str) -> Result<Licence, MessageCompose> {
        kadenz_licence::verifier_selon(cle, poste, &self.perpetuelles, &self.datees)
    }
}

/// Crée la table temporaire des modules actifs ; `tous` les active d'emblée
/// (base en mémoire des tests).
pub(crate) fn preparer(conn: &Connection, tous: bool) -> Resultat<()> {
    conn.execute(
        "CREATE TEMP TABLE IF NOT EXISTS module_actif (code TEXT PRIMARY KEY)",
        [],
    )?;
    if tous {
        remplir(conn, &Module::TOUS)?;
    }
    Ok(())
}

fn remplir(conn: &Connection, modules: &[Module]) -> Resultat<()> {
    conn.execute("DELETE FROM temp.module_actif", [])?;
    for m in modules {
        conn.execute(
            "INSERT INTO temp.module_actif (code) VALUES (?1)",
            [m.code()],
        )?;
    }
    Ok(())
}

/// Paramètre système ; la colonne `valeur` contient du JSON, le texte y est
/// une chaîne JSON.
fn lire_parametre(conn: &Connection, cle: &str) -> Resultat<Option<String>> {
    let valeur: Option<String> = conn
        .query_row(
            "SELECT valeur FROM parametre_systeme WHERE cle = ?1",
            [cle],
            |r| r.get(0),
        )
        .optional()?;
    Ok(
        valeur.map(|v| match serde_json::from_str::<serde_json::Value>(&v) {
            Ok(serde_json::Value::String(texte)) => texte,
            _ => String::new(),
        }),
    )
}

fn ecrire_parametre(conn: &Connection, cle: &str, texte: &str) -> Resultat<()> {
    conn.execute(
        "INSERT INTO parametre_systeme (cle, valeur) VALUES (?1, ?2)
         ON CONFLICT (cle) DO UPDATE SET valeur = excluded.valeur",
        params![cle, serde_json::Value::String(texte.to_owned()).to_string()],
    )?;
    Ok(())
}

/// État de la clé enregistrée pour ce poste, à la date du jour.
pub fn etat(conn: &Connection, poste: &str) -> Resultat<EtatLicence> {
    match Cles::embarquees() {
        Ok(cles) => etat_avec(conn, poste, &cles, horloge()),
        Err(e) => Ok(EtatLicence::Invalide(e)),
    }
}

/// État avec des clés et une date données.
pub fn etat_avec(
    conn: &Connection,
    poste: &str,
    cles: &Cles,
    horloge: NaiveDate,
) -> Resultat<EtatLicence> {
    match lire_parametre(conn, CLE)? {
        None => Ok(EtatLicence::Absente),
        Some(cle) if cle.trim().is_empty() => Ok(EtatLicence::Absente),
        Some(cle) => match cles.verifier(&cle, poste) {
            Ok(l) => situer(conn, l, horloge),
            Err(e) => Ok(EtatLicence::Invalide(e)),
        },
    }
}

/// Vérifie la clé enregistrée et active ses modules ; à appeler à l'ouverture
/// de la base (et après une restauration).
pub fn appliquer(conn: &Connection, poste: &str) -> Resultat<EtatLicence> {
    let e = etat(conn, poste)?;
    remplir(conn, e.modules())?;
    Ok(e)
}

/// Idem avec des clés et une date données.
pub fn appliquer_avec(
    conn: &Connection,
    poste: &str,
    cles: &Cles,
    horloge: NaiveDate,
) -> Resultat<EtatLicence> {
    let e = etat_avec(conn, poste, cles, horloge)?;
    remplir(conn, e.modules())?;
    Ok(e)
}

/// Remplace la clé de licence, au droit « Gérer les paramètres » ; une clé
/// refusée n'est pas enregistrée et la licence en place reste active.
pub fn enregistrer(
    conn: &Connection,
    acteur: &Utilisateur,
    cle: &str,
    poste: &str,
) -> Resultat<EtatLicence> {
    enregistrer_avec(conn, acteur, cle, poste, &Cles::embarquees()?, horloge())
}

/// Idem avec des clés et une date données. Une licence datée déjà expirée est
/// enregistrée (elle reste lisible) mais n'active aucun module.
pub fn enregistrer_avec(
    conn: &Connection,
    acteur: &Utilisateur,
    cle: &str,
    poste: &str,
    cles: &Cles,
    horloge: NaiveDate,
) -> Resultat<EtatLicence> {
    exiger_droit(conn, acteur, Droit::GererParametres)?;
    let cle: String = cle.chars().filter(|c| !c.is_whitespace()).collect();
    let licence = cles.verifier(&cle, poste)?;
    ecrire_parametre(conn, CLE, &cle)?;
    audit::journaliser(
        conn,
        Entree {
            entite: "parametres",
            entite_id: 0,
            action: "modification_licence",
            ancienne_valeur: None,
            nouvelle_valeur: Some(json!({
                "client": licence.client,
                "poste": licence.poste,
                "modules": licence.modules.iter().map(|m| m.code()).collect::<Vec<_>>(),
                "emise_le": licence.emise_le,
                "echeance": licence.echeance,
                "postes": licence.postes,
            })),
            utilisateur_id: Some(acteur.id),
        },
    )?;
    let e = situer(conn, licence, horloge)?;
    remplir(conn, e.modules())?;
    Ok(e)
}

/// Le module est-il activé sur cette connexion ?
pub fn module_actif(conn: &Connection, m: Module) -> Resultat<bool> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM temp.module_actif WHERE code = ?1",
            [m.code()],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

/// Refuse l'opération si le module n'est pas activé.
pub fn exiger(conn: &Connection, m: Module) -> Resultat<()> {
    if module_actif(conn, m)? {
        Ok(())
    } else {
        Err(Erreur::from(MessageCompose::new(
            "le module « {} » n'est pas activé par la licence de ce poste",
            &[&m.libelle()],
        )))
    }
}

/// Refuse l'opération si le type de document relève d'un module non activé.
pub(crate) fn exiger_type(
    conn: &Connection,
    t: kadenz_domaine::document::TypeDocument,
) -> Resultat<()> {
    match t.module() {
        Some(m) => exiger(conn, m),
        None => Ok(()),
    }
}

/// Idem pour un document existant ; un document introuvable passe (l'opération
/// signale elle-même qu'il n'existe pas).
pub(crate) fn exiger_document(conn: &Connection, id: i64) -> Resultat<()> {
    let code: Option<String> = conn
        .query_row("SELECT type FROM document WHERE id = ?1", [id], |r| {
            r.get(0)
        })
        .optional()?;
    match code.and_then(|c| kadenz_domaine::document::TypeDocument::depuis_code(&c)) {
        Some(t) => exiger_type(conn, t),
        None => Ok(()),
    }
}

/// Modèles d'erreur de la licence (crate de licence et refus des modules), pour
/// vérifier leurs traductions.
pub fn modeles_erreur() -> Vec<&'static str> {
    let mut m = kadenz_licence::MODELES_ERREUR.to_vec();
    m.push("le module « {} » n'est pas activé par la licence de ce poste");
    m
}

/// Active ou désactive les modules d'une base de test.
#[cfg(test)]
pub(crate) fn activer_pour_test(conn: &Connection, modules: &[Module]) {
    remplir(conn, modules).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::droits::tests::compte;
    use crate::ouvrir_en_memoire;
    use kadenz_domaine::role::Role;

    #[test]
    fn sans_cle_socle_seul_et_cle_refusee() {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        assert!(
            module_actif(&conn, Module::Devis).unwrap(),
            "tests : tout actif"
        );
        let e = appliquer(&conn, "AAAA-AAAA-AAAA-AAAA").unwrap();
        assert_eq!(e, EtatLicence::Absente);
        assert!(!module_actif(&conn, Module::Devis).unwrap());
        assert_eq!(
            exiger(&conn, Module::Paiements).unwrap_err().to_string(),
            "le module « Paiements » n'est pas activé par la licence de ce poste"
        );
        assert!(enregistrer(&conn, &admin, "KADENZ-faux.faux", "AAAA-AAAA-AAAA-AAAA").is_err());
        assert_eq!(
            etat(&conn, "AAAA-AAAA-AAAA-AAAA").unwrap(),
            EtatLicence::Absente,
            "clé refusée jamais enregistrée"
        );
    }

    #[test]
    fn operations_refusees_hors_licence() {
        use crate::documents::{Saisie, creer_brouillon};
        use kadenz_domaine::document::TypeDocument;
        let mut conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        activer_pour_test(&conn, &[Module::Paiements]);
        let s = Saisie::vide(chrono::NaiveDate::from_ymd_opt(2026, 11, 11).unwrap());
        let refus = creer_brouillon(&mut conn, &admin, TypeDocument::Devis, &s).unwrap_err();
        assert_eq!(
            refus.to_string(),
            "le module « Devis » n'est pas activé par la licence de ce poste"
        );
        // Le socle reste ouvert : le refus éventuel d'une facture vient de la saisie.
        let facture = creer_brouillon(&mut conn, &admin, TypeDocument::Facture, &s);
        assert!(!matches!(facture, Err(ref e) if e.to_string().contains("licence")));
        assert!(
            crate::pilotage::indicateurs(
                &conn,
                chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                chrono::NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()
            )
            .is_err()
        );
    }

    #[test]
    fn cle_valide_enregistree_puis_reappliquee() {
        use ed25519_dalek::SigningKey;
        let privee = SigningKey::from_bytes(&[3u8; 32]);
        let cles = Cles {
            perpetuelles: vec![privee.verifying_key()],
            datees: vec![],
        };
        let jour = NaiveDate::from_ymd_opt(2026, 11, 18).unwrap();
        let poste = "6VBZ-Z2E3-KFW9-N6QX";
        let cle = kadenz_licence::emettre(
            &Licence {
                client: "Client Essai".into(),
                poste: poste.into(),
                modules: vec![Module::Devis, Module::Paiements],
                emise_le: "2026-11-11".into(),
                echeance: None,
                postes: 1,
            },
            &privee,
        )
        .unwrap();
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        let vente = compte(&conn, "vente", Role::Commercial);
        assert!(enregistrer_avec(&conn, &vente, &cle, poste, &cles, jour).is_err());
        let e = enregistrer_avec(&conn, &admin, &cle, poste, &cles, jour).unwrap();
        assert_eq!(e.jours_restants(), None);
        assert_eq!(e.avertissement(), Avertissement::Aucun);
        assert_eq!(e.modules(), [Module::Devis, Module::Paiements]);
        assert!(module_actif(&conn, Module::Devis).unwrap());
        assert!(!module_actif(&conn, Module::Livraisons).unwrap());
        // Réouverture : la clé relue active les mêmes modules.
        remplir(&conn, &[]).unwrap();
        let relue = appliquer_avec(&conn, poste, &cles, jour).unwrap();
        assert_eq!(relue, e);
        assert!(module_actif(&conn, Module::Paiements).unwrap());
        // Autre poste : socle seul.
        let ailleurs = appliquer_avec(&conn, "AAAA-AAAA-AAAA-AAAA", &cles, jour).unwrap();
        assert!(matches!(ailleurs, EtatLicence::Invalide(_)));
        assert!(!module_actif(&conn, Module::Devis).unwrap());
    }

    fn licence_datee(privee: &ed25519_dalek::SigningKey, poste: &str, echeance: &str) -> String {
        kadenz_licence::emettre(
            &Licence {
                client: "Client Essai".into(),
                poste: poste.into(),
                modules: vec![Module::Devis, Module::Paiements],
                emise_le: "2026-11-18".into(),
                echeance: Some(echeance.into()),
                postes: 1,
            },
            privee,
        )
        .unwrap()
    }

    #[test]
    fn echeance_avertissements_puis_socle() {
        use ed25519_dalek::SigningKey;
        let v1 = SigningKey::from_bytes(&[3u8; 32]);
        let portail = SigningKey::from_bytes(&[4u8; 32]);
        let cles = Cles {
            perpetuelles: vec![v1.verifying_key()],
            datees: vec![portail.verifying_key()],
        };
        let poste = "6VBZ-Z2E3-KFW9-N6QX";
        let j = |t: &str| NaiveDate::parse_from_str(t, "%Y-%m-%d").unwrap();
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        // Datée signée par la clé V1 : refusée, rien d'enregistré.
        let mauvaise = licence_datee(&v1, poste, "2027-11-17");
        assert!(enregistrer_avec(&conn, &admin, &mauvaise, poste, &cles, j("2026-11-18")).is_err());
        let cle = licence_datee(&portail, poste, "2026-12-31");
        let e = enregistrer_avec(&conn, &admin, &cle, poste, &cles, j("2026-11-18")).unwrap();
        assert_eq!(e.jours_restants(), Some(43));
        assert_eq!(e.avertissement(), Avertissement::Aucun);
        for (jour, palier) in [
            ("2026-12-01", Avertissement::Trente),
            ("2026-12-24", Avertissement::Sept),
            ("2026-12-30", Avertissement::Veille),
            ("2026-12-31", Avertissement::Veille),
        ] {
            let e = appliquer_avec(&conn, poste, &cles, j(jour)).unwrap();
            assert_eq!(e.avertissement(), palier, "{jour}");
            assert!(module_actif(&conn, Module::Devis).unwrap(), "{jour}");
        }
        // Lendemain de l'échéance : socle seul, licence encore lisible.
        let e = appliquer_avec(&conn, poste, &cles, j("2027-01-01")).unwrap();
        assert!(matches!(&e, EtatLicence::Expiree(l) if l.modules == [Module::Devis, Module::Paiements]));
        assert_eq!(e.avertissement(), Avertissement::Expiree);
        assert!(!module_actif(&conn, Module::Devis).unwrap());
        // Horloge reculée : la date retenue ne recule pas.
        let e = appliquer_avec(&conn, poste, &cles, j("2026-11-20")).unwrap();
        assert!(matches!(e, EtatLicence::Expiree(_)));
        assert!(!module_actif(&conn, Module::Devis).unwrap());
        // Renouvellement : nouvelle licence datée, modules rouverts.
        let suivante = licence_datee(&portail, poste, "2027-12-31");
        let e = enregistrer_avec(&conn, &admin, &suivante, poste, &cles, j("2027-01-02")).unwrap();
        assert_eq!(e.avertissement(), Avertissement::Aucun);
        assert!(module_actif(&conn, Module::Devis).unwrap());
    }
}
