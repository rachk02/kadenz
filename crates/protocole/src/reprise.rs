//! Reprise d'une base V1 dans un serveur neuf (phase 2, jalon F7 ; décisions
//! 19 et 53 à 55 ; écrans, section 8).
//!
//! Le poste envoie la base V1 sous la forme d'une sauvegarde `.kzs` et de son
//! mot de passe ; le serveur la lit, la verse dans sa base et rend le bilan.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};

/// Taille maximale de la sauvegarde envoyée : elle voyage en base64 dans une
/// seule trame, bornée à [`crate::messages::TAILLE_MAX`].
pub const TAILLE_MAX_REPRISE: usize = 5 * 1024 * 1024;

/// Sauvegarde mise en texte pour `Requete::Reprise`.
pub fn encoder(sauvegarde: &[u8]) -> String {
    STANDARD.encode(sauvegarde)
}

/// Sauvegarde reçue dans `Requete::Reprise` ; `None` si le texte est illisible
/// ou si elle dépasse [`TAILLE_MAX_REPRISE`].
pub fn decoder(texte: &str) -> Option<Vec<u8>> {
    // Base64 : 4 caractères pour 3 octets.
    if texte.len() > TAILLE_MAX_REPRISE.div_ceil(3) * 4 {
        return None;
    }
    STANDARD
        .decode(texte)
        .ok()
        .filter(|o| o.len() <= TAILLE_MAX_REPRISE)
}

/// Contenu d'une base V1 : résumé montré avant la reprise, puis bilan mesuré
/// sur le serveur après la reprise, avec les mêmes nombres.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeReprise {
    /// Nom de l'entreprise de la base V1.
    pub entreprise: String,
    /// Tiers clients (y compris ceux qui sont aussi fournisseurs).
    pub clients: u32,
    /// Tiers fournisseurs (y compris ceux qui sont aussi clients).
    pub fournisseurs: u32,
    pub produits: u32,
    pub reglements: u32,
    /// Documents par type, dans l'ordre des types de la V1 ; seuls les types
    /// présents figurent.
    pub documents: Vec<DocumentsRepris>,
    /// Identifiants des comptes V1 repris.
    pub comptes_repris: Vec<String>,
    /// Identifiants des comptes V1 écartés, parce qu'un compte du serveur
    /// porte déjà le même identifiant (décision 53).
    pub comptes_ecartes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentsRepris {
    /// Code du type (`facture`, `avoir`…).
    pub type_document: String,
    pub nombre: u32,
    /// Numéro du dernier document numéroté de ce type, s'il y en a un.
    pub dernier_numero: Option<String>,
}
