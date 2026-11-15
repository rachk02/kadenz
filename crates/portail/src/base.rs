//! Base du portail : SQLite dans le dossier du portail (spécification du
//! portail, section 3). Pas de `kadenz-stockage` : son chiffrement repose sur
//! la protection des données de Windows, et le portail tourne sous Linux.

use std::path::Path;

use rusqlite::Connection;

/// Version du schéma, dans `PRAGMA user_version`.
const VERSION: i64 = 1;

const SCHEMA: &str = "
CREATE TABLE compte (
    id INTEGER PRIMARY KEY,
    identifiant TEXT NOT NULL UNIQUE COLLATE NOCASE,
    nom TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('editeur', 'revendeur')),
    mot_de_passe TEXT NOT NULL,
    actif INTEGER NOT NULL DEFAULT 1 CHECK (actif IN (0, 1)),
    cree_le TEXT NOT NULL
);
CREATE TABLE entreprise (
    identifiant TEXT PRIMARY KEY,
    client TEXT NOT NULL,
    revendeur INTEGER REFERENCES compte (id),
    cree_le TEXT NOT NULL
);
CREATE TABLE paiement (
    id INTEGER PRIMARY KEY,
    entreprise TEXT NOT NULL REFERENCES entreprise (identifiant),
    recu_le TEXT NOT NULL,
    mode TEXT NOT NULL CHECK (mode IN ('virement', 'cheque', 'especes')),
    montant INTEGER NOT NULL CHECK (montant > 0),
    devise TEXT NOT NULL,
    reference TEXT,
    postes INTEGER NOT NULL CHECK (postes BETWEEN 1 AND 1000),
    formule TEXT NOT NULL CHECK (formule IN ('mensuelle', 'annuelle')),
    saisi_par INTEGER NOT NULL REFERENCES compte (id),
    saisi_le TEXT NOT NULL
);
CREATE TABLE licence_emise (
    id INTEGER PRIMARY KEY,
    entreprise TEXT NOT NULL REFERENCES entreprise (identifiant),
    paiement INTEGER NOT NULL UNIQUE REFERENCES paiement (id),
    texte TEXT NOT NULL,
    debut TEXT NOT NULL,
    echeance TEXT NOT NULL,
    postes INTEGER NOT NULL,
    modules TEXT NOT NULL,
    formule TEXT NOT NULL,
    emetteur TEXT NOT NULL,
    emise_le TEXT NOT NULL,
    par INTEGER NOT NULL REFERENCES compte (id)
);
CREATE TABLE journal (
    id INTEGER PRIMARY KEY,
    heure TEXT NOT NULL,
    compte INTEGER REFERENCES compte (id),
    action TEXT NOT NULL,
    cible TEXT,
    details TEXT NOT NULL DEFAULT ''
);
-- Journal en ajout seul : ni modification ni effacement.
CREATE TRIGGER journal_sans_modification BEFORE UPDATE ON journal
BEGIN SELECT RAISE(ABORT, 'journal en ajout seul'); END;
CREATE TRIGGER journal_sans_effacement BEFORE DELETE ON journal
BEGIN SELECT RAISE(ABORT, 'journal en ajout seul'); END;
CREATE INDEX paiement_entreprise ON paiement (entreprise);
CREATE INDEX licence_entreprise ON licence_emise (entreprise);
";

/// Ouvre (et crée au besoin) la base du portail dans le dossier.
pub(crate) fn ouvrir(dossier: &Path) -> rusqlite::Result<Connection> {
    std::fs::create_dir_all(dossier).map_err(|e| {
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
            Some(format!("{} : {e}", dossier.display())),
        )
    })?;
    restreindre(dossier, 0o700);
    let chemin = dossier.join("portail.sqlite");
    let conn = Connection::open(&chemin)?;
    restreindre(&chemin, 0o600);
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version == 0 {
        conn.execute_batch(&format!(
            "BEGIN; {SCHEMA} PRAGMA user_version = {VERSION}; COMMIT;"
        ))?;
    }
    Ok(conn)
}

/// Dossier et fichiers du portail réservés au seul compte du service (Linux).
#[cfg(unix)]
pub(crate) fn restreindre(chemin: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(chemin, std::fs::Permissions::from_mode(mode));
}

#[cfg(not(unix))]
pub(crate) fn restreindre(_chemin: &Path, _mode: u32) {}
