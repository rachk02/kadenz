//! Serveur auquel ce poste est relié (phase 2, jalon F4 ; écrans, section 2) :
//! adresse, empreinte du certificat du serveur (épinglage), entreprise, nom du
//! poste et identité TLS du poste. La clé privée du poste est protégée par
//! Windows pour le compte de la personne, comme `kadenz.cle`.

use rusqlite::Connection;
use serde_json::{Value, json};

use crate::chiffrement::{self, Portee};
use crate::{Erreur, Resultat};

const CLE: &str = "poste_relie";

#[derive(Clone, PartialEq, Eq)]
pub struct PosteRelie {
    /// Adresse `machine:port` du serveur.
    pub adresse: String,
    /// Empreinte SHA-256 du certificat du serveur, en hexadécimal.
    pub empreinte_serveur: String,
    pub entreprise: String,
    /// Nom donné au poste par l'Admin à la création du code.
    pub nom: String,
    /// Certificat du poste (DER).
    pub certificat: Vec<u8>,
    /// Clé privée du poste (PKCS 8), en clair en mémoire seulement.
    pub cle: Vec<u8>,
}

impl std::fmt::Debug for PosteRelie {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // La clé privée n'apparaît jamais dans une trace.
        f.debug_struct("PosteRelie")
            .field("adresse", &self.adresse)
            .field("empreinte_serveur", &self.empreinte_serveur)
            .field("entreprise", &self.entreprise)
            .field("nom", &self.nom)
            .finish_non_exhaustive()
    }
}

fn hex(octets: &[u8]) -> String {
    octets.iter().map(|o| format!("{o:02x}")).collect()
}

fn octets(texte: &str) -> Option<Vec<u8>> {
    if !texte.len().is_multiple_of(2) {
        return None;
    }
    (0..texte.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(texte.get(i..i + 2)?, 16).ok())
        .collect()
}

/// Garde le serveur de ce poste (après l'inscription, ou un changement
/// d'adresse).
pub fn garder(conn: &Connection, poste: &PosteRelie) -> Resultat<()> {
    let protegee = chiffrement::proteger(&poste.cle, Portee::Compte)
        .ok_or_else(|| Erreur::Validation("clé du poste non protégée".into()))?;
    let valeur = json!({
        "adresse": poste.adresse,
        "empreinte_serveur": poste.empreinte_serveur,
        "entreprise": poste.entreprise,
        "nom": poste.nom,
        "certificat": hex(&poste.certificat),
        "cle": hex(&protegee),
    });
    conn.execute(
        "INSERT INTO parametre_systeme (cle, valeur) VALUES (?1, ?2)
         ON CONFLICT (cle) DO UPDATE SET valeur = excluded.valeur",
        rusqlite::params![CLE, valeur.to_string()],
    )?;
    Ok(())
}

/// Serveur de ce poste ; `None` si le poste n'est relié à aucun serveur.
/// Une clé protégée pour un autre compte Windows est une erreur.
pub fn lire(conn: &Connection) -> Resultat<Option<PosteRelie>> {
    let valeur: Option<String> = rusqlite::OptionalExtension::optional(conn.query_row(
        "SELECT valeur FROM parametre_systeme WHERE cle = ?1",
        [CLE],
        |r| r.get(0),
    ))?;
    let Some(valeur) = valeur else {
        return Ok(None);
    };
    let illisible = || Erreur::Validation("serveur du poste illisible".into());
    let v: Value = serde_json::from_str(&valeur).map_err(|_| illisible())?;
    let texte = |champ: &str| v[champ].as_str().map(str::to_owned).ok_or_else(illisible);
    let protegee = octets(&texte("cle")?).ok_or_else(illisible)?;
    let cle = chiffrement::deproteger(&protegee).ok_or(Erreur::CleAutreCompte)?;
    Ok(Some(PosteRelie {
        adresse: texte("adresse")?,
        empreinte_serveur: texte("empreinte_serveur")?,
        entreprise: texte("entreprise")?,
        nom: texte("nom")?,
        certificat: octets(&texte("certificat")?).ok_or_else(illisible)?,
        cle,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn garde_et_relit() {
        let conn = crate::ouvrir_en_memoire().unwrap();
        assert!(lire(&conn).unwrap().is_none());
        let poste = PosteRelie {
            adresse: "serveur:47380".into(),
            empreinte_serveur: "ab".repeat(32),
            entreprise: "e".into(),
            nom: "Comptabilité".into(),
            certificat: vec![1, 2, 3],
            cle: vec![9, 8, 7],
        };
        garder(&conn, &poste).unwrap();
        assert_eq!(lire(&conn).unwrap().unwrap(), poste);
        // La clé n'est pas en clair dans la base.
        let brut: String = conn
            .query_row(
                "SELECT valeur FROM parametre_systeme WHERE cle = ?1",
                [CLE],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!brut.contains("\"cle\":\"090807\""));
        assert!(!format!("{poste:?}").contains("cle"));
    }
}
