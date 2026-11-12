//! Serveur Kadenz de l'entreprise (phase 2, jalon F2) : base de référence,
//! sessions TLS 1.3 avec les postes inscrits, un seul fil d'accès à la base.
//!
//! Spécification de synchronisation, section 7 : le poste est reconnu à
//! l'empreinte de son certificat (table `poste_inscrit`), le compte par son
//! mot de passe ; une opération n'est acceptée que si son poste est celui du
//! certificat et son compte celui de la session. Limites : taille des
//! messages, délai de lecture par connexion, nombre de connexions
//! simultanées, attente croissante après des échecs de connexion.

pub mod droits;

use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use kadenz_client::{Identite, NOM_TLS, empreinte, fournisseur};
use kadenz_protocole::messages::{self, CHANGEMENTS_MAX, ErreurTrame, Reponse, Requete};
use kadenz_protocole::{Entite, VERSION};
use kadenz_stockage::chiffrement::{self, Portee};
use kadenz_stockage::{Erreur, comptes, postes, synchro};
use rusqlite::Connection;
use rustls::client::danger::HandshakeSignatureValid;
use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, UnixTime};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::{
    DigitallySignedStruct, DistinguishedName, ServerConfig, ServerConnection, SignatureScheme,
    StreamOwned,
};

/// Port par défaut, dans une plage non attribuée du registre de l'IANA
/// (47101 à 47556).
pub const PORT_PAR_DEFAUT: u16 = 47380;

/// Base de référence, dans le dossier des données ; sa clé est à côté
/// (`kadenz-serveur.cle`), protégée par Windows au niveau de la machine.
pub const NOM_BASE: &str = "kadenz-serveur.sqlite";
/// Certificat TLS du serveur (DER) ; son empreinte est épinglée par les postes.
pub const NOM_CERTIFICAT: &str = "serveur.crt";
/// Clé privée TLS, protégée par Windows au niveau de la machine.
const NOM_CLE_TLS: &str = "serveur.cle-tls";

/// Échecs de connexion permis avant la première attente.
const ECHECS_LIBRES: u32 = 3;
/// Première attente imposée, doublée à chaque nouvel échec.
const ATTENTE_BASE: u64 = 5;
/// Attente maximale après des échecs de connexion.
const ATTENTE_MAX: u64 = 900;

#[derive(Debug, Clone)]
pub struct Config {
    pub dossier: PathBuf,
    pub adresse: SocketAddr,
    /// Connexions simultanées ; au-delà, une connexion est fermée aussitôt.
    pub connexions_max: usize,
    /// Délai d'attente de chaque lecture et écriture sur une connexion.
    pub delai_lecture: Duration,
    /// Droits du dossier posés à sa création et vérifiés au démarrage. Coupé
    /// seulement pour les essais, sous un compte sans droits d'administrateur.
    pub controle_droits: bool,
}

impl Config {
    pub fn nouvelle(dossier: PathBuf) -> Config {
        Config {
            dossier,
            adresse: SocketAddr::from(([0, 0, 0, 0], PORT_PAR_DEFAUT)),
            connexions_max: 64,
            delai_lecture: Duration::from_secs(60),
            controle_droits: true,
        }
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

fn preparer_dossier(config: &Config) -> Result<(), String> {
    let nouveau = !config.dossier.exists();
    fs::create_dir_all(&config.dossier)
        .map_err(|e| format!("dossier {} non créé : {e}", config.dossier.display()))?;
    if config.controle_droits {
        if nouveau {
            droits::restreindre(&config.dossier)?;
        }
        let en_trop = droits::verifier(&config.dossier)?;
        if !en_trop.is_empty() {
            return Err(format!(
                "le dossier {} est accessible à d'autres comptes que SYSTEM et les \
                 Administrateurs ({}) : démarrage refusé",
                config.dossier.display(),
                en_trop.join(", ")
            ));
        }
    }
    Ok(())
}

/// Identité TLS du serveur, créée au premier démarrage. Tant que le
/// certificat n'est pas écrit, son empreinte n'a pu être donnée à aucun poste :
/// une clé seule est remplacée.
fn identite(dossier: &Path) -> Result<Identite, String> {
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
    let protegee = chiffrement::proteger(&nouvelle.cle, Portee::Machine)
        .ok_or("clé TLS du serveur non protégée")?;
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

/// Échecs de connexion, comptés par poste et par identifiant.
#[derive(Default)]
struct Essais(HashMap<String, (u32, Instant)>);

fn duree_attente(echecs: u32) -> u64 {
    if echecs < ECHECS_LIBRES {
        0
    } else {
        (ATTENTE_BASE << (echecs - ECHECS_LIBRES).min(10)).min(ATTENTE_MAX)
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
        for cle in cles {
            let entree = self.0.entry(cle.clone()).or_insert((0, maintenant));
            entree.0 += 1;
            entree.1 = maintenant + Duration::from_secs(duree_attente(entree.0));
        }
        self.attente(cles, maintenant)
    }

    fn reussite(&mut self, cles: &[String]) {
        for cle in cles {
            self.0.remove(cle);
        }
    }
}

struct Contexte {
    base: Base,
    tls: Arc<ServerConfig>,
    delai: Duration,
    entreprise: String,
    essais: Mutex<Essais>,
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
}

impl Serveur {
    /// Prépare le dossier, ouvre la base, puis écoute ; rend la main aussitôt.
    pub fn demarrer(config: Config) -> Result<Serveur, String> {
        preparer_dossier(&config)?;
        let identite = identite(&config.dossier)?;
        let tls = config_tls(&identite)?;
        let conn = kadenz_stockage::ouvrir_serveur(&config.dossier.join(NOM_BASE))
            .map_err(|e| format!("base du serveur : {e}"))?;
        let entreprise = synchro::identifiant_entreprise(&conn).map_err(|e| e.to_string())?;
        let base = Base::lancer(conn);
        let ecoute = TcpListener::bind(config.adresse)
            .map_err(|e| format!("écoute sur {} impossible : {e}", config.adresse))?;
        let adresse = ecoute.local_addr().map_err(|e| e.to_string())?;
        let contexte = Arc::new(Contexte {
            base: base.clone(),
            tls,
            delai: config.delai_lecture,
            entreprise,
            essais: Mutex::new(Essais::default()),
        });
        let arret = Arc::new(AtomicBool::new(false));
        let fil = {
            let arret = arret.clone();
            let max = config.connexions_max;
            thread::spawn(move || ecouter(ecoute, contexte, arret, max))
        };
        Ok(Serveur {
            adresse,
            empreinte: identite.empreinte(),
            base,
            arret,
            ecoute: Some(fil),
        })
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

    /// Cesse d'écouter. Les sessions ouvertes finissent d'elles-mêmes, au plus
    /// tard au bout du délai de lecture.
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
    }
}

impl Drop for Serveur {
    fn drop(&mut self) {
        self.fermer();
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
            session(&contexte, tcp);
        });
    }
}

type Flux = StreamOwned<ServerConnection, TcpStream>;

fn repondre(flux: &mut Flux, reponse: &Reponse) -> io::Result<()> {
    messages::ecrire(flux, reponse)
}

fn fermer(flux: &mut Flux) {
    flux.conn.send_close_notify();
    let _ = flux.flush();
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

fn session(contexte: &Contexte, tcp: TcpStream) {
    let preparee = tcp
        .set_read_timeout(Some(contexte.delai))
        .and_then(|_| tcp.set_write_timeout(Some(contexte.delai)))
        .and_then(|_| tcp.set_nodelay(true));
    let Ok(connexion) = ServerConnection::new(contexte.tls.clone()) else {
        return;
    };
    if preparee.is_err() {
        return;
    }
    let mut flux = StreamOwned::new(connexion, tcp);
    // La poignée de main TLS se fait à la première lecture.
    let Some(premiere) = lire(&mut flux) else {
        return;
    };
    let Some(poste) = flux
        .conn
        .peer_certificates()
        .and_then(|c| c.first())
        .map(|c| empreinte(c))
    else {
        return;
    };
    let p = poste.clone();
    match contexte.base.executer(move |c| postes::est_inscrit(c, &p)) {
        Ok(Ok(true)) => {}
        Ok(Ok(false)) => {
            let _ = repondre(&mut flux, &Reponse::PosteNonInscrit);
            return fermer(&mut flux);
        }
        _ => return fermer(&mut flux),
    }
    let accueil = match premiere {
        Requete::Bonjour { version } if version == VERSION => Reponse::Bienvenue {
            version: VERSION,
            entreprise: contexte.entreprise.clone(),
            decalage_minutes: chrono::Local::now().offset().local_minus_utc() / 60,
        },
        Requete::Bonjour { .. } => {
            let _ = repondre(&mut flux, &Reponse::VersionRefusee { version: VERSION });
            return fermer(&mut flux);
        }
        _ => {
            let _ = repondre(
                &mut flux,
                &Reponse::Rejet {
                    motif: "Bonjour attendu en premier".into(),
                },
            );
            return fermer(&mut flux);
        }
    };
    if repondre(&mut flux, &accueil).is_err() {
        return;
    }
    let mut compte: Option<String> = None;
    while let Some(requete) = lire(&mut flux) {
        let reponse = traiter(contexte, &poste, &mut compte, requete);
        if repondre(&mut flux, &reponse).is_err() {
            return;
        }
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
            if operation.compte != uid {
                return rejet("le compte de l'opération n'est pas celui de la session");
            }
            match contexte
                .base
                .executer(move |c| synchro::appliquer(c, &operation))
            {
                Ok(Ok(issue)) => Reponse::Resultat { issue },
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
    }
}

fn connexion(
    contexte: &Contexte,
    poste: &str,
    compte: &mut Option<String>,
    identifiant: String,
    mot_de_passe: String,
) -> Reponse {
    let cles = [
        format!("poste:{poste}"),
        format!("compte:{}", identifiant.trim().to_lowercase()),
    ];
    let essais = |f: &mut dyn FnMut(&mut Essais) -> u64| {
        let mut garde = contexte.essais.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut garde)
    };
    let attente = essais(&mut |e| e.attente(&cles, Instant::now()));
    if attente > 0 {
        return Reponse::ConnexionRefusee {
            attente_secondes: attente,
        };
    }
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
            essais(&mut |e| {
                e.reussite(&cles);
                0
            });
            *compte = Some(uid.clone());
            Reponse::Connecte {
                compte: uid,
                nom_complet,
            }
        }
        Ok(Err(Erreur::ConnexionRefusee)) => {
            *compte = None;
            Reponse::ConnexionRefusee {
                attente_secondes: essais(&mut |e| e.echec(&cles, Instant::now())),
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
        assert_eq!(duree_attente(1), 0);
        assert_eq!(duree_attente(2), 0);
        assert_eq!(duree_attente(3), 5);
        assert_eq!(duree_attente(4), 10);
        assert_eq!(duree_attente(30), ATTENTE_MAX);
        let mut essais = Essais::default();
        let t = Instant::now();
        let poste_a = ["poste:a".to_string(), "compte:x".to_string()];
        assert_eq!(essais.echec(&poste_a, t), 0);
        assert_eq!(essais.echec(&poste_a, t), 0);
        assert_eq!(essais.echec(&poste_a, t), 5);
        // Le même compte depuis un autre poste attend aussi.
        let poste_b = ["poste:b".to_string(), "compte:x".to_string()];
        assert_eq!(essais.attente(&poste_b, t), 5);
        assert_eq!(essais.attente(&poste_a, t + Duration::from_secs(5)), 0);
        essais.reussite(&poste_a);
        assert_eq!(essais.attente(&poste_b, t), 0);
    }
}
