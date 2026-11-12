//! État de l'abonnement de l'entreprise, rendu par le serveur aux postes
//! (phase 2, jalon F3 ; décisions 6, 20, 21, 25, 26 et 29 ; écrans, section 7).
//!
//! Le serveur seul juge de l'échéance, avec sa propre date, qui ne recule
//! jamais : l'heure du poste n'entre pas en compte.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Abonnement {
    pub statut: Statut,
    /// Date retenue par le serveur, `AAAA-MM-JJ` ; elle ne recule jamais.
    pub aujourdhui: String,
    /// Échéance moins aujourd'hui : 0 le jour de l'échéance, négatif après ;
    /// `None` sans licence valide.
    pub jours_restants: Option<i64>,
    pub avertissement: Option<Avertissement>,
    /// Postes inscrits sur le serveur.
    pub postes_utilises: u32,
    /// Licence en place (aussi quand elle est échue).
    pub licence: Option<LicenceAbonnement>,
    /// Raison du refus de la licence enregistrée (statut `Invalide`).
    pub erreur: Option<Refus>,
}

impl Abonnement {
    /// Seul point de décision de la lecture seule (décisions 6 et 20) :
    /// consultation, impression et export restent possibles, la saisie et la
    /// validation sont bloquées.
    pub fn lecture_seule(&self) -> bool {
        self.statut.lecture_seule()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Statut {
    /// Aucune licence enregistrée.
    Absent,
    /// Licence enregistrée mais refusée (signature, autre entreprise...).
    Invalide,
    Actif,
    /// Échéance passée : lecture seule dès le lendemain, sans délai de grâce
    /// (décision 21).
    Echu,
}

impl Statut {
    /// Absent et Invalide sont aussi en lecture seule (décision 40).
    pub fn lecture_seule(self) -> bool {
        self != Statut::Actif
    }
}

/// Avertissement avant l'échéance (décisions 25 et 26).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Avertissement {
    pub palier: Palier,
    /// Faux : Admin seul ; vrai : tous les comptes.
    pub tous_les_comptes: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Palier {
    /// De 30 à 8 jours restants : Admin seul.
    Trente,
    /// De 7 à 2 jours restants : tous les comptes (seuil proposé, décision 26
    /// « à confirmer »).
    Sept,
    /// La veille et le jour même de l'échéance : tous les comptes.
    Dernier,
}

impl Avertissement {
    /// Avertissement dû pour un nombre de jours restants ; aucun au-delà de 30
    /// jours ni après l'échéance (lecture seule).
    pub fn pour(jours_restants: i64) -> Option<Avertissement> {
        let palier = match jours_restants {
            0..=1 => Palier::Dernier,
            2..=7 => Palier::Sept,
            8..=30 => Palier::Trente,
            _ => return None,
        };
        Some(Avertissement {
            palier,
            tous_les_comptes: palier != Palier::Trente,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicenceAbonnement {
    /// Identifiant de l'entreprise.
    pub entreprise: String,
    pub client: String,
    pub formule: Formule,
    /// Postes permis.
    pub postes: u32,
    /// Codes des modules activés.
    pub modules: Vec<String>,
    /// Premier jour couvert, `AAAA-MM-JJ`.
    pub debut: String,
    /// Dernier jour couvert, `AAAA-MM-JJ`.
    pub echeance: String,
    pub emetteur: Emetteur,
    pub emise_le: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Formule {
    Mensuelle,
    Annuelle,
}

impl Formule {
    pub fn code(self) -> &'static str {
        match self {
            Formule::Mensuelle => "mensuelle",
            Formule::Annuelle => "annuelle",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Formule> {
        match code {
            "mensuelle" => Some(Formule::Mensuelle),
            "annuelle" => Some(Formule::Annuelle),
            _ => None,
        }
    }
}

/// Qui a émis la licence (décision 29).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Emetteur {
    Editeur,
    Revendeur { identifiant: String, nom: String },
}

/// Message de refus à traduire : modèle fixe et valeurs, comme
/// [`crate::Issue::Refusee`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Refus {
    pub modele: String,
    pub valeurs: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paliers() {
        assert_eq!(Avertissement::pour(31), None);
        assert_eq!(Avertissement::pour(30).unwrap().palier, Palier::Trente);
        assert!(!Avertissement::pour(8).unwrap().tous_les_comptes);
        assert_eq!(Avertissement::pour(7).unwrap().palier, Palier::Sept);
        assert!(Avertissement::pour(2).unwrap().tous_les_comptes);
        assert_eq!(Avertissement::pour(1).unwrap().palier, Palier::Dernier);
        assert_eq!(Avertissement::pour(0).unwrap().palier, Palier::Dernier);
        assert_eq!(Avertissement::pour(-1), None);
        assert!(Statut::Echu.lecture_seule() && !Statut::Actif.lecture_seule());
    }
}
