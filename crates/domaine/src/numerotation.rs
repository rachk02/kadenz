//! Format des numéros de documents : `{Préfixe}{AAAA}-{MM}-{####}` (section 6.1).
//!
//! Le compteur lui-même est tenu en base (`kadenz-stockage`), dans la transaction
//! de validation. Le choix du mois (date du document ou date de validation) reste
//! une question ouverte du cahier des charges : l'appelant fournit année et mois.

use crate::document::TypeDocument;

/// Formate un numéro. Au-delà de 9999, le rang s'allonge sans perte.
pub fn formater_numero(type_document: TypeDocument, annee: u16, mois: u8, rang: u32) -> String {
    debug_assert!((1..=12).contains(&mois));
    format!("{}{annee:04}-{mois:02}-{rang:04}", type_document.prefixe())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exemple_du_cahier() {
        assert_eq!(
            formater_numero(TypeDocument::Devis, 2026, 5, 4),
            "D2026-05-0004"
        );
    }

    #[test]
    fn prefixes() {
        assert_eq!(
            formater_numero(TypeDocument::BonCommande, 2026, 5, 3),
            "C2026-05-0003"
        );
        assert_eq!(
            formater_numero(TypeDocument::Livraison, 2026, 5, 1),
            "L2026-05-0001"
        );
        assert_eq!(
            formater_numero(TypeDocument::Facture, 2026, 12, 1),
            "F2026-12-0001"
        );
        assert_eq!(
            formater_numero(TypeDocument::Proforma, 2026, 1, 1),
            "P2026-01-0001"
        );
    }

    #[test]
    fn rang_au_dela_de_9999() {
        assert_eq!(
            formater_numero(TypeDocument::Facture, 2026, 5, 12_345),
            "F2026-05-12345"
        );
    }
}
