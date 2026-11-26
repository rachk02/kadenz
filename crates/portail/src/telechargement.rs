//! Téléchargement des installateurs (décision 77,
//! `docs/spec-telechargements-essai.md`, section 3).
//!
//! Les paquets sont rangés à plat dans le dépôt du portail (option
//! `--depot`) : `KadenzSetup-X.Y.Z.exe` pour Kadenz Entreprise,
//! `KadenzMonoposteSetup-X.Y.Z.exe` pour l'édition un poste. Le portail donne
//! un lien signé (HMAC-SHA-256 du nom et de l'échéance) valable dix minutes,
//! qui peut resservir dans ce délai pour reprendre un téléchargement coupé.
//! Aucun chemin n'est pris dans la requête : un nom n'est servi que s'il
//! suit le modèle d'un paquet.

use std::path::Path;

use hmac::{Hmac, Mac};
use kadenz_licence::entreprise::Produit;
use sha2::Sha256;

/// Durée d'un lien de téléchargement, en secondes.
pub const DUREE_LIEN: i64 = 10 * 60;

const SUFFIXE: &str = ".exe";

/// Préfixe du nom des paquets d'une édition.
fn prefixe(produit: Produit) -> &'static str {
    match produit {
        Produit::Entreprise => "KadenzSetup-",
        Produit::Monoposte => "KadenzMonoposteSetup-",
    }
}

/// Version `X.Y.Z` d'un nom de paquet, et son édition ; `None` pour tout
/// autre nom (chemin, extension, version mal formée).
pub fn lire_nom(nom: &str) -> Option<(Produit, (u32, u32, u32))> {
    for produit in [Produit::Entreprise, Produit::Monoposte] {
        let Some(reste) = nom
            .strip_prefix(prefixe(produit))
            .and_then(|r| r.strip_suffix(SUFFIXE))
        else {
            continue;
        };
        let parties: Vec<&str> = reste.split('.').collect();
        if parties.len() != 3 {
            return None;
        }
        let mut v = [0u32; 3];
        for (i, p) in parties.iter().enumerate() {
            if p.is_empty() || p.len() > 6 || !p.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            v[i] = p.parse().ok()?;
        }
        return Some((produit, (v[0], v[1], v[2])));
    }
    None
}

/// Dernier paquet d'une édition présent dans le dépôt.
pub fn dernier(depot: &Path, produit: Produit) -> Option<String> {
    std::fs::read_dir(depot)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .filter_map(|e| e.file_name().into_string().ok())
        .filter_map(|nom| match lire_nom(&nom) {
            Some((p, version)) if p == produit => Some((version, nom)),
            _ => None,
        })
        .max()
        .map(|(_, nom)| nom)
}

/// Signataire des liens : clé tirée au démarrage du portail. Un redémarrage
/// invalide les liens en cours ; la page de téléchargement en redonne un.
#[derive(Clone)]
pub struct Signataire([u8; 32]);

impl Signataire {
    pub fn nouveau() -> Result<Signataire, String> {
        let mut cle = [0u8; 32];
        getrandom::fill(&mut cle).map_err(|e| e.to_string())?;
        Ok(Signataire(cle))
    }

    #[doc(hidden)]
    pub fn depuis(cle: [u8; 32]) -> Signataire {
        Signataire(cle)
    }

    fn mac(&self, nom: &str, echeance: i64) -> Hmac<Sha256> {
        let mut mac =
            <Hmac<Sha256> as Mac>::new_from_slice(&self.0).expect("clé HMAC de 32 octets");
        mac.update(nom.as_bytes());
        mac.update(b"\n");
        mac.update(echeance.to_string().as_bytes());
        mac
    }

    /// Signature en hexadécimal du nom et de l'échéance.
    pub fn signer(&self, nom: &str, echeance: i64) -> String {
        self.mac(nom, echeance)
            .finalize()
            .into_bytes()
            .iter()
            .map(|o| format!("{o:02x}"))
            .collect()
    }

    /// Lien relatif d'un paquet, valable `DUREE_LIEN` secondes depuis `maintenant`.
    pub fn lien(&self, nom: &str, maintenant: i64) -> String {
        let echeance = maintenant + DUREE_LIEN;
        format!(
            "/fichiers/{nom}?exp={echeance}&sig={}",
            self.signer(nom, echeance)
        )
    }

    /// Vrai si le lien est signé par ce portail, pour un nom de paquet, et
    /// pas échu. Comparaison en temps constant.
    pub fn verifier(&self, nom: &str, echeance: i64, signature: &str, maintenant: i64) -> bool {
        if lire_nom(nom).is_none() || echeance < maintenant || echeance > maintenant + DUREE_LIEN {
            return false;
        }
        let Some(octets) = hexa(signature) else {
            return false;
        };
        self.mac(nom, echeance).verify_slice(&octets).is_ok()
    }
}

fn hexa(texte: &str) -> Option<Vec<u8>> {
    if texte.len() != 64 {
        return None;
    }
    (0..texte.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(texte.get(i..i + 2)?, 16).ok())
        .collect()
}

/// Partie demandée d'un fichier (en-tête `Range`, une seule plage).
#[derive(Debug, PartialEq, Eq)]
pub enum Plage {
    /// Pas d'en-tête, ou un en-tête ignoré : tout le fichier.
    Tout,
    /// Octets de `debut` à `fin` compris.
    Partie(u64, u64),
    /// Plage hors du fichier : 416.
    Hors,
}

/// Lit l'en-tête `Range` pour un fichier de `taille` octets. Seules les
/// formes `bytes=a-`, `bytes=a-b` et `bytes=-n` sont prises ; plusieurs
/// plages ou une forme inconnue donnent tout le fichier.
pub fn plage(entete: Option<&str>, taille: u64) -> Plage {
    let Some(spec) = entete.and_then(|e| e.trim().strip_prefix("bytes=")) else {
        return Plage::Tout;
    };
    if spec.contains(',') {
        return Plage::Tout;
    }
    let Some((a, b)) = spec.split_once('-') else {
        return Plage::Tout;
    };
    let (a, b) = (a.trim(), b.trim());
    let nombre = |t: &str| t.parse::<u64>().ok();
    let (debut, fin) = match (a.is_empty(), b.is_empty()) {
        // Les n derniers octets.
        (true, false) => match nombre(b) {
            Some(0) => return Plage::Hors,
            Some(n) => (taille.saturating_sub(n), taille.saturating_sub(1)),
            None => return Plage::Tout,
        },
        (false, true) => match nombre(a) {
            Some(d) => (d, taille.saturating_sub(1)),
            None => return Plage::Tout,
        },
        (false, false) => match (nombre(a), nombre(b)) {
            (Some(d), Some(f)) if d <= f => (d, f.min(taille.saturating_sub(1))),
            (Some(_), Some(_)) => return Plage::Hors,
            _ => return Plage::Tout,
        },
        (true, true) => return Plage::Tout,
    };
    if taille == 0 || debut >= taille {
        return Plage::Hors;
    }
    Plage::Partie(debut, fin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noms_de_paquets() {
        assert_eq!(
            lire_nom("KadenzSetup-0.3.4.exe"),
            Some((Produit::Entreprise, (0, 3, 4)))
        );
        assert_eq!(
            lire_nom("KadenzMonoposteSetup-1.10.2.exe"),
            Some((Produit::Monoposte, (1, 10, 2)))
        );
        for faux in [
            "KadenzSetup-0.3.exe",
            "KadenzSetup-0.3.4.msi",
            "../KadenzSetup-0.3.4.exe",
            "KadenzSetup-0.3.4.exe/..",
            "KadenzSetup-0.3.a.exe",
            "KadenzSetup-0..4.exe",
            "KadenzSetup-+1.3.4.exe",
            "kadenz.db",
            "",
        ] {
            assert_eq!(lire_nom(faux), None, "{faux}");
        }
    }

    #[test]
    fn dernier_paquet_par_edition() {
        let dossier = std::env::temp_dir().join(format!("kadenz-depot-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(&dossier).unwrap();
        for nom in [
            "KadenzSetup-0.3.4.exe",
            "KadenzSetup-0.10.0.exe",
            "KadenzSetup-0.9.9.exe",
            "KadenzMonoposteSetup-0.3.4.exe",
            "notes.txt",
        ] {
            std::fs::write(dossier.join(nom), b"x").unwrap();
        }
        // Un dossier au nom de paquet n'est pas un paquet.
        std::fs::create_dir_all(dossier.join("KadenzSetup-9.9.9.exe")).unwrap();
        assert_eq!(
            dernier(&dossier, Produit::Entreprise).as_deref(),
            Some("KadenzSetup-0.10.0.exe")
        );
        assert_eq!(
            dernier(&dossier, Produit::Monoposte).as_deref(),
            Some("KadenzMonoposteSetup-0.3.4.exe")
        );
        std::fs::remove_dir_all(&dossier).unwrap();
        assert_eq!(dernier(&dossier, Produit::Entreprise), None);
    }

    #[test]
    fn liens_signes() {
        let s = Signataire::depuis([3u8; 32]);
        let nom = "KadenzSetup-0.3.4.exe";
        let lien = s.lien(nom, 1_000);
        let echeance = 1_000 + DUREE_LIEN;
        let sig = s.signer(nom, echeance);
        assert_eq!(lien, format!("/fichiers/{nom}?exp={echeance}&sig={sig}"));
        assert!(s.verifier(nom, echeance, &sig, 1_000));
        // Resert dans le délai, plus après.
        assert!(s.verifier(nom, echeance, &sig, echeance));
        assert!(!s.verifier(nom, echeance, &sig, echeance + 1));
        // Autre nom, autre échéance, autre clé, signature abîmée.
        assert!(!s.verifier("KadenzSetup-0.3.5.exe", echeance, &sig, 1_000));
        assert!(!s.verifier(nom, echeance + 60, &sig, 1_000));
        assert!(!Signataire::depuis([4u8; 32]).verifier(nom, echeance, &sig, 1_000));
        assert!(!s.verifier(nom, echeance, &sig[..63], 1_000));
        assert!(!s.verifier(nom, echeance, "zz", 1_000));
        // Échéance trop lointaine, même bien signée : refusée.
        let loin = 1_000 + DUREE_LIEN * 10;
        assert!(!s.verifier(nom, loin, &s.signer(nom, loin), 1_000));
        // Un nom hors modèle n'est jamais servi, même signé.
        assert!(!s.verifier(
            "kadenz.db",
            echeance,
            &s.signer("kadenz.db", echeance),
            1_000
        ));
    }

    #[test]
    fn plages() {
        assert_eq!(plage(None, 100), Plage::Tout);
        assert_eq!(plage(Some("bytes=10-"), 100), Plage::Partie(10, 99));
        assert_eq!(plage(Some("bytes=10-19"), 100), Plage::Partie(10, 19));
        assert_eq!(plage(Some("bytes=90-200"), 100), Plage::Partie(90, 99));
        assert_eq!(plage(Some("bytes=-10"), 100), Plage::Partie(90, 99));
        assert_eq!(plage(Some("bytes=-500"), 100), Plage::Partie(0, 99));
        assert_eq!(plage(Some("bytes=100-"), 100), Plage::Hors);
        assert_eq!(plage(Some("bytes=20-10"), 100), Plage::Hors);
        assert_eq!(plage(Some("bytes=-0"), 100), Plage::Hors);
        assert_eq!(plage(Some("bytes=0-1,5-6"), 100), Plage::Tout);
        assert_eq!(plage(Some("items=0-1"), 100), Plage::Tout);
        assert_eq!(plage(Some("bytes=a-b"), 100), Plage::Tout);
    }
}
