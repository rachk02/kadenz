//! Poste relié au serveur de l'entreprise (phase 2, jalon F5) : l'application
//! travaille sur la copie du serveur (décision 45), tenue à jour par le
//! synchroniseur de `kadenz-poste`, qui tourne dans son propre fil. Ses
//! événements reviennent ici dans le fil de l'interface : état de la connexion
//! (indicateur, écritures permises ou non), listes à recharger, abonnement.

use std::cell::RefCell;
use std::path::PathBuf;

use kadenz_poste::{ErreurPoste, EtatConnexion, Evenement, Synchroniseur};
use kadenz_protocole::{Entite, Issue};
use slint::{ComponentHandle, SharedString};

use crate::traduction::{tr, trf};
use crate::{Alerte, ConnexionApi, Etat, EtatPartage, FenetrePrincipale};

thread_local! {
    /// État de l'application, pour les événements du synchroniseur.
    static ETAT: RefCell<Option<EtatPartage>> = const { RefCell::new(None) };
}

/// Ce que l'écran sait du synchroniseur : gardé dans l'état de l'application.
pub(crate) struct Relie {
    pub(crate) synchro: Synchroniseur,
    pub(crate) connexion: EtatConnexion,
    /// Une écriture attend la confirmation du serveur.
    pub(crate) confirmation: bool,
    /// Rattrapage en cours : rang reçu, dernier rang du serveur.
    pub(crate) avancement: Option<(u64, u64)>,
    /// Le compte de la session est connecté sur le serveur : sans cela (connexion
    /// locale de la décision 48, ou compte refusé), rien ne s'écrit.
    pub(crate) compte_serveur: bool,
    /// Compte accepté par le serveur mais pas encore reçu dans une copie neuve :
    /// la session s'ouvre dès qu'il arrive.
    pub(crate) compte_attendu: Option<String>,
}

/// Démarre le synchroniseur sur la copie ; ses événements reviennent dans le
/// fil de l'interface. Un échec laisse le poste sans synchroniseur : tout reste
/// consultable, rien ne s'écrit (indicateur « Serveur injoignable »).
pub(crate) fn demarrer(f: &FenetrePrincipale, etat: &EtatPartage, copie: PathBuf) {
    ETAT.set(Some(etat.clone()));
    let fenetre = f.as_weak();
    let rappel = Box::new(move |evenement: Evenement| {
        let _ = fenetre.upgrade_in_event_loop(move |f| recevoir(&f, evenement));
    });
    let relie = match Synchroniseur::demarrer(copie, rappel) {
        Ok(synchro) => {
            let connexion = synchro.etat();
            Some(Relie {
                synchro,
                connexion,
                confirmation: false,
                avancement: None,
                compte_serveur: false,
                compte_attendu: None,
            })
        }
        Err(e) => {
            crate::consigner_erreur(&format!("Synchroniseur non démarré : {e}"));
            None
        }
    };
    etat.borrow_mut().relie = relie;
    afficher(f, &etat.borrow());
}

/// Modules d'un poste relié : ceux de l'abonnement gardé dans la copie, au lieu
/// de la licence V1 du poste.
pub(crate) fn appliquer_modules(f: &FenetrePrincipale, conn: &kadenz_stockage::Connexion) {
    if let Err(e) = kadenz_stockage::poste_relie::appliquer_modules(conn) {
        crate::consigner_erreur(&format!("Modules de l'abonnement non appliqués : {e}"));
    }
    crate::licence::afficher_modules(f, conn);
}

/// Texte traduit d'une erreur du synchroniseur.
pub(crate) fn texte_erreur(e: &ErreurPoste) -> String {
    match e {
        ErreurPoste::HorsConnexion => tr("serveur injoignable"),
        ErreurPoste::Injoignable => tr("connexion au serveur impossible"),
        ErreurPoste::LectureSeule { .. } => tr("abonnement échu : Kadenz est en lecture seule"),
        ErreurPoste::Delai => {
            tr("Le serveur n'a pas répondu à temps : l'opération sera confirmée à la reconnexion")
        }
        ErreurPoste::CompteRefuse { attente_secondes } => trf(
            "identifiant ou mot de passe incorrect (nouvel essai dans {} s)",
            &[attente_secondes],
        ),
        ErreurPoste::Refus { modele, valeurs } => trf(
            modele,
            &valeurs
                .iter()
                .map(|v| v as &dyn std::fmt::Display)
                .collect::<Vec<_>>(),
        ),
        ErreurPoste::Base(message) => trf("Erreur technique : {}", &[message]),
    }
}

/// Connexion d'un compte sur un poste relié : par le serveur, ou en local avec
/// les empreintes de la copie quand le serveur est injoignable (décision 48).
/// `None` si le poste n'est pas relié ; sinon le message de l'écran de
/// connexion, vide si la session s'est ouverte.
pub(crate) fn connecter(
    f: &FenetrePrincipale,
    etat: &EtatPartage,
    identifiant: &str,
    mot_de_passe: &str,
) -> Option<SharedString> {
    let resultat = etat
        .borrow()
        .relie
        .as_ref()?
        .synchro
        .connecter(identifiant, mot_de_passe);
    Some(match resultat {
        Ok(compte) => {
            if let Some(r) = etat.borrow_mut().relie.as_mut() {
                r.compte_serveur = true;
                r.compte_attendu = Some(compte.compte.clone());
            }
            if ouvrir_session_attendue(f, etat) {
                SharedString::new()
            } else {
                // Copie neuve : le compte arrive avec la première synchronisation.
                tr("Connexion acceptée par le serveur. Les données de l'entreprise arrivent : la session s'ouvrira dès que votre compte sera reçu.").into()
            }
        }
        Err(ErreurPoste::HorsConnexion | ErreurPoste::Injoignable) => {
            // Décision 48 : consultation sur la copie ; le synchroniseur garde
            // le compte et le connecte au retour du serveur.
            let local =
                kadenz_stockage::comptes::connecter(&etat.borrow().conn, identifiant, mot_de_passe);
            match local {
                Ok(utilisateur) => {
                    crate::ouvrir_session(f, &mut etat.borrow_mut(), utilisateur);
                    SharedString::new()
                }
                Err(e) => crate::phrase(e),
            }
        }
        Err(e) => crate::phrase(texte_erreur(&e)),
    })
}

/// Ouvre la session du compte accepté par le serveur s'il est dans la copie.
fn ouvrir_session_attendue(f: &FenetrePrincipale, etat: &EtatPartage) -> bool {
    let mut e = etat.borrow_mut();
    let Some(uid) = e.relie.as_ref().and_then(|r| r.compte_attendu.clone()) else {
        return false;
    };
    let compte = kadenz_stockage::synchro::id_de(&e.conn, Entite::Utilisateur, &uid)
        .ok()
        .flatten()
        .and_then(|id| kadenz_stockage::comptes::lire(&e.conn, id).ok().flatten());
    let Some(utilisateur) = compte else {
        return false;
    };
    if let Some(r) = e.relie.as_mut() {
        r.compte_attendu = None;
    }
    crate::ouvrir_session(f, &mut e, utilisateur);
    true
}

/// Déconnexion du compte : le serveur ferme sa session.
pub(crate) fn deconnecter(etat: &mut Etat) {
    if let Some(r) = etat.relie.as_mut() {
        r.synchro.deconnecter();
        r.compte_serveur = false;
        r.compte_attendu = None;
    }
}

/// Arrêt du synchroniseur à la fermeture de la fenêtre.
pub(crate) fn arreter(etat: &mut Etat) {
    if let Some(r) = etat.relie.take() {
        r.synchro.arreter();
    }
}

/// Texte et niveau de l'indicateur (0 relié, 1 en cours, 2 coupé ou refusé).
fn indicateur(
    connexion: &EtatConnexion,
    confirmation: bool,
    avancement: Option<(u64, u64)>,
) -> (String, i32) {
    match connexion {
        EtatConnexion::Connexion => (tr("Connexion au serveur..."), 1),
        EtatConnexion::Connecte | EtatConnexion::LectureSeule { .. } if confirmation => {
            (tr("Confirmation en attente"), 1)
        }
        EtatConnexion::Connecte => (tr("Connecté au serveur"), 0),
        EtatConnexion::LectureSeule { .. } => (tr("Connecté au serveur, lecture seule"), 0),
        EtatConnexion::Synchronisation => match avancement {
            Some((recus, dernier)) if dernier > 0 => {
                let pourcentage = (recus.min(dernier) * 100 / dernier).min(99);
                (trf("Synchronisation... {} %", &[&pourcentage]), 1)
            }
            _ => (tr("Synchronisation..."), 1),
        },
        EtatConnexion::Reconnexion { essai } => {
            (trf("Reconnexion au serveur (essai {})", &[essai]), 1)
        }
        EtatConnexion::Injoignable => (tr("Hors connexion : serveur injoignable"), 2),
        EtatConnexion::CertificatDifferent => (tr("Serveur non reconnu"), 2),
        EtatConnexion::PosteRetire => (tr("Poste retiré du serveur"), 2),
        EtatConnexion::VersionDifferente { .. } => (tr("Version différente du serveur"), 2),
    }
}

/// Résultat d'une écriture confirmée après coup.
fn texte_confirmation(issue: &Issue) -> String {
    match issue {
        Issue::Acceptee {
            numero: Some(numero),
            ..
        } => trf("Opération confirmée par le serveur : numéro {}.", &[numero]),
        Issue::Acceptee { .. } => tr("Opération confirmée par le serveur."),
        Issue::Refusee { modele, valeurs } => trf(
            "Opération refusée par le serveur : {}",
            &[&crate::traduction::trf(
                modele,
                &valeurs
                    .iter()
                    .map(|v| v as &dyn std::fmt::Display)
                    .collect::<Vec<_>>(),
            )],
        ),
        _ => tr("Opération mise en attente par le serveur : l'Admin décidera."),
    }
}

/// Le serveur répond (connecté, même en rattrapage ou en lecture seule).
fn connexion_ouverte(etat: &EtatPartage) -> bool {
    matches!(
        etat.borrow().relie.as_ref().map(|r| &r.connexion),
        Some(
            EtatConnexion::Connecte
                | EtatConnexion::Synchronisation
                | EtatConnexion::LectureSeule { .. }
        )
    )
}

/// Raison du blocage des écritures due à la connexion, ou `None` si le
/// serveur peut les recevoir (décision 46). Sans synchroniseur, rien ne part.
fn raison_connexion(connexion: Option<&EtatConnexion>) -> Option<String> {
    match connexion {
        Some(
            EtatConnexion::Connecte
            | EtatConnexion::Synchronisation
            | EtatConnexion::LectureSeule { .. },
        ) => None,
        Some(e) if e.definitif() => Some(tr(
            "Connexion au serveur impossible : aucune saisie n'est possible. Voir Paramètres, rubrique « Serveur ».",
        )),
        _ => Some(tr(
            "Serveur injoignable : la saisie reprendra au retour du serveur. Consultation, impression et exports restent possibles.",
        )),
    }
}

/// Raison du blocage des écritures sur ce poste, ou `None` si la saisie est
/// permise : seul point de décision pour les écrans. Poste V1 : seule la
/// lecture seule de l'abonnement compte, et elle n'arrive jamais.
pub(crate) fn blocage(etat: &Etat) -> Option<String> {
    if crate::serveur::inscrit(&etat.conn) {
        // Connexion locale (décision 48) : rien ne part tant que le compte n'est
        // pas connecté sur le serveur.
        let connexion = etat
            .relie
            .as_ref()
            .filter(|r| r.compte_serveur || etat.utilisateur.is_none())
            .map(|r| &r.connexion);
        if let Some(raison) = raison_connexion(connexion) {
            return Some(raison);
        }
    }
    crate::abonnement::blocage(&etat.conn)
}

/// Recopie l'état de la connexion dans l'indicateur.
pub(crate) fn afficher(f: &FenetrePrincipale, etat: &Etat) {
    let api = f.global::<ConnexionApi>();
    let inscrit = crate::serveur::inscrit(&etat.conn);
    api.set_visible(inscrit);
    if !inscrit {
        return;
    }
    let (texte, niveau) = match &etat.relie {
        Some(r) => indicateur(&r.connexion, r.confirmation, r.avancement),
        None => indicateur(&EtatConnexion::Injoignable, false, None),
    };
    api.set_texte(texte.into());
    api.set_niveau(niveau);
}

/// Événement du synchroniseur, dans le fil de l'interface.
fn recevoir(f: &FenetrePrincipale, evenement: Evenement) {
    let Some(etat) = ETAT.with_borrow(|e| e.clone()) else {
        return;
    };
    match evenement {
        Evenement::Etat(connexion) => {
            {
                let mut e = etat.borrow_mut();
                if let Some(r) = e.relie.as_mut() {
                    if connexion != EtatConnexion::Synchronisation {
                        r.avancement = None;
                    }
                    r.connexion = connexion;
                }
            }
            if connexion_ouverte(&etat) {
                ouvrir_session_attendue(f, &etat);
            }
            // Écritures permises ou non : droits, menu et fiche ouverte suivent.
            let e = etat.borrow();
            afficher(f, &e);
            crate::appliquer_droits(f, &e);
        }
        Evenement::Avancement { recus, dernier } => {
            let mut e = etat.borrow_mut();
            if let Some(r) = e.relie.as_mut() {
                r.avancement = Some((recus, dernier));
            }
            afficher(f, &e);
        }
        Evenement::ConfirmationEnAttente(attente) => {
            let mut e = etat.borrow_mut();
            if let Some(r) = e.relie.as_mut() {
                r.confirmation = attente;
            }
            afficher(f, &e);
        }
        Evenement::Changements { sortes } => {
            let api = f.global::<ConnexionApi>();
            let mut droits = false;
            for sorte in sortes {
                match sorte {
                    Entite::Tiers => api.set_changements_tiers(api.get_changements_tiers() + 1),
                    Entite::Produit => {
                        api.set_changements_produits(api.get_changements_produits() + 1)
                    }
                    Entite::Document | Entite::Reglement | Entite::Instantane => {
                        api.set_changements_documents(api.get_changements_documents() + 1)
                    }
                    Entite::Utilisateur | Entite::Entreprise | Entite::Parametres => droits = true,
                }
            }
            // Comptes, matrice des droits, entreprise ou réglages : la session
            // relit le compte, les droits et le nom de l'entreprise ; ou s'ouvre,
            // si son compte vient d'arriver dans une copie neuve.
            if droits && !ouvrir_session_attendue(f, &etat) {
                crate::relire_session(f, &etat);
            }
        }
        Evenement::Abonnement(_) => {
            // L'abonnement est gardé dans la copie par le synchroniseur :
            // modules, bandeaux, rubrique et lecture seule le relisent.
            let e = etat.borrow();
            appliquer_modules(f, &e.conn);
            crate::appliquer_droits(f, &e);
        }
        Evenement::Confirmee { issue, .. } => {
            // Écriture rendue sans réponse à temps, confirmée plus tard : son
            // résultat d'origine (décision de kadenz-34 et kadenz-8e, F5c).
            f.global::<Alerte>()
                .invoke_montrer(texte_confirmation(&issue).into());
        }
        Evenement::Compte(resultat) => {
            let refuse = matches!(resultat, Err(ErreurPoste::CompteRefuse { .. }));
            if let Some(r) = etat.borrow_mut().relie.as_mut() {
                r.compte_serveur = resultat.is_ok();
            }
            let session = etat.borrow().utilisateur.is_some();
            if refuse && session {
                // Mot de passe changé ailleurs ou compte désactivé : retour à
                // l'écran de connexion.
                crate::fermer_session(f, &mut etat.borrow_mut());
                f.global::<Alerte>().invoke_montrer(
                    tr("Votre mot de passe a changé ou votre compte a été désactivé sur le serveur : reconnectez-vous.").into(),
                );
            } else {
                let e = etat.borrow();
                crate::appliquer_droits(f, &e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indicateur_selon_l_etat() {
        assert_eq!(indicateur(&EtatConnexion::Connecte, false, None).1, 0);
        assert_eq!(
            indicateur(&EtatConnexion::Connecte, true, None).0,
            "Confirmation en attente"
        );
        assert_eq!(
            indicateur(&EtatConnexion::Synchronisation, false, Some((50, 200))).0,
            "Synchronisation... 25 %"
        );
        assert_eq!(indicateur(&EtatConnexion::Injoignable, false, None).1, 2);
        for final_ in [
            EtatConnexion::CertificatDifferent,
            EtatConnexion::PosteRetire,
            EtatConnexion::VersionDifferente { serveur: 3 },
        ] {
            assert_eq!(indicateur(&final_, false, None).1, 2);
        }
    }

    #[test]
    fn ecritures_selon_la_connexion() {
        assert_eq!(raison_connexion(Some(&EtatConnexion::Connecte)), None);
        assert_eq!(
            raison_connexion(Some(&EtatConnexion::Synchronisation)),
            None
        );
        // La lecture seule est jugée par l'abonnement, pas par la connexion.
        assert_eq!(
            raison_connexion(Some(&EtatConnexion::LectureSeule { echeance: None })),
            None
        );
        for coupe in [
            EtatConnexion::Connexion,
            EtatConnexion::Reconnexion { essai: 2 },
            EtatConnexion::Injoignable,
        ] {
            assert!(
                raison_connexion(Some(&coupe))
                    .unwrap()
                    .contains("injoignable")
            );
        }
        assert!(
            raison_connexion(Some(&EtatConnexion::PosteRetire))
                .unwrap()
                .contains("impossible")
        );
        // Sans synchroniseur : rien ne part.
        assert!(raison_connexion(None).is_some());
    }
}
