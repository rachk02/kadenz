//! Portail d'essai (jalon F9c) : le portail, avec la clé d'essai du banc
//! (`kadenz-serveur`, exemple `banc_essai`) au lieu de la clé du portail ; les
//! licences émises sont acceptées par un banc d'essai. Exemple de
//! développement : il n'est jamais livré.
//!
//! Usage : `portail_essai <dossier> [<adresse:port> [<mandataire>]]`. Au
//! premier lancement, crée le compte éditeur `editeur` (mot de passe
//! `phrase-de-passe-essai-12`). Écoute par défaut sur `127.0.0.1:47382`.
//! L'éditeur d'essai a la double authentification active (obligatoire pour
//! un éditeur) : à sa création, la clé et les codes de secours sont
//! affichés et écrits dans `<dossier>/essai-double-authentification.txt`.
//! `<mandataire>` : adresse IP du mandataire de confiance, comme
//! `--mandataire` du vrai portail (essai derrière Caddy ou Funnel). Arrêt :
//! fin du processus.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;

use ed25519_dalek::SigningKey;
use kadenz_portail::Portail;
use kadenz_portail::web::{self, ConfigWeb};

const MOT_DE_PASSE: &str = "phrase-de-passe-essai-12";

/// Clé d'essai du banc, la même que dans `banc_essai`.
fn cle_essai() -> SigningKey {
    SigningKey::from_bytes(&[5u8; 32])
}

/// Double authentification de l'éditeur d'essai : clé tirée au hasard,
/// activée avec le code du moment, affichée et notée dans le dossier.
fn activer_double(
    portail: &Portail,
    editeur: &kadenz_portail::Compte,
    dossier: &std::path::Path,
) -> Result<(), String> {
    let secret = portail
        .preparer_double_authentification(editeur)
        .map_err(|e| e.to_string())?;
    let unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let code = kadenz_portail::totp::code_pour(&secret, unix).ok_or("clé illisible")?;
    let codes = portail
        .activer_double_authentification(editeur, &code)
        .map_err(|e| e.to_string())?;
    let texte = format!(
        "Double authentification de l'éditeur d'essai (portail d'essai seulement)\n\
         Clé à saisir dans l'application : {}\n\
         Codes de secours :\n{}\n",
        kadenz_portail::totp::groupes(&secret),
        codes.join("\n")
    );
    print!("{texte}");
    let fichier = dossier.join("essai-double-authentification.txt");
    std::fs::write(&fichier, &texte).map_err(|e| e.to_string())?;
    println!("(noté dans {})", fichier.display());
    Ok(())
}

fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1);
    let Some(dossier) = arguments.next().map(PathBuf::from) else {
        eprintln!("usage : portail_essai <dossier> [<adresse:port> [<mandataire>]]");
        return ExitCode::FAILURE;
    };
    let ecoute: SocketAddr = match arguments.next() {
        None => SocketAddr::from(([127, 0, 0, 1], 47382)),
        Some(a) => match a.parse() {
            Ok(a) => a,
            Err(_) => {
                eprintln!("adresse invalide : {a}");
                return ExitCode::FAILURE;
            }
        },
    };
    let mut config = ConfigWeb::default();
    if let Some(m) = arguments.next() {
        match m.parse() {
            Ok(m) => config.mandataire = Some(m),
            Err(_) => {
                eprintln!("adresse de mandataire invalide : {m}");
                return ExitCode::FAILURE;
            }
        }
    }
    let portail = match Portail::ouvrir(&dossier, cle_essai()) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    match portail.creer_editeur("editeur", "Éditeur d'essai", MOT_DE_PASSE) {
        Ok(editeur) => {
            println!("Compte éditeur créé : editeur ; mot de passe : {MOT_DE_PASSE}");
            if let Err(e) = activer_double(&portail, &editeur, &dossier) {
                eprintln!("double authentification de l'éditeur d'essai : {e}");
                return ExitCode::FAILURE;
            }
        }
        Err(kadenz_portail::ErreurPortail::Validation(m)) if m == "identifiant déjà pris" => {
            println!(
                "Compte éditeur : editeur (déjà créé ; clé et codes de secours dans {})",
                dossier.join("essai-double-authentification.txt").display()
            )
        }
        Err(e) => {
            eprintln!("compte éditeur non créé : {e}");
            return ExitCode::FAILURE;
        }
    }
    let lance = match web::lancer(portail, ecoute, config) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("écoute sur {ecoute} impossible : {e}");
            return ExitCode::FAILURE;
        }
    };
    println!("Portail d'essai Kadenz : http://{}", lance.adresse());
    lance.attendre();
    ExitCode::SUCCESS
}
