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
/// Taille des messages envoyés, comme le tampon de copie du relais.
const MORCEAU: usize = 16 * 1024;
/// Taille d'un message ou d'une trame reçus, au plus.
const MESSAGE_MAX: usize = 64 * 1024;
/// Ouverture : poignée de main HTTP, et connexion locale vers le relais.
const OUVERTURE: Duration = Duration::from_secs(10);
/// WebSocket ouvertes en même temps sur la passerelle, au plus (comme le
/// plafond de connexions du relais).
const WEBSOCKETS_MAX: usize = 4096;

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

    fn url(&self) -> String {
        let schema = if self.chiffree { "wss" } else { "ws" };
        format!("{schema}://{}:{}/", self.hote, self.port)
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
/// l'une ou de l'autre. Ping toutes les 30 secondes ; rien reçu pendant 90
/// secondes, message texte, trame trop grande : fin.
async fn pomper<S>(ws: WebSocketStream<S>, tcp: &mut tokio::net::TcpStream)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let (mut envoi, mut reception) = ws.split();
    let (mut lecture, mut ecriture) = tcp.split();
    let mut tampon = vec![0u8; MORCEAU];
    let mut battement = tokio::time::interval(PING);
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
                if recu.elapsed() > SANS_NOUVELLES
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
/// fournisseur ring, TLS 1.3. Gardée une fois construite.
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
        .with_protocol_versions(&[&rustls::version::TLS13])
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
                    pomper(ws, &mut pont).await;
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
            Ok((pont, pair)) if pair == attendu => return Ok((local, pont)),
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

impl Passerelle {
    /// Écoute des WebSocket sur `ecoute` ; chacune est rouverte en TCP vers
    /// `relais`, en 127.0.0.1 (la table des clients n'a de sens que là).
    /// `mandataire` : seule adresse dont l'en-tête `CF-Connecting-IP` est cru.
    pub(crate) fn demarrer(
        ecoute: SocketAddr,
        relais: SocketAddr,
        mandataire: Option<IpAddr>,
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
        execution.spawn(async move {
            loop {
                let Ok((flux, pair)) = ecouteur.accept().await else {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    continue;
                };
                let Ok(place) = places.clone().try_acquire_owned() else {
                    continue;
                };
                let clients = clients.clone();
                tokio::spawn(async move {
                    let _place = place;
                    servir(flux, pair, relais, mandataire, clients).await;
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
        pomper(ws, &mut tcp).await;
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
                Passerelle::demarrer((Ipv4Addr::LOCALHOST, 0).into(), cible, mandataire, clients)
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
