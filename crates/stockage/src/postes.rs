//! Postes inscrits auprès du serveur (phase 2, jalon F2 ; inscription par code
//! au jalon F4). Un poste est désigné par l'empreinte SHA-256 de son
//! certificat TLS, en hexadécimal.

use rusqlite::{Connection, OptionalExtension, params};

use crate::{Resultat, maintenant};

/// Inscrit un poste (ou le réinscrit s'il avait été retiré).
pub fn inscrire(conn: &Connection, empreinte: &str, nom: &str) -> Resultat<()> {
    conn.execute(
        "INSERT INTO poste_inscrit (empreinte, nom, inscrit_le) VALUES (?1, ?2, ?3)
         ON CONFLICT (empreinte) DO UPDATE SET nom = ?2, inscrit_le = ?3, retire_le = NULL",
        params![empreinte, nom, maintenant()],
    )?;
    Ok(())
}

/// Retire un poste : il ne peut plus se connecter.
pub fn retirer(conn: &Connection, empreinte: &str) -> Resultat<()> {
    conn.execute(
        "UPDATE poste_inscrit SET retire_le = ?2 WHERE empreinte = ?1 AND retire_le IS NULL",
        params![empreinte, maintenant()],
    )?;
    Ok(())
}

/// Vrai si le poste est inscrit et n'a pas été retiré.
pub fn est_inscrit(conn: &Connection, empreinte: &str) -> Resultat<bool> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM poste_inscrit WHERE empreinte = ?1 AND retire_le IS NULL",
            [empreinte],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PosteInscrit {
    pub empreinte: String,
    pub nom: String,
    pub inscrit_le: String,
}

/// Postes inscrits et non retirés, par nom.
pub fn liste(conn: &Connection) -> Resultat<Vec<PosteInscrit>> {
    let mut requete = conn.prepare(
        "SELECT empreinte, nom, inscrit_le FROM poste_inscrit
         WHERE retire_le IS NULL ORDER BY nom, inscrit_le",
    )?;
    let postes = requete
        .query_map([], |r| {
            Ok(PosteInscrit {
                empreinte: r.get(0)?,
                nom: r.get(1)?,
                inscrit_le: r.get(2)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(postes)
}

/// Nombre de postes inscrits et non retirés (limite de la licence, F3).
pub fn nombre_inscrits(conn: &Connection) -> Resultat<u32> {
    Ok(conn.query_row(
        "SELECT count(*) FROM poste_inscrit WHERE retire_le IS NULL",
        [],
        |r| r.get(0),
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inscription_et_retrait() {
        let conn = crate::ouvrir_en_memoire().unwrap();
        assert!(!est_inscrit(&conn, "ab").unwrap());
        inscrire(&conn, "ab", "Poste comptable").unwrap();
        assert!(est_inscrit(&conn, "ab").unwrap());
        assert_eq!(nombre_inscrits(&conn).unwrap(), 1);
        assert_eq!(liste(&conn).unwrap()[0].nom, "Poste comptable");
        retirer(&conn, "ab").unwrap();
        assert!(!est_inscrit(&conn, "ab").unwrap());
        assert_eq!(nombre_inscrits(&conn).unwrap(), 0);
        inscrire(&conn, "ab", "Poste comptable").unwrap();
        assert!(est_inscrit(&conn, "ab").unwrap());
    }
}
