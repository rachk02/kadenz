//! Types des référentiels : nature d'un tiers, sens d'utilisation d'un produit
//! (cahier des charges, sections 5.2 et 5.3).

use std::fmt;

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
}
