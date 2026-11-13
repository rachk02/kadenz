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
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub use kadenz_client::CompteConnecte;
use kadenz_client::{Client, ErreurClient, Identite};
use kadenz_protocole::messages::LIGNES_MAX;
use kadenz_protocole::{Abonnement, Action, EnCoupure, Entite, Issue, Motif, Operation, Suspendue};
use kadenz_stockage::poste_relie::{self, PosteRelie};
use kadenz_stockage::{replique, synchro};
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
    Arreter,
}

/// Rappel des événements : appelé depuis le fil du synchroniseur.
pub type Rappel = Box<dyn Fn(Evenement) + Send>;

pub struct Synchroniseur {
    commandes: Sender<Commande>,
    etat: Arc<Mutex<EtatConnexion>>,
    a_envoyer: Arc<AtomicU32>,
    fil: Option<JoinHandle<()>>,
}

impl Synchroniseur {
    /// Lit le lien au serveur dans la copie, ouvre une connexion à la copie
    /// pour le fil et part en connexion.
    pub fn demarrer(copie: PathBuf, evenements: Rappel) -> Result<Synchroniseur, ErreurPoste> {
        let conn = poste_relie::ouvrir_copie(&copie).map_err(base)?;
        let poste = poste_relie::lire(&conn)
            .map_err(base)?
            .ok_or_else(|| ErreurPoste::Base("la copie n'est reliée à aucun serveur".into()))?;
        let entreprise = synchro::identifiant_entreprise(&conn).map_err(base)?;
        let a_envoyer = Arc::new(AtomicU32::new(replique::a_envoyer(&conn).map_err(base)?));
        let etat = Arc::new(Mutex::new(EtatConnexion::Connexion));
        let (commandes, reception) = mpsc::channel();
        let identite = Identite {
            certificat: poste.certificat.clone(),
            cle: poste.cle.clone(),
        };
        let mut fil = Fil {
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

/// Compte connecté : de quoi le reconnecter après une coupure (en mémoire
/// seulement).
struct Compte {
    identifiant: String,
    mot_de_passe: String,
    /// `None` tant que le serveur n'a pas encore connecté le compte.
    uid: Option<String>,
}

struct Fil {
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
}

impl Fil {
    fn tourner(&mut self, commandes: mpsc::Receiver<Commande>) {
        loop {
            let attente = if self.client.is_some() || self.etat_present().definitif() {
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
                self.client = None;
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
            Commande::Arreter => {}
        }
    }

    fn periodique(&mut self) {
        if self.etat_present().definitif() {
            return;
        }
        if self.client.is_none() {
            if Instant::now() >= self.prochain_essai {
                self.essayer_connexion();
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
        }
        if self.rattraper().is_ok() {
            self.poser_etat(self.etat_ouvert());
        }
    }

    /// Ouvre la session TLS ; reconnecte le compte s'il y en avait un.
    fn essayer_connexion(&mut self) {
        match Client::connecter(
            &self.poste.adresse,
            &self.poste.empreinte_serveur,
            &self.identite,
            DELAI,
        ) {
            Ok((client, _)) => {
                self.client = Some(client);
                self.echecs = 0;
                self.poser_etat(self.etat_ouvert());
                if let Some(mut compte) = self.compte.take() {
                    let reprise = self
                        .client
                        .as_mut()
                        .map(|c| c.connexion(&compte.identifiant, &compte.mot_de_passe));
                    match reprise {
                        Some(Ok(connecte)) => {
                            let _ =
                                poste_relie::oublier_compte_refuse(&self.copie, &connecte.compte);
                            compte.uid = Some(connecte.compte.clone());
                            self.compte = Some(compte);
                            (self.evenements)(Evenement::Compte(Ok(connecte)));
                            self.apres_connexion();
                        }
                        // Mot de passe changé ailleurs, compte désactivé : le
                        // compte n'est plus connecté.
                        Some(Err(ErreurClient::CompteRefuse { attente_secondes })) => {
                            self.noter_refus(&compte.identifiant);
                            (self.evenements)(Evenement::Compte(Err(ErreurPoste::CompteRefuse {
                                attente_secondes,
                            })));
                        }
                        Some(Err(e)) => {
                            self.compte = Some(compte);
                            let _ = self.perdre(e);
                        }
                        None => {}
                    }
                }
            }
            Err(e) => {
                let _ = self.perdre(e);
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
    }

    /// Connexion perdue ou refusée : état et prochain essai. Rend l'erreur
    /// pour l'écran.
    fn perdre(&mut self, e: ErreurClient) -> ErreurPoste {
        self.client = None;
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
        }
        if self.client.is_none() {
            self.essayer_connexion();
        }
        let Some(client) = self.client.as_mut() else {
            let erreur = if self.etat_present().definitif() {
                ErreurPoste::Injoignable
            } else {
                // Décision 48 : le compte se connectera au retour du serveur.
                self.compte = Some(Compte {
                    identifiant,
                    mot_de_passe,
                    uid: None,
                });
                ErreurPoste::HorsConnexion
            };
            let _ = retour.send(Err(erreur));
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
                let _ = retour.send(Ok(connecte));
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
                let _ = retour.send(Err(erreur));
            }
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
        let issue = self.envoyer(op)?;
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
