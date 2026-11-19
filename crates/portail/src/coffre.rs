//! Chiffrement des secrets de double authentification (audit d'OpenCode du
//! 2026-11-19, lot B) : une lecture du fichier de la base ne donne plus les
//! secrets TOTP.
//!
//! - Clé : HKDF-SHA256 de la graine Ed25519 du portail, étiquette
//!   `kadenz-portail-totp-1` ; aucun secret de plus à garder. Changer la clé
//!   du portail rend les secrets illisibles : le compte passe alors par
//!   `retirer-double-authentification`.
//! - Chiffrement authentifié ChaCha20-Poly1305 (ring, déjà dans l'arbre par
//!   rustls), nonce aléatoire de 12 octets, l'identifiant du compte en donnée
//!   associée : un secret recopié sur un autre compte ne se déchiffre pas.
//! - Forme stockée : `c1:` puis l'hexadécimal de nonce, chiffré et étiquette.

use ed25519_dalek::SigningKey;
use ring::{aead, hkdf};
use zeroize::Zeroizing;

/// Préfixe d'un secret chiffré (version 1 du format).
pub(crate) const PREFIXE: &str = "c1:";

/// Séparation de domaine : cette clé ne sert à rien d'autre.
const ETIQUETTE: &[u8] = b"kadenz-portail-totp-1";

const NONCE: usize = 12;

pub(crate) struct Coffre(aead::LessSafeKey);

impl Coffre {
    pub(crate) fn depuis_cle(cle: &SigningKey) -> Coffre {
        let graine = Zeroizing::new(cle.to_bytes());
        let prk = hkdf::Salt::new(hkdf::HKDF_SHA256, &[]).extract(&graine[..]);
        let okm = prk
            .expand(&[ETIQUETTE], &aead::CHACHA20_POLY1305)
            .expect("longueur de la clé ChaCha20-Poly1305 permise par HKDF");
        Coffre(aead::LessSafeKey::new(aead::UnboundKey::from(okm)))
    }

    /// Secret (Base32) chiffré pour ce compte.
    pub(crate) fn chiffrer(&self, compte: i64, secret: &str) -> Result<String, String> {
        let mut nonce = [0u8; NONCE];
        getrandom::fill(&mut nonce).map_err(|e| e.to_string())?;
        let mut tampon = secret.as_bytes().to_vec();
        self.0
            .seal_in_place_append_tag(
                aead::Nonce::assume_unique_for_key(nonce),
                aead::Aad::from(compte.to_le_bytes()),
                &mut tampon,
            )
            .map_err(|_| "chiffrement du secret impossible".to_string())?;
        let mut tout = nonce.to_vec();
        tout.extend_from_slice(&tampon);
        Ok(format!(
            "{PREFIXE}{}",
            data_encoding::HEXLOWER.encode(&tout)
        ))
    }

    /// Secret (Base32) d'un compte ; `None` si la valeur n'est pas un secret
    /// chiffré pour ce compte avec cette clé (autre compte, autre clé,
    /// valeur altérée).
    pub(crate) fn dechiffrer(&self, compte: i64, valeur: &str) -> Option<String> {
        let hexa = valeur.strip_prefix(PREFIXE)?;
        let tout = data_encoding::HEXLOWER.decode(hexa.as_bytes()).ok()?;
        if tout.len() < NONCE + aead::CHACHA20_POLY1305.tag_len() {
            return None;
        }
        let (nonce, chiffre) = tout.split_at(NONCE);
        let nonce = aead::Nonce::try_assume_unique_for_key(nonce).ok()?;
        let mut tampon = Zeroizing::new(chiffre.to_vec());
        let clair = self
            .0
            .open_in_place(nonce, aead::Aad::from(compte.to_le_bytes()), &mut tampon)
            .ok()?;
        String::from_utf8(clair.to_vec()).ok()
    }
}

/// Valeur déjà chiffrée (format reconnu par son préfixe).
pub(crate) fn chiffre(valeur: &str) -> bool {
    valeur.starts_with(PREFIXE)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP";

    fn coffre(octet: u8) -> Coffre {
        Coffre::depuis_cle(&SigningKey::from_bytes(&[octet; 32]))
    }

    #[test]
    fn aller_retour() {
        let c = coffre(7);
        let valeur = c.chiffrer(3, SECRET).unwrap();
        assert!(chiffre(&valeur));
        assert!(!valeur.contains(SECRET));
        assert_eq!(c.dechiffrer(3, &valeur).as_deref(), Some(SECRET));
        // Nonce aléatoire : deux chiffrements diffèrent.
        assert_ne!(valeur, c.chiffrer(3, SECRET).unwrap());
    }

    /// Recopié sur un autre compte, altéré, ou lu avec une autre clé : refusé.
    #[test]
    fn autre_compte_autre_cle_ou_altere_refuse() {
        let c = coffre(7);
        let valeur = c.chiffrer(3, SECRET).unwrap();
        assert_eq!(c.dechiffrer(4, &valeur), None);
        assert_eq!(coffre(8).dechiffrer(3, &valeur), None);
        let mut altere = valeur.clone();
        let dernier = altere.pop().unwrap();
        altere.push(if dernier == '0' { '1' } else { '0' });
        assert_eq!(c.dechiffrer(3, &altere), None);
        assert_eq!(c.dechiffrer(3, SECRET), None);
        assert_eq!(c.dechiffrer(3, "c1:00"), None);
    }
}
