//! Côté poste distant : se présenter au relais et obtenir une connexion
//! reliée au serveur de son entreprise, sur laquelle la session TLS se fait
//! ensuite comme en réseau local.

use std::net::TcpStream;
use std::time::Duration;

use crate::preuve::{self, ROLE_POSTE};
use crate::trames::{self, TAILLE_EN_CLAIR, Trame};
use crate::{ErreurRelais, VERSION, injoignable, tcp_vers};

fn lire(tcp: &mut TcpStream) -> Result<Trame, ErreurRelais> {
    match trames::lire(tcp, TAILLE_EN_CLAIR) {
        Ok(Trame::Refus { motif, valeurs }) => Err(ErreurRelais::Refus { motif, valeurs }),
        Ok(trame) => Ok(trame),
        Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {
            Err(ErreurRelais::Protocole(e.to_string()))
        }
        Err(e) => Err(injoignable(e)),
    }
}

/// Connexion reliée au serveur de l'entreprise, par le relais à `adresse`
/// (`hôte:port`). `certificat` et `cle` sont l'identité TLS du poste ; la
/// clé ne sort pas du poste, seule la signature du défi est envoyée.
pub fn ouvrir(
    adresse: &str,
    entreprise: &str,
    certificat: &[u8],
    cle: &[u8],
    delai: Duration,
) -> Result<TcpStream, ErreurRelais> {
    let mut tcp = tcp_vers(adresse, delai)?;
    trames::ecrire(
        &mut tcp,
        &Trame::Poste {
            version: VERSION,
            entreprise: entreprise.into(),
        },
    )
    .map_err(injoignable)?;
    let defi = match lire(&mut tcp)? {
        Trame::Defi { defi } => preuve::depuis_base64(&defi)
            .ok_or_else(|| ErreurRelais::Protocole("défi illisible".into()))?,
        autre => return Err(ErreurRelais::Protocole(format!("{autre:?}"))),
    };
    let signature = preuve::signer(cle, &preuve::message(ROLE_POSTE, entreprise, &defi))
        .map_err(ErreurRelais::Protocole)?;
    trames::ecrire(
        &mut tcp,
        &Trame::Preuve {
            certificat: preuve::base64(certificat),
            signature: preuve::base64(&signature),
        },
    )
    .map_err(injoignable)?;
    match lire(&mut tcp)? {
        Trame::Relie => Ok(tcp),
        autre => Err(ErreurRelais::Protocole(format!("{autre:?}"))),
    }
}
