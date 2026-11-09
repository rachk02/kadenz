//! Journal d'audit en ajout seul (cahier des charges, sections 7.4 et 10.4).
//!
//! La table refuse toute modification et toute suppression (déclencheurs SQLite) ;
//! ce module ne fait qu'ajouter des entrées.

use rusqlite::{Connection, params};
use serde_json::Value;

use crate::{Resultat, maintenant};

/// Une entrée du journal.
pub struct Entree<'a> {
    pub entite: &'a str,
    pub entite_id: i64,
    pub action: &'a str,
    pub ancienne_valeur: Option<Value>,
    pub nouvelle_valeur: Option<Value>,
    pub utilisateur_id: Option<i64>,
}

pub fn journaliser(conn: &Connection, entree: Entree<'_>) -> Resultat<()> {
    conn.execute(
        "INSERT INTO audit_log
             (entite, entite_id, action, ancienne_valeur, nouvelle_valeur, utilisateur_id, date_action)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            entree.entite,
            entree.entite_id,
            entree.action,
            entree.ancienne_valeur.map(|v| v.to_string()),
            entree.nouvelle_valeur.map(|v| v.to_string()),
            entree.utilisateur_id,
            maintenant(),
        ],
    )?;
    Ok(())
}

/// Nombre d'entrées pour une action donnée (utile aux tests et aux contrôles).
pub fn compter(conn: &Connection, action: &str) -> Resultat<i64> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM audit_log WHERE action = ?1",
        [action],
        |r| r.get(0),
    )?)
}

/// Entrée relue du journal, avec le nom du compte qui a agi.
#[derive(Debug, Clone, PartialEq)]
pub struct EntreeLue {
    pub date_action: String,
    pub action: String,
    pub ancienne_valeur: Option<Value>,
    pub nouvelle_valeur: Option<Value>,
    /// Nom complet du compte, vide si l'action n'a pas d'auteur connu.
    pub utilisateur: String,
}

/// Entrées d'une entité, de la plus ancienne à la plus récente.
pub fn lister_pour(conn: &Connection, entite: &str, entite_id: i64) -> Resultat<Vec<EntreeLue>> {
    let mut requete = conn.prepare(
        "SELECT a.date_action, a.action, a.ancienne_valeur, a.nouvelle_valeur,
                COALESCE(u.nom_complet, '')
         FROM audit_log a LEFT JOIN utilisateur u ON u.id = a.utilisateur_id
         WHERE a.entite = ?1 AND a.entite_id = ?2
         ORDER BY a.id",
    )?;
    let json = |t: Option<String>| t.and_then(|t| serde_json::from_str(&t).ok());
    let entrees = requete
        .query_map(params![entite, entite_id], |r| {
            Ok(EntreeLue {
                date_action: r.get(0)?,
                action: r.get(1)?,
                ancienne_valeur: json(r.get(2)?),
                nouvelle_valeur: json(r.get(3)?),
                utilisateur: r.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(entrees)
}
