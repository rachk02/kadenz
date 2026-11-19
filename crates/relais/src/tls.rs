//! Outils TLS du relais : identité du relais, épinglage côté serveur, et
//! canal de trames sur une connexion TLS, lu par petites attentes pour
//! mêler réception, battements et demandes sur un seul fil.

use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{Connection, DigitallySignedStruct, SignatureScheme};

use crate::preuve::empreinte;
use crate::trames::{self, Trame};

/// Nom présenté en TLS par le relais ; sans valeur de confiance, le relais
/// est reconnu à l'empreinte de son certificat.
pub(crate) const NOM_TLS: &str = "kadenz-relais";
const NOM_CERTIFICAT: &str = "relais.crt";
const NOM_CLE: &str = "relais.cle";

pub(crate) fn fournisseur() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

/// Certificat auto-signé du relais et sa clé privée (PKCS 8, Ed25519).
#[derive(Clone)]
pub struct IdentiteRelais {
    pub certificat: Vec<u8>,
    pub cle: Vec<u8>,
}

impl std::fmt::Debug for IdentiteRelais {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // La clé privée n'apparaît jamais dans une trace.
        write!(f, "IdentiteRelais({})", self.empreinte())
    }
}

impl IdentiteRelais {
    pub fn empreinte(&self) -> String {
        empreinte(&self.certificat)
    }
}

pub fn nouvelle_identite(nom: &str) -> Result<IdentiteRelais, String> {
    let paire = rcgen::KeyPair::generate_for(&rcgen::PKCS_ED25519).map_err(|e| e.to_string())?;
    let parametres =
        rcgen::CertificateParams::new(vec![nom.to_string()]).map_err(|e| e.to_string())?;
    let certificat = parametres.self_signed(&paire).map_err(|e| e.to_string())?;
    Ok(IdentiteRelais {
        certificat: certificat.der().to_vec(),
        cle: paire.serialize_der(),
    })
}

/// Identité du relais dans son dossier de données, créée au premier
/// lancement, quand ni le certificat ni la clé n'existent. Si un seul des deux
/// est là (restauration partielle), le relais refuse de démarrer au lieu d'en
/// créer une autre : une nouvelle identité change l'empreinte réglée sur les
/// serveurs des entreprises, qui ne pourraient plus s'y connecter (lot R1).
/// Le dossier est restreint à son propriétaire (700) : il garde la clé et des
/// traces qui nomment les entreprises et les postes.
pub fn identite_relais(dossier: &Path) -> Result<IdentiteRelais, String> {
    std::fs::create_dir_all(dossier).map_err(|e| format!("{}: {e}", dossier.display()))?;
    restreindre_dossier(dossier)?;
    let chemin_certificat = dossier.join(NOM_CERTIFICAT);
    let chemin_cle = dossier.join(NOM_CLE);
    match (chemin_certificat.exists(), chemin_cle.exists()) {
        (true, true) => {
            let certificat = std::fs::read(&chemin_certificat)
                .map_err(|e| format!("{} : {e}", chemin_certificat.display()))?;
            let cle = std::fs::read(&chemin_cle)
                .map_err(|e| format!("{} : {e}", chemin_cle.display()))?;
            Ok(IdentiteRelais { certificat, cle })
        }
        (true, false) => Err(format!(
            "certificat du relais présent mais clé absente ({}) : restaurez la clé \
             sauvegardée avec ce certificat ; le relais ne crée pas d'autre identité, \
             qui changerait l'empreinte réglée sur les serveurs",
            chemin_cle.display()
        )),
        (false, true) => Err(format!(
            "clé du relais présente mais certificat absent ({}) : restaurez le certificat \
             sauvegardé avec cette clé ; si le relais n'a jamais servi (aucune empreinte \
             donnée), supprimez la clé pour créer une nouvelle identité",
            chemin_certificat.display()
        )),
        (false, false) => {
            let identite = nouvelle_identite(NOM_TLS)?;
            ecrire_cle(&chemin_cle, &identite.cle)?;
            std::fs::write(&chemin_certificat, &identite.certificat)
                .map_err(|e| format!("{} : {e}", chemin_certificat.display()))?;
            Ok(identite)
        }
    }
}

/// Empreinte du certificat déjà présent dans le dossier, sans rien créer
/// (sous-commande de santé).
pub fn empreinte_enregistree(dossier: &Path) -> Result<String, String> {
    let chemin = dossier.join(NOM_CERTIFICAT);
    std::fs::read(&chemin)
        .map(|c| empreinte(&c))
        .map_err(|e| format!("{} : {e}", chemin.display()))
}

#[cfg(unix)]
fn restreindre_dossier(dossier: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(dossier, std::fs::Permissions::from_mode(0o700))
        .map_err(|e| format!("{} : droits 700 impossibles : {e}", dossier.display()))
}

#[cfg(not(unix))]
fn restreindre_dossier(_dossier: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn ecrire_cle(chemin: &Path, cle: &[u8]) -> Result<(), String> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut fichier = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(chemin)
        .map_err(|e| e.to_string())?;
    fichier.write_all(cle).map_err(|e| e.to_string())
}

#[cfg(not(unix))]
fn ecrire_cle(chemin: &Path, cle: &[u8]) -> Result<(), String> {
    std::fs::write(chemin, cle).map_err(|e| e.to_string())
}

/// Vérification par épinglage du certificat du relais.
#[derive(Debug)]
pub(crate) struct Epinglage {
    pub(crate) empreinte: String,
    pub(crate) fournisseur: Arc<CryptoProvider>,
    pub(crate) ecart: Arc<AtomicBool>,
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

fn delai_depasse() -> io::Error {
    io::Error::new(io::ErrorKind::TimedOut, "délai dépassé")
}

/// Trames de contrôle sur une connexion TLS.
pub(crate) struct Canal {
    conn: Connection,
    pub(crate) tcp: TcpStream,
    tampon: Vec<u8>,
}

impl Canal {
    pub(crate) fn nouveau(conn: impl Into<Connection>, tcp: TcpStream) -> Canal {
        Canal {
            conn: conn.into(),
            tcp,
            tampon: Vec::new(),
        }
    }

    pub(crate) fn poignee(&mut self, delai: Duration) -> io::Result<()> {
        let fin = Instant::now() + delai;
        while self.conn.is_handshaking() {
            let reste = fin.saturating_duration_since(Instant::now());
            if reste.is_zero() {
                return Err(delai_depasse());
            }
            self.tcp.set_read_timeout(Some(reste))?;
            self.tcp.set_write_timeout(Some(reste))?;
            self.conn.complete_io(&mut self.tcp)?;
        }
        Ok(())
    }

    fn vider(&mut self) -> io::Result<()> {
        self.tcp.set_write_timeout(Some(Duration::from_secs(10)))?;
        while self.conn.wants_write() {
            self.conn.write_tls(&mut self.tcp)?;
        }
        Ok(())
    }

    pub(crate) fn envoyer(&mut self, trame: &Trame) -> io::Result<()> {
        let octets = trames::encoder(trame, trames::TAILLE_CONTROLE)?;
        self.conn.writer().write_all(&octets)?;
        self.vider()
    }

    /// Attend une trame `attente` au plus ; `None` si rien n'est arrivé.
    pub(crate) fn recevoir(&mut self, attente: Duration) -> io::Result<Option<Trame>> {
        if let Some(trame) = trames::extraire(&mut self.tampon, trames::TAILLE_CONTROLE)? {
            return Ok(Some(trame));
        }
        let fin = Instant::now() + attente;
        loop {
            let reste = fin.saturating_duration_since(Instant::now());
            if reste.is_zero() {
                return Ok(None);
            }
            self.tcp.set_read_timeout(Some(reste))?;
            match self.conn.read_tls(&mut self.tcp) {
                Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
                Ok(_) => {}
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                    ) =>
                {
                    return Ok(None);
                }
                Err(e) => return Err(e),
            }
            let etat = self
                .conn
                .process_new_packets()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            self.vider()?;
            let n = etat.plaintext_bytes_to_read();
            if n > 0 {
                let debut = self.tampon.len();
                self.tampon.resize(debut + n, 0);
                self.conn.reader().read_exact(&mut self.tampon[debut..])?;
            } else if etat.peer_has_closed() {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }
            if let Some(trame) = trames::extraire(&mut self.tampon, trames::TAILLE_CONTROLE)? {
                return Ok(Some(trame));
            }
        }
    }

    /// Attend une trame `delai` au plus ; rien d'arrivé est une erreur.
    pub(crate) fn attendre(&mut self, delai: Duration) -> io::Result<Trame> {
        self.recevoir(delai)?.ok_or_else(delai_depasse)
    }
}
