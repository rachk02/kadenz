//! Outils de la fenêtre d'administration du serveur (phase 2, jalon F4 ;
//! décision 30 ; écrans, section 1). La fenêtre tourne en administrateur sur
//! la machine du serveur et ouvre la base directement, comme le service ;
//! chaque écriture passe par les fonctions du stockage, et le service ne garde
//! rien en mémoire qui puisse devenir faux.

use std::net::{IpAddr, UdpSocket};
use std::path::Path;

use kadenz_protocole::inscription::CodeComplet;
use kadenz_stockage::comptes::Utilisateur;
use kadenz_stockage::inscription::CodeCree;
use kadenz_stockage::installation::{self, InstallationInitiale};
use kadenz_stockage::{Erreur, abonnement, synchro};
use rusqlite::{Connection, OptionalExtension};

use crate::{NOM_BASE, PORT_PAR_DEFAUT, empreinte_du_dossier};

const CLE_PORT: &str = "port_serveur";

/// Ouvre la base du serveur (clé protégée au niveau de la machine), avec un
/// délai d'attente sur verrou : le service peut écrire en même temps.
pub fn ouvrir_base(dossier: &Path) -> Result<Connection, String> {
    kadenz_stockage::ouvrir_serveur(&dossier.join(NOM_BASE))
        .map_err(|e| format!("base du serveur : {e}"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtatService {
    NonInstalle,
    Arrete,
    Demarrage,
    Demarre,
    Arret,
    /// Pause ou reprise : le service Kadenz ne les accepte pas.
    Autre,
}

/// État du service Windows du serveur.
#[cfg(windows)]
pub fn etat_service() -> Result<EtatService, String> {
    use windows_service::service::{ServiceAccess, ServiceState};
    use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

    /// Le service n'existe pas (ERROR_SERVICE_DOES_NOT_EXIST).
    const INEXISTANT: i32 = 1060;
    let gestionnaire = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .map_err(|e| format!("gestionnaire de services inaccessible : {e}"))?;
    let service = match gestionnaire.open_service(crate::NOM_SERVICE, ServiceAccess::QUERY_STATUS) {
        Ok(s) => s,
        Err(windows_service::Error::Winapi(e)) if e.raw_os_error() == Some(INEXISTANT) => {
            return Ok(EtatService::NonInstalle);
        }
        Err(e) => return Err(format!("service illisible : {e}")),
    };
    let statut = service
        .query_status()
        .map_err(|e| format!("état du service illisible : {e}"))?;
    Ok(match statut.current_state {
        ServiceState::Stopped => EtatService::Arrete,
        ServiceState::StartPending => EtatService::Demarrage,
        ServiceState::Running => EtatService::Demarre,
        ServiceState::StopPending => EtatService::Arret,
        _ => EtatService::Autre,
    })
}

#[cfg(not(windows))]
pub fn etat_service() -> Result<EtatService, String> {
    Ok(EtatService::NonInstalle)
}

/// Garde le port d'écoute du service, pour le code complet.
pub(crate) fn noter_port(conn: &Connection, port: u16) -> Result<(), String> {
    conn.execute(
        "INSERT INTO parametre_systeme (cle, valeur) VALUES (?1, ?2)
         ON CONFLICT (cle) DO UPDATE SET valeur = excluded.valeur",
        rusqlite::params![CLE_PORT, port.to_string()],
    )
    .map(|_| ())
    .map_err(|e| format!("base du serveur : {e}"))
}

/// Port d'écoute noté au dernier démarrage du service, sinon le port par
/// défaut.
pub fn port(conn: &Connection) -> u16 {
    conn.query_row(
        "SELECT valeur FROM parametre_systeme WHERE cle = ?1",
        [CLE_PORT],
        |r| r.get::<_, String>(0),
    )
    .optional()
    .ok()
    .flatten()
    .and_then(|p| p.parse().ok())
    .unwrap_or(PORT_PAR_DEFAUT)
}

/// Adresse IPv4 de la machine sur le réseau local, par la table de routage
/// (une socket UDP « connectée » n'envoie rien).
fn adresse_locale() -> Option<IpAddr> {
    let socket = UdpSocket::bind(("0.0.0.0", 0)).ok()?;
    socket.connect(("192.0.2.1", 9)).ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_unspecified() && !ip.is_loopback()).then_some(ip)
}

/// Code complet d'un code d'inscription, à transmettre au poste : identifiant
/// de l'entreprise, empreinte du certificat du serveur, adresses (nom de la
/// machine, puis adresse IPv4 locale) au port du service, et secret.
pub fn code_complet(conn: &Connection, dossier: &Path, code: &CodeCree) -> Result<String, String> {
    let entreprise = synchro::identifiant_entreprise(conn).map_err(|e| e.to_string())?;
    let empreinte = empreinte_du_dossier(dossier)?;
    let port = port(conn);
    let mut adresses = Vec::new();
    if let Ok(machine) = std::env::var("COMPUTERNAME")
        && !machine.is_empty()
    {
        adresses.push(format!("{machine}:{port}"));
    }
    if let Some(ip) = adresse_locale() {
        adresses.push(format!("{ip}:{port}"));
    }
    Ok(CodeComplet {
        entreprise,
        empreinte,
        adresses,
        secret: code.secret.clone(),
    }
    .encoder())
}

/// Assistant d'un serveur neuf : crée l'entreprise et le premier Admin et
/// enregistre la licence d'abonnement, tout ou rien ; une licence refusée
/// n'installe rien. Sans licence, le serveur est en lecture seule (décision
/// 40) et un seul poste peut s'inscrire, le temps d'installer la licence
/// depuis ce poste (décision 41).
pub fn installer(
    conn: &mut Connection,
    init: &InstallationInitiale,
    licence: Option<&str>,
) -> Result<Utilisateur, Erreur> {
    installer_avec(
        conn,
        init,
        licence,
        &kadenz_licence::entreprise::cles_portail(),
    )
}

#[doc(hidden)]
pub fn installer_avec(
    conn: &mut Connection,
    init: &InstallationInitiale,
    licence: Option<&str>,
    cles_portail: &[kadenz_licence::VerifyingKey],
) -> Result<Utilisateur, Erreur> {
    conn.execute_batch("BEGIN IMMEDIATE")?;
    let resultat = installation::installer(conn, init).and_then(|admin| match licence {
        Some(cle) => {
            abonnement::enregistrer_avec(conn, &admin, cle, abonnement::horloge(), cles_portail)
                .map(|_| admin)
        }
        None => Ok(admin),
    });
    match resultat {
        Ok(admin) => {
            conn.execute_batch("COMMIT")?;
            Ok(admin)
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

/// Fichier de trace du service, dans son dossier des données.
pub const NOM_TRACE: &str = "kadenz-serveur.log";

/// Dernière erreur du service : échec du dernier démarrage, ou problème non
/// bloquant noté depuis (découverte impossible, décision 44). Un démarrage
/// réussi sans problème efface les erreurs précédentes.
pub fn derniere_erreur(dossier: &Path) -> Option<String> {
    let trace = std::fs::read_to_string(dossier.join(NOM_TRACE)).ok()?;
    for ligne in trace.lines().rev().filter(|l| !l.trim().is_empty()) {
        if ligne.contains("impossible") {
            return Some(ligne.to_string());
        }
        if ligne.contains("] démarrage sur ") {
            return None;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derniere_erreur_de_la_trace() {
        let dossier =
            std::env::temp_dir().join(format!("kadenz-serveur-trace-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        assert_eq!(derniere_erreur(&dossier), None);
        let erreur = "[2026-11-12 10:00:00] démarrage impossible : port pris";
        std::fs::write(dossier.join(NOM_TRACE), format!("{erreur}\n")).unwrap();
        assert_eq!(derniere_erreur(&dossier).as_deref(), Some(erreur));
        std::fs::write(
            dossier.join(NOM_TRACE),
            format!("{erreur}\n[2026-11-12 10:01:00] démarrage sur 0.0.0.0:47380\n"),
        )
        .unwrap();
        assert_eq!(derniere_erreur(&dossier), None);
        // Découverte impossible, notée après le démarrage, puis arrêt.
        let udp = "[2026-11-12 10:01:00] découverte sur 0.0.0.0:47380 impossible : port pris";
        std::fs::write(
            dossier.join(NOM_TRACE),
            format!(
                "{erreur}\n[2026-11-12 10:01:00] démarrage sur 0.0.0.0:47380\n{udp}\n\
                 [2026-11-12 11:00:00] arrêt\n"
            ),
        )
        .unwrap();
        assert_eq!(derniere_erreur(&dossier).as_deref(), Some(udp));
        std::fs::remove_dir_all(&dossier).unwrap();
        // Le service n'est pas installé sur la machine des essais, ou il l'est :
        // la lecture de son état ne doit pas échouer.
        assert!(etat_service().is_ok());
    }
}
