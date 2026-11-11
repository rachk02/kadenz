//! Licence du poste et modules activés (cahier des charges, sections 3.2 et 15).
//!
//! La clé saisie est gardée dans les paramètres système. Les modules qu'elle
//! active, une fois sa signature et son poste vérifiés, sont tenus dans une
//! table temporaire de la connexion : jamais écrits dans le fichier de la base,
//! ils ne peuvent pas être forcés en modifiant ce fichier. Sans clé valide,
//! seul le socle est actif (décision de l'utilisateur du 2026-11-11) ; les
//! données des modules restent en base.

use kadenz_domaine::message::MessageCompose;
use kadenz_domaine::module::Module;
use kadenz_domaine::role::Droit;
use kadenz_licence::Licence;
use kadenz_licence::VerifyingKey;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::json;

use crate::audit::{self, Entree};
use crate::comptes::Utilisateur;
use crate::droits::exiger as exiger_droit;
use crate::{Erreur, Resultat};

const CLE: &str = "licence.cle";

/// État de la licence du poste.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EtatLicence {
    /// Aucune clé saisie : socle seul.
    Absente,
    /// Clé refusée (signature, autre poste...) : socle seul.
    Invalide(MessageCompose),
    Valide(Licence),
}

impl EtatLicence {
    /// Modules activés ; vide sans licence valide.
    pub fn modules(&self) -> &[Module] {
        match self {
            EtatLicence::Valide(l) => &l.modules,
            _ => &[],
        }
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

/// Clé enregistrée ; la colonne `valeur` contient du JSON, la clé y est une
/// chaîne JSON.
fn cle_enregistree(conn: &Connection) -> Resultat<Option<String>> {
    let valeur: Option<String> = conn
        .query_row(
            "SELECT valeur FROM parametre_systeme WHERE cle = ?1",
            [CLE],
            |r| r.get(0),
        )
        .optional()?;
    Ok(
        valeur.map(|v| match serde_json::from_str::<serde_json::Value>(&v) {
            Ok(serde_json::Value::String(cle)) => cle,
            _ => String::new(),
        }),
    )
}

/// État de la clé enregistrée pour ce poste.
pub fn etat(conn: &Connection, poste: &str) -> Resultat<EtatLicence> {
    let publique = match kadenz_licence::cle_publique() {
        Ok(p) => p,
        Err(e) => return Ok(EtatLicence::Invalide(e)),
    };
    etat_avec(conn, poste, &publique)
}

fn etat_avec(conn: &Connection, poste: &str, publique: &VerifyingKey) -> Resultat<EtatLicence> {
    Ok(match cle_enregistree(conn)? {
        None => EtatLicence::Absente,
        Some(cle) if cle.trim().is_empty() => EtatLicence::Absente,
        Some(cle) => match kadenz_licence::verifier_avec(&cle, poste, publique) {
            Ok(l) => EtatLicence::Valide(l),
            Err(e) => EtatLicence::Invalide(e),
        },
    })
}

/// Vérifie la clé enregistrée et active ses modules ; à appeler à l'ouverture
/// de la base (et après une restauration).
pub fn appliquer(conn: &Connection, poste: &str) -> Resultat<EtatLicence> {
    let e = etat(conn, poste)?;
    remplir(conn, e.modules())?;
    Ok(e)
}

#[cfg(test)]
fn appliquer_avec(
    conn: &Connection,
    poste: &str,
    publique: &VerifyingKey,
) -> Resultat<EtatLicence> {
    let e = etat_avec(conn, poste, publique)?;
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
    enregistrer_avec(conn, acteur, cle, poste, &kadenz_licence::cle_publique()?)
}

fn enregistrer_avec(
    conn: &Connection,
    acteur: &Utilisateur,
    cle: &str,
    poste: &str,
    publique: &VerifyingKey,
) -> Resultat<EtatLicence> {
    exiger_droit(conn, acteur, Droit::GererParametres)?;
    let cle: String = cle.chars().filter(|c| !c.is_whitespace()).collect();
    let licence = kadenz_licence::verifier_avec(&cle, poste, publique)?;
    conn.execute(
        "INSERT INTO parametre_systeme (cle, valeur) VALUES (?1, ?2)
         ON CONFLICT (cle) DO UPDATE SET valeur = excluded.valeur",
        params![CLE, serde_json::Value::String(cle).to_string()],
    )?;
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
            })),
            utilisateur_id: Some(acteur.id),
        },
    )?;
    remplir(conn, &licence.modules)?;
    Ok(EtatLicence::Valide(licence))
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
        let publique = privee.verifying_key();
        let poste = "6VBZ-Z2E3-KFW9-N6QX";
        let cle = kadenz_licence::emettre(
            &Licence {
                client: "Client Essai".into(),
                poste: poste.into(),
                modules: vec![Module::Devis, Module::Paiements],
                emise_le: "2026-11-11".into(),
            },
            &privee,
        )
        .unwrap();
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        let vente = compte(&conn, "vente", Role::Commercial);
        assert!(enregistrer_avec(&conn, &vente, &cle, poste, &publique).is_err());
        let e = enregistrer_avec(&conn, &admin, &cle, poste, &publique).unwrap();
        assert_eq!(e.modules(), [Module::Devis, Module::Paiements]);
        assert!(module_actif(&conn, Module::Devis).unwrap());
        assert!(!module_actif(&conn, Module::Livraisons).unwrap());
        // Réouverture : la clé relue active les mêmes modules.
        remplir(&conn, &[]).unwrap();
        let relue = appliquer_avec(&conn, poste, &publique).unwrap();
        assert_eq!(relue, e);
        assert!(module_actif(&conn, Module::Paiements).unwrap());
        // Autre poste : socle seul.
        let ailleurs = appliquer_avec(&conn, "AAAA-AAAA-AAAA-AAAA", &publique).unwrap();
        assert!(matches!(ailleurs, EtatLicence::Invalide(_)));
        assert!(!module_actif(&conn, Module::Devis).unwrap());
    }
}
