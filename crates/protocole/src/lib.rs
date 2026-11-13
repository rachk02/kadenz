//! Format des échanges entre les postes et le serveur de Kadenz (phase 2,
//! jalon F1 ; `docs/spec-synchronisation.md`, sections 3, 4 et 6).
//!
//! Ce crate ne fait ni réseau ni accès à la base : il décrit ce qui voyage.
//! Les références entre lignes passent toujours par l'identifiant global
//! (`uid`, UUID version 7), jamais par les identifiants entiers propres à
//! chaque base. Le serveur applique les opérations avec `kadenz-stockage`.
//!
//! Catalogue de F1 : création et modification d'un tiers, création et
//! modification d'un brouillon, validation. Les autres actions de la V1
//! s'ajoutent avec le poste relié (F5).

use serde::{Deserialize, Serialize};

pub mod abonnement;
pub mod inscription;
pub mod messages;

pub use abonnement::Abonnement;

/// Version du protocole, annoncée à la connexion (F2) : un poste d'une autre
/// version est refusé.
pub const VERSION: u32 = 2;

/// Action d'un compte, faite sur un poste et appliquée par le serveur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Operation {
    /// UUID version 7 créé par le poste ; une opération renvoyée garde le sien.
    pub id: String,
    /// Identifiant de l'entreprise du serveur visé.
    pub entreprise: String,
    pub poste: String,
    /// `uid` du compte qui agit.
    pub compte: String,
    /// Faite pendant une coupure : un refus pour un compte ou un droit la met
    /// alors en attente au lieu de la rejeter (décisions 18 et 32).
    pub hors_connexion: bool,
    /// Dernier changement reçu du serveur par le poste au moment de l'action :
    /// sert à reconnaître un conflit (section 6).
    pub base: u64,
    pub action: Action,
}

/// Famille d'une action : le socle commun de la suite (entreprise, comptes,
/// tiers, produits, paramètres) ou la gestion commerciale. La comptabilité
/// ajoutera la sienne (cahier de la phase 2, section 4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Famille {
    Socle,
    GestionCommerciale,
}

/// Entité synchronisée, telle que nommée dans la suite des changements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Entite {
    Utilisateur,
    Tiers,
    Produit,
    Document,
    Reglement,
    /// Instantané de l'entreprise figé à la validation d'un document (F5).
    Instantane,
    /// Fiche de l'entreprise et ses logos : une seule ligne (F5).
    Entreprise,
    /// Paramètres partagés par tous les postes, en un seul bloc (F5).
    Parametres,
}

impl Entite {
    pub const TOUTES: [Entite; 8] = [
        Entite::Utilisateur,
        Entite::Tiers,
        Entite::Produit,
        Entite::Document,
        Entite::Reglement,
        Entite::Instantane,
        Entite::Entreprise,
        Entite::Parametres,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Entite::Utilisateur => "utilisateur",
            Entite::Tiers => "tiers",
            Entite::Produit => "produit",
            Entite::Document => "document",
            Entite::Reglement => "reglement",
            Entite::Instantane => "instantane",
            Entite::Entreprise => "entreprise",
            Entite::Parametres => "parametres",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Entite> {
        Entite::TOUTES.into_iter().find(|e| e.code() == code)
    }

    /// Entité d'une seule ligne, désignée par son code au lieu d'un `uid`.
    pub fn unique(self) -> bool {
        matches!(self, Entite::Entreprise | Entite::Parametres)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "nature", rename_all = "snake_case")]
pub enum Action {
    CreerTiers {
        uid: String,
        fiche: Tiers,
    },
    ModifierTiers {
        uid: String,
        fiche: Tiers,
    },
    CreerBrouillon {
        uid: String,
        /// Code du type de document de la V1 (`facture`, `devis`, etc.).
        type_document: String,
        saisie: Saisie,
    },
    ModifierBrouillon {
        uid: String,
        saisie: Saisie,
    },
    Valider {
        uid: String,
    },
}

impl Action {
    pub fn famille(&self) -> Famille {
        match self {
            Action::CreerTiers { .. } | Action::ModifierTiers { .. } => Famille::Socle,
            Action::CreerBrouillon { .. }
            | Action::ModifierBrouillon { .. }
            | Action::Valider { .. } => Famille::GestionCommerciale,
        }
    }

    /// Entité et `uid` de la ligne visée.
    pub fn cible(&self) -> (Entite, &str) {
        match self {
            Action::CreerTiers { uid, .. } | Action::ModifierTiers { uid, .. } => {
                (Entite::Tiers, uid)
            }
            Action::CreerBrouillon { uid, .. }
            | Action::ModifierBrouillon { uid, .. }
            | Action::Valider { uid } => (Entite::Document, uid),
        }
    }
}

/// Fiche d'un tiers.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Tiers {
    pub nom: String,
    pub adresse: String,
    pub telephone: String,
    pub localisation: String,
    pub ifu: String,
    pub rccm: String,
    pub regime_imposition: String,
    /// Code du type de tiers de la V1 (`client`, `fournisseur`, `les_deux`).
    pub type_tiers: String,
}

/// Contenu d'un brouillon. Dates au format `AAAA-MM-JJ`, montants en francs,
/// taux et quantités dans les unités de la V1.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Saisie {
    pub date: String,
    /// `uid` du tiers.
    pub tiers: Option<String>,
    pub libelle: String,
    pub remise: i64,
    pub tva: i64,
    pub retenue: i64,
    pub mode_paiement: String,
    pub observations: String,
    pub date_echeance: Option<String>,
    pub lignes: Vec<Ligne>,
    pub visa_client: String,
    pub visa_fournisseur: String,
    pub date_livraison: String,
    pub date_reception: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Ligne {
    /// `uid` du produit d'origine ; `None` pour une ligne libre.
    pub produit: Option<String>,
    pub reference: String,
    pub designation: String,
    pub prix: i64,
    pub quantite: i64,
    pub quantite_livree: i64,
    pub observations: String,
}

/// Résultat d'une opération. Tous sont définitifs pour l'identifiant de
/// l'opération (gardés par le serveur et rendus tels quels à chaque renvoi),
/// sauf `Bloquee`, qui demande de renvoyer la même opération plus tard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "issue", rename_all = "snake_case")]
pub enum Issue {
    /// Appliquée ; `numero` est le numéro attribué par une validation.
    Acceptee { numero: Option<String> },
    /// Refusée par une règle de la V1 : texte fixe à traduire et valeurs de ses
    /// `{}`, comme les messages du stockage.
    Refusee {
        modele: String,
        valeurs: Vec<String>,
    },
    /// Faite hors connexion par un compte désactivé ou privé du droit voulu :
    /// ni appliquée ni rejetée, l'Admin décide (décisions 18 et 32).
    EnAttente,
    /// La ligne a changé sur un autre poste depuis la base de l'opération : une
    /// personne tranche (décisions 17, 33 et 34).
    EnConflit,
    /// Validation d'un brouillon qui a changé sur un autre poste depuis : refusée,
    /// le poste reçoit d'abord la version du serveur (section 5).
    Depassee,
    /// Une opération précédente du même poste sur la même ligne est en attente
    /// ou en conflit : celle-ci n'est pas gardée, le poste la renvoie une fois la
    /// décision prise, pour garder l'ordre de ses actions.
    Bloquee,
}

/// Une ligne qui a changé, telle qu'elle est maintenant sur le serveur. Pour
/// une entité d'une seule ligne, `uid` est le code de l'entité.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Changement {
    /// Rang dans la suite des changements ; le poste garde le dernier reçu.
    pub seq: u64,
    pub entite: Entite,
    pub uid: String,
    pub etat: Etat,
}

/// Valeurs d'une ligne, par nom de colonne (F5). Les deux côtés ont le même
/// schéma, garanti par la version du protocole. Une référence à une autre
/// ligne vaut son `uid`, un contenu binaire (logo) son encodage base64 ; les
/// paramètres partagés valent leur valeur JSON, par clé.
pub type Valeurs = serde_json::Map<String, serde_json::Value>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "etat", rename_all = "snake_case")]
pub enum Etat {
    Ligne {
        valeurs: Valeurs,
        /// Lignes d'un document, dans l'ordre ; remplacées en bloc.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        lignes: Vec<Valeurs>,
    },
    Supprime,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_aller_retour() {
        let op = Operation {
            id: "0190a0b0-0000-7000-8000-000000000001".into(),
            entreprise: "e".into(),
            poste: "p".into(),
            compte: "c".into(),
            hors_connexion: true,
            base: 42,
            action: Action::CreerBrouillon {
                uid: "d".into(),
                type_document: "facture".into(),
                saisie: Saisie {
                    date: "2026-11-12".into(),
                    tiers: Some("t".into()),
                    lignes: vec![Ligne {
                        designation: "Ciment".into(),
                        prix: 5000,
                        quantite: 2,
                        ..Ligne::default()
                    }],
                    ..Saisie::default()
                },
            },
        };
        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains("\"nature\":\"creer_brouillon\""));
        assert_eq!(serde_json::from_str::<Operation>(&json).unwrap(), op);
        assert_eq!(op.action.famille(), Famille::GestionCommerciale);
        assert_eq!(op.action.cible(), (Entite::Document, "d"));
    }

    #[test]
    fn issues_et_entites() {
        let i = Issue::Refusee {
            modele: "x {}".into(),
            valeurs: vec!["1".into()],
        };
        let json = serde_json::to_string(&i).unwrap();
        assert_eq!(serde_json::from_str::<Issue>(&json).unwrap(), i);
        assert_eq!(
            serde_json::to_string(&Issue::EnAttente).unwrap(),
            "{\"issue\":\"en_attente\"}"
        );
        for e in Entite::TOUTES {
            assert_eq!(Entite::depuis_code(e.code()), Some(e));
        }
        assert_eq!(Entite::depuis_code("autre"), None);
    }
}
