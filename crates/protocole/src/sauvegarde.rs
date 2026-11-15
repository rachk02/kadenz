//! Sauvegardes du serveur (phase 2, jalon F7 ; décisions 35 et 56).

use serde::{Deserialize, Serialize};

/// Avertissement permanent sur les sauvegardes du serveur, montré dans la
/// fenêtre d'administration et sur les postes connectés avec un compte Admin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AvertissementSauvegarde {
    /// Aucun mot de passe choisi : aucune sauvegarde n'est faite.
    SansMotDePasse,
    /// Dossier des sauvegardes sur le même disque que la base : une panne du
    /// disque ou un vol de la machine emporte les deux.
    MemeDisque,
}
