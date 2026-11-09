//! Fiches des référentiels et leurs règles de saisie (cahier des charges,
//! sections 5.2 et 5.3). Le stockage et l'aperçu d'import appliquent les mêmes
//! règles, celles de ce module.

use std::fmt;

use crate::calcul::Montant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeTiers {
    Client,
    Fournisseur,
    LesDeux,
}

impl TypeTiers {
    pub const TOUS: [TypeTiers; 3] = [
        TypeTiers::Client,
        TypeTiers::Fournisseur,
        TypeTiers::LesDeux,
    ];

    pub fn code(self) -> &'static str {
        match self {
            TypeTiers::Client => "client",
            TypeTiers::Fournisseur => "fournisseur",
            TypeTiers::LesDeux => "les_deux",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|t| t.code() == code)
    }

    pub fn est_client(self) -> bool {
        self != TypeTiers::Fournisseur
    }

    pub fn est_fournisseur(self) -> bool {
        self != TypeTiers::Client
    }
}

impl fmt::Display for TypeTiers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            TypeTiers::Client => "Client",
            TypeTiers::Fournisseur => "Fournisseur",
            TypeTiers::LesDeux => "Client et fournisseur",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sens {
    Vente,
    Achat,
    LesDeux,
}

impl Sens {
    pub const TOUS: [Sens; 3] = [Sens::Vente, Sens::Achat, Sens::LesDeux];

    pub fn code(self) -> &'static str {
        match self {
            Sens::Vente => "vente",
            Sens::Achat => "achat",
            Sens::LesDeux => "les_deux",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|s| s.code() == code)
    }

    pub fn en_vente(self) -> bool {
        self != Sens::Achat
    }

    pub fn en_achat(self) -> bool {
        self != Sens::Vente
    }
}

impl fmt::Display for Sens {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Sens::Vente => "Vente",
            Sens::Achat => "Achat",
            Sens::LesDeux => "Vente et achat",
        })
    }
}

/// Contenu modifiable d'une fiche tiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FicheTiers {
    pub nom: String,
    pub adresse: String,
    pub telephone: String,
    pub localisation: String,
    pub ifu: String,
    pub rccm: String,
    pub regime_imposition: String,
    pub type_tiers: TypeTiers,
}

impl FicheTiers {
    /// Fiche vide du type donné.
    pub fn vide(type_tiers: TypeTiers) -> Self {
        FicheTiers {
            nom: String::new(),
            adresse: String::new(),
            telephone: String::new(),
            localisation: String::new(),
            ifu: String::new(),
            rccm: String::new(),
            regime_imposition: String::new(),
            type_tiers,
        }
    }

    /// Fiche aux champs sans espaces superflus, ou la raison du refus.
    pub fn nettoyee(&self) -> Result<FicheTiers, String> {
        let t = |s: &String| s.trim().to_string();
        let f = FicheTiers {
            nom: t(&self.nom),
            adresse: t(&self.adresse),
            telephone: t(&self.telephone),
            localisation: t(&self.localisation),
            ifu: t(&self.ifu),
            rccm: t(&self.rccm),
            regime_imposition: t(&self.regime_imposition),
            type_tiers: self.type_tiers,
        };
        if f.nom.is_empty() {
            return Err("le nom du tiers est obligatoire".into());
        }
        Ok(f)
    }
}

/// Contenu modifiable d'une fiche produit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FicheProduit {
    pub designation: String,
    pub reference: String,
    /// Prix unitaire en FCFA.
    pub prix: Montant,
    pub sens: Sens,
}

impl FicheProduit {
    /// Fiche aux champs sans espaces superflus, ou la raison du refus.
    pub fn nettoyee(&self) -> Result<FicheProduit, String> {
        let f = FicheProduit {
            designation: self.designation.trim().to_string(),
            reference: self.reference.trim().to_string(),
            prix: self.prix,
            sens: self.sens,
        };
        if f.designation.is_empty() {
            return Err("la désignation est obligatoire".into());
        }
        if f.reference.is_empty() {
            return Err("la référence est obligatoire".into());
        }
        if f.prix < 0 {
            return Err("le prix ne peut pas être négatif".into());
        }
        Ok(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_et_appartenance() {
        for t in TypeTiers::TOUS {
            assert_eq!(TypeTiers::depuis_code(t.code()), Some(t));
        }
        for s in Sens::TOUS {
            assert_eq!(Sens::depuis_code(s.code()), Some(s));
        }
        assert!(TypeTiers::LesDeux.est_client() && TypeTiers::LesDeux.est_fournisseur());
        assert!(!TypeTiers::Fournisseur.est_client());
        assert!(Sens::LesDeux.en_vente() && !Sens::Achat.en_vente());
    }

    #[test]
    fn validation_des_fiches() {
        let mut t = FicheTiers::vide(TypeTiers::Client);
        t.nom = "  Alpha  ".into();
        assert_eq!(t.nettoyee().unwrap().nom, "Alpha");
        t.nom = " ".into();
        assert!(t.nettoyee().is_err());
        let p = FicheProduit {
            designation: "Pelle".into(),
            reference: " ".into(),
            prix: 10,
            sens: Sens::Vente,
        };
        assert!(p.nettoyee().is_err());
    }
}
