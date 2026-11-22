//! Transport WebSocket du relais, pour passer par Cloudflare Tunnel
//! (docs/spec-relais-websocket.md). Chaque WebSocket porte exactement le flux
//! d'octets d'une connexion TCP au relais, en messages binaires : trames,
//! TLS épinglé et sessions de bout en bout passent au-dessus sans changer.
//!
//! - Côté poste et serveur : [`ouvrir`] ouvre la WebSocket puis rend un
//!   `TcpStream` local relié à elle par un pont, si bien que les appelants de
//!   [`crate::tcp_vers`] ne changent pas.
//! - Côté relais : [`Passerelle`] accepte les WebSocket (cloudflared) et
//!   rouvre chacune en TCP vers le relais lui-même ; l'adresse du client lue
//!   dans `CF-Connecting-IP` est transmise au frein par une table interne.

use std::collections::HashMap;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use rustls::ClientConfig;
use rustls::pki_types::ServerName;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::Semaphore;
use tokio::time::{MissedTickBehavior, timeout};
use tokio_rustls::TlsConnector;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::StatusCode;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::{Bytes, Message};

use crate::{ErreurRelais, injoignable};

/// Ping envoyé par chaque bout, pour le délai d'inactivité de Cloudflare
/// (non publié pour le plan gratuit ; avis d'OpenCode : 30 secondes).
pub const PING: Duration = Duration::from_secs(30);
/// Rien reçu pendant ce temps, pas même un pong : connexion fermée.
pub const SANS_NOUVELLES: Duration = Duration::from_secs(90);

/// Délais d'une WebSocket : [`PING`] et [`SANS_NOUVELLES`] par défaut ; les
/// essais les raccourcissent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DelaisWs {
    pub ping: Duration,
    pub sans_nouvelles: Duration,
}

impl Default for DelaisWs {
    fn default() -> DelaisWs {
        DelaisWs {
            ping: PING,
            sans_nouvelles: SANS_NOUVELLES,
        }
    }
}
/// Taille des messages envoyés, comme le tampon de copie du relais.
const MORCEAU: usize = 16 * 1024;
/// Taille d'un message ou d'une trame reçus, au plus.
const MESSAGE_MAX: usize = 64 * 1024;
/// Ouverture : poignée de main HTTP, et connexion locale vers le relais.
const OUVERTURE: Duration = Duration::from_secs(10);
/// WebSocket ouvertes en même temps sur la passerelle, au plus (comme le
/// plafond de connexions du relais).
const WEBSOCKETS_MAX: usize = 4096;

/// Sonde de la passerelle : la sonde TCP du relais ne voit pas si le port
/// WebSocket a cessé d'accepter (tâche tombée, places épuisées). Ouvre une
/// WebSocket locale et vérifie la poignée de main, sans envoyer de trame.
pub fn verifier_sante(adresse: SocketAddr, delai: Duration) -> Result<(), String> {
    let execution = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("sonde WebSocket : runtime impossible : {e}"))?;
    execution.block_on(async move {
        let url = format!("ws://{adresse}/");
        let ouverture = async {
            let flux = tokio::net::TcpStream::connect(adresse)
                .await
                .map_err(|e| format!("sonde WebSocket : connexion à {adresse} impossible : {e}"))?;
            tokio_tungstenite::client_async(url.as_str(), flux)
                .await
                .map_err(|e| format!("sonde WebSocket : poignée de main refusée : {e}"))?;
            Ok(())
        };
        timeout(delai, ouverture)
            .await
            .map_err(|_| format!("sonde WebSocket : délai dépassé sur {adresse}"))?
    })
}

/// Adresse WebSocket d'un relais : `wss://hôte[:port]`, ou `ws://` vers
/// `127.0.0.1` et `localhost` seulement (essais, banc).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdresseWs {
    pub chiffree: bool,
    pub hote: String,
    pub port: u16,
}

impl AdresseWs {
    /// `None` si `texte` n'est pas une adresse WebSocket admise.
    pub fn lire(texte: &str) -> Option<AdresseWs> {
        let texte = texte.trim();
        let (chiffree, reste) = if let Some(r) = texte.strip_prefix("wss://") {
            (true, r)
        } else {
            (false, texte.strip_prefix("ws://")?)
        };
        let reste = reste.strip_suffix('/').unwrap_or(reste);
        let (hote, port) = match reste.rsplit_once(':') {
            Some((h, p)) => (h, p.parse::<u16>().ok().filter(|p| *p > 0)?),
            None => (reste, if chiffree { 443 } else { 80 }),
        };
        let permis = |c: char| c.is_ascii_alphanumeric() || c == '.' || c == '-';
        if hote.is_empty() || !hote.chars().all(permis) || hote.starts_with(['.', '-']) {
            return None;
        }
        let hote = hote.to_ascii_lowercase();
        if !chiffree && hote != "127.0.0.1" && hote != "localhost" {
            return None;
        }
        Some(AdresseWs {
            chiffree,
            hote,
            port,
        })
    }

    /// Forme à garder : port omis quand c'est celui par défaut.
    pub fn texte(&self) -> String {
        let schema = if self.chiffree { "wss" } else { "ws" };
        let defaut = if self.chiffree { 443 } else { 80 };
        if self.port == defaut {
            format!("{schema}://{}", self.hote)
        } else {
            format!("{schema}://{}:{}", self.hote, self.port)
        }
    }

    /// Adresse de la requête d'ouverture ; port par défaut omis, pour que
    /// l'en-tête `Host` soit le nom seul, comme pour un navigateur.
    fn url(&self) -> String {
        format!("{}/", self.texte())
    }
}

/// `true` si `texte` commence comme une adresse WebSocket, admise ou non.
pub(crate) fn ressemble(texte: &str) -> bool {
    let t = texte.trim();
    t.starts_with("ws://") || t.starts_with("wss://")
}

fn config_ws() -> WebSocketConfig {
    WebSocketConfig::default()
        .max_message_size(Some(MESSAGE_MAX))
        .max_frame_size(Some(MESSAGE_MAX))
}

// ----- Copie dans les deux sens -----

/// Copie entre une WebSocket et une connexion TCP, jusqu'à la fermeture de
/// l'une ou de l'autre. Ping à chaque `delais.ping` ; rien reçu pendant
/// `delais.sans_nouvelles`, message texte, trame trop grande : fin.
async fn pomper<S>(ws: WebSocketStream<S>, tcp: &mut tokio::net::TcpStream, delais: DelaisWs)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let (mut envoi, mut reception) = ws.split();
    let (mut lecture, mut ecriture) = tcp.split();
    let mut tampon = vec![0u8; MORCEAU];
    let mut battement = tokio::time::interval(delais.ping);
    battement.set_missed_tick_behavior(MissedTickBehavior::Delay);
    battement.tick().await;
    let mut recu = Instant::now();
    loop {
        tokio::select! {
            lu = lecture.read(&mut tampon) => match lu {
                Ok(0) | Err(_) => {
                    let _ = envoi.send(Message::Close(None)).await;
                    break;
                }
                Ok(n) => {
                    let message = Message::Binary(Bytes::copy_from_slice(&tampon[..n]));
                    if envoi.send(message).await.is_err() {
                        break;
                    }
                }
            },
            message = reception.next() => {
                recu = Instant::now();
                match message {
                    Some(Ok(Message::Binary(octets))) => {
                        if ecriture.write_all(&octets).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
                    _ => break,
                }
            },
            _ = battement.tick() => {
                if recu.elapsed() > delais.sans_nouvelles
                    || envoi.send(Message::Ping(Bytes::new())).await.is_err()
                {
                    break;
                }
            }
        }
    }
    let _ = ecriture.shutdown().await;
}

// ----- Côté poste et serveur : pont local -----

/// Configuration TLS vers Cloudflare : racines du magasin du système (elles
/// suivent ses mises à jour et les autorités ajoutées par une entreprise),
/// fournisseur ring, versions par défaut de rustls (TLS 1.2 et 1.3 : un proxy
/// d'inspection d'entreprise ne parle souvent que 1.2 ; la protection reste
/// le TLS 1.3 épinglé du relais, à l'intérieur). Gardée une fois construite.
fn config_tls() -> Result<Arc<ClientConfig>, ErreurRelais> {
    static CONFIG: OnceLock<Arc<ClientConfig>> = OnceLock::new();
    if let Some(c) = CONFIG.get() {
        return Ok(c.clone());
    }
    let trouvees = rustls_native_certs::load_native_certs();
    let mut racines = rustls::RootCertStore::empty();
    let (ajoutees, _) = racines.add_parsable_certificates(trouvees.certs);
    if ajoutees == 0 {
        return Err(ErreurRelais::Injoignable(
            "aucune autorité de certification lisible dans le magasin du système".into(),
        ));
    }
    let mut config = ClientConfig::builder_with_provider(crate::tls::fournisseur())
        .with_safe_default_protocol_versions()
        .map_err(|e| ErreurRelais::Injoignable(e.to_string()))?
        .with_root_certificates(racines)
        .with_no_client_auth();
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(CONFIG.get_or_init(|| Arc::new(config)).clone())
}

fn erreur_ws(e: impl std::fmt::Display) -> ErreurRelais {
    ErreurRelais::Injoignable(e.to_string())
}

async fn tcp_vers_ws(adresse: &AdresseWs) -> Result<tokio::net::TcpStream, ErreurRelais> {
    let tcp = tokio::net::TcpStream::connect((adresse.hote.as_str(), adresse.port))
        .await
        .map_err(injoignable)?;
    tcp.set_nodelay(true).map_err(injoignable)?;
    Ok(tcp)
}

/// Ouvre une WebSocket vers le relais à `adresse` et rend un `TcpStream`
/// local relié à elle : ce qui y est écrit part dans la WebSocket, et
/// inversement. Le pont vit dans son propre fil et se ferme avec l'un ou
/// l'autre bout.
pub(crate) fn ouvrir(adresse: &AdresseWs, delai: Duration) -> Result<TcpStream, ErreurRelais> {
    let execution = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .map_err(injoignable)?;
    let url = adresse.url();
    let depasse = || ErreurRelais::Injoignable("délai dépassé".into());
    if adresse.chiffree {
        let tls = config_tls()?;
        let nom = ServerName::try_from(adresse.hote.clone()).map_err(erreur_ws)?;
        let ws = execution.block_on(async {
            timeout(delai, async {
                let tcp = tcp_vers_ws(adresse).await?;
                let flux = TlsConnector::from(tls)
                    .connect(nom, tcp)
                    .await
                    .map_err(injoignable)?;
                tokio_tungstenite::client_async_with_config(url, flux, Some(config_ws()))
                    .await
                    .map(|(ws, _)| ws)
                    .map_err(erreur_ws)
            })
            .await
            .map_err(|_| depasse())?
        })?;
        lancer_pont(execution, ws, delai)
    } else {
        let ws = execution.block_on(async {
            timeout(delai, async {
                let tcp = tcp_vers_ws(adresse).await?;
                tokio_tungstenite::client_async_with_config(url, tcp, Some(config_ws()))
                    .await
                    .map(|(ws, _)| ws)
                    .map_err(erreur_ws)
            })
            .await
            .map_err(|_| depasse())?
        })?;
        lancer_pont(execution, ws, delai)
    }
}

fn lancer_pont<S>(
    execution: tokio::runtime::Runtime,
    ws: WebSocketStream<S>,
    delai: Duration,
) -> Result<TcpStream, ErreurRelais>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (local, pont) = paire_locale(delai)?;
    pont.set_nonblocking(true).map_err(injoignable)?;
    thread::Builder::new()
        .name("pont-relais".into())
        .spawn(move || {
            execution.block_on(async move {
                if let Ok(mut pont) = tokio::net::TcpStream::from_std(pont) {
                    pomper(ws, &mut pont, DelaisWs::default()).await;
                }
            });
        })
        .map_err(injoignable)?;
    Ok(local)
}

/// Deux bouts d'une connexion TCP locale. L'écoute, sur un port tiré par le
/// système, n'accepte que la connexion ouverte ici même (adresse source
/// comparée) : un autre programme du poste qui s'y présenterait est fermé
/// sans suite. L'écoute est fermée aussitôt après.
fn paire_locale(delai: Duration) -> Result<(TcpStream, TcpStream), ErreurRelais> {
    let ecoute = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).map_err(injoignable)?;
    let cible = ecoute.local_addr().map_err(injoignable)?;
    let local = TcpStream::connect_timeout(&cible, delai).map_err(injoignable)?;
    let attendu = local.local_addr().map_err(injoignable)?;
    ecoute.set_nonblocking(true).map_err(injoignable)?;
    let fin = Instant::now() + delai;
    loop {
        match ecoute.accept() {
            Ok((pont, pair)) if pair == attendu => {
                // Sous Windows, la connexion acceptée hérite du mode non
                // bloquant de l'écoute : rendue bloquante, comme `local`.
                pont.set_nonblocking(false).map_err(injoignable)?;
                return Ok((local, pont));
            }
            Ok((etranger, _)) => drop(etranger),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                if Instant::now() >= fin {
                    return Err(ErreurRelais::Injoignable(
                        "pont local : délai dépassé".into(),
                    ));
                }
                thread::sleep(Duration::from_millis(1));
            }
            Err(e) => return Err(injoignable(e)),
        }
    }
}

// ----- Côté relais : passerelle -----

/// Adresse du client de chaque connexion locale ouverte par la passerelle,
/// par port source : lue par l'écoute du relais pour son frein, et retirée
/// aussitôt (ou par la passerelle si la connexion échoue ou se termine).
pub(crate) type Clients = Arc<Mutex<HashMap<u16, IpAddr>>>;

pub(crate) struct Passerelle {
    adresse: SocketAddr,
    execution: Option<tokio::runtime::Runtime>,
}

/// Limite globale d'ouverture, avant le sémaphore : derrière Cloudflare, les
/// adresses client ne sont connues qu'après la poignée de main, donc un flot
/// réparti pourrait occuper les 4096 places avant que le frein du relais ne
/// voie qui que ce soit. La rafale laisse passer une entreprise qui se
/// reconnecte d'un coup, le rythme borne ensuite l'occupation des places ;
/// le frein par adresse reste la première barrière, côté relais.
const RAFALE_WS: f64 = 1024.0;
const PAR_SECONDE_WS: f64 = 256.0;

#[derive(Debug)]
struct Debit {
    jetons: f64,
    vu: Instant,
}

impl Passerelle {
    /// Écoute des WebSocket sur `ecoute` ; chacune est rouverte en TCP vers
    /// `relais`, en 127.0.0.1 (la table des clients n'a de sens que là).
    /// `mandataire` : seule adresse dont l'en-tête `CF-Connecting-IP` est cru.
    pub(crate) fn demarrer(
        ecoute: SocketAddr,
        relais: SocketAddr,
        mandataire: Option<IpAddr>,
        delais: DelaisWs,
        clients: Clients,
    ) -> Result<Passerelle, String> {
        if relais.ip() != IpAddr::V4(Ipv4Addr::LOCALHOST) {
            return Err(format!(
                "passerelle WebSocket : le relais doit être joint en 127.0.0.1, pas {relais}"
            ));
        }
        let execution = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("passerelle")
            .enable_io()
            .enable_time()
            .build()
            .map_err(|e| format!("passerelle WebSocket impossible : {e}"))?;
        let ecouteur = execution
            .block_on(tokio::net::TcpListener::bind(ecoute))
            .map_err(|e| format!("écoute WebSocket sur {ecoute} impossible : {e}"))?;
        let adresse = ecouteur.local_addr().map_err(|e| e.to_string())?;
        let places = Arc::new(Semaphore::new(WEBSOCKETS_MAX));
        let debit = Arc::new(Mutex::new(Debit {
            jetons: RAFALE_WS,
            vu: Instant::now(),
        }));
        execution.spawn(async move {
            loop {
                let Ok((flux, pair)) = ecouteur.accept().await else {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    continue;
                };
                // Admis seulement au rythme global, avant de consommer une place.
                {
                    let Ok(mut debit) = debit.lock() else {
                        continue;
                    };
                    let maintenant = Instant::now();
                    let ecoule = maintenant.saturating_duration_since(debit.vu).as_secs_f64();
                    debit.jetons = (debit.jetons + ecoule * PAR_SECONDE_WS).min(RAFALE_WS);
                    debit.vu = maintenant;
                    if debit.jetons < 1.0 {
                        continue;
                    }
                    debit.jetons -= 1.0;
                }
                let Ok(place) = places.clone().try_acquire_owned() else {
                    continue;
                };
                let clients = clients.clone();
                tokio::spawn(async move {
                    let _place = place;
                    servir(flux, pair, relais, mandataire, delais, clients).await;
                });
            }
        });
        Ok(Passerelle {
            adresse,
            execution: Some(execution),
        })
    }

    pub(crate) fn adresse(&self) -> SocketAddr {
        self.adresse
    }
}

impl Drop for Passerelle {
    fn drop(&mut self) {
        if let Some(e) = self.execution.take() {
            e.shutdown_background();
        }
    }
}

async fn servir(
    flux: tokio::net::TcpStream,
    pair: SocketAddr,
    relais: SocketAddr,
    mandataire: Option<IpAddr>,
    delais: DelaisWs,
    clients: Clients,
) {
    let _ = flux.set_nodelay(true);
    let du_mandataire = mandataire == Some(pair.ip());
    let mut client = pair.ip();
    // Signature imposée par tungstenite (`Callback`) : l'erreur est la
    // réponse HTTP de refus elle-même.
    #[allow(clippy::result_large_err)]
    let rappel = |requete: &Request, reponse: Response| -> Result<Response, ErrorResponse> {
        if requete.uri().path() != "/" {
            let mut refus = ErrorResponse::new(None);
            *refus.status_mut() = StatusCode::NOT_FOUND;
            return Err(refus);
        }
        if du_mandataire
            && let Some(ip) = requete
                .headers()
                .get("cf-connecting-ip")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.trim().parse::<IpAddr>().ok())
        {
            client = ip;
        }
        Ok(reponse)
    };
    let ouverture =
        tokio_tungstenite::accept_hdr_async_with_config(flux, rappel, Some(config_ws()));
    let Ok(Ok(ws)) = timeout(OUVERTURE, ouverture).await else {
        return;
    };
    let Ok(prise) = tokio::net::TcpSocket::new_v4() else {
        return;
    };
    if prise.bind((Ipv4Addr::LOCALHOST, 0).into()).is_err() {
        return;
    }
    let Ok(port) = prise.local_addr().map(|a| a.port()) else {
        return;
    };
    if let Ok(mut c) = clients.lock() {
        c.insert(port, client);
    }
    // L'entrée est retirée avant de fermer la connexion locale : son port ne
    // peut pas avoir été repris entre-temps par une autre.
    let connexion = timeout(OUVERTURE, prise.connect(relais)).await;
    if let Ok(Ok(mut tcp)) = connexion {
        let _ = tcp.set_nodelay(true);
        pomper(ws, &mut tcp, delais).await;
        retirer(&clients, port);
        drop(tcp);
    } else {
        retirer(&clients, port);
    }
}

fn retirer(clients: &Clients, port: u16) {
    if let Ok(mut c) = clients.lock() {
        c.remove(&port);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adresses_admises_et_refusees() {
        let a = AdresseWs::lire("wss://Relais.Exemple.fr/").unwrap();
        assert_eq!(
            a,
            AdresseWs {
                chiffree: true,
                hote: "relais.exemple.fr".into(),
                port: 443
            }
        );
        assert_eq!(a.texte(), "wss://relais.exemple.fr");
        let b = AdresseWs::lire("wss://relais.exemple.fr:8443").unwrap();
        assert_eq!(b.port, 8443);
        assert_eq!(b.texte(), "wss://relais.exemple.fr:8443");
        assert_eq!(
            AdresseWs::lire("ws://127.0.0.1:47383").unwrap().texte(),
            "ws://127.0.0.1:47383"
        );
        for refusee in [
            "relais.exemple.fr:47381",
            "ws://relais.exemple.fr",
            "wss://",
            "wss://relais.exemple.fr/chemin",
            "wss://relais exemple.fr",
            "wss://relais.exemple.fr:0",
            "wss://relais.exemple.fr:70000",
            "wss://-relais.fr",
            "wss://a\"b",
        ] {
            assert_eq!(AdresseWs::lire(refusee), None, "{refusee}");
        }
    }

    #[test]
    fn pont_local_relie_dans_les_deux_sens() {
        // La paire rendue est reliée dans les deux sens.
        let (mut local, mut pont) = paire_locale(Duration::from_secs(2)).unwrap();
        use std::io::{Read, Write};
        local.write_all(b"aller").unwrap();
        let mut lu = [0u8; 5];
        pont.read_exact(&mut lu).unwrap();
        assert_eq!(&lu, b"aller");
        pont.write_all(b"retour").unwrap();
        let mut lu = [0u8; 6];
        local.read_exact(&mut lu).unwrap();
        assert_eq!(&lu, b"retour");
    }
}

#[cfg(test)]
mod essais_passerelle {
    use super::*;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;

    /// Ouvre une WebSocket vers la passerelle avec un `CF-Connecting-IP`, et
    /// rend l'adresse notée pour le frein, lue à l'arrivée sur un faux relais.
    async fn adresse_notee(mandataire: Option<IpAddr>) -> IpAddr {
        let faux_relais = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let cible = faux_relais.local_addr().unwrap();
        let clients = Clients::default();
        let passerelle = tokio::task::spawn_blocking({
            let clients = clients.clone();
            move || {
                Passerelle::demarrer(
                    (Ipv4Addr::LOCALHOST, 0).into(),
                    cible,
                    mandataire,
                    DelaisWs::default(),
                    clients,
                )
            }
        })
        .await
        .unwrap()
        .unwrap();
        let mut requete = format!("ws://{}/", passerelle.adresse())
            .into_client_request()
            .unwrap();
        requete
            .headers_mut()
            .insert("cf-connecting-ip", "203.0.113.7".parse().unwrap());
        let tcp = tokio::net::TcpStream::connect(passerelle.adresse())
            .await
            .unwrap();
        let (mut ws, _) = tokio_tungstenite::client_async(requete, tcp).await.unwrap();
        let (mut arrivee, pair) = faux_relais.accept().await.unwrap();
        let note = clients.lock().unwrap().get(&pair.port()).copied().unwrap();

        // Le flux passe, puis un message texte ferme la connexion.
        ws.send(Message::Binary(Bytes::from_static(b"octets")))
            .await
            .unwrap();
        let mut lu = [0u8; 6];
        arrivee.read_exact(&mut lu).await.unwrap();
        assert_eq!(&lu, b"octets");
        ws.send(Message::Text("texte".into())).await.unwrap();
        let mut reste = Vec::new();
        timeout(Duration::from_secs(5), arrivee.read_to_end(&mut reste))
            .await
            .unwrap()
            .unwrap();
        assert!(reste.is_empty());
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(clients.lock().unwrap().is_empty());
        tokio::task::spawn_blocking(move || drop(passerelle))
            .await
            .unwrap();
        note
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn en_tete_cru_seulement_du_mandataire() {
        let client: IpAddr = "203.0.113.7".parse().unwrap();
        assert_eq!(
            adresse_notee(Some(IpAddr::V4(Ipv4Addr::LOCALHOST))).await,
            client
        );
        assert_eq!(adresse_notee(None).await, IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert_eq!(
            adresse_notee(Some("10.0.0.9".parse().unwrap())).await,
            IpAddr::V4(Ipv4Addr::LOCALHOST)
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn autre_chemin_refuse() {
        let faux_relais = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let cible = faux_relais.local_addr().unwrap();
        let passerelle = tokio::task::spawn_blocking(move || {
            Passerelle::demarrer(
                (Ipv4Addr::LOCALHOST, 0).into(),
                cible,
                None,
                DelaisWs::default(),
                Clients::default(),
            )
        })
        .await
        .unwrap()
        .unwrap();
        let tcp = tokio::net::TcpStream::connect(passerelle.adresse())
            .await
            .unwrap();
        let url = format!("ws://{}/autre", passerelle.adresse());
        assert!(tokio_tungstenite::client_async(url, tcp).await.is_err());
        tokio::task::spawn_blocking(move || drop(passerelle))
            .await
            .unwrap();
    }
}

/// Essai à la main, par le réseau : `cargo test -p kadenz-relais -- --ignored`.
#[cfg(test)]
mod essais_cloudflare {
    use super::*;

    /// Ouverture TLS vers Cloudflare avec les racines du système, par le nom
    /// du portail, servi par le même tunnel. Le portail n'est pas une
    /// passerelle : il répond par un statut HTTP (redirection ou refus),
    /// preuve que la chaîne de certificats, le SNI et l'aiguillage par `Host`
    /// jusqu'à cloudflared fonctionnent. Une erreur TLS ou de certificat ferait
    /// échouer l'essai.
    #[test]
    #[ignore = "réseau : passe par Cloudflare"]
    fn tls_vers_cloudflare() {
        let adresse = AdresseWs::lire("wss://licences.mcarre.dev").unwrap();
        match ouvrir(&adresse, Duration::from_secs(15)) {
            Err(ErreurRelais::Injoignable(e)) => {
                eprintln!("réponse du portail : {e}");
                assert!(e.starts_with("HTTP error"), "{e}");
            }
            Err(autre) => panic!("{autre}"),
            Ok(_) => panic!("le portail a accepté une WebSocket"),
        }
    }
}

#[cfg(test)]
mod essais_limites {
    use super::*;

    async fn passerelle(delais: DelaisWs) -> (Passerelle, tokio::net::TcpListener) {
        let faux_relais = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let cible = faux_relais.local_addr().unwrap();
        let p = tokio::task::spawn_blocking(move || {
            Passerelle::demarrer(
                (Ipv4Addr::LOCALHOST, 0).into(),
                cible,
                None,
                delais,
                Clients::default(),
            )
        })
        .await
        .unwrap()
        .unwrap();
        (p, faux_relais)
    }

    async fn client(p: &Passerelle) -> WebSocketStream<tokio::net::TcpStream> {
        let tcp = tokio::net::TcpStream::connect(p.adresse()).await.unwrap();
        tokio_tungstenite::client_async(format!("ws://{}/", p.adresse()), tcp)
            .await
            .unwrap()
            .0
    }

    /// Lit jusqu'à la fin du flux (5 secondes au plus) ; rend ce qui restait.
    async fn fin_du_flux(arrivee: &mut tokio::net::TcpStream) -> Vec<u8> {
        let mut reste = Vec::new();
        timeout(Duration::from_secs(5), arrivee.read_to_end(&mut reste))
            .await
            .unwrap()
            .unwrap();
        reste
    }

    async fn fermer(p: Passerelle) {
        tokio::task::spawn_blocking(move || drop(p)).await.unwrap();
    }

    /// Message de plus de 64 Kio reçu par la passerelle : connexion fermée
    /// sans rien transmettre au relais, sans panique ; la passerelle sert
    /// toujours les suivantes.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn message_trop_grand_refuse_par_la_passerelle() {
        let (p, faux_relais) = passerelle(DelaisWs::default()).await;
        let mut ws = client(&p).await;
        let (mut arrivee, _) = faux_relais.accept().await.unwrap();
        let _ = ws
            .send(Message::Binary(Bytes::from(vec![7u8; 70 * 1024])))
            .await;
        assert!(fin_du_flux(&mut arrivee).await.is_empty());

        let mut suivante = client(&p).await;
        let (mut arrivee, _) = faux_relais.accept().await.unwrap();
        suivante
            .send(Message::Binary(Bytes::from_static(b"ok")))
            .await
            .unwrap();
        let mut lu = [0u8; 2];
        arrivee.read_exact(&mut lu).await.unwrap();
        assert_eq!(&lu, b"ok");
        fermer(p).await;
    }

    /// Message de plus de 64 Kio reçu par le pont du client : la connexion
    /// locale se ferme sans rien rendre, sans panique.
    #[test]
    fn message_trop_grand_refuse_par_le_pont() {
        use std::io::Read;
        let ecoute = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = ecoute.local_addr().unwrap().port();
        let faux_relais = thread::spawn(move || {
            let execution = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            execution.block_on(async move {
                ecoute.set_nonblocking(true).unwrap();
                let ecoute = tokio::net::TcpListener::from_std(ecoute).unwrap();
                let (tcp, _) = ecoute.accept().await.unwrap();
                let mut ws = tokio_tungstenite::accept_async(tcp).await.unwrap();
                let _ = ws
                    .send(Message::Binary(Bytes::from(vec![7u8; 70 * 1024])))
                    .await;
                while let Some(Ok(_)) = ws.next().await {}
            });
        });
        let adresse = AdresseWs::lire(&format!("ws://127.0.0.1:{port}")).unwrap();
        let mut local = ouvrir(&adresse, Duration::from_secs(5)).unwrap();
        local
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut reste = Vec::new();
        local.read_to_end(&mut reste).unwrap();
        assert!(reste.is_empty());
        faux_relais.join().unwrap();
    }

    /// Délais raccourcis (ping 100 ms, silence 400 ms) : un pair qui lit, et
    /// répond donc aux pings, garde une session muette ouverte bien au-delà
    /// du silence ; un pair qui ne répond plus est coupé après le silence.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn ping_garde_ouverte_et_silence_coupe() {
        let delais = DelaisWs {
            ping: Duration::from_millis(100),
            sans_nouvelles: Duration::from_millis(400),
        };
        let (p, faux_relais) = passerelle(delais).await;

        let ws = client(&p).await;
        let (mut arrivee, _) = faux_relais.accept().await.unwrap();
        let (mut envoi, mut reception) = ws.split();
        let lecteur =
            tokio::spawn(async move { while let Some(Ok(_)) = reception.next().await {} });
        tokio::time::sleep(Duration::from_millis(1500)).await;
        envoi
            .send(Message::Binary(Bytes::from_static(b"encore")))
            .await
            .unwrap();
        let mut lu = [0u8; 6];
        timeout(Duration::from_secs(2), arrivee.read_exact(&mut lu))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(&lu, b"encore");

        let _muet = client(&p).await;
        let (mut arrivee, _) = faux_relais.accept().await.unwrap();
        let debut = Instant::now();
        assert!(fin_du_flux(&mut arrivee).await.is_empty());
        let duree = debut.elapsed();
        assert!(
            duree >= Duration::from_millis(400) && duree < Duration::from_secs(3),
            "{duree:?}"
        );
        lecteur.abort();
        fermer(p).await;
    }
}
