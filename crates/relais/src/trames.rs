//! Trames du relais : longueur sur 2 octets (gros-boutiste), puis un objet
//! JSON. Les trames en clair (poste, session) font 4 Kio au plus, celles de
//! la connexion de contrôle 16 Kio. La lecture prend exactement la longueur
//! annoncée : rien de la poignée de main TLS qui suit n'est avalé.

use std::io::{self, Read, Write};

use serde::{Deserialize, Serialize};

/// Taille maximale d'une trame en clair.
pub const TAILLE_EN_CLAIR: usize = 4096;
/// Taille maximale d'une trame de la connexion de contrôle.
pub const TAILLE_CONTROLE: usize = 16384;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "trame", rename_all = "snake_case")]
pub enum Trame {
    /// Poste distant vers relais : première trame.
    Poste {
        version: u32,
        entreprise: String,
    },
    /// Serveur vers relais, sur une nouvelle connexion : session demandée.
    Session {
        version: u32,
        session: String,
    },
    /// Relais vers poste ou serveur : aléa à signer, en base64.
    Defi {
        defi: String,
    },
    /// Poste vers relais : certificat (DER) et signature du défi, en base64.
    Preuve {
        certificat: String,
        signature: String,
    },
    /// Relais vers poste : la session TLS peut commencer.
    Relie,
    Refus {
        motif: String,
        #[serde(default)]
        valeurs: Vec<String>,
    },
    /// Serveur vers relais (contrôle) : enregistrement de l'entreprise.
    Enregistrer {
        version: u32,
        entreprise: String,
        licence: String,
        certificat: String,
        signature: String,
        postes: Vec<String>,
    },
    Enregistre,
    /// Serveur vers relais (contrôle) : empreintes des postes autorisés.
    Postes {
        postes: Vec<String>,
    },
    /// Relais vers serveur (contrôle) : ouvrir une connexion pour cette session.
    Demande {
        session: String,
    },
    Battement,
}

fn invalide(texte: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, texte.into())
}

/// Trame encodée, longueur comprise.
pub fn encoder(trame: &Trame, taille_max: usize) -> io::Result<Vec<u8>> {
    let json = serde_json::to_vec(trame)?;
    if json.len() > taille_max.min(u16::MAX as usize) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "trame trop grande",
        ));
    }
    let mut octets = Vec::with_capacity(json.len() + 2);
    octets.extend_from_slice(&(json.len() as u16).to_be_bytes());
    octets.extend_from_slice(&json);
    Ok(octets)
}

/// Écrit une trame en clair en un seul envoi.
pub fn ecrire(flux: &mut impl Write, trame: &Trame) -> io::Result<()> {
    flux.write_all(&encoder(trame, TAILLE_EN_CLAIR)?)?;
    flux.flush()
}

/// Lit une trame, longueur contrôlée avant d'allouer.
pub fn lire(flux: &mut impl Read, taille_max: usize) -> io::Result<Trame> {
    let mut entete = [0u8; 2];
    flux.read_exact(&mut entete)?;
    let longueur = u16::from_be_bytes(entete) as usize;
    if longueur > taille_max {
        return Err(invalide(format!("trame trop grande ({longueur} octets)")));
    }
    let mut corps = vec![0u8; longueur];
    flux.read_exact(&mut corps)?;
    serde_json::from_slice(&corps).map_err(|e| invalide(e.to_string()))
}

/// Retire du tampon la première trame complète, s'il y en a une.
pub(crate) fn extraire(tampon: &mut Vec<u8>, taille_max: usize) -> io::Result<Option<Trame>> {
    if tampon.len() < 2 {
        return Ok(None);
    }
    let longueur = u16::from_be_bytes([tampon[0], tampon[1]]) as usize;
    if longueur > taille_max {
        return Err(invalide(format!("trame trop grande ({longueur} octets)")));
    }
    if tampon.len() < 2 + longueur {
        return Ok(None);
    }
    let trame = serde_json::from_slice(&tampon[2..2 + longueur]);
    tampon.drain(..2 + longueur);
    trame.map(Some).map_err(|e| invalide(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lecture_exacte_sans_avaler_la_suite() {
        let mut octets = encoder(
            &Trame::Poste {
                version: 1,
                entreprise: "e".into(),
            },
            TAILLE_EN_CLAIR,
        )
        .unwrap();
        octets.extend_from_slice(b"\x16suite");
        let mut flux = io::Cursor::new(octets);
        assert_eq!(
            lire(&mut flux, TAILLE_EN_CLAIR).unwrap(),
            Trame::Poste {
                version: 1,
                entreprise: "e".into()
            }
        );
        let mut reste = Vec::new();
        flux.read_to_end(&mut reste).unwrap();
        assert_eq!(reste, b"\x16suite");
    }

    #[test]
    fn trame_trop_grande_refusee_avant_lecture() {
        let mut flux = io::Cursor::new(vec![0xff, 0xff]);
        assert!(lire(&mut flux, TAILLE_EN_CLAIR).is_err());
    }

    #[test]
    fn extraction_par_morceaux() {
        let octets = encoder(&Trame::Battement, TAILLE_CONTROLE).unwrap();
        let mut tampon = octets[..3].to_vec();
        assert_eq!(extraire(&mut tampon, TAILLE_CONTROLE).unwrap(), None);
        tampon.extend_from_slice(&octets[3..]);
        tampon.extend_from_slice(&octets);
        assert_eq!(
            extraire(&mut tampon, TAILLE_CONTROLE).unwrap(),
            Some(Trame::Battement)
        );
        assert_eq!(tampon, octets);
    }
}
