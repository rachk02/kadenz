//! Base du portail : SQLite dans le dossier du portail (spécification du
//! portail, section 3). Pas de `kadenz-stockage` : son chiffrement repose sur
//! la protection des données de Windows, et le portail tourne sous Linux.

use std::path::Path;

use rusqlite::Connection;

/// Version du schéma, dans `PRAGMA user_version` : celle du schéma de départ,
/// puis une par pas de `MIGRATIONS`.
const VERSION_DEPART: i64 = 1;

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

/// Pas de migration, dans l'ordre : le pas `i` mène de la version
/// `VERSION_DEPART + i` à la suivante. Une base neuve reçoit le schéma de
/// départ puis tous les pas, comme une base existante.
const MIGRATIONS: &[&str] = &[
    // 1 vers 2 : codes de licence courts (décision 74, `docs/spec-codes-licence.md`).
    // Le code en clair n'est jamais gardé : son empreinte SHA-256, et sa fin
    // pour le reconnaître. Le lien entre code et licence est porté par
    // `licence_emise.code` seulement.
    "
CREATE TABLE code_activation (
    id INTEGER PRIMARY KEY,
    empreinte TEXT NOT NULL UNIQUE,
    fin TEXT NOT NULL,
    postes INTEGER NOT NULL CHECK (postes BETWEEN 1 AND 1000),
    formule TEXT NOT NULL CHECK (formule IN ('mensuelle', 'annuelle')),
    modules TEXT NOT NULL,
    paiement INTEGER REFERENCES paiement (id),
    entreprise TEXT REFERENCES entreprise (identifiant),
    etat TEXT NOT NULL DEFAULT 'libre' CHECK (etat IN ('libre', 'active', 'annule')),
    cree_par INTEGER NOT NULL REFERENCES compte (id),
    cree_le TEXT NOT NULL,
    active_le TEXT
);
CREATE INDEX code_entreprise ON code_activation (entreprise);
CREATE INDEX code_paiement ON code_activation (paiement);
ALTER TABLE licence_emise ADD COLUMN code INTEGER REFERENCES code_activation (id);
",
    // 2 vers 3 : codes à vendre, sans paiement. Le lien de la licence au
    // paiement devient facultatif ; une licence vient d'un paiement ou d'un
    // code. SQLite ne sait pas changer une contrainte : table reconstruite,
    // données et colonne `code` gardées, index recréé.
    "
CREATE TABLE licence_emise_nouvelle (
    id INTEGER PRIMARY KEY,
    entreprise TEXT NOT NULL REFERENCES entreprise (identifiant),
    paiement INTEGER UNIQUE REFERENCES paiement (id),
    texte TEXT NOT NULL,
    debut TEXT NOT NULL,
    echeance TEXT NOT NULL,
    postes INTEGER NOT NULL,
    modules TEXT NOT NULL,
    formule TEXT NOT NULL,
    emetteur TEXT NOT NULL,
    emise_le TEXT NOT NULL,
    par INTEGER NOT NULL REFERENCES compte (id),
    code INTEGER REFERENCES code_activation (id),
    CHECK (paiement IS NOT NULL OR code IS NOT NULL)
);
INSERT INTO licence_emise_nouvelle
    (id, entreprise, paiement, texte, debut, echeance, postes, modules, formule, emetteur, emise_le, par, code)
    SELECT id, entreprise, paiement, texte, debut, echeance, postes, modules, formule, emetteur, emise_le, par, code
    FROM licence_emise;
DROP TABLE licence_emise;
ALTER TABLE licence_emise_nouvelle RENAME TO licence_emise;
CREATE INDEX licence_entreprise ON licence_emise (entreprise);
",
    // 3 vers 4 : frein des connexions et des activations gardé dans la base
    // (lot D1, docs/spec-portail.md, section 9), pour survivre à un
    // redémarrage. Dates en secondes depuis 1970 (UTC).
    TABLE_FREIN,
    // 4 vers 5 : codes de la ligne V1 (décision 75, lot L8,
    // docs/spec-codes-licence.md, section 7 bis). Un code a une gamme ; un
    // code V1 donne une licence V1 datée liée à un poste, une ligne de
    // `licence_v1` par poste qui l'a activé. La ligne en cours est celle sans
    // date de libération (une au plus par code).
    "
ALTER TABLE code_activation ADD COLUMN gamme TEXT NOT NULL DEFAULT 'entreprise'
    CHECK (gamme IN ('entreprise', 'v1'));
CREATE TABLE licence_v1 (
    id INTEGER PRIMARY KEY,
    code INTEGER NOT NULL REFERENCES code_activation (id),
    poste TEXT NOT NULL,
    client TEXT NOT NULL,
    texte TEXT NOT NULL,
    debut TEXT NOT NULL,
    echeance TEXT NOT NULL,
    emise_le TEXT NOT NULL,
    liberee_le TEXT,
    liberee_par INTEGER REFERENCES compte (id)
);
CREATE INDEX licence_v1_code ON licence_v1 (code);
CREATE UNIQUE INDEX licence_v1_en_cours ON licence_v1 (code) WHERE liberee_le IS NULL;
",
];

/// Table du frein (`frein.rs`) : une ligne par clé, échecs comptés, fin de
/// l'attente en cours, dernier échec.
pub(crate) const TABLE_FREIN: &str = "
CREATE TABLE frein (
    cle TEXT PRIMARY KEY,
    echecs INTEGER NOT NULL CHECK (echecs > 0),
    jusqua INTEGER,
    derniere INTEGER NOT NULL
);
CREATE INDEX frein_derniere ON frein (derniere);
";

/// Dernière version du schéma.
pub(crate) const VERSION: i64 = VERSION_DEPART + MIGRATIONS.len() as i64;

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
    // busy_timeout : une sauvegarde à chaud (`.backup`) ou un autre lecteur
    // ne fait pas échouer une écriture, qui attend jusqu'à 5 secondes.
    conn.execute_batch(
        "PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA busy_timeout = 5000;",
    )?;
    migrer(&conn)?;
    Ok(conn)
}

/// Amène la base à la dernière version, un pas par transaction. Une base
/// plus récente que ce programme est refusée. Les clés étrangères sont
/// suspendues pendant un pas (une reconstruction de table l'exige, et le
/// réglage est sans effet dans une transaction), puis contrôlées avant de
/// valider le pas.
pub(crate) fn migrer(conn: &Connection) -> rusqlite::Result<()> {
    let mut version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version == 0 {
        conn.execute_batch(&format!(
            "BEGIN; {SCHEMA} PRAGMA user_version = {VERSION_DEPART}; COMMIT;"
        ))?;
        version = VERSION_DEPART;
    }
    if version > VERSION {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
            Some(format!(
                "base du portail en version {version}, plus récente que ce programme ({VERSION})"
            )),
        ));
    }
    while version < VERSION {
        let pas = MIGRATIONS[(version - VERSION_DEPART) as usize];
        let suivante = version + 1;
        conn.execute_batch("PRAGMA foreign_keys = OFF;")?;
        let resultat = appliquer_pas(conn, pas, suivante);
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        resultat?;
        version = suivante;
    }
    Ok(())
}

fn appliquer_pas(conn: &Connection, pas: &str, suivante: i64) -> rusqlite::Result<()> {
    let transaction = conn.unchecked_transaction()?;
    transaction.execute_batch(pas)?;
    transaction.pragma_update(None, "user_version", suivante)?;
    let violation = transaction
        .prepare("PRAGMA foreign_key_check")?
        .query([])?
        .next()?
        .is_some();
    if violation {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT),
            Some(format!(
                "migration vers la version {suivante} : clés étrangères rompues"
            )),
        ));
    }
    transaction.commit()
}

/// Dossier et fichiers du portail réservés au seul compte du service (Linux).
#[cfg(unix)]
pub(crate) fn restreindre(chemin: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(chemin, std::fs::Permissions::from_mode(mode));
}

#[cfg(not(unix))]
pub(crate) fn restreindre(_chemin: &Path, _mode: u32) {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Une base en version 1 qui contient déjà une licence émise passe en
    /// dernière version sans rien perdre.
    #[test]
    fn base_en_version_1_migree() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        conn.execute_batch(&format!("BEGIN; {SCHEMA} PRAGMA user_version = 1; COMMIT;"))
            .unwrap();
        conn.execute_batch(
            "INSERT INTO compte (id, identifiant, nom, role, mot_de_passe, actif, cree_le)
                 VALUES (1, 'ed', 'Éditeur', 'editeur', 'x', 1, '2026-11-01');
             INSERT INTO entreprise (identifiant, client, cree_le)
                 VALUES ('00000000-0000-4000-8000-000000000001', 'Client', '2026-11-01');
             INSERT INTO paiement (id, entreprise, recu_le, mode, montant, devise, postes, formule, saisi_par, saisi_le)
                 VALUES (1, '00000000-0000-4000-8000-000000000001', '2026-11-01', 'virement', 100, 'XOF', 3, 'annuelle', 1, '2026-11-01');
             INSERT INTO licence_emise (entreprise, paiement, texte, debut, echeance, postes, modules, formule, emetteur, emise_le, par)
                 VALUES ('00000000-0000-4000-8000-000000000001', 1, 'KADENZ-E-x', '2026-11-01', '2027-11-01', 3, '', 'annuelle', 'Éditeur', '2026-11-01', 1);",
        )
        .unwrap();
        migrer(&conn).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, VERSION);
        let (texte, paiement, code): (String, Option<i64>, Option<i64>) = conn
            .query_row("SELECT texte, paiement, code FROM licence_emise", [], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })
            .unwrap();
        assert_eq!(
            (texte.as_str(), paiement, code),
            ("KADENZ-E-x", Some(1), None)
        );
        // Index recréé, clés étrangères de nouveau actives.
        let index: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = 'licence_entreprise'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(index, 1);
        let cles: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
            .unwrap();
        assert_eq!(cles, 1);
        // Une licence sans paiement ni code est refusée.
        assert!(
            conn.execute(
                "INSERT INTO licence_emise (entreprise, texte, debut, echeance, postes, modules, formule, emetteur, emise_le, par)
                 VALUES ('00000000-0000-4000-8000-000000000001', 'x', 'a', 'b', 1, '', 'annuelle', 'É', 'c', 1)",
                [],
            )
            .is_err()
        );
        // Une seconde ouverture ne refait rien.
        migrer(&conn).unwrap();
    }

    #[test]
    fn base_neuve_en_derniere_version() {
        let conn = Connection::open_in_memory().unwrap();
        migrer(&conn).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, VERSION);
        conn.execute_batch(
            "SELECT empreinte FROM code_activation; SELECT code FROM licence_emise;",
        )
        .unwrap();
    }
}
