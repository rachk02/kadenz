//! Accès à distance par le relais (phase 2, jalon F8 ; décisions 16, 57) :
//! sur le serveur, activation par l'Admin, relais réglé dans la fenêtre du
//! serveur, case « accès à distance » de chaque poste inscrit ; sur un poste,
//! adresse du relais apprise du serveur. Clés propres à chaque base, jamais
//! transmises (`replique::CLES_LOCALES`).

use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use crate::audit::{self, Entree};
use crate::{Erreur, Resultat};

/// Serveur : accès à distance activé par l'Admin.
pub const CLE_ACTIF: &str = "acces_distant.actif";
/// Serveur : adresse et empreinte du relais, réglées dans la fenêtre.
pub const CLE_RELAIS: &str = "acces_distant.relais";
/// Poste : adresse du relais apprise du serveur.
pub const CLE_RELAIS_DU_POSTE: &str = "acces_distant.relais_du_poste";
/// Serveur : début de la dernière session distante, gardé à travers les
/// arrêts du service (rubrique « Accès à distance »).
pub const CLE_DERNIERE_DISTANTE: &str = "acces_distant.derniere_distante";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relais {
    /// `hôte:port`.
    pub adresse: String,
    /// Empreinte SHA-256 du certificat du relais, en hexadécimal minuscule.
    pub empreinte: String,
}

fn lire(conn: &Connection, cle: &str) -> Resultat<Option<Value>> {
    let valeur: Option<String> = conn
        .query_row(
            "SELECT valeur FROM parametre_systeme WHERE cle = ?1",
            [cle],
            |r| r.get(0),
        )
        .optional()?;
    Ok(valeur.and_then(|v| serde_json::from_str(&v).ok()))
}

fn ecrire(conn: &Connection, cle: &str, valeur: Option<Value>) -> Resultat<()> {
    match valeur {
        Some(v) => conn.execute(
            "INSERT INTO parametre_systeme (cle, valeur) VALUES (?1, ?2)
             ON CONFLICT (cle) DO UPDATE SET valeur = excluded.valeur",
            params![cle, v.to_string()],
        )?,
        None => conn.execute("DELETE FROM parametre_systeme WHERE cle = ?1", [cle])?,
    };
    Ok(())
}

pub fn actif(conn: &Connection) -> Resultat<bool> {
    Ok(lire(conn, CLE_ACTIF)?
        .and_then(|v| v.as_bool())
        .unwrap_or(false))
}

/// Active ou désactive l'accès à distance ; noté au journal d'audit.
pub fn activer(conn: &Connection, actif: bool, acteur: Option<i64>) -> Resultat<()> {
    let avant = self::actif(conn)?;
    ecrire(conn, CLE_ACTIF, Some(Value::Bool(actif)))?;
    if avant != actif {
        audit::journaliser(
            conn,
            Entree {
                entite: "parametres",
                entite_id: 0,
                action: "modification_acces_distant",
                ancienne_valeur: Some(json!({ "actif": avant })),
                nouvelle_valeur: Some(json!({ "actif": actif })),
                utilisateur_id: acteur,
            },
        )?;
    }
    Ok(())
}

pub fn relais(conn: &Connection) -> Resultat<Option<Relais>> {
    Ok(lire(conn, CLE_RELAIS)?.and_then(|v| {
        Some(Relais {
            adresse: v["adresse"].as_str()?.to_string(),
            empreinte: v["empreinte"].as_str()?.to_string(),
        })
    }))
}

/// Règle le relais (fenêtre du serveur) ; `None` l'efface. La forme est
/// contrôlée par l'appelant (`kadenz_serveur::admin::regler_relais`).
pub fn regler_relais(conn: &Connection, relais: Option<&Relais>) -> Resultat<()> {
    ecrire(
        conn,
        CLE_RELAIS,
        relais.map(|r| json!({ "adresse": r.adresse, "empreinte": r.empreinte })),
    )
}

/// Coche ou décoche « accès à distance » pour un poste inscrit (décision
/// 57) ; noté au journal d'audit. Poste inconnu ou retiré : erreur.
pub fn autoriser(
    conn: &Connection,
    empreinte: &str,
    autorise: bool,
    acteur: Option<i64>,
) -> Resultat<()> {
    let avant: Option<(String, bool)> = conn
        .query_row(
            "SELECT nom, distance FROM poste_inscrit WHERE empreinte = ?1 AND retire_le IS NULL",
            [empreinte],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((nom, avant)) = avant else {
        return Err(Erreur::Validation("poste inconnu".into()));
    };
    if avant == autorise {
        return Ok(());
    }
    conn.execute(
        "UPDATE poste_inscrit SET distance = ?2 WHERE empreinte = ?1",
        params![empreinte, autorise],
    )?;
    audit::journaliser(
        conn,
        Entree {
            entite: "parametres",
            entite_id: 0,
            action: "modification_poste_distance",
            ancienne_valeur: Some(json!({ "poste": nom, "autorise": avant })),
            nouvelle_valeur: Some(json!({ "poste": nom, "autorise": autorise })),
            utilisateur_id: acteur,
        },
    )
}

/// Postes inscrits et non retirés : empreinte, nom, autorisé à distance.
pub fn postes(conn: &Connection) -> Resultat<Vec<(String, String, bool)>> {
    let mut requete = conn.prepare(
        "SELECT empreinte, nom, distance FROM poste_inscrit
         WHERE retire_le IS NULL ORDER BY nom, inscrit_le",
    )?;
    let postes = requete
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    Ok(postes)
}

/// Empreintes des postes autorisés à distance (liste envoyée au relais).
pub fn autorises(conn: &Connection) -> Resultat<Vec<String>> {
    Ok(postes(conn)?
        .into_iter()
        .filter(|(_, _, autorise)| *autorise)
        .map(|(empreinte, ..)| empreinte)
        .collect())
}

/// Poste : adresse du relais apprise du serveur.
pub fn relais_du_poste(conn: &Connection) -> Resultat<Option<String>> {
    Ok(lire(conn, CLE_RELAIS_DU_POSTE)?.and_then(|v| v.as_str().map(str::to_owned)))
}

/// Serveur : début de la dernière session distante, `AAAA-MM-JJTHH:MM`.
pub fn derniere_distante(conn: &Connection) -> Resultat<Option<String>> {
    Ok(lire(conn, CLE_DERNIERE_DISTANTE)?.and_then(|v| v.as_str().map(str::to_owned)))
}

/// Serveur : note le début d'une session distante.
pub fn noter_derniere_distante(conn: &Connection, debut: &str) -> Resultat<()> {
    ecrire(
        conn,
        CLE_DERNIERE_DISTANTE,
        Some(Value::String(debut.into())),
    )
}

/// Poste : garde l'adresse du relais reçue dans `Bienvenue` (`None` :
/// le poste n'est pas autorisé à distance, ou l'accès est désactivé).
pub fn garder_relais_du_poste(conn: &Connection, relais: Option<&str>) -> Resultat<()> {
    ecrire(
        conn,
        CLE_RELAIS_DU_POSTE,
        relais.map(|r| Value::String(r.into())),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reglages_et_cases() {
        let conn = crate::ouvrir_en_memoire().unwrap();
        assert!(!actif(&conn).unwrap());
        assert_eq!(relais(&conn).unwrap(), None);
        activer(&conn, true, None).unwrap();
        assert!(actif(&conn).unwrap());
        let r = Relais {
            adresse: "relais.exemple:47381".into(),
            empreinte: "ab".repeat(32),
        };
        regler_relais(&conn, Some(&r)).unwrap();
        assert_eq!(relais(&conn).unwrap(), Some(r));
        regler_relais(&conn, None).unwrap();
        assert_eq!(relais(&conn).unwrap(), None);

        crate::postes::inscrire(&conn, "p1", "Caisse").unwrap();
        crate::postes::inscrire(&conn, "p2", "Bureau").unwrap();
        assert!(autorises(&conn).unwrap().is_empty());
        autoriser(&conn, "p1", true, None).unwrap();
        assert_eq!(autorises(&conn).unwrap(), vec!["p1".to_string()]);
        assert_eq!(
            postes(&conn).unwrap(),
            vec![
                ("p2".to_string(), "Bureau".to_string(), false),
                ("p1".to_string(), "Caisse".to_string(), true)
            ]
        );
        assert!(autoriser(&conn, "inconnu", true, None).is_err());
        crate::postes::retirer(&conn, "p1").unwrap();
        assert!(autorises(&conn).unwrap().is_empty());

        assert_eq!(relais_du_poste(&conn).unwrap(), None);
        garder_relais_du_poste(&conn, Some("relais.exemple:47381")).unwrap();
        assert_eq!(
            relais_du_poste(&conn).unwrap().as_deref(),
            Some("relais.exemple:47381")
        );
    }
}
