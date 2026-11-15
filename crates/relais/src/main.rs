//! Programme du relais (jalon F8), pour Linux dans un conteneur.
//!
//! Usage : `kadenz-relais [--dossier <dossier>] [--ecoute <adresse:port>]
//! [--sans-traces]`. Par défaut, dossier `./donnees-relais`, écoute sur
//! `0.0.0.0:47381`, traces dans `<dossier>/traces`. L'empreinte affichée au
//! lancement se règle dans la fenêtre du serveur de chaque entreprise.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;

use kadenz_relais::PORT_PAR_DEFAUT;
use kadenz_relais::relais::{Config, Relais};

fn main() -> ExitCode {
    let mut dossier = PathBuf::from("donnees-relais");
    let mut ecoute = SocketAddr::from(([0, 0, 0, 0], PORT_PAR_DEFAUT));
    let mut traces = true;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--dossier" => match arguments.next() {
                Some(d) => dossier = d.into(),
                None => return usage(),
            },
            "--ecoute" => match arguments.next().and_then(|a| a.parse().ok()) {
                Some(a) => ecoute = a,
                None => return usage(),
            },
            "--sans-traces" => traces = false,
            _ => return usage(),
        }
    }
    let cles = kadenz_licence::entreprise::cles_portail();
    if cles.is_empty() {
        eprintln!("aucune clé du portail : les licences ne peuvent pas être vérifiées");
        return ExitCode::FAILURE;
    }
    let mut config = Config::nouvelle(ecoute, dossier, cles);
    if !traces {
        config.traces = None;
    }
    let relais = match Relais::demarrer(config) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("démarrage du relais impossible : {e}");
            return ExitCode::FAILURE;
        }
    };
    println!("Relais Kadenz à l'écoute sur {}", relais.adresse());
    println!("Empreinte du relais : {}", relais.empreinte());
    loop {
        std::thread::park();
    }
}

fn usage() -> ExitCode {
    eprintln!(
        "usage : kadenz-relais [--dossier <dossier>] [--ecoute <adresse:port>] [--sans-traces]"
    );
    ExitCode::FAILURE
}
