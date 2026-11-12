//! Mises à jour de Kadenz (cahier des charges, section 16 ; cahier de la
//! phase 2, pont V1, règles a et b).
//!
//! L'application parcourt la liste des releases publiées sur GitHub
//! ([`verifier`], [`rechercher`]). Chaque release porte un manifeste
//! `kadenz-maj.json` (version, génération, version minimale d'où la mise à
//! jour est permise, nom et empreinte SHA-256 de l'installateur), signé en
//! Ed25519 par la clé de mise à jour de l'éditeur (`kadenz-maj.json.sig`). Seule
//! la release la plus récente dont le manifeste est valablement signé, de la
//! génération de cette application ([`GENERATION`]) et permise depuis la
//! version installée est proposée. [`telecharger`] récupère alors l'installateur
//! et vérifie son empreinte, celle du manifeste signé, puis [`lancer`] l'ouvre ;
//! l'application se ferme ensuite.
//!
//! Les versions d'une autre génération (phase 2) sont publiées dans le même
//! dépôt sans le drapeau Latest, que les postes antérieurs au pont suivent
//! encore : elles ne sont jamais proposées ici, seulement signalées.
//!
//! Aucune donnée du poste, de la licence ou du client n'est envoyée : la seule
//! en-tête propre à Kadenz est `User-Agent: Kadenz/<version>`. Toute erreur de
//! la vérification automatique (hors connexion, délai dépassé, réponse
//! illisible) est silencieuse.

use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use ed25519_dalek::{Signature, Signer, Verifier};
pub use ed25519_dalek::{SigningKey, VerifyingKey};
use kadenz_domaine::message::MessageCompose;
use sha2::{Digest, Sha256};
use ureq::Agent;
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

/// Releases publiées, les plus récentes d'abord (par date de création : le tri
/// par version est refait ici).
const RELEASES: &str = "https://api.github.com/repos/rachk02/kadenz/releases?per_page=30";

/// Version de cette application.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Génération de cette application : seules les releases de la même
/// génération lui sont proposées.
pub const GENERATION: &str = "v1";

/// Nom du manifeste joint à chaque release, et de sa signature.
pub const MANIFESTE: &str = "kadenz-maj.json";
pub const SIGNATURE: &str = "kadenz-maj.json.sig";

/// Clé publique de mise à jour, en hexadécimal (64 caractères), produite par
/// `kadenz-editeur generer`. Vide tant qu'elle n'est pas générée : aucune
/// release n'est alors possible (contrôle de `kadenz-editeur manifeste`).
const CLE_MAJ: &str = include_str!("../cle-maj.hex");

/// Variable d'environnement qui désactive la vérification (développement).
pub const SANS_MAJ: &str = "KADENZ_SANS_MAJ";

/// Taille maximale acceptée pour l'installateur.
const TAILLE_MAX: u64 = 300 * 1024 * 1024;
/// Taille maximale acceptée pour le manifeste et sa signature.
const TAILLE_MAX_MANIFESTE: u64 = 64 * 1024;

const ERREUR_TELECHARGEMENT: &str = "téléchargement impossible : {}";
const ERREUR_EMPREINTE: &str =
    "l'installateur téléchargé ne correspond pas à son empreinte : fichier supprimé";
const ERREUR_ECRITURE: &str = "écriture de l'installateur impossible : {}";
const ERREUR_LANCEMENT: &str = "lancement de l'installateur impossible : {}";

/// Modèles des messages d'erreur, à traduire.
pub const MODELES_ERREUR: &[&str] = &[
    ERREUR_TELECHARGEMENT,
    ERREUR_EMPREINTE,
    ERREUR_ECRITURE,
    ERREUR_LANCEMENT,
];

/// Release plus récente que l'application, à installer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MiseAJour {
    /// Version sans le « v » (ex. 0.3.0).
    pub version: String,
    /// Page de la release sur GitHub.
    pub page: String,
    /// Notes de version (section du CHANGELOG), en Markdown.
    pub notes: String,
    pub installateur_nom: String,
    pub installateur_url: String,
    /// Empreinte SHA-256 de l'installateur, tirée du manifeste signé.
    pub empreinte: [u8; 32],
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

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

/// Manifeste de mise à jour d'une release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifeste {
    pub version: Version,
    pub generation: String,
    /// Version minimale depuis laquelle la mise à jour est permise.
    pub depuis: Version,
    /// Nom du fichier de l'installateur dans la release.
    pub installateur: String,
    pub empreinte: [u8; 32],
}

impl Manifeste {
    /// Texte JSON du manifeste, tel qu'il est signé et publié.
    pub fn json(&self) -> String {
        serde_json::json!({
            "version": self.version.to_string(),
            "generation": self.generation,
            "depuis": self.depuis.to_string(),
            "installateur": self.installateur,
            "sha256": en_hex(&self.empreinte),
        })
        .to_string()
    }

    /// Lit un manifeste ; `None` s'il manque un champ ou qu'un champ est illisible.
    pub fn lire(octets: &[u8]) -> Option<Manifeste> {
        let v: serde_json::Value = serde_json::from_slice(octets).ok()?;
        let texte = |cle: &str| v[cle].as_str();
        Some(Manifeste {
            version: Version::lire(texte("version")?)?,
            generation: texte("generation")?.to_owned(),
            depuis: Version::lire(texte("depuis")?)?,
            installateur: texte("installateur")?.to_owned(),
            empreinte: lire_empreinte(texte("sha256")?)?,
        })
    }
}

fn en_hex(octets: &[u8]) -> String {
    octets.iter().map(|o| format!("{o:02x}")).collect()
}

fn depuis_hex<const N: usize>(texte: &str) -> Option<[u8; N]> {
    let texte = texte.trim();
    if texte.len() != 2 * N {
        return None;
    }
    let mut octets = [0u8; N];
    for (k, o) in octets.iter_mut().enumerate() {
        *o = u8::from_str_radix(texte.get(2 * k..2 * k + 2)?, 16).ok()?;
    }
    Some(octets)
}

/// Clé publique de mise à jour embarquée ; `None` si elle n'est pas encore générée.
pub fn cle_maj() -> Option<VerifyingKey> {
    cle_publique_depuis_hex(CLE_MAJ)
}

pub fn cle_publique_depuis_hex(texte: &str) -> Option<VerifyingKey> {
    VerifyingKey::from_bytes(&depuis_hex::<32>(texte)?).ok()
}

pub fn cle_privee_depuis_hex(texte: &str) -> Option<SigningKey> {
    Some(SigningKey::from_bytes(&depuis_hex::<32>(texte)?))
}

/// Signature d'un manifeste (outil de l'éditeur), en hexadécimal.
pub fn signer_manifeste(json: &[u8], cle: &SigningKey) -> String {
    en_hex(&cle.sign(json).to_bytes())
}

/// Manifeste lu si `signature` (hexadécimal) le signe valablement avec `cle`.
pub fn verifier_manifeste(json: &[u8], signature: &str, cle: &VerifyingKey) -> Option<Manifeste> {
    let signature = Signature::from_bytes(&depuis_hex::<64>(signature)?);
    cle.verify(json, &signature).ok()?;
    Manifeste::lire(json)
}

/// Issue d'une recherche de mise à jour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recherche {
    Disponible(MiseAJour),
    /// Aucune release plus récente à proposer.
    AJour,
    /// Une release plus récente existe, mais son manifeste n'est pas signé
    /// valablement (ou ne correspond pas à sa release) : elle n'est pas proposée.
    Refusee,
    /// La release la plus récente est d'une autre génération (version donnée) :
    /// elle s'installe séparément.
    AutreGeneration(String),
    /// GitHub injoignable ou réponse illisible.
    HorsLigne,
}

/// Choisit la release à proposer dans la liste renvoyée par l'API GitHub.
/// `lire(url)` rend le contenu d'un fichier joint (manifeste, signature), ou
/// `None` s'il est injoignable ; `cle` est la clé publique de mise à jour.
pub fn choisir(
    liste: &str,
    actuelle: &str,
    cle: Option<&VerifyingKey>,
    lire: impl Fn(&str) -> Option<Vec<u8>>,
) -> Recherche {
    let Ok(serde_json::Value::Array(releases)) = serde_json::from_str(liste) else {
        return Recherche::HorsLigne;
    };
    let Some(actuelle) = Version::lire(actuelle) else {
        return Recherche::HorsLigne;
    };
    let mut candidates: Vec<(Version, &serde_json::Value)> = releases
        .iter()
        .filter(|r| !r["draft"].as_bool().unwrap_or(false))
        .filter(|r| !r["prerelease"].as_bool().unwrap_or(false))
        .filter_map(|r| Some((Version::lire(r["tag_name"].as_str()?)?, r)))
        .filter(|(v, _)| *v > actuelle)
        .collect();
    candidates.sort_by_key(|(v, _)| std::cmp::Reverse(*v));

    let mut refusee = false;
    let mut autre_generation: Option<Version> = None;
    for (version, release) in candidates {
        let actif = |nom: &str| {
            release["assets"].as_array()?.iter().find_map(|a| {
                (a["name"].as_str() == Some(nom))
                    .then(|| a["browser_download_url"].as_str().map(str::to_owned))
                    .flatten()
            })
        };
        // Releases d'avant le pont : sans manifeste, jamais proposées.
        let (Some(url_manifeste), Some(url_signature)) = (actif(MANIFESTE), actif(SIGNATURE))
        else {
            continue;
        };
        let (Some(json), Some(signature)) = (lire(&url_manifeste), lire(&url_signature)) else {
            continue;
        };
        let manifeste = cle
            .zip(std::str::from_utf8(&signature).ok())
            .and_then(|(cle, signature)| verifier_manifeste(&json, signature, cle))
            .filter(|m| m.version == version);
        let Some(manifeste) = manifeste else {
            refusee = true;
            continue;
        };
        if manifeste.generation != GENERATION {
            autre_generation = autre_generation.max(Some(version));
            continue;
        }
        if manifeste.depuis > actuelle {
            continue;
        }
        let Some(installateur_url) = actif(&manifeste.installateur) else {
            continue;
        };
        return Recherche::Disponible(MiseAJour {
            version: version.to_string(),
            page: release["html_url"].as_str().unwrap_or_default().to_owned(),
            notes: release["body"]
                .as_str()
                .unwrap_or_default()
                .trim()
                .to_owned(),
            installateur_nom: manifeste.installateur,
            installateur_url,
            empreinte: manifeste.empreinte,
        });
    }
    if refusee {
        Recherche::Refusee
    } else if let Some(version) = autre_generation {
        Recherche::AutreGeneration(version.to_string())
    } else {
        Recherche::AJour
    }
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

/// Cherche une release à installer. `None` si l'application est à jour, si la
/// vérification est désactivée (`KADENZ_SANS_MAJ`) ou en cas d'erreur.
/// Bloquant (quelques secondes au plus) : à appeler hors du fil de l'interface.
pub fn verifier() -> Option<MiseAJour> {
    if std::env::var_os(SANS_MAJ).is_some_and(|v| !v.is_empty()) {
        return None;
    }
    match choisir(&releases()?, VERSION, cle_maj().as_ref(), lire_actif) {
        Recherche::Disponible(maj) => Some(maj),
        _ => None,
    }
}

/// Recherche à la demande (bouton « Rechercher une mise à jour » des
/// Paramètres) : comme [`verifier`], mais dit pourquoi il n'y a rien à
/// installer. `KADENZ_SANS_MAJ` ne s'applique pas : l'utilisateur a demandé la
/// recherche. Bloquant : à appeler hors du fil de l'interface.
pub fn rechercher() -> Recherche {
    match releases() {
        Some(liste) => choisir(&liste, VERSION, cle_maj().as_ref(), lire_actif),
        None => Recherche::HorsLigne,
    }
}

/// Variable des versions de développement : chemin d'un fichier JSON local
/// (liste de releases) lu à la place de la réponse de GitHub, pour essayer
/// l'invite ; ses fichiers joints peuvent alors être des chemins locaux.
pub const FICHIER_ESSAI: &str = "KADENZ_MAJ_JSON";

/// Réponse brute de l'API pour la liste des releases ; en développement
/// seulement, le contenu du fichier désigné par `KADENZ_MAJ_JSON`.
fn releases() -> Option<String> {
    #[cfg(debug_assertions)]
    if let Some(chemin) = std::env::var_os(FICHIER_ESSAI).filter(|v| !v.is_empty()) {
        return std::fs::read_to_string(chemin).ok();
    }
    releases_github()
}

/// Réponse brute de l'API GitHub pour la liste des releases.
fn releases_github() -> Option<String> {
    agent(Some(Duration::from_secs(8)))
        .get(RELEASES)
        .header("Accept", "application/vnd.github+json")
        .call()
        .ok()?
        .body_mut()
        .read_to_string()
        .ok()
}

/// Contenu d'un fichier joint (manifeste, signature), au plus 64 Ko.
fn lire_actif(url: &str) -> Option<Vec<u8>> {
    #[cfg(debug_assertions)]
    if !url.starts_with("https://") {
        return std::fs::read(url).ok();
    }
    let mut octets = Vec::new();
    agent(Some(Duration::from_secs(8)))
        .get(url)
        .call()
        .ok()?
        .into_body()
        .into_with_config()
        .limit(TAILLE_MAX_MANIFESTE)
        .reader()
        .read_to_end(&mut octets)
        .ok()?;
    Some(octets)
}

/// Lit une empreinte SHA-256 (« <hex>  <nom> » ou « <hex> »).
pub fn lire_empreinte(texte: &str) -> Option<[u8; 32]> {
    depuis_hex::<32>(texte.split_whitespace().next()?)
}

fn refus(modele: &'static str, valeur: impl std::fmt::Display) -> MessageCompose {
    MessageCompose::new(modele, &[&valeur])
}

fn refus_fixe(modele: &'static str) -> MessageCompose {
    MessageCompose::new(modele, &[])
}

/// Télécharge l'installateur dans le dossier temporaire, vérifie son empreinte
/// (celle du manifeste signé) et renvoie son chemin. `progression(reçus,
/// total)` est appelée au fil du téléchargement (total inconnu si le serveur ne
/// l'annonce pas). Bloquant : à appeler hors du fil de l'interface.
pub fn telecharger(
    maj: &MiseAJour,
    progression: impl Fn(u64, Option<u64>),
) -> Result<PathBuf, MessageCompose> {
    let corps = agent(None)
        .get(&maj.installateur_url)
        .call()
        .map_err(|e| refus(ERREUR_TELECHARGEMENT, e))?
        .into_body();
    let total = corps.content_length();
    let mut lecteur = corps.into_with_config().limit(TAILLE_MAX).reader();
    let mut octets = Vec::with_capacity(total.unwrap_or(0).min(TAILLE_MAX) as usize);
    let mut tampon = vec![0u8; 64 * 1024];
    progression(0, total);
    loop {
        let n = lecteur
            .read(&mut tampon)
            .map_err(|e| refus(ERREUR_TELECHARGEMENT, e))?;
        if n == 0 {
            break;
        }
        octets.extend_from_slice(&tampon[..n]);
        progression(octets.len() as u64, total);
    }
    if Sha256::digest(&octets).as_slice() != maj.empreinte {
        return Err(refus_fixe(ERREUR_EMPREINTE));
    }
    let dossier = std::env::temp_dir().join("Kadenz");
    std::fs::create_dir_all(&dossier).map_err(|e| refus(ERREUR_ECRITURE, e))?;
    let chemin = dossier.join(&maj.installateur_nom);
    std::fs::write(&chemin, &octets).map_err(|e| refus(ERREUR_ECRITURE, e))?;
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
        Err(refus(ERREUR_LANCEMENT, format!("code {code}")))
    }
}

#[cfg(not(windows))]
pub fn lancer(chemin: &Path) -> Result<(), MessageCompose> {
    std::process::Command::new(chemin)
        .spawn()
        .map(|_| ())
        .map_err(|e| refus(ERREUR_LANCEMENT, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn cle_essai() -> SigningKey {
        SigningKey::from_bytes(&[3u8; 32])
    }

    /// Release de la liste de l'API, avec son manifeste signé par la clé donnée
    /// (ou sans manifeste) ; les fichiers joints sont rangés dans `fichiers`
    /// sous leur adresse.
    fn release(
        tag: &str,
        manifeste: Option<(&Manifeste, &SigningKey)>,
        fichiers: &mut HashMap<String, Vec<u8>>,
    ) -> serde_json::Value {
        let v = tag.trim_start_matches('v');
        let url = |nom: &str| format!("https://exemple/{v}/{nom}");
        let installateur = format!("KadenzSetup-{v}.exe");
        let mut actifs = vec![
            serde_json::json!({"name": installateur, "browser_download_url": url(&installateur)}),
            serde_json::json!({"name": format!("{installateur}.sha256"), "browser_download_url": url("sha")}),
        ];
        if let Some((m, cle)) = manifeste {
            let json = m.json();
            fichiers.insert(
                url(SIGNATURE),
                signer_manifeste(json.as_bytes(), cle).into_bytes(),
            );
            fichiers.insert(url(MANIFESTE), json.into_bytes());
            actifs.push(
                serde_json::json!({"name": MANIFESTE, "browser_download_url": url(MANIFESTE)}),
            );
            actifs.push(
                serde_json::json!({"name": SIGNATURE, "browser_download_url": url(SIGNATURE)}),
            );
        }
        serde_json::json!({
            "tag_name": tag,
            "draft": false,
            "prerelease": false,
            "html_url": format!("https://github.com/rachk02/kadenz/releases/tag/{tag}"),
            "body": "  Notes  ",
            "assets": actifs,
        })
    }

    fn manifeste(version: &str, generation: &str, depuis: &str) -> Manifeste {
        Manifeste {
            version: Version::lire(version).unwrap(),
            generation: generation.into(),
            depuis: Version::lire(depuis).unwrap(),
            installateur: format!("KadenzSetup-{version}.exe"),
            empreinte: [0xab; 32],
        }
    }

    fn choisir_dans(
        releases: &[serde_json::Value],
        fichiers: &HashMap<String, Vec<u8>>,
        actuelle: &str,
    ) -> Recherche {
        let cle = cle_essai().verifying_key();
        let liste = serde_json::Value::Array(releases.to_vec()).to_string();
        choisir(&liste, actuelle, Some(&cle), |url| {
            fichiers.get(url).cloned()
        })
    }

    #[test]
    fn versions_comparees_numeriquement() {
        assert_eq!(Version::lire("v0.10.0"), Some(Version(0, 10, 0)));
        assert!(Version::lire("0.10.0") > Version::lire("0.9.9"));
        assert_eq!(Version::lire("1.2"), None);
        assert_eq!(Version::lire("1.2.3-beta"), None);
        assert_eq!(Version::lire("1.2.3.4"), None);
        assert_eq!(Version(0, 3, 1).to_string(), "0.3.1");
    }

    #[test]
    fn manifeste_signe_aller_retour() {
        let m = manifeste("0.3.0", "v1", "0.0.0");
        let json = m.json();
        let signature = signer_manifeste(json.as_bytes(), &cle_essai());
        let cle = cle_essai().verifying_key();
        assert_eq!(
            verifier_manifeste(json.as_bytes(), &signature, &cle),
            Some(m)
        );
        // Une donnée changée, ou une autre clé : refusé.
        let modifie = json.replace("0.3.0", "0.3.1");
        assert_eq!(
            verifier_manifeste(modifie.as_bytes(), &signature, &cle),
            None
        );
        let autre = SigningKey::from_bytes(&[4u8; 32]).verifying_key();
        assert_eq!(
            verifier_manifeste(json.as_bytes(), &signature, &autre),
            None
        );
        assert_eq!(verifier_manifeste(json.as_bytes(), "zz", &cle), None);
    }

    #[test]
    fn plus_recente_valide_proposee() {
        let mut f = HashMap::new();
        let cle = cle_essai();
        // Ordre de l'API (date) différent de l'ordre des versions.
        let releases = [
            release(
                "v0.3.0",
                Some((&manifeste("0.3.0", "v1", "0.0.0"), &cle)),
                &mut f,
            ),
            release(
                "v0.4.0",
                Some((&manifeste("0.4.0", "v1", "0.0.0"), &cle)),
                &mut f,
            ),
            release("v0.2.1", None, &mut f),
        ];
        let Recherche::Disponible(m) = choisir_dans(&releases, &f, "0.2.1") else {
            panic!("mise à jour attendue");
        };
        assert_eq!(m.version, "0.4.0");
        assert_eq!(m.installateur_nom, "KadenzSetup-0.4.0.exe");
        assert_eq!(
            m.installateur_url,
            "https://exemple/0.4.0/KadenzSetup-0.4.0.exe"
        );
        assert_eq!(m.empreinte, [0xab; 32]);
        assert_eq!(m.notes, "Notes");
        assert_eq!(choisir_dans(&releases, &f, "0.4.0"), Recherche::AJour);
    }

    #[test]
    fn releases_ecartees() {
        let cle = cle_essai();
        let pirate = SigningKey::from_bytes(&[4u8; 32]);
        let v1 = |v: &str, depuis: &str| manifeste(v, "v1", depuis);

        // Sans manifeste (avant le pont) : jamais proposée.
        let mut f = HashMap::new();
        let r = [release("v0.3.0", None, &mut f)];
        assert_eq!(choisir_dans(&r, &f, "0.2.1"), Recherche::AJour);

        // Signée par une autre clé : refusée, la précédente valide est proposée.
        let mut f = HashMap::new();
        let r = [
            release("v0.4.0", Some((&v1("0.4.0", "0.0.0"), &pirate)), &mut f),
            release("v0.3.0", Some((&v1("0.3.0", "0.0.0"), &cle)), &mut f),
        ];
        assert!(
            matches!(choisir_dans(&r, &f, "0.2.1"), Recherche::Disponible(m) if m.version == "0.3.0")
        );
        assert_eq!(choisir_dans(&r[..1], &f, "0.2.1"), Recherche::Refusee);

        // Manifeste valide mais d'une autre version que le tag : refusée.
        let mut f = HashMap::new();
        let r = [release(
            "v0.5.0",
            Some((&v1("0.4.0", "0.0.0"), &cle)),
            &mut f,
        )];
        assert_eq!(choisir_dans(&r, &f, "0.2.1"), Recherche::Refusee);

        // Autre génération : signalée, jamais proposée.
        let mut f = HashMap::new();
        let r = [
            release(
                "v2.0.0",
                Some((&manifeste("2.0.0", "v2", "0.0.0"), &cle)),
                &mut f,
            ),
            release("v0.3.0", Some((&v1("0.3.0", "0.0.0"), &cle)), &mut f),
        ];
        assert!(
            matches!(choisir_dans(&r, &f, "0.2.1"), Recherche::Disponible(m) if m.version == "0.3.0")
        );
        assert_eq!(
            choisir_dans(&r, &f, "0.3.0"),
            Recherche::AutreGeneration("2.0.0".into())
        );

        // Version minimale trop haute : pas proposée.
        let mut f = HashMap::new();
        let r = [release(
            "v0.4.0",
            Some((&v1("0.4.0", "0.3.0"), &cle)),
            &mut f,
        )];
        assert_eq!(choisir_dans(&r, &f, "0.2.1"), Recherche::AJour);
        assert!(matches!(
            choisir_dans(&r, &f, "0.3.0"),
            Recherche::Disponible(_)
        ));

        // Préversion et brouillon : ignorés.
        let mut f = HashMap::new();
        let mut pre = release("v0.4.0", Some((&v1("0.4.0", "0.0.0"), &cle)), &mut f);
        pre["prerelease"] = true.into();
        let mut brouillon = release("v0.5.0", Some((&v1("0.5.0", "0.0.0"), &cle)), &mut f);
        brouillon["draft"] = true.into();
        assert_eq!(
            choisir_dans(&[pre, brouillon], &f, "0.2.1"),
            Recherche::AJour
        );
    }

    #[test]
    fn sans_cle_ou_reponse_illisible() {
        let mut f = HashMap::new();
        let r = [release(
            "v0.3.0",
            Some((&manifeste("0.3.0", "v1", "0.0.0"), &cle_essai())),
            &mut f,
        )];
        let liste = serde_json::Value::Array(r.to_vec()).to_string();
        // Clé de mise à jour pas encore embarquée : rien n'est proposé.
        assert_eq!(
            choisir(&liste, "0.2.1", None, |u| f.get(u).cloned()),
            Recherche::Refusee
        );
        // Fichier joint injoignable : release passée.
        let cle = cle_essai().verifying_key();
        assert_eq!(
            choisir(&liste, "0.2.1", Some(&cle), |_| None),
            Recherche::AJour
        );
        assert_eq!(choisir_dans(&[], &f, "0.2.1"), Recherche::AJour);
        assert_eq!(
            choisir("<html>erreur</html>", "0.2.1", Some(&cle), |_| None),
            Recherche::HorsLigne
        );
        assert_eq!(
            choisir("{}", "0.2.1", Some(&cle), |_| None),
            Recherche::HorsLigne
        );
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

    /// Interroge réellement GitHub (réseau requis) : `cargo test -p kadenz-maj -- --ignored`.
    #[test]
    #[ignore]
    fn liste_des_releases_reelle() {
        let json = releases_github().expect("réponse de GitHub");
        let v: serde_json::Value = serde_json::from_str(&json).expect("JSON");
        let tags: Vec<&str> = v
            .as_array()
            .expect("liste")
            .iter()
            .filter_map(|r| r["tag_name"].as_str())
            .collect();
        assert!(
            tags.iter().any(|t| Version::lire(t).is_some()),
            "aucun tag lisible : {tags:?}"
        );
    }
}
