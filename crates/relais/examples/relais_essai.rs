//! Relais d'essai (jalon F8c) : le relais, qui accepte les licences signées
//! par la clé d'essai du banc (`kadenz-serveur`, exemple `banc_essai`) au lieu
//! de la clé du portail. Exemple de développement : il n'est jamais livré.
//!
//! Usage : `relais_essai [--dossier <dossier>] [--ecoute <adresse:port>]`.
//! Par défaut, dossier `./donnees-relais-essai`, écoute sur `0.0.0.0:47381`,
//! traces dans `<dossier>/traces`. Arrêt : fin du processus.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;

use ed25519_dalek::SigningKey;
use kadenz_relais::PORT_PAR_DEFAUT;
use kadenz_relais::relais::{Config, Relais};

/// Clé d'essai du banc, la même que dans `banc_essai`.
fn cle_essai() -> SigningKey {
    SigningKey::from_bytes(&[5u8; 32])
}

fn main() -> ExitCode {
    let mut dossier = PathBuf::from("donnees-relais-essai");
    let mut ecoute = SocketAddr::from(([0, 0, 0, 0], PORT_PAR_DEFAUT));
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match (argument.as_str(), arguments.next()) {
            ("--dossier", Some(d)) => dossier = d.into(),
            ("--ecoute", Some(a)) => match a.parse() {
                Ok(a) => ecoute = a,
                Err(_) => return usage(),
            },
            _ => return usage(),
        }
    }
    let config = Config::nouvelle(ecoute, dossier, vec![cle_essai().verifying_key()]);
    let relais = match Relais::demarrer(config) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("démarrage du relais d'essai impossible : {e}");
            return ExitCode::FAILURE;
        }
    };
    println!("Relais d'essai Kadenz à l'écoute sur {}", relais.adresse());
    println!("Empreinte du relais : {}", relais.empreinte());
    loop {
        std::thread::park();
    }
}

fn usage() -> ExitCode {
    eprintln!("usage : relais_essai [--dossier <dossier>] [--ecoute <adresse:port>]");
    ExitCode::FAILURE
}
