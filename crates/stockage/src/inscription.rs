//! Codes d'inscription des postes, sur le serveur (phase 2, jalon F4 ;
//! décision 36 ; spécification de synchronisation, section 7).
//!
//! L'Admin crée un code pour un poste nommé, dans la fenêtre du serveur ou
//! depuis un poste ; le secret n'est montré qu'une fois, seule son empreinte
//! SHA-256 est gardée. Un code sert une fois, expire au bout de 24 heures et
//! peut être annulé ; un nouveau code pour le même nom annule le précédent.
//! Le nombre de postes est limité par l'abonnement.

use chrono::{DateTime, Duration, Utc};
use kadenz_domaine::message::MessageCompose;
use kadenz_protocole::inscription::{ALPHABET, LONGUEUR_SECRET, normaliser_secret};
use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest, Sha256};

use crate::{Erreur, Resultat, postes};

/// Durée de validité d'un code (décision 36).
pub const VALIDITE_HEURES: i64 = 24;

/// Modèles des refus d'inscription, pour leurs traductions.
pub const MODELES_ERREUR: &[&str] = &[
    "code d'inscription inconnu ou déjà utilisé",
    "code d'inscription expiré",
    "code d'inscription annulé",
    "nombre de postes atteint : {} postes permis par l'abonnement",
    "trop d'essais d'inscription : nouvel essai dans {} s",
];

fn horodatage(t: DateTime<Utc>) -> String {
    t.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn hacher(secret: &str) -> String {
    Sha256::digest(secret.as_bytes())
        .iter()
        .map(|o| format!("{o:02x}"))
        .collect()
}

fn nouveau_secret() -> Resultat<String> {
    let mut octets = [0u8; LONGUEUR_SECRET];
    getrandom::fill(&mut octets).map_err(|e| Erreur::Validation(e.to_string()))?;
    // 5 bits par caractère : 60 bits d'aléa.
    Ok(octets
        .iter()
        .map(|o| ALPHABET[(o & 0x1f) as usize] as char)
        .collect())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeCree {
    pub id: i64,
    /// Secret normalisé, à montrer une seule fois.
    pub secret: String,
    pub expire_le: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeEnCours {
    pub id: i64,
    pub nom_poste: String,
    pub cree_le: String,
    pub expire_le: String,
}

/// Crée un code pour le poste nommé ; les codes encore ouverts pour ce nom
/// sont annulés.
pub fn creer_code(
    conn: &Connection,
    nom_poste: &str,
    maintenant: DateTime<Utc>,
) -> Resultat<CodeCree> {
    let nom = nom_poste.trim();
    if nom.is_empty() {
        return Err(Erreur::Validation("le nom du poste est obligatoire".into()));
    }
    let secret = nouveau_secret()?;
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE code_inscription SET annule_le = ?2
         WHERE nom_poste = ?1 AND servi_le IS NULL AND annule_le IS NULL",
        params![nom, horodatage(maintenant)],
    )?;
    let expire_le = horodatage(maintenant + Duration::hours(VALIDITE_HEURES));
    tx.execute(
        "INSERT INTO code_inscription (secret_hache, nom_poste, cree_le, expire_le)
         VALUES (?1, ?2, ?3, ?4)",
        params![hacher(&secret), nom, horodatage(maintenant), expire_le],
    )?;
    let id = tx.last_insert_rowid();
    tx.commit()?;
    Ok(CodeCree {
        id,
        secret,
        expire_le,
    })
}

/// Codes ni servis, ni annulés, ni expirés.
pub fn codes_en_cours(conn: &Connection, maintenant: DateTime<Utc>) -> Resultat<Vec<CodeEnCours>> {
    let mut requete = conn.prepare(
        "SELECT id, nom_poste, cree_le, expire_le FROM code_inscription
         WHERE servi_le IS NULL AND annule_le IS NULL AND expire_le > ?1
         ORDER BY cree_le",
    )?;
    let codes = requete
        .query_map([horodatage(maintenant)], |r| {
            Ok(CodeEnCours {
                id: r.get(0)?,
                nom_poste: r.get(1)?,
                cree_le: r.get(2)?,
                expire_le: r.get(3)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(codes)
}

pub fn annuler_code(conn: &Connection, id: i64, maintenant: DateTime<Utc>) -> Resultat<()> {
    conn.execute(
        "UPDATE code_inscription SET annule_le = ?2
         WHERE id = ?1 AND servi_le IS NULL AND annule_le IS NULL",
        params![id, horodatage(maintenant)],
    )?;
    Ok(())
}

struct CodeLu {
    id: i64,
    nom: String,
    expire_le: String,
    servi_le: Option<String>,
    annule_le: Option<String>,
}

/// Inscrit le poste au certificat `empreinte` avec le secret donné, sous le
/// nom choisi à la création du code. Un refus est rendu comme message à
/// traduire, sans rien écrire ; `postes_permis` vient de l'abonnement.
/// Rend le nom du poste.
pub fn consommer(
    conn: &Connection,
    secret: &str,
    empreinte: &str,
    postes_permis: u32,
    maintenant: DateTime<Utc>,
) -> Resultat<Result<String, MessageCompose>> {
    let inconnu = || {
        Ok(Err(MessageCompose::new(
            "code d'inscription inconnu ou déjà utilisé",
            &[],
        )))
    };
    let Some(secret) = normaliser_secret(secret) else {
        return inconnu();
    };
    let tx = conn.unchecked_transaction()?;
    let code = tx
        .query_row(
            "SELECT id, nom_poste, expire_le, servi_le, annule_le FROM code_inscription
             WHERE secret_hache = ?1",
            [hacher(&secret)],
            |r| {
                Ok(CodeLu {
                    id: r.get(0)?,
                    nom: r.get(1)?,
                    expire_le: r.get(2)?,
                    servi_le: r.get(3)?,
                    annule_le: r.get(4)?,
                })
            },
        )
        .optional()?;
    let Some(CodeLu {
        id,
        nom,
        expire_le,
        servi_le,
        annule_le,
    }) = code
    else {
        return inconnu();
    };
    if servi_le.is_some() {
        return inconnu();
    }
    if annule_le.is_some() {
        return Ok(Err(MessageCompose::new("code d'inscription annulé", &[])));
    }
    if expire_le <= horodatage(maintenant) {
        return Ok(Err(MessageCompose::new("code d'inscription expiré", &[])));
    }
    if !postes::est_inscrit(&tx, empreinte)? && postes::nombre_inscrits(&tx)? >= postes_permis {
        return Ok(Err(MessageCompose::new(
            "nombre de postes atteint : {} postes permis par l'abonnement",
            &[&postes_permis],
        )));
    }
    postes::inscrire(&tx, empreinte, &nom)?;
    tx.execute(
        "UPDATE code_inscription SET servi_le = ?2, poste = ?3 WHERE id = ?1",
        params![id, horodatage(maintenant), empreinte],
    )?;
    tx.commit()?;
    Ok(Ok(nom))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ouvrir_en_memoire;

    fn t(texte: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(texte).unwrap().to_utc()
    }

    fn modele(r: Resultat<Result<String, MessageCompose>>) -> &'static str {
        r.unwrap().unwrap_err().modele
    }

    #[test]
    fn code_servi_une_fois() {
        let conn = ouvrir_en_memoire().unwrap();
        let debut = t("2026-11-12T10:00:00Z");
        let code = creer_code(&conn, "Comptabilité", debut).unwrap();
        assert_eq!(code.secret.len(), LONGUEUR_SECRET);
        assert_eq!(code.expire_le, "2026-11-13T10:00:00Z");
        assert_eq!(codes_en_cours(&conn, debut).unwrap().len(), 1);
        // Saisi en minuscules avec des tirets.
        let tape = kadenz_protocole::inscription::presenter_secret(&code.secret).to_lowercase();
        assert_eq!(
            consommer(&conn, &tape, "poste-a", 5, debut)
                .unwrap()
                .unwrap(),
            "Comptabilité"
        );
        assert!(postes::est_inscrit(&conn, "poste-a").unwrap());
        assert!(codes_en_cours(&conn, debut).unwrap().is_empty());
        assert_eq!(
            modele(consommer(&conn, &code.secret, "poste-b", 5, debut)),
            "code d'inscription inconnu ou déjà utilisé"
        );
        assert_eq!(
            modele(consommer(&conn, "AAAA-BBBB-CCCC", "poste-b", 5, debut)),
            "code d'inscription inconnu ou déjà utilisé"
        );
    }

    #[test]
    fn code_expire_annule_ou_remplace() {
        let conn = ouvrir_en_memoire().unwrap();
        let debut = t("2026-11-12T10:00:00Z");
        let expire = creer_code(&conn, "A", debut).unwrap();
        assert_eq!(
            modele(consommer(
                &conn,
                &expire.secret,
                "p",
                5,
                t("2026-11-13T10:00:00Z")
            )),
            "code d'inscription expiré"
        );
        let annule = creer_code(&conn, "B", debut).unwrap();
        annuler_code(&conn, annule.id, debut).unwrap();
        assert_eq!(
            modele(consommer(&conn, &annule.secret, "p", 5, debut)),
            "code d'inscription annulé"
        );
        // Un nouveau code pour le même nom annule le précédent.
        let premier = creer_code(&conn, "C", debut).unwrap();
        let second = creer_code(&conn, "C", debut).unwrap();
        assert_eq!(
            modele(consommer(&conn, &premier.secret, "p", 5, debut)),
            "code d'inscription annulé"
        );
        assert!(
            consommer(&conn, &second.secret, "p", 5, debut)
                .unwrap()
                .is_ok()
        );
    }

    #[test]
    fn nombre_de_postes_limite() {
        let conn = ouvrir_en_memoire().unwrap();
        let debut = t("2026-11-12T10:00:00Z");
        let a = creer_code(&conn, "A", debut).unwrap();
        let b = creer_code(&conn, "B", debut).unwrap();
        assert!(consommer(&conn, &a.secret, "pa", 1, debut).unwrap().is_ok());
        assert_eq!(
            modele(consommer(&conn, &b.secret, "pb", 1, debut)),
            "nombre de postes atteint : {} postes permis par l'abonnement"
        );
        // Refusé sans rien écrire : le code reste utilisable.
        assert_eq!(codes_en_cours(&conn, debut).unwrap().len(), 1);
        assert!(consommer(&conn, &b.secret, "pb", 2, debut).unwrap().is_ok());
    }
}
