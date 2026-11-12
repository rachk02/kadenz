//! Code d'inscription d'un poste (phase 2, jalon F4 ; décision 36 ;
//! spécification de synchronisation, section 7 ; écrans, section 2).
//!
//! Le secret est court, pour être tapé : 12 caractères en base 32 de
//! Crockford (60 bits), présentés `XXXX-XXXX-XXXX`. Le code complet, à
//! copier-coller, porte aussi l'identifiant de l'entreprise, l'empreinte du
//! certificat du serveur et ses adresses : le poste vérifie seul qu'il parle
//! au bon serveur. Format : `KADENZ-I-<JSON en base64 URL sans remplissage>`.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};

pub const PREFIXE: &str = "KADENZ-I-";

/// Alphabet de Crockford : sans I, L, O ni U.
pub const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Longueur du secret, sans les tirets.
pub const LONGUEUR_SECRET: usize = 12;

/// Requête de découverte envoyée en diffusion UDP sur le port du serveur.
pub const DECOUVERTE: &[u8] = b"KADENZ-DECOUVERTE 1";

/// Port par défaut du serveur, en TCP (sessions) et en UDP (découverte), dans
/// une plage non attribuée du registre de l'IANA (47101 à 47556).
pub const PORT_PAR_DEFAUT: u16 = 47380;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeComplet {
    pub entreprise: String,
    /// Empreinte SHA-256 du certificat du serveur, en hexadécimal.
    pub empreinte: String,
    /// Adresses `machine:port` où joindre le serveur.
    pub adresses: Vec<String>,
    /// Secret normalisé, sans tirets.
    pub secret: String,
}

impl CodeComplet {
    pub fn encoder(&self) -> String {
        let json = serde_json::to_vec(self).unwrap_or_default();
        format!("{PREFIXE}{}", URL_SAFE_NO_PAD.encode(json))
    }

    /// Lit un code complet ; `None` s'il est illisible ou si son secret est
    /// mal formé.
    pub fn decoder(texte: &str) -> Option<CodeComplet> {
        let texte: String = texte.chars().filter(|c| !c.is_whitespace()).collect();
        let json = URL_SAFE_NO_PAD.decode(texte.strip_prefix(PREFIXE)?).ok()?;
        let mut code: CodeComplet = serde_json::from_slice(&json).ok()?;
        code.secret = normaliser_secret(&code.secret)?;
        Some(code)
    }
}

/// Secret tapé ramené à sa forme normale : majuscules, sans tirets ni
/// espaces, O lu 0, I et L lus 1 (règle de Crockford). `None` si la longueur
/// ou un caractère ne va pas.
pub fn normaliser_secret(texte: &str) -> Option<String> {
    let secret: String = texte
        .chars()
        .filter(|c| !matches!(c, '-' | ' ' | '\t'))
        .map(|c| match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            autre => autre,
        })
        .collect();
    (secret.len() == LONGUEUR_SECRET && secret.bytes().all(|o| ALPHABET.contains(&o)))
        .then_some(secret)
}

/// Présentation du secret : `XXXX-XXXX-XXXX`.
pub fn presenter_secret(secret: &str) -> String {
    secret
        .as_bytes()
        .chunks(4)
        .map(|g| String::from_utf8_lossy(g).into_owned())
        .collect::<Vec<_>>()
        .join("-")
}

/// Réponse du serveur à une requête de découverte. Elle ne donne aucune
/// confiance : elle sert seulement à trouver une adresse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServeurTrouve {
    /// Nom de la machine du serveur.
    pub machine: String,
    /// Port TCP des sessions.
    pub port: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_et_code_complet() {
        assert_eq!(
            normaliser_secret("abcd-efgh-jkmo").as_deref(),
            Some("ABCDEFGHJKM0")
        );
        assert_eq!(
            normaliser_secret("ilo1 2345 6789").as_deref(),
            Some("110123456789")
        );
        assert_eq!(normaliser_secret("ABCD-EFGH-JKU0"), None);
        assert_eq!(normaliser_secret("ABCD-EFGH"), None);
        assert_eq!(presenter_secret("ABCDEFGHJKM0"), "ABCD-EFGH-JKM0");
        let code = CodeComplet {
            entreprise: "e".into(),
            empreinte: "ab".repeat(32),
            adresses: vec!["serveur:47380".into()],
            secret: "ABCDEFGHJKM0".into(),
        };
        let texte = code.encoder();
        assert!(texte.starts_with(PREFIXE));
        assert_eq!(CodeComplet::decoder(&texte), Some(code));
        assert_eq!(CodeComplet::decoder("KADENZ-I-zz"), None);
    }
}
