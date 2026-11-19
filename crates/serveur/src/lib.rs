//! Serveur Kadenz de l'entreprise (phase 2, jalon F2) : base de référence,
//! sessions TLS 1.3 avec les postes inscrits, un seul fil d'accès à la base.
//!
//! Spécification de synchronisation, section 7 : le poste est reconnu à
//! l'empreinte de son certificat (table `poste_inscrit`), le compte par son
//! mot de passe ; une opération n'est acceptée que si son poste est celui du
//! certificat et son compte celui de la session. Limites : taille des
//! messages, délai de lecture par connexion, nombre de connexions
//! simultanées, attente croissante après des échecs de connexion.

pub mod admin;
mod distance;
pub mod droits;
mod restauration;
pub mod sauvegardes;

use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use kadenz_client::{Identite, NOM_TLS, empreinte, fournisseur};
use kadenz_domaine::message::MessageCompose;
use kadenz_domaine::role::{Droit, Role};
use kadenz_licence::VerifyingKey;
use kadenz_protocole::abonnement::Refus;
use kadenz_protocole::inscription::{DECOUVERTE, ServeurTrouve};
use kadenz_protocole::messages::{
    self, CHANGEMENTS_MAX, ErreurTrame, LIGNES_MAX, Reponse, Requete,
};
use kadenz_protocole::{Entite, VERSION};
use kadenz_stockage::chiffrement::{self, Portee};
use kadenz_stockage::{
    Erreur, abonnement, comptes, inscription, postes, reprise, sauvegarde, synchro,
};
use rusqlite::Connection;
use rustls::client::danger::HandshakeSignatureValid;
use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, UnixTime};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::{
    DigitallySignedStruct, DistinguishedName, ServerConfig, ServerConnection, SignatureScheme,
    StreamOwned,
};

pub use kadenz_protocole::inscription::PORT_PAR_DEFAUT;

/// Nom du service Windows.
pub const NOM_SERVICE: &str = "KadenzServeur";

/// Base de référence, dans le dossier des données ; sa clé est à côté
/// (`kadenz-serveur.cle`), protégée par Windows au niveau de la machine.
pub const NOM_BASE: &str = "kadenz-serveur.sqlite";
/// Certificat TLS du serveur (DER) ; son empreinte est épinglée par les postes.
pub const NOM_CERTIFICAT: &str = "serveur.crt";
/// Clé privée TLS, protégée par Windows au niveau de la machine.
const NOM_CLE_TLS: &str = "serveur.cle-tls";

/// Marque d'un dossier de serveur intégré (portée `Compte`, monoposte) :
/// sans elle, le dossier est celui d'un service (portée `Machine`).
pub const NOM_MARQUE_COMPTE: &str = "portee-compte";

/// Verrou du dossier, tenu tant que le serveur tourne : deux serveurs ne
/// s'ouvrent jamais sur la même base.
pub const NOM_VERROU: &str = "serveur.verrou";

/// Échecs de connexion permis avant la première attente.
const ECHECS_LIBRES: u32 = 3;
/// Première attente imposée, doublée à chaque nouvel échec.
const ATTENTE_BASE: u64 = 5;
/// Attente maximale après des échecs de connexion.
const ATTENTE_MAX: u64 = 900;
/// Échecs d'inscription permis par adresse avant la première attente.
const ECHECS_LIBRES_INSCRIPTION: u32 = 5;
/// Échecs d'inscription par heure, toutes adresses confondues : par le relais
/// (F8), tous les postes distants ont la même adresse vue du serveur.
const ECHECS_INSCRIPTION_PAR_HEURE: usize = 20;
/// Taille maximale de la trame d'un poste pas encore inscrit.
const TAILLE_INSCRIPTION: usize = 4096;

#[derive(Debug, Clone)]
pub struct Config {
    pub dossier: PathBuf,
    pub adresse: SocketAddr,
    /// Connexions simultanées ; au-delà, une connexion est fermée aussitôt.
    pub connexions_max: usize,
    /// Délai d'attente de chaque lecture et écriture sur une connexion.
    pub delai_lecture: Duration,
    /// Délai total de l'ouverture d'une session (poignée de main TLS puis
    /// `Bonjour`) : une connexion qui envoie un octet de temps en temps ne
    /// garde pas sa place.
    pub delai_ouverture: Duration,
    /// Droits du dossier posés à sa création et vérifiés au démarrage. Coupé
    /// seulement pour les essais, sous un compte sans droits d'administrateur.
    pub controle_droits: bool,
    /// Clés du portail qui signent les licences d'abonnement : celles
    /// embarquées, sauf pour les essais.
    pub cles_portail: Vec<VerifyingKey>,
    /// Réponse aux requêtes de découverte du réseau local, en UDP sur le même
    /// port que les sessions.
    pub decouverte: bool,
    /// Version du protocole parlée : [`VERSION`], sauf pour les essais d'un
    /// poste face à un serveur d'une autre version (F10).
    pub version_protocole: u32,
    /// Portée de la protection des clés et droits du dossier : `Machine` pour
    /// le service du multiposte, `Compte` pour le serveur intégré du
    /// monoposte (spécification du monoposte, section 2.3).
    pub portee: Portee,
    /// Écoute sur le port noté au démarrage précédent (adresse de
    /// `adresse`), ou sur un port libre s'il est pris ; le port effectif est
    /// noté et rendu par [`Serveur::adresse`]. Pour le monoposte, dont le
    /// poste réécrit son adresse (section 2.1).
    pub reprendre_port: bool,
}

impl Config {
    pub fn nouvelle(dossier: PathBuf) -> Config {
        Config {
            dossier,
            adresse: SocketAddr::from(([0, 0, 0, 0], PORT_PAR_DEFAUT)),
            connexions_max: 64,
            delai_lecture: Duration::from_secs(60),
            delai_ouverture: Duration::from_secs(10),
            controle_droits: true,
            cles_portail: kadenz_licence::entreprise::cles_portail(),
            decouverte: true,
            version_protocole: VERSION,
            portee: Portee::Machine,
            reprendre_port: false,
        }
    }

    /// Serveur intégré du monoposte : 127.0.0.1 seulement, sans découverte,
    /// clés et droits au niveau du compte, port repris d'un lancement à
    /// l'autre.
    pub fn monoposte(dossier: PathBuf) -> Config {
        Config {
            adresse: SocketAddr::from(([127, 0, 0, 1], 0)),
            decouverte: false,
            portee: Portee::Compte,
            reprendre_port: true,
            ..Config::nouvelle(dossier)
        }
    }
}

/// Refus du démarrage du serveur, par cause : l'écran « Démarrage
/// impossible » de l'application choisit son message traduit d'après la
/// cause (spécification du monoposte, section 2) ; `Display` donne le texte
/// de la trace.
#[derive(Debug)]
pub enum ErreurDemarrage {
    /// Un autre serveur tient déjà le verrou du dossier.
    Verrouille,
    /// Droits du dossier impossibles à poser ou différents de ceux voulus.
    Droits(String),
    /// Le dossier appartient à l'autre édition (service ou serveur intégré).
    AutreEdition,
    /// Base du serveur ou sa clé : introuvable, d'un autre compte,
    /// endommagée, indéchiffrable, d'une version plus récente.
    Base(kadenz_stockage::Erreur),
    /// Certificat ou clé TLS du serveur illisible.
    Identite(String),
    /// Écoute impossible, même sur un port libre.
    Ecoute(String),
    /// Autre cause (dossier non créé, port non noté).
    Autre(String),
}

impl std::fmt::Display for ErreurDemarrage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErreurDemarrage::Verrouille => {
                write!(
                    f,
                    "un serveur est déjà ouvert sur ce dossier : démarrage refusé"
                )
            }
            ErreurDemarrage::AutreEdition => write!(
                f,
                "le dossier appartient à l'autre édition du serveur : démarrage refusé"
            ),
            ErreurDemarrage::Base(e) => write!(f, "base du serveur : {e}"),
            ErreurDemarrage::Droits(m)
            | ErreurDemarrage::Identite(m)
            | ErreurDemarrage::Ecoute(m)
            | ErreurDemarrage::Autre(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for ErreurDemarrage {}

impl From<ErreurDemarrage> for String {
    fn from(e: ErreurDemarrage) -> String {
        e.to_string()
    }
}

/// Dossier du serveur intégré du monoposte, dans le profil du compte :
/// `%LOCALAPPDATA%\Kadenz\Serveur`.
pub fn dossier_monoposte() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|d| d.join("Kadenz").join("Serveur"))
}

/// Portée d'un dossier de serveur, d'après sa marque.
pub fn portee_du_dossier(dossier: &Path) -> Portee {
    if dossier.join(NOM_MARQUE_COMPTE).is_file() {
        Portee::Compte
    } else {
        Portee::Machine
    }
}

/// Dossier des données du service : `%ProgramData%\Kadenz\Serveur`.
pub fn dossier_par_defaut() -> PathBuf {
    std::env::var_os("ProgramData")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"))
        .join("Kadenz")
        .join("Serveur")
}

/// Empreinte du certificat du serveur, à donner aux postes pour l'inscription.
pub fn empreinte_du_dossier(dossier: &Path) -> Result<String, String> {
    let certificat = fs::read(dossier.join(NOM_CERTIFICAT))
        .map_err(|e| format!("certificat du serveur illisible : {e}"))?;
    Ok(empreinte(&certificat))
}

fn preparer_dossier(config: &Config) -> Result<(), ErreurDemarrage> {
    // Un dossier vide, créé par un autre que le service, est traité comme neuf.
    let nouveau = fs::read_dir(&config.dossier)
        .map(|mut d| d.next().is_none())
        .unwrap_or(true);
    fs::create_dir_all(&config.dossier).map_err(|e| {
        ErreurDemarrage::Autre(format!(
            "dossier {} non créé : {e}",
            config.dossier.display()
        ))
    })?;
    if config.controle_droits {
        if nouveau {
            droits::restreindre(&config.dossier, config.portee).map_err(ErreurDemarrage::Droits)?;
        }
        let ecarts =
            droits::verifier(&config.dossier, config.portee).map_err(ErreurDemarrage::Droits)?;
        if !ecarts.is_empty() {
            return Err(ErreurDemarrage::Droits(match config.portee {
                Portee::Machine => format!(
                    "le dossier {} doit appartenir à SYSTEM ou aux Administrateurs et leur être \
                     réservé ({}) : démarrage refusé",
                    config.dossier.display(),
                    ecarts.join(", ")
                ),
                Portee::Compte => format!(
                    "le dossier {} doit appartenir au compte Windows courant et lui être \
                     réservé, avec SYSTEM ({}) : démarrage refusé",
                    config.dossier.display(),
                    ecarts.join(", ")
                ),
            }));
        }
    }
    // Un dossier ne change jamais de portée : les clés déjà protégées ne
    // suivraient pas (passage au multiposte, section 8, non spécifié).
    if nouveau && config.portee == Portee::Compte {
        fs::write(config.dossier.join(NOM_MARQUE_COMPTE), b"")
            .map_err(|e| ErreurDemarrage::Autre(format!("marque du dossier non écrite : {e}")))?;
    }
    if portee_du_dossier(&config.dossier) != config.portee {
        return Err(ErreurDemarrage::AutreEdition);
    }
    Ok(())
}

/// Verrou exclusif du dossier, gardé par le serveur jusqu'à son arrêt ; un
/// second serveur sur le même dossier est refusé.
fn prendre_verrou(dossier: &Path) -> Result<fs::File, ErreurDemarrage> {
    let fichier = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dossier.join(NOM_VERROU))
        .map_err(|e| ErreurDemarrage::Autre(format!("verrou du dossier impossible : {e}")))?;
    match fichier.try_lock() {
        Ok(()) => Ok(fichier),
        Err(fs::TryLockError::WouldBlock) => Err(ErreurDemarrage::Verrouille),
        Err(fs::TryLockError::Error(e)) => Err(ErreurDemarrage::Autre(format!(
            "verrou du dossier impossible : {e}"
        ))),
    }
}

/// Écoute du serveur. Avec `reprendre_port`, le port noté au démarrage
/// précédent est repris, ou un port libre s'il est pris.
fn ecouter_sur(config: &Config, conn: &Connection) -> Result<TcpListener, ErreurDemarrage> {
    let voulue = match admin::port_note(conn).filter(|_| config.reprendre_port) {
        Some(port) => SocketAddr::new(config.adresse.ip(), port),
        None => config.adresse,
    };
    match TcpListener::bind(voulue) {
        Ok(ecoute) => Ok(ecoute),
        Err(_) if config.reprendre_port && voulue.port() != 0 => {
            TcpListener::bind(SocketAddr::new(voulue.ip(), 0)).map_err(|e| {
                ErreurDemarrage::Ecoute(format!("écoute sur {} impossible : {e}", voulue.ip()))
            })
        }
        Err(e) => Err(ErreurDemarrage::Ecoute(format!(
            "écoute sur {voulue} impossible : {e}"
        ))),
    }
}

/// Identité TLS du serveur, créée au premier démarrage. Tant que le
/// certificat n'est pas écrit, son empreinte n'a pu être donnée à aucun poste :
/// une clé seule est remplacée.
fn identite(dossier: &Path, portee: Portee) -> Result<Identite, String> {
    let chemin_certificat = dossier.join(NOM_CERTIFICAT);
    let chemin_cle = dossier.join(NOM_CLE_TLS);
    if chemin_certificat.exists() {
        let certificat = fs::read(&chemin_certificat)
            .map_err(|e| format!("certificat du serveur illisible : {e}"))?;
        let protegee =
            fs::read(&chemin_cle).map_err(|e| format!("clé TLS du serveur illisible : {e}"))?;
        let cle = chiffrement::deproteger(&protegee)
            .ok_or("clé TLS du serveur illisible sur cette machine")?;
        return Ok(Identite { certificat, cle });
    }
    let nouvelle = Identite::nouvelle(NOM_TLS)?;
    let protegee =
        chiffrement::proteger(&nouvelle.cle, portee).ok_or("clé TLS du serveur non protégée")?;
    fs::write(&chemin_cle, protegee).map_err(|e| format!("clé TLS non écrite : {e}"))?;
    fs::write(&chemin_certificat, &nouvelle.certificat)
        .map_err(|e| format!("certificat non écrit : {e}"))?;
    Ok(nouvelle)
}

/// Le certificat du poste est exigé et sa signature de la poignée de main
/// vérifiée ; son inscription est contrôlée juste après, contre la base.
#[derive(Debug)]
struct CertificatDuPoste {
    fournisseur: Arc<CryptoProvider>,
}

impl ClientCertVerifier for CertificatDuPoste {
    fn client_auth_mandatory(&self) -> bool {
        true
    }

    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &[]
    }

    fn verify_client_cert(
        &self,
        _certificat: &CertificateDer<'_>,
        _intermediaires: &[CertificateDer<'_>],
        _maintenant: UnixTime,
    ) -> Result<ClientCertVerified, rustls::Error> {
        Ok(ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        certificat: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(
            message,
            certificat,
            signature,
            &self.fournisseur.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificat: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            certificat,
            signature,
            &self.fournisseur.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.fournisseur
            .signature_verification_algorithms
            .supported_schemes()
    }
}

fn config_tls(identite: &Identite) -> Result<Arc<ServerConfig>, String> {
    let fournisseur = fournisseur();
    let verification = Arc::new(CertificatDuPoste {
        fournisseur: fournisseur.clone(),
    });
    let config = ServerConfig::builder_with_provider(fournisseur)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|e| e.to_string())?
        .with_client_cert_verifier(verification)
        .with_single_cert(identite.chaine(), identite.cle_privee())
        .map_err(|e| e.to_string())?;
    Ok(Arc::new(config))
}

type Tache = Box<dyn FnOnce(&mut Connection) + Send>;

/// Accès à la base, toujours depuis le même fil : les opérations sont
/// appliquées l'une après l'autre.
#[derive(Clone)]
struct Base(mpsc::Sender<Tache>);

impl Base {
    fn lancer(mut conn: Connection) -> Base {
        let (envoi, reception) = mpsc::channel::<Tache>();
        thread::spawn(move || {
            for tache in reception {
                if catch_unwind(AssertUnwindSafe(|| tache(&mut conn))).is_err()
                    && !conn.is_autocommit()
                {
                    // Une tâche interrompue ne laisse pas de transaction ouverte.
                    let _ = conn.execute_batch("ROLLBACK");
                }
            }
        });
        Base(envoi)
    }

    fn executer<R: Send + 'static>(
        &self,
        tache: impl FnOnce(&mut Connection) -> R + Send + 'static,
    ) -> Result<R, String> {
        let (envoi, reception) = mpsc::sync_channel(1);
        self.0
            .send(Box::new(move |conn| {
                let _ = envoi.send(tache(conn));
            }))
            .map_err(|_| "base du serveur arrêtée".to_string())?;
        reception
            .recv()
            .map_err(|_| "erreur du serveur".to_string())
    }
}

/// Échecs de connexion, comptés par poste.
#[derive(Default)]
struct Essais(HashMap<String, (u32, Instant)>);

fn duree_attente(echecs: u32, libres: u32) -> u64 {
    if echecs < libres {
        0
    } else {
        (ATTENTE_BASE << (echecs - libres).min(10)).min(ATTENTE_MAX)
    }
}

impl Essais {
    /// Secondes à attendre avant un nouvel essai.
    fn attente(&self, cles: &[String], maintenant: Instant) -> u64 {
        cles.iter()
            .filter_map(|c| self.0.get(c))
            .map(|(_, fin)| {
                fin.saturating_duration_since(maintenant)
                    .as_millis()
                    .div_ceil(1000) as u64
            })
            .max()
            .unwrap_or(0)
    }

    fn echec(&mut self, cles: &[String], maintenant: Instant) -> u64 {
        self.echec_avec(cles, maintenant, ECHECS_LIBRES)
    }

    fn echec_avec(&mut self, cles: &[String], maintenant: Instant, libres: u32) -> u64 {
        for cle in cles {
            let entree = self.0.entry(cle.clone()).or_insert((0, maintenant));
            entree.0 += 1;
            entree.1 = maintenant + Duration::from_secs(duree_attente(entree.0, libres));
        }
        self.attente(cles, maintenant)
    }

    fn reussite(&mut self, cles: &[String]) {
        for cle in cles {
            self.0.remove(cle);
        }
    }
}

/// Échecs d'inscription : par adresse, et toutes adresses confondues sur la
/// dernière heure.
#[derive(Default)]
struct Inscriptions {
    par_adresse: Essais,
    globaux: std::collections::VecDeque<Instant>,
}

impl Inscriptions {
    fn attente(&mut self, cles: &[String], maintenant: Instant) -> u64 {
        let heure = Duration::from_secs(3600);
        while self
            .globaux
            .front()
            .is_some_and(|&t| maintenant.duration_since(t) >= heure)
        {
            self.globaux.pop_front();
        }
        let globale = match self.globaux.front() {
            Some(&t) if self.globaux.len() >= ECHECS_INSCRIPTION_PAR_HEURE => (t + heure)
                .saturating_duration_since(maintenant)
                .as_secs()
                .max(1),
            _ => 0,
        };
        globale.max(self.par_adresse.attente(cles, maintenant))
    }

    /// L'essai compte comme un échec dès maintenant, effacé s'il réussit.
    fn reserver(&mut self, cles: &[String], maintenant: Instant) {
        self.globaux.push_back(maintenant);
        self.par_adresse
            .echec_avec(cles, maintenant, ECHECS_LIBRES_INSCRIPTION);
    }

    fn reussite(&mut self, cles: &[String]) {
        self.globaux.pop_back();
        self.par_adresse.reussite(cles);
    }
}

struct Contexte {
    base: Base,
    tls: Arc<ServerConfig>,
    delai: Duration,
    ouverture: Duration,
    entreprise: String,
    /// Génération de la base, lue au démarrage (F7).
    generation: String,
    essais: Mutex<Essais>,
    inscriptions: Mutex<Inscriptions>,
    cles: Arc<[VerifyingKey]>,
    /// Serveur arrêté : chaque session finit à sa requête suivante.
    arret: Arc<AtomicBool>,
    /// Dossier des données, pour le fichier de travail d'une reprise (F7).
    dossier: PathBuf,
    /// Accès à distance par le relais (F8).
    distance: distance::Distance,
    /// Version du protocole parlée ([`Config::version_protocole`]).
    version: u32,
}

/// Rend sa place à la connexion quand la session se termine.
struct Place(Arc<AtomicUsize>);

impl Drop for Place {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

pub struct Serveur {
    adresse: SocketAddr,
    empreinte: String,
    base: Base,
    arret: Arc<AtomicBool>,
    ecoute: Option<JoinHandle<()>>,
    decouverte: Option<JoinHandle<()>>,
    /// Sauvegarde quotidienne (F7).
    planification: Option<JoinHandle<()>>,
    /// Connexion de contrôle vers le relais (F8).
    relais: Option<JoinHandle<()>>,
    dossier: PathBuf,
    avertissement: Option<String>,
    /// Verrou du dossier : rendu quand le serveur est détruit, après l'arrêt
    /// de ses fils (`Drop`).
    _verrou: fs::File,
}

impl Serveur {
    /// Prépare le dossier, ouvre la base, puis écoute ; rend la main aussitôt.
    pub fn demarrer(config: Config) -> Result<Serveur, ErreurDemarrage> {
        preparer_dossier(&config)?;
        let verrou = prendre_verrou(&config.dossier)?;
        let identite =
            identite(&config.dossier, config.portee).map_err(ErreurDemarrage::Identite)?;
        let tls = config_tls(&identite).map_err(ErreurDemarrage::Identite)?;
        let conn =
            kadenz_stockage::ouvrir_serveur_avec(&config.dossier.join(NOM_BASE), config.portee)
                .map_err(ErreurDemarrage::Base)?;
        let entreprise = synchro::identifiant_entreprise(&conn).map_err(ErreurDemarrage::Base)?;
        let generation = synchro::generation(&conn).map_err(ErreurDemarrage::Base)?;
        let ecoute = ecouter_sur(&config, &conn)?;
        let adresse = ecoute
            .local_addr()
            .map_err(|e| ErreurDemarrage::Ecoute(e.to_string()))?;
        // Un port UDP déjà pris n'empêche pas le démarrage (décision 44) : les
        // postes saisissent alors l'adresse, ou collent le code complet.
        let mut avertissement = None;
        let udp = if config.decouverte {
            match UdpSocket::bind(adresse) {
                Ok(socket) => Some(socket),
                Err(e) => {
                    avertissement = Some(format!(
                        "découverte sur {adresse} impossible : {e} ; \
                         les postes doivent saisir l'adresse du serveur"
                    ));
                    None
                }
            }
        } else {
            None
        };
        admin::noter_port(&conn, adresse.port()).map_err(ErreurDemarrage::Autre)?;
        let base = Base::lancer(conn);
        let arret = Arc::new(AtomicBool::new(false));
        let contexte = Arc::new(Contexte {
            base: base.clone(),
            tls,
            delai: config.delai_lecture,
            ouverture: config.delai_ouverture,
            entreprise,
            generation,
            essais: Mutex::new(Essais::default()),
            inscriptions: Mutex::new(Inscriptions::default()),
            cles: config.cles_portail.clone().into(),
            arret: arret.clone(),
            dossier: config.dossier.clone(),
            distance: distance::Distance::default(),
            version: config.version_protocole,
        });
        contexte.distance.rafraichir(&contexte.base);
        let relais = {
            let contexte = contexte.clone();
            let identite = identite.clone();
            thread::Builder::new()
                .name("kadenz-relais".into())
                .spawn(move || distance::veiller(contexte, identite))
                .ok()
        };
        let fil = {
            let arret = arret.clone();
            let max = config.connexions_max;
            thread::spawn(move || ecouter(ecoute, contexte, arret, max))
        };
        let decouverte = udp.map(|socket| {
            let arret = arret.clone();
            thread::spawn(move || repondre_decouverte(socket, adresse.port(), arret))
        });
        let planification = {
            let arret = arret.clone();
            let base = base.clone();
            let dossier = config.dossier.clone();
            thread::Builder::new()
                .name("kadenz-sauvegardes".into())
                .spawn(move || planifier(base, dossier, arret))
                .ok()
        };
        Ok(Serveur {
            adresse,
            empreinte: identite.empreinte(),
            base,
            arret,
            ecoute: Some(fil),
            decouverte,
            planification,
            relais,
            dossier: config.dossier,
            avertissement,
            _verrou: verrou,
        })
    }

    /// Problème non bloquant du démarrage (découverte impossible), à noter
    /// dans la trace du service.
    pub fn avertissement(&self) -> Option<&str> {
        self.avertissement.as_deref()
    }

    /// Adresse d'écoute effective (port choisi par le système si 0).
    pub fn adresse(&self) -> SocketAddr {
        self.adresse
    }

    /// Empreinte du certificat du serveur.
    pub fn empreinte(&self) -> &str {
        &self.empreinte
    }

    /// Exécute une tâche sur la base, dans le fil qui applique les opérations.
    pub fn base<R: Send + 'static>(
        &self,
        tache: impl FnOnce(&mut Connection) -> R + Send + 'static,
    ) -> Result<R, String> {
        self.base.executer(tache)
    }

    /// Cesse d'écouter. Les sessions ouvertes finissent à leur requête
    /// suivante, ou au plus tard au bout du délai de lecture.
    pub fn arreter(mut self) {
        self.fermer();
    }

    fn fermer(&mut self) {
        let Some(fil) = self.ecoute.take() else {
            return;
        };
        self.arret.store(true, Ordering::SeqCst);
        // Réveille l'attente de connexion.
        let mut reveil = self.adresse;
        if reveil.ip().is_unspecified() {
            reveil.set_ip([127, 0, 0, 1].into());
        }
        let _ = TcpStream::connect_timeout(&reveil, Duration::from_secs(1));
        let _ = fil.join();
        if let Some(fil) = self.decouverte.take() {
            let _ = fil.join();
        }
        if let Some(fil) = self.planification.take() {
            let _ = fil.join();
        }
        if let Some(fil) = self.relais.take() {
            let _ = fil.join();
        }
        // Sauvegarde à l'arrêt (décision 35), après la dernière session.
        if sauvegardes::lire_reglage(&self.dossier)
            .mot_de_passe
            .is_some()
        {
            sauvegarder_et_noter(&self.base, &self.dossier, sauvegarde::Genre::Arret);
        }
    }
}

/// Sauvegarde automatique sur le fil de la base, notée dans la trace du
/// service ; rend `false` sur un échec.
fn sauvegarder_et_noter(base: &Base, dossier: &Path, genre: sauvegarde::Genre) -> bool {
    let quand = match genre {
        sauvegarde::Genre::Arret => "à l'arrêt",
        _ => "quotidienne",
    };
    let donnees = dossier.to_path_buf();
    let resultat = base
        .executer(move |c| sauvegardes::sauvegarder(c, &donnees, genre))
        .and_then(|r| r);
    match resultat {
        Ok(chemin) => {
            admin::noter_trace(
                dossier,
                &format!("sauvegarde {quand} : {}", chemin.display()),
            );
            true
        }
        Err(e) => {
            admin::noter_trace(dossier, &format!("sauvegarde {quand} impossible : {e}"));
            false
        }
    }
}

/// Fait la sauvegarde quotidienne quand elle est due ; après un échec, attend
/// une heure avant de réessayer.
fn planifier(base: Base, dossier: PathBuf, arret: Arc<AtomicBool>) {
    let mut prochain = Instant::now();
    while !arret.load(Ordering::SeqCst) {
        if Instant::now() >= prochain {
            prochain = Instant::now() + sauvegardes::EXAMEN;
            if sauvegardes::quotidienne_due(&dossier)
                && !sauvegarder_et_noter(&base, &dossier, sauvegarde::Genre::Automatique)
            {
                prochain = Instant::now() + sauvegardes::APRES_ECHEC;
            }
        }
        thread::sleep(Duration::from_millis(250));
    }
}

// Le serveur se déplace dans un autre fil (fermeture du monoposte, faite hors
// du fil de l'interface) : une régression ne passe pas sans bruit.
const _: fn() = || {
    fn envoyable<T: Send>() {}
    envoyable::<Serveur>();
    envoyable::<ErreurDemarrage>();
};

impl Drop for Serveur {
    fn drop(&mut self) {
        self.fermer();
    }
}

/// Répond aux requêtes de découverte du réseau local avec le nom de la
/// machine et le port des sessions ; rien de secret, aucune confiance donnée.
fn repondre_decouverte(socket: UdpSocket, port: u16, arret: Arc<AtomicBool>) {
    let _ = socket.set_read_timeout(Some(Duration::from_millis(300)));
    let reponse = serde_json::to_vec(&ServeurTrouve {
        machine: std::env::var("COMPUTERNAME").unwrap_or_default(),
        port,
    })
    .unwrap_or_default();
    let mut tampon = [0u8; 64];
    while !arret.load(Ordering::SeqCst) {
        if let Ok((n, source)) = socket.recv_from(&mut tampon)
            && &tampon[..n] == DECOUVERTE
        {
            let _ = socket.send_to(&reponse, source);
        }
    }
}

fn ecouter(ecoute: TcpListener, contexte: Arc<Contexte>, arret: Arc<AtomicBool>, max: usize) {
    let actives = Arc::new(AtomicUsize::new(0));
    for flux in ecoute.incoming() {
        if arret.load(Ordering::SeqCst) {
            break;
        }
        let tcp = match flux {
            Ok(tcp) => tcp,
            Err(_) => {
                // Ressources épuisées : on n'insiste pas en boucle.
                thread::sleep(Duration::from_millis(50));
                continue;
            }
        };
        if actives.fetch_add(1, Ordering::SeqCst) >= max {
            actives.fetch_sub(1, Ordering::SeqCst);
            continue;
        }
        let place = Place(actives.clone());
        let contexte = contexte.clone();
        thread::spawn(move || {
            let _place = place;
            session(&contexte, tcp, false);
        });
    }
}

/// Connexion TCP dont chaque lecture et écriture attend au plus `delai`, et
/// jamais au-delà de `fin` quand elle est fixée.
struct Echeance {
    tcp: TcpStream,
    delai: Duration,
    fin: Option<Instant>,
}

impl Echeance {
    fn borner(&self, ecriture: bool) -> io::Result<()> {
        let mut delai = self.delai;
        if let Some(fin) = self.fin {
            let reste = fin.saturating_duration_since(Instant::now());
            if reste.is_zero() {
                return Err(io::Error::new(io::ErrorKind::TimedOut, "délai dépassé"));
            }
            delai = delai.min(reste);
        }
        if ecriture {
            self.tcp.set_write_timeout(Some(delai))
        } else {
            self.tcp.set_read_timeout(Some(delai))
        }
    }
}

impl Read for Echeance {
    fn read(&mut self, tampon: &mut [u8]) -> io::Result<usize> {
        self.borner(false)?;
        self.tcp.read(tampon)
    }
}

impl Write for Echeance {
    fn write(&mut self, octets: &[u8]) -> io::Result<usize> {
        self.borner(true)?;
        self.tcp.write(octets)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.tcp.flush()
    }
}

type Flux = StreamOwned<ServerConnection, Echeance>;

fn repondre(flux: &mut Flux, reponse: &Reponse) -> io::Result<()> {
    messages::ecrire(flux, reponse)
}

/// Termine la session. Ce que le poste a déjà envoyé est lu et jeté avant de
/// fermer (une seconde, 1 Mio au plus) : sinon Windows coupe la connexion et
/// le poste peut perdre la dernière réponse.
fn fermer(flux: &mut Flux) {
    flux.conn.send_close_notify();
    let _ = flux.flush();
    let _ = flux.sock.tcp.shutdown(Shutdown::Write);
    flux.sock.fin = Some(Instant::now() + Duration::from_secs(1));
    let mut tampon = [0u8; 4096];
    let mut total = 0;
    while total < 1 << 20 {
        match flux.sock.read(&mut tampon) {
            Ok(0) | Err(_) => break,
            Ok(n) => total += n,
        }
    }
}

/// Lit une requête ; une trame trop grande ou illisible reçoit un rejet et
/// met fin à la session.
fn lire(flux: &mut Flux) -> Option<Requete> {
    match messages::lire::<Requete>(flux) {
        Ok(requete) => Some(requete),
        Err(ErreurTrame::Io(_)) => None,
        Err(e) => {
            let _ = repondre(
                flux,
                &Reponse::Rejet {
                    motif: e.to_string(),
                },
            );
            fermer(flux);
            None
        }
    }
}

/// Session d'un poste ; `distante` : connexion ouverte par le serveur à la
/// demande du relais (F8). Le serveur ne croit pas le relais : le poste doit
/// être inscrit et autorisé à distance, l'accès actif, et l'inscription est
/// refusée (réservée au réseau local).
fn session(contexte: &Contexte, tcp: TcpStream, distante: bool) {
    if tcp.set_nodelay(true).is_err() {
        return;
    }
    let ip = tcp
        .peer_addr()
        .map(|a| a.ip().to_string())
        .unwrap_or_default();
    let Ok(connexion) = ServerConnection::new(contexte.tls.clone()) else {
        return;
    };
    let sock = Echeance {
        tcp,
        delai: contexte.delai,
        fin: Some(Instant::now() + contexte.ouverture),
    };
    let mut flux = StreamOwned::new(connexion, sock);
    // Poignée de main, puis inscription du poste vérifiée avant de lire quoi
    // que ce soit de lui.
    while flux.conn.is_handshaking() {
        if flux.conn.complete_io(&mut flux.sock).is_err() {
            return;
        }
    }
    let Some(poste) = flux
        .conn
        .peer_certificates()
        .and_then(|c| c.first())
        .map(|c| empreinte(c))
    else {
        return;
    };
    if distante && !contexte.distance.autorise(&poste) {
        return fermer(&mut flux);
    }
    let p = poste.clone();
    match contexte.base.executer(move |c| postes::est_inscrit(c, &p)) {
        Ok(Ok(true)) => {}
        Ok(Ok(false)) if !distante => return inscription(contexte, &mut flux, &poste, &ip),
        _ => return fermer(&mut flux),
    }
    let Some(premiere) = lire(&mut flux) else {
        return;
    };
    let accueil = match premiere {
        Requete::Bonjour { version } if version == contexte.version => Reponse::Bienvenue {
            version: contexte.version,
            entreprise: contexte.entreprise.clone(),
            decalage_minutes: chrono::Local::now().offset().local_minus_utc() / 60,
            generation: contexte.generation.clone(),
            relais: contexte.distance.relais_pour(&poste),
            logiciel: kadenz_protocole::logiciel(),
        },
        Requete::Bonjour { .. } => {
            let _ = repondre(&mut flux, &version_refusee(contexte));
            return fermer(&mut flux);
        }
        _ => {
            let _ = repondre(&mut flux, &rejet("Bonjour attendu en premier"));
            return fermer(&mut flux);
        }
    };
    if repondre(&mut flux, &accueil).is_err() {
        return;
    }
    if distante {
        contexte.distance.noter_session(&contexte.base);
    }
    // Session ouverte : seul le délai de chaque lecture compte désormais.
    flux.sock.fin = None;
    let mut compte: Option<String> = None;
    while let Some(requete) = lire(&mut flux) {
        // Serveur arrêté : la session finit sans servir la requête, pour
        // qu'un poste qui relève sans cesse voie bien la coupure.
        if contexte.arret.load(Ordering::SeqCst) {
            return fermer(&mut flux);
        }
        // Poste décoché ou accès désactivé : la session distante finit.
        if distante && !contexte.distance.autorise(&poste) {
            return fermer(&mut flux);
        }
        let reponse = traiter(contexte, &poste, &mut compte, requete);
        if repondre(&mut flux, &reponse).is_err() {
            return;
        }
    }
}

/// Refus d'un poste d'une autre version du protocole, avec la version
/// logicielle du serveur (F10) : le poste sait s'il doit attendre la mise à
/// jour du serveur ou se mettre à jour lui-même.
fn version_refusee(contexte: &Contexte) -> Reponse {
    Reponse::VersionRefusee {
        version: contexte.version,
        logiciel: kadenz_protocole::logiciel(),
    }
}

/// Poste pas encore inscrit : une seule trame, courte, est lue ; seule une
/// demande d'inscription par code est servie, puis la connexion est fermée.
fn inscription(contexte: &Contexte, flux: &mut Flux, poste: &str, ip: &str) {
    let reponse = match messages::lire_borne::<Requete>(flux, TAILLE_INSCRIPTION) {
        Ok(Requete::Inscription { version, .. }) if version != contexte.version => {
            version_refusee(contexte)
        }
        Ok(Requete::Inscription { secret, .. }) => inscrire(contexte, poste, ip, secret),
        Ok(_) | Err(ErreurTrame::TropGrand(_)) | Err(ErreurTrame::Illisible(_)) => {
            Reponse::PosteNonInscrit
        }
        Err(ErreurTrame::Io(_)) => return,
    };
    let _ = repondre(flux, &reponse);
    fermer(flux);
}

fn refus(m: MessageCompose) -> Reponse {
    Reponse::Refus {
        refus: Refus {
            modele: m.modele.into(),
            valeurs: m.valeurs,
        },
    }
}

fn inscrire(contexte: &Contexte, poste: &str, ip: &str, secret: String) -> Reponse {
    let cles = [format!("ip:{ip}")];
    let verrou = || {
        contexte
            .inscriptions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    };
    {
        let mut inscriptions = verrou();
        let maintenant = Instant::now();
        let attente = inscriptions.attente(&cles, maintenant);
        if attente > 0 {
            return refus(MessageCompose::new(
                "trop d'essais d'inscription : nouvel essai dans {} s",
                &[&attente],
            ));
        }
        inscriptions.reserver(&cles, maintenant);
    }
    let poste = poste.to_string();
    let cles_portail = contexte.cles.clone();
    let resultat = contexte.base.executer(move |c| {
        let a = abonnement::appliquer_avec(c, abonnement::horloge(), &cles_portail)?;
        inscription::consommer(
            c,
            &secret,
            &poste,
            abonnement::postes_permis(&a),
            chrono::Utc::now(),
        )
    });
    match resultat {
        Ok(Ok(Ok(nom))) => {
            verrou().reussite(&cles);
            Reponse::Inscrit {
                entreprise: contexte.entreprise.clone(),
                nom,
            }
        }
        Ok(Ok(Err(m))) => refus(m),
        Ok(Err(e)) => rejet(&e.to_string()),
        Err(e) => rejet(&e),
    }
}

fn rejet(motif: &str) -> Reponse {
    Reponse::Rejet {
        motif: motif.into(),
    }
}

fn traiter(
    contexte: &Contexte,
    poste: &str,
    compte: &mut Option<String>,
    requete: Requete,
) -> Reponse {
    match requete {
        Requete::Bonjour { .. } => rejet("Bonjour déjà reçu"),
        Requete::Inscription { .. } => rejet("poste déjà inscrit"),
        Requete::Connexion {
            identifiant,
            mot_de_passe,
        } => connexion(contexte, poste, compte, identifiant, mot_de_passe),
        Requete::Operation { operation } => {
            let Some(uid) = compte.as_deref() else {
                return rejet("aucun compte connecté");
            };
            if operation.poste != poste {
                return rejet("le poste de l'opération n'est pas celui du certificat");
            }
            // Écriture hors connexion d'un compte qui ne peut plus se
            // connecter, apportée par la session d'un autre (décision 52) :
            // toujours mise en attente pour l'Admin.
            let relayee = operation.compte != uid;
            if relayee && !operation.hors_connexion {
                return rejet("le compte de l'opération n'est pas celui de la session");
            }
            let cles = contexte.cles.clone();
            match contexte.base.executer(move |c| {
                // L'abonnement est réévalué à chaque opération : le jour change.
                abonnement::appliquer_avec(c, abonnement::horloge(), &cles)?;
                if relayee {
                    synchro::appliquer_relayee(c, &operation)
                } else {
                    synchro::appliquer(c, &operation)
                }
            }) {
                Ok(Ok(issue)) => Reponse::Resultat { issue },
                Ok(Err(Erreur::LectureSeule { echeance })) => Reponse::LectureSeule { echeance },
                Ok(Err(e)) => rejet(&e.to_string()),
                Err(e) => rejet(&e),
            }
        }
        Requete::Changements { depuis, limite } => {
            if compte.is_none() {
                return rejet("aucun compte connecté");
            }
            let limite = limite.clamp(1, CHANGEMENTS_MAX);
            match contexte.base.executer(move |c| {
                Ok::<_, Erreur>((
                    synchro::changements_depuis(c, depuis, limite)?,
                    synchro::dernier_changement(c)?,
                ))
            }) {
                Ok(Ok((changements, dernier))) => Reponse::Changements {
                    changements,
                    dernier,
                },
                Ok(Err(e)) => rejet(&e.to_string()),
                Err(e) => rejet(&e),
            }
        }
        Requete::Abonnement => {
            if compte.is_none() {
                return rejet("aucun compte connecté");
            }
            let cles = contexte.cles.clone();
            reponse_abonnement(
                contexte
                    .base
                    .executer(move |c| abonnement::appliquer_avec(c, abonnement::horloge(), &cles)),
            )
        }
        Requete::RenouvelerAbonnement { cle } => {
            let Some(uid) = compte.clone() else {
                return rejet("aucun compte connecté");
            };
            let cles = contexte.cles.clone();
            let reponse = reponse_abonnement(contexte.base.executer(move |c| {
                let id = synchro::id_de(c, Entite::Utilisateur, &uid)?
                    .ok_or_else(|| Erreur::Validation("compte inconnu sur le serveur".into()))?;
                let acteur = comptes::lire(c, id)?
                    .ok_or_else(|| Erreur::Validation("compte inconnu sur le serveur".into()))?;
                abonnement::enregistrer_avec(c, &acteur, &cle, abonnement::horloge(), &cles)
            }));
            // Nouvelle licence : le relais la reçoit sans attendre la fin
            // d'une attente de reconnexion (F8).
            if matches!(reponse, Reponse::Abonnement { .. }) {
                contexte.distance.changer(&contexte.base);
            }
            reponse
        }
        Requete::Suspendues => {
            let Some(uid) = compte.clone() else {
                return rejet("aucun compte connecté");
            };
            match contexte.base.executer(move |c| {
                let id = synchro::id_de(c, Entite::Utilisateur, &uid)?
                    .ok_or_else(|| Erreur::Validation("compte inconnu sur le serveur".into()))?;
                let acteur = comptes::lire(c, id)?
                    .ok_or_else(|| Erreur::Validation("compte inconnu sur le serveur".into()))?;
                synchro::suspendues(c, &acteur)
            }) {
                Ok(Ok(suspendues)) => Reponse::Suspendues { suspendues },
                Ok(Err(e)) => rejet(&e.to_string()),
                Err(e) => rejet(&e),
            }
        }
        Requete::Lignes { lignes } => {
            if compte.is_none() {
                return rejet("aucun compte connecté");
            }
            if lignes.len() > LIGNES_MAX {
                return rejet("trop de lignes demandées");
            }
            match contexte.base.executer(move |c| synchro::lignes(c, &lignes)) {
                Ok(Ok(lignes)) => Reponse::Lignes { lignes },
                Ok(Err(e)) => rejet(&e.to_string()),
                Err(e) => rejet(&e),
            }
        }
        Requete::ReprisePossible => {
            if compte.is_none() {
                return rejet("aucun compte connecté");
            }
            match contexte.base.executer(|c| reprise::serveur_neuf(c)) {
                Ok(Ok(possible)) => Reponse::ReprisePossible { possible },
                Ok(Err(e)) => rejet(&e.to_string()),
                Err(e) => rejet(&e),
            }
        }
        Requete::Reprise {
            sauvegarde,
            mot_de_passe,
        } => {
            let Some(uid) = compte.clone() else {
                return rejet("aucun compte connecté");
            };
            let Some(octets) = kadenz_protocole::reprise::decoder(&sauvegarde) else {
                return rejet("sauvegarde illisible ou trop grande");
            };
            drop(sauvegarde);
            let cles = contexte.cles.clone();
            let fichier = contexte
                .dossier
                .join(format!("reprise-{}.kzs", synchro::nouvel_uid()));
            reponse_reprise(contexte.base.executer(move |c| {
                let id = synchro::id_de(c, Entite::Utilisateur, &uid)?
                    .ok_or_else(|| Erreur::Validation("compte inconnu sur le serveur".into()))?;
                let acteur = comptes::lire(c, id)?
                    .ok_or_else(|| Erreur::Validation("compte inconnu sur le serveur".into()))?;
                abonnement::appliquer_avec(c, abonnement::horloge(), &cles)?;
                fs::write(&fichier, &octets)?;
                let resultat = reprise::ouvrir_pour_reprise(&fichier, &mot_de_passe)
                    .and_then(|source| reprise::reprendre(c, &acteur, &source));
                let _ = fs::remove_file(&fichier);
                resultat
            }))
        }
        Requete::AccesDistant => acces_distant(contexte, compte, None),
        Requete::ReglerAccesDistant { actif } => {
            acces_distant(contexte, compte, Some(ReglageDistant::Actif(actif)))
        }
        Requete::AutoriserDistance { poste, autorise } => acces_distant(
            contexte,
            compte,
            Some(ReglageDistant::Poste(poste, autorise)),
        ),
        Requete::Sauvegarde => {
            let Some(uid) = compte.clone() else {
                return rejet("aucun compte connecté");
            };
            let admin = contexte.base.executer(move |c| {
                let id = synchro::id_de(c, Entite::Utilisateur, &uid)?;
                Ok::<_, Erreur>(match id {
                    Some(id) => comptes::lire(c, id)?.is_some_and(|u| u.role == Role::Admin),
                    None => false,
                })
            });
            match admin {
                Ok(Ok(admin)) => Reponse::Sauvegarde {
                    avertissement: admin
                        .then(|| sauvegardes::avertissement(&contexte.dossier))
                        .flatten(),
                },
                Ok(Err(e)) => rejet(&e.to_string()),
                Err(e) => rejet(&e),
            }
        }
    }
}

enum ReglageDistant {
    Actif(bool),
    Poste(String, bool),
}

/// Rubrique « Accès à distance » (F8), compte Admin seul : réglage
/// éventuel, puis état à jour. Permis en lecture seule : c'est un réglage du
/// serveur, pas une opération sur les données.
fn acces_distant(
    contexte: &Contexte,
    compte: &Option<String>,
    reglage: Option<ReglageDistant>,
) -> Reponse {
    let Some(uid) = compte.clone() else {
        return rejet("aucun compte connecté");
    };
    let change = reglage.is_some();
    let resultat = contexte.base.executer(move |c| {
        let inconnu = || Erreur::Validation("compte inconnu sur le serveur".into());
        let id = synchro::id_de(c, Entite::Utilisateur, &uid)?.ok_or_else(inconnu)?;
        let acteur = comptes::lire(c, id)?.ok_or_else(inconnu)?;
        if acteur.role != Role::Admin {
            return Err(Erreur::Interdit(Droit::GererParametres));
        }
        match reglage {
            Some(ReglageDistant::Actif(actif)) => {
                kadenz_stockage::distance::activer(c, actif, Some(id))?
            }
            Some(ReglageDistant::Poste(poste, autorise)) => {
                kadenz_stockage::distance::autoriser(c, &poste, autorise, Some(id))?
            }
            None => {}
        }
        distance::lire_etat(c)
    });
    if change && matches!(resultat, Ok(Ok(_))) {
        contexte.distance.changer(&contexte.base);
    }
    match resultat {
        Ok(Ok(lu)) => Reponse::AccesDistant {
            etat: Box::new(contexte.distance.etat(lu)),
        },
        Ok(Err(Erreur::Interdit(droit))) => Reponse::Refus {
            refus: Refus {
                modele: synchro::MODELE_INTERDIT.into(),
                valeurs: vec![droit.to_string()],
            },
        },
        Ok(Err(Erreur::Validation(modele))) => Reponse::Refus {
            refus: Refus {
                modele,
                valeurs: Vec::new(),
            },
        },
        Ok(Err(e)) => rejet(&e.to_string()),
        Err(e) => rejet(&e),
    }
}

/// Bilan d'une reprise, ou refus à traduire (serveur pas neuf, mot de passe
/// de la sauvegarde, droit manquant, lecture seule).
fn reponse_reprise(
    resultat: Result<kadenz_stockage::Resultat<kadenz_protocole::reprise::ResumeReprise>, String>,
) -> Reponse {
    match resultat {
        Ok(Ok(bilan)) => Reponse::Reprise {
            bilan: Box::new(bilan),
        },
        Ok(Err(Erreur::LectureSeule { echeance })) => Reponse::LectureSeule { echeance },
        Ok(Err(Erreur::Validation(modele))) => Reponse::Refus {
            refus: Refus {
                modele,
                valeurs: Vec::new(),
            },
        },
        Ok(Err(Erreur::ValidationComposee { modele, valeurs })) => Reponse::Refus {
            refus: Refus { modele, valeurs },
        },
        Ok(Err(Erreur::Interdit(droit))) => Reponse::Refus {
            refus: Refus {
                modele: synchro::MODELE_INTERDIT.into(),
                valeurs: vec![droit.to_string()],
            },
        },
        Ok(Err(e)) => rejet(&e.to_string()),
        Err(e) => rejet(&e),
    }
}

/// État de l'abonnement, ou refus à traduire (licence refusée, droit manquant).
fn reponse_abonnement(
    resultat: Result<kadenz_stockage::Resultat<kadenz_protocole::Abonnement>, String>,
) -> Reponse {
    match resultat {
        Ok(Ok(a)) => Reponse::Abonnement {
            abonnement: Box::new(a),
        },
        Ok(Err(Erreur::ValidationComposee { modele, valeurs })) => Reponse::Refus {
            refus: Refus { modele, valeurs },
        },
        Ok(Err(Erreur::Interdit(droit))) => Reponse::Refus {
            refus: Refus {
                modele: synchro::MODELE_INTERDIT.into(),
                valeurs: vec![droit.to_string()],
            },
        },
        Ok(Err(e)) => rejet(&e.to_string()),
        Err(e) => rejet(&e),
    }
}

fn connexion(
    contexte: &Contexte,
    poste: &str,
    compte: &mut Option<String>,
    identifiant: String,
    mot_de_passe: String,
) -> Reponse {
    // Échecs comptés par poste, tous identifiants confondus : un poste qui
    // essaie des mots de passe est freiné, sans pouvoir bloquer un compte
    // utilisé depuis un autre poste.
    let cles = [format!("poste:{poste}")];
    let verrou = || contexte.essais.lock().unwrap_or_else(|e| e.into_inner());
    // L'essai compte comme un échec dès maintenant, sous le verrou, et il est
    // effacé si le mot de passe est bon : des sessions parallèles du même
    // poste n'obtiennent pas plus d'essais.
    let attente_si_echec = {
        let mut essais = verrou();
        let maintenant = Instant::now();
        let attente = essais.attente(&cles, maintenant);
        if attente > 0 {
            return Reponse::ConnexionRefusee {
                attente_secondes: attente,
            };
        }
        essais.echec(&cles, maintenant)
    };
    let resultat = contexte.base.executer(move |c| {
        let utilisateur = comptes::connecter(c, &identifiant, &mot_de_passe)?;
        let uid = match synchro::uid_de(c, Entite::Utilisateur, utilisateur.id)? {
            Some(uid) => Some(uid),
            None => {
                synchro::completer_uids(c)?;
                synchro::uid_de(c, Entite::Utilisateur, utilisateur.id)?
            }
        };
        Ok::<_, Erreur>((uid, utilisateur.nom_complet))
    });
    match resultat {
        Ok(Ok((Some(uid), nom_complet))) => {
            verrou().reussite(&cles);
            *compte = Some(uid.clone());
            Reponse::Connecte {
                compte: uid,
                nom_complet,
            }
        }
        Ok(Err(Erreur::ConnexionRefusee)) => {
            *compte = None;
            Reponse::ConnexionRefusee {
                attente_secondes: attente_si_echec,
            }
        }
        Ok(Ok((None, _))) => rejet("compte sans identifiant global"),
        Ok(Err(e)) => rejet(&e.to_string()),
        Err(e) => rejet(&e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attente_croissante_apres_des_echecs() {
        assert_eq!(duree_attente(1, ECHECS_LIBRES), 0);
        assert_eq!(duree_attente(2, ECHECS_LIBRES), 0);
        assert_eq!(duree_attente(3, ECHECS_LIBRES), 5);
        assert_eq!(duree_attente(4, ECHECS_LIBRES), 10);
        assert_eq!(duree_attente(30, ECHECS_LIBRES), ATTENTE_MAX);
        let mut essais = Essais::default();
        let t = Instant::now();
        let poste_a = ["poste:a".to_string()];
        assert_eq!(essais.echec(&poste_a, t), 0);
        assert_eq!(essais.echec(&poste_a, t), 0);
        assert_eq!(essais.echec(&poste_a, t), 5);
        // Un autre poste n'attend pas.
        assert_eq!(essais.attente(&["poste:b".to_string()], t), 0);
        assert_eq!(essais.attente(&poste_a, t + Duration::from_secs(5)), 0);
        essais.reussite(&poste_a);
        assert_eq!(essais.attente(&poste_a, t), 0);
    }
}
