//! Programme du relais (jalon F8), pour Linux dans un conteneur.
//!
//! Usage : `kadenz-relais [--dossier <dossier>] [--ecoute <adresse:port>]
//! [--ecoute-ws <adresse:port> [--mandataire <adresse>] [--secret-ws <fichier>]] [--sans-traces]`.
//! Par défaut, dossier `./donnees-relais`, écoute sur `0.0.0.0:47381`,
//! traces dans `<dossier>/traces`. L'empreinte affichée au lancement se règle
//! dans la fenêtre du serveur de chaque entreprise.
//!
//! `--ecoute-ws` ouvre en plus une écoute WebSocket en HTTP simple, pour
//! cloudflared (docs/spec-relais-websocket.md) ; `--mandataire` donne
//! l'adresse de cloudflared, seule à faire croire `CF-Connecting-IP`.
//! `--secret-ws` lit un secret dans un fichier de l'hôte : une règle de
//! transformation Cloudflare le pose sur l'en-tête `x-kadenz-secret`, et la
//! passerelle refuse toute WebSocket distante qui ne le porte pas
//! (`docs/spec-relais-websocket.md`).
//!
//! Santé (sonde du conteneur, lot R1) : `kadenz-relais sante [--dossier
//! <dossier>] [--adresse <hôte:port>] [--ws <hôte:port>]` se connecte au relais
//! (par défaut `127.0.0.1:47381`) avec l'empreinte du certificat du dossier,
//! puis, si `--ws` est donné, ouvre une WebSocket sur la passerelle. Sans rien
//! créer ; code de sortie 0 si les deux répondent, 1 sinon.

use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use kadenz_relais::PORT_PAR_DEFAUT;
use kadenz_relais::relais::{Config, Relais};

/// Lit le secret WebSocket depuis un fichier de l'hôte. Une fin de ligne est
/// tolérée (le secret est souvent créé par `openssl rand -hex 32 > fichier`).
fn lire_secret_ws(chemin: &Path) -> Result<Arc<[u8]>, String> {
    let brut = std::fs::read(chemin)
        .map_err(|e| format!("secret WebSocket illisible dans {chemin:?} : {e}"))?;
    let texte = std::str::from_utf8(&brut)
        .map_err(|_| "secret WebSocket : le fichier n'est pas en UTF-8".to_string())?;
    let secret = texte.trim();
    if secret.is_empty() || secret.len() > 256 || secret.chars().any(char::is_control) {
        return Err(
            "secret WebSocket : valeur vide, trop longue ou avec un caractère de contrôle".into(),
        );
    }
    Ok(Arc::from(secret.as_bytes()))
}

fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1).peekable();
    if arguments.peek().map(String::as_str) == Some("sante") {
        arguments.next();
        return sante(arguments);
    }
    let mut dossier = PathBuf::from("donnees-relais");
    let mut ecoute = SocketAddr::from(([0, 0, 0, 0], PORT_PAR_DEFAUT));
    let mut traces = true;
    let mut ecoute_ws: Option<SocketAddr> = None;
    let mut mandataire: Option<IpAddr> = None;
    let mut secret_ws: Option<Arc<[u8]>> = None;
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
            "--ecoute-ws" => match arguments.next().and_then(|a| a.parse().ok()) {
                Some(a) => ecoute_ws = Some(a),
                None => return usage(),
            },
            "--mandataire" => match arguments.next().and_then(|a| a.parse().ok()) {
                Some(a) => mandataire = Some(a),
                None => return usage(),
            },
            "--secret-ws" => match arguments.next() {
                Some(f) => match lire_secret_ws(Path::new(&f)) {
                    Ok(s) => secret_ws = Some(s),
                    Err(e) => {
                        eprintln!("{e}");
                        return ExitCode::FAILURE;
                    }
                },
                None => return usage(),
            },
            "--sans-traces" => traces = false,
            _ => return usage(),
        }
    }
    if mandataire.is_some() && ecoute_ws.is_none() {
        return usage();
    }
    if secret_ws.is_some() && ecoute_ws.is_none() {
        return usage();
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
    config.ecoute_ws = ecoute_ws;
    config.mandataire = mandataire;
    config.secret_ws = secret_ws;
    let relais = match Relais::demarrer(config) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("démarrage du relais impossible : {e}");
            return ExitCode::FAILURE;
        }
    };
    println!("Relais Kadenz à l'écoute sur {}", relais.adresse());
    if let Some(a) = relais.adresse_ws() {
        println!("WebSocket à l'écoute sur {a}");
    }
    println!("Empreinte du relais : {}", relais.empreinte());
    loop {
        std::thread::park();
    }
}

fn sante(mut arguments: impl Iterator<Item = String>) -> ExitCode {
    let mut dossier = PathBuf::from("donnees-relais");
    let mut adresse = format!("127.0.0.1:{PORT_PAR_DEFAUT}");
    let mut websocket: Option<String> = None;
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
            "--ws" => match arguments.next() {
                Some(a) => websocket = Some(a),
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
    if let Err(e) =
        kadenz_relais::controle::verifier_sante(&adresse, &empreinte, Duration::from_secs(5))
    {
        eprintln!("santé : {e}");
        return ExitCode::FAILURE;
    }
    if let Some(adresse_ws) = websocket {
        let cible: SocketAddr = match adresse_ws.parse() {
            Ok(a) => a,
            Err(e) => {
                eprintln!("santé : adresse WebSocket invalide : {e}");
                return ExitCode::FAILURE;
            }
        };
        if let Err(e) = kadenz_relais::ws::verifier_sante(cible, Duration::from_secs(5)) {
            eprintln!("santé : {e}");
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

fn usage() -> ExitCode {
    eprintln!(
        "usage : kadenz-relais [--dossier <dossier>] [--ecoute <adresse:port>] [--ecoute-ws <adresse:port> [--mandataire <adresse>] [--secret-ws <fichier>]] [--sans-traces]\n\
         \x20       kadenz-relais sante [--dossier <dossier>] [--adresse <hôte:port>] [--ws <hôte:port>]"
    );
    ExitCode::FAILURE
}
