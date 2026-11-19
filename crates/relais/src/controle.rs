//! Côté serveur de l'entreprise : connexion de contrôle vers le relais
//! (enregistrement, liste des postes autorisés, battements, demandes de
//! session), et connexions de session ouvertes à la demande du relais.

use std::io;
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection};

use crate::preuve::{self, ROLE_SERVEUR};
use crate::tls::{Canal, Epinglage, NOM_TLS, fournisseur};
use crate::trames::{self, Trame};
use crate::{ErreurRelais, VERSION, injoignable, tcp_vers};

/// Battement envoyé sur la connexion de contrôle, dans les deux sens.
pub const BATTEMENT: Duration = Duration::from_secs(15);
/// Silence au-delà duquel la connexion de contrôle est tenue pour morte.
pub const SILENCE: Duration = Duration::from_secs(45);

/// Ce qu'il faut au serveur pour s'enregistrer.
pub struct Enregistrement<'a> {
    /// Adresse `hôte:port` du relais.
    pub relais: &'a str,
    /// Empreinte du certificat du relais (épinglage).
    pub empreinte_relais: &'a str,
    pub entreprise: &'a str,
    /// Licence d'entreprise (`KADENZ-E-...`).
    pub licence: &'a str,
    /// Certificat TLS du serveur (DER) et sa clé (PKCS 8).
    pub certificat: &'a [u8],
    pub cle: &'a [u8],
    /// Empreintes des postes autorisés à distance.
    pub postes: Vec<String>,
}

pub struct Controle {
    canal: Canal,
    dernier_envoi: Instant,
    vu: Instant,
    battement: Duration,
    silence: Duration,
}

fn erreur_io(e: io::Error) -> ErreurRelais {
    if e.kind() == io::ErrorKind::InvalidData {
        ErreurRelais::Protocole(e.to_string())
    } else {
        injoignable(e)
    }
}

/// Connexion TLS 1.3 vers le relais, certificat épinglé, poignée de main faite.
fn canal_epingle(relais: &str, empreinte: &str, delai: Duration) -> Result<Canal, ErreurRelais> {
    let tcp = tcp_vers(relais, delai)?;
    let fournisseur = fournisseur();
    let ecart = Arc::new(AtomicBool::new(false));
    let verification = Arc::new(Epinglage {
        empreinte: empreinte.to_lowercase(),
        fournisseur: fournisseur.clone(),
        ecart: ecart.clone(),
    });
    let config = ClientConfig::builder_with_provider(fournisseur)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|e| ErreurRelais::Protocole(e.to_string()))?
        .dangerous()
        .with_custom_certificate_verifier(verification)
        .with_no_client_auth();
    let nom = ServerName::try_from(NOM_TLS).map_err(|e| ErreurRelais::Protocole(e.to_string()))?;
    let conn = ClientConnection::new(Arc::new(config), nom)
        .map_err(|e| ErreurRelais::Protocole(e.to_string()))?;
    let mut canal = Canal::nouveau(conn, tcp);
    if let Err(err) = canal.poignee(delai) {
        return Err(if ecart.load(Ordering::SeqCst) {
            ErreurRelais::CertificatDifferent
        } else {
            erreur_io(err)
        });
    }
    Ok(canal)
}

/// Contrôle de santé du relais (sonde du conteneur, lot R1) : connexion,
/// poignée de main TLS avec le certificat épinglé, puis réception du défi.
/// Le relais a donc accepté la connexion, lancé son fil et répondu ; aucune
/// entreprise n'est enregistrée, la connexion est fermée aussitôt.
pub fn verifier_sante(relais: &str, empreinte: &str, delai: Duration) -> Result<(), ErreurRelais> {
    let mut canal = canal_epingle(relais, empreinte, delai)?;
    match canal.attendre(delai).map_err(erreur_io)? {
        Trame::Defi { .. } => Ok(()),
        Trame::Refus { motif, valeurs } => Err(ErreurRelais::Refus { motif, valeurs }),
        autre => Err(ErreurRelais::Protocole(format!("{autre:?}"))),
    }
}

impl Controle {
    /// Ouvre la connexion de contrôle et enregistre l'entreprise.
    pub fn enregistrer(e: &Enregistrement, delai: Duration) -> Result<Controle, ErreurRelais> {
        let mut canal = canal_epingle(e.relais, e.empreinte_relais, delai)?;
        let defi = match canal.attendre(delai).map_err(erreur_io)? {
            Trame::Defi { defi } => preuve::depuis_base64(&defi)
                .ok_or_else(|| ErreurRelais::Protocole("défi illisible".into()))?,
            Trame::Refus { motif, valeurs } => return Err(ErreurRelais::Refus { motif, valeurs }),
            autre => return Err(ErreurRelais::Protocole(format!("{autre:?}"))),
        };
        let signature = preuve::signer(e.cle, &preuve::message(ROLE_SERVEUR, e.entreprise, &defi))
            .map_err(ErreurRelais::Protocole)?;
        canal
            .envoyer(&Trame::Enregistrer {
                version: VERSION,
                entreprise: e.entreprise.into(),
                licence: e.licence.into(),
                certificat: preuve::base64(e.certificat),
                signature: preuve::base64(&signature),
                postes: e.postes.clone(),
            })
            .map_err(erreur_io)?;
        match canal.attendre(delai).map_err(erreur_io)? {
            Trame::Enregistre => {}
            Trame::Refus { motif, valeurs } => return Err(ErreurRelais::Refus { motif, valeurs }),
            autre => return Err(ErreurRelais::Protocole(format!("{autre:?}"))),
        }
        let maintenant = Instant::now();
        Ok(Controle {
            canal,
            dernier_envoi: maintenant,
            vu: maintenant,
            battement: BATTEMENT,
            silence: SILENCE,
        })
    }

    /// Autres délais de battement et de silence (essais).
    #[doc(hidden)]
    pub fn regler_battement(&mut self, battement: Duration, silence: Duration) {
        self.battement = battement;
        self.silence = silence;
    }

    /// Attend `attente` au plus une demande de session du relais, et envoie
    /// le battement quand il est dû. `Ok(None)` : rien demandé. Une erreur
    /// veut dire que la connexion est perdue (relais coupé, silence).
    pub fn attendre(&mut self, attente: Duration) -> Result<Option<String>, ErreurRelais> {
        let fin = Instant::now() + attente;
        loop {
            if self.dernier_envoi.elapsed() >= self.battement {
                self.canal.envoyer(&Trame::Battement).map_err(erreur_io)?;
                self.dernier_envoi = Instant::now();
            }
            if self.vu.elapsed() > self.silence {
                return Err(ErreurRelais::Injoignable("relais silencieux".into()));
            }
            let reste = fin
                .saturating_duration_since(Instant::now())
                .min(self.battement);
            match self.canal.recevoir(reste).map_err(erreur_io)? {
                Some(Trame::Demande { session }) => {
                    self.vu = Instant::now();
                    return Ok(Some(session));
                }
                Some(Trame::Battement) => self.vu = Instant::now(),
                Some(Trame::Refus { motif, valeurs }) => {
                    return Err(ErreurRelais::Refus { motif, valeurs });
                }
                Some(autre) => return Err(ErreurRelais::Protocole(format!("{autre:?}"))),
                None => {}
            }
            if Instant::now() >= fin {
                return Ok(None);
            }
        }
    }

    /// Envoie la liste des postes autorisés, après un changement.
    pub fn envoyer_postes(&mut self, postes: Vec<String>) -> Result<(), ErreurRelais> {
        self.canal
            .envoyer(&Trame::Postes { postes })
            .map_err(erreur_io)?;
        self.dernier_envoi = Instant::now();
        Ok(())
    }

    /// Ferme la connexion de contrôle.
    pub fn fermer(self) {
        let _ = self.canal.tcp.shutdown(std::net::Shutdown::Both);
    }
}

/// Connexion de session ouverte à la demande du relais : le relais la relie
/// au poste qui attend, et la session TLS commence aussitôt (le serveur y
/// est le côté serveur TLS).
pub fn ouvrir_session(
    relais: &str,
    session: &str,
    delai: Duration,
) -> Result<TcpStream, ErreurRelais> {
    let mut tcp = tcp_vers(relais, delai)?;
    trames::ecrire(
        &mut tcp,
        &Trame::Session {
            version: VERSION,
            session: session.into(),
        },
    )
    .map_err(injoignable)?;
    Ok(tcp)
}
