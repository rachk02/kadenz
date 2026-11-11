//! Modules activables par la clé de licence (cahier des charges, section 3.2).
//!
//! Le socle (référentiels, administration, factures et avoirs) est toujours
//! actif ; les autres modules ne le sont que si la clé de licence du poste les
//! contient.

use crate::document::TypeDocument;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Module {
    Devis,
    Proforma,
    BonsCommande,
    Livraisons,
    Paiements,
    TableauDeBord,
    Relances,
}

impl Module {
    pub const TOUS: [Module; 7] = [
        Module::Devis,
        Module::Proforma,
        Module::BonsCommande,
        Module::Livraisons,
        Module::Paiements,
        Module::TableauDeBord,
        Module::Relances,
    ];

    /// Code stable, écrit dans la clé de licence.
    pub fn code(self) -> &'static str {
        match self {
            Module::Devis => "devis",
            Module::Proforma => "proforma",
            Module::BonsCommande => "bons_commande",
            Module::Livraisons => "livraisons",
            Module::Paiements => "paiements",
            Module::TableauDeBord => "tableau_de_bord",
            Module::Relances => "relances",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Module> {
        Module::TOUS.into_iter().find(|m| m.code() == code)
    }

    /// Libellé (texte source des traductions).
    pub fn libelle(self) -> &'static str {
        match self {
            Module::Devis => "Devis",
            Module::Proforma => "Proformas",
            Module::BonsCommande => "Bons de commande",
            Module::Livraisons => "Livraisons",
            Module::Paiements => "Paiements",
            Module::TableauDeBord => "Tableau de bord",
            Module::Relances => "Relances",
        }
    }

    /// Module dont celui-ci a besoin pour fonctionner.
    pub fn requis(self) -> Option<Module> {
        match self {
            Module::Relances => Some(Module::Paiements),
            _ => None,
        }
    }
}

impl TypeDocument {
    /// Module qui porte ce type de document ; aucun pour le socle (factures et
    /// avoirs).
    pub fn module(self) -> Option<Module> {
        match self {
            TypeDocument::Devis => Some(Module::Devis),
            TypeDocument::Proforma => Some(Module::Proforma),
            TypeDocument::BonCommande => Some(Module::BonsCommande),
            TypeDocument::Livraison => Some(Module::Livraisons),
            TypeDocument::Facture | TypeDocument::Avoir => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_et_dependances() {
        for m in Module::TOUS {
            assert_eq!(Module::depuis_code(m.code()), Some(m));
        }
        assert_eq!(Module::Relances.requis(), Some(Module::Paiements));
        assert_eq!(TypeDocument::Facture.module(), None);
        assert_eq!(TypeDocument::Livraison.module(), Some(Module::Livraisons));
    }
}
