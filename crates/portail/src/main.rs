//! Portail des licences de Kadenz (jalon F9).
//!
//! Usage :
//! - `kadenz-portail --dossier <dossier> --cle <fichier> [--ecoute <adresse:port>] [--mandataire <adresse> [--entete-client <en-tête>]]` :
//!   lance le portail. Le fichier de clé contient la clé privée du portail
//!   en hexadécimal (format de `kadenz-editeur`) ; sans lui, le portail ne
//!   démarre pas. Écoute par défaut sur `127.0.0.1:47382`.
//!   `--mandataire <adresse>` : adresse IP du mandataire de confiance (Caddy,
//!   tailscaled pour Funnel, ou cloudflared) ; pour ses requêtes seulement,
//!   l'adresse du client est lue dans l'en-tête donné par `--entete-client` :
//!   `x-forwarded-for` (défaut, dernière valeur) ou `cf-connecting-ip`
//!   (derrière Cloudflare Tunnel).
//! - `kadenz-portail sante [--ecoute <adresse:port>]` : interroge `/sante` du
//!   portail local (vérification de santé de Docker, l'image n'a pas curl) ;
//!   code de sortie 0 si le portail répond 200.
//! - `kadenz-portail --dossier <dossier> creer-editeur <identifiant> <nom>` :
//!   crée un compte éditeur, mot de passe saisi sans écho (ou lu sur l'entrée
//!   standard quand elle n'est pas un terminal).
//! - `kadenz-portail --dossier <dossier> retirer-double-authentification <identifiant>` :
//!   retire la double authentification d'un compte (téléphone perdu sans
//!   code de secours) ; journalisé, le compte la remet en place à sa
//!   prochaine connexion. Sans la clé du portail.
//! - `kadenz-portail --dossier <dossier> journal-verifier` : recalcule la
//!   chaîne de hachage du journal ; affiche le nombre d'entrées et le sceau,
//!   code de sortie 1 si la chaîne est rompue.
//! - `kadenz-portail --dossier <dossier> journal-exporter [--depuis <id>]` :
//!   écrit sur la sortie standard les entrées du journal après `<id>`, une
//!   ligne JSON par entrée avec son empreinte, à copier hors du serveur.

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
        "usage : kadenz-portail --dossier <dossier> --cle <fichier> [--ecoute <adresse:port>] [--mandataire <adresse> [--entete-client x-forwarded-for|cf-connecting-ip]]\n\
         \x20       kadenz-portail --dossier <dossier> creer-editeur <identifiant> <nom>\n\
         \x20       kadenz-portail --dossier <dossier> retirer-double-authentification <identifiant>\n\
         \x20       kadenz-portail --dossier <dossier> journal-verifier\n\
         \x20       kadenz-portail --dossier <dossier> journal-exporter [--depuis <id>]\n\
         \x20       kadenz-portail sante [--ecoute <adresse:port>]"
    );
    ExitCode::FAILURE
}

/// Ouvre la base pour une commande qui ne signe rien (création d'un compte,
/// journal, retrait de la double authentification) : la clé du portail n'est
/// pas lue, et les secrets de double authentification ne sont pas touchés.
fn ouvrir_sans_cle(dossier: &std::path::Path) -> Result<Portail, ExitCode> {
    Portail::ouvrir_sans_cle(dossier).map_err(|e| {
        eprintln!("{e}");
        ExitCode::FAILURE
    })
}

fn verifier_journal(portail: &Portail) -> ExitCode {
    match portail.verifier_journal() {
        Ok(e) => match e.rupture {
            None => {
                println!("Chaîne du journal intacte : {} entrées.", e.entrees);
                println!("Sceau : {}", e.sceau);
                ExitCode::SUCCESS
            }
            Some(id) => {
                println!(
                    "Chaîne du journal rompue à l'entrée {id} ({} entrées vérifiées avant).",
                    e.entrees
                );
                println!("Dernier sceau valable : {}", e.sceau);
                ExitCode::FAILURE
            }
        },
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn exporter_journal(portail: &Portail, depuis: i64) -> ExitCode {
    let entrees = match portail.exporter_journal(depuis) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let mut sortie = std::io::stdout().lock();
    for e in entrees {
        let ligne = serde_json::json!({
            "id": e.id,
            "heure": e.heure,
            "compte": e.compte,
            "action": e.action,
            "cible": e.cible,
            "details": e.details,
            "precedente": e.precedente,
            "empreinte": e.empreinte,
        });
        if writeln!(sortie, "{ligne}").is_err() {
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let mut dossier: Option<PathBuf> = None;
    let mut cle: Option<PathBuf> = None;
    let mut ecoute = SocketAddr::from(ECOUTE_PAR_DEFAUT);
    let mut config = ConfigWeb::default();
    let mut creer: Option<(String, String)> = None;
    let mut sante = false;
    let mut verifier = false;
    let mut retirer: Option<String> = None;
    let mut exporter: Option<i64> = None;
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
            "--entete-client" => {
                match arguments
                    .next()
                    .and_then(|a| web::EnteteClient::depuis_option(&a))
                {
                    Some(e) => config.entete_client = e,
                    None => return usage(),
                }
            }
            "sante" => sante = true,
            "journal-verifier" => verifier = true,
            "retirer-double-authentification" => match arguments.next() {
                Some(i) => retirer = Some(i),
                None => return usage(),
            },
            "journal-exporter" => exporter = Some(exporter.unwrap_or(0)),
            "--depuis" => match arguments.next().and_then(|a| a.parse().ok()) {
                Some(d) => exporter = Some(d),
                None => return usage(),
            },
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

    if let Some(identifiant) = retirer {
        let portail = match ouvrir_sans_cle(&dossier) {
            Ok(p) => p,
            Err(code) => return code,
        };
        return match portail.retirer_double_authentification(&identifiant) {
            Ok(()) => {
                println!(
                    "Double authentification retirée pour {identifiant} : le compte la remettra en place à sa prochaine connexion."
                );
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        };
    }

    if verifier || exporter.is_some() {
        let portail = match ouvrir_sans_cle(&dossier) {
            Ok(p) => p,
            Err(code) => return code,
        };
        return match exporter {
            Some(depuis) => exporter_journal(&portail, depuis),
            None => verifier_journal(&portail),
        };
    }

    if let Some((identifiant, nom)) = creer {
        let portail = match ouvrir_sans_cle(&dossier) {
            Ok(p) => p,
            Err(code) => return code,
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
