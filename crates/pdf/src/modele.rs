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
    /// Quantité commandée sur un bordereau de livraison.
    pub quantite: Quantite,
    /// Quantité livrée, imprimée seulement sur un bordereau (sans montants).
    pub quantite_livree: Quantite,
    pub total: Montant,
}

/// Zones de visa et dates d'un bordereau de livraison, en texte libre (section
/// 6.5) ; ignorées sur un document à montants.
#[derive(Debug, Clone, Default)]
pub struct LivraisonPdf {
    pub visa_client: String,
    pub visa_fournisseur: String,
    pub date_livraison: String,
    pub date_reception: String,
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
    /// Faux pour le bordereau de livraison, qui ne porte pas de montants : ses
    /// lignes montrent les quantités commandée et livrée, et les zones de visa
    /// remplacent le signataire.
    pub avec_montants: bool,
    pub livraison: LivraisonPdf,
    pub taux: Taux,
    pub totaux: Totaux,
    /// Montant TTC en lettres, déjà écrit selon l'orthographe réglée, avec majuscule
    /// initiale (en_lettres_majuscule).
    pub montant_en_lettres: String,
    pub filigrane: Filigrane,
    /// Couleur principale (en-tête du tableau, titre).
    pub couleur: [u8; 3],
}

/// Lettre de relance d'une facture (section 8), textes déjà remplis.
#[derive(Debug, Clone, Default)]
pub struct LettrePdf {
    pub entreprise: EntreprisePdf,
    /// Couleur principale (titre, en-tête du tableau).
    pub couleur: [u8; 3],
    /// Ligne de lieu et de date, déjà écrite (« Ouagadougou, le 11/06/2026 »).
    pub lieu_et_date: String,
    pub destinataire: TiersPdf,
    pub titre: String,
    pub objet: String,
    /// Texte avant et après le tableau ; une ligne vide sépare deux paragraphes.
    pub avant: String,
    pub apres: String,
    /// Tableau récapitulatif : en-têtes et valeurs, déjà écrits, dans le même ordre.
    pub recapitulatif: Vec<(String, String)>,
    pub signataire: String,
    pub nom_signataire: String,
}

/// Colonne d'une liste imprimée.
#[derive(Debug, Clone, Default)]
pub struct ColonnePdf {
    pub titre: String,
    /// Largeur en points ; 0 pour se partager la place restante.
    pub largeur: f32,
    pub a_droite: bool,
}

/// Registre ou liste imprimée (règlements, relances) : tableau paginé, en-tête
/// et pied de page de l'entreprise, totaux en fin de liste.
#[derive(Debug, Clone, Default)]
pub struct ListePdf {
    pub entreprise: EntreprisePdf,
    pub couleur: [u8; 3],
    /// Titre imprimé en haut à droite, ex. « REGISTRE DES RÈGLEMENTS ».
    pub titre: String,
    /// Date d'édition, déjà écrite (jj/mm/aaaa).
    pub date: String,
    /// Critères de la sélection, une ligne chacun, en page 1.
    pub criteres: Vec<String>,
    pub colonnes: Vec<ColonnePdf>,
    /// Valeurs déjà écrites, une par colonne.
    pub lignes: Vec<Vec<String>>,
    /// Totaux en fin de liste.
    pub totaux: Vec<TotalPdf>,
}

/// Ligne de total d'une liste ; un total principal est mis en valeur, un
/// sous-total (« dont ... ») ne l'est pas.
#[derive(Debug, Clone, Default)]
pub struct TotalPdf {
    pub libelle: String,
    /// Valeur déjà écrite.
    pub valeur: String,
    pub principal: bool,
}
