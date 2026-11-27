//! Postes inscrits auprès du serveur (phase 2, jalon F2 ; inscription par code
//! au jalon F4). Un poste est désigné par l'empreinte SHA-256 de son
//! certificat TLS, en hexadécimal.

use rusqlite::{Connection, OptionalExtension, params};

use crate::{Resultat, maintenant};

/// Inscrit un poste (ou le réinscrit s'il avait été retiré).
pub fn inscrire(conn: &Connection, empreinte: &str, nom: &str) -> Resultat<()> {
    inscrire_sur(conn, empreinte, nom, "")
}

/// Comme [`inscrire`], avec l'identifiant de l'ordinateur du poste
/// (`identifiant_poste`, décision 80) ; vide pour un poste ancien.
pub fn inscrire_sur(
    conn: &Connection,
    empreinte: &str,
    nom: &str,
    ordinateur: &str,
) -> Resultat<()> {
    conn.execute(
        "INSERT INTO poste_inscrit (empreinte, nom, inscrit_le, ordinateur) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (empreinte) DO UPDATE SET nom = ?2, inscrit_le = ?3, ordinateur = ?4,
             retire_le = NULL",
        params![empreinte, nom, maintenant(), ordinateur],
    )?;
    Ok(())
}

/// Note le contact d'un poste avec le serveur (début de session).
pub fn noter_contact(conn: &Connection, empreinte: &str) -> Resultat<()> {
    conn.execute(
        "UPDATE poste_inscrit SET vu_le = ?2 WHERE empreinte = ?1",
        params![empreinte, maintenant()],
    )?;
    Ok(())
}

/// Retire les postes inscrits sur l'un des ordinateurs donnés (bloqués au
/// portail, décision 80) ; rend le nombre de postes retirés.
pub fn retirer_ordinateurs(conn: &Connection, ordinateurs: &[String]) -> Resultat<usize> {
    let mut retires = 0;
    for o in ordinateurs.iter().filter(|o| !o.is_empty()) {
        retires += conn.execute(
            "UPDATE poste_inscrit SET retire_le = ?2
             WHERE ordinateur = ?1 COLLATE NOCASE AND retire_le IS NULL",
            params![o, maintenant()],
        )?;
    }
    Ok(retires)
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
    /// Identifiant de l'ordinateur ; vide pour un poste ancien.
    pub ordinateur: String,
    /// Dernier contact avec le serveur.
    pub vu_le: Option<String>,
}

/// Postes inscrits et non retirés, par nom.
pub fn liste(conn: &Connection) -> Resultat<Vec<PosteInscrit>> {
    let mut requete = conn.prepare(
        "SELECT empreinte, nom, inscrit_le, ordinateur, vu_le FROM poste_inscrit
         WHERE retire_le IS NULL ORDER BY nom, inscrit_le",
    )?;
    let postes = requete
        .query_map([], |r| {
            Ok(PosteInscrit {
                empreinte: r.get(0)?,
                nom: r.get(1)?,
                inscrit_le: r.get(2)?,
                ordinateur: r.get(3)?,
                vu_le: r.get(4)?,
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
