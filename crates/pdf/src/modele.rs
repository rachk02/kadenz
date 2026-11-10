//! Données d'entrée du générateur : tout ce qui s'imprime, déjà lu en base.
//! Le générateur ne lit rien lui-même ; l'application remplit ces structures à
//! partir du document validé, de la fiche entreprise et des paramètres.

use kadenz_domaine::calcul::{Montant, Quantite, Taux, Totaux};

#[derive(Debug, Clone, Default)]
pub struct EntreprisePdf {
    pub nom: String,
    pub adresse: String,
    pub telephone: String,
    pub email: String,
    pub site_web: String,
    pub regime: String,
    pub rccm: String,
    pub ifu: String,
    pub banque1: String,
    pub banque2: String,
    /// Logo principal et logo secondaire (PNG ou JPEG), tels que stockés en base.
    pub logo: Option<Vec<u8>>,
    pub logo2: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Default)]
pub struct TiersPdf {
    pub nom: String,
    pub adresse: String,
    pub ifu: String,
    pub rccm: String,
    pub regime: String,
}

#[derive(Debug, Clone, Default)]
pub struct LignePdf {
    pub reference: String,
    pub designation: String,
    pub observations: String,
    pub prix: Montant,
    pub quantite: Quantite,
    pub total: Montant,
}

/// Textes réglables par type de document (section 5.4, « Textes des documents »).
#[derive(Debug, Clone, Default)]
pub struct TextesPdf {
    /// Titre imprimé, ex. « FACTURE ».
    pub titre: String,
    /// Libellé du bloc tiers : « Client » ou « Fournisseur ».
    pub libelle_tiers: String,
    /// Début de la phrase du montant, ex. « Arrêté la présente facture à la somme de » ;
    /// suivie de « <lettres> francs (<chiffres>) FCFA. » (modèle de l'outil Excel).
    pub phrase_somme: String,
    /// Libellé du signataire, ex. « LA COMPTABILITE » ; vide pour aucun.
    pub signataire: String,
    /// Nom imprimé sous la signature, facultatif.
    pub nom_signataire: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Filigrane {
    #[default]
    Aucun,
    /// Aperçu d'un brouillon.
    Brouillon,
    /// Document annulé.
    Annule,
}

#[derive(Debug, Clone, Default)]
pub struct DocumentPdf {
    pub entreprise: EntreprisePdf,
    pub tiers: TiersPdf,
    pub textes: TextesPdf,
    /// Numéro attribué à la validation ; vide sur l'aperçu d'un brouillon.
    pub numero: String,
    /// Document de référence déjà écrit, sous la date (ex. « Facture d'origine :
    /// F2026-0001 » sur un avoir) ; vide pour aucun.
    pub reference: String,
    /// Dates déjà écrites (jj/mm/aaaa).
    pub date: String,
    pub echeance: String,
    /// Étiquette de `echeance` : « Validité » sur un devis ou une proforma ;
    /// vide pour « Échéance ».
    pub libelle_echeance: String,
    pub libelle: String,
    pub mode_paiement: String,
    pub observations: String,
    pub lignes: Vec<LignePdf>,
    /// Faux pour le bordereau de livraison, qui ne porte pas de montants.
    pub avec_montants: bool,
    pub taux: Taux,
    pub totaux: Totaux,
    /// Montant TTC en lettres, déjà écrit selon l'orthographe réglée, avec majuscule
    /// initiale (en_lettres_majuscule).
    pub montant_en_lettres: String,
    pub filigrane: Filigrane,
    /// Couleur principale (bande, en-tête du tableau, titre).
    pub couleur: [u8; 3],
}
