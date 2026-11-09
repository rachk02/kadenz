//! Génération des documents PDF (cahier des charges, section 12).

pub mod modele;
pub mod pagination;
mod rendu;
pub mod texte;

pub use rendu::generer;
