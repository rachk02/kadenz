//! Mises à jour de Kadenz (cahier des charges, section 16).
//!
//! Au démarrage, l'application demande la dernière release publiée sur GitHub
//! ([`verifier`]). Si elle est plus récente, l'utilisateur peut l'installer :
//! [`telecharger`] récupère l'installateur `KadenzSetup-X.Y.Z.exe` et vérifie
//! son empreinte SHA-256 (fichier `.sha256` publié avec la release), puis
//! [`lancer`] l'ouvre ; l'application se ferme ensuite.
//!
//! Aucune donnée du poste, de la licence ou du client n'est envoyée : la seule
//! en-tête propre à Kadenz est `User-Agent: Kadenz/<version>`. Toute erreur de
//! la vérification (hors connexion, délai dépassé, réponse illisible) est
//! silencieuse.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use kadenz_domaine::message::MessageCompose;
use sha2::{Digest, Sha256};
use ureq::Agent;
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

/// Adresse de la dernière release publiée (ni brouillon ni préversion).
const DERNIERE_RELEASE: &str = "https://api.github.com/repos/rachk02/kadenz/releases/latest";

/// Version de cette application.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Variable d'environnement qui désactive la vérification (développement).
pub const SANS_MAJ: &str = "KADENZ_SANS_MAJ";

/// Taille maximale acceptée pour l'installateur.
const TAILLE_MAX: u64 = 300 * 1024 * 1024;

/// Modèles des messages d'erreur, à traduire.
pub const MODELES_ERREUR: &[&str] = &[
    "téléchargement impossible : {}",
    "la release ne contient pas d'empreinte SHA-256 de l'installateur",
    "empreinte SHA-256 illisible",
    "l'installateur téléchargé ne correspond pas à son empreinte : fichier supprimé",
    "écriture de l'installateur impossible : {}",
    "lancement de l'installateur impossible : {}",
];

/// Release plus récente que l'application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MiseAJour {
    /// Version sans le « v » (ex. 0.2.0).
    pub version: String,
    /// Page de la release sur GitHub.
    pub page: String,
    /// Notes de version (section du CHANGELOG), en Markdown.
    pub notes: String,
    pub installateur_nom: String,
    pub installateur_url: String,
    /// Adresse du fichier `.sha256` ; `None` si la release n'en a pas, auquel
    /// cas [`telecharger`] refuse.
    pub empreinte_url: Option<String>,
}

/// Version `majeure.mineure.corrective`, comparée numériquement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(pub u32, pub u32, pub u32);

impl Version {
    /// Lit « 1.2.3 » ou « v1.2.3 » ; rien d'autre (pas de suffixe).
    pub fn lire(texte: &str) -> Option<Version> {
        let t = texte.trim();
        let t = t.strip_prefix('v').unwrap_or(t);
        let mut parties = t.split('.').map(|p| p.parse::<u32>().ok());
        let v = Version(parties.next()??, parties.next()??, parties.next()??);
        parties.next().is_none().then_some(v)
    }
}

/// Lit la réponse de l'API GitHub : la release si elle est plus récente que
/// `actuelle` et qu'elle contient l'installateur attendu.
pub fn analyser(json: &str, actuelle: &str) -> Option<MiseAJour> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let tag = v["tag_name"].as_str()?;
    let distante = Version::lire(tag)?;
    if distante <= Version::lire(actuelle)? {
        return None;
    }
    let version = format!("{}.{}.{}", distante.0, distante.1, distante.2);
    let installateur_nom = format!("KadenzSetup-{version}.exe");
    let empreinte_nom = format!("{installateur_nom}.sha256");
    let actif = |nom: &str| {
        v["assets"].as_array()?.iter().find_map(|a| {
            (a["name"].as_str() == Some(nom))
                .then(|| a["browser_download_url"].as_str().map(str::to_owned))
                .flatten()
        })
    };
    Some(MiseAJour {
        installateur_url: actif(&installateur_nom)?,
        empreinte_url: actif(&empreinte_nom),
        installateur_nom,
        version,
        page: v["html_url"].as_str().unwrap_or_default().to_owned(),
        notes: v["body"].as_str().unwrap_or_default().trim().to_owned(),
    })
}

fn agent(delai_global: Option<Duration>) -> Agent {
    Agent::config_builder()
        .user_agent(format!("Kadenz/{VERSION}"))
        .timeout_connect(Some(Duration::from_secs(5)))
        .timeout_global(delai_global)
        .tls_config(
            TlsConfig::builder()
                .provider(TlsProvider::NativeTls)
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .into()
}

/// Cherche une release plus récente. `None` si l'application est à jour, si
/// la vérification est désactivée (`KADENZ_SANS_MAJ`) ou en cas d'erreur.
/// Bloquant (quelques secondes au plus) : à appeler hors du fil de l'interface.
pub fn verifier() -> Option<MiseAJour> {
    if std::env::var_os(SANS_MAJ).is_some_and(|v| !v.is_empty()) {
        return None;
    }
    analyser(&derniere_release()?, VERSION)
}

/// Issue d'une recherche demandée par l'utilisateur (bouton « Rechercher une
/// mise à jour » des Paramètres).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recherche {
    Disponible(MiseAJour),
    /// Aucune release plus récente ne contient d'installateur.
    AJour,
    /// GitHub injoignable ou réponse illisible.
    HorsLigne,
}

/// Recherche à la demande : comme [`verifier`], mais dit pourquoi il n'y a
/// rien à installer. `KADENZ_SANS_MAJ` ne s'applique pas : l'utilisateur a
/// demandé la recherche. Bloquant : à appeler hors du fil de l'interface.
pub fn rechercher() -> Recherche {
    match derniere_release() {
        Some(json) => trier(&json, VERSION),
        None => Recherche::HorsLigne,
    }
}

/// Classe une réponse de l'API : illisible (sans tag de version) ou à jour.
pub fn trier(json: &str, actuelle: &str) -> Recherche {
    if let Some(maj) = analyser(json, actuelle) {
        return Recherche::Disponible(maj);
    }
    let lisible = serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|v| v["tag_name"].as_str().and_then(Version::lire))
        .is_some();
    if lisible {
        Recherche::AJour
    } else {
        Recherche::HorsLigne
    }
}

/// Variable des versions de développement : chemin d'un fichier JSON local
/// lu à la place de la réponse de GitHub, pour essayer l'invite.
pub const FICHIER_ESSAI: &str = "KADENZ_MAJ_JSON";

/// Réponse brute de l'API pour la dernière release ; en développement
/// seulement, le contenu du fichier désigné par `KADENZ_MAJ_JSON`.
fn derniere_release() -> Option<String> {
    #[cfg(debug_assertions)]
    if let Some(chemin) = std::env::var_os(FICHIER_ESSAI).filter(|v| !v.is_empty()) {
        return std::fs::read_to_string(chemin).ok();
    }
    derniere_release_github()
}

/// Réponse brute de l'API GitHub pour la dernière release.
fn derniere_release_github() -> Option<String> {
    agent(Some(Duration::from_secs(8)))
        .get(DERNIERE_RELEASE)
        .header("Accept", "application/vnd.github+json")
        .call()
        .ok()?
        .body_mut()
        .read_to_string()
        .ok()
}

/// Lit l'empreinte d'un fichier `.sha256` (« <hex>  <nom> » ou « <hex> »).
pub fn lire_empreinte(texte: &str) -> Option<[u8; 32]> {
    let hex = texte.split_whitespace().next()?;
    if hex.len() != 64 {
        return None;
    }
    let mut octets = [0u8; 32];
    for (i, o) in octets.iter_mut().enumerate() {
        *o = u8::from_str_radix(hex.get(2 * i..2 * i + 2)?, 16).ok()?;
    }
    Some(octets)
}

fn refus(modele: &'static str, valeur: impl std::fmt::Display) -> MessageCompose {
    MessageCompose::new(modele, &[&valeur])
}

fn refus_fixe(modele: &'static str) -> MessageCompose {
    MessageCompose::new(modele, &[])
}

/// Télécharge l'installateur dans le dossier temporaire, vérifie son empreinte
/// et renvoie son chemin. `progression(reçus, total)` est appelée au fil du
/// téléchargement (total inconnu si le serveur ne l'annonce pas). Bloquant : à
/// appeler hors du fil de l'interface.
pub fn telecharger(
    maj: &MiseAJour,
    progression: impl Fn(u64, Option<u64>),
) -> Result<PathBuf, MessageCompose> {
    let url_empreinte = maj
        .empreinte_url
        .as_deref()
        .ok_or_else(|| refus_fixe(MODELES_ERREUR[1]))?;
    let agent = agent(None);
    let texte = agent
        .get(url_empreinte)
        .call()
        .and_then(|mut r| r.body_mut().read_to_string())
        .map_err(|e| refus(MODELES_ERREUR[0], e))?;
    let attendue = lire_empreinte(&texte).ok_or_else(|| refus_fixe(MODELES_ERREUR[2]))?;

    let corps = agent
        .get(&maj.installateur_url)
        .call()
        .map_err(|e| refus(MODELES_ERREUR[0], e))?
        .into_body();
    let total = corps.content_length();
    let mut lecteur = corps.into_with_config().limit(TAILLE_MAX).reader();
    let mut octets = Vec::with_capacity(total.unwrap_or(0).min(TAILLE_MAX) as usize);
    let mut tampon = vec![0u8; 64 * 1024];
    progression(0, total);
    loop {
        let n = lecteur
            .read(&mut tampon)
            .map_err(|e| refus(MODELES_ERREUR[0], e))?;
        if n == 0 {
            break;
        }
        octets.extend_from_slice(&tampon[..n]);
        progression(octets.len() as u64, total);
    }
    if Sha256::digest(&octets).as_slice() != attendue {
        return Err(refus_fixe(MODELES_ERREUR[3]));
    }
    let dossier = std::env::temp_dir().join("Kadenz");
    std::fs::create_dir_all(&dossier).map_err(|e| refus(MODELES_ERREUR[4], e))?;
    let chemin = dossier.join(&maj.installateur_nom);
    std::fs::write(&chemin, &octets).map_err(|e| refus(MODELES_ERREUR[4], e))?;
    Ok(chemin)
}

/// Ouvre l'installateur comme un double-clic : Windows demande l'autorisation
/// d'administrateur si nécessaire. L'appelant ferme ensuite l'application.
#[cfg(windows)]
pub fn lancer(chemin: &Path) -> Result<(), MessageCompose> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let large = |s: &std::ffi::OsStr| s.encode_wide().chain([0]).collect::<Vec<u16>>();
    let verbe = large("open".as_ref());
    let fichier = large(chemin.as_os_str());
    // SAFETY : chaînes UTF-16 terminées par zéro, vivantes pendant l'appel ;
    // les pointeurs nuls sont admis pour les paramètres facultatifs.
    let code = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verbe.as_ptr(),
            fichier.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    } as isize;
    // ShellExecuteW renvoie une valeur supérieure à 32 en cas de succès.
    if code > 32 {
        Ok(())
    } else {
        Err(refus(MODELES_ERREUR[5], format!("code {code}")))
    }
}

#[cfg(not(windows))]
pub fn lancer(chemin: &Path) -> Result<(), MessageCompose> {
    std::process::Command::new(chemin)
        .spawn()
        .map(|_| ())
        .map_err(|e| refus(MODELES_ERREUR[5], e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, avec_empreinte: bool) -> String {
        let v = tag.trim_start_matches('v');
        let mut actifs = vec![
            serde_json::json!({"name": "kadenz.exe", "browser_download_url": "https://exemple/kadenz.exe"}),
            serde_json::json!({"name": format!("KadenzSetup-{v}.exe"), "browser_download_url": format!("https://exemple/KadenzSetup-{v}.exe")}),
        ];
        if avec_empreinte {
            actifs.push(serde_json::json!({"name": format!("KadenzSetup-{v}.exe.sha256"), "browser_download_url": "https://exemple/empreinte"}));
        }
        serde_json::json!({
            "tag_name": tag,
            "html_url": "https://github.com/rachk02/kadenz/releases/tag/x",
            "body": "  Notes  ",
            "assets": actifs,
        })
        .to_string()
    }

    #[test]
    fn versions_comparees_numeriquement() {
        assert_eq!(Version::lire("v0.10.0"), Some(Version(0, 10, 0)));
        assert!(Version::lire("0.10.0") > Version::lire("0.9.9"));
        assert_eq!(Version::lire("1.2"), None);
        assert_eq!(Version::lire("1.2.3-beta"), None);
        assert_eq!(Version::lire("1.2.3.4"), None);
    }

    #[test]
    fn release_plus_recente_seulement() {
        let m = analyser(&release("v0.2.0", true), "0.1.0").expect("plus récente");
        assert_eq!(m.version, "0.2.0");
        assert_eq!(m.installateur_nom, "KadenzSetup-0.2.0.exe");
        assert_eq!(m.installateur_url, "https://exemple/KadenzSetup-0.2.0.exe");
        assert_eq!(
            m.empreinte_url.as_deref(),
            Some("https://exemple/empreinte")
        );
        assert_eq!(m.notes, "Notes");
        assert_eq!(analyser(&release("v0.1.0", true), "0.1.0"), None);
        assert_eq!(analyser(&release("v0.0.9", true), "0.1.0"), None);
        assert_eq!(analyser("pas du json", "0.1.0"), None);
        let sans = analyser(&release("v0.2.0", false), "0.1.0").expect("sans empreinte");
        assert_eq!(sans.empreinte_url, None);
    }

    #[test]
    fn recherche_a_la_demande_triee() {
        assert!(matches!(
            trier(&release("v0.2.0", true), "0.1.0"),
            Recherche::Disponible(m) if m.version == "0.2.0"
        ));
        assert_eq!(trier(&release("v0.1.0", true), "0.1.0"), Recherche::AJour);
        assert_eq!(trier("<html>erreur</html>", "0.1.0"), Recherche::HorsLigne);
        assert_eq!(trier("{}", "0.1.0"), Recherche::HorsLigne);
    }

    #[test]
    fn empreinte_lue() {
        let hex = "ab".repeat(32);
        assert_eq!(
            lire_empreinte(&format!("{hex}  KadenzSetup-0.2.0.exe\n")),
            Some([0xab; 32])
        );
        assert_eq!(lire_empreinte(&hex.to_uppercase()), Some([0xab; 32]));
        assert_eq!(lire_empreinte("abc"), None);
        assert_eq!(lire_empreinte(&"zz".repeat(32)), None);
    }

    #[test]
    fn sans_empreinte_refuse_avant_tout_telechargement() {
        let m = analyser(&release("v0.2.0", false), "0.1.0").unwrap();
        assert_eq!(
            telecharger(&m, |_, _| {}),
            Err(refus_fixe(MODELES_ERREUR[1]))
        );
    }

    /// Interroge réellement GitHub (réseau requis) : `cargo test -p kadenz-maj -- --ignored`.
    #[test]
    #[ignore]
    fn derniere_release_reelle() {
        let json = derniere_release_github().expect("réponse de GitHub");
        let v: serde_json::Value = serde_json::from_str(&json).expect("JSON");
        let tag = v["tag_name"].as_str().expect("tag");
        assert!(Version::lire(tag).is_some(), "tag illisible : {tag}");
    }
}
