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
//! d'origine, sans l'appliquer deux fois. Hors connexion, une écriture est
//! refusée jusqu'au jalon F6 (décision 46).

use std::collections::BTreeSet;
use std::fmt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub use kadenz_client::CompteConnecte;
use kadenz_client::{Client, ErreurClient, Identite};
use kadenz_protocole::{Abonnement, Action, Entite, Issue, Operation};
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
    /// Résultat d'une écriture rendue sans réponse à temps (`ErreurPoste::Delai`),
    /// confirmée plus tard.
    Confirmee {
        action: Box<Action>,
        issue: Issue,
    },
    /// Une écriture attend (vrai) ou n'attend plus (faux) la confirmation du
    /// serveur : « Confirmation en attente ».
    ConfirmationEnAttente(bool),
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
    Arreter,
}

/// Rappel des événements : appelé depuis le fil du synchroniseur.
pub type Rappel = Box<dyn Fn(Evenement) + Send>;

pub struct Synchroniseur {
    commandes: Sender<Commande>,
    etat: Arc<Mutex<EtatConnexion>>,
    en_attente: Arc<AtomicBool>,
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
        let deja = replique::operation_en_cours(&conn).map_err(base)?.is_some();
        let en_attente = Arc::new(AtomicBool::new(deja));
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
            en_attente: en_attente.clone(),
            client: None,
            compte: None,
            echecs: 0,
            prochain_essai: Instant::now(),
            releves: 0,
            lecture_seule: None,
        };
        let fil = std::thread::Builder::new()
            .name("kadenz-synchroniseur".into())
            .spawn(move || fil.tourner(reception))
            .map_err(base)?;
        Ok(Synchroniseur {
            commandes,
            etat,
            en_attente,
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
    /// la copie à jour (la ligne créée y est), en `DELAI` au plus.
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

    /// Une écriture envoyée sans réponse attend sa confirmation.
    pub fn confirmation_en_attente(&self) -> bool {
        self.en_attente.load(Ordering::Relaxed)
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
    en_attente: Arc<AtomicBool>,
    client: Option<Client>,
    compte: Option<Compte>,
    echecs: u32,
    prochain_essai: Instant,
    releves: u32,
    /// Lecture seule selon le dernier abonnement lu : son échéance.
    lecture_seule: Option<Option<String>>,
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
        let attente = op.is_some();
        if self.en_attente.swap(attente, Ordering::Relaxed) != attente {
            (self.evenements)(Evenement::ConfirmationEnAttente(attente));
        }
        Ok(())
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
                            compte.uid = Some(connecte.compte.clone());
                            self.compte = Some(compte);
                            (self.evenements)(Evenement::Compte(Ok(connecte)));
                            self.apres_connexion();
                        }
                        // Mot de passe changé ailleurs, compte désactivé : le
                        // compte n'est plus connecté.
                        Some(Err(ErreurClient::CompteRefuse { attente_secondes })) => {
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
    /// réponse, rattrapage de la copie.
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
        if self.rattraper().is_ok() {
            self.poser_etat(self.etat_ouvert());
        }
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
                let erreur = self.erreur_requete(e);
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
        let uid = match self.compte.as_ref().map(|c| c.uid.clone()) {
            None => return Err(ErreurPoste::Base("aucun compte connecté".into())),
            // Compte pas encore connecté par le serveur (décision 48).
            Some(None) => return Err(ErreurPoste::HorsConnexion),
            Some(Some(uid)) => uid,
        };
        if self.client.is_none() {
            return Err(ErreurPoste::HorsConnexion);
        }
        // Écriture précédente restée sans réponse : confirmée d'abord.
        if let Some(op) = replique::operation_en_cours(&self.copie).map_err(base)? {
            let precedente = op.action.clone();
            match self.envoyer(op) {
                Ok(issue) => (self.evenements)(Evenement::Confirmee {
                    action: Box::new(precedente),
                    issue,
                }),
                Err(ErreurPoste::HorsConnexion | ErreurPoste::Delai) => {
                    return Err(ErreurPoste::Delai);
                }
                Err(_) => {}
            }
        }
        let op = Operation {
            id: synchro::nouvel_uid(),
            entreprise: self.entreprise.clone(),
            poste: self.identite.empreinte(),
            compte: uid,
            hors_connexion: false,
            base: replique::dernier_recu(&self.copie).map_err(base)?,
            action,
        };
        self.envoyer(op)
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
