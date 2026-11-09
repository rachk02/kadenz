//! Rôles et matrice des droits (cahier des charges, section 10.1).
//!
//! La matrice est une donnée : celle du cahier des charges sert de valeur par
//! défaut, et l'Admin peut la modifier (réglage « Rôles »). Un seul droit est
//! intangible : le rôle Admin garde toujours la gestion des utilisateurs, pour
//! qu'aucune installation ne se retrouve sans administrateur.

use std::collections::BTreeSet;
use std::fmt;

use crate::document::TypeDocument;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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

/// Droits soumis à la matrice, une ligne par ligne du tableau du cahier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Droit {
    /// Créer et modifier un brouillon de Devis, Proforma, Bon de commande, Livraison.
    BrouillonDocuments,
    /// Créer et modifier un brouillon de Facture ou d'Avoir.
    BrouillonFacture,
    ValiderDocument,
    AnnulerDocumentValide,
    SaisirReglement,
    AjouterReferentiel,
    ModifierReferentiel,
    GererUtilisateurs,
    /// Fiche entreprise et paramètres système (section 5.4).
    GererParametres,
    Consulter,
    SauvegarderBase,
}

impl Droit {
    pub const TOUS: [Droit; 11] = [
        Droit::BrouillonDocuments,
        Droit::BrouillonFacture,
        Droit::ValiderDocument,
        Droit::AnnulerDocumentValide,
        Droit::SaisirReglement,
        Droit::AjouterReferentiel,
        Droit::ModifierReferentiel,
        Droit::GererUtilisateurs,
        Droit::GererParametres,
        Droit::Consulter,
        Droit::SauvegarderBase,
    ];

    /// Droit nécessaire pour modifier un brouillon du type donné.
    pub fn brouillon(type_document: TypeDocument) -> Droit {
        match type_document {
            TypeDocument::Facture | TypeDocument::Avoir => Droit::BrouillonFacture,
            _ => Droit::BrouillonDocuments,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Droit::BrouillonDocuments => "brouillon_documents",
            Droit::BrouillonFacture => "brouillon_facture",
            Droit::ValiderDocument => "valider_document",
            Droit::AnnulerDocumentValide => "annuler_document_valide",
            Droit::SaisirReglement => "saisir_reglement",
            Droit::AjouterReferentiel => "ajouter_referentiel",
            Droit::ModifierReferentiel => "modifier_referentiel",
            Droit::GererUtilisateurs => "gerer_utilisateurs",
            Droit::GererParametres => "gerer_parametres",
            Droit::Consulter => "consulter",
            Droit::SauvegarderBase => "sauvegarder_base",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|d| d.code() == code)
    }
}

impl fmt::Display for Droit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Droit::BrouillonDocuments => {
                "Créer/modifier un brouillon (Devis, Proforma, BC, Livraison)"
            }
            Droit::BrouillonFacture => "Créer/modifier un brouillon Facture ou Avoir",
            Droit::ValiderDocument => "Valider un document",
            Droit::AnnulerDocumentValide => "Annuler un document validé",
            Droit::SaisirReglement => "Saisir un règlement",
            Droit::AjouterReferentiel => "Ajouter un client, un fournisseur, un produit",
            Droit::ModifierReferentiel => {
                "Modifier ou désactiver un client, un fournisseur, un produit"
            }
            Droit::GererUtilisateurs => "Gérer les utilisateurs et droits",
            Droit::GererParametres => "Gérer la fiche entreprise et les paramètres",
            Droit::Consulter => "Consulter tableau de bord et registres",
            Droit::SauvegarderBase => "Sauvegarder / exporter la base",
        })
    }
}

/// Matrice des droits : ensemble des couples (rôle, droit) autorisés.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatriceDroits {
    autorisations: BTreeSet<(Role, Droit)>,
}

impl Default for MatriceDroits {
    /// Matrice du cahier des charges, section 10.1.
    fn default() -> Self {
        use Droit::*;
        use Role::*;
        let mut autorisations = BTreeSet::new();
        for droit in Droit::TOUS {
            autorisations.insert((Admin, droit));
        }
        for droit in [
            BrouillonDocuments,
            BrouillonFacture,
            ValiderDocument,
            AnnulerDocumentValide,
            SaisirReglement,
            AjouterReferentiel,
            ModifierReferentiel,
            Consulter,
        ] {
            autorisations.insert((Comptable, droit));
        }
        for droit in [BrouillonDocuments, AjouterReferentiel, Consulter] {
            autorisations.insert((Commercial, droit));
        }
        autorisations.insert((Consultation, Consulter));
        MatriceDroits { autorisations }
    }
}

impl MatriceDroits {
    pub fn vide() -> Self {
        MatriceDroits {
            autorisations: BTreeSet::new(),
        }
    }

    pub fn autorise(&self, role: Role, droit: Droit) -> bool {
        (role == Role::Admin && droit == Droit::GererUtilisateurs)
            || self.autorisations.contains(&(role, droit))
    }

    pub fn definir(&mut self, role: Role, droit: Droit, autorise: bool) {
        if autorise {
            self.autorisations.insert((role, droit));
        } else {
            self.autorisations.remove(&(role, droit));
        }
    }

    /// Couples autorisés, pour l'enregistrement.
    pub fn autorisations(&self) -> impl Iterator<Item = (Role, Droit)> + '_ {
        self.autorisations.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrice_du_cahier() {
        use Droit::*;
        use Role::*;
        let m = MatriceDroits::default();
        let facture = Droit::brouillon(TypeDocument::Facture);
        let devis = Droit::brouillon(TypeDocument::Devis);

        assert!(m.autorise(Admin, GererUtilisateurs));
        assert!(m.autorise(Admin, SauvegarderBase));
        assert!(m.autorise(Admin, GererParametres));

        assert!(m.autorise(Comptable, facture));
        assert!(m.autorise(Comptable, ValiderDocument));
        assert!(m.autorise(Comptable, SaisirReglement));
        assert!(m.autorise(Comptable, ModifierReferentiel));
        assert!(!m.autorise(Comptable, GererUtilisateurs));
        assert!(!m.autorise(Comptable, GererParametres));
        assert!(!m.autorise(Comptable, SauvegarderBase));

        assert!(m.autorise(Commercial, devis));
        assert!(!m.autorise(Commercial, facture));
        assert!(!m.autorise(Commercial, ValiderDocument));
        assert!(m.autorise(Commercial, AjouterReferentiel));
        assert!(!m.autorise(Commercial, ModifierReferentiel));

        assert!(m.autorise(Consultation, Consulter));
        assert!(!m.autorise(Consultation, devis));
        assert!(!m.autorise(Consultation, AjouterReferentiel));
    }

    #[test]
    fn avoir_suit_la_facture() {
        assert_eq!(
            Droit::brouillon(TypeDocument::Avoir),
            Droit::BrouillonFacture
        );
        assert_eq!(
            Droit::brouillon(TypeDocument::Livraison),
            Droit::BrouillonDocuments
        );
    }

    #[test]
    fn admin_garde_toujours_la_gestion_des_utilisateurs() {
        let mut m = MatriceDroits::default();
        m.definir(Role::Admin, Droit::GererUtilisateurs, false);
        assert!(m.autorise(Role::Admin, Droit::GererUtilisateurs));
        assert!(MatriceDroits::vide().autorise(Role::Admin, Droit::GererUtilisateurs));
        assert!(!MatriceDroits::vide().autorise(Role::Admin, Droit::Consulter));
    }

    #[test]
    fn matrice_modifiable() {
        let mut m = MatriceDroits::default();
        m.definir(Role::Commercial, Droit::ModifierReferentiel, true);
        assert!(m.autorise(Role::Commercial, Droit::ModifierReferentiel));
    }

    #[test]
    fn codes_aller_retour() {
        for r in Role::TOUS {
            assert_eq!(Role::depuis_code(r.code()), Some(r));
        }
        for d in Droit::TOUS {
            assert_eq!(Droit::depuis_code(d.code()), Some(d));
        }
    }
}
