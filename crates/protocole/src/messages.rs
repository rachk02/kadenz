//! Messages échangés sur la session TLS entre un poste et le serveur (jalon
//! F2) : chaque message est un objet JSON précédé de sa longueur sur 4 octets
//! (gros-boutiste). Une longueur au-delà de [`TAILLE_MAX`] est refusée avant
//! toute allocation.
//!
//! Ordre : le serveur vérifie d'abord que le poste est inscrit (certificat TLS),
//! puis le poste dit `Bonjour` avec sa version du protocole, puis connecte un
//! compte ; ensuite seulement viennent les opérations et les changements.

use std::io::{self, Read, Write};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::abonnement::{Abonnement, Refus};
use crate::{Changement, Entite, Issue, Operation, Suspendue};

/// Taille maximale d'un message, en octets.
pub const TAILLE_MAX: usize = 8 * 1024 * 1024;

/// Nombre maximal de changements par demande.
pub const CHANGEMENTS_MAX: u32 = 500;

/// Nombre maximal de lignes par demande d'état présent.
pub const LIGNES_MAX: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "requete", rename_all = "snake_case")]
pub enum Requete {
    Bonjour {
        version: u32,
    },
    Connexion {
        identifiant: String,
        mot_de_passe: String,
    },
    Operation {
        operation: Box<Operation>,
    },
    Changements {
        depuis: u64,
        limite: u32,
    },
    /// Inscription d'un poste avec un code (F4) : seule requête reçue d'un
    /// poste pas encore inscrit, à la place de `Bonjour`.
    Inscription {
        version: u32,
        secret: String,
    },
    /// État de l'abonnement de l'entreprise.
    Abonnement,
    /// Nouvelle licence d'abonnement, au droit « Gérer les paramètres ».
    RenouvelerAbonnement {
        cle: String,
    },
    /// Opérations suspendues que le compte connecté peut trancher ou
    /// décider (F6).
    Suspendues,
    /// État présent de lignes désignées, pour défaire dans la copie une
    /// écriture provisoire (F6) ; au plus [`LIGNES_MAX`].
    Lignes {
        lignes: Vec<(Entite, String)>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reponse", rename_all = "snake_case")]
pub enum Reponse {
    /// Réponse à `Bonjour` : identifiant de l'entreprise et décalage horaire du
    /// serveur, fuseau unique de l'entreprise (décision 37).
    Bienvenue {
        version: u32,
        entreprise: String,
        decalage_minutes: i32,
    },
    /// Version du protocole différente : la connexion est fermée.
    VersionRefusee {
        version: u32,
    },
    /// Certificat du poste inconnu ou retiré : la connexion est fermée.
    PosteNonInscrit,
    Connecte {
        compte: String,
        nom_complet: String,
    },
    /// Identifiant ou mot de passe refusé ; après plusieurs échecs, le serveur
    /// impose une attente avant le prochain essai.
    ConnexionRefusee {
        attente_secondes: u64,
    },
    Resultat {
        issue: Issue,
    },
    Changements {
        changements: Vec<Changement>,
        /// Rang du dernier changement de la base au moment de la réponse.
        dernier: u64,
    },
    /// Poste inscrit : il se reconnecte avec le même certificat.
    Inscrit {
        entreprise: String,
        nom: String,
    },
    Abonnement {
        abonnement: Box<Abonnement>,
    },
    /// Opération refusée pendant la lecture seule (abonnement échu ou sans
    /// licence valide) : elle n'est pas gardée, le poste la renverra après le
    /// renouvellement.
    LectureSeule {
        echeance: Option<String>,
    },
    Suspendues {
        suspendues: Vec<Suspendue>,
    },
    /// État présent de chaque ligne demandée, `Supprime` pour une ligne
    /// inconnue du serveur ; `seq` est le rang du dernier changement de la
    /// ligne (0 si aucun) et ne déplace pas le dernier rang reçu du poste.
    Lignes {
        lignes: Vec<Changement>,
    },
    /// Demande refusée, message à traduire (licence refusée, droit manquant).
    Refus {
        refus: Refus,
    },
    /// Requête refusée par le protocole (ordre, compte ou poste qui ne
    /// correspond pas à la session, erreur du serveur).
    Rejet {
        motif: String,
    },
}

#[derive(Debug)]
pub enum ErreurTrame {
    Io(io::Error),
    /// Longueur annoncée au-delà de [`TAILLE_MAX`].
    TropGrand(usize),
    Illisible(String),
}

impl std::fmt::Display for ErreurTrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErreurTrame::Io(e) => write!(f, "{e}"),
            ErreurTrame::TropGrand(n) => write!(f, "message trop grand ({n} octets)"),
            ErreurTrame::Illisible(e) => write!(f, "message illisible : {e}"),
        }
    }
}

impl From<io::Error> for ErreurTrame {
    fn from(e: io::Error) -> Self {
        ErreurTrame::Io(e)
    }
}

/// Écrit un message et vide le flux.
pub fn ecrire<T: Serialize>(flux: &mut impl Write, message: &T) -> io::Result<()> {
    let json = serde_json::to_vec(message)?;
    if json.len() > TAILLE_MAX {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "message trop grand",
        ));
    }
    flux.write_all(&(json.len() as u32).to_be_bytes())?;
    flux.write_all(&json)?;
    flux.flush()
}

/// Lit un message ; la longueur est contrôlée avant d'allouer.
pub fn lire<T: DeserializeOwned>(flux: &mut impl Read) -> Result<T, ErreurTrame> {
    lire_borne(flux, TAILLE_MAX)
}

/// Comme [`lire`], avec une taille maximale plus petite (poste pas encore
/// inscrit).
pub fn lire_borne<T: DeserializeOwned>(
    flux: &mut impl Read,
    taille_max: usize,
) -> Result<T, ErreurTrame> {
    let mut entete = [0u8; 4];
    flux.read_exact(&mut entete)?;
    let longueur = u32::from_be_bytes(entete) as usize;
    if longueur > taille_max.min(TAILLE_MAX) {
        return Err(ErreurTrame::TropGrand(longueur));
    }
    let mut corps = vec![0u8; longueur];
    flux.read_exact(&mut corps)?;
    serde_json::from_slice(&corps).map_err(|e| ErreurTrame::Illisible(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trame_aller_retour() {
        let mut tampon = Vec::new();
        let r = Requete::Changements {
            depuis: 7,
            limite: 100,
        };
        ecrire(&mut tampon, &r).unwrap();
        assert_eq!(lire::<Requete>(&mut tampon.as_slice()).unwrap(), r);
    }

    #[test]
    fn trame_trop_grande_refusee_avant_lecture() {
        let mut tampon = ((TAILLE_MAX + 1) as u32).to_be_bytes().to_vec();
        tampon.extend_from_slice(b"{}");
        assert!(matches!(
            lire::<Requete>(&mut tampon.as_slice()),
            Err(ErreurTrame::TropGrand(_))
        ));
        let mut illisible = 2u32.to_be_bytes().to_vec();
        illisible.extend_from_slice(b"zz");
        assert!(matches!(
            lire::<Requete>(&mut illisible.as_slice()),
            Err(ErreurTrame::Illisible(_))
        ));
    }
}
