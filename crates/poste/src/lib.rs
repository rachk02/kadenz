//! Synchroniseur d'un poste relié au serveur de l'entreprise (phase 2, jalon
//! F5c ; `docs/spec-synchronisation.md`, section 6).
//!
//! Un fil garde la session TLS avec le serveur et une connexion à la copie du
//! poste (`kadenz-poste.sqlite`). Il connecte les comptes par le serveur,
//! envoie les écritures une à une et attend leur résultat, puis applique à la
//! copie les changements du serveur : à chaque écriture, et toutes les deux
//! secondes entre deux écritures. L'écran reçoit des événements (état de la
//! connexion, listes à recharger, avancement, abonnement).
//!
//! Une écriture sans réponse à temps est gardée dans la copie et renvoyée avec
//! le même identifiant avant toute autre : le serveur rend alors le résultat
//! d'origine, sans l'appliquer deux fois.
//!
//! Hors connexion (F6, décisions 50 et 51), une écriture permise est appliquée
//! à titre provisoire dans la copie et mise dans la file d'envoi. Au retour du
//! serveur, la file part dans l'ordre : une écriture refusée ou dépassée est
//! retirée et ses lignes remises à l'état du serveur ; une écriture en conflit
//! ou en attente est gardée comme suspendue, ses lignes remises aussi, et
//! renvoyée ensuite pour apprendre la décision.

use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub use kadenz_client::CompteConnecte;
use kadenz_client::{Bienvenue, Client, ErreurClient, Identite};
pub use kadenz_protocole::distance::{EtatAccesDistant, MessageRelais, PosteDistant};
use kadenz_protocole::messages::LIGNES_MAX;
use kadenz_protocole::reprise::TAILLE_MAX_REPRISE;
pub use kadenz_protocole::reprise::{DocumentsRepris, ResumeReprise};
pub use kadenz_protocole::sauvegarde::AvertissementSauvegarde;
use kadenz_protocole::{Abonnement, Action, EnCoupure, Entite, Issue, Motif, Operation, Suspendue};
use kadenz_stockage::poste_relie::{self, PosteRelie};
use kadenz_stockage::{distance, replique, synchro};
use rusqlite::Connection;

/// Attente au plus d'une réponse du serveur, et d'une écriture pour l'écran.
pub const DELAI: Duration = Duration::from_secs(5);
/// Relève des changements entre deux écritures.
const RELEVE: Duration = Duration::from_secs(2);
/// Attentes successives avant de réessayer la connexion, en secondes.
const ATTENTES: [u64; 4] = [1, 2, 4, 8];
/// Échecs de connexion d'affilée avant l'état « injoignable ».
const ECHECS_INJOIGNABLE: u32 = 3;
/// Changements demandés par page.
const PAGE: u32 = 500;
/// Retard au-delà duquel le rattrapage est annoncé (« Synchronisation... »).
const RETARD_ANNONCE: u64 = PAGE as u64;
/// L'abonnement est relu toutes les 30 relèves (une minute environ) : le jour
/// change, une licence peut arriver d'un autre poste.
const RELEVES_ABONNEMENT: u32 = 30;

/// État de la connexion au serveur, pour l'indicateur de l'écran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EtatConnexion {
    /// Première connexion en cours.
    Connexion,
    Connecte,
    /// Première synchronisation, ou long rattrapage.
    Synchronisation,
    /// Connexion perdue ; nouvel essai en cours (numéro de l'essai).
    Reconnexion {
        essai: u32,
    },
    /// Plusieurs essais sans réponse ; les essais continuent.
    Injoignable,
    /// Abonnement échu ou sans licence valide.
    LectureSeule {
        echeance: Option<String>,
    },
    /// État final : le serveur ne présente pas le certificat attendu.
    CertificatDifferent,
    /// État final : le poste a été retiré par l'Admin.
    PosteRetire,
    /// État final : il faut mettre ce poste (ou le serveur) à jour.
    VersionDifferente {
        serveur: u32,
    },
}

impl EtatConnexion {
    /// État final, sans nouvel essai automatique.
    pub fn definitif(&self) -> bool {
        matches!(
            self,
            EtatConnexion::CertificatDifferent
                | EtatConnexion::PosteRetire
                | EtatConnexion::VersionDifferente { .. }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Evenement {
    Etat(EtatConnexion),
    /// Une page de changements est appliquée : listes à recharger.
    Changements {
        sortes: BTreeSet<Entite>,
    },
    /// Rattrapage annoncé : dernier rang reçu sur dernier rang du serveur.
    Avancement {
        recus: u64,
        dernier: u64,
    },
    Abonnement(Box<Abonnement>),
    /// Résultat appris plus tard : écriture rendue sans réponse à temps
    /// (`ErreurPoste::Delai`), écriture de la file d'envoi (`Issue::EnFile`)
    /// envoyée au retour du serveur, ou décision sur une écriture suspendue.
    Confirmee {
        action: Box<Action>,
        issue: Issue,
    },
    /// Nombre d'écritures qui attendent d'être envoyées au serveur (file
    /// d'envoi, écriture sans réponse), à chaque changement.
    File(u32),
    /// Nombre de brouillons à valider au retour du serveur : à chaque
    /// changement de la liste, et au retour du serveur s'il en reste.
    AValider(u32),
    /// Opérations suspendues que le compte connecté peut voir : à sa
    /// connexion, quand le nombre change (relu toutes les minutes environ),
    /// et après un conflit, une attente ou une décision venus de ce poste.
    /// Les attentes ne sont montrées qu'à l'Admin.
    Suspendues {
        conflits: u32,
        attentes: u32,
    },
    /// Réponse à `Synchroniseur::demander_suspendues`.
    ListeSuspendues(Result<Vec<Suspendue>, ErreurPoste>),
    /// Connexion d'un compte faite par le synchroniseur : connexion différée
    /// (serveur injoignable lors de `connecter`) ou reprise après une
    /// coupure. Sur un refus (mot de passe changé ailleurs, compte
    /// désactivé), le compte n'est plus connecté.
    Compte(Result<CompteConnecte, ErreurPoste>),
    /// Avertissement sur les sauvegardes du serveur, pour un compte Admin
    /// (décision 56) : à la connexion d'un compte, puis à chaque changement
    /// (relu toutes les minutes environ) ; `None` l'efface.
    Sauvegarde(Option<AvertissementSauvegarde>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErreurPoste {
    /// Connexion au serveur coupée : nouvel essai en cours.
    HorsConnexion,
    /// État final (certificat, poste retiré, version) : rien n'est envoyé.
    Injoignable,
    LectureSeule {
        echeance: Option<String>,
    },
    /// Pas de réponse à temps : l'écriture sera confirmée à la reconnexion
    /// (`Evenement::Confirmee`), il ne faut pas la refaire.
    Delai,
    /// Identifiant ou mot de passe refusé ; attente avant le prochain essai.
    CompteRefuse {
        attente_secondes: u64,
    },
    /// Refus du serveur, à traduire comme les messages du stockage.
    Refus {
        modele: String,
        valeurs: Vec<String>,
    },
    /// Erreur technique (base, protocole) : message brut.
    Base(String),
}

impl fmt::Display for ErreurPoste {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErreurPoste::HorsConnexion => f.write_str("serveur injoignable"),
            ErreurPoste::Injoignable => f.write_str("connexion au serveur impossible"),
            ErreurPoste::LectureSeule { .. } => {
                f.write_str("abonnement échu : Kadenz est en lecture seule")
            }
            ErreurPoste::Delai => f.write_str("le serveur n'a pas répondu à temps"),
            ErreurPoste::CompteRefuse { attente_secondes } => write!(
                f,
                "identifiant ou mot de passe incorrect (nouvel essai dans {attente_secondes} s)"
            ),
            ErreurPoste::Refus { modele, valeurs } => {
                let mut valeurs = valeurs.iter();
                let mut reste = modele.as_str();
                while let Some(position) = reste.find("{}") {
                    f.write_str(&reste[..position])?;
                    f.write_str(valeurs.next().map_or("{}", String::as_str))?;
                    reste = &reste[position + 2..];
                }
                f.write_str(reste)
            }
            ErreurPoste::Base(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ErreurPoste {}

fn base(e: impl fmt::Display) -> ErreurPoste {
    ErreurPoste::Base(e.to_string())
}

/// Refus du stockage à traduire (mot de passe d'une sauvegarde, base V1
/// introuvable) ; une autre erreur reste technique.
fn stockage(e: kadenz_stockage::Erreur) -> ErreurPoste {
    match e {
        kadenz_stockage::Erreur::Validation(modele) => ErreurPoste::Refus {
            modele,
            valeurs: Vec::new(),
        },
        kadenz_stockage::Erreur::ValidationComposee { modele, valeurs } => {
            ErreurPoste::Refus { modele, valeurs }
        }
        autre => base(autre),
    }
}

/// Sauvegarde `.kzs` de la source et son mot de passe ; `true` si elle a été
/// faite pour l'occasion dans `dossier` (base V1 du poste), à effacer après
/// usage.
fn sauvegarde_v1(
    dossier: &Path,
    source: &SourceV1,
) -> Result<(PathBuf, String, bool), ErreurPoste> {
    match source {
        SourceV1::Sauvegarde {
            chemin,
            mot_de_passe,
        } => Ok((chemin.clone(), mot_de_passe.clone(), false)),
        SourceV1::Base(chemin) => {
            let kzs = dossier.join("reprise-v1.kzs");
            // Mot de passe à usage unique : la sauvegarde ne sert qu'à l'envoi.
            let mot_de_passe = synchro::nouvel_uid();
            kadenz_stockage::reprise::exporter_base_v1(chemin, &kzs, &mot_de_passe)
                .map_err(stockage)?;
            Ok((kzs, mot_de_passe, true))
        }
    }
}

/// Refus d'une base V1 trop grande pour la reprise ; `{}` : taille permise en
/// mégaoctets.
pub const MODELE_TROP_GRANDE: &str = "la base V1 dépasse la taille permise pour la reprise ({} Mo)";

enum Commande {
    Connecter {
        identifiant: String,
        mot_de_passe: String,
        retour: Sender<Result<CompteConnecte, ErreurPoste>>,
    },
    Deconnecter,
    Ecrire {
        action: Box<Action>,
        retour: Sender<Result<Issue, ErreurPoste>>,
    },
    Renouveler {
        cle: String,
        retour: Sender<Result<Abonnement, ErreurPoste>>,
    },
    /// Travail sur la copie ou la session, dans le fil.
    Fil(Box<dyn FnOnce(&mut Fil) + Send>),
    /// Fin d'une tentative de connexion menée par le fil auxiliaire.
    /// Session ouverte, avec la génération de la base du serveur (F7).
    /// Session ouverte : client, accueil du serveur, et vrai si elle passe
    /// par le relais (F8).
    Connectee(Result<(Box<Client>, Bienvenue, bool), ErreurClient>),
    Arreter,
}

/// Rappel des événements : appelé depuis le fil du synchroniseur.
pub type Rappel = Box<dyn Fn(Evenement) + Send>;

/// Base V1 à reprendre dans le serveur (F7, écrans, section 8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceV1 {
    /// Base V1 du poste (`kadenz.sqlite`) : jamais ouverte ni modifiée, une
    /// copie de ses fichiers est sauvegardée pour l'envoi (décision 45).
    Base(PathBuf),
    /// Sauvegarde `.kzs` de la V1, avec son mot de passe.
    Sauvegarde {
        chemin: PathBuf,
        mot_de_passe: String,
    },
}

/// Attente de la réponse du serveur à une reprise : il verse toute la base.
pub const DELAI_REPRISE: Duration = Duration::from_secs(120);

pub struct Synchroniseur {
    commandes: Sender<Commande>,
    etat: Arc<Mutex<EtatConnexion>>,
    a_envoyer: Arc<AtomicU32>,
    fil: Option<JoinHandle<()>>,
    /// Dossier de la copie, pour la sauvegarde envoyée par une reprise.
    dossier: PathBuf,
    /// La session en cours passe par le relais (F8).
    distant: Arc<AtomicBool>,
}

impl Synchroniseur {
    /// Lit le lien au serveur dans la copie, ouvre une connexion à la copie
    /// pour le fil et part en connexion.
    pub fn demarrer(copie: PathBuf, evenements: Rappel) -> Result<Synchroniseur, ErreurPoste> {
        let dossier = copie
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let conn = poste_relie::ouvrir_copie(&copie).map_err(base)?;
        // Modules de la licence gardée, sur la connexion du fil : une
        // écriture hors connexion est appliquée dans la copie avec eux.
        poste_relie::appliquer_modules(&conn).map_err(base)?;
        let poste = poste_relie::lire(&conn)
            .map_err(base)?
            .ok_or_else(|| ErreurPoste::Base("la copie n'est reliée à aucun serveur".into()))?;
        let entreprise = synchro::identifiant_entreprise(&conn).map_err(base)?;
        let a_envoyer = Arc::new(AtomicU32::new(replique::a_envoyer(&conn).map_err(base)?));
        let etat = Arc::new(Mutex::new(EtatConnexion::Connexion));
        let distant = Arc::new(AtomicBool::new(false));
        let (commandes, reception) = mpsc::channel();
        let identite = Identite {
            certificat: poste.certificat.clone(),
            cle: poste.cle.clone(),
        };
        let mut fil = Fil {
            relais: commandes.clone(),
            essai_en_cours: false,
            connexion_attendue: None,
            copie: conn,
            poste,
            identite,
            entreprise,
            evenements,
            etat: etat.clone(),
            a_envoyer: a_envoyer.clone(),
            client: None,
            compte: None,
            echecs: 0,
            prochain_essai: Instant::now(),
            releves: 0,
            lecture_seule: None,
            suspendues_vues: None,
            sauvegarde_vue: None,
            distant: distant.clone(),
            relais_d_abord: false,
        };
        let fil = std::thread::Builder::new()
            .name("kadenz-synchroniseur".into())
            .spawn(move || fil.tourner(reception))
            .map_err(base)?;
        Ok(Synchroniseur {
            commandes,
            etat,
            a_envoyer,
            fil: Some(fil),
            dossier,
            distant,
        })
    }

    /// Connexion d'un compte par le serveur. La copie peut ne pas contenir
    /// encore le compte (première synchronisation) : l'état passe alors par
    /// `Synchronisation`. Serveur injoignable (hors état final) : rend
    /// `HorsConnexion` et garde le compte, connecté de lui-même au retour du
    /// serveur (`Evenement::Compte`), décision 48.
    pub fn connecter(
        &self,
        identifiant: &str,
        mot_de_passe: &str,
    ) -> Result<CompteConnecte, ErreurPoste> {
        let (retour, reponse) = mpsc::channel();
        self.envoyer(Commande::Connecter {
            identifiant: identifiant.into(),
            mot_de_passe: mot_de_passe.into(),
            retour,
        })?;
        // Connexion réseau, puis vérification du mot de passe par le serveur.
        reponse
            .recv_timeout(DELAI * 2)
            .unwrap_or(Err(ErreurPoste::Delai))
    }

    /// Ferme la session du compte (le poste reste relié au serveur).
    pub fn deconnecter(&self) {
        let _ = self.commandes.send(Commande::Deconnecter);
    }

    /// Envoie une écriture et attend son résultat ; ne rend la main qu'avec
    /// la copie à jour (la ligne créée y est), en `DELAI` au plus. Hors
    /// connexion, une écriture permise (décision 50) est appliquée dans la
    /// copie et mise en file : `Issue::EnFile`, son résultat viendra par
    /// `Evenement::Confirmee` ; une autre rend `HorsConnexion`.
    pub fn ecrire(&self, action: Action) -> Result<Issue, ErreurPoste> {
        let (retour, reponse) = mpsc::channel();
        self.envoyer(Commande::Ecrire {
            action: Box::new(action),
            retour,
        })?;
        reponse
            .recv_timeout(DELAI)
            .unwrap_or(Err(ErreurPoste::Delai))
    }

    /// Installe une nouvelle licence d'abonnement sur le serveur.
    pub fn renouveler(&self, cle: &str) -> Result<Abonnement, ErreurPoste> {
        let (retour, reponse) = mpsc::channel();
        self.envoyer(Commande::Renouveler {
            cle: cle.into(),
            retour,
        })?;
        reponse
            .recv_timeout(DELAI * 2)
            .unwrap_or(Err(ErreurPoste::Delai))
    }

    /// Nombre d'écritures qui attendent d'être envoyées au serveur.
    pub fn file(&self) -> u32 {
        self.a_envoyer.load(Ordering::Relaxed)
    }

    /// Note un brouillon à valider au retour du serveur (écrans, section 4).
    pub fn valider_au_retour(&self, document: &str) -> Result<(), ErreurPoste> {
        let document = document.to_string();
        self.sur_fil(move |fil| {
            poste_relie::noter_a_valider(&fil.copie, &document).map_err(base)?;
            fil.signaler_a_valider();
            Ok(())
        })
    }

    /// Retire un brouillon de la liste « à valider au retour ».
    pub fn oublier_a_valider(&self, document: &str) -> Result<(), ErreurPoste> {
        let document = document.to_string();
        self.sur_fil(move |fil| {
            poste_relie::oublier_a_valider(&fil.copie, &document).map_err(base)?;
            fil.signaler_a_valider();
            Ok(())
        })
    }

    /// Opérations suspendues sur le serveur (conflits, attentes) que le
    /// compte connecté peut voir ; à trancher par `ecrire` avec
    /// `Action::TrancherConflit` ou `Action::DeciderAttente`.
    pub fn suspendues(&self) -> Result<Vec<Suspendue>, ErreurPoste> {
        self.sur_fil(|fil| fil.suspendues())
    }

    /// Comme `suspendues`, sans attendre : la liste arrive par
    /// `Evenement::ListeSuspendues`, et le nombre par `Evenement::Suspendues`
    /// s'il a changé.
    pub fn demander_suspendues(&self) {
        let _ = self.envoyer(Commande::Fil(Box::new(|fil: &mut Fil| {
            let liste = fil.suspendues();
            if let Ok(l) = &liste {
                fil.noter_suspendues(l, false);
            }
            (fil.evenements)(Evenement::ListeSuspendues(liste));
        })));
    }

    /// Poignée pour les appels longs (reprise d'une base V1), utilisable
    /// depuis un autre fil que celui de l'écran.
    pub fn poignee(&self) -> Poignee {
        Poignee {
            commandes: self.commandes.clone(),
            dossier: self.dossier.clone(),
            distant: self.distant.clone(),
        }
    }

    /// Voir [`Poignee::distant`].
    pub fn distant(&self) -> bool {
        self.distant.load(Ordering::SeqCst)
    }

    /// Voir [`Poignee::acces_distant`].
    pub fn acces_distant(&self) -> Result<EtatAccesDistant, ErreurPoste> {
        self.poignee().acces_distant()
    }

    /// Voir [`Poignee::regler_acces_distant`].
    pub fn regler_acces_distant(&self, actif: bool) -> Result<EtatAccesDistant, ErreurPoste> {
        self.poignee().regler_acces_distant(actif)
    }

    /// Voir [`Poignee::autoriser_distance`].
    pub fn autoriser_distance(
        &self,
        poste: &str,
        autorise: bool,
    ) -> Result<EtatAccesDistant, ErreurPoste> {
        self.poignee().autoriser_distance(poste, autorise)
    }

    /// Voir [`Poignee::resume_reprise`].
    pub fn resume_reprise(&self, source: &SourceV1) -> Result<ResumeReprise, ErreurPoste> {
        self.poignee().resume_reprise(source)
    }

    /// Voir [`Poignee::reprise_possible`].
    pub fn reprise_possible(&self) -> Result<bool, ErreurPoste> {
        self.poignee().reprise_possible()
    }

    /// Voir [`Poignee::reprendre`].
    pub fn reprendre(&self, source: &SourceV1) -> Result<ResumeReprise, ErreurPoste> {
        self.poignee().reprendre(source)
    }

    pub fn etat(&self) -> EtatConnexion {
        self.etat
            .lock()
            .map(|e| e.clone())
            .unwrap_or(EtatConnexion::Connexion)
    }

    /// Arrête le fil et attend sa fin.
    pub fn arreter(mut self) {
        let _ = self.commandes.send(Commande::Arreter);
        if let Some(fil) = self.fil.take() {
            let _ = fil.join();
        }
    }

    fn envoyer(&self, commande: Commande) -> Result<(), ErreurPoste> {
        self.commandes
            .send(commande)
            .map_err(|_| ErreurPoste::Base("synchroniseur arrêté".into()))
    }

    fn sur_fil<T: Send + 'static>(
        &self,
        travail: impl FnOnce(&mut Fil) -> Result<T, ErreurPoste> + Send + 'static,
    ) -> Result<T, ErreurPoste> {
        let (retour, reponse) = mpsc::channel();
        self.envoyer(Commande::Fil(Box::new(move |fil: &mut Fil| {
            let _ = retour.send(travail(fil));
        })))?;
        reponse
            .recv_timeout(DELAI * 2)
            .unwrap_or(Err(ErreurPoste::Delai))
    }
}

impl Drop for Synchroniseur {
    fn drop(&mut self) {
        let _ = self.commandes.send(Commande::Arreter);
    }
}

/// Poignée du synchroniseur, `Clone` et `Send` : ses appels attendent la
/// réponse du fil du synchroniseur, et se font donc hors du fil de l'écran.
/// Elle n'arrête pas le synchroniseur ; après son arrêt, ses appels rendent
/// une erreur.
#[derive(Clone)]
pub struct Poignee {
    commandes: Sender<Commande>,
    dossier: PathBuf,
    distant: Arc<AtomicBool>,
}

impl Poignee {
    /// Vrai tant que la session en cours passe par le relais (F8) : la barre
    /// d'état dit alors « Connecté à distance ». Posé avant l'événement
    /// `Evenement::Etat` qui annonce la session.
    pub fn distant(&self) -> bool {
        self.distant.load(Ordering::SeqCst)
    }

    /// État de l'accès à distance (F8), compte Admin seul ; `HorsConnexion`
    /// sans serveur.
    pub fn acces_distant(&self) -> Result<EtatAccesDistant, ErreurPoste> {
        self.sur_fil(DELAI * 2, |fil| fil.requete_distance(|c| c.acces_distant()))
    }

    /// Active ou désactive l'accès à distance (F8), compte Admin seul.
    pub fn regler_acces_distant(&self, actif: bool) -> Result<EtatAccesDistant, ErreurPoste> {
        self.sur_fil(DELAI * 2, move |fil| {
            fil.requete_distance(|c| c.regler_acces_distant(actif))
        })
    }

    /// Coche ou décoche « accès à distance » pour un poste inscrit (F8,
    /// décision 57), compte Admin seul.
    pub fn autoriser_distance(
        &self,
        poste: &str,
        autorise: bool,
    ) -> Result<EtatAccesDistant, ErreurPoste> {
        let poste = poste.to_string();
        self.sur_fil(DELAI * 2, move |fil| {
            fil.requete_distance(|c| c.autoriser_distance(&poste, autorise))
        })
    }

    /// Résumé d'une base V1 avant sa reprise (F7), lu sur ce poste, sans le
    /// serveur : entreprise, tiers, produits, règlements, documents et dernier
    /// numéro par type, comptes repris et comptes écartés parce que leur
    /// identifiant existe déjà dans la copie (décision 53).
    pub fn resume_reprise(&self, source: &SourceV1) -> Result<ResumeReprise, ErreurPoste> {
        let (kzs, mot_de_passe, temporaire) = sauvegarde_v1(&self.dossier, source)?;
        let lu = kadenz_stockage::ouvrir_sauvegarde(&kzs, &mot_de_passe)
            .and_then(|conn| kadenz_stockage::reprise::resume(&conn));
        if temporaire {
            let _ = std::fs::remove_file(&kzs);
        }
        let mut resume = lu.map_err(stockage)?;
        self.sur_fil(DELAI * 2, move |fil| {
            kadenz_stockage::reprise::separer_comptes(&mut resume, &fil.copie).map_err(base)?;
            Ok(resume)
        })
    }

    /// Le serveur est-il neuf, prêt pour une reprise (décision 55) ? Demande
    /// au serveur, avec un compte connecté. Après un `Delai` de `reprendre`,
    /// `Ok(false)` dit que la reprise est faite : elle est d'un seul tenant.
    pub fn reprise_possible(&self) -> Result<bool, ErreurPoste> {
        self.sur_fil(DELAI * 2, |fil| fil.reprise_possible())
    }

    /// Reprend la base V1 dans le serveur (F7), compte Admin seul ; rend le
    /// bilan mesuré sur le serveur. Refus à traduire : serveur qui n'est pas
    /// neuf ([`kadenz_stockage::reprise::MODELE_PAS_NEUF`]), base trop grande
    /// ([`MODELE_TROP_GRANDE`]), base introuvable, mot de passe de la
    /// sauvegarde, droit manquant. Attend jusqu'à [`DELAI_REPRISE`] ; les
    /// lignes reprises arrivent ensuite dans la copie par les changements
    /// habituels.
    pub fn reprendre(&self, source: &SourceV1) -> Result<ResumeReprise, ErreurPoste> {
        let source = source.clone();
        let dossier = self.dossier.clone();
        self.sur_fil(DELAI_REPRISE + DELAI * 2, move |fil| {
            let (kzs, mot_de_passe, temporaire) = sauvegarde_v1(&dossier, &source)?;
            let lu = std::fs::read(&kzs);
            if temporaire {
                let _ = std::fs::remove_file(&kzs);
            }
            let octets = lu.map_err(base)?;
            if octets.len() > TAILLE_MAX_REPRISE {
                return Err(ErreurPoste::Refus {
                    modele: MODELE_TROP_GRANDE.into(),
                    valeurs: vec![(TAILLE_MAX_REPRISE / (1024 * 1024)).to_string()],
                });
            }
            fil.reprendre_v1(&octets, &mot_de_passe)
        })
    }

    fn sur_fil<T: Send + 'static>(
        &self,
        delai: Duration,
        travail: impl FnOnce(&mut Fil) -> Result<T, ErreurPoste> + Send + 'static,
    ) -> Result<T, ErreurPoste> {
        let (retour, reponse) = mpsc::channel();
        self.commandes
            .send(Commande::Fil(Box::new(move |fil: &mut Fil| {
                let _ = retour.send(travail(fil));
            })))
            .map_err(|_| ErreurPoste::Base("synchroniseur arrêté".into()))?;
        reponse
            .recv_timeout(delai)
            .unwrap_or(Err(ErreurPoste::Delai))
    }
}

/// Compte connecté : de quoi le reconnecter après une coupure (en mémoire
/// seulement).
struct Compte {
    identifiant: String,
    mot_de_passe: String,
    /// `None` tant que le serveur n'a pas encore connecté le compte.
    uid: Option<String>,
}

struct Fil {
    /// Pour le fil auxiliaire de connexion. Le fil garde ainsi l'envoi
    /// ouvert : il s'arrête sur `Commande::Arreter`, envoyée par `Drop`.
    relais: Sender<Commande>,
    /// Une tentative de connexion est en cours dans le fil auxiliaire.
    essai_en_cours: bool,
    /// Connexion d'un compte qui attend la fin de cette tentative.
    connexion_attendue: Option<Sender<Result<CompteConnecte, ErreurPoste>>>,
    copie: Connection,
    poste: PosteRelie,
    identite: Identite,
    entreprise: String,
    evenements: Rappel,
    etat: Arc<Mutex<EtatConnexion>>,
    a_envoyer: Arc<AtomicU32>,
    client: Option<Client>,
    compte: Option<Compte>,
    echecs: u32,
    prochain_essai: Instant,
    releves: u32,
    /// Lecture seule selon le dernier abonnement lu : son échéance.
    lecture_seule: Option<Option<String>>,
    /// Derniers nombres de suspendues signalés (conflits, attentes).
    suspendues_vues: Option<(u32, u32)>,
    /// Dernier avertissement des sauvegardes du serveur signalé (F7).
    sauvegarde_vue: Option<Option<AvertissementSauvegarde>>,
    /// La session en cours passe par le relais (F8).
    distant: Arc<AtomicBool>,
    /// La dernière session ouverte passait par le relais : le prochain essai
    /// commence par lui.
    relais_d_abord: bool,
}

impl Fil {
    fn tourner(&mut self, commandes: mpsc::Receiver<Commande>) {
        loop {
            let attente = if self.client.is_some()
                || self.essai_en_cours
                || self.etat_present().definitif()
            {
                RELEVE
            } else {
                self.prochain_essai
                    .saturating_duration_since(Instant::now())
                    .max(Duration::from_millis(50))
            };
            match commandes.recv_timeout(attente) {
                Ok(Commande::Arreter) | Err(RecvTimeoutError::Disconnected) => break,
                Ok(commande) => self.traiter(commande),
                Err(RecvTimeoutError::Timeout) => self.periodique(),
            }
        }
    }

    fn etat_present(&self) -> EtatConnexion {
        self.etat
            .lock()
            .map(|e| e.clone())
            .unwrap_or(EtatConnexion::Connexion)
    }

    fn poser_etat(&self, etat: EtatConnexion) {
        let change = match self.etat.lock() {
            Ok(mut present) if *present != etat => {
                *present = etat.clone();
                true
            }
            _ => false,
        };
        if change {
            (self.evenements)(Evenement::Etat(etat));
        }
    }

    /// Garde (ou oublie) l'opération sans réponse, et le signale à l'écran.
    fn garder(&self, op: Option<&Operation>) -> Result<(), ErreurPoste> {
        replique::garder_en_cours(&self.copie, op).map_err(base)?;
        self.signaler_file();
        Ok(())
    }

    /// Nombre d'écritures à envoyer, signalé à l'écran s'il a changé.
    fn signaler_file(&self) {
        let n = replique::a_envoyer(&self.copie).unwrap_or(0);
        if self.a_envoyer.swap(n, Ordering::Relaxed) != n {
            (self.evenements)(Evenement::File(n));
        }
    }

    /// Nombre de brouillons à valider au retour, signalé à l'écran.
    fn signaler_a_valider(&self) -> u32 {
        let n = poste_relie::a_valider(&self.copie).map_or(0, |l| l.len() as u32);
        (self.evenements)(Evenement::AValider(n));
        n
    }

    /// Connexion d'un compte refusée par le serveur : ses écritures de la
    /// file partiront sous la session d'un autre compte (décision 52).
    fn noter_refus(&self, identifiant: &str) {
        if let Ok(Some(uid)) = poste_relie::uid_du_compte(&self.copie, identifiant) {
            let _ = poste_relie::noter_compte_refuse(&self.copie, &uid);
        }
    }

    /// Compte de la session connecté par le serveur.
    fn uid_session(&self) -> Option<String> {
        self.compte.as_ref().and_then(|c| c.uid.clone())
    }

    /// État d'une session ouverte, selon l'abonnement.
    fn etat_ouvert(&self) -> EtatConnexion {
        match &self.lecture_seule {
            Some(echeance) => EtatConnexion::LectureSeule {
                echeance: echeance.clone(),
            },
            None => EtatConnexion::Connecte,
        }
    }

    fn traiter(&mut self, commande: Commande) {
        match commande {
            Commande::Connecter {
                identifiant,
                mot_de_passe,
                retour,
            } => self.connecter(identifiant, mot_de_passe, retour),
            Commande::Deconnecter => {
                // Le protocole n'a pas de fin de session de compte : la
                // session TLS est refermée et rouverte sans compte.
                self.compte = None;
                self.suspendues_vues = None;
                self.sauvegarde_vue = None;
                self.connexion_attendue = None;
                self.client = None;
                self.distant.store(false, Ordering::SeqCst);
                self.prochain_essai = Instant::now();
            }
            Commande::Ecrire { action, retour } => {
                let resultat = self.ecrire((*action).clone());
                // L'écran n'attend plus (délai dépassé) : il apprend le
                // résultat par un événement.
                if let Err(mpsc::SendError(Ok(issue))) = retour.send(resultat) {
                    (self.evenements)(Evenement::Confirmee { action, issue });
                }
            }
            Commande::Renouveler { cle, retour } => {
                let resultat = self.renouveler(&cle);
                let _ = retour.send(resultat);
            }
            Commande::Fil(travail) => travail(self),
            Commande::Connectee(resultat) => self.connectee(resultat),
            Commande::Arreter => {}
        }
    }

    fn periodique(&mut self) {
        if self.etat_present().definitif() {
            return;
        }
        if self.client.is_none() {
            if Instant::now() >= self.prochain_essai {
                self.lancer_essai();
            }
            return;
        }
        if self.compte.is_none() {
            return;
        }
        self.releves += 1;
        if self.releves >= RELEVES_ABONNEMENT {
            self.releves = 0;
            let _ = self.lire_abonnement();
            // Écritures restées dans la file (bloquées, lecture seule) et
            // suspendues : la décision a pu être prise depuis.
            let _ = self.vider_file(true);
            self.compter_suspendues(false);
            self.lire_sauvegarde(false);
        }
        if self.rattraper().is_ok() {
            self.poser_etat(self.etat_ouvert());
        }
    }

    /// Lance une tentative de connexion dans un fil auxiliaire, s'il n'y en a
    /// pas déjà une : l'ouverture TCP et TLS peut durer plusieurs secondes,
    /// et le fil continue de servir l'écran pendant ce temps (écritures en
    /// file hors connexion). Le résultat revient par `Commande::Connectee`.
    fn lancer_essai(&mut self) {
        if self.essai_en_cours {
            return;
        }
        let adresse = self.poste.adresse.clone();
        let empreinte = self.poste.empreinte_serveur.clone();
        let identite = self.identite.clone();
        let entreprise = self.entreprise.clone();
        let relais = self.relais.clone();
        // Adresse du relais apprise du serveur (F8) : hors du bureau, le poste
        // passe par lui. Le chemin qui a réussi la dernière fois est essayé
        // en premier ; on ne change de chemin que si le premier est
        // injoignable (un certificat différent, un poste retiré ou une autre
        // version sont des réponses du serveur).
        let par_relais = distance::relais_du_poste(&self.copie).ok().flatten();
        let relais_d_abord = self.relais_d_abord;
        let lance = std::thread::Builder::new()
            .name("kadenz-connexion".into())
            .spawn(move || {
                let direct = || {
                    Client::connecter(&adresse, &empreinte, &identite, DELAI)
                        .map(|(c, b)| (Box::new(c), b, false))
                };
                let distant = |r: &str| {
                    Client::connecter_par_relais(r, &entreprise, &empreinte, &identite, DELAI)
                        .map(|(c, b)| (Box::new(c), b, true))
                };
                let resultat = match par_relais.as_deref() {
                    None => direct(),
                    Some(r) if relais_d_abord => match distant(r) {
                        Err(ErreurClient::Injoignable(_)) => direct(),
                        autre => autre,
                    },
                    Some(r) => match direct() {
                        Err(ErreurClient::Injoignable(_)) => distant(r),
                        autre => autre,
                    },
                };
                // Synchroniseur arrêté entre-temps : l'envoi échoue, sans suite.
                let _ = relais.send(Commande::Connectee(resultat));
            });
        match lance {
            Ok(_) => self.essai_en_cours = true,
            Err(e) => self.connectee(Err(ErreurClient::Injoignable(e.to_string()))),
        }
    }

    /// Fin d'une tentative : session TLS ouverte, puis le compte gardé est
    /// connecté ; ou connexion perdue, et la connexion qui attendait rend
    /// `HorsConnexion` (le compte reste gardé, décision 48).
    fn connectee(&mut self, resultat: Result<(Box<Client>, Bienvenue, bool), ErreurClient>) {
        self.essai_en_cours = false;
        let attendue = self.connexion_attendue.take();
        match resultat {
            Ok((client, bienvenue, distant)) => {
                let generation = bienvenue.generation;
                // Relais donné par le serveur à un poste autorisé (F8) ;
                // aucun : le poste n'essaie plus le relais.
                let _ = distance::garder_relais_du_poste(&self.copie, bienvenue.relais.as_deref());
                // Serveur restauré (F7) : la copie est à refaire, avant tout
                // envoi de la file. Les lignes reviennent par le rattrapage ;
                // celles qui ne reviennent pas sont retirées à la fin.
                if let Err(e) = replique::changer_de_generation(&mut self.copie, &generation) {
                    let erreur = base(e);
                    if let Some(retour) = attendue {
                        let _ = retour.send(Err(erreur));
                    }
                    return;
                }
                self.client = Some(*client);
                self.echecs = 0;
                self.distant.store(distant, Ordering::SeqCst);
                self.relais_d_abord = distant;
                self.poser_etat(self.etat_ouvert());
                match self.compte.take() {
                    Some(compte) => self.reprendre(compte, attendue),
                    None => {
                        if let Some(retour) = attendue {
                            let _ = retour.send(Err(ErreurPoste::HorsConnexion));
                        }
                    }
                }
            }
            Err(e) => {
                let erreur = self.perdre(e);
                if let Some(retour) = attendue {
                    let _ = retour.send(Err(erreur));
                }
            }
        }
    }

    /// Connecte le compte gardé sur la session qui vient de s'ouvrir. Une
    /// connexion demandée par l'écran (`attendue`) reçoit le résultat ;
    /// sinon c'est une reprise après coupure, annoncée par un événement.
    fn reprendre(
        &mut self,
        mut compte: Compte,
        attendue: Option<Sender<Result<CompteConnecte, ErreurPoste>>>,
    ) {
        let Some(client) = self.client.as_mut() else {
            self.compte = Some(compte);
            return;
        };
        match client.connexion(&compte.identifiant, &compte.mot_de_passe) {
            Ok(connecte) => {
                let _ = poste_relie::oublier_compte_refuse(&self.copie, &connecte.compte);
                compte.uid = Some(connecte.compte.clone());
                self.compte = Some(compte);
                match attendue {
                    Some(retour) => self.rendre_compte(retour, Ok(connecte)),
                    None => (self.evenements)(Evenement::Compte(Ok(connecte))),
                }
                self.apres_connexion();
            }
            // Mot de passe changé ailleurs, compte désactivé : le compte
            // n'est plus connecté.
            Err(ErreurClient::CompteRefuse { attente_secondes }) => {
                self.noter_refus(&compte.identifiant);
                let refus = Err(ErreurPoste::CompteRefuse { attente_secondes });
                match attendue {
                    Some(retour) => self.rendre_compte(retour, refus),
                    None => (self.evenements)(Evenement::Compte(refus)),
                }
            }
            Err(e) => {
                // Le compte reste gardé pour la prochaine tentative.
                self.compte = Some(compte);
                let erreur = self.perdre(e);
                if let Some(retour) = attendue {
                    let _ = retour.send(Err(erreur));
                }
            }
        }
    }

    /// Après la connexion d'un compte : abonnement, écriture restée sans
    /// réponse, file d'envoi, rattrapage de la copie.
    fn apres_connexion(&mut self) {
        if self.lire_abonnement().is_err() {
            return;
        }
        if let Ok(Some(op)) = replique::operation_en_cours(&self.copie) {
            let action = op.action.clone();
            match self.envoyer(op) {
                Ok(issue) => (self.evenements)(Evenement::Confirmee {
                    action: Box::new(action),
                    issue,
                }),
                Err(ErreurPoste::HorsConnexion | ErreurPoste::Delai) => return,
                Err(_) => {}
            }
        }
        if let Err(ErreurPoste::HorsConnexion) = self.vider_file(true) {
            return;
        }
        if self.rattraper().is_ok() {
            self.poser_etat(self.etat_ouvert());
        }
        // Retour du serveur : l'écran propose de valider les brouillons notés.
        if poste_relie::a_valider(&self.copie).is_ok_and(|l| !l.is_empty()) {
            self.signaler_a_valider();
        }
        self.compter_suspendues(true);
        self.lire_sauvegarde(true);
    }

    /// Connexion perdue ou refusée : état et prochain essai. Rend l'erreur
    /// pour l'écran.
    fn perdre(&mut self, e: ErreurClient) -> ErreurPoste {
        self.client = None;
        self.distant.store(false, Ordering::SeqCst);
        let definitif = match &e {
            ErreurClient::CertificatDifferent => Some(EtatConnexion::CertificatDifferent),
            ErreurClient::PosteNonInscrit => Some(EtatConnexion::PosteRetire),
            ErreurClient::VersionDifferente { serveur } => {
                Some(EtatConnexion::VersionDifferente { serveur: *serveur })
            }
            _ => None,
        };
        if let Some(etat) = definitif {
            self.poser_etat(etat);
            return ErreurPoste::Injoignable;
        }
        self.echecs += 1;
        let rang = (self.echecs as usize - 1).min(ATTENTES.len() - 1);
        self.prochain_essai = Instant::now() + Duration::from_secs(ATTENTES[rang]);
        self.poser_etat(if self.echecs >= ECHECS_INJOIGNABLE {
            EtatConnexion::Injoignable
        } else {
            EtatConnexion::Reconnexion { essai: self.echecs }
        });
        ErreurPoste::HorsConnexion
    }

    fn reprise_possible(&mut self) -> Result<bool, ErreurPoste> {
        if self.etat_present().definitif() {
            return Err(ErreurPoste::Injoignable);
        }
        if self.uid_session().is_none() {
            return Err(ErreurPoste::HorsConnexion);
        }
        let Some(client) = self.client.as_mut() else {
            return Err(ErreurPoste::HorsConnexion);
        };
        match client.reprise_possible() {
            Ok(possible) => Ok(possible),
            Err(e) => Err(self.erreur_requete(e)),
        }
    }

    /// Requête de la rubrique « Accès à distance » (F8).
    fn requete_distance(
        &mut self,
        requete: impl FnOnce(&mut Client) -> Result<EtatAccesDistant, ErreurClient>,
    ) -> Result<EtatAccesDistant, ErreurPoste> {
        if self.etat_present().definitif() {
            return Err(ErreurPoste::Injoignable);
        }
        if self.uid_session().is_none() {
            return Err(ErreurPoste::HorsConnexion);
        }
        let Some(client) = self.client.as_mut() else {
            return Err(ErreurPoste::HorsConnexion);
        };
        match requete(client) {
            Ok(etat) => Ok(etat),
            Err(e) => Err(self.erreur_requete(e)),
        }
    }

    fn reprendre_v1(
        &mut self,
        sauvegarde: &[u8],
        mot_de_passe: &str,
    ) -> Result<ResumeReprise, ErreurPoste> {
        if self.etat_present().definitif() {
            return Err(ErreurPoste::Injoignable);
        }
        if self.uid_session().is_none() {
            return Err(ErreurPoste::HorsConnexion);
        }
        let Some(client) = self.client.as_mut() else {
            return Err(ErreurPoste::HorsConnexion);
        };
        match client.reprise(sauvegarde, mot_de_passe, DELAI_REPRISE) {
            Ok(bilan) => Ok(bilan),
            Err(e) => Err(self.erreur_requete(e)),
        }
    }

    /// Erreur d'une requête : la session reste ouverte pour un refus, elle est
    /// perdue pour une coupure ou une réponse illisible.
    fn erreur_requete(&mut self, e: ErreurClient) -> ErreurPoste {
        match e {
            ErreurClient::CompteRefuse { attente_secondes } => {
                ErreurPoste::CompteRefuse { attente_secondes }
            }
            ErreurClient::Refus { modele, valeurs } => ErreurPoste::Refus { modele, valeurs },
            ErreurClient::LectureSeule { echeance } => {
                self.lecture_seule = Some(echeance.clone());
                self.poser_etat(self.etat_ouvert());
                ErreurPoste::LectureSeule { echeance }
            }
            ErreurClient::Rejet(motif) => ErreurPoste::Base(motif),
            autre => self.perdre(autre),
        }
    }

    fn connecter(
        &mut self,
        identifiant: String,
        mot_de_passe: String,
        retour: Sender<Result<CompteConnecte, ErreurPoste>>,
    ) {
        if self.etat_present().definitif() {
            let _ = retour.send(Err(ErreurPoste::Injoignable));
            return;
        }
        // Une seule session par compte : la précédente est refermée.
        if self.compte.is_some() {
            self.compte = None;
            self.client = None;
            self.distant.store(false, Ordering::SeqCst);
        }
        let Some(client) = self.client.as_mut() else {
            // Sans session : le compte est gardé et la connexion attend la
            // tentative en cours (ou une nouvelle), sans en empiler une
            // seconde. Échec : `HorsConnexion`, et le compte se connectera
            // au retour du serveur (décision 48).
            self.compte = Some(Compte {
                identifiant,
                mot_de_passe,
                uid: None,
            });
            if let Some(ancienne) = self.connexion_attendue.replace(retour) {
                let _ = ancienne.send(Err(ErreurPoste::HorsConnexion));
            }
            self.lancer_essai();
            return;
        };
        match client.connexion(&identifiant, &mot_de_passe) {
            Ok(connecte) => {
                let _ = poste_relie::oublier_compte_refuse(&self.copie, &connecte.compte);
                self.compte = Some(Compte {
                    identifiant,
                    mot_de_passe,
                    uid: Some(connecte.compte.clone()),
                });
                // L'écran reprend la main avant le rattrapage, qui peut être
                // long la première fois.
                self.rendre_compte(retour, Ok(connecte));
                self.apres_connexion();
            }
            Err(e) => {
                if matches!(e, ErreurClient::CompteRefuse { .. }) {
                    self.noter_refus(&identifiant);
                }
                let erreur = self.erreur_requete(e);
                // Session TLS perdue en route (serveur relancé, coupure) :
                // l'écran ouvre la session en local, et le compte se
                // connectera au retour du serveur, comme sans client
                // (décision 48).
                if erreur == ErreurPoste::HorsConnexion {
                    self.compte = Some(Compte {
                        identifiant,
                        mot_de_passe,
                        uid: None,
                    });
                }
                self.rendre_compte(retour, Err(erreur));
            }
        }
    }

    /// Rend le résultat de la connexion d'un compte. Si l'écran n'attend
    /// plus (délai dépassé, session ouverte en local), il l'apprend par un
    /// événement, comme au retour du serveur : compte connecté, ou refusé
    /// (la session locale se ferme).
    fn rendre_compte(
        &self,
        retour: Sender<Result<CompteConnecte, ErreurPoste>>,
        resultat: Result<CompteConnecte, ErreurPoste>,
    ) {
        if let Err(mpsc::SendError(r)) = retour.send(resultat)
            && matches!(r, Ok(_) | Err(ErreurPoste::CompteRefuse { .. }))
        {
            (self.evenements)(Evenement::Compte(r));
        }
    }

    fn lire_abonnement(&mut self) -> Result<(), ErreurPoste> {
        let Some(client) = self.client.as_mut() else {
            return Err(ErreurPoste::HorsConnexion);
        };
        match client.abonnement() {
            Ok(abonnement) => {
                self.noter_abonnement(abonnement);
                Ok(())
            }
            Err(e) => Err(self.erreur_requete(e)),
        }
    }

    fn noter_abonnement(&mut self, abonnement: Abonnement) {
        self.lecture_seule = abonnement
            .lecture_seule()
            .then(|| abonnement.licence.as_ref().map(|l| l.echeance.clone()));
        // Gardé dans la copie pour un démarrage hors connexion (lecture
        // seule, bandeaux, modules).
        let _ = kadenz_stockage::abonnement::garder(&self.copie, &abonnement);
        let _ = poste_relie::appliquer_modules(&self.copie);
        (self.evenements)(Evenement::Abonnement(Box::new(abonnement)));
        self.poser_etat(self.etat_ouvert());
    }

    /// Applique les changements du serveur jusqu'au dernier.
    fn rattraper(&mut self) -> Result<(), ErreurPoste> {
        let mut annonce = false;
        loop {
            let depuis = replique::dernier_recu(&self.copie).map_err(base)?;
            let Some(client) = self.client.as_mut() else {
                return Err(ErreurPoste::HorsConnexion);
            };
            let (page, dernier) = match client.changements(depuis, PAGE) {
                Ok(reponse) => reponse,
                Err(e) => return Err(self.erreur_requete(e)),
            };
            if page.is_empty() {
                // À jour après une restauration du serveur : les lignes qui ne
                // sont pas revenues n'existent plus sur le serveur.
                if replique::a_revoir(&self.copie).map_err(base)? > 0 {
                    let bilan =
                        replique::retirer_lignes_disparues(&mut self.copie).map_err(base)?;
                    if !bilan.sortes.is_empty() {
                        (self.evenements)(Evenement::Changements {
                            sortes: bilan.sortes,
                        });
                    }
                    if poste_relie::purger_a_valider(&self.copie).map_err(base)? {
                        self.signaler_a_valider();
                    }
                }
                break;
            }
            if !annonce && dernier.saturating_sub(depuis) > RETARD_ANNONCE {
                annonce = true;
                self.poser_etat(EtatConnexion::Synchronisation);
            }
            let bilan = replique::recevoir(&mut self.copie, &page).map_err(base)?;
            if !bilan.sortes.is_empty() {
                (self.evenements)(Evenement::Changements {
                    sortes: bilan.sortes,
                });
            }
            // Brouillons validés ou supprimés : hors de la liste à valider.
            if poste_relie::purger_a_valider(&self.copie).map_err(base)? {
                self.signaler_a_valider();
            }
            if annonce {
                (self.evenements)(Evenement::Avancement {
                    recus: bilan.dernier,
                    dernier,
                });
            }
        }
        if annonce {
            self.poser_etat(self.etat_ouvert());
        }
        Ok(())
    }

    fn ecrire(&mut self, action: Action) -> Result<Issue, ErreurPoste> {
        if self.etat_present().definitif() {
            return Err(ErreurPoste::Injoignable);
        }
        if self.compte.is_none() {
            return Err(ErreurPoste::Base("aucun compte connecté".into()));
        }
        // Compte pas encore connecté par le serveur (décision 48) ou
        // serveur injoignable : hors connexion.
        let (Some(uid), true) = (self.uid_session(), self.client.is_some()) else {
            return self.hors_ligne(action);
        };
        // Écriture précédente restée sans réponse : confirmée d'abord.
        if let Some(op) = replique::operation_en_cours(&self.copie).map_err(base)? {
            let precedente = op.action.clone();
            match self.envoyer(op) {
                Ok(issue) => (self.evenements)(Evenement::Confirmee {
                    action: Box::new(precedente),
                    issue,
                }),
                // Coupure : la nouvelle écriture la suit dans la file.
                Err(ErreurPoste::HorsConnexion | ErreurPoste::Delai) => {
                    return self.hors_ligne(action);
                }
                Err(_) => {}
            }
        }
        // File d'envoi d'abord, pour garder l'ordre des écritures du poste.
        match self.vider_file(false) {
            Ok(()) => {}
            Err(ErreurPoste::HorsConnexion) => return self.hors_ligne(action),
            Err(e) => return Err(e),
        }
        let decision = matches!(
            action,
            Action::TrancherConflit { .. } | Action::DeciderAttente { .. }
        );
        let op = Operation {
            id: synchro::nouvel_uid(),
            entreprise: self.entreprise.clone(),
            poste: self.identite.empreinte(),
            compte: uid,
            hors_connexion: false,
            heure_poste: None,
            base: replique::dernier_recu(&self.copie).map_err(base)?,
            action,
        };
        let permise = op.action.en_coupure() == EnCoupure::Permise;
        let issue = match self.envoyer(op.clone()) {
            // Coupure vue pendant l'envoi : une écriture permise hors
            // connexion passe dans la file avec le même identifiant, comme si
            // la coupure était connue avant (décision 50) ; le serveur ne
            // l'appliquera pas deux fois s'il l'avait déjà reçue.
            Err(ErreurPoste::Delai) if permise => self.verser_garde(op)?,
            autre => autre?,
        };
        // Décision prise depuis ce poste : ses écritures suspendues sont
        // renvoyées tout de suite pour l'apprendre.
        if decision && matches!(issue, Issue::Acceptee { .. }) {
            let _ = self.vider_file(true);
            let _ = self.rattraper();
            self.compter_suspendues(false);
        }
        Ok(issue)
    }

    /// Envoie une opération gardée d'abord dans la copie, puis rattrape la
    /// copie. Une coupure laisse l'opération gardée (`Delai`).
    fn envoyer(&mut self, op: Operation) -> Result<Issue, ErreurPoste> {
        replique::garder_en_cours(&self.copie, Some(&op)).map_err(base)?;
        let Some(client) = self.client.as_mut() else {
            self.garder(Some(&op))?;
            return Err(ErreurPoste::Delai);
        };
        match client.envoyer(&op) {
            Ok(issue) => {
                self.garder(None)?;
                // Une coupure pendant le rattrapage n'enlève rien au résultat ;
                // la copie sera rattrapée à la reconnexion.
                let _ = self.rattraper();
                Ok(issue)
            }
            Err(e @ (ErreurClient::Injoignable(_) | ErreurClient::Protocole(_))) => {
                self.garder(Some(&op))?;
                self.perdre(e);
                Err(ErreurPoste::Delai)
            }
            Err(e) => {
                // Refusée sans être gardée par le serveur (lecture seule,
                // requête rejetée) : rien à renvoyer.
                self.garder(None)?;
                Err(self.erreur_requete(e))
            }
        }
    }

    /// Écriture hors connexion : appliquée à titre provisoire dans la copie et
    /// mise en file si elle est permise (décision 50), par le compte connecté,
    /// même en local (décision 51). Un refus de la copie (droit, règle) est
    /// rendu tout de suite, sans rien garder.
    fn hors_ligne(&mut self, action: Action) -> Result<Issue, ErreurPoste> {
        if action.en_coupure() != EnCoupure::Permise {
            return Err(ErreurPoste::HorsConnexion);
        }
        if let Some(echeance) = &self.lecture_seule
            && !action.personnelle()
        {
            return Err(ErreurPoste::LectureSeule {
                echeance: echeance.clone(),
            });
        }
        let compte = match self.compte.as_ref() {
            None => return Err(ErreurPoste::Base("aucun compte connecté".into())),
            Some(Compte { uid: Some(uid), .. }) => uid.clone(),
            // Copie jamais synchronisée avec ce compte : rien à faire
            // hors connexion.
            Some(Compte { identifiant, .. }) => {
                poste_relie::uid_du_compte(&self.copie, identifiant)
                    .map_err(base)?
                    .ok_or(ErreurPoste::HorsConnexion)?
            }
        };
        let op = Operation {
            id: synchro::nouvel_uid(),
            entreprise: self.entreprise.clone(),
            poste: self.identite.empreinte(),
            compte,
            hors_connexion: true,
            heure_poste: Some(chrono::Local::now().to_rfc3339()),
            base: replique::dernier_recu(&self.copie).map_err(base)?,
            action,
        };
        self.verser_en_file(op)
    }

    /// Écriture gardée sans réponse après une coupure (`envoyer`) : versée
    /// dans la file telle quelle, puis oubliée comme opération en cours. Si la
    /// copie la refuse, elle reste gardée et attend le retour (`Delai`).
    fn verser_garde(&mut self, op: Operation) -> Result<Issue, ErreurPoste> {
        match self.verser_en_file(op) {
            Ok(Issue::EnFile) => {
                self.garder(None)?;
                Ok(Issue::EnFile)
            }
            _ => Err(ErreurPoste::Delai),
        }
    }

    /// Applique une opération à titre provisoire dans la copie et la met en
    /// file, ensemble ou pas du tout : `Issue::EnFile`, ou le refus de la
    /// copie, rendu sans rien garder.
    fn verser_en_file(&mut self, op: Operation) -> Result<Issue, ErreurPoste> {
        // Appliquée dans la copie comme en ligne : un droit manquant y est
        // refusé, et non mis en attente.
        let locale = Operation {
            hors_connexion: false,
            ..op.clone()
        };
        self.copie
            .execute_batch("SAVEPOINT file_envoi")
            .map_err(base)?;
        let essai = replique::mettre_en_file(&self.copie, &op)
            .and_then(|lignes| Ok((lignes, synchro::appliquer(&mut self.copie, &locale)?)));
        match essai {
            Ok((lignes, Issue::Acceptee { .. })) => {
                self.copie
                    .execute_batch("RELEASE file_envoi")
                    .map_err(base)?;
                self.signaler_file();
                let sortes: BTreeSet<Entite> = lignes.into_iter().map(|(e, _)| e).collect();
                if !sortes.is_empty() {
                    (self.evenements)(Evenement::Changements { sortes });
                }
                Ok(Issue::EnFile)
            }
            autre => {
                self.copie
                    .execute_batch("ROLLBACK TO file_envoi; RELEASE file_envoi")
                    .map_err(base)?;
                match autre {
                    Ok((_, issue)) => Ok(issue),
                    Err(e) => Err(base(e)),
                }
            }
        }
    }

    /// Envoie la file d'envoi dans l'ordre : les écritures du compte de la
    /// session et celles des comptes refusés par le serveur (décision 52, mises
    /// en attente pour l'Admin), et avec `suspendues` celles qui attendent une
    /// décision. Une coupure arrête l'envoi ; ce qui reste part au prochain
    /// retour.
    fn vider_file(&mut self, suspendues: bool) -> Result<(), ErreurPoste> {
        let Some(uid) = self.uid_session() else {
            return Ok(());
        };
        let file = replique::file(&self.copie).map_err(base)?;
        let refuses = poste_relie::comptes_refuses(&self.copie).map_err(base)?;
        let mut resultat = Ok(());
        let mut suspendue = false;
        for entree in file {
            let compte = &entree.operation.compte;
            if (*compte != uid && !refuses.contains(compte)) || (entree.suspendue && !suspendues) {
                continue;
            }
            let Some(client) = self.client.as_mut() else {
                resultat = Err(ErreurPoste::HorsConnexion);
                break;
            };
            let issue = match client.envoyer(&entree.operation) {
                Ok(issue) => issue,
                // Refusée sans être gardée (écriture non reçue hors
                // connexion, requête rejetée) : elle ne passera jamais.
                Err(ErreurClient::Refus { modele, valeurs }) => Issue::Refusee { modele, valeurs },
                Err(ErreurClient::Rejet(motif)) => Issue::Refusee {
                    modele: motif,
                    valeurs: Vec::new(),
                },
                Err(e) => {
                    resultat = Err(self.erreur_requete(e));
                    break;
                }
            };
            let etape = match &issue {
                // Une écriture suspendue de ce poste sur la même ligne attend
                // encore : celle-ci reste dans la file.
                Issue::Bloquee | Issue::EnFile => continue,
                Issue::EnConflit | Issue::EnAttente if entree.suspendue => continue,
                Issue::EnConflit | Issue::EnAttente => {
                    self.remettre_lignes(&entree.lignes).and_then(|()| {
                        replique::suspendre_en_file(&self.copie, entree.rang).map_err(base)
                    })
                }
                Issue::Acceptee { .. } => {
                    replique::retirer_de_file(&self.copie, entree.rang).map_err(base)
                }
                Issue::Refusee { .. } | Issue::Depassee => {
                    self.remettre_lignes(&entree.lignes).and_then(|()| {
                        replique::retirer_de_file(&self.copie, entree.rang).map_err(base)
                    })
                }
            };
            // Lignes non remises (coupure) : l'écriture reste dans la file, et
            // son renvoi rendra le même résultat.
            if let Err(e) = etape {
                resultat = Err(e);
                break;
            }
            suspendue |= matches!(issue, Issue::EnConflit | Issue::EnAttente);
            (self.evenements)(Evenement::Confirmee {
                action: Box::new(entree.operation.action),
                issue,
            });
        }
        self.signaler_file();
        if suspendue {
            self.compter_suspendues(false);
        }
        resultat
    }

    /// Remet des lignes de la copie à l'état présent du serveur.
    fn remettre_lignes(&mut self, lignes: &[(Entite, String)]) -> Result<(), ErreurPoste> {
        for morceau in lignes.chunks(LIGNES_MAX) {
            let Some(client) = self.client.as_mut() else {
                return Err(ErreurPoste::HorsConnexion);
            };
            let rendues = match client.lignes(morceau.to_vec()) {
                Ok(rendues) => rendues,
                Err(e) => return Err(self.erreur_requete(e)),
            };
            let bilan = replique::remettre(&mut self.copie, &rendues).map_err(base)?;
            if !bilan.sortes.is_empty() {
                (self.evenements)(Evenement::Changements {
                    sortes: bilan.sortes,
                });
            }
        }
        Ok(())
    }

    /// Relit les suspendues visibles du compte connecté et signale leur
    /// nombre s'il a changé (ou toujours, avec `toujours`).
    fn compter_suspendues(&mut self, toujours: bool) {
        if let Ok(liste) = self.suspendues() {
            self.noter_suspendues(&liste, toujours);
        }
    }

    /// Relit l'avertissement des sauvegardes du serveur (décision 56 ; rien
    /// pour un compte qui n'est pas Admin) et le signale s'il a changé (ou
    /// toujours, avec `toujours`).
    fn lire_sauvegarde(&mut self, toujours: bool) {
        if self.etat_present().definitif() || self.uid_session().is_none() {
            return;
        }
        let Some(client) = self.client.as_mut() else {
            return;
        };
        let avertissement = match client.sauvegarde() {
            Ok(a) => a,
            Err(e) => {
                let _ = self.erreur_requete(e);
                return;
            }
        };
        if toujours || self.sauvegarde_vue != Some(avertissement) {
            self.sauvegarde_vue = Some(avertissement);
            (self.evenements)(Evenement::Sauvegarde(avertissement));
        }
    }

    fn noter_suspendues(&mut self, liste: &[Suspendue], toujours: bool) {
        let compter = |motif: Motif| liste.iter().filter(|s| s.motif == motif).count() as u32;
        let nombres = (compter(Motif::Conflit), compter(Motif::Attente));
        if toujours || self.suspendues_vues != Some(nombres) {
            self.suspendues_vues = Some(nombres);
            (self.evenements)(Evenement::Suspendues {
                conflits: nombres.0,
                attentes: nombres.1,
            });
        }
    }

    fn suspendues(&mut self) -> Result<Vec<Suspendue>, ErreurPoste> {
        if self.etat_present().definitif() {
            return Err(ErreurPoste::Injoignable);
        }
        if self.uid_session().is_none() {
            return Err(ErreurPoste::HorsConnexion);
        }
        let Some(client) = self.client.as_mut() else {
            return Err(ErreurPoste::HorsConnexion);
        };
        match client.suspendues() {
            Ok(liste) => Ok(liste),
            Err(e) => Err(self.erreur_requete(e)),
        }
    }

    fn renouveler(&mut self, cle: &str) -> Result<Abonnement, ErreurPoste> {
        if self.etat_present().definitif() {
            return Err(ErreurPoste::Injoignable);
        }
        let Some(client) = self.client.as_mut() else {
            return Err(ErreurPoste::HorsConnexion);
        };
        match client.renouveler(cle) {
            Ok(abonnement) => {
                self.noter_abonnement(abonnement.clone());
                Ok(abonnement)
            }
            Err(e) => Err(self.erreur_requete(e)),
        }
    }
}
