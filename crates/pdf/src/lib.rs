//! Génération des documents PDF (cahier des charges, section 12).

mod calibrage;
mod courrier;
pub mod modele;
pub mod pagination;
mod rendu;
pub mod texte;

pub use calibrage::generer_calibrage;
pub use courrier::{generer_lettre, generer_liste};
pub use rendu::generer;
