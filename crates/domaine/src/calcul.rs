//! Calcul des montants d'un document (cahier des charges, section 6.1).
//!
//! Montants en FCFA entiers, pourcentages en centièmes de point (1800 = 18 %),
//! quantités en millièmes (1000 = 1 unité). Aucun nombre à virgule flottante.

/// Montant en FCFA (pas de décimale).
pub type Montant = i64;

/// Pourcentage en centièmes de point : 1800 = 18 %, 250 = 2,5 %.
pub type Pourcentage = i64;

/// Quantité en millièmes : 1000 = 1 unité, 2500 = 2,5 unités.
pub type Quantite = i64;

pub const UNE_UNITE: Quantite = 1000;
pub const CENT_POUR_CENT: Pourcentage = 10_000;

/// Division entière arrondie : point unique de toute règle d'arrondi du domaine.
///
/// Arrondi au franc le plus proche, la moitié vers le haut (question 2 du cahier
/// des charges, tranchée le 2026-11-09).
pub fn arrondir_division(numerateur: i128, denominateur: i128) -> Montant {
    debug_assert!(denominateur > 0);
    let moitie = denominateur / 2;
    let q = if numerateur >= 0 {
        (numerateur + moitie) / denominateur
    } else {
        (numerateur - moitie) / denominateur
    };
    q as Montant
}

/// Applique un pourcentage à un montant.
pub fn appliquer_pourcentage(montant: Montant, pourcentage: Pourcentage) -> Montant {
    arrondir_division(
        montant as i128 * pourcentage as i128,
        CENT_POUR_CENT as i128,
    )
}

/// Total d'une ligne : prix unitaire × quantité.
pub fn total_ligne(prix: Montant, quantite: Quantite) -> Montant {
    arrondir_division(prix as i128 * quantite as i128, UNE_UNITE as i128)
}

/// Taux appliqués à un document.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Taux {
    pub remise: Pourcentage,
    pub tva: Pourcentage,
    /// Ignorée si le type de document n'admet pas de retenue.
    pub retenue: Pourcentage,
}

/// Totaux calculés d'un document.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Totaux {
    pub ht: Montant,
    pub remise: Montant,
    pub base_tva: Montant,
    pub tva: Montant,
    pub retenue: Montant,
    pub ttc: Montant,
}

impl Totaux {
    /// Montant dû au titre du document (section 8) : le TTC, retenue déjà déduite.
    pub fn montant_du(&self) -> Montant {
        self.ttc
    }
}

/// Calcule les totaux à partir des lignes `(prix, quantite)`.
pub fn calculer_totaux(
    lignes: impl IntoIterator<Item = (Montant, Quantite)>,
    taux: Taux,
    retenue_applicable: bool,
) -> Totaux {
    let ht: Montant = lignes.into_iter().map(|(p, q)| total_ligne(p, q)).sum();
    let remise = appliquer_pourcentage(ht, taux.remise);
    let base_tva = ht - remise;
    let tva = appliquer_pourcentage(base_tva, taux.tva);
    let retenue = if retenue_applicable {
        appliquer_pourcentage(base_tva, taux.retenue)
    } else {
        0
    };
    Totaux {
        ht,
        remise,
        base_tva,
        tva,
        retenue,
        ttc: base_tva + tva - retenue,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrondi_commercial() {
        assert_eq!(arrondir_division(5, 10), 1);
        assert_eq!(arrondir_division(4, 10), 0);
        assert_eq!(arrondir_division(15, 10), 2);
        assert_eq!(arrondir_division(-5, 10), -1);
    }

    #[test]
    fn quantite_fractionnaire() {
        // 2,5 unités à 1 001 FCFA = 2 502,5 arrondi à 2 503.
        assert_eq!(total_ligne(1001, 2500), 2503);
    }

    #[test]
    fn facture_remise_et_tva() {
        let t = calculer_totaux(
            [(100_000, 2 * UNE_UNITE), (50_000, UNE_UNITE)],
            Taux {
                remise: 500,
                tva: 1800,
                retenue: 500,
            },
            false,
        );
        assert_eq!(t.ht, 250_000);
        assert_eq!(t.remise, 12_500);
        assert_eq!(t.base_tva, 237_500);
        assert_eq!(t.tva, 42_750);
        assert_eq!(t.retenue, 0, "pas de retenue hors bon de commande");
        assert_eq!(t.ttc, 280_250);
    }

    #[test]
    fn bon_commande_retenue_deduite() {
        let t = calculer_totaux(
            [(100_000, UNE_UNITE)],
            Taux {
                remise: 0,
                tva: 1800,
                retenue: 500,
            },
            true,
        );
        assert_eq!(t.base_tva, 100_000);
        assert_eq!(t.tva, 18_000);
        assert_eq!(t.retenue, 5_000);
        assert_eq!(t.ttc, 113_000);
        assert_eq!(t.montant_du(), 113_000);
    }

    #[test]
    fn document_vide() {
        assert_eq!(
            calculer_totaux([], Taux::default(), true),
            Totaux::default()
        );
    }
}
