//! Le relais : accepte les connexions de contrôle des serveurs (TLS) et les
//! trames en clair des postes et des connexions de session, sur un seul
//! port ; relie chaque poste autorisé à une connexion ouverte par son
//! serveur, puis copie les octets sans les lire.

use std::collections::{HashMap, HashSet};
use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use ed25519_dalek::VerifyingKey;
use rustls::{ServerConfig, ServerConnection};

use crate::controle::{BATTEMENT, SILENCE};
use crate::preuve::{self, ROLE_POSTE, ROLE_SERVEUR, empreinte};
use crate::tls::{Canal, fournisseur, identite_relais};
use crate::trames::{self, TAILLE_EN_CLAIR, Trame};
use crate::{VERSION, motif};

/// Sessions en attente de mise en relation, par entreprise et en tout.
const ATTENTES_PAR_ENTREPRISE: usize = 16;
const ATTENTES_EN_TOUT: usize = 256;
/// Sessions reliées par entreprise, comme le serveur.
const RELIEES_PAR_ENTREPRISE: usize = 64;
/// Connexions ouvertes en tout, toutes sortes confondues.
const CONNEXIONS_MAX: usize = 4096;
/// Délai des trames d'ouverture (poste, session, enregistrement).
const DELAI_TRAMES: Duration = Duration::from_secs(5);
/// Session reliée sans aucun octet pendant ce délai : fermée.
const INACTIVITE: Duration = Duration::from_secs(600);
/// Traces gardées (décision 59).
const JOURS_DE_TRACES: i64 = 30;

pub struct Config {
    pub adresse: SocketAddr,
    /// Dossier des données : identité TLS du relais.
    pub dossier: PathBuf,
    /// Dossier des traces ; `None` : aucune trace écrite.
    pub traces: Option<PathBuf>,
    /// Clés du portail qui signent les licences d'entreprise.
    pub cles_portail: Vec<VerifyingKey>,
    /// Attente de la connexion du serveur après une demande de session.
    pub attente_serveur: Duration,
    pub battement: Duration,
    pub silence: Duration,
}

impl Config {
    pub fn nouvelle(
        adresse: SocketAddr,
        dossier: PathBuf,
        cles_portail: Vec<VerifyingKey>,
    ) -> Config {
        Config {
            adresse,
            traces: Some(dossier.join("traces")),
            dossier,
            cles_portail,
            attente_serveur: Duration::from_secs(3),
            battement: BATTEMENT,
            silence: SILENCE,
        }
    }
}

/// Une entreprise enregistrée par son serveur.
struct Inscrite {
    entreprise: String,
    /// Empreinte du certificat du serveur.
    serveur: String,
    postes: Mutex<HashSet<String>>,
    demandes: Mutex<mpsc::Sender<String>>,
    /// Connexion de contrôle, pour la couper quand un autre enregistrement
    /// la remplace.
    controle: TcpStream,
    en_attente: AtomicUsize,
    /// Sessions reliées : identifiant, poste, connexions des deux côtés.
    reliees: Mutex<Vec<(u64, String, TcpStream, TcpStream)>>,
}

impl Inscrite {
    fn couper(flux: &TcpStream) {
        let _ = flux.shutdown(Shutdown::Both);
    }

    /// Nouvelle liste des postes autorisés ; les sessions des postes retirés
    /// sont coupées aussitôt.
    fn mettre_postes(&self, postes: Vec<String>) {
        let nouveaux: HashSet<String> = postes.into_iter().collect();
        if let Ok(reliees) = self.reliees.lock() {
            for (_, poste, a, b) in reliees.iter() {
                if !nouveaux.contains(poste) {
                    Inscrite::couper(a);
                    Inscrite::couper(b);
                }
            }
        }
        if let Ok(mut postes) = self.postes.lock() {
            *postes = nouveaux;
        }
    }

    fn couper_tout(&self) {
        Inscrite::couper(&self.controle);
        if let Ok(reliees) = self.reliees.lock() {
            for (_, _, a, b) in reliees.iter() {
                Inscrite::couper(a);
                Inscrite::couper(b);
            }
        }
    }
}

/// Traces du relais (décision 59) : un fichier par jour, jamais de contenu.
struct Traces {
    dossier: Option<PathBuf>,
    jour: Mutex<String>,
}

impl Traces {
    fn noter(&self, ligne: &str) {
        let Some(dossier) = &self.dossier else {
            return;
        };
        let maintenant = chrono::Utc::now();
        let jour = maintenant.format("%Y-%m-%d").to_string();
        let Ok(mut courant) = self.jour.lock() else {
            return;
        };
        if *courant != jour {
            let _ = std::fs::create_dir_all(dossier);
            self.purger(dossier, maintenant.date_naive());
            *courant = jour.clone();
        }
        let chemin = dossier.join(format!("relais-{jour}.log"));
        if let Ok(mut fichier) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(chemin)
        {
            let _ = writeln!(fichier, "{} {ligne}", maintenant.format("%H:%M:%SZ"));
        }
    }

    fn purger(&self, dossier: &std::path::Path, aujourdhui: chrono::NaiveDate) {
        let Ok(entrees) = std::fs::read_dir(dossier) else {
            return;
        };
        for entree in entrees.flatten() {
            let nom = entree.file_name().to_string_lossy().into_owned();
            let Some(date) = nom
                .strip_prefix("relais-")
                .and_then(|n| n.strip_suffix(".log"))
                .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
            else {
                continue;
            };
            if (aujourdhui - date).num_days() > JOURS_DE_TRACES {
                let _ = std::fs::remove_file(entree.path());
            }
        }
    }
}

struct Etat {
    tls: Arc<ServerConfig>,
    cles: Vec<VerifyingKey>,
    inscrites: Mutex<HashMap<String, Arc<Inscrite>>>,
    /// Sessions demandées au serveur : identifiant, où remettre sa connexion.
    attente: Mutex<HashMap<String, mpsc::Sender<TcpStream>>>,
    en_attente: AtomicUsize,
    numero: AtomicU64,
    traces: Traces,
    arret: AtomicBool,
    attente_serveur: Duration,
    battement: Duration,
    silence: Duration,
}

/// Rend sa place au compteur quand la connexion se termine.
struct Place<'a>(&'a AtomicUsize);

impl Drop for Place<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

pub struct Relais {
    adresse: SocketAddr,
    empreinte: String,
    etat: Arc<Etat>,
    ecoute: Option<JoinHandle<()>>,
}

impl Relais {
    /// Lit ou crée l'identité du relais, puis écoute ; rend la main aussitôt.
    pub fn demarrer(config: Config) -> Result<Relais, String> {
        let identite = identite_relais(&config.dossier)?;
        let tls = ServerConfig::builder_with_provider(fournisseur())
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(|e| e.to_string())?
            .with_no_client_auth()
            .with_single_cert(
                vec![identite.certificat.clone().into()],
                rustls::pki_types::PrivateKeyDer::Pkcs8(identite.cle.clone().into()),
            )
            .map_err(|e| e.to_string())?;
        let ecoute = TcpListener::bind(config.adresse)
            .map_err(|e| format!("écoute sur {} impossible : {e}", config.adresse))?;
        let adresse = ecoute.local_addr().map_err(|e| e.to_string())?;
        let etat = Arc::new(Etat {
            tls: Arc::new(tls),
            cles: config.cles_portail,
            inscrites: Mutex::default(),
            attente: Mutex::default(),
            en_attente: AtomicUsize::new(0),
            numero: AtomicU64::new(0),
            traces: Traces {
                dossier: config.traces,
                jour: Mutex::default(),
            },
            arret: AtomicBool::new(false),
            attente_serveur: config.attente_serveur,
            battement: config.battement,
            silence: config.silence,
        });
        let fil = {
            let etat = etat.clone();
            thread::spawn(move || ecouter(ecoute, etat))
        };
        Ok(Relais {
            adresse,
            empreinte: identite.empreinte(),
            etat,
            ecoute: Some(fil),
        })
    }

    pub fn adresse(&self) -> SocketAddr {
        self.adresse
    }

    /// Empreinte du certificat du relais, à régler sur les serveurs.
    pub fn empreinte(&self) -> &str {
        &self.empreinte
    }

    /// Entreprises enregistrées en ce moment (essais, supervision).
    pub fn entreprises(&self) -> Vec<String> {
        self.etat
            .inscrites
            .lock()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Arrête l'écoute et coupe toutes les connexions.
    pub fn arreter(mut self) {
        self.fermer();
    }

    fn fermer(&mut self) {
        self.etat.arret.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect_timeout(&self.adresse, Duration::from_secs(1));
        if let Some(fil) = self.ecoute.take() {
            let _ = fil.join();
        }
        if let Ok(inscrites) = self.etat.inscrites.lock() {
            for inscrite in inscrites.values() {
                inscrite.couper_tout();
            }
        }
    }
}

impl Drop for Relais {
    fn drop(&mut self) {
        if self.ecoute.is_some() {
            self.fermer();
        }
    }
}

fn ecouter(ecoute: TcpListener, etat: Arc<Etat>) {
    let ouvertes = Arc::new(AtomicUsize::new(0));
    for flux in ecoute.incoming() {
        if etat.arret.load(Ordering::SeqCst) {
            break;
        }
        let Ok(tcp) = flux else {
            thread::sleep(Duration::from_millis(50));
            continue;
        };
        if ouvertes.fetch_add(1, Ordering::SeqCst) >= CONNEXIONS_MAX {
            ouvertes.fetch_sub(1, Ordering::SeqCst);
            continue;
        }
        let etat = etat.clone();
        let ouvertes = ouvertes.clone();
        thread::spawn(move || {
            let _place = Place(&ouvertes);
            connexion(&etat, tcp);
        });
    }
}

/// Premier octet 0x16 : poignée de main TLS, connexion de contrôle d'un
/// serveur ; sinon, trames en clair (une trame en clair fait moins de 4 Kio,
/// son premier octet est au plus 0x10).
fn connexion(etat: &Etat, tcp: TcpStream) {
    if tcp.set_nodelay(true).is_err() || tcp.set_read_timeout(Some(DELAI_TRAMES)).is_err() {
        return;
    }
    let _ = tcp.set_write_timeout(Some(DELAI_TRAMES));
    let mut premier = [0u8; 1];
    match tcp.peek(&mut premier) {
        Ok(1) if premier[0] == 0x16 => controle(etat, tcp),
        Ok(1) => en_clair(etat, tcp),
        _ => {}
    }
}

fn refuser(flux: &mut impl Write, motif: &str, valeurs: Vec<String>) {
    let _ = trames::ecrire(
        flux,
        &Trame::Refus {
            motif: motif.into(),
            valeurs,
        },
    );
}

fn en_clair(etat: &Etat, mut tcp: TcpStream) {
    match trames::lire(&mut tcp, TAILLE_EN_CLAIR) {
        Ok(Trame::Poste {
            version,
            entreprise,
        }) => poste(etat, tcp, version, &entreprise),
        Ok(Trame::Session { session, .. }) => {
            let attendue = etat
                .attente
                .lock()
                .ok()
                .and_then(|mut a| a.remove(&session));
            if let Some(remise) = attendue {
                let _ = remise.send(tcp);
            }
        }
        Ok(_) => refuser(&mut tcp, motif::PROTOCOLE, Vec::new()),
        Err(_) => {}
    }
}

fn poste(etat: &Etat, mut tcp: TcpStream, version: u32, entreprise: &str) {
    let refus = |tcp: &mut TcpStream, motif: &str, poste: &str, valeurs: Vec<String>| {
        refuser(tcp, motif, valeurs);
        etat.traces.noter(&format!(
            "refus_poste entreprise={entreprise} poste={poste} motif={motif}"
        ));
    };
    if version != VERSION {
        return refus(&mut tcp, motif::VERSION, "-", vec![VERSION.to_string()]);
    }
    let inscrite = etat
        .inscrites
        .lock()
        .ok()
        .and_then(|m| m.get(entreprise).cloned());
    let Some(inscrite) = inscrite else {
        return refus(&mut tcp, motif::SERVEUR_INJOIGNABLE, "-", Vec::new());
    };
    let Ok(defi) = preuve::alea::<32>() else {
        return;
    };
    if trames::ecrire(
        &mut tcp,
        &Trame::Defi {
            defi: preuve::base64(&defi),
        },
    )
    .is_err()
    {
        return;
    }
    let (certificat, signature) = match trames::lire(&mut tcp, TAILLE_EN_CLAIR) {
        Ok(Trame::Preuve {
            certificat,
            signature,
        }) => match (
            preuve::depuis_base64(&certificat),
            preuve::depuis_base64(&signature),
        ) {
            (Some(c), Some(s)) => (c, s),
            _ => return refus(&mut tcp, motif::PROTOCOLE, "-", Vec::new()),
        },
        Ok(_) => return refus(&mut tcp, motif::PROTOCOLE, "-", Vec::new()),
        Err(_) => return,
    };
    let poste = empreinte(&certificat);
    let autorise = inscrite
        .postes
        .lock()
        .map(|p| p.contains(&poste))
        .unwrap_or(false);
    if !autorise
        || !preuve::verifier(
            &certificat,
            &preuve::message(ROLE_POSTE, entreprise, &defi),
            &signature,
        )
    {
        return refus(&mut tcp, motif::POSTE_NON_AUTORISE, &poste, Vec::new());
    }
    // Places : attente par entreprise et en tout, sessions reliées.
    let reliees = inscrite.reliees.lock().map(|r| r.len()).unwrap_or(0);
    let par_entreprise = inscrite.en_attente.fetch_add(1, Ordering::SeqCst);
    let en_tout = etat.en_attente.fetch_add(1, Ordering::SeqCst);
    let place_entreprise = Place(&inscrite.en_attente);
    let place_totale = Place(&etat.en_attente);
    if par_entreprise >= ATTENTES_PAR_ENTREPRISE
        || en_tout >= ATTENTES_EN_TOUT
        || reliees >= RELIEES_PAR_ENTREPRISE
    {
        return refus(&mut tcp, motif::TROP_DE_SESSIONS, &poste, Vec::new());
    }
    let Ok(session) = preuve::alea::<16>().map(|o| preuve::hex(&o)) else {
        return;
    };
    let (remise, reception) = mpsc::channel();
    if let Ok(mut attente) = etat.attente.lock() {
        attente.insert(session.clone(), remise);
    }
    let demandee = inscrite
        .demandes
        .lock()
        .map(|d| d.send(session.clone()).is_ok())
        .unwrap_or(false);
    let serveur = if demandee {
        reception.recv_timeout(etat.attente_serveur).ok()
    } else {
        None
    };
    if let Ok(mut attente) = etat.attente.lock() {
        attente.remove(&session);
    }
    drop(place_entreprise);
    drop(place_totale);
    let Some(serveur) = serveur else {
        return refus(&mut tcp, motif::SERVEUR_INJOIGNABLE, &poste, Vec::new());
    };
    if trames::ecrire(&mut tcp, &Trame::Relie).is_err() {
        return;
    }
    relier(etat, &inscrite, &poste, tcp, serveur);
}

/// Copie d'un côté vers l'autre ; rend le volume copié. À la fin d'un côté,
/// l'autre est fermé en écriture ; sur une erreur, tout est coupé.
fn copier(mut de: TcpStream, mut vers: TcpStream) -> u64 {
    let mut tampon = vec![0u8; 16384];
    let mut total = 0u64;
    loop {
        match de.read(&mut tampon) {
            Ok(0) => {
                let _ = vers.shutdown(Shutdown::Write);
                break;
            }
            Ok(n) => {
                if vers.write_all(&tampon[..n]).is_err() {
                    let _ = de.shutdown(Shutdown::Both);
                    let _ = vers.shutdown(Shutdown::Both);
                    break;
                }
                total += n as u64;
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => {
                let _ = de.shutdown(Shutdown::Both);
                let _ = vers.shutdown(Shutdown::Both);
                break;
            }
        }
    }
    total
}

fn relier(etat: &Etat, inscrite: &Inscrite, poste: &str, tcp: TcpStream, serveur: TcpStream) {
    let debut = Instant::now();
    let preparer = |t: &TcpStream| {
        t.set_read_timeout(Some(INACTIVITE)).is_ok()
            && t.set_write_timeout(Some(INACTIVITE)).is_ok()
            && t.set_nodelay(true).is_ok()
    };
    if !preparer(&tcp) || !preparer(&serveur) {
        return;
    }
    let (Ok(tcp2), Ok(serveur2), Ok(tcp3), Ok(serveur3)) = (
        tcp.try_clone(),
        serveur.try_clone(),
        tcp.try_clone(),
        serveur.try_clone(),
    ) else {
        return;
    };
    let numero = etat.numero.fetch_add(1, Ordering::SeqCst);
    if let Ok(mut reliees) = inscrite.reliees.lock() {
        reliees.push((numero, poste.to_string(), tcp3, serveur3));
    }
    // Un poste retiré pendant la mise en relation : coupé tout de suite.
    let encore = inscrite
        .postes
        .lock()
        .map(|p| p.contains(poste))
        .unwrap_or(false);
    if !encore {
        let _ = tcp.shutdown(Shutdown::Both);
        let _ = serveur.shutdown(Shutdown::Both);
    }
    let retour = thread::spawn(move || copier(serveur2, tcp2));
    let montant = copier(tcp, serveur);
    let descendant = retour.join().unwrap_or(0);
    if let Ok(mut reliees) = inscrite.reliees.lock() {
        reliees.retain(|(n, ..)| *n != numero);
    }
    etat.traces.noter(&format!(
        "session entreprise={} poste={poste} duree={}s poste_vers_serveur={montant} serveur_vers_poste={descendant}",
        inscrite.entreprise,
        debut.elapsed().as_secs()
    ));
}

fn aujourdhui_utc() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

fn controle(etat: &Etat, tcp: TcpStream) {
    let Ok(copie) = tcp.try_clone() else {
        return;
    };
    let Ok(conn) = ServerConnection::new(etat.tls.clone()) else {
        return;
    };
    let mut canal = Canal::nouveau(conn, tcp);
    if canal.poignee(DELAI_TRAMES).is_err() {
        return;
    }
    let Ok(defi) = preuve::alea::<32>() else {
        return;
    };
    if canal
        .envoyer(&Trame::Defi {
            defi: preuve::base64(&defi),
        })
        .is_err()
    {
        return;
    }
    let refus = |canal: &mut Canal, motif: &str, entreprise: &str, valeurs: Vec<String>| {
        let _ = canal.envoyer(&Trame::Refus {
            motif: motif.into(),
            valeurs,
        });
        etat.traces.noter(&format!(
            "refus_serveur entreprise={entreprise} motif={motif}"
        ));
    };
    let (entreprise, licence, certificat, signature, postes) = match canal.attendre(DELAI_TRAMES) {
        Ok(Trame::Enregistrer {
            version,
            entreprise,
            licence,
            certificat,
            signature,
            postes,
        }) => {
            if version != VERSION {
                return refus(
                    &mut canal,
                    motif::VERSION,
                    &entreprise,
                    vec![VERSION.to_string()],
                );
            }
            match (
                preuve::depuis_base64(&certificat),
                preuve::depuis_base64(&signature),
            ) {
                (Some(c), Some(s)) => (entreprise, licence, c, s, postes),
                _ => return refus(&mut canal, motif::PROTOCOLE, &entreprise, Vec::new()),
            }
        }
        Ok(_) => return refus(&mut canal, motif::PROTOCOLE, "-", Vec::new()),
        Err(_) => return,
    };
    // Licence de cette entreprise, signée par le portail, pas échue
    // (décision 58).
    match kadenz_licence::entreprise::verifier(&licence, &entreprise, &etat.cles) {
        Ok(l) if l.echeance < aujourdhui_utc() => {
            return refus(
                &mut canal,
                motif::LICENCE_ECHUE,
                &entreprise,
                vec![l.echeance],
            );
        }
        Ok(_) => {}
        Err(_) => return refus(&mut canal, motif::LICENCE_REFUSEE, &entreprise, Vec::new()),
    }
    if !preuve::verifier(
        &certificat,
        &preuve::message(ROLE_SERVEUR, &entreprise, &defi),
        &signature,
    ) {
        return refus(&mut canal, motif::PREUVE_INVALIDE, &entreprise, Vec::new());
    }
    let serveur = empreinte(&certificat);
    let (demandes, a_envoyer) = mpsc::channel();
    let inscrite = Arc::new(Inscrite {
        entreprise: entreprise.clone(),
        serveur: serveur.clone(),
        postes: Mutex::new(postes.into_iter().collect()),
        demandes: Mutex::new(demandes),
        controle: copie,
        en_attente: AtomicUsize::new(0),
        reliees: Mutex::default(),
    });
    // Même certificat : le nouvel enregistrement remplace l'ancien (serveur
    // restauré sur une autre machine, connexion à demi morte). Autre
    // certificat : refusé tant que l'ancien est vivant.
    {
        let Ok(mut inscrites) = etat.inscrites.lock() else {
            return;
        };
        if let Some(ancienne) = inscrites.get(&entreprise) {
            if ancienne.serveur != serveur {
                drop(inscrites);
                return refus(&mut canal, motif::DEJA_ENREGISTREE, &entreprise, Vec::new());
            }
            Inscrite::couper(&ancienne.controle);
        }
        inscrites.insert(entreprise.clone(), inscrite.clone());
    }
    etat.traces.noter(&format!(
        "enregistrement entreprise={entreprise} serveur={serveur}"
    ));
    if canal.envoyer(&Trame::Enregistre).is_ok() {
        servir_controle(etat, &inscrite, &mut canal, &a_envoyer);
    }
    let retiree = etat
        .inscrites
        .lock()
        .map(|mut m| {
            let courante = m
                .get(&entreprise)
                .is_some_and(|c| Arc::ptr_eq(c, &inscrite));
            if courante {
                m.remove(&entreprise);
            }
            courante
        })
        .unwrap_or(false);
    Inscrite::couper(&inscrite.controle);
    if retiree {
        etat.traces
            .noter(&format!("fin_enregistrement entreprise={entreprise}"));
    }
}

fn servir_controle(
    etat: &Etat,
    inscrite: &Arc<Inscrite>,
    canal: &mut Canal,
    a_envoyer: &mpsc::Receiver<String>,
) {
    let mut dernier_envoi = Instant::now();
    let mut vu = Instant::now();
    loop {
        if etat.arret.load(Ordering::SeqCst) {
            return;
        }
        let courante = etat
            .inscrites
            .lock()
            .map(|m| {
                m.get(&inscrite.entreprise)
                    .is_some_and(|c| Arc::ptr_eq(c, inscrite))
            })
            .unwrap_or(false);
        if !courante {
            return;
        }
        while let Ok(session) = a_envoyer.try_recv() {
            if canal.envoyer(&Trame::Demande { session }).is_err() {
                return;
            }
            dernier_envoi = Instant::now();
        }
        if dernier_envoi.elapsed() >= etat.battement {
            if canal.envoyer(&Trame::Battement).is_err() {
                return;
            }
            dernier_envoi = Instant::now();
        }
        match canal.recevoir(Duration::from_millis(50)) {
            Ok(Some(Trame::Postes { postes })) => {
                vu = Instant::now();
                inscrite.mettre_postes(postes);
            }
            Ok(Some(Trame::Battement)) => vu = Instant::now(),
            Ok(Some(_)) | Err(_) => return,
            Ok(None) => {
                if vu.elapsed() > etat.silence {
                    return;
                }
            }
        }
    }
}
