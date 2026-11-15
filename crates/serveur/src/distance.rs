//! Accès à distance (phase 2, jalon F8 ; spécification de la synchronisation,
//! section 7) : connexion de contrôle vers le relais quand l'Admin l'a activé
//! et qu'un relais est réglé, reconnexion, sessions ouvertes à la demande du
//! relais. Le serveur ne croit pas le relais : chaque session distante est
//! contrôlée ici (poste autorisé, accès actif) comme une session locale.

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use kadenz_client::Identite;
use kadenz_protocole::distance::{EtatAccesDistant, MessageRelais, PosteDistant};
use kadenz_relais::controle::{Controle, Enregistrement, ouvrir_session};
use kadenz_relais::{ErreurRelais, motif};
use kadenz_stockage::distance::{self, Relais};
use kadenz_stockage::{Resultat, abonnement};

use crate::{Base, Contexte, session};

/// Délai de l'enregistrement et des connexions de session.
const DELAI_RELAIS: Duration = Duration::from_secs(10);
/// Reconnexion après un échec : 5 secondes doublées jusqu'à 5 minutes.
const ATTENTE_MIN: Duration = Duration::from_secs(5);
const ATTENTE_MAX: Duration = Duration::from_secs(300);
/// Relecture du réglage pendant que la connexion de contrôle est ouverte, et
/// pendant l'attente d'une reconnexion : un réglage écrit par la fenêtre du
/// serveur est pris sans attendre la fin du recul.
const RELECTURE: Duration = Duration::from_secs(2);

#[derive(Default)]
struct Memoire {
    actif: bool,
    relais: Option<Relais>,
    autorises: HashSet<String>,
    joignable: bool,
    erreur: Option<MessageRelais>,
}

/// État de l'accès à distance gardé en mémoire par le service, relu de la
/// base à chaque changement.
#[derive(Default)]
pub(crate) struct Distance {
    memoire: Mutex<Memoire>,
    /// Compteur de changements faits par les requêtes de l'Admin : la veille
    /// du relais les prend aussitôt.
    changements: AtomicU64,
}

/// Réglage lu de la base : actif, relais, licence, postes autorisés.
type Reglage = (bool, Option<Relais>, Option<String>, Vec<String>);

fn lire_reglage(conn: &rusqlite::Connection) -> Resultat<Reglage> {
    Ok((
        distance::actif(conn)?,
        distance::relais(conn)?,
        abonnement::licence(conn)?,
        distance::autorises(conn)?,
    ))
}

impl Distance {
    fn memoire(&self) -> std::sync::MutexGuard<'_, Memoire> {
        self.memoire.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Relit le réglage de la base ; rend ce qui a été lu.
    pub(crate) fn rafraichir(&self, base: &Base) -> Option<Reglage> {
        let reglage = base.executer(|c| lire_reglage(c)).ok()?.ok()?;
        let mut m = self.memoire();
        m.actif = reglage.0;
        m.relais = reglage.1.clone();
        m.autorises = reglage.3.iter().cloned().collect();
        Some(reglage)
    }

    /// Réglage changé par une requête : relu tout de suite.
    pub(crate) fn changer(&self, base: &Base) {
        self.rafraichir(base);
        self.changements.fetch_add(1, Ordering::SeqCst);
    }

    /// Poste autorisé à passer par le relais en ce moment.
    pub(crate) fn autorise(&self, poste: &str) -> bool {
        let m = self.memoire();
        m.actif && m.autorises.contains(poste)
    }

    /// Adresse du relais à donner à ce poste dans `Bienvenue`.
    pub(crate) fn relais_pour(&self, poste: &str) -> Option<String> {
        let m = self.memoire();
        if m.actif && m.autorises.contains(poste) {
            m.relais.as_ref().map(|r| r.adresse.clone())
        } else {
            None
        }
    }

    /// Début d'une session distante, gardé dans la base pour survivre aux
    /// arrêts du service.
    pub(crate) fn noter_session(&self, base: &Base) {
        let debut = chrono::Local::now().format("%Y-%m-%dT%H:%M").to_string();
        let _ = base.executer(move |c| distance::noter_derniere_distante(c, &debut));
    }

    fn noter(&self, joignable: bool, erreur: Option<MessageRelais>) {
        let mut m = self.memoire();
        m.joignable = joignable;
        m.erreur = erreur;
    }

    /// État complet pour la rubrique « Accès à distance », avec ce qui a été
    /// lu de la base par [`lire_etat`].
    pub(crate) fn etat(&self, lu: EtatLu) -> EtatAccesDistant {
        let (actif, relais, derniere_distante, postes) = lu;
        let m = self.memoire();
        EtatAccesDistant {
            actif,
            relais,
            joignable: m.joignable,
            erreur: m.erreur.clone(),
            derniere_distante,
            postes,
        }
    }
}

/// Ce que la rubrique lit de la base : actif, adresse du relais, dernière
/// session distante, postes.
pub(crate) type EtatLu = (bool, Option<String>, Option<String>, Vec<PosteDistant>);

pub(crate) fn lire_etat(conn: &rusqlite::Connection) -> Resultat<EtatLu> {
    let postes = distance::postes(conn)?
        .into_iter()
        .map(|(empreinte, nom, autorise)| PosteDistant {
            empreinte,
            nom,
            autorise,
        })
        .collect();
    Ok((
        distance::actif(conn)?,
        distance::relais(conn)?.map(|r| r.adresse),
        distance::derniere_distante(conn)?,
        postes,
    ))
}

/// Message à traduire pour un échec avec le relais.
fn message(e: &ErreurRelais) -> MessageRelais {
    match e {
        ErreurRelais::Injoignable(texte) => {
            MessageRelais::new("relais injoignable : {}", &[texte.as_str()])
        }
        ErreurRelais::CertificatDifferent => {
            MessageRelais::new("le relais ne présente pas le certificat attendu", &[])
        }
        ErreurRelais::Refus { motif: m, valeurs } => match m.as_str() {
            motif::LICENCE_ECHUE => MessageRelais::new(
                "abonnement échu le {} : l'accès à distance demande un abonnement actif",
                &[valeurs.first().map_or("", String::as_str)],
            ),
            motif::LICENCE_REFUSEE | motif::PREUVE_INVALIDE => {
                MessageRelais::new("licence d'abonnement refusée par le relais", &[])
            }
            motif::DEJA_ENREGISTREE => MessageRelais::new(
                "un autre serveur de cette entreprise est déjà enregistré sur le relais",
                &[],
            ),
            motif::VERSION => MessageRelais::new(
                "le relais utilise une autre version ({})",
                &[valeurs.first().map_or("", String::as_str)],
            ),
            autre => MessageRelais::new("réponse du relais inattendue : {}", &[autre]),
        },
        ErreurRelais::Protocole(texte) => {
            MessageRelais::new("réponse du relais inattendue : {}", &[texte.as_str()])
        }
    }
}

/// Attend `duree` au plus ; rend la main plus tôt à l'arrêt du serveur ou à
/// un changement du réglage.
fn dormir(contexte: &Contexte, duree: Duration, vu: u64) {
    let fin = Instant::now() + duree;
    while Instant::now() < fin {
        if contexte.arret.load(Ordering::SeqCst)
            || contexte.distance.changements.load(Ordering::SeqCst) != vu
        {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
}

/// Attente avant un nouvel essai d'enregistrement : finie plus tôt par une
/// requête de l'Admin, ou par un réglage changé dans la base par la fenêtre du
/// serveur (accès désactivé, autre relais, autre licence), relu toutes les
/// 2 secondes.
fn attendre_reprise(contexte: &Contexte, duree: Duration, vu: u64, relais: &Relais, licence: &str) {
    let fin = Instant::now() + duree;
    loop {
        let reste = fin.saturating_duration_since(Instant::now());
        if reste.is_zero() {
            return;
        }
        dormir(contexte, RELECTURE.min(reste), vu);
        if contexte.arret.load(Ordering::SeqCst)
            || contexte.distance.changements.load(Ordering::SeqCst) != vu
        {
            return;
        }
        if let Some((actif, nouveau, nouvelle_licence, _)) =
            contexte.distance.rafraichir(&contexte.base)
            && (!actif
                || nouveau.as_ref() != Some(relais)
                || nouvelle_licence.as_deref() != Some(licence))
        {
            return;
        }
    }
}

/// Fil de la connexion de contrôle, jusqu'à l'arrêt du serveur.
pub(crate) fn veiller(contexte: Arc<Contexte>, identite: Identite) {
    let mut attente = ATTENTE_MIN;
    while !contexte.arret.load(Ordering::SeqCst) {
        let vu = contexte.distance.changements.load(Ordering::SeqCst);
        let Some((actif, relais, licence, postes)) = contexte.distance.rafraichir(&contexte.base)
        else {
            dormir(&contexte, RELECTURE, vu);
            continue;
        };
        let Some(relais) = relais.filter(|_| actif) else {
            contexte.distance.noter(false, None);
            attente = ATTENTE_MIN;
            dormir(&contexte, RELECTURE, vu);
            continue;
        };
        let Some(licence) = licence else {
            contexte.distance.noter(
                false,
                Some(MessageRelais::new(
                    "aucune licence d'abonnement : l'accès à distance demande un abonnement actif",
                    &[],
                )),
            );
            dormir(&contexte, RELECTURE, vu);
            continue;
        };
        let enregistrement = Enregistrement {
            relais: &relais.adresse,
            empreinte_relais: &relais.empreinte,
            entreprise: &contexte.entreprise,
            licence: &licence,
            certificat: &identite.certificat,
            cle: &identite.cle,
            postes: postes.clone(),
        };
        match Controle::enregistrer(&enregistrement, DELAI_RELAIS) {
            Err(e) => {
                contexte.distance.noter(false, Some(message(&e)));
                attendre_reprise(&contexte, attente, vu, &relais, &licence);
                attente = (attente * 2).min(ATTENTE_MAX);
            }
            Ok(controle) => {
                attente = ATTENTE_MIN;
                contexte.distance.noter(true, None);
                servir(&contexte, controle, &relais, &licence, postes);
            }
        }
    }
    contexte.distance.noter(false, None);
}

/// Connexion de contrôle ouverte : demandes de session, liste des postes,
/// fin si le réglage change ou si le relais est perdu.
fn servir(
    contexte: &Arc<Contexte>,
    mut controle: Controle,
    relais: &Relais,
    licence: &str,
    mut postes: Vec<String>,
) {
    let mut vu = contexte.distance.changements.load(Ordering::SeqCst);
    let mut relu = Instant::now();
    loop {
        if contexte.arret.load(Ordering::SeqCst) {
            break;
        }
        match controle.attendre(Duration::from_millis(200)) {
            Ok(Some(session_demandee)) => {
                let contexte = contexte.clone();
                let adresse = relais.adresse.clone();
                thread::spawn(move || {
                    if let Ok(tcp) = ouvrir_session(&adresse, &session_demandee, DELAI_RELAIS) {
                        session(&contexte, tcp, true);
                    }
                });
            }
            Ok(None) => {}
            Err(e) => {
                contexte.distance.noter(false, Some(message(&e)));
                return;
            }
        }
        let change = contexte.distance.changements.load(Ordering::SeqCst);
        if change == vu && relu.elapsed() < RELECTURE {
            continue;
        }
        vu = change;
        relu = Instant::now();
        let Some((actif, nouveau, nouvelle_licence, nouveaux)) =
            contexte.distance.rafraichir(&contexte.base)
        else {
            continue;
        };
        // Accès désactivé, autre relais, autre licence : on se réenregistre
        // (ou on s'arrête) avec le nouveau réglage.
        if !actif
            || nouveau.as_ref() != Some(relais)
            || nouvelle_licence.as_deref() != Some(licence)
        {
            break;
        }
        if nouveaux != postes {
            if let Err(e) = controle.envoyer_postes(nouveaux.clone()) {
                contexte.distance.noter(false, Some(message(&e)));
                return;
            }
            postes = nouveaux;
        }
    }
    controle.fermer();
    contexte.distance.noter(false, None);
}
