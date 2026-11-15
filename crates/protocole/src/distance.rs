//! Accès à distance par le relais (phase 2, jalon F8 ; décisions 16, 28, 57
//! à 59 ; écrans, section 9).

use serde::{Deserialize, Serialize};

/// Message à traduire : modèle avec des `{}` et leurs valeurs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageRelais {
    pub modele: String,
    pub valeurs: Vec<String>,
}

impl MessageRelais {
    pub fn new(modele: &str, valeurs: &[&str]) -> MessageRelais {
        MessageRelais {
            modele: modele.into(),
            valeurs: valeurs.iter().map(|v| v.to_string()).collect(),
        }
    }
}

/// Modèles possibles de [`EtatAccesDistant::erreur`], pour les traductions.
pub const MODELES_ERREUR: &[&str] = &[
    "relais injoignable : {}",
    "le relais ne présente pas le certificat attendu",
    "aucune licence d'abonnement : l'accès à distance demande un abonnement actif",
    "abonnement échu le {} : l'accès à distance demande un abonnement actif",
    "licence d'abonnement refusée par le relais",
    "un autre serveur de cette entreprise est déjà enregistré sur le relais",
    "le relais utilise une autre version ({})",
    "réponse du relais inattendue : {}",
];

/// Un poste inscrit et sa case « accès à distance » (décision 57).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PosteDistant {
    /// Empreinte du certificat du poste.
    pub empreinte: String,
    pub nom: String,
    pub autorise: bool,
}

/// État de l'accès à distance, vu par le serveur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EtatAccesDistant {
    /// Activé par l'Admin (désactivé par défaut).
    pub actif: bool,
    /// Adresse du relais réglée dans la fenêtre du serveur ; `None` : aucun
    /// relais réglé.
    pub relais: Option<String>,
    /// Le serveur est enregistré auprès du relais en ce moment.
    pub joignable: bool,
    /// Motif du dernier échec avec le relais.
    pub erreur: Option<MessageRelais>,
    /// Début de la dernière session distante, `AAAA-MM-JJTHH:MM`, fuseau de
    /// l'entreprise.
    pub derniere_distante: Option<String>,
    pub postes: Vec<PosteDistant>,
}
