//! Relais de l'accès à distance (phase 2, jalon F8 ; spécification de la
//! synchronisation, section 7).
//!
//! Le serveur de l'entreprise ouvre lui-même une connexion de contrôle vers
//! le relais (TLS 1.3, relais épinglé) et s'y enregistre avec sa licence
//! d'entreprise. Un poste distant se présente au relais par quelques trames
//! courtes en clair, prouve qu'il détient la clé d'un poste autorisé, puis le
//! relais le relie à une connexion ouverte par le serveur et ne fait plus que
//! copier les octets. La session TLS du poste et du serveur passe de bout en
//! bout à travers le relais, qui ne peut rien en lire.
//!
//! Ce crate sert aux trois côtés : le programme `kadenz-relais` ([`relais`]),
//! le poste ([`poste`]) et le serveur ([`controle`]). Il ne dépend ni du
//! stockage ni de Windows.

pub mod controle;
mod frein;
pub mod poste;
pub mod preuve;
pub mod relais;
mod tls;
pub mod trames;

use std::fmt;

pub use tls::{IdentiteRelais, empreinte_enregistree, identite_relais};

/// Identité neuve, sans l'écrire (essais du poste et du serveur).
#[doc(hidden)]
pub use tls::nouvelle_identite;

/// Version des trames du relais.
pub const VERSION: u32 = 1;

/// Port par défaut du relais (plage 47101 à 47556 non attribuée de l'IANA,
/// à côté de celui des sessions, 47380).
pub const PORT_PAR_DEFAUT: u16 = 47381;

/// Motifs de refus du relais, rendus dans `Trame::Refus`.
pub mod motif {
    pub const SERVEUR_INJOIGNABLE: &str = "serveur_injoignable";
    pub const POSTE_NON_AUTORISE: &str = "poste_non_autorise";
    pub const TROP_DE_SESSIONS: &str = "trop_de_sessions";
    pub const VERSION: &str = "version";
    pub const LICENCE_REFUSEE: &str = "licence_refusee";
    pub const LICENCE_ECHUE: &str = "licence_echue";
    pub const PREUVE_INVALIDE: &str = "preuve_invalide";
    pub const DEJA_ENREGISTREE: &str = "deja_enregistree";
    pub const PROTOCOLE: &str = "protocole";
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErreurRelais {
    /// Adresse introuvable, relais éteint, réseau coupé, délai dépassé.
    Injoignable(String),
    /// Le relais ne présente pas le certificat épinglé (connexion de contrôle).
    CertificatDifferent,
    /// Refus du relais, avec l'un des [`motif`]s.
    Refus { motif: String, valeurs: Vec<String> },
    /// Trame inattendue ou illisible.
    Protocole(String),
}

impl fmt::Display for ErreurRelais {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErreurRelais::Injoignable(e) => write!(f, "relais injoignable : {e}"),
            ErreurRelais::CertificatDifferent => {
                f.write_str("le relais ne présente pas le certificat attendu")
            }
            ErreurRelais::Refus { motif, valeurs } if valeurs.is_empty() => {
                write!(f, "refus du relais : {motif}")
            }
            ErreurRelais::Refus { motif, valeurs } => {
                write!(f, "refus du relais : {motif} ({})", valeurs.join(", "))
            }
            ErreurRelais::Protocole(e) => write!(f, "réponse du relais inattendue : {e}"),
        }
    }
}

impl std::error::Error for ErreurRelais {}

pub(crate) fn injoignable(e: std::io::Error) -> ErreurRelais {
    ErreurRelais::Injoignable(e.to_string())
}

/// Connexion TCP vers `adresse` (`hôte:port`), délais posés.
pub(crate) fn tcp_vers(
    adresse: &str,
    delai: std::time::Duration,
) -> Result<std::net::TcpStream, ErreurRelais> {
    use std::net::{TcpStream, ToSocketAddrs};
    let adresses: Vec<_> = adresse.to_socket_addrs().map_err(injoignable)?.collect();
    let mut derniere = ErreurRelais::Injoignable("adresse sans résultat".into());
    for a in adresses {
        match TcpStream::connect_timeout(&a, delai) {
            Ok(tcp) => {
                tcp.set_read_timeout(Some(delai)).map_err(injoignable)?;
                tcp.set_write_timeout(Some(delai)).map_err(injoignable)?;
                tcp.set_nodelay(true).map_err(injoignable)?;
                return Ok(tcp);
            }
            Err(e) => derniere = injoignable(e),
        }
    }
    Err(derniere)
}
