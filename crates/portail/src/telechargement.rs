//! Téléchargement des installateurs et des mises à jour (décision 77,
//! `docs/spec-telechargements-essai.md`, sections 3 et 4).
//!
//! Les fichiers sont rangés à plat dans le dépôt du portail (option
//! `--depot`), chacun avec sa version dans son nom :
//! `KadenzSetup-X.Y.Z.exe` (Kadenz Entreprise), `KadenzMonoposteSetup-X.Y.Z.exe`
//! (édition un poste), `KadenzServeur-X.Y.Z.msi`, `Kadenz-X.Y.Z.msi`, le
//! manifeste signé `kadenz-maj-X.Y.Z.json` et sa signature
//! `kadenz-maj-X.Y.Z.json.sig`, les notes `notes-X.Y.Z.md`. Le portail donne
//! un lien signé (HMAC-SHA-256 du nom et de l'échéance) valable dix minutes,
//! qui peut resservir dans ce délai pour reprendre un téléchargement coupé.
//! Aucun chemin n'est pris dans la requête : un nom n'est servi que s'il
//! suit l'un de ces modèles.

use std::path::Path;

use hmac::{Hmac, Mac};
use kadenz_licence::entreprise::Produit;
use sha2::Sha256;

/// Durée d'un lien de téléchargement, en secondes.
pub const DUREE_LIEN: i64 = 10 * 60;

/// Version `X.Y.Z` lue dans un nom de fichier.
pub type Version = (u32, u32, u32);

/// Sorte d'un fichier du dépôt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sorte {
    /// Installateur de Kadenz Entreprise (poste).
    Poste,
    /// Installateur de l'édition un poste.
    Monoposte,
    /// Paquet du serveur.
    Serveur,
    /// Paquet MSI du poste.
    PaquetPoste,
    /// Manifeste de mise à jour signé.
    Manifeste,
    /// Signature du manifeste.
    Signature,
    /// Notes de version, lues par le portail, jamais servies en fichier.
    Notes,
}

/// Modèles des noms : préfixe, suffixe, sorte. La signature passe avant le
/// manifeste, dont le suffixe est plus court.
const MODELES: [(&str, &str, Sorte); 7] = [
    ("KadenzSetup-", ".exe", Sorte::Poste),
    ("KadenzMonoposteSetup-", ".exe", Sorte::Monoposte),
    ("KadenzServeur-", ".msi", Sorte::Serveur),
    ("Kadenz-", ".msi", Sorte::PaquetPoste),
    ("kadenz-maj-", ".json.sig", Sorte::Signature),
    ("kadenz-maj-", ".json", Sorte::Manifeste),
    ("notes-", ".md", Sorte::Notes),
];

fn lire_version(texte: &str) -> Option<Version> {
    let parties: Vec<&str> = texte.split('.').collect();
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
    Some((v[0], v[1], v[2]))
}

/// Sorte et version d'un nom de fichier du dépôt ; `None` pour tout autre nom
/// (chemin, extension, version mal formée).
pub fn lire_fichier(nom: &str) -> Option<(Sorte, Version)> {
    MODELES.iter().find_map(|(prefixe, suffixe, sorte)| {
        let reste = nom.strip_prefix(prefixe)?.strip_suffix(suffixe)?;
        Some((*sorte, lire_version(reste)?))
    })
}

/// Nom du fichier d'une sorte pour une version.
pub fn nom_fichier(sorte: Sorte, version: Version) -> String {
    let (prefixe, suffixe, _) = MODELES
        .iter()
        .find(|(_, _, s)| *s == sorte)
        .expect("chaque sorte a son modèle");
    format!(
        "{prefixe}{}.{}.{}{suffixe}",
        version.0, version.1, version.2
    )
}

/// Édition et version d'un nom d'installateur ; `None` pour tout autre nom.
pub fn lire_nom(nom: &str) -> Option<(Produit, Version)> {
    match lire_fichier(nom)? {
        (Sorte::Poste, v) => Some((Produit::Entreprise, v)),
        (Sorte::Monoposte, v) => Some((Produit::Monoposte, v)),
        _ => None,
    }
}

/// Fichiers du dépôt dont le nom suit un modèle.
fn fichiers(depot: &Path) -> Vec<(Sorte, Version, String)> {
    let Ok(lecture) = std::fs::read_dir(depot) else {
        return Vec::new();
    };
    lecture
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .filter_map(|e| e.file_name().into_string().ok())
        .filter_map(|nom| lire_fichier(&nom).map(|(s, v)| (s, v, nom)))
        .collect()
}

/// Dernier installateur d'une édition présent dans le dépôt.
pub fn dernier(depot: &Path, produit: Produit) -> Option<String> {
    fichiers(depot)
        .into_iter()
        .filter_map(|(sorte, version, nom)| {
            let p = match sorte {
                Sorte::Poste => Produit::Entreprise,
                Sorte::Monoposte => Produit::Monoposte,
                _ => return None,
            };
            (p == produit).then_some((version, nom))
        })
        .max()
        .map(|(_, nom)| nom)
}

/// Version publiée : manifeste et signature présents, avec ses autres fichiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Publication {
    pub version: Version,
    /// Fichiers servis de cette version (manifeste et signature compris).
    pub fichiers: Vec<(Sorte, String)>,
    /// Notes de version, s'il y en a.
    pub notes: Option<String>,
}

/// Versions publiées dans le dépôt, de la plus récente à la plus ancienne,
/// `maximum` au plus. Une version sans manifeste ou sans signature n'est pas
/// publiée.
pub fn publications(depot: &Path, maximum: usize) -> Vec<Publication> {
    let tous = fichiers(depot);
    let mut versions: Vec<Version> = tous
        .iter()
        .filter(|(s, _, _)| *s == Sorte::Manifeste)
        .map(|(_, v, _)| *v)
        .filter(|v| {
            tous.iter()
                .any(|(s, w, _)| *s == Sorte::Signature && w == v)
        })
        .collect();
    versions.sort_unstable_by(|a, b| b.cmp(a));
    versions.dedup();
    versions
        .into_iter()
        .take(maximum)
        .map(|version| {
            let notes = std::fs::read(depot.join(nom_fichier(Sorte::Notes, version)))
                .ok()
                .filter(|o| o.len() <= 64 * 1024)
                .and_then(|o| String::from_utf8(o).ok());
            let mut fichiers: Vec<(Sorte, String)> = tous
                .iter()
                .filter(|(s, v, _)| *v == version && *s != Sorte::Notes)
                .map(|(s, _, nom)| (*s, nom.clone()))
                .collect();
            fichiers.sort_by(|a, b| a.1.cmp(&b.1));
            Publication {
                version,
                fichiers,
                notes,
            }
        })
        .collect()
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
        let servi = lire_fichier(nom).is_some_and(|(sorte, _)| sorte != Sorte::Notes);
        if !servi || echeance < maintenant || echeance > maintenant + DUREE_LIEN {
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
        assert_eq!(
            lire_fichier("kadenz-maj-0.3.4.json.sig"),
            Some((Sorte::Signature, (0, 3, 4)))
        );
        assert_eq!(
            lire_fichier("kadenz-maj-0.3.4.json"),
            Some((Sorte::Manifeste, (0, 3, 4)))
        );
        assert_eq!(
            lire_fichier("Kadenz-0.3.4.msi"),
            Some((Sorte::PaquetPoste, (0, 3, 4)))
        );
        assert_eq!(lire_nom("KadenzServeur-0.3.4.msi"), None);
        assert_eq!(
            nom_fichier(Sorte::Signature, (1, 2, 3)),
            "kadenz-maj-1.2.3.json.sig"
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
    fn versions_publiees() {
        let dossier = std::env::temp_dir().join(format!("kadenz-depot-pub-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(&dossier).unwrap();
        for nom in [
            "kadenz-maj-0.3.4.json",
            "kadenz-maj-0.3.4.json.sig",
            "KadenzSetup-0.3.4.exe",
            "notes-0.3.4.md",
            "kadenz-maj-0.3.3.json",
            "kadenz-maj-0.3.3.json.sig",
            // Sans signature : pas publiée.
            "kadenz-maj-0.3.5.json",
        ] {
            std::fs::write(dossier.join(nom), b"x").unwrap();
        }
        let liste = publications(&dossier, 30);
        assert_eq!(
            liste.iter().map(|p| p.version).collect::<Vec<_>>(),
            vec![(0, 3, 4), (0, 3, 3)]
        );
        assert_eq!(liste[0].notes.as_deref(), Some("x"));
        assert_eq!(
            liste[0]
                .fichiers
                .iter()
                .map(|f| f.1.as_str())
                .collect::<Vec<_>>(),
            vec![
                "KadenzSetup-0.3.4.exe",
                "kadenz-maj-0.3.4.json",
                "kadenz-maj-0.3.4.json.sig"
            ]
        );
        assert_eq!(publications(&dossier, 1).len(), 1);
        std::fs::remove_dir_all(&dossier).unwrap();
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
        // Les notes ne sont pas servies en fichier.
        let notes = "notes-0.3.4.md";
        assert!(!s.verifier(notes, echeance, &s.signer(notes, echeance), 1_000));
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
