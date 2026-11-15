//! Portail des licences de Kadenz (jalon F9).
//!
//! Usage :
//! - `kadenz-portail --dossier <dossier> --cle <fichier> [--ecoute <adresse:port>] [--mandataire]` :
//!   lance le portail. Le fichier de clé contient la clé privée du portail
//!   en hexadécimal (format de `kadenz-editeur`) ; sans lui, le portail ne
//!   démarre pas. Écoute par défaut sur `127.0.0.1:47382`. `--mandataire` :
//!   derrière un mandataire (VPS), adresse du client lue dans
//!   `X-Forwarded-For`.
//! - `kadenz-portail --dossier <dossier> creer-editeur <identifiant> <nom>` :
//!   crée un compte éditeur, mot de passe lu sur l'entrée standard.

use std::io::BufRead;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;

use kadenz_portail::Portail;
use kadenz_portail::web::{self, ConfigWeb};

const ECOUTE_PAR_DEFAUT: ([u8; 4], u16) = ([127, 0, 0, 1], 47382);

fn usage() -> ExitCode {
    eprintln!(
        "usage : kadenz-portail --dossier <dossier> --cle <fichier> [--ecoute <adresse:port>] [--mandataire]\n\
         \x20       kadenz-portail --dossier <dossier> creer-editeur <identifiant> <nom>"
    );
    ExitCode::FAILURE
}

fn main() -> ExitCode {
    let mut dossier: Option<PathBuf> = None;
    let mut cle: Option<PathBuf> = None;
    let mut ecoute = SocketAddr::from(ECOUTE_PAR_DEFAUT);
    let mut config = ConfigWeb::default();
    let mut creer: Option<(String, String)> = None;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--dossier" => dossier = arguments.next().map(PathBuf::from),
            "--cle" => cle = arguments.next().map(PathBuf::from),
            "--ecoute" => match arguments.next().and_then(|a| a.parse().ok()) {
                Some(a) => ecoute = a,
                None => return usage(),
            },
            "--mandataire" => config.mandataire = true,
            "creer-editeur" => match (arguments.next(), arguments.next()) {
                (Some(i), Some(n)) => creer = Some((i, n)),
                _ => return usage(),
            },
            _ => return usage(),
        }
    }
    let Some(dossier) = dossier else {
        return usage();
    };

    if let Some((identifiant, nom)) = creer {
        // La création d'un compte n'a pas besoin de la clé de signature : une
        // clé jetable sert à ouvrir la base, rien n'est signé.
        let mut graine = [0u8; 32];
        if getrandom::fill(&mut graine).is_err() {
            eprintln!("générateur aléatoire du système indisponible");
            return ExitCode::FAILURE;
        }
        let portail =
            match Portail::ouvrir(&dossier, ed25519_dalek::SigningKey::from_bytes(&graine)) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::FAILURE;
                }
            };
        eprintln!("Mot de passe du compte éditeur (10 caractères au moins) :");
        let mut mot_de_passe = String::new();
        if std::io::stdin()
            .lock()
            .read_line(&mut mot_de_passe)
            .is_err()
        {
            return ExitCode::FAILURE;
        }
        let mot_de_passe = mot_de_passe.trim_end_matches(['\r', '\n']);
        return match portail.creer_editeur(&identifiant, &nom, mot_de_passe) {
            Ok(c) => {
                println!("Compte éditeur créé : {}", c.identifiant);
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        };
    }

    // Jamais de clé d'essai : sans fichier de clé, le portail ne démarre pas.
    let Some(fichier) = cle else {
        eprintln!("clé du portail manquante : --cle <fichier>");
        return usage();
    };
    let texte = match std::fs::read_to_string(&fichier) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{} : {e}", fichier.display());
            return ExitCode::FAILURE;
        }
    };
    let Some(cle) = kadenz_licence::cle_privee_depuis_hex(texte.trim()) else {
        eprintln!("{} : clé privée illisible", fichier.display());
        return ExitCode::FAILURE;
    };
    // Seule une clé dont Kadenz connaît la moitié publique émet des licences
    // acceptées par les installations : toute autre clé est refusée.
    if !kadenz_licence::entreprise::cles_portail().contains(&cle.verifying_key()) {
        eprintln!(
            "{} : cette clé n'est pas une clé du portail reconnue par Kadenz",
            fichier.display()
        );
        return ExitCode::FAILURE;
    }
    let portail = match Portail::ouvrir(&dossier, cle) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let lance = match web::lancer(portail, ecoute, config) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("écoute sur {ecoute} impossible : {e}");
            return ExitCode::FAILURE;
        }
    };
    println!("Portail Kadenz à l'écoute sur http://{}", lance.adresse());
    lance.attendre();
    ExitCode::SUCCESS
}
