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
