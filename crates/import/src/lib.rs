//! Import des référentiels depuis un fichier CSV, Excel ou OpenDocument
//! (cahier des charges, section 6.8).
//!
//! Ce crate ne touche pas à la base : il lit le fichier, reconnaît les colonnes
//! et prépare l'aperçu (fiches valides, doublons, erreurs par ligne). Le stockage
//! enregistre ensuite les fiches valides en une seule transaction.

pub mod analyse;
pub mod lecture;
