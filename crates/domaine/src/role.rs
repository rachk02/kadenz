//! Rôles et matrice des droits (cahier des charges, section 10.1).

use std::fmt;

use crate::document::TypeDocument;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Admin,
    Comptable,
    Commercial,
    Consultation,
}

impl Role {
    pub const TOUS: [Role; 4] = [
        Role::Admin,
        Role::Comptable,
        Role::Commercial,
        Role::Consultation,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::Comptable => "comptable",
            Role::Commercial => "commercial",
            Role::Consultation => "consultation",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|r| r.code() == code)
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Role::Admin => "Administrateur",
            Role::Comptable => "Comptable",
            Role::Commercial => "Commercial",
            Role::Consultation => "Consultation",
        })
    }
}

/// Actions soumises à droits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    ModifierBrouillon(TypeDocument),
    ValiderDocument,
    AnnulerDocumentValide,
    SaisirReglement,
    AjouterReferentiel,
    ModifierReferentiel,
    GererUtilisateurs,
    Consulter,
    SauvegarderBase,
}

/// Le rôle autorise-t-il l'action ?
pub fn autorise(role: Role, action: Action) -> bool {
    use Action::*;
    use Role::*;
    match role {
        Admin => true,
        Comptable => !matches!(action, GererUtilisateurs | SauvegarderBase),
        Commercial => match action {
            ModifierBrouillon(t) => t != TypeDocument::Facture,
            AjouterReferentiel | Consulter => true,
            _ => false,
        },
        Consultation => action == Consulter,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrice_du_cahier() {
        use Action::*;
        use Role::*;
        let facture = ModifierBrouillon(TypeDocument::Facture);
        let devis = ModifierBrouillon(TypeDocument::Devis);

        assert!(autorise(Admin, GererUtilisateurs));
        assert!(autorise(Admin, SauvegarderBase));

        assert!(autorise(Comptable, facture));
        assert!(autorise(Comptable, ValiderDocument));
        assert!(autorise(Comptable, SaisirReglement));
        assert!(!autorise(Comptable, GererUtilisateurs));
        assert!(!autorise(Comptable, SauvegarderBase));

        assert!(autorise(Commercial, devis));
        assert!(!autorise(Commercial, facture));
        assert!(!autorise(Commercial, ValiderDocument));
        assert!(autorise(Commercial, AjouterReferentiel));
        assert!(!autorise(Commercial, ModifierReferentiel));

        assert!(autorise(Consultation, Consulter));
        assert!(!autorise(Consultation, devis));
    }

    #[test]
    fn codes_aller_retour() {
        for r in Role::TOUS {
            assert_eq!(Role::depuis_code(r.code()), Some(r));
        }
    }
}
