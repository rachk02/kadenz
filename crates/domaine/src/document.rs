//! Types de documents, statuts de workflow et transitions autorisées
//! (cahier des charges, section 7).

use std::fmt;

/// Type de document commercial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeDocument {
    Devis,
    Proforma,
    BonCommande,
    Livraison,
    Facture,
    Avoir,
}

impl TypeDocument {
    pub const TOUS: [TypeDocument; 6] = [
        TypeDocument::Devis,
        TypeDocument::Proforma,
        TypeDocument::BonCommande,
        TypeDocument::Livraison,
        TypeDocument::Facture,
        TypeDocument::Avoir,
    ];

    /// Préfixe de numérotation (section 6.1).
    pub fn prefixe(self) -> char {
        match self {
            TypeDocument::Devis => 'D',
            TypeDocument::Proforma => 'P',
            TypeDocument::BonCommande => 'C',
            TypeDocument::Livraison => 'L',
            TypeDocument::Facture => 'F',
            TypeDocument::Avoir => 'A',
        }
    }

    /// Code stable utilisé en base de données.
    pub fn code(self) -> &'static str {
        match self {
            TypeDocument::Devis => "devis",
            TypeDocument::Proforma => "proforma",
            TypeDocument::BonCommande => "bon_commande",
            TypeDocument::Livraison => "livraison",
            TypeDocument::Facture => "facture",
            TypeDocument::Avoir => "avoir",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|t| t.code() == code)
    }

    /// Le document porte-t-il des montants ? (le bordereau de livraison n'en a pas)
    pub fn a_des_montants(self) -> bool {
        self != TypeDocument::Livraison
    }

    /// La retenue à la source ne s'applique qu'aux documents d'achat.
    pub fn retenue_applicable(self) -> bool {
        self == TypeDocument::BonCommande
    }

    /// Documents qui reçoivent des règlements et portent un état de paiement (section 8).
    pub fn recoit_des_reglements(self) -> bool {
        matches!(self, TypeDocument::Facture | TypeDocument::BonCommande)
    }

    /// Documents convertibles en Facture (section 7.7).
    pub fn convertible_en_facture(self) -> bool {
        matches!(self, TypeDocument::Devis | TypeDocument::Proforma)
    }
}

impl fmt::Display for TypeDocument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let libelle = match self {
            TypeDocument::Devis => "Devis",
            TypeDocument::Proforma => "Proforma",
            TypeDocument::BonCommande => "Bon de commande",
            TypeDocument::Livraison => "Bordereau de livraison",
            TypeDocument::Facture => "Facture",
            TypeDocument::Avoir => "Avoir",
        };
        f.write_str(libelle)
    }
}

/// Statut de workflow d'un document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Statut {
    Brouillon,
    Valide,
    Envoye,
    Converti,
    ReceptionPartielle,
    ReceptionComplete,
    LivraisonPartielle,
    LivraisonComplete,
    Annule,
}

impl Statut {
    pub const TOUS: [Statut; 9] = [
        Statut::Brouillon,
        Statut::Valide,
        Statut::Envoye,
        Statut::Converti,
        Statut::ReceptionPartielle,
        Statut::ReceptionComplete,
        Statut::LivraisonPartielle,
        Statut::LivraisonComplete,
        Statut::Annule,
    ];

    /// Code stable utilisé en base de données.
    pub fn code(self) -> &'static str {
        match self {
            Statut::Brouillon => "brouillon",
            Statut::Valide => "valide",
            Statut::Envoye => "envoye",
            Statut::Converti => "converti",
            Statut::ReceptionPartielle => "reception_partielle",
            Statut::ReceptionComplete => "reception_complete",
            Statut::LivraisonPartielle => "livraison_partielle",
            Statut::LivraisonComplete => "livraison_complete",
            Statut::Annule => "annule",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|s| s.code() == code)
    }

    /// Seul le Brouillon est entièrement modifiable (section 7.2).
    pub fn lignes_modifiables(self) -> bool {
        self == Statut::Brouillon
    }

    /// Le numéro définitif existe dès la validation, jamais avant. Un document
    /// Annulé garde son numéro s'il avait été validé, et n'en a pas sinon.
    pub fn numero_obligatoire(self) -> bool {
        !matches!(self, Statut::Brouillon | Statut::Annule)
    }

    /// Suppression physique : uniquement un Brouillon (section 7.3).
    pub fn suppression_physique_autorisee(self) -> bool {
        self == Statut::Brouillon
    }
}

impl fmt::Display for Statut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let libelle = match self {
            Statut::Brouillon => "Brouillon",
            Statut::Valide => "Validé",
            Statut::Envoye => "Envoyé",
            Statut::Converti => "Converti",
            Statut::ReceptionPartielle => "Réception partielle",
            Statut::ReceptionComplete => "Réception complète",
            Statut::LivraisonPartielle => "Livraison partielle",
            Statut::LivraisonComplete => "Livraison complète",
            Statut::Annule => "Annulé",
        };
        f.write_str(libelle)
    }
}

/// Transition refusée : le statut demandé n'est pas atteignable depuis le statut courant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionInterdite {
    pub type_document: TypeDocument,
    pub depuis: Statut,
    pub vers: Statut,
}

impl fmt::Display for TransitionInterdite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} : passage de « {} » à « {} » non autorisé",
            self.type_document, self.depuis, self.vers
        )
    }
}

impl std::error::Error for TransitionInterdite {}

/// Statuts atteignables depuis `depuis` pour un type de document (section 7.1).
pub fn transitions_possibles(type_document: TypeDocument, depuis: Statut) -> &'static [Statut] {
    use Statut::*;
    use TypeDocument::*;
    match (type_document, depuis) {
        // Devis / Proforma : conversion possible dès Validé (question 6 du cahier, tranchée).
        (Devis | Proforma, Brouillon) => &[Valide, Annule],
        (Devis | Proforma, Valide) => &[Envoye, Converti, Annule],
        (Devis | Proforma, Envoye) => &[Converti, Annule],

        (BonCommande, Brouillon) => &[Valide, Annule],
        (BonCommande, Valide) => &[Envoye, Annule],
        (BonCommande, Envoye) => &[ReceptionPartielle, ReceptionComplete, Annule],
        (BonCommande, ReceptionPartielle) => &[ReceptionComplete],

        (Livraison, Brouillon) => &[Valide, Annule],
        (Livraison, Valide) => &[LivraisonPartielle, LivraisonComplete, Annule],
        (Livraison, LivraisonPartielle) => &[LivraisonComplete],

        // Facture et Avoir suivent le même cycle (section 7.1).
        (Facture | Avoir, Brouillon) => &[Valide, Annule],
        (Facture | Avoir, Valide) => &[Envoye, Annule],
        (Facture | Avoir, Envoye) => &[Annule],

        _ => &[],
    }
}

/// Vérifie une transition de statut.
pub fn verifier_transition(
    type_document: TypeDocument,
    depuis: Statut,
    vers: Statut,
) -> Result<(), TransitionInterdite> {
    if transitions_possibles(type_document, depuis).contains(&vers) {
        Ok(())
    } else {
        Err(TransitionInterdite {
            type_document,
            depuis,
            vers,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_aller_retour() {
        for t in TypeDocument::TOUS {
            assert_eq!(TypeDocument::depuis_code(t.code()), Some(t));
        }
        for s in Statut::TOUS {
            assert_eq!(Statut::depuis_code(s.code()), Some(s));
        }
    }

    #[test]
    fn cycle_facture() {
        use Statut::*;
        assert!(verifier_transition(TypeDocument::Facture, Brouillon, Valide).is_ok());
        assert!(verifier_transition(TypeDocument::Facture, Valide, Envoye).is_ok());
        assert!(verifier_transition(TypeDocument::Facture, Envoye, Annule).is_ok());
        assert!(verifier_transition(TypeDocument::Facture, Brouillon, Envoye).is_err());
        assert!(verifier_transition(TypeDocument::Facture, Valide, Converti).is_err());
    }

    #[test]
    fn annule_et_converti_sont_terminaux() {
        for t in TypeDocument::TOUS {
            assert!(transitions_possibles(t, Statut::Annule).is_empty());
            assert!(transitions_possibles(t, Statut::Converti).is_empty());
        }
    }

    #[test]
    fn bon_commande_non_annulable_apres_reception() {
        use Statut::*;
        let t = TypeDocument::BonCommande;
        assert!(verifier_transition(t, Envoye, ReceptionPartielle).is_ok());
        assert!(verifier_transition(t, ReceptionPartielle, ReceptionComplete).is_ok());
        assert!(verifier_transition(t, ReceptionPartielle, Annule).is_err());
    }

    #[test]
    fn seuls_devis_et_proforma_se_convertissent() {
        for t in TypeDocument::TOUS {
            let convertible = transitions_possibles(t, Statut::Valide).contains(&Statut::Converti);
            assert_eq!(convertible, t.convertible_en_facture());
        }
    }
}
