//! Programme du relais (jalon F8), pour Linux dans un conteneur.
//!
//! Usage : `kadenz-relais [--dossier <dossier>] [--ecoute <adresse:port>]
//! [--sans-traces]`. Par défaut, dossier `./donnees-relais`, écoute sur
//! `0.0.0.0:47381`, traces dans `<dossier>/traces`. L'empreinte affichée au
//! lancement se règle dans la fenêtre du serveur de chaque entreprise.
//!
//! Santé (sonde du conteneur, lot R1) : `kadenz-relais sante [--dossier
//! <dossier>] [--adresse <hôte:port>]` se connecte au relais (par défaut
//! `127.0.0.1:47381`) avec l'empreinte du certificat du dossier, sans rien
//! créer ; code de sortie 0 si le relais répond, 1 sinon.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use kadenz_relais::PORT_PAR_DEFAUT;
use kadenz_relais::relais::{Config, Relais};

fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1).peekable();
    if arguments.peek().map(String::as_str) == Some("sante") {
        arguments.next();
        return sante(arguments);
    }
    let mut dossier = PathBuf::from("donnees-relais");
    let mut ecoute = SocketAddr::from(([0, 0, 0, 0], PORT_PAR_DEFAUT));
    let mut traces = true;
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

fn sante(mut arguments: impl Iterator<Item = String>) -> ExitCode {
    let mut dossier = PathBuf::from("donnees-relais");
    let mut adresse = format!("127.0.0.1:{PORT_PAR_DEFAUT}");
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--dossier" => match arguments.next() {
                Some(d) => dossier = d.into(),
                None => return usage(),
            },
            "--adresse" => match arguments.next() {
                Some(a) => adresse = a,
                None => return usage(),
            },
            _ => return usage(),
        }
    }
    let empreinte = match kadenz_relais::empreinte_enregistree(&dossier) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("santé : certificat du relais illisible : {e}");
            return ExitCode::FAILURE;
        }
    };
    match kadenz_relais::controle::verifier_sante(&adresse, &empreinte, Duration::from_secs(5)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("santé : {e}");
            ExitCode::FAILURE
        }
    }
}

fn usage() -> ExitCode {
    eprintln!(
        "usage : kadenz-relais [--dossier <dossier>] [--ecoute <adresse:port>] [--sans-traces]\n\
         \x20       kadenz-relais sante [--dossier <dossier>] [--adresse <hôte:port>]"
    );
    ExitCode::FAILURE
}
