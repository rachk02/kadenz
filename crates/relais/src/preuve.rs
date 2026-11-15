//! Preuve de détention d'une clé : le relais envoie un défi, le poste ou le
//! serveur signe un texte qui contient son rôle, l'entreprise et ce défi
//! avec la clé Ed25519 de son certificat TLS. Un certificat seul est public
//! et pourrait être rejoué ; la signature d'un défi neuf ne le peut pas.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use rustls::SignatureScheme;
use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};
use sha2::{Digest, Sha256};

pub const ROLE_POSTE: &str = "poste";
pub const ROLE_SERVEUR: &str = "serveur";

/// Texte signé : rôle, entreprise et défi.
pub fn message(role: &str, entreprise: &str, defi: &[u8]) -> Vec<u8> {
    let mut texte = format!("kadenz-relais/{role}/1\n{entreprise}\n").into_bytes();
    texte.extend_from_slice(defi);
    texte
}

/// Signe avec une clé privée PKCS 8 Ed25519.
pub fn signer(cle_pkcs8: &[u8], message: &[u8]) -> Result<Vec<u8>, String> {
    let cle = PrivatePkcs8KeyDer::from(cle_pkcs8.to_vec());
    let cle = rustls::crypto::ring::sign::any_eddsa_type(&cle).map_err(|e| e.to_string())?;
    let signataire = cle
        .choose_scheme(&[SignatureScheme::ED25519])
        .ok_or_else(|| "la clé n'est pas une clé Ed25519".to_string())?;
    signataire.sign(message).map_err(|e| e.to_string())
}

/// Vérifie la signature avec la clé publique du certificat (DER).
pub fn verifier(certificat: &[u8], message: &[u8], signature: &[u8]) -> bool {
    let der = CertificateDer::from(certificat);
    let Ok(certificat) = webpki::EndEntityCert::try_from(&der) else {
        return false;
    };
    certificat
        .verify_signature(webpki::ring::ED25519, message, signature)
        .is_ok()
}

/// Empreinte SHA-256 d'un certificat, en hexadécimal minuscule (comme
/// `kadenz_client::empreinte`).
pub fn empreinte(certificat: &[u8]) -> String {
    Sha256::digest(certificat)
        .iter()
        .map(|o| format!("{o:02x}"))
        .collect()
}

pub(crate) fn alea<const N: usize>() -> std::io::Result<[u8; N]> {
    let mut octets = [0u8; N];
    getrandom::fill(&mut octets).map_err(|e| std::io::Error::other(e.to_string()))?;
    Ok(octets)
}

pub(crate) fn base64(octets: &[u8]) -> String {
    STANDARD.encode(octets)
}

pub(crate) fn depuis_base64(texte: &str) -> Option<Vec<u8>> {
    STANDARD.decode(texte).ok()
}

pub(crate) fn hex(octets: &[u8]) -> String {
    octets.iter().map(|o| format!("{o:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_verifiee_avec_le_certificat() {
        let identite = crate::tls::nouvelle_identite("essai").unwrap();
        let autre = crate::tls::nouvelle_identite("autre").unwrap();
        let m = message(ROLE_POSTE, "entreprise", b"defi");
        let s = signer(&identite.cle, &m).unwrap();
        assert!(verifier(&identite.certificat, &m, &s));
        assert!(!verifier(&autre.certificat, &m, &s));
        let m2 = message(ROLE_SERVEUR, "entreprise", b"defi");
        assert!(!verifier(&identite.certificat, &m2, &s));
        assert!(!verifier(b"pas un certificat", &m, &s));
    }
}
