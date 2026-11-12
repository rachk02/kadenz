//! Exécutable du serveur Kadenz.
//!
//! - `kadenz-serveur service [--dossier D] [--port P]` : lancé par le
//!   gestionnaire de services de Windows (service `KadenzServeur`).
//! - `kadenz-serveur console [--dossier D] [--port P] [--sans-controle-droits]` :
//!   même serveur au premier plan, pour un essai. À lancer en administrateur :
//!   le dossier des données est réservé à SYSTEM et aux Administrateurs ;
//!   `--sans-controle-droits` sert aux essais dans un autre dossier.
//! - `kadenz-serveur empreinte [--dossier D]` : empreinte du certificat du
//!   serveur, à donner aux postes pour l'inscription.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;

use kadenz_serveur::{Config, PORT_PAR_DEFAUT, Serveur, dossier_par_defaut, empreinte_du_dossier};

const AIDE: &str = "usage : kadenz-serveur service|console|empreinte [--dossier D] [--port P] \
                    [--sans-controle-droits]";

fn config(options: &[String]) -> Result<Config, String> {
    let mut config = Config::nouvelle(dossier_par_defaut());
    let mut port = PORT_PAR_DEFAUT;
    let mut i = 0;
    while i < options.len() {
        match options[i].as_str() {
            "--dossier" => {
                i += 1;
                config.dossier = PathBuf::from(options.get(i).ok_or(AIDE)?);
            }
            "--port" => {
                i += 1;
                port = options
                    .get(i)
                    .and_then(|p| p.parse().ok())
                    .ok_or("port invalide")?;
            }
            "--sans-controle-droits" => config.controle_droits = false,
            autre => return Err(format!("option inconnue : {autre}\n{AIDE}")),
        }
        i += 1;
    }
    config.adresse = SocketAddr::from(([0, 0, 0, 0], port));
    Ok(config)
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let Some((commande, options)) = arguments.split_first() else {
        eprintln!("{AIDE}");
        return ExitCode::FAILURE;
    };
    let config = match config(options) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let resultat = match commande.as_str() {
        "service" => service::lancer(config),
        "console" => console(config),
        "empreinte" => empreinte_du_dossier(&config.dossier).map(|e| println!("{e}")),
        _ => Err(AIDE.to_string()),
    };
    match resultat {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn console(config: Config) -> Result<(), String> {
    let serveur = Serveur::demarrer(config)?;
    println!(
        "Serveur Kadenz à l'écoute sur {}\nEmpreinte du certificat : {}\nArrêt : Ctrl+C",
        serveur.adresse(),
        serveur.empreinte()
    );
    loop {
        std::thread::park();
    }
}

/// Trace du service, dans son dossier des données (il n'a pas de console).
fn noter(config: &Config, texte: &str) {
    use std::io::Write;
    let ligne = format!(
        "[{}] {texte}\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(config.dossier.join(kadenz_serveur::admin::NOM_TRACE))
    {
        let _ = f.write_all(ligne.as_bytes());
    }
}

#[cfg(windows)]
mod service {
    use std::ffi::OsString;
    use std::sync::{OnceLock, mpsc};
    use std::time::Duration;

    use kadenz_serveur::{Config, NOM_SERVICE, Serveur};
    use windows_service::service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    };
    use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
    use windows_service::{define_windows_service, service_dispatcher};

    use super::noter;

    static CONFIG: OnceLock<Config> = OnceLock::new();

    define_windows_service!(entree_service, service_principal);

    pub fn lancer(config: Config) -> Result<(), String> {
        let _ = CONFIG.set(config);
        service_dispatcher::start(NOM_SERVICE, entree_service).map_err(|e| e.to_string())
    }

    fn etat(etat: ServiceState, acceptes: ServiceControlAccept, code: u32) -> ServiceStatus {
        ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: etat,
            controls_accepted: acceptes,
            exit_code: if code == 0 {
                ServiceExitCode::Win32(0)
            } else {
                ServiceExitCode::ServiceSpecific(code)
            },
            checkpoint: 0,
            wait_hint: Duration::from_secs(10),
            process_id: None,
        }
    }

    fn service_principal(_arguments: Vec<OsString>) {
        let Some(config) = CONFIG.get().cloned() else {
            return;
        };
        let (arret, attente) = mpsc::channel();
        let gestion = move |controle| match controle {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                let _ = arret.send(());
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        };
        let Ok(statut) = service_control_handler::register(NOM_SERVICE, gestion) else {
            return;
        };
        let vide = ServiceControlAccept::empty();
        let _ = statut.set_service_status(etat(ServiceState::StartPending, vide, 0));
        match Serveur::demarrer(config.clone()) {
            Ok(serveur) => {
                noter(
                    &config,
                    &format!(
                        "démarrage sur {}, empreinte {}",
                        serveur.adresse(),
                        serveur.empreinte()
                    ),
                );
                let acceptes = ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN;
                let _ = statut.set_service_status(etat(ServiceState::Running, acceptes, 0));
                let _ = attente.recv();
                let _ = statut.set_service_status(etat(ServiceState::StopPending, vide, 0));
                serveur.arreter();
                noter(&config, "arrêt");
                let _ = statut.set_service_status(etat(ServiceState::Stopped, vide, 0));
            }
            Err(e) => {
                noter(&config, &format!("démarrage impossible : {e}"));
                let _ = statut.set_service_status(etat(ServiceState::Stopped, vide, 1));
            }
        }
    }
}

#[cfg(not(windows))]
mod service {
    use kadenz_serveur::Config;

    pub fn lancer(_config: Config) -> Result<(), String> {
        Err("le service n'existe que sous Windows".into())
    }
}
