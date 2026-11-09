//! État de paiement et contrôle des règlements (cahier des charges, section 8).
//!
//! L'état de paiement est un axe indépendant du statut de workflow ; il est
//! toujours calculé à partir des règlements, jamais saisi.

use std::fmt;

use crate::calcul::Montant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtatPaiement {
    Impaye,
    Partiel,
    Paye,
}

impl fmt::Display for EtatPaiement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            EtatPaiement::Impaye => "Impayé",
            EtatPaiement::Partiel => "Partiel",
            EtatPaiement::Paye => "Payé",
        })
    }
}

/// Calcule l'état de paiement à partir du montant dû et du total déjà réglé.
pub fn etat_paiement(montant_du: Montant, total_regle: Montant) -> EtatPaiement {
    if total_regle <= 0 {
        EtatPaiement::Impaye
    } else if total_regle < montant_du {
        EtatPaiement::Partiel
    } else {
        EtatPaiement::Paye
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReglementRefuse {
    MontantNonPositif,
    DepasseLeMontantDu { reste_a_payer: Montant },
}

impl fmt::Display for ReglementRefuse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReglementRefuse::MontantNonPositif => {
                f.write_str("le montant du règlement doit être positif")
            }
            ReglementRefuse::DepasseLeMontantDu { reste_a_payer } => write!(
                f,
                "le règlement dépasse le reste à payer ({reste_a_payer} FCFA)"
            ),
        }
    }
}

impl std::error::Error for ReglementRefuse {}

/// Vérifie qu'un nouveau règlement ne porte pas le total payé au-delà du montant dû.
pub fn verifier_reglement(
    montant_du: Montant,
    deja_regle: Montant,
    nouveau: Montant,
) -> Result<(), ReglementRefuse> {
    if nouveau <= 0 {
        return Err(ReglementRefuse::MontantNonPositif);
    }
    let reste_a_payer = montant_du - deja_regle;
    if nouveau > reste_a_payer {
        return Err(ReglementRefuse::DepasseLeMontantDu { reste_a_payer });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn etats() {
        assert_eq!(etat_paiement(100_000, 0), EtatPaiement::Impaye);
        assert_eq!(etat_paiement(100_000, 40_000), EtatPaiement::Partiel);
        assert_eq!(etat_paiement(100_000, 100_000), EtatPaiement::Paye);
    }

    #[test]
    fn plafond_du_montant_du() {
        assert!(verifier_reglement(100_000, 60_000, 40_000).is_ok());
        assert_eq!(
            verifier_reglement(100_000, 60_000, 40_001),
            Err(ReglementRefuse::DepasseLeMontantDu {
                reste_a_payer: 40_000
            })
        );
        assert_eq!(
            verifier_reglement(100_000, 0, 0),
            Err(ReglementRefuse::MontantNonPositif)
        );
    }
}
