//! Chaîne de hachage du journal (relecture d'OpenCode, défaut 12) : chaque
//! entrée reçoit une empreinte SHA-256 qui couvre ses champs et l'empreinte
//! de l'entrée précédente. Modifier, retirer ou intercaler une entrée rompt
//! la chaîne à cet endroit.
//!
//! Limite : quelqu'un qui peut écrire dans la base peut aussi recalculer
//! toute la chaîne. La chaîne ne prouve donc l'historique que jusqu'à la
//! dernière empreinte notée hors du serveur (export du journal, sceau relevé
//! sur la page du journal) : une entrée antérieure à ce sceau ne peut plus
//! être changée sans que la vérification le voie.
//!
//! Les empreintes sont dans la table `journal_chaine` (schéma 6), en ajout
//! seul comme le journal ; `precedente` est unique, si bien que deux entrées
//! ne peuvent pas suivre la même (deux écritures concurrentes échouent au
//! lieu de faire bifurquer la chaîne).

use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest, Sha256};

/// Empreinte qui précède la première entrée.
pub(crate) const ORIGINE: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Séparation de domaine : cette empreinte ne sert à rien d'autre.
const DOMAINE: &[u8] = b"kadenz-portail-journal-1";

/// État de la chaîne du journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EtatChaine {
    /// Entrées vérifiées, depuis la première.
    pub entrees: i64,
    /// Empreinte de la dernière entrée vérifiée : le sceau à relever.
    pub sceau: String,
    /// Première entrée où la chaîne est rompue (identifiant du journal).
    pub rupture: Option<i64>,
}

/// Entrée du journal avec son empreinte, pour l'export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntreeChainee {
    pub id: i64,
    pub heure: String,
    pub compte: Option<i64>,
    pub action: String,
    pub cible: Option<String>,
    pub details: String,
    pub precedente: i64,
    pub empreinte: String,
}

/// Champ encodé sans ambiguïté : absent, ou présent avec sa longueur.
fn champ(h: &mut Sha256, valeur: Option<&[u8]>) {
    match valeur {
        None => h.update([0u8]),
        Some(v) => {
            h.update([1u8]);
            h.update((v.len() as u64).to_le_bytes());
            h.update(v);
        }
    }
}

/// Empreinte d'une entrée, à la suite de l'empreinte `precedente`.
pub(crate) fn empreinte(
    precedente: &str,
    id: i64,
    heure: &str,
    compte: Option<i64>,
    action: &str,
    cible: Option<&str>,
    details: &str,
) -> String {
    let mut h = Sha256::new();
    h.update(DOMAINE);
    champ(&mut h, Some(precedente.as_bytes()));
    champ(&mut h, Some(&id.to_le_bytes()));
    champ(&mut h, Some(heure.as_bytes()));
    let compte = compte.map(i64::to_le_bytes);
    champ(&mut h, compte.as_ref().map(|c| c.as_slice()));
    champ(&mut h, Some(action.as_bytes()));
    champ(&mut h, cible.map(str::as_bytes));
    champ(&mut h, Some(details.as_bytes()));
    h.finalize().iter().map(|o| format!("{o:02x}")).collect()
}

/// Dernier maillon : identifiant et empreinte (0 et l'origine si vide).
fn dernier(conn: &Connection) -> rusqlite::Result<(i64, String)> {
    Ok(conn
        .query_row(
            "SELECT id, empreinte FROM journal_chaine ORDER BY id DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .unwrap_or((0, ORIGINE.to_string())))
}

/// Ajoute à la chaîne l'entrée `id` du journal, qui vient d'être écrite.
pub(crate) fn chainer(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    let (precedente, empreinte_precedente) = dernier(conn)?;
    let (heure, compte, action, cible, details): (
        String,
        Option<i64>,
        String,
        Option<String>,
        String,
    ) = conn.query_row(
        "SELECT heure, compte, action, cible, details FROM journal WHERE id = ?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
    )?;
    let e = empreinte(
        &empreinte_precedente,
        id,
        &heure,
        compte,
        &action,
        cible.as_deref(),
        &details,
    );
    conn.execute(
        "INSERT INTO journal_chaine (id, precedente, empreinte) VALUES (?1, ?2, ?3)",
        params![id, precedente, e],
    )?;
    Ok(())
}

/// Chaîne les entrées écrites avant la chaîne (migration vers le schéma 6),
/// dans l'ordre du journal.
pub(crate) fn rattraper(conn: &Connection) -> rusqlite::Result<()> {
    let ids: Vec<i64> = conn
        .prepare(
            "SELECT id FROM journal WHERE id NOT IN (SELECT id FROM journal_chaine) ORDER BY id",
        )?
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    for id in ids {
        chainer(conn, id)?;
    }
    Ok(())
}

/// Entrées du journal à partir de `depuis` (exclu), avec leur maillon.
pub(crate) fn entrees(conn: &Connection, depuis: i64) -> rusqlite::Result<Vec<EntreeChainee>> {
    conn.prepare(
        "SELECT j.id, j.heure, j.compte, j.action, j.cible, j.details,
                coalesce(c.precedente, -1), coalesce(c.empreinte, '')
         FROM journal j LEFT JOIN journal_chaine c ON c.id = j.id
         WHERE j.id > ?1 ORDER BY j.id",
    )?
    .query_map([depuis], |r| {
        Ok(EntreeChainee {
            id: r.get(0)?,
            heure: r.get(1)?,
            compte: r.get(2)?,
            action: r.get(3)?,
            cible: r.get(4)?,
            details: r.get(5)?,
            precedente: r.get(6)?,
            empreinte: r.get(7)?,
        })
    })?
    .collect()
}

/// Recalcule toute la chaîne depuis la première entrée.
pub(crate) fn verifier(conn: &Connection) -> rusqlite::Result<EtatChaine> {
    let (mut precedente, mut sceau) = (0i64, ORIGINE.to_string());
    let mut compte = 0i64;
    for e in entrees(conn, 0)? {
        let attendue = empreinte(
            &sceau,
            e.id,
            &e.heure,
            e.compte,
            &e.action,
            e.cible.as_deref(),
            &e.details,
        );
        if e.precedente != precedente || e.empreinte != attendue {
            return Ok(EtatChaine {
                entrees: compte,
                sceau,
                rupture: Some(e.id),
            });
        }
        precedente = e.id;
        sceau = e.empreinte;
        compte += 1;
    }
    // Maillon sans entrée du journal (entrée retirée malgré le déclencheur).
    let orphelin: Option<i64> = conn
        .query_row(
            "SELECT min(id) FROM journal_chaine WHERE id NOT IN (SELECT id FROM journal)",
            [],
            |r| r.get(0),
        )
        .optional()?
        .flatten();
    Ok(EtatChaine {
        entrees: compte,
        sceau,
        rupture: orphelin,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::base::migrer(&conn).unwrap();
        conn
    }

    fn noter(conn: &Connection, action: &str, cible: Option<&str>, details: &str) -> i64 {
        conn.execute(
            "INSERT INTO journal (heure, compte, action, cible, details)
             VALUES ('2026-11-19 10:00:00', NULL, ?1, ?2, ?3)",
            params![action, cible, details],
        )
        .unwrap();
        let id = conn.last_insert_rowid();
        chainer(conn, id).unwrap();
        id
    }

    #[test]
    fn champs_sans_ambiguite() {
        let a = empreinte(ORIGINE, 1, "h", None, "x", None, "");
        let b = empreinte(ORIGINE, 1, "h", None, "x", Some(""), "");
        let c = empreinte(ORIGINE, 1, "h", None, "x", Some("a"), "");
        let d = empreinte(ORIGINE, 1, "h", None, "xa", None, "");
        assert_eq!(a.len(), 64);
        assert!(a != b && b != c && c != d && a != d);
    }

    #[test]
    fn chaine_intacte_puis_rompue() {
        let conn = base();
        assert_eq!(
            verifier(&conn).unwrap(),
            EtatChaine {
                entrees: 0,
                sceau: ORIGINE.into(),
                rupture: None
            }
        );
        noter(&conn, "connexion", None, "adresse 10.0.0.1");
        let deux = noter(&conn, "licence_emise", Some("ent"), "3 postes");
        noter(&conn, "deconnexion", None, "");
        let etat = verifier(&conn).unwrap();
        assert_eq!((etat.entrees, etat.rupture), (3, None));
        let sceau = etat.sceau.clone();
        // Deux entrées ne peuvent pas suivre la même.
        assert!(
            conn.execute(
                "INSERT INTO journal_chaine (id, precedente, empreinte) VALUES (99, ?1, 'x')",
                [deux],
            )
            .is_err()
        );
        // Ni modification ni effacement par les voies normales.
        assert!(
            conn.execute("UPDATE journal_chaine SET empreinte = 'x'", [])
                .is_err()
        );
        assert!(conn.execute("DELETE FROM journal_chaine", []).is_err());
        // Une modification qui contourne les déclencheurs rompt la chaîne à
        // l'entrée modifiée.
        conn.execute_batch(
            "DROP TRIGGER journal_sans_modification;
             UPDATE journal SET details = '30 postes' WHERE action = 'licence_emise';",
        )
        .unwrap();
        let etat = verifier(&conn).unwrap();
        assert_eq!((etat.entrees, etat.rupture), (1, Some(deux)));
        assert_ne!(etat.sceau, sceau);
    }

    #[test]
    fn entree_sans_maillon_rompt_la_chaine() {
        let conn = base();
        noter(&conn, "connexion", None, "");
        conn.execute(
            "INSERT INTO journal (heure, compte, action, cible, details)
             VALUES ('2026-11-19 10:00:01', NULL, 'connexion', NULL, 'glissée')",
            [],
        )
        .unwrap();
        let glissee = conn.last_insert_rowid();
        let etat = verifier(&conn).unwrap();
        assert_eq!((etat.entrees, etat.rupture), (1, Some(glissee)));
    }
}
