//! Stock d'un produit suivi : coût moyen pondéré en entiers (décisions 64 à
//! 67 ; `docs/spec-stock.md`, sections 3.3 et 3.4).
//!
//! Un produit garde sa quantité (millièmes) et sa valeur (FCFA), jamais un
//! coût unitaire arrondi, plus son dernier coût unitaire connu, qui sert à
//! valoriser une sortie à découvert. La sortie qui vide le stock prend toute
//! la valeur restante : la valeur ne dérive pas au fil des arrondis.

use std::fmt;

use crate::calcul::{
    Montant, Pourcentage, Quantite, UNE_UNITE, appliquer_pourcentage, arrondir_division,
    total_ligne,
};

/// Nature d'un mouvement de stock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NatureMouvement {
    /// Réception enregistrée sur un bon de commande (ou sa correction).
    Reception,
    /// Confirmation d'un bordereau de livraison.
    Livraison,
    /// Saisie à la main, avec un motif.
    Ajustement,
    /// Écart constaté à la validation d'un inventaire.
    Inventaire,
}

impl NatureMouvement {
    pub const TOUTES: [NatureMouvement; 4] = [
        NatureMouvement::Reception,
        NatureMouvement::Livraison,
        NatureMouvement::Ajustement,
        NatureMouvement::Inventaire,
    ];

    pub fn code(self) -> &'static str {
        match self {
            NatureMouvement::Reception => "reception",
            NatureMouvement::Livraison => "livraison",
            NatureMouvement::Ajustement => "ajustement",
            NatureMouvement::Inventaire => "inventaire",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|n| n.code() == code)
    }
}

impl fmt::Display for NatureMouvement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            NatureMouvement::Reception => "Réception",
            NatureMouvement::Livraison => "Livraison",
            NatureMouvement::Ajustement => "Ajustement",
            NatureMouvement::Inventaire => "Inventaire",
        })
    }
}

/// Stock d'un produit après un mouvement.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stock {
    /// Quantité en millièmes, négative après une sortie à découvert.
    pub quantite: Quantite,
    /// Valeur en FCFA, négative avec la quantité.
    pub valeur: Montant,
    /// Dernier coût unitaire connu (FCFA par unité), recalculé quand la
    /// quantité est positive après une entrée ; nul avant toute entrée.
    pub dernier_cout: Montant,
}

impl Stock {
    /// Coût moyen unitaire du moment (FCFA par unité) : le dernier coût connu
    /// quand la quantité n'est pas positive.
    pub fn cout_moyen(&self) -> Montant {
        if self.quantite > 0 {
            arrondir_division(
                self.valeur as i128 * UNE_UNITE as i128,
                self.quantite as i128,
            )
        } else {
            self.dernier_cout
        }
    }

    /// Entrée de `quantite` (positive) pour une valeur `valeur`.
    pub fn entree(&self, quantite: Quantite, valeur: Montant) -> Stock {
        debug_assert!(quantite >= 0);
        let mut s = Stock {
            quantite: self.quantite + quantite,
            valeur: self.valeur + valeur,
            dernier_cout: self.dernier_cout,
        };
        if s.quantite > 0 {
            s.dernier_cout = s.cout_moyen();
        }
        s
    }

    /// Valeur d'une sortie de `quantite` (positive) au coût moyen : la part
    /// en stock prend sa part de la valeur (toute la valeur si elle vide le
    /// stock), l'excédent sort au dernier coût connu.
    pub fn valeur_sortie(&self, quantite: Quantite) -> Montant {
        debug_assert!(quantite >= 0);
        if quantite == 0 {
            return 0;
        }
        if quantite < self.quantite {
            return arrondir_division(
                self.valeur as i128 * quantite as i128,
                self.quantite as i128,
            );
        }
        let en_stock = self.quantite.max(0);
        let valeur_en_stock = if en_stock > 0 { self.valeur } else { 0 };
        valeur_en_stock + total_ligne(self.dernier_cout, quantite - en_stock)
    }

    /// Sortie de `quantite` (positive) : le stock après, et la valeur sortie.
    pub fn sortie(&self, quantite: Quantite) -> (Stock, Montant) {
        let valeur = self.valeur_sortie(quantite);
        (
            Stock {
                quantite: self.quantite - quantite,
                valeur: self.valeur - valeur,
                dernier_cout: self.dernier_cout,
            },
            valeur,
        )
    }

    /// Sortie valorisée à une valeur imposée (correction d'une réception à
    /// la baisse, qui reprend son coût d'entrée).
    pub fn sortie_a(&self, quantite: Quantite, valeur: Montant) -> Stock {
        let mut s = Stock {
            quantite: self.quantite - quantite,
            valeur: self.valeur - valeur,
            dernier_cout: self.dernier_cout,
        };
        if s.quantite > 0 {
            s.dernier_cout = s.cout_moyen();
        }
        s
    }
}

/// Coût d'entrée d'une réception de `quantite` sur une ligne de prix `prix`
/// d'un bon de commande de remise `remise` : hors TVA, remise du document
/// déduite, retenue non déduite (`docs/spec-stock.md`, 3.3).
pub fn cout_entree(prix: Montant, quantite: Quantite, remise: Pourcentage) -> Montant {
    let brut = total_ligne(prix, quantite);
    brut - appliquer_pourcentage(brut, remise)
}

#[cfg(test)]
mod tests {
    use super::*;

    const U: Quantite = UNE_UNITE;

    #[test]
    fn codes() {
        for n in NatureMouvement::TOUTES {
            assert_eq!(NatureMouvement::depuis_code(n.code()), Some(n));
        }
    }

    #[test]
    fn cout_moyen_sur_plusieurs_entrees() {
        // 10 à 1 000, puis 5 à 1 300 : 16 500 pour 15, soit 1 100.
        let s = Stock::default().entree(10 * U, 10_000).entree(5 * U, 6_500);
        assert_eq!(s.quantite, 15 * U);
        assert_eq!(s.valeur, 16_500);
        assert_eq!(s.cout_moyen(), 1_100);
        assert_eq!(s.dernier_cout, 1_100);
    }

    #[test]
    fn sortie_qui_vide_le_stock_sans_reste() {
        // 3 unités pour 1 000 : 333,33 l'unité ; trois sorties d'une unité.
        let s = Stock::default().entree(3 * U, 1_000);
        let (s, v1) = s.sortie(U);
        let (s, v2) = s.sortie(U);
        let (s, v3) = s.sortie(U);
        assert_eq!((v1, v2, v3), (333, 334, 333));
        assert_eq!(v1 + v2 + v3, 1_000);
        assert_eq!(s.quantite, 0);
        assert_eq!(s.valeur, 0);
    }

    #[test]
    fn stock_negatif_puis_entree() {
        // 2 à 500 ; sortie de 5 : 2 pour 1 000, 3 au dernier coût, 1 500.
        let s = Stock::default().entree(2 * U, 1_000);
        let (s, v) = s.sortie(5 * U);
        assert_eq!(v, 2_500);
        assert_eq!((s.quantite, s.valeur), (-3 * U, -1_500));
        // Nouvelle sortie à découvert : au dernier coût.
        let (s, v) = s.sortie(U);
        assert_eq!(v, 500);
        assert_eq!((s.quantite, s.valeur), (-4 * U, -2_000));
        // Entrée de 10 à 600 : le négatif est comblé, 6 pour 4 000.
        let s = s.entree(10 * U, 6_000);
        assert_eq!((s.quantite, s.valeur), (6 * U, 4_000));
        assert_eq!(s.cout_moyen(), 667);
    }

    #[test]
    fn sortie_a_decouvert_sans_aucune_entree() {
        let (s, v) = Stock::default().sortie(2 * U);
        assert_eq!(v, 0);
        assert_eq!((s.quantite, s.valeur), (-2 * U, 0));
    }

    #[test]
    fn quantite_fractionnaire() {
        // 2,5 unités pour 2 500, sortie de 0,4 : 400.
        let s = Stock::default().entree(2_500, 2_500);
        let (s, v) = s.sortie(400);
        assert_eq!(v, 400);
        assert_eq!((s.quantite, s.valeur), (2_100, 2_100));
    }

    #[test]
    fn cout_d_entree_remise_deduite() {
        // 4 à 2 500 avec 10 % de remise : 10 000 - 1 000.
        assert_eq!(cout_entree(2_500, 4 * U, 1_000), 9_000);
        assert_eq!(cout_entree(2_500, 4 * U, 0), 10_000);
    }

    #[test]
    fn correction_de_reception_a_son_cout() {
        let s = Stock::default()
            .entree(10 * U, 10_000)
            .entree(10 * U, 12_000);
        let s = s.sortie_a(10 * U, 12_000);
        assert_eq!((s.quantite, s.valeur), (10 * U, 10_000));
        assert_eq!(s.cout_moyen(), 1_000);
    }
}
