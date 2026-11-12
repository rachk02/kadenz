//! Client réseau d'un poste relié au serveur de l'entreprise (phase 2, jalon
//! F2 ; utilisé par l'application aux jalons F4 et F5).
//!
//! API bloquante, à appeler depuis un fil secondaire comme `kadenz-maj`. La
//! session est en TLS 1.3 : le poste reconnaît son serveur à l'empreinte de
//! son certificat (épinglage, sans autorité de certification), et se fait
//! reconnaître par son propre certificat (identité du poste, créée à
//! l'inscription). Les erreurs sont typées pour être traduites à l'écran.
//!
//! Les outils TLS communs (fournisseur cryptographique, identité, empreinte)
//! servent aussi au serveur.

use std::fmt;
use std::io;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use kadenz_protocole::inscription::{DECOUVERTE, PORT_PAR_DEFAUT, ServeurTrouve};
use kadenz_protocole::messages::{self, ErreurTrame, Reponse, Requete};
use kadenz_protocole::{Abonnement, Changement, Issue, Operation, VERSION};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use rustls::{ClientConfig, ClientConnection, DigitallySignedStruct, SignatureScheme, StreamOwned};
use sha2::{Digest, Sha256};

/// Nom de serveur présenté en TLS : sans valeur de confiance, le serveur est
/// reconnu à l'empreinte de son certificat.
pub const NOM_TLS: &str = "kadenz-serveur";

/// Fournisseur cryptographique commun aux postes et au serveur (ring).
pub fn fournisseur() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

/// Empreinte SHA-256 d'un certificat, en hexadécimal minuscule.
pub fn empreinte(certificat_der: &[u8]) -> String {
    Sha256::digest(certificat_der)
        .iter()
        .map(|o| format!("{o:02x}"))
        .collect()
}

/// Certificat auto-signé et sa clé privée (PKCS 8), Ed25519 : identité d'un
/// poste ou du serveur.
#[derive(Clone)]
pub struct Identite {
    pub certificat: Vec<u8>,
    pub cle: Vec<u8>,
}

impl fmt::Debug for Identite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // La clé privée n'apparaît jamais dans une trace.
        write!(f, "Identite({})", self.empreinte())
    }
}

impl Identite {
    pub fn nouvelle(nom: &str) -> Result<Identite, String> {
        let paire =
            rcgen::KeyPair::generate_for(&rcgen::PKCS_ED25519).map_err(|e| e.to_string())?;
        let parametres =
            rcgen::CertificateParams::new(vec![nom.to_string()]).map_err(|e| e.to_string())?;
        let certificat = parametres.self_signed(&paire).map_err(|e| e.to_string())?;
        Ok(Identite {
            certificat: certificat.der().to_vec(),
            cle: paire.serialize_der(),
        })
    }

    pub fn empreinte(&self) -> String {
        empreinte(&self.certificat)
    }

    pub fn chaine(&self) -> Vec<CertificateDer<'static>> {
        vec![CertificateDer::from(self.certificat.clone())]
    }

    pub fn cle_privee(&self) -> PrivateKeyDer<'static> {
        PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(self.cle.clone()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErreurClient {
    /// Adresse introuvable, serveur éteint, réseau coupé, délai dépassé.
    Injoignable(String),
    /// Le serveur ne présente pas le certificat épinglé.
    CertificatDifferent,
    VersionDifferente {
        serveur: u32,
    },
    PosteNonInscrit,
    /// Identifiant ou mot de passe refusé ; attente imposée avant le prochain essai.
    CompteRefuse {
        attente_secondes: u64,
    },
    /// Demande refusée, message à traduire : licence d'abonnement refusée,
    /// droit manquant.
    Refus {
        modele: String,
        valeurs: Vec<String>,
    },
    /// Opération refusée pendant la lecture seule (abonnement échu ou sans
    /// licence valide) : elle n'est pas gardée par le serveur, à renvoyer
    /// après le renouvellement.
    LectureSeule {
        echeance: Option<String>,
    },
    /// Requête refusée par le serveur (ordre, session, erreur du serveur).
    Rejet(String),
    /// Réponse inattendue ou illisible.
    Protocole(String),
}

impl fmt::Display for ErreurClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErreurClient::Injoignable(e) => write!(f, "serveur injoignable : {e}"),
            ErreurClient::CertificatDifferent => {
                f.write_str("le serveur ne présente pas le certificat attendu")
            }
            ErreurClient::VersionDifferente { serveur } => {
                write!(
                    f,
                    "le serveur utilise une autre version du protocole ({serveur})"
                )
            }
            ErreurClient::PosteNonInscrit => {
                f.write_str("ce poste n'est pas inscrit sur le serveur")
            }
            ErreurClient::CompteRefuse { attente_secondes } => write!(
                f,
                "identifiant ou mot de passe incorrect (nouvel essai dans {attente_secondes} s)"
            ),
            ErreurClient::Refus { modele, valeurs } => {
                let mut valeurs = valeurs.iter();
                let mut reste = modele.as_str();
                while let Some(position) = reste.find("{}") {
                    f.write_str(&reste[..position])?;
                    f.write_str(valeurs.next().map_or("{}", String::as_str))?;
                    reste = &reste[position + 2..];
                }
                f.write_str(reste)
            }
            ErreurClient::LectureSeule { .. } => {
                f.write_str("abonnement échu : Kadenz est en lecture seule")
            }
            ErreurClient::Rejet(motif) => write!(f, "requête refusée par le serveur : {motif}"),
            ErreurClient::Protocole(e) => write!(f, "réponse du serveur inattendue : {e}"),
        }
    }
}

/// Vérification par épinglage : le certificat du serveur doit avoir
/// l'empreinte attendue ; les signatures de la poignée de main sont vérifiées
/// avec sa clé.
#[derive(Debug)]
struct Epinglage {
    empreinte: String,
    fournisseur: Arc<CryptoProvider>,
    ecart: Arc<AtomicBool>,
}

impl ServerCertVerifier for Epinglage {
    fn verify_server_cert(
        &self,
        certificat: &CertificateDer<'_>,
        _intermediaires: &[CertificateDer<'_>],
        _nom: &ServerName<'_>,
        _ocsp: &[u8],
        _maintenant: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        if empreinte(certificat) == self.empreinte {
            Ok(ServerCertVerified::assertion())
        } else {
            self.ecart.store(true, Ordering::SeqCst);
            Err(rustls::Error::InvalidCertificate(
                rustls::CertificateError::ApplicationVerificationFailure,
            ))
        }
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

/// Réponse du serveur à l'ouverture de la session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bienvenue {
    pub entreprise: String,
    pub decalage_minutes: i32,
}

/// Réponse du serveur à une inscription réussie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inscrit {
    pub entreprise: String,
    /// Nom du poste, choisi par l'Admin à la création du code.
    pub nom: String,
}

/// Cherche les serveurs Kadenz du réseau local, par diffusion UDP sur le
/// port par défaut. La réponse ne donne aucune confiance : elle sert
/// seulement à trouver une adresse.
pub fn decouvrir(delai: Duration) -> Vec<(SocketAddr, ServeurTrouve)> {
    decouvrir_vers(
        SocketAddr::from(([255, 255, 255, 255], PORT_PAR_DEFAUT)),
        delai,
    )
}

/// Comme [`decouvrir`], vers une adresse donnée (essais).
pub fn decouvrir_vers(cible: SocketAddr, delai: Duration) -> Vec<(SocketAddr, ServeurTrouve)> {
    let mut trouves: Vec<(SocketAddr, ServeurTrouve)> = Vec::new();
    let Ok(socket) = UdpSocket::bind(("0.0.0.0", 0)) else {
        return trouves;
    };
    if socket.set_broadcast(true).is_err() || socket.send_to(DECOUVERTE, cible).is_err() {
        return trouves;
    }
    let fin = Instant::now() + delai;
    let mut tampon = [0u8; 512];
    while let Some(reste) = fin
        .checked_duration_since(Instant::now())
        .filter(|d| !d.is_zero())
    {
        if socket.set_read_timeout(Some(reste)).is_err() {
            break;
        }
        match socket.recv_from(&mut tampon) {
            Ok((n, source)) => {
                if let Ok(serveur) = serde_json::from_slice::<ServeurTrouve>(&tampon[..n]) {
                    let adresse = SocketAddr::new(source.ip(), serveur.port);
                    if !trouves.iter().any(|(a, _)| *a == adresse) {
                        trouves.push((adresse, serveur));
                    }
                }
            }
            Err(_) => break,
        }
    }
    trouves
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompteConnecte {
    pub compte: String,
    pub nom_complet: String,
}

pub struct Client {
    flux: StreamOwned<ClientConnection, TcpStream>,
    ecart: Arc<AtomicBool>,
}

impl Client {
    /// Ouvre une session : connexion TCP, TLS épinglé, puis `Bonjour`.
    /// `adresse` est de la forme `machine:port`.
    pub fn connecter(
        adresse: &str,
        empreinte_serveur: &str,
        identite: &Identite,
        delai: Duration,
    ) -> Result<(Client, Bienvenue), ErreurClient> {
        Client::connecter_en_version(adresse, empreinte_serveur, identite, delai, VERSION)
    }

    /// Comme [`Client::connecter`], en annonçant une autre version du
    /// protocole : pour les tests du serveur.
    #[doc(hidden)]
    pub fn connecter_en_version(
        adresse: &str,
        empreinte_serveur: &str,
        identite: &Identite,
        delai: Duration,
        version: u32,
    ) -> Result<(Client, Bienvenue), ErreurClient> {
        let mut client = Client::ouvrir(adresse, empreinte_serveur, identite, delai)?;
        match client.echanger(&Requete::Bonjour { version })? {
            Reponse::Bienvenue {
                entreprise,
                decalage_minutes,
                ..
            } => Ok((
                client,
                Bienvenue {
                    entreprise,
                    decalage_minutes,
                },
            )),
            Reponse::VersionRefusee { version } => {
                Err(ErreurClient::VersionDifferente { serveur: version })
            }
            Reponse::PosteNonInscrit => Err(ErreurClient::PosteNonInscrit),
            autre => Err(inattendue(&autre)),
        }
    }

    /// Inscrit ce poste avec un code (jalon F4) : `identite` est le
    /// certificat créé pour lui, que le serveur retient ; le poste se
    /// reconnecte ensuite avec [`Client::connecter`]. `empreinte_serveur`
    /// vient du code complet, ou de l'empreinte confirmée par l'Admin.
    pub fn inscrire(
        adresse: &str,
        empreinte_serveur: &str,
        identite: &Identite,
        secret: &str,
        delai: Duration,
    ) -> Result<Inscrit, ErreurClient> {
        let mut client = Client::ouvrir(adresse, empreinte_serveur, identite, delai)?;
        match client.echanger(&Requete::Inscription {
            version: VERSION,
            secret: secret.into(),
        })? {
            Reponse::Inscrit { entreprise, nom } => Ok(Inscrit { entreprise, nom }),
            Reponse::VersionRefusee { version } => {
                Err(ErreurClient::VersionDifferente { serveur: version })
            }
            autre => Err(inattendue(&autre)),
        }
    }

    /// Comme [`Client::ouvrir`], sans aucun message : pour les tests du
    /// serveur.
    #[doc(hidden)]
    pub fn ouvrir_sans_message(
        adresse: &str,
        empreinte_serveur: &str,
        identite: &Identite,
        delai: Duration,
    ) -> Result<Client, ErreurClient> {
        Client::ouvrir(adresse, empreinte_serveur, identite, delai)
    }

    /// Connexion TCP et TLS épinglé, sans message.
    fn ouvrir(
        adresse: &str,
        empreinte_serveur: &str,
        identite: &Identite,
        delai: Duration,
    ) -> Result<Client, ErreurClient> {
        let injoignable = |e: io::Error| ErreurClient::Injoignable(e.to_string());
        let adresses: Vec<_> = adresse.to_socket_addrs().map_err(injoignable)?.collect();
        let mut derniere = ErreurClient::Injoignable("adresse sans résultat".into());
        let mut tcp = None;
        for a in adresses {
            match TcpStream::connect_timeout(&a, delai) {
                Ok(s) => {
                    tcp = Some(s);
                    break;
                }
                Err(e) => derniere = injoignable(e),
            }
        }
        let tcp = tcp.ok_or(derniere)?;
        tcp.set_read_timeout(Some(delai)).map_err(injoignable)?;
        tcp.set_write_timeout(Some(delai)).map_err(injoignable)?;
        tcp.set_nodelay(true).map_err(injoignable)?;

        let fournisseur = fournisseur();
        let ecart = Arc::new(AtomicBool::new(false));
        let verification = Arc::new(Epinglage {
            empreinte: empreinte_serveur.to_lowercase(),
            fournisseur: fournisseur.clone(),
            ecart: ecart.clone(),
        });
        let config = ClientConfig::builder_with_provider(fournisseur)
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(|e| ErreurClient::Protocole(e.to_string()))?
            .dangerous()
            .with_custom_certificate_verifier(verification)
            .with_client_auth_cert(identite.chaine(), identite.cle_privee())
            .map_err(|e| ErreurClient::Protocole(e.to_string()))?;
        let nom =
            ServerName::try_from(NOM_TLS).map_err(|e| ErreurClient::Protocole(e.to_string()))?;
        let connexion = ClientConnection::new(Arc::new(config), nom)
            .map_err(|e| ErreurClient::Protocole(e.to_string()))?;
        Ok(Client {
            flux: StreamOwned::new(connexion, tcp),
            ecart,
        })
    }

    /// Connecte un compte de l'entreprise.
    pub fn connexion(
        &mut self,
        identifiant: &str,
        mot_de_passe: &str,
    ) -> Result<CompteConnecte, ErreurClient> {
        match self.echanger(&Requete::Connexion {
            identifiant: identifiant.into(),
            mot_de_passe: mot_de_passe.into(),
        })? {
            Reponse::Connecte {
                compte,
                nom_complet,
            } => Ok(CompteConnecte {
                compte,
                nom_complet,
            }),
            Reponse::ConnexionRefusee { attente_secondes } => {
                Err(ErreurClient::CompteRefuse { attente_secondes })
            }
            autre => Err(inattendue(&autre)),
        }
    }

    /// Envoie une opération et rend son résultat. Sans réponse (coupure), la
    /// même opération doit être renvoyée plus tard : elle n'est jamais appliquée
    /// deux fois.
    pub fn envoyer(&mut self, operation: &Operation) -> Result<Issue, ErreurClient> {
        match self.echanger(&Requete::Operation {
            operation: Box::new(operation.clone()),
        })? {
            Reponse::Resultat { issue } => Ok(issue),
            autre => Err(inattendue(&autre)),
        }
    }

    /// Changements après le rang `depuis`, et le dernier rang du serveur.
    pub fn changements(
        &mut self,
        depuis: u64,
        limite: u32,
    ) -> Result<(Vec<Changement>, u64), ErreurClient> {
        match self.echanger(&Requete::Changements { depuis, limite })? {
            Reponse::Changements {
                changements,
                dernier,
            } => Ok((changements, dernier)),
            autre => Err(inattendue(&autre)),
        }
    }

    /// État de l'abonnement de l'entreprise, jugé par le serveur.
    pub fn abonnement(&mut self) -> Result<Abonnement, ErreurClient> {
        match self.echanger(&Requete::Abonnement)? {
            Reponse::Abonnement { abonnement } => Ok(*abonnement),
            autre => Err(inattendue(&autre)),
        }
    }

    /// Installe une nouvelle licence d'abonnement (droit « Gérer les
    /// paramètres ») ; une licence refusée laisse celle en place.
    pub fn renouveler(&mut self, cle: &str) -> Result<Abonnement, ErreurClient> {
        match self.echanger(&Requete::RenouvelerAbonnement { cle: cle.into() })? {
            Reponse::Abonnement { abonnement } => Ok(*abonnement),
            autre => Err(inattendue(&autre)),
        }
    }

    /// Flux TLS de la session, pour envoyer des trames malformées : pour les
    /// tests du serveur.
    #[doc(hidden)]
    pub fn flux(&mut self) -> &mut StreamOwned<ClientConnection, TcpStream> {
        &mut self.flux
    }

    fn echanger(&mut self, requete: &Requete) -> Result<Reponse, ErreurClient> {
        let ecart = self.ecart.clone();
        let erreur_io = move |e: io::Error| {
            if ecart.load(Ordering::SeqCst) {
                ErreurClient::CertificatDifferent
            } else {
                ErreurClient::Injoignable(e.to_string())
            }
        };
        messages::ecrire(&mut self.flux, requete).map_err(&erreur_io)?;
        match messages::lire::<Reponse>(&mut self.flux) {
            Ok(Reponse::Rejet { motif }) => Err(ErreurClient::Rejet(motif)),
            Ok(Reponse::Refus { refus }) => Err(ErreurClient::Refus {
                modele: refus.modele,
                valeurs: refus.valeurs,
            }),
            Ok(Reponse::LectureSeule { echeance }) => Err(ErreurClient::LectureSeule { echeance }),
            Ok(Reponse::PosteNonInscrit) => Err(ErreurClient::PosteNonInscrit),
            Ok(r) => Ok(r),
            Err(ErreurTrame::Io(e)) => Err(erreur_io(e)),
            Err(e) => Err(ErreurClient::Protocole(e.to_string())),
        }
    }
}

fn inattendue(r: &Reponse) -> ErreurClient {
    ErreurClient::Protocole(format!("{r:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identite_et_empreinte() {
        let a = Identite::nouvelle("poste").unwrap();
        let b = Identite::nouvelle("poste").unwrap();
        assert_eq!(a.empreinte().len(), 64);
        assert_ne!(a.empreinte(), b.empreinte());
        assert!(!format!("{a:?}").contains(&format!("{:?}", a.cle)));
    }
}
