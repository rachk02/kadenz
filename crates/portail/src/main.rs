//! Portail des licences de Kadenz (jalon F9).
//!
//! Usage :
//! - `kadenz-portail --dossier <dossier> --cle <fichier> [--ecoute <adresse:port>] [--mandataire <adresse>]` :
//!   lance le portail. Le fichier de clé contient la clé privée du portail
//!   en hexadécimal (format de `kadenz-editeur`) ; sans lui, le portail ne
//!   démarre pas. Écoute par défaut sur `127.0.0.1:47382`.
//!   `--mandataire <adresse>` : adresse IP du mandataire de confiance (Caddy,
//!   ou tailscaled pour Funnel) ; pour ses requêtes seulement, l'adresse du
//!   client est lue dans `X-Forwarded-For`.
//! - `kadenz-portail sante [--ecoute <adresse:port>]` : interroge `/sante` du
//!   portail local (vérification de santé de Docker, l'image n'a pas curl) ;
//!   code de sortie 0 si le portail répond 200.
//! - `kadenz-portail --dossier <dossier> creer-editeur <identifiant> <nom>` :
//!   crée un compte éditeur, mot de passe saisi sans écho (ou lu sur l'entrée
//!   standard quand elle n'est pas un terminal).

use std::io::{BufRead, IsTerminal, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use kadenz_portail::web::{self, ConfigWeb};
use kadenz_portail::{LONGUEUR_MIN_MOT_DE_PASSE, Portail};
use zeroize::Zeroize;

const ECOUTE_PAR_DEFAUT: ([u8; 4], u16) = ([127, 0, 0, 1], 47382);

fn usage() -> ExitCode {
    eprintln!(
        "usage : kadenz-portail --dossier <dossier> --cle <fichier> [--ecoute <adresse:port>] [--mandataire <adresse>]\n\
         \x20       kadenz-portail --dossier <dossier> creer-editeur <identifiant> <nom>\n\
         \x20       kadenz-portail sante [--ecoute <adresse:port>]"
    );
    ExitCode::FAILURE
}

fn main() -> ExitCode {
    let mut dossier: Option<PathBuf> = None;
    let mut cle: Option<PathBuf> = None;
    let mut ecoute = SocketAddr::from(ECOUTE_PAR_DEFAUT);
    let mut config = ConfigWeb::default();
    let mut creer: Option<(String, String)> = None;
    let mut sante = false;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--dossier" => dossier = arguments.next().map(PathBuf::from),
            "--cle" => cle = arguments.next().map(PathBuf::from),
            "--ecoute" => match arguments.next().and_then(|a| a.parse().ok()) {
                Some(a) => ecoute = a,
                None => return usage(),
            },
            "--mandataire" => match arguments.next().and_then(|a| a.parse().ok()) {
                Some(a) => config.mandataire = Some(a),
                None => return usage(),
            },
            "sante" => sante = true,
            "creer-editeur" => match (arguments.next(), arguments.next()) {
                (Some(i), Some(n)) => creer = Some((i, n)),
                _ => return usage(),
            },
            _ => return usage(),
        }
    }
    if sante {
        return verifier_sante(ecoute);
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
        let invite = format!(
            "Mot de passe du compte éditeur ({LONGUEUR_MIN_MOT_DE_PASSE} caractères au moins) : "
        );
        let mut mot_de_passe = if std::io::stdin().is_terminal() {
            match rpassword::prompt_password(&invite) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("saisie du mot de passe impossible : {e}");
                    return ExitCode::FAILURE;
                }
            }
        } else {
            eprintln!("{invite}");
            let mut lu = String::new();
            if std::io::stdin().lock().read_line(&mut lu).is_err() {
                return ExitCode::FAILURE;
            }
            lu
        };
        let resultat = portail.creer_editeur(
            &identifiant,
            &nom,
            mot_de_passe.trim_end_matches(['\r', '\n']),
        );
        mot_de_passe.zeroize();
        return match resultat {
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
    if let Err(message) = droits_de_la_cle(&fichier) {
        eprintln!("{} : {message}", fichier.display());
        return ExitCode::FAILURE;
    }
    let mut texte = match std::fs::read_to_string(&fichier) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{} : {e}", fichier.display());
            return ExitCode::FAILURE;
        }
    };
    let cle = kadenz_licence::cle_privee_depuis_hex(texte.trim());
    // Le texte de la clé est effacé de la mémoire dès qu'il est lu ; la clé
    // elle-même s'efface à sa destruction (ed25519-dalek, zeroize).
    texte.zeroize();
    let Some(cle) = cle else {
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

/// Le fichier de clé ne doit être lisible que par son propriétaire (mode 400
/// ou 600, docs/deploiement-vps.md) : un fichier lisible par le groupe ou par
/// tous est refusé, avant même d'être lu.
#[cfg(unix)]
fn droits_de_la_cle(fichier: &std::path::Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::metadata(fichier)
        .map_err(|e| e.to_string())?
        .permissions()
        .mode();
    if mode & 0o077 != 0 {
        return Err(format!(
            "fichier de clé lisible par d'autres que son propriétaire (mode {:o}) : chmod 400",
            mode & 0o777
        ));
    }
    Ok(())
}

/// Sous Windows (développement seulement), les droits passent par les ACL :
/// pas de contrôle ici.
#[cfg(not(unix))]
fn droits_de_la_cle(_fichier: &std::path::Path) -> Result<(), String> {
    Ok(())
}

/// Santé du portail local : `GET /sante` sur l'adresse d'écoute (une écoute
/// sur toutes les adresses est interrogée en 127.0.0.1), réponse 200 attendue
/// en 3 secondes au plus.
fn verifier_sante(ecoute: SocketAddr) -> ExitCode {
    let mut cible = ecoute;
    if cible.ip().is_unspecified() {
        cible.set_ip([127, 0, 0, 1].into());
    }
    let delai = Duration::from_secs(3);
    let reponse = (|| -> std::io::Result<String> {
        let mut flux = TcpStream::connect_timeout(&cible, delai)?;
        flux.set_read_timeout(Some(delai))?;
        flux.set_write_timeout(Some(delai))?;
        flux.write_all(b"GET /sante HTTP/1.0\r\nHost: localhost\r\nConnection: close\r\n\r\n")?;
        let mut texte = String::new();
        flux.take(4096).read_to_string(&mut texte)?;
        Ok(texte)
    })();
    match reponse {
        Ok(texte) if texte.starts_with("HTTP/1.1 200") || texte.starts_with("HTTP/1.0 200") => {
            ExitCode::SUCCESS
        }
        Ok(texte) => {
            eprintln!(
                "santé : réponse inattendue : {}",
                texte.lines().next().unwrap_or("")
            );
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("santé : {cible} injoignable : {e}");
            ExitCode::FAILURE
        }
    }
}
