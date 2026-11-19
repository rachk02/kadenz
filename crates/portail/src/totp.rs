//! Codes à usage unique basés sur l'heure (TOTP, RFC 6238, sur HOTP de la
//! RFC 4226) pour la double authentification des éditeurs : HMAC-SHA-1, pas
//! de 30 secondes, 6 chiffres, les réglages par défaut que lisent les
//! applications d'authentification. Le secret compte 20 octets (160 bits,
//! longueur conseillée par la RFC 4226, section 4) et s'affiche en Base32.

use data_encoding::BASE32_NOPAD;
use hmac::{Hmac, Mac};
use sha1::Sha1;
use subtle::ConstantTimeEq;

/// Durée d'un pas, en secondes.
pub const PAS_SECONDES: u64 = 30;
/// Chiffres d'un code.
pub const CHIFFRES: u32 = 6;
/// Taille du secret, en octets.
pub(crate) const OCTETS_SECRET: usize = 20;

/// HOTP (RFC 4226, section 5.3) : HMAC-SHA-1 du compteur, troncature
/// dynamique, puis les `chiffres` derniers chiffres.
pub(crate) fn hotp(secret: &[u8], compteur: u64, chiffres: u32) -> u32 {
    let mut mac =
        Hmac::<Sha1>::new_from_slice(secret).expect("HMAC accepte une clé de toute longueur");
    mac.update(&compteur.to_be_bytes());
    let h = mac.finalize().into_bytes();
    let decalage = usize::from(h[19] & 0x0f);
    let binaire = ((u32::from(h[decalage]) & 0x7f) << 24)
        | (u32::from(h[decalage + 1]) << 16)
        | (u32::from(h[decalage + 2]) << 8)
        | u32::from(h[decalage + 3]);
    binaire % 10u32.pow(chiffres)
}

/// Secret en Base32 (sans remplissage).
pub(crate) fn encoder(octets: &[u8]) -> String {
    BASE32_NOPAD.encode(octets)
}

/// Secret lu en Base32 : espaces et tirets ignorés, casse indifférente.
pub fn decoder(secret: &str) -> Option<Vec<u8>> {
    let propre: String = secret
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect::<String>()
        .to_ascii_uppercase();
    BASE32_NOPAD.decode(propre.as_bytes()).ok()
}

/// Secret présenté en groupes de quatre caractères, pour la saisie à la main.
pub fn groupes(secret: &str) -> String {
    secret
        .as_bytes()
        .chunks(4)
        .map(|g| String::from_utf8_lossy(g).into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Code attendu à l'instant `unix` (secondes depuis 1970) pour un secret en
/// Base32 ; `None` si le secret est illisible.
pub fn code_pour(secret: &str, unix: u64) -> Option<String> {
    let octets = decoder(secret)?;
    Some(format!(
        "{:06}",
        hotp(&octets, unix / PAS_SECONDES, CHIFFRES)
    ))
}

/// Code saisi vérifié à l'instant `unix`, avec un pas d'écart permis de
/// chaque côté (horloge du téléphone). Un pas déjà accepté ou plus ancien
/// (`dernier_pas`) est refusé : un code ne sert qu'une fois. Rend le pas
/// accepté.
pub(crate) fn verifier(secret: &[u8], saisi: &str, unix: u64, dernier_pas: i64) -> Option<i64> {
    if saisi.len() != CHIFFRES as usize || !saisi.bytes().all(|o| o.is_ascii_digit()) {
        return None;
    }
    let actuel = i64::try_from(unix / PAS_SECONDES).ok()?;
    let mut accepte = None;
    for pas in [actuel - 1, actuel, actuel + 1] {
        if pas <= dernier_pas || pas < 0 {
            continue;
        }
        let attendu = format!("{:06}", hotp(secret, pas as u64, CHIFFRES));
        if bool::from(attendu.as_bytes().ct_eq(saisi.as_bytes())) && accepte.is_none() {
            accepte = Some(pas);
        }
    }
    accepte
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET_RFC: &[u8] = b"12345678901234567890";

    /// RFC 4226, annexe D : HOTP à 6 chiffres, compteurs 0 à 9.
    #[test]
    fn vecteurs_hotp_rfc_4226() {
        let attendus = [
            755224, 287082, 359152, 969429, 338314, 254676, 287922, 162583, 399871, 520489,
        ];
        for (compteur, attendu) in attendus.into_iter().enumerate() {
            assert_eq!(hotp(SECRET_RFC, compteur as u64, 6), attendu, "{compteur}");
        }
    }

    /// RFC 6238, annexe B : TOTP SHA-1 à 8 chiffres, pas de 30 secondes.
    #[test]
    fn vecteurs_totp_rfc_6238() {
        let attendus: [(u64, u32); 6] = [
            (59, 94287082),
            (1111111109, 7081804),
            (1111111111, 14050471),
            (1234567890, 89005924),
            (2000000000, 69279037),
            (20000000000, 65353130),
        ];
        for (unix, attendu) in attendus {
            assert_eq!(hotp(SECRET_RFC, unix / PAS_SECONDES, 8), attendu, "{unix}");
        }
    }

    #[test]
    fn base32_groupes_et_code() {
        let secret = encoder(SECRET_RFC);
        assert_eq!(secret, "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ");
        assert_eq!(&groupes(&secret)[..9], "GEZD GNBV");
        assert_eq!(
            decoder(&groupes(&secret).to_lowercase()).unwrap(),
            SECRET_RFC
        );
        // 59 s : compteur 1, 287082 en 6 chiffres (RFC 4226).
        assert_eq!(code_pour(&secret, 59).as_deref(), Some("287082"));
        assert_eq!(code_pour("pas du base32 !", 59), None);
    }

    #[test]
    fn fenetre_et_rejeu() {
        let unix = 1_800_000_000;
        let actuel = (unix / PAS_SECONDES) as i64;
        let code = |pas: i64| format!("{:06}", hotp(SECRET_RFC, pas as u64, 6));
        assert_eq!(verifier(SECRET_RFC, &code(actuel), unix, 0), Some(actuel));
        assert_eq!(
            verifier(SECRET_RFC, &code(actuel - 1), unix, 0),
            Some(actuel - 1)
        );
        assert_eq!(
            verifier(SECRET_RFC, &code(actuel + 1), unix, 0),
            Some(actuel + 1)
        );
        assert_eq!(verifier(SECRET_RFC, &code(actuel + 2), unix, 0), None);
        assert_eq!(verifier(SECRET_RFC, &code(actuel - 2), unix, 0), None);
        // Déjà accepté : refusé.
        assert_eq!(verifier(SECRET_RFC, &code(actuel), unix, actuel), None);
        // Saisie mal formée.
        assert_eq!(verifier(SECRET_RFC, "12345", unix, 0), None);
        assert_eq!(verifier(SECRET_RFC, "12a456", unix, 0), None);
    }
}
