//! Accès SQLite de Kadenz (cahier des charges, sections 1, 10.3 et 11).

pub mod audit;
pub mod comptes;
pub mod entreprise;
pub mod installation;
pub mod parametres;

use std::fmt;
use std::path::Path;

use kadenz_domaine::document::TypeDocument;
use rusqlite::{Connection, params};

/// Connexion à la base, ré-exportée pour que l'application n'ait pas à dépendre
/// directement de `rusqlite`.
pub type Connexion = Connection;

/// Horodatage local au format ISO 8601, à la seconde.
pub(crate) fn maintenant() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

/// Migrations embarquées, appliquées dans l'ordre. Le rang d'une migration
/// correspond à `PRAGMA user_version` une fois appliquée. Ne jamais modifier
/// une migration publiée : en ajouter une nouvelle.
const MIGRATIONS: &[&str] = &[include_str!("migrations/0001_initial.sql")];

#[derive(Debug)]
pub enum Erreur {
    Sqlite(rusqlite::Error),
    Io(std::io::Error),
    /// La base a été créée par une version plus récente de l'application.
    VersionSchemaInconnue {
        base: u32,
        application: u32,
    },
    /// Saisie refusée ; le message est destiné à l'utilisateur.
    Validation(String),
    /// Identifiant inconnu, mot de passe faux ou compte désactivé (sans préciser lequel).
    ConnexionRefusee,
    /// L'opération laisserait l'installation sans administrateur actif.
    DernierAdmin,
    /// L'installation initiale a déjà été faite.
    DejaInstalle,
    Hachage(String),
}

impl fmt::Display for Erreur {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Erreur::Sqlite(e) => write!(f, "erreur de base de données : {e}"),
            Erreur::Io(e) => write!(f, "erreur de fichier : {e}"),
            Erreur::Validation(message) => f.write_str(message),
            Erreur::ConnexionRefusee => f.write_str("identifiant ou mot de passe incorrect"),
            Erreur::DernierAdmin => f.write_str(
                "impossible : l'installation doit garder au moins un administrateur actif",
            ),
            Erreur::DejaInstalle => f.write_str("l'installation initiale a déjà été faite"),
            Erreur::Hachage(e) => write!(f, "erreur de hachage du mot de passe : {e}"),
            Erreur::VersionSchemaInconnue { base, application } => write!(
                f,
                "la base est au schéma {base}, cette version de l'application ne connaît que le schéma {application}"
            ),
        }
    }
}

impl std::error::Error for Erreur {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Erreur::Sqlite(e) => Some(e),
            Erreur::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for Erreur {
    fn from(e: rusqlite::Error) -> Self {
        Erreur::Sqlite(e)
    }
}

impl From<std::io::Error> for Erreur {
    fn from(e: std::io::Error) -> Self {
        Erreur::Io(e)
    }
}

pub type Resultat<T> = Result<T, Erreur>;

/// Ouvre (ou crée) la base au chemin donné et applique les migrations en attente.
pub fn ouvrir(chemin: &Path) -> Resultat<Connection> {
    if let Some(dossier) = chemin.parent() {
        std::fs::create_dir_all(dossier)?;
    }
    let mut conn = Connection::open(chemin)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    preparer(&mut conn)?;
    Ok(conn)
}

/// Base en mémoire, pour les tests.
pub fn ouvrir_en_memoire() -> Resultat<Connection> {
    let mut conn = Connection::open_in_memory()?;
    preparer(&mut conn)?;
    Ok(conn)
}

fn preparer(conn: &mut Connection) -> Resultat<()> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrer(conn)
}

/// Version de schéma connue de cette version de l'application.
pub fn version_schema_application() -> u32 {
    MIGRATIONS.len() as u32
}

fn migrer(conn: &mut Connection) -> Resultat<()> {
    let base: u32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    let application = version_schema_application();
    if base > application {
        return Err(Erreur::VersionSchemaInconnue { base, application });
    }
    for (rang, sql) in MIGRATIONS.iter().enumerate().skip(base as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", rang as u32 + 1)?;
        tx.commit()?;
    }
    Ok(())
}

/// Réserve le prochain rang de numérotation pour (type, année, mois).
///
/// À appeler dans la transaction qui valide le document, pour qu'un échec de
/// validation n'entame pas la séquence.
pub fn prochain_rang(
    conn: &Connection,
    type_document: TypeDocument,
    annee: u16,
    mois: u8,
) -> Resultat<u32> {
    let rang = conn.query_row(
        "INSERT INTO compteur_numerotation (type_document, annee, mois, dernier_numero)
         VALUES (?1, ?2, ?3, 1)
         ON CONFLICT (type_document, annee, mois)
         DO UPDATE SET dernier_numero = dernier_numero + 1
         RETURNING dernier_numero",
        params![type_document.code(), annee, mois],
        |r| r.get(0),
    )?;
    Ok(rang)
}

/// Sauvegarde cohérente de la base ouverte vers `destination`, par l'API de
/// sauvegarde en ligne de SQLite (jamais une copie brute du fichier).
pub fn sauvegarder(conn: &Connection, destination: &Path) -> Resultat<()> {
    if let Some(dossier) = destination.parent() {
        std::fs::create_dir_all(dossier)?;
    }
    conn.backup(
        rusqlite::MAIN_DB,
        destination,
        None::<fn(rusqlite::backup::Progress)>,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Connection {
        ouvrir_en_memoire().expect("ouverture de la base en mémoire")
    }

    #[test]
    fn migrations_appliquees() {
        let conn = base();
        let v: u32 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(v, version_schema_application());
    }

    #[test]
    fn compteur_par_type_et_mois() {
        let conn = base();
        let f = TypeDocument::Facture;
        assert_eq!(prochain_rang(&conn, f, 2026, 5).unwrap(), 1);
        assert_eq!(prochain_rang(&conn, f, 2026, 5).unwrap(), 2);
        assert_eq!(prochain_rang(&conn, f, 2026, 6).unwrap(), 1);
        assert_eq!(
            prochain_rang(&conn, TypeDocument::Devis, 2026, 5).unwrap(),
            1
        );
    }

    #[test]
    fn compteur_annule_avec_la_transaction() {
        let mut conn = base();
        {
            let tx = conn.transaction().unwrap();
            assert_eq!(
                prochain_rang(&tx, TypeDocument::Facture, 2026, 5).unwrap(),
                1
            );
            // Pas de commit : la validation a échoué.
        }
        assert_eq!(
            prochain_rang(&conn, TypeDocument::Facture, 2026, 5).unwrap(),
            1
        );
    }

    #[test]
    fn audit_en_ajout_seul() {
        let conn = base();
        conn.execute(
            "INSERT INTO audit_log (entite, entite_id, action, date_action)
             VALUES ('document', 1, 'validation', '2026-11-09T10:00:00')",
            [],
        )
        .unwrap();
        assert!(
            conn.execute("UPDATE audit_log SET action = 'x'", [])
                .is_err()
        );
        assert!(conn.execute("DELETE FROM audit_log", []).is_err());
    }

    fn inserer_document(conn: &Connection, type_document: &str) -> i64 {
        conn.execute(
            "INSERT INTO document (type, date, date_creation) VALUES (?1, '2026-11-09', '2026-11-09T10:00:00')",
            [type_document],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn reglement_limite_a_facture_et_bon_commande() {
        let conn = base();
        let insertion =
            "INSERT INTO reglement (document_id, date, montant) VALUES (?1, '2026-11-09', 1000)";
        let devis = inserer_document(&conn, "devis");
        let facture = inserer_document(&conn, "facture");
        let bc = inserer_document(&conn, "bon_commande");
        assert!(conn.execute(insertion, [devis]).is_err());
        assert!(conn.execute(insertion, [facture]).is_ok());
        assert!(conn.execute(insertion, [bc]).is_ok());
    }

    #[test]
    fn brouillon_sans_numero() {
        let conn = base();
        let r = conn.execute(
            "INSERT INTO document (type, numero, date, date_creation)
             VALUES ('facture', 'F2026-05-0001', '2026-11-09', '2026-11-09T10:00:00')",
            [],
        );
        assert!(r.is_err(), "un brouillon ne doit pas porter de numéro");
    }

    #[test]
    fn sauvegarde_lisible() {
        let conn = base();
        prochain_rang(&conn, TypeDocument::Facture, 2026, 5).unwrap();
        let dossier = std::env::temp_dir().join(format!("kadenz-test-{}", std::process::id()));
        let cible = dossier.join("sauvegarde.sqlite");
        sauvegarder(&conn, &cible).unwrap();

        let copie = ouvrir(&cible).unwrap();
        let n: u32 = copie
            .query_row(
                "SELECT dernier_numero FROM compteur_numerotation",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
        drop(copie);
        let _ = std::fs::remove_dir_all(dossier);
    }
}
