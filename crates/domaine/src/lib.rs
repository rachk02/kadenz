//! Règles métier de Kadenz.
//!
//! Ce crate ne fait aucune entrée/sortie : l'interface et le stockage s'appuient
//! sur lui pour tout calcul et toute décision de statut (cahier des charges, section 3.1).

pub mod calcul;
pub mod document;
pub mod langue;
pub mod lettres;
pub mod message;
pub mod mise_en_page;
pub mod module;
pub mod numerotation;
pub mod paiement;
pub mod referentiel;
pub mod role;
pub mod saisie;
pub mod texte;
