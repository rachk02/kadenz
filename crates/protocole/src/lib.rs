//! Format des échanges entre les postes et le serveur de Kadenz (phase 2,
//! jalon F1 ; `docs/spec-synchronisation.md`, sections 3, 4 et 6).
//!
//! Ce crate ne fait ni réseau ni accès à la base : il décrit ce qui voyage.
//! Les références entre lignes passent toujours par l'identifiant global
//! (`uid`, UUID version 7), jamais par les identifiants entiers propres à
//! chaque base. Le serveur applique les opérations avec `kadenz-stockage`.
//!
//! Catalogue : toutes les écritures de la V1 sur des données partagées (F5),
//! chacune appliquée par le serveur avec la fonction habituelle du stockage.
//! Les écritures propres à chaque machine (licence V1, sauvegardes, dossiers
//! d'export, chemin des PDF, lien au serveur) ne voyagent pas.

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
    // Tiers.
    CreerTiers {
        uid: String,
        fiche: Tiers,
    },
    ModifierTiers {
        uid: String,
        fiche: Tiers,
    },
    RetirerTiers {
        uid: String,
    },
    DefinirActifTiers {
        uid: String,
        actif: bool,
    },
    /// `uids` donne l'identifiant de chaque fiche, dans le même ordre.
    ImporterTiers {
        fiches: Vec<Tiers>,
        uids: Vec<String>,
    },

    // Produits.
    CreerProduit {
        uid: String,
        fiche: Produit,
    },
    ModifierProduit {
        uid: String,
        fiche: Produit,
    },
    RetirerProduit {
        uid: String,
    },
    DefinirActifProduit {
        uid: String,
        actif: bool,
    },
    ImporterProduits {
        fiches: Vec<Produit>,
        uids: Vec<String>,
    },

    // Documents. `nouveau` est l'identifiant du document créé.
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
    ModifierLivraison {
        uid: String,
        visa_client: String,
        visa_fournisseur: String,
        date_livraison: String,
        date_reception: String,
    },
    ModifierAdministratif {
        uid: String,
        mode_paiement: String,
        observations: String,
    },
    Valider {
        uid: String,
    },
    Annuler {
        uid: String,
    },
    SupprimerBrouillon {
        uid: String,
    },
    Dupliquer {
        uid: String,
        nouveau: String,
        annuler_original: bool,
    },
    CreerAvoir {
        uid: String,
        nouveau: String,
    },
    ConvertirEnFacture {
        uid: String,
        nouveau: String,
    },
    CreerBordereau {
        uid: String,
        nouveau: String,
    },
    ConfirmerLivraison {
        uid: String,
    },
    /// Quantités reçues d'un bon de commande, une par ligne dans l'ordre.
    EnregistrerReception {
        uid: String,
        recues: Vec<i64>,
    },
    MarquerEnvoye {
        uid: String,
    },

    // Règlements.
    AjouterReglement {
        uid: String,
        /// `uid` du document réglé.
        document: String,
        /// `AAAA-MM-JJ`.
        date: String,
        montant: i64,
        mode_paiement: String,
        reference: String,
    },
    SupprimerReglement {
        uid: String,
    },

    // Comptes.
    CreerCompte {
        uid: String,
        nom_complet: String,
        identifiant: String,
        mot_de_passe: String,
        /// Code du rôle de la V1.
        role: String,
    },
    DefinirActifCompte {
        uid: String,
        actif: bool,
    },
    ChangerRole {
        uid: String,
        role: String,
    },
    /// Écritures personnelles : toujours sur le compte de la session, d'où
    /// l'absence de compte cible ; seules permises en lecture seule.
    ChangerMotDePasse {
        ancien: String,
        nouveau: String,
    },
    ReinitialiserMotDePasse {
        uid: String,
        nouveau: String,
    },
    ChangerLangue {
        /// Code de la langue.
        langue: String,
    },
    ChangerAffichage {
        /// Code de la navigation (`bandeau`, `barre`).
        navigation: String,
        animations: bool,
    },

    // Réglages partagés. `contenu` est la forme JSON gardée par le stockage
    // pour ce réglage ; le serveur la relit et applique les contrôles habituels.
    EnregistrerParametres {
        contenu: serde_json::Value,
    },
    EnregistrerEntreprise {
        contenu: serde_json::Value,
    },
    DefinirLogo {
        /// `principal` ou `secondaire`.
        logo: String,
        /// Image en base64 ; `None` retire le logo.
        octets: Option<String>,
    },
    EnregistrerMatrice {
        contenu: serde_json::Value,
    },
    EnregistrerTextesRelance {
        contenu: serde_json::Value,
    },
    EnregistrerMiseEnPage {
        contenu: serde_json::Value,
    },
    EnregistrerBas {
        type_document: String,
        contenu: serde_json::Value,
    },
}

/// Ce qu'une ligne changée ailleurs depuis la base de l'opération fait de
/// celle-ci (section 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiChangee {
    /// Appliquée quand même : création, ou transition jugée par les règles
    /// de la V1 sur l'état présent.
    Appliquer,
    /// Gardée comme conflit, qu'une personne tranche (décision 17).
    Conflit,
    /// Refusée : le poste reçoit d'abord la version du serveur, la personne
    /// refait son geste.
    Depassee,
}

/// Ce qu'un poste peut faire d'une action quand le serveur est injoignable
/// (décision 50).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnCoupure {
    /// Appliquée à titre provisoire dans la copie et mise en file d'envoi.
    Permise,
    /// La validation, qui donne le numéro, attend le serveur (décision 9).
    Validation,
    /// Inactive jusqu'au retour du serveur : comptes et réglages.
    Refusee,
}

/// Texte mis à la place d'un mot de passe dans une opération gardée.
pub const MASQUE: &str = "(masqué)";

impl Action {
    pub fn famille(&self) -> Famille {
        use Action::*;
        match self {
            CreerBrouillon { .. }
            | ModifierBrouillon { .. }
            | ModifierLivraison { .. }
            | ModifierAdministratif { .. }
            | Valider { .. }
            | Annuler { .. }
            | SupprimerBrouillon { .. }
            | Dupliquer { .. }
            | CreerAvoir { .. }
            | ConvertirEnFacture { .. }
            | CreerBordereau { .. }
            | ConfirmerLivraison { .. }
            | EnregistrerReception { .. }
            | MarquerEnvoye { .. }
            | AjouterReglement { .. }
            | SupprimerReglement { .. } => Famille::GestionCommerciale,
            _ => Famille::Socle,
        }
    }

    /// Entité et `uid` de la ligne visée, pour reconnaître un conflit et
    /// garder l'ordre des actions d'un poste. `None` pour un import et pour
    /// une écriture personnelle (le compte de la session).
    pub fn cible(&self) -> Option<(Entite, &str)> {
        use Action::*;
        Some(match self {
            CreerTiers { uid, .. }
            | ModifierTiers { uid, .. }
            | RetirerTiers { uid }
            | DefinirActifTiers { uid, .. } => (Entite::Tiers, uid),
            CreerProduit { uid, .. }
            | ModifierProduit { uid, .. }
            | RetirerProduit { uid }
            | DefinirActifProduit { uid, .. } => (Entite::Produit, uid),
            CreerBrouillon { uid, .. }
            | ModifierBrouillon { uid, .. }
            | ModifierLivraison { uid, .. }
            | ModifierAdministratif { uid, .. }
            | Valider { uid }
            | Annuler { uid }
            | SupprimerBrouillon { uid }
            | Dupliquer { uid, .. }
            | CreerAvoir { uid, .. }
            | ConvertirEnFacture { uid, .. }
            | CreerBordereau { uid, .. }
            | ConfirmerLivraison { uid }
            | EnregistrerReception { uid, .. }
            | MarquerEnvoye { uid } => (Entite::Document, uid),
            AjouterReglement { uid, .. } | SupprimerReglement { uid } => (Entite::Reglement, uid),
            CreerCompte { uid, .. }
            | DefinirActifCompte { uid, .. }
            | ChangerRole { uid, .. }
            | ReinitialiserMotDePasse { uid, .. } => (Entite::Utilisateur, uid),
            EnregistrerEntreprise { .. } | DefinirLogo { .. } => (Entite::Entreprise, "entreprise"),
            EnregistrerParametres { .. }
            | EnregistrerMatrice { .. }
            | EnregistrerTextesRelance { .. }
            | EnregistrerMiseEnPage { .. }
            | EnregistrerBas { .. } => (Entite::Parametres, "parametres"),
            ImporterTiers { .. }
            | ImporterProduits { .. }
            | ChangerMotDePasse { .. }
            | ChangerLangue { .. }
            | ChangerAffichage { .. } => return None,
        })
    }

    /// Ligne créée par l'action, avec l'identifiant choisi par le poste
    /// (création, duplication, avoir, conversion, bordereau, règlement,
    /// compte). Un import en crée une par fiche : voir son champ `uids`.
    pub fn creee(&self) -> Option<(Entite, &str)> {
        use Action::*;
        Some(match self {
            CreerTiers { uid, .. } => (Entite::Tiers, uid),
            CreerProduit { uid, .. } => (Entite::Produit, uid),
            CreerBrouillon { uid, .. } => (Entite::Document, uid),
            Dupliquer { nouveau, .. }
            | CreerAvoir { nouveau, .. }
            | ConvertirEnFacture { nouveau, .. }
            | CreerBordereau { nouveau, .. } => (Entite::Document, nouveau),
            AjouterReglement { uid, .. } => (Entite::Reglement, uid),
            CreerCompte { uid, .. } => (Entite::Utilisateur, uid),
            _ => return None,
        })
    }

    /// Écriture du compte de la session sur lui-même : seule permise en
    /// lecture seule (abonnement échu).
    pub fn personnelle(&self) -> bool {
        matches!(
            self,
            Action::ChangerMotDePasse { .. }
                | Action::ChangerLangue { .. }
                | Action::ChangerAffichage { .. }
        )
    }

    /// Effet d'un changement de la ligne visée, fait ailleurs depuis la base.
    pub fn si_changee(&self) -> SiChangee {
        use Action::*;
        match self {
            // Contenu d'une fiche ou d'un brouillon : décision 17.
            ModifierTiers { .. }
            | ModifierProduit { .. }
            | ModifierBrouillon { .. }
            | ModifierLivraison { .. }
            | ModifierAdministratif { .. } => SiChangee::Conflit,
            // Valider ou supprimer un brouillon que l'on n'a pas vu changer ;
            // réglages enregistrés en bloc par-dessus ceux d'un autre poste.
            Valider { .. }
            | SupprimerBrouillon { .. }
            | EnregistrerParametres { .. }
            | EnregistrerEntreprise { .. }
            | DefinirLogo { .. }
            | EnregistrerMatrice { .. }
            | EnregistrerTextesRelance { .. }
            | EnregistrerMiseEnPage { .. }
            | EnregistrerBas { .. } => SiChangee::Depassee,
            _ => SiChangee::Appliquer,
        }
    }

    /// Possible ou non quand le serveur est injoignable (décision 50). Aucune
    /// action permise ne porte de secret masqué dans la copie gardée par le
    /// serveur : une opération en attente acceptée plus tard rejoue son
    /// contenu exact.
    pub fn en_coupure(&self) -> EnCoupure {
        use Action::*;
        // Pas de cas par défaut : une action nouvelle doit être classée.
        match self {
            CreerTiers { .. }
            | ModifierTiers { .. }
            | RetirerTiers { .. }
            | DefinirActifTiers { .. }
            | ImporterTiers { .. }
            | CreerProduit { .. }
            | ModifierProduit { .. }
            | RetirerProduit { .. }
            | DefinirActifProduit { .. }
            | ImporterProduits { .. }
            | CreerBrouillon { .. }
            | ModifierBrouillon { .. }
            | ModifierLivraison { .. }
            | ModifierAdministratif { .. }
            | Annuler { .. }
            | SupprimerBrouillon { .. }
            | Dupliquer { .. }
            | CreerAvoir { .. }
            | ConvertirEnFacture { .. }
            | CreerBordereau { .. }
            | ConfirmerLivraison { .. }
            | EnregistrerReception { .. }
            | MarquerEnvoye { .. }
            | AjouterReglement { .. }
            | SupprimerReglement { .. }
            | ChangerLangue { .. }
            | ChangerAffichage { .. } => EnCoupure::Permise,
            Valider { .. } => EnCoupure::Validation,
            CreerCompte { .. }
            | DefinirActifCompte { .. }
            | ChangerRole { .. }
            | ChangerMotDePasse { .. }
            | ReinitialiserMotDePasse { .. }
            | EnregistrerParametres { .. }
            | EnregistrerEntreprise { .. }
            | DefinirLogo { .. }
            | EnregistrerMatrice { .. }
            | EnregistrerTextesRelance { .. }
            | EnregistrerMiseEnPage { .. }
            | EnregistrerBas { .. } => EnCoupure::Refusee,
        }
    }

    /// Copie à garder dans l'historique du serveur : sans mot de passe, et sans
    /// le contenu d'un logo (seulement sa taille).
    pub fn masquee(&self) -> Action {
        let mut a = self.clone();
        match &mut a {
            Action::CreerCompte { mot_de_passe, .. } => *mot_de_passe = MASQUE.into(),
            Action::ChangerMotDePasse { ancien, nouveau } => {
                *ancien = MASQUE.into();
                *nouveau = MASQUE.into();
            }
            Action::ReinitialiserMotDePasse { nouveau, .. } => *nouveau = MASQUE.into(),
            Action::DefinirLogo {
                octets: Some(octets),
                ..
            } => *octets = format!("({} caractères en base64)", octets.len()),
            _ => {}
        }
        a
    }
}

/// Fiche d'un produit.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Produit {
    pub designation: String,
    pub reference: String,
    pub prix: i64,
    /// Code du sens de la V1 (`vente`, `achat`).
    pub sens: String,
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
    /// Appliquée ; `numero` est le numéro attribué par une validation,
    /// `bilan` le détail d'un retrait ou d'un import.
    Acceptee {
        numero: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bilan: Option<Bilan>,
    },
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

impl Issue {
    /// Appliquée, sans numéro ni bilan.
    pub fn acceptee() -> Issue {
        Issue::Acceptee {
            numero: None,
            bilan: None,
        }
    }
}

/// Détail d'une opération acceptée.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "bilan", rename_all = "snake_case")]
pub enum Bilan {
    /// Fiche retirée : supprimée, ou seulement désactivée parce qu'un
    /// document la cite.
    Retrait { supprimee: bool },
    /// Import : fiches ajoutées, et rang (à partir de 0) et raison de chaque
    /// fiche ignorée.
    Import {
        ajoutees: u32,
        ignorees: Vec<(u32, String)>,
    },
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
        assert_eq!(op.action.cible(), Some((Entite::Document, "d")));
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

    #[test]
    fn ecritures_en_coupure() {
        let masquees = [
            Action::CreerCompte {
                uid: "u".into(),
                nom_complet: "N".into(),
                identifiant: "i".into(),
                mot_de_passe: "secret".into(),
                role: "admin".into(),
            },
            Action::ChangerMotDePasse {
                ancien: "a".into(),
                nouveau: "n".into(),
            },
            Action::ReinitialiserMotDePasse {
                uid: "u".into(),
                nouveau: "n".into(),
            },
            Action::DefinirLogo {
                logo: "principal".into(),
                octets: Some("QUJD".into()),
            },
        ];
        // Une action dont la copie gardée diffère ne doit jamais attendre.
        for a in &masquees {
            assert_ne!(a.masquee(), *a);
            assert_eq!(a.en_coupure(), EnCoupure::Refusee, "{a:?}");
        }
        assert_eq!(
            Action::Valider { uid: "d".into() }.en_coupure(),
            EnCoupure::Validation
        );
        let permises = [
            Action::CreerTiers {
                uid: "t".into(),
                fiche: Tiers::default(),
            },
            Action::AjouterReglement {
                uid: "r".into(),
                document: "d".into(),
                date: "2026-11-13".into(),
                montant: 1,
                mode_paiement: String::new(),
                reference: String::new(),
            },
            Action::ChangerLangue {
                langue: "fr".into(),
            },
        ];
        for a in &permises {
            assert_eq!(a.en_coupure(), EnCoupure::Permise, "{a:?}");
            assert_eq!(a.masquee(), *a);
        }
        assert_eq!(
            Action::EnregistrerParametres {
                contenu: serde_json::Value::Null
            }
            .en_coupure(),
            EnCoupure::Refusee
        );
    }

    #[test]
    fn secrets_masques_et_ecritures_personnelles() {
        let a = Action::ChangerMotDePasse {
            ancien: "a1".into(),
            nouveau: "n1".into(),
        };
        assert!(a.personnelle());
        assert_eq!(a.cible(), None);
        let texte = serde_json::to_string(&a.masquee()).unwrap();
        assert!(!texte.contains("a1") && !texte.contains("n1"), "{texte}");
        let c = Action::CreerCompte {
            uid: "u".into(),
            nom_complet: "N".into(),
            identifiant: "i".into(),
            mot_de_passe: "secret".into(),
            role: "admin".into(),
        };
        assert!(!c.personnelle());
        assert!(
            !serde_json::to_string(&c.masquee())
                .unwrap()
                .contains("secret")
        );
        let r = Action::ReinitialiserMotDePasse {
            uid: "u".into(),
            nouveau: "secret".into(),
        };
        assert!(!r.personnelle());
        assert!(
            !serde_json::to_string(&r.masquee())
                .unwrap()
                .contains("secret")
        );
        let l = Action::DefinirLogo {
            logo: "principal".into(),
            octets: Some("QUJD".into()),
        };
        assert!(
            !serde_json::to_string(&l.masquee())
                .unwrap()
                .contains("QUJD")
        );
        assert_eq!(l.si_changee(), SiChangee::Depassee);
        let i = Issue::Acceptee {
            numero: None,
            bilan: Some(Bilan::Retrait { supprimee: false }),
        };
        let json = serde_json::to_string(&i).unwrap();
        assert_eq!(serde_json::from_str::<Issue>(&json).unwrap(), i);
        // Un résultat sans bilan garde sa forme d'avant.
        assert!(
            !serde_json::to_string(&Issue::acceptee())
                .unwrap()
                .contains("bilan")
        );
    }
}
