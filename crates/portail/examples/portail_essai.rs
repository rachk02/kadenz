//! Portail d'essai (jalon F9c) : le portail, avec la clé d'essai du banc
//! (`kadenz-serveur`, exemple `banc_essai`) au lieu de la clé du portail ; les
//! licences émises sont acceptées par un banc d'essai. Exemple de
//! développement : il n'est jamais livré.
//!
//! Usage : `portail_essai <dossier> [<adresse:port>]`. Au premier lancement,
//! crée le compte éditeur `editeur` (mot de passe `motdepasse12`). Écoute par
//! défaut sur `127.0.0.1:47382`. Arrêt : fin du processus.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;

use ed25519_dalek::SigningKey;
use kadenz_portail::Portail;
use kadenz_portail::web::{self, ConfigWeb};

const MOT_DE_PASSE: &str = "motdepasse12";

/// Clé d'essai du banc, la même que dans `banc_essai`.
fn cle_essai() -> SigningKey {
    SigningKey::from_bytes(&[5u8; 32])
}

fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1);
    let Some(dossier) = arguments.next().map(PathBuf::from) else {
        eprintln!("usage : portail_essai <dossier> [<adresse:port>]");
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
    let portail = match Portail::ouvrir(&dossier, cle_essai()) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    match portail.creer_editeur("editeur", "Éditeur d'essai", MOT_DE_PASSE) {
        Ok(_) => println!("Compte éditeur créé : editeur ; mot de passe : {MOT_DE_PASSE}"),
        Err(_) => println!("Compte éditeur : editeur (déjà créé)"),
    }
    let lance = match web::lancer(portail, ecoute, ConfigWeb::default()) {
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
