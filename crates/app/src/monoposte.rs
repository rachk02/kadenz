//! Édition monoposte (docs/spec-monoposte.md, lot M2) : le serveur de
//! l'entreprise tourne dans le processus de l'application, sur 127.0.0.1, et
//! ce poste en est l'unique poste relié. L'application garde un seul chemin :
//! poste relié, copie du serveur, synchroniseur.
//!
//! Démarrage, selon l'état trouvé :
//! - serveur sans entreprise : présentation, assistant (installation par
//!   `admin::installer` sur la base du serveur), puis inscription automatique ;
//! - entreprise installée, sans copie : inscription interrompue, reprise à
//!   l'inscription automatique ;
//! - entreprise et copie : adresse du poste mise au port effectif, puis
//!   connexion.
//!
//! Le serveur démarre hors du fil de l'interface, derrière l'écran d'attente ;
//! l'installation et l'inscription aussi.
//!
//! Démarrage impossible (lot M4) : l'écran propose de restaurer une sauvegarde
//! du serveur (serveur arrêté, restauration sur le dossier fermé, redémarrage,
//! puis la reprise normale), et, quand une copie du poste existe sans serveur
//! qui lui corresponde, « Repartir à neuf » : la copie est mise de côté,
//! renommée avec la date, jamais effacée.
//!
//! Fermeture (lot M3) : voile, vidage borné de la file d'envoi, puis arrêt du
//! serveur intégré, qui fait la sauvegarde d'arrêt ; la fenêtre se ferme
//! ensuite. La copie du poste est dans `%LOCALAPPDATA%\Kadenz`, avec le
//! serveur (`dossier_copie`).

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::thread::JoinHandle;
use std::time::Duration;

use chrono::{NaiveTime, Utc};
use kadenz_client::{Client, ErreurClient, Identite, Inscrit};
use kadenz_domaine::lettres::Orthographe;
use kadenz_domaine::role::Role;
use kadenz_domaine::saisie;
use kadenz_serveur::admin::EtatService;
use kadenz_serveur::{Config, ErreurDemarrage, Serveur, admin, sauvegardes};
use kadenz_stockage::comptes::{self, EtatInstallation, NouvelUtilisateur, Utilisateur};
use kadenz_stockage::entreprise::Entreprise;
use kadenz_stockage::inscription::CodeCree;
use kadenz_stockage::installation::InstallationInitiale;
use kadenz_stockage::parametres::Parametres;
use kadenz_stockage::poste_relie::{self, PosteRelie};
use kadenz_stockage::{Connexion, inscription, postes, synchro};
use slint::{ComponentHandle, SharedString};

use crate::traduction::{tr, trf};
use crate::{
    ActivationUi, DonneesAssistant, Ecran, FenetrePrincipale, LicenceApi, MonoposteApi, Page,
    ReglagesApi, Rubrique,
};

/// Délai de l'inscription du poste auprès du serveur intégré.
const DELAI: Duration = Duration::from_secs(10);

/// Serveur intégré, gardé par `main` jusqu'à la fermeture de la fenêtre.
pub(crate) type Integre = Rc<RefCell<Option<Serveur>>>;

/// Démarrage normal de l'application sur une base ouverte (fermeture
/// `demarrer` de `main`) : ici, sur la copie du poste relié.
pub(crate) type Demarrer =
    Rc<dyn Fn(&FenetrePrincipale, Connexion) -> Result<(), kadenz_stockage::Erreur>>;

/// Session ouverte de `main` : la fermeture en retire le synchroniseur.
pub(crate) type Session = Rc<RefCell<Option<crate::EtatPartage>>>;

/// Vidage de la file d'envoi à la fermeture (spécification, section 4).
const DELAI_VIDAGE: Duration = Duration::from_secs(25);
/// Fermeture longue : texte changé, puis bouton « Fermer sans attendre ».
const FERMETURE_LENTE: Duration = Duration::from_secs(10);
const FERMETURE_LONGUE: Duration = Duration::from_secs(30);

struct Contexte {
    integre: Integre,
    demarrer: Demarrer,
    session: Session,
    /// Dossier du serveur intégré.
    dossier: PathBuf,
    /// Travail en cours hors du fil de l'interface (`en_fond`) : il tient le
    /// serveur et le rend en finissant.
    fil: RefCell<Option<JoinHandle<Option<Serveur>>>>,
}

thread_local! {
    static CONTEXTE: RefCell<Option<Rc<Contexte>>> = const { RefCell::new(None) };
    /// Fermeture en cours : une seconde demande est ignorée.
    static FERMETURE: Cell<bool> = const { Cell::new(false) };
    /// Reprise de la base V1 à proposer à la prochaine connexion d'un Admin
    /// (spécification, section 3.0) : posé après l'inscription automatique.
    static REPRISE_A_PROPOSER: Cell<bool> = const { Cell::new(false) };
}

fn contexte() -> Option<Rc<Contexte>> {
    CONTEXTE.with_borrow(|c| c.clone())
}

/// Vrai dans l'édition monoposte : marque posée à côté de l'exécutable par
/// l'installateur monoposte (`kadenz_maj::edition`), ou, en version de
/// développement seulement, `KADENZ_MONOPOSTE=1`. Sans marque, c'est l'édition
/// multiposte, et rien ne change pour un poste actuel.
pub(crate) fn edition_monoposte() -> bool {
    kadenz_maj::edition::monoposte()
        || cfg!(debug_assertions) && std::env::var_os("KADENZ_MONOPOSTE").is_some_and(|v| v == "1")
}

/// Dossier du serveur intégré : `%LOCALAPPDATA%\Kadenz\Serveur`, ou
/// `<dossier de KADENZ_BASE>\Serveur` quand `KADENZ_BASE` est posé (essais :
/// jamais les données réelles du compte).
fn dossier_serveur() -> Option<PathBuf> {
    if std::env::var_os("KADENZ_BASE").is_some() {
        return Some(crate::dossier_donnees().join("Serveur"));
    }
    kadenz_serveur::dossier_monoposte()
}

/// Dossier de la copie du poste relié. Monoposte : à côté du serveur intégré,
/// dans `%LOCALAPPDATA%\Kadenz`, non itinérant : sur un profil itinérant, la
/// copie ne suit pas le compte sur une machine qui n'a pas ce serveur. La
/// base V1, elle, reste lue dans `%APPDATA%\Kadenz`. Multiposte : le dossier
/// des données.
pub(crate) fn dossier_copie() -> PathBuf {
    if edition_monoposte()
        && let Some(dossier) = dossier_serveur().as_deref().and_then(Path::parent)
    {
        return dossier.to_path_buf();
    }
    crate::dossier_donnees()
}

/// Nom de ce poste sur le serveur : le nom de la machine.
fn nom_machine() -> String {
    std::env::var("COMPUTERNAME")
        .ok()
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| "poste".into())
}

/// Message de l'écran « Démarrage impossible » selon la cause ; le détail
/// technique va dans erreurs.log.
fn message_erreur(e: &ErreurDemarrage) -> String {
    match e {
        ErreurDemarrage::Verrouille => tr(
            "Kadenz est déjà ouvert sous ce compte Windows. Fermez l'autre fenêtre de Kadenz, puis relancez.",
        ),
        ErreurDemarrage::Droits(_) => tr(
            "Les droits du dossier des données de Kadenz ne sont pas ceux attendus, ou n'ont pas pu être posés. Le détail est dans le fichier erreurs.log.",
        ),
        ErreurDemarrage::AutreEdition => tr(
            "Le dossier des données appartient au serveur de l'édition multiposte : l'édition monoposte ne l'ouvre pas.",
        ),
        ErreurDemarrage::Base(erreur) => crate::sauvegardes::message_demarrage(erreur),
        ErreurDemarrage::Identite(_) => tr(
            "Le certificat de Kadenz sur cet ordinateur est illisible. Le détail est dans le fichier erreurs.log.",
        ),
        ErreurDemarrage::Ecoute(_) | ErreurDemarrage::Autre(_) => tr(
            "Kadenz n'a pas pu démarrer sur cet ordinateur. Le détail est dans le fichier erreurs.log.",
        ),
    }
}

fn impossible(f: &FenetrePrincipale, message: String) {
    f.set_repartir_propose(false);
    f.set_message_demarrage(crate::phrase(message));
    f.set_ecran(Ecran::DemarrageImpossible);
}

/// « Démarrage impossible » avec « Repartir à neuf » : une copie du poste existe
/// sans que le serveur intégré lui corresponde.
fn impossible_avec_repartir(f: &FenetrePrincipale, message: String) {
    impossible(f, message);
    f.set_repartir_propose(true);
}

/// Met de côté la copie du poste : renommée avec la date, jamais effacée (ses
/// fichiers annexes SQLite suivent).
fn mettre_la_copie_de_cote(copie: &Path) -> std::io::Result<PathBuf> {
    let date = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let nom = copie
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let nom_cible = format!("{nom}.avant-{date}");
    let cible = copie.with_file_name(&nom_cible);
    std::fs::rename(copie, &cible)?;
    for suffixe in ["-wal", "-shm", "-journal"] {
        let annexe = copie.with_file_name(format!("{nom}{suffixe}"));
        if annexe.exists() {
            std::fs::rename(
                &annexe,
                copie.with_file_name(format!("{nom_cible}{suffixe}")),
            )?;
        }
    }
    Ok(cible)
}

/// Restaure une sauvegarde du serveur sur son dossier, serveur fermé.
#[cfg(test)]
fn restaurer_dossier(dossier: &Path, sauvegarde: &Path, mot_de_passe: &str) -> Result<(), String> {
    let preparee = admin::preparer_restauration(dossier, sauvegarde, mot_de_passe)?;
    admin::restaurer_si(dossier, &preparee, EtatService::NonInstalle).map(|_| ())
}

/// Ferme la session ouverte avant une restauration : synchroniseur arrêté (la
/// file reste dans la copie), copie libérée, retour à l'écran de connexion.
fn fermer_la_session(f: &FenetrePrincipale, etat: &crate::EtatPartage) {
    let synchro = etat.borrow_mut().relie.take().map(|relie| relie.synchro);
    if let Some(synchro) = synchro {
        let _ = synchro.vider(DELAI_VIDAGE);
        synchro.arreter();
    }
    let mut e = etat.borrow_mut();
    if let Ok(provisoire) = kadenz_stockage::ouvrir_en_memoire() {
        drop(std::mem::replace(&mut e.conn, provisoire));
    }
    crate::fermer_session(f, &mut e);
}

/// Rubrique « Sauvegardes » des Paramètres en monoposte (spécification, section
/// 5) : elle règle, liste, déclenche et restaure les sauvegardes du serveur
/// intégré, et non celles de la copie du poste.
pub(crate) fn brancher_sauvegardes(f: &FenetrePrincipale, etat: &crate::EtatPartage) {
    use crate::SauvegardesApi;
    use crate::referentiels::{modele_lignes, retour_erreur, retour_ok};
    use kadenz_stockage::sauvegarde::Genre;

    let api = f.global::<SauvegardesApi>();
    let chemins: Rc<RefCell<Vec<PathBuf>>> = Rc::default();

    let afficher = {
        let (fa, chemins) = (f.as_weak(), chemins.clone());
        Rc::new(move || {
            let (Some(f), Some(c)) = (fa.upgrade(), contexte()) else {
                return;
            };
            let api = f.global::<SauvegardesApi>();
            let r = sauvegardes::lire_reglage(&c.dossier);
            api.set_dossier(r.dossier.display().to_string().into());
            api.set_mot_de_passe_defini(r.mot_de_passe.is_some());
            let liste = sauvegardes::liste(&c.dossier).unwrap_or_else(|e| {
                crate::consigner_erreur(&format!("Liste des sauvegardes du serveur : {e}"));
                Vec::new()
            });
            let lignes: Vec<Vec<String>> = liste
                .iter()
                .map(|s| {
                    vec![
                        s.date.format("%d/%m/%Y %H:%M:%S").to_string(),
                        crate::sauvegardes::genre(&s.genre),
                        crate::sauvegardes::taille(s.taille),
                    ]
                })
                .collect();
            *chemins.borrow_mut() = liste.into_iter().map(|s| s.chemin).collect();
            api.set_lignes(modele_lignes(lignes));
            api.set_ligne_courante(-1);
        })
    };

    let a = afficher.clone();
    api.on_ouvrir(move || a());

    let (fa, e, a) = (f.as_weak(), etat.clone(), afficher.clone());
    api.on_choisir_dossier(move || {
        let (Some(f), Some(c)) = (fa.upgrade(), contexte()) else {
            return crate::Retour::default();
        };
        if let Err(refus) = crate::sauvegardes::autorise(&e.borrow()) {
            return refus;
        }
        let mut boite = rfd::FileDialog::new()
            .set_title(tr("Dossier des sauvegardes"))
            .set_parent(&f.window().window_handle());
        let actuel = sauvegardes::lire_reglage(&c.dossier);
        if actuel.dossier.is_dir() {
            boite = boite.set_directory(&actuel.dossier);
        }
        let Some(dossier) = boite.pick_folder() else {
            return crate::Retour::default();
        };
        let resultat = sauvegardes::enregistrer_reglage(&c.dossier, &dossier, actuel.heure, None);
        a();
        match resultat {
            Ok(()) => retour_ok("dossier des sauvegardes enregistré"),
            Err(e) => retour_erreur(e),
        }
    });

    api.on_ouvrir_dossier(|| {
        let Some(c) = contexte() else { return };
        let dossier = sauvegardes::lire_reglage(&c.dossier).dossier;
        let _ = std::fs::create_dir_all(&dossier);
        let _ = std::process::Command::new("explorer").arg(&dossier).spawn();
    });

    let (e, a) = (etat.clone(), afficher.clone());
    api.on_definir_mot_de_passe(move |mot_de_passe, confirmation| {
        let Some(c) = contexte() else {
            return crate::Retour::default();
        };
        if let Err(refus) = crate::sauvegardes::autorise(&e.borrow()) {
            return refus;
        }
        if mot_de_passe != confirmation {
            return retour_erreur("les deux mots de passe ne correspondent pas");
        }
        let actuel = sauvegardes::lire_reglage(&c.dossier);
        let resultat = sauvegardes::enregistrer_reglage(
            &c.dossier,
            &actuel.dossier,
            actuel.heure,
            Some(&mot_de_passe),
        );
        a();
        match resultat {
            Ok(()) => retour_ok("mot de passe des sauvegardes enregistré"),
            Err(e) => retour_erreur(e),
        }
    });

    let (e, a) = (etat.clone(), afficher.clone());
    api.on_sauvegarder(move || {
        let Some(c) = contexte() else {
            return crate::Retour::default();
        };
        if let Err(refus) = crate::sauvegardes::autorise(&e.borrow()) {
            return refus;
        }
        let dossier = c.dossier.clone();
        let resultat = sur_le_serveur(&c, move |conn| {
            sauvegardes::sauvegarder(conn, &dossier, Genre::Manuelle)
        });
        a();
        match resultat {
            Ok(Ok(_)) => retour_ok("sauvegarde enregistrée"),
            Ok(Err(e)) => retour_erreur(e),
            Err(message) => retour_erreur(message.to_string()),
        }
    });

    let ch = chemins.clone();
    api.on_chemin(move |ligne| {
        usize::try_from(ligne)
            .ok()
            .and_then(|l| ch.borrow().get(l).map(|p| p.display().to_string().into()))
            .unwrap_or_default()
    });

    let (fa, e) = (f.as_weak(), etat.clone());
    api.on_restaurer(move |ligne| {
        let (Some(f), Some(c)) = (fa.upgrade(), contexte()) else {
            return crate::Retour::default();
        };
        if let Err(refus) = crate::sauvegardes::autorise(&e.borrow()) {
            return refus;
        }
        let Some(chemin) = usize::try_from(ligne)
            .ok()
            .and_then(|l| chemins.borrow().get(l).cloned())
        else {
            return retour_erreur("choisissez une sauvegarde dans la liste");
        };
        let Some(mot_de_passe) = sauvegardes::lire_reglage(&c.dossier).mot_de_passe else {
            return retour_erreur("saisissez le mot de passe de la sauvegarde");
        };
        restaurer_sauvegarde(&f, &chemin.display().to_string(), &mot_de_passe, Some(&e))
    });

    let fa = f.as_weak();
    api.on_choisir_fichier(move || match fa.upgrade() {
        Some(f) => choisir_sauvegarde(&f),
        None => SharedString::new(),
    });

    let (fa, e) = (f.as_weak(), etat.clone());
    api.on_restaurer_fichier(move |chemin, mot_de_passe| {
        let Some(f) = fa.upgrade() else {
            return crate::Retour::default();
        };
        if let Err(refus) = crate::sauvegardes::autorise(&e.borrow()) {
            return refus;
        }
        restaurer_sauvegarde(&f, &chemin, &mot_de_passe, Some(&e))
    });
}

/// Écran « Démarrage impossible », bouton « Restaurer » : le serveur intégré
/// s'arrête (sauvegarde d'arrêt comprise), la sauvegarde est restaurée sur le
/// dossier fermé, le serveur redémarre, puis le démarrage reprend comme au
/// lancement. En cas d'échec de la restauration, le serveur redémarre sur la
/// base en place.
fn restaurer_sauvegarde(
    f: &FenetrePrincipale,
    chemin: &str,
    mot_de_passe: &str,
    en_session: Option<&crate::EtatPartage>,
) -> crate::Retour {
    let Some(c) = contexte() else {
        return crate::referentiels::retour_erreur(tr("le serveur de Kadenz est arrêté"));
    };
    // Vérifiée d'abord, serveur en marche : un mot de passe faux ou un fichier
    // illisible n'arrête rien et ne ferme pas la session.
    let preparee = match admin::preparer_restauration(&c.dossier, Path::new(chemin), mot_de_passe) {
        Ok(preparee) => preparee,
        Err(e) => return crate::referentiels::retour_erreur(e),
    };
    if let Some(etat) = en_session {
        fermer_la_session(f, etat);
    }
    let serveur = c.integre.borrow_mut().take();
    if let Some(serveur) = serveur {
        serveur.arreter();
    }
    let resultat = admin::restaurer_si(&c.dossier, &preparee, EtatService::NonInstalle);
    drop(preparee);
    match Serveur::demarrer(Config::monoposte(c.dossier.clone())) {
        Ok(serveur) => {
            *c.integre.borrow_mut() = Some(serveur);
            // Restauration réussie ou non, le poste rouvre sur le serveur en place.
            serveur_demarre(f);
            match resultat {
                Ok(_) => crate::referentiels::retour_ok("sauvegarde restaurée"),
                Err(e) => {
                    crate::consigner_erreur(&format!("Restauration du serveur intégré : {e}"));
                    crate::referentiels::retour_erreur(e)
                }
            }
        }
        Err(e) => {
            crate::consigner_erreur(&format!("Redémarrage du serveur intégré : {e}"));
            impossible(f, message_erreur(&e));
            crate::referentiels::retour_erreur(message_erreur(&e))
        }
    }
}

/// « Repartir à neuf » : la copie du poste est mise de côté, puis le démarrage
/// reprend sur le serveur intégré (assistant si le serveur est neuf, sinon
/// nouvelle inscription du poste).
fn repartir_a_neuf(f: &FenetrePrincipale) {
    let Some(c) = contexte() else { return };
    if let Some(copie) = poste_relie::copie(&dossier_copie()) {
        match mettre_la_copie_de_cote(&copie) {
            Ok(cible) => crate::consigner_erreur(&format!(
                "Copie du poste mise de côté : {}",
                cible.display()
            )),
            Err(e) => {
                crate::consigner_erreur(&format!("Copie du poste non mise de côté : {e}"));
                impossible(
                    f,
                    tr(
                        "La copie des données de ce poste n'a pas pu être mise de côté. Le détail est dans le fichier erreurs.log.",
                    ),
                );
                return;
            }
        }
    }
    f.set_repartir_propose(false);
    if c.integre.borrow().is_none() {
        impossible(
            f,
            tr(
                "Kadenz n'a pas pu démarrer sur cet ordinateur. Le détail est dans le fichier erreurs.log.",
            ),
        );
        return;
    }
    serveur_demarre(f);
}

/// Choix d'un fichier de sauvegarde du serveur, en partant de son dossier.
fn choisir_sauvegarde(f: &FenetrePrincipale) -> SharedString {
    let mut boite = rfd::FileDialog::new()
        .set_title(tr("Choisir une sauvegarde"))
        .add_filter(tr("Sauvegardes Kadenz"), &["kzs"])
        .set_parent(&f.window().window_handle());
    if let Some(c) = contexte() {
        let dossier = sauvegardes::lire_reglage(&c.dossier).dossier;
        if dossier.is_dir() {
            boite = boite.set_directory(&dossier);
        }
    }
    match boite.pick_file() {
        Some(chemin) => chemin.display().to_string().into(),
        None => SharedString::new(),
    }
}

fn attendre(f: &FenetrePrincipale, texte: String) {
    let api = f.global::<MonoposteApi>();
    api.set_erreur(SharedString::new());
    api.set_attente(texte.into());
    f.set_ecran(Ecran::Attente);
}

/// Échec de l'inscription : l'écran d'attente montre l'erreur et
/// « Réessayer », qui relance l'inscription.
fn echec_inscription(f: &FenetrePrincipale, message: SharedString) {
    f.global::<MonoposteApi>().set_erreur(message);
    f.set_ecran(Ecran::Attente);
}

/// Exécute une tâche sur la base du serveur intégré, dans son fil de base.
/// Serveur absent ou arrêté : erreur, notée dans erreurs.log.
fn sur_le_serveur<R: Send + 'static>(
    contexte: &Contexte,
    tache: impl FnOnce(&mut Connexion) -> R + Send + 'static,
) -> Result<R, SharedString> {
    let integre = contexte.integre.borrow();
    let Some(serveur) = integre.as_ref() else {
        return Err(crate::phrase(tr("le serveur de Kadenz est arrêté")));
    };
    serveur.base(tache).map_err(ne_repond_pas)
}

fn ne_repond_pas(detail: String) -> SharedString {
    crate::consigner_erreur(&format!("Serveur intégré : {detail}"));
    crate::phrase(tr("le serveur de Kadenz ne répond pas"))
}

/// Travail long sur le serveur intégré, hors du fil de l'interface : le
/// serveur passe au fil le temps du travail (`None` avant son démarrage), puis
/// revient, et `suite` reçoit le résultat dans le fil de l'interface. Une
/// fermeture demandée entre-temps attend ce fil, puis arrête le serveur
/// qu'il rend (fenêtre jamais fermée sur un serveur encore ouvert).
fn en_fond<T: Send + 'static>(
    f: &FenetrePrincipale,
    c: &Contexte,
    travail: impl FnOnce(Option<Serveur>) -> (Option<Serveur>, T) + Send + 'static,
    suite: fn(&FenetrePrincipale, T),
) {
    let serveur = c.integre.borrow_mut().take();
    let fa = f.as_weak();
    let fil = std::thread::spawn(move || {
        let (serveur, resultat) = travail(serveur);
        let _ = fa.upgrade_in_event_loop(move |f| {
            if revenir(&f) {
                suite(&f, resultat);
            }
        });
        serveur
    });
    *c.fil.borrow_mut() = Some(fil);
}

/// Fin d'un travail `en_fond` : le serveur revient dans `integre`. Faux si la
/// fermeture a pris le fil entre-temps : c'est elle qui arrête le serveur.
fn revenir(f: &FenetrePrincipale) -> bool {
    let Some(c) = contexte() else { return false };
    let Some(fil) = c.fil.borrow_mut().take() else {
        return false;
    };
    match fil.join() {
        Ok(serveur) => {
            *c.integre.borrow_mut() = serveur;
            true
        }
        Err(_) => {
            crate::consigner_erreur("Serveur intégré : fil de travail interrompu");
            impossible(
                f,
                tr(
                    "Kadenz n'a pas pu démarrer sur cet ordinateur. Le détail est dans le fichier erreurs.log.",
                ),
            );
            false
        }
    }
}

/// Démarre l'édition monoposte : écran d'attente, serveur intégré démarré dans
/// un fil à part, puis l'écran qui convient à l'état trouvé.
pub(crate) fn lancer(
    f: &FenetrePrincipale,
    integre: Integre,
    demarrer: Demarrer,
    session: Session,
) {
    f.global::<MonoposteApi>().set_actif(true);
    attendre(f, tr("Démarrage de Kadenz..."));
    // erreurs.log est à côté de la base V1 : son dossier peut ne pas exister
    // encore sur une installation neuve de l'édition monoposte.
    let _ = std::fs::create_dir_all(crate::dossier_donnees());
    let Some(dossier) = dossier_serveur() else {
        crate::consigner_erreur("Serveur intégré : variable LOCALAPPDATA absente");
        impossible(
            f,
            tr(
                "Kadenz n'a pas pu démarrer sur cet ordinateur. Le détail est dans le fichier erreurs.log.",
            ),
        );
        return;
    };
    let c = Rc::new(Contexte {
        integre,
        demarrer,
        session,
        dossier: dossier.clone(),
        fil: RefCell::default(),
    });
    CONTEXTE.set(Some(c.clone()));
    brancher(f);
    en_fond(
        f,
        &c,
        move |_| match Serveur::demarrer(Config::monoposte(dossier)) {
            Ok(serveur) => (Some(serveur), Ok(())),
            Err(e) => (None, Err(e)),
        },
        |f, resultat| match resultat {
            Ok(()) => serveur_demarre(f),
            Err(e) => {
                crate::consigner_erreur(&format!("Démarrage du serveur intégré : {e}"));
                impossible(f, message_erreur(&e));
            }
        },
    );
}

fn brancher(f: &FenetrePrincipale) {
    let api = f.global::<MonoposteApi>();

    let fa = f.as_weak();
    api.on_choisir_dossier_sauvegardes(move |actuel| match fa.upgrade() {
        Some(f) => choisir_dossier(&f, &actuel),
        None => actuel,
    });

    let fa = f.as_weak();
    api.on_terminer(move |d| match fa.upgrade() {
        Some(f) => terminer(&f, &d),
        None => SharedString::new(),
    });

    let fa = f.as_weak();
    api.on_reessayer(move || {
        if let Some(f) = fa.upgrade() {
            inscrire(&f);
        }
    });

    let fa = f.as_weak();
    f.on_choisir_sauvegarde_demarrage(move || match fa.upgrade() {
        Some(f) => choisir_sauvegarde(&f),
        None => SharedString::new(),
    });

    let fa = f.as_weak();
    f.on_restaurer_au_demarrage(move |chemin, mot_de_passe| match fa.upgrade() {
        Some(f) => restaurer_sauvegarde(&f, &chemin, &mot_de_passe, None),
        None => crate::Retour::default(),
    });

    let fa = f.as_weak();
    f.on_repartir_a_neuf(move || {
        if let Some(f) = fa.upgrade() {
            repartir_a_neuf(&f);
        }
    });

    // Démarrage impossible : le dossier des données est celui du serveur.
    f.on_ouvrir_dossier_donnees(|| {
        if let Some(c) = contexte() {
            let _ = std::process::Command::new("explorer")
                .arg(&c.dossier)
                .spawn();
        }
    });

    // Fermeture : la fenêtre reste affichée sous le voile pendant
    // l'enregistrement et la sauvegarde d'arrêt (spécification, section 4).
    let fa = f.as_weak();
    f.window().on_close_requested(move || {
        if let Some(f) = fa.upgrade() {
            fermer(&f);
        }
        slint::CloseRequestResponse::KeepWindowShown
    });

    api.on_fermer_sans_attendre(|| {
        crate::consigner_erreur("Fermeture sans attendre la sauvegarde d'arrêt");
        let _ = slint::quit_event_loop();
    });
}

/// Fermeture du monoposte : voile, puis, dans un fil à part, vidage borné de
/// la file d'envoi, arrêt du synchroniseur, puis arrêt du serveur intégré,
/// qui fait la sauvegarde d'arrêt, ferme sa base et rend le verrou. La
/// fenêtre se ferme à la fin du fil ; au bout de 30 s, le voile propose de
/// fermer sans attendre la sauvegarde. Une seconde demande est ignorée.
fn fermer(f: &FenetrePrincipale) {
    if FERMETURE.replace(true) {
        return;
    }
    let api = f.global::<MonoposteApi>();
    api.set_fermeture_texte(tr("Enregistrement et sauvegarde en cours...").into());
    api.set_fermeture(true);
    let (synchro, fil, serveur) = match contexte() {
        Some(c) => {
            let synchro = c
                .session
                .borrow()
                .as_ref()
                .and_then(|etat| etat.borrow_mut().relie.take())
                .map(|relie| relie.synchro);
            let fil = c.fil.borrow_mut().take();
            (synchro, fil, c.integre.borrow_mut().take())
        }
        None => (None, None, None),
    };
    std::thread::spawn(move || {
        if let Some(synchro) = synchro {
            if !synchro.vider(DELAI_VIDAGE) {
                // Rien n'est perdu : la file reste dans la copie et part au
                // lancement suivant.
                crate::consigner_erreur("Fermeture : file d'envoi non vide après le délai");
            }
            synchro.arreter();
        }
        // Démarrage ou installation en cours : le serveur est dans ce fil.
        let serveur = serveur.or_else(|| fil.and_then(|fil| fil.join().ok().flatten()));
        if let Some(serveur) = serveur {
            serveur.arreter();
        }
        let _ = slint::quit_event_loop();
    });
    let fa = f.as_weak();
    slint::Timer::single_shot(FERMETURE_LENTE, move || {
        if let Some(f) = fa.upgrade() {
            f.global::<MonoposteApi>()
                .set_fermeture_texte(tr("La sauvegarde prend plus de temps que prévu...").into());
        }
    });
    let fa = f.as_weak();
    slint::Timer::single_shot(FERMETURE_LONGUE, move || {
        if let Some(f) = fa.upgrade() {
            f.global::<MonoposteApi>().set_fermeture_longue(true);
        }
    });
}

/// Sortie de la boucle d'événements sans passer par `fermer` (mise à jour) :
/// travail en cours attendu, puis serveur intégré arrêté, sauvegarde d'arrêt
/// comprise. Après `fermer`, il n'y a plus rien à arrêter.
pub(crate) fn arreter_a_la_sortie() {
    let Some(c) = contexte() else { return };
    let fil = c.fil.borrow_mut().take();
    let serveur = c.integre.borrow_mut().take();
    let serveur = serveur.or_else(|| fil.and_then(|fil| fil.join().ok().flatten()));
    if let Some(serveur) = serveur {
        serveur.arreter();
    }
}

/// Essai encore à confirmer : la date de début est demandée au portail (ou
/// lue dans l'attestation gardée sur ce poste), hors du fil de l'interface,
/// puis retenue par le serveur intégré. Sans réponse, rien ne change : la
/// demande repart au lancement suivant (décision 78).
fn confirmer_essai(f: &FenetrePrincipale) {
    let Some(c) = contexte() else { return };
    let Ok(Ok(true)) = sur_le_serveur(&c, |conn| {
        kadenz_stockage::abonnement::essai_a_confirmer(conn)
    }) else {
        return;
    };
    let fa = f.as_weak();
    std::thread::spawn(move || {
        let essai = kadenz_maj::essai::obtenir(kadenz_licence::entreprise::Produit::Monoposte);
        let _ = fa.upgrade_in_event_loop(move |_| {
            let Ok(essai) = essai else { return };
            let Ok(debut) = chrono::NaiveDate::parse_from_str(&essai.debut, "%Y-%m-%d") else {
                return;
            };
            if let Some(c) = contexte() {
                let retenu = sur_le_serveur(&c, move |conn| {
                    kadenz_stockage::abonnement::confirmer_essai(conn, debut)
                });
                match retenu {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => {
                        crate::consigner_erreur(&format!("Confirmation de l'essai : {e}"))
                    }
                    Err(e) => crate::consigner_erreur(&format!("Confirmation de l'essai : {e}")),
                }
            }
        });
    });
}

fn serveur_demarre(f: &FenetrePrincipale) {
    let Some(c) = contexte() else { return };
    confirmer_essai(f);
    match sur_le_serveur(&c, |conn| comptes::etat_installation(conn)) {
        Ok(Ok(EtatInstallation::AssistantRequis))
            if poste_relie::copie(&dossier_copie()).is_some() =>
        {
            // Dossier du serveur perdu, copie du poste gardée : l'assistant
            // créerait une entreprise vide (spécification, section 3.2).
            crate::consigner_erreur("Serveur intégré sans entreprise, copie du poste présente");
            impossible_avec_repartir(
                f,
                tr(
                    "Le dossier du serveur de Kadenz est vide, alors qu'une copie des données de ce poste existe. Restaurez une sauvegarde du serveur, ou repartez à neuf : la copie est alors mise de côté, jamais effacée.",
                ),
            );
        }
        Ok(Ok(EtatInstallation::AssistantRequis)) => {
            preparer_assistant(f, &c);
            f.set_ecran(Ecran::Presentation);
        }
        Ok(Ok(EtatInstallation::ConnexionRequise)) => {
            match poste_relie::copie(&dossier_copie()) {
                Some(copie) => ouvrir_poste(f, &c, &copie),
                // Entreprise installée, poste pas encore inscrit : installation
                // interrompue, reprise à l'inscription (spécification, section 3).
                None => inscrire(f),
            }
        }
        Ok(Err(e)) => {
            crate::consigner_erreur(&format!("Base du serveur intégré : {e}"));
            impossible(f, crate::sauvegardes::message_demarrage(&e));
        }
        Err(message) => impossible(f, message.to_string()),
    }
}

/// Adresse `127.0.0.1:port` et empreinte du serveur intégré, s'il tourne.
fn adresse_du_serveur(c: &Contexte) -> Option<(String, String)> {
    c.integre.borrow().as_ref().map(|serveur| {
        (
            format!("127.0.0.1:{}", serveur.adresse().port()),
            serveur.empreinte().to_string(),
        )
    })
}

/// Copie du poste relié présente : son adresse est mise au port effectif du
/// serveur intégré, avant le synchroniseur (spécification, section 2.1).
fn ouvrir_poste(f: &FenetrePrincipale, c: &Contexte, copie: &Path) {
    let Some((adresse, empreinte)) = adresse_du_serveur(c) else {
        crate::consigner_erreur("Ouverture du poste : serveur intégré absent");
        impossible(
            f,
            tr(
                "Kadenz n'a pas pu démarrer sur cet ordinateur. Le détail est dans le fichier erreurs.log.",
            ),
        );
        return;
    };
    let ouverture = poste_relie::ouvrir_copie(copie).and_then(|conn| {
        let poste = poste_relie::lire(&conn)?;
        Ok((conn, poste))
    });
    let (conn, poste) = match ouverture {
        Ok((conn, Some(poste))) => (conn, poste),
        Ok((_, None)) => {
            crate::consigner_erreur("Copie du poste sans lien au serveur intégré");
            impossible(
                f,
                tr(
                    "La copie des données de ce poste est incomplète. Le détail est dans le fichier erreurs.log.",
                ),
            );
            return;
        }
        Err(e) => {
            crate::consigner_erreur(&format!("Ouverture de la copie du poste : {e}"));
            impossible(f, crate::sauvegardes::message_demarrage(&e));
            return;
        }
    };
    // Un autre serveur (dossier du serveur recréé) : rien n'est écrasé.
    if poste.empreinte_serveur != empreinte {
        crate::consigner_erreur(&format!(
            "Copie du poste liée au serveur {}, serveur intégré {empreinte}",
            poste.empreinte_serveur
        ));
        impossible_avec_repartir(
            f,
            tr(
                "La copie des données de ce poste appartient à un autre serveur que celui de cet ordinateur : elle n'est pas ouverte, et rien n'est effacé. Le détail est dans le fichier erreurs.log.",
            ),
        );
        return;
    }
    if poste.adresse != adresse {
        let poste = PosteRelie { adresse, ..poste };
        if let Err(e) = poste_relie::garder(&conn, &poste) {
            crate::consigner_erreur(&format!("Adresse du poste relié : {e}"));
            impossible(f, crate::sauvegardes::message_demarrage(&e));
            return;
        }
    }
    if let Err(e) = (c.demarrer)(f, conn) {
        crate::consigner_erreur(&format!("Démarrage sur la copie du poste : {e}"));
        impossible(f, crate::sauvegardes::message_demarrage(&e));
    }
}

/// Valeurs proposées par l'assistant et activation par code : la licence
/// d'abonnement est liée à l'identifiant de l'entreprise du serveur.
fn preparer_assistant(f: &FenetrePrincipale, c: &Contexte) {
    let api = f.global::<MonoposteApi>();
    let defaut = Parametres::default();
    api.set_defaut_taux_tva(saisie::liste_pourcentages_en_texte(&defaut.taux_tva).into());
    api.set_defaut_taux_remise(saisie::liste_pourcentages_en_texte(&defaut.taux_remise).into());
    api.set_defaut_taux_retenue(saisie::liste_pourcentages_en_texte(&defaut.taux_retenue).into());
    api.set_defaut_modes_paiement(defaut.modes_paiement.join(" ; ").into());
    // Lu après le démarrage du serveur, qui pose la marque de portée compte :
    // le dossier proposé est alors Documents\Kadenz\Sauvegardes\Serveur.
    let reglage = sauvegardes::lire_reglage(&c.dossier);
    api.set_defaut_dossier_sauvegardes(reglage.dossier.display().to_string().into());
    api.set_defaut_heure_sauvegardes(reglage.heure.format("%H:%M").to_string().into());

    let entreprise = sur_le_serveur(c, |conn| synchro::identifiant_entreprise(conn))
        .ok()
        .and_then(Result::ok)
        .unwrap_or_default();
    f.global::<LicenceApi>()
        .set_poste(entreprise.clone().into());
    brancher_activation(f, entreprise);
}

/// Activation d'un code au « Suivant » de l'étape Entreprise (L5 bis), dans un
/// fil à part. Remplacé par celui de la rubrique « Abonnement » au démarrage
/// du poste relié.
fn brancher_activation(f: &FenetrePrincipale, entreprise: String) {
    let fa = f.as_weak();
    f.global::<ActivationUi>().on_activer(move |code, client| {
        let Some(f) = fa.upgrade() else { return };
        let ui = f.global::<ActivationUi>();
        ui.set_message(SharedString::new());
        ui.set_etat(1);
        let (code, client, entreprise, fa) = (
            code.to_string(),
            client.to_string(),
            entreprise.clone(),
            fa.clone(),
        );
        std::thread::spawn(move || {
            let resultat = crate::activation_ui::activer(&code, &entreprise, &client);
            let _ = fa.upgrade_in_event_loop(move |f| {
                let ui = f.global::<ActivationUi>();
                match resultat {
                    Ok((texte, l)) => {
                        ui.set_licence(texte.into());
                        ui.set_resume(
                            trf(
                                "{}, {} postes, jusqu'au {}",
                                &[&l.client, &l.postes, &date_jour(&l.echeance)],
                            )
                            .into(),
                        );
                        ui.set_etat(2);
                    }
                    Err(m) => {
                        ui.set_message(crate::activation_ui::message(&m).into());
                        ui.set_etat(3);
                    }
                }
            });
        });
    });
}

/// Date `AAAA-MM-JJ` en JJ/MM/AAAA.
fn date_jour(texte: &str) -> String {
    chrono::NaiveDate::parse_from_str(texte, "%Y-%m-%d")
        .map(|d| d.format("%d/%m/%Y").to_string())
        .unwrap_or_else(|_| texte.to_string())
}

/// Dossier choisi dans la boîte de Windows ; `actuel` si le choix est abandonné.
fn choisir_dossier(f: &FenetrePrincipale, actuel: &str) -> SharedString {
    let mut boite = rfd::FileDialog::new()
        .set_title(tr("Dossier des sauvegardes"))
        .set_parent(&f.window().window_handle());
    if Path::new(actuel).is_dir() {
        boite = boite.set_directory(actuel);
    }
    match boite.pick_folder() {
        Some(dossier) => dossier.display().to_string().into(),
        None => actuel.into(),
    }
}

/// Heure de la sauvegarde quotidienne, saisie HH:MM (12:00, ou 12h00).
fn heure_sauvegarde(texte: &str) -> Result<NaiveTime, String> {
    NaiveTime::parse_from_str(&texte.trim().replace(['h', 'H'], ":"), "%H:%M").map_err(|_| {
        tr("heure des sauvegardes invalide : la saisir sous la forme HH:MM, par exemple 12:00")
    })
}

/// Données de l'assistant, comme dans la fenêtre d'administration du serveur.
fn installation_depuis_assistant(d: &DonneesAssistant) -> Result<InstallationInitiale, String> {
    let taux = |libelle: &str, texte: &SharedString| {
        saisie::liste_pourcentages_depuis_texte(texte).ok_or_else(|| {
            trf(
                "taux de {} invalides : saisir des pourcentages entre 0 et 100 séparés par des points-virgules",
                &[&tr(libelle)],
            )
        })
    };
    Ok(InstallationInitiale {
        entreprise: Entreprise {
            nom: d.nom.to_string(),
            adresse: d.adresse.to_string(),
            telephone: d.telephone.to_string(),
            email: d.email.to_string(),
            site_web: d.site_web.to_string(),
            localisation: d.localisation.to_string(),
            regime_imposition: d.regime.to_string(),
            rccm: d.rccm.to_string(),
            ifu: d.ifu.to_string(),
            banque1: d.banque1.to_string(),
            banque2: d.banque2.to_string(),
            responsable_facturation: d.responsable_facturation.to_string(),
            responsable_comptabilite: d.responsable_comptabilite.to_string(),
        },
        admin: NouvelUtilisateur {
            nom_complet: d.admin_nom.to_string(),
            identifiant: d.admin_identifiant.to_string(),
            mot_de_passe: d.admin_mot_de_passe.to_string(),
            role: Role::Admin,
        },
        parametres: Parametres {
            taux_tva: taux("TVA", &d.taux_tva)?,
            taux_remise: taux("remise", &d.taux_remise)?,
            taux_retenue: taux("retenue", &d.taux_retenue)?,
            modes_paiement: saisie::liste_libelles_depuis_texte(&d.modes_paiement),
            orthographe: if d.orthographe_traditionnelle {
                Orthographe::Traditionnelle
            } else {
                Orthographe::Rectifiee
            },
            validite_devis_jours: None,
            delai_paiement_jours: None,
            numerotation: Default::default(),
            ..Parametres::default()
        },
    })
}

/// Fin de l'assistant : réglage des sauvegardes d'abord (un réglage refusé
/// n'installe rien), puis installation tout ou rien sur la base du serveur,
/// puis inscription automatique. Rend un message d'erreur, ou une chaîne vide.
fn terminer(f: &FenetrePrincipale, d: &DonneesAssistant) -> SharedString {
    let Some(c) = contexte() else {
        return SharedString::new();
    };
    let init = match installation_depuis_assistant(d) {
        Ok(init) => init,
        Err(message) => return message.into(),
    };
    let heure = match heure_sauvegarde(&d.sauvegarde_heure) {
        Ok(heure) => heure,
        Err(message) => return message.into(),
    };
    if let Err(e) = sauvegardes::enregistrer_reglage(
        &c.dossier,
        Path::new(d.sauvegarde_dossier.trim()),
        heure,
        Some(d.sauvegarde_mot_de_passe.as_str()),
    ) {
        return tr(&e).into();
    }
    // Licence facultative : sans elle, lecture seule (décision 40), un seul
    // poste permis (décision 41), celui-ci.
    let cle = d.cle_licence.trim().to_string();
    let api = f.global::<MonoposteApi>();
    api.set_erreur_installation(SharedString::new());
    api.set_installation(true);
    // Hors du fil de l'interface : le hachage du mot de passe de l'Admin
    // (Argon2) gèlerait la fenêtre.
    en_fond(
        f,
        &c,
        move |serveur| {
            let resultat = serveur.as_ref().map(|serveur| {
                serveur.base(move |conn| {
                    let licence = (!cle.is_empty()).then_some(cle.as_str());
                    admin::installer(conn, &init, licence)?;
                    // Sans licence : l'essai attend la confirmation du portail.
                    if licence.is_none() {
                        kadenz_stockage::abonnement::noter_essai_a_confirmer(conn)?;
                    }
                    Ok::<(), kadenz_stockage::Erreur>(())
                })
            });
            (serveur, resultat)
        },
        |f, resultat| {
            let api = f.global::<MonoposteApi>();
            api.set_installation(false);
            let erreur = match resultat {
                Some(Ok(Ok(()))) => {
                    confirmer_essai(f);
                    inscrire(f);
                    return;
                }
                Some(Ok(Err(e))) => crate::phrase(e),
                Some(Err(detail)) => ne_repond_pas(detail),
                None => crate::phrase(tr("le serveur de Kadenz est arrêté")),
            };
            api.set_erreur_installation(erreur);
        },
    );
    SharedString::new()
}

/// Inscription automatique de ce poste sur le serveur intégré
/// (spécification, section 3, étape 4) : code créé sur le serveur, puis
/// `Client::inscrire` sur 127.0.0.1 dans un fil à part ; un échec annule le
/// code aussitôt.
fn inscrire(f: &FenetrePrincipale) {
    let Some(c) = contexte() else { return };
    attendre(f, tr("Inscription de ce poste..."));
    let Some((adresse, empreinte)) = adresse_du_serveur(&c) else {
        echec_inscription(f, crate::phrase(tr("le serveur de Kadenz est arrêté")));
        return;
    };
    let machine = nom_machine();
    let nom = machine.clone();
    let code = sur_le_serveur(&c, move |conn| nouveau_code(conn, &nom));
    let code = match code {
        Ok(Ok(code)) => code,
        Ok(Err(e)) => {
            crate::consigner_erreur(&format!("Code d'inscription du poste : {e}"));
            echec_inscription(f, crate::phrase(e));
            return;
        }
        Err(message) => {
            echec_inscription(f, message);
            return;
        }
    };
    let fa = f.as_weak();
    std::thread::spawn(move || {
        let resultat = Identite::nouvelle(&machine)
            .map_err(ErreurClient::Protocole)
            .and_then(|identite| {
                Client::inscrire(&adresse, &empreinte, &identite, &code.secret, DELAI)
                    .map(|inscrit| (identite, inscrit))
            });
        let _ = fa.upgrade_in_event_loop(move |f| match resultat {
            Ok((identite, inscrit)) => {
                inscription_reussie(&f, adresse, empreinte, identite, inscrit)
            }
            Err(e) => {
                if let Some(c) = contexte() {
                    let id = code.id;
                    let _ = sur_le_serveur(&c, move |conn| {
                        inscription::annuler_code(conn, id, Utc::now())
                    });
                }
                crate::consigner_erreur(&format!("Inscription automatique du poste : {e:?}"));
                echec_inscription(&f, crate::phrase(crate::serveur::texte_erreur_client(&e)));
            }
        });
    });
}

/// Code d'inscription de ce poste. Sans copie, un poste déjà inscrit est
/// celui d'une inscription interrompue, dont l'identité est perdue : en
/// monoposte, aucun autre poste ne peut exister. Il est retiré, sinon la
/// limite d'un poste (décision 41) refuserait la nouvelle inscription.
fn nouveau_code(conn: &mut Connexion, nom: &str) -> Result<CodeCree, kadenz_stockage::Erreur> {
    for poste in postes::liste(conn)? {
        postes::retirer(conn, &poste.empreinte)?;
    }
    inscription::creer_code(conn, nom, Utc::now())
}

fn inscription_reussie(
    f: &FenetrePrincipale,
    adresse: String,
    empreinte: String,
    identite: Identite,
    inscrit: Inscrit,
) {
    let Some(c) = contexte() else { return };
    // Fermeture en cours : pas de copie ; au lancement suivant, l'inscription
    // reprend (poste retiré, nouveau code).
    if FERMETURE.get() {
        return;
    }
    let poste = PosteRelie {
        adresse,
        empreinte_serveur: empreinte,
        entreprise: inscrit.entreprise,
        nom: inscrit.nom,
        certificat: identite.certificat,
        cle: identite.cle,
    };
    let ouverture = poste_relie::creer_copie(&dossier_copie(), &poste)
        .and_then(|chemin| poste_relie::ouvrir_copie(&chemin));
    let conn = match ouverture {
        Ok(conn) => conn,
        Err(e) => {
            crate::consigner_erreur(&format!("Copie du poste relié : {e}"));
            echec_inscription(f, crate::phrase(e));
            return;
        }
    };
    // Client de la V1 : sa base est proposée à la reprise à la connexion
    // de l'Admin (spécification, section 3.0) ; elle reste intacte.
    REPRISE_A_PROPOSER.set(crate::chemin_base().is_file());
    if let Err(e) = (c.demarrer)(f, conn) {
        crate::consigner_erreur(&format!("Démarrage sur la copie du poste : {e}"));
        impossible(f, crate::sauvegardes::message_demarrage(&e));
    }
}

/// Après la connexion : si une base V1 attend sa reprise et que le compte est
/// l'Admin, la session s'ouvre sur l'assistant de reprise (Paramètres,
/// rubrique « Serveur »), une seule fois.
pub(crate) fn apres_connexion(f: &FenetrePrincipale, utilisateur: &Utilisateur) {
    if utilisateur.role != Role::Admin || !REPRISE_A_PROPOSER.get() {
        return;
    }
    REPRISE_A_PROPOSER.set(false);
    f.set_page(Page::Parametres);
    f.global::<ReglagesApi>().set_rubrique(Rubrique::Serveur);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heure_des_sauvegardes() {
        let heure = |h, m| NaiveTime::from_hms_opt(h, m, 0).unwrap();
        assert_eq!(heure_sauvegarde("07:30").unwrap(), heure(7, 30));
        assert_eq!(heure_sauvegarde(" 12h00 ").unwrap(), heure(12, 0));
        assert!(heure_sauvegarde("25:00").is_err());
        assert!(heure_sauvegarde("").is_err());
    }

    #[test]
    fn multiposte_sans_la_variable() {
        if std::env::var_os("KADENZ_MONOPOSTE").is_none() {
            assert!(!edition_monoposte());
        }
    }

    fn donnees() -> DonneesAssistant {
        let defaut = Parametres::default();
        DonneesAssistant {
            nom: "Essai monoposte".into(),
            admin_nom: "Admin essai".into(),
            admin_identifiant: "admin".into(),
            admin_mot_de_passe: "motdepasse-essai".into(),
            taux_tva: saisie::liste_pourcentages_en_texte(&defaut.taux_tva).into(),
            taux_remise: saisie::liste_pourcentages_en_texte(&defaut.taux_remise).into(),
            taux_retenue: saisie::liste_pourcentages_en_texte(&defaut.taux_retenue).into(),
            modes_paiement: defaut.modes_paiement.join(" ; ").into(),
            orthographe_traditionnelle: true,
            ..Default::default()
        }
    }

    /// Inscription réussie sur le serveur, puis processus arrêté avant la
    /// copie : l'identité du poste est perdue. La reprise retire l'ancien
    /// poste, et la nouvelle inscription passe malgré la limite d'un poste
    /// sans licence (décision 41). Serveur intégré réel, dossier temporaire.
    #[test]
    fn reprise_d_une_inscription_interrompue() {
        let dossier =
            std::env::temp_dir().join(format!("kadenz-monoposte-m2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        let serveur = Serveur::demarrer(Config::monoposte(dossier.join("Serveur"))).unwrap();
        let init = installation_depuis_assistant(&donnees()).unwrap();
        serveur
            .base(move |c| admin::installer(c, &init, None).map(|_| ()))
            .unwrap()
            .unwrap();
        let adresse = format!("127.0.0.1:{}", serveur.adresse().port());
        let empreinte = serveur.empreinte().to_string();
        let inscrire = |code: &CodeCree| {
            let identite = Identite::nouvelle("POSTE-ESSAI").unwrap();
            Client::inscrire(&adresse, &empreinte, &identite, &code.secret, DELAI)
        };

        let code = serveur
            .base(|c| nouveau_code(c, "POSTE-ESSAI"))
            .unwrap()
            .unwrap();
        inscrire(&code).unwrap();

        // Sans retrait : refusée, un seul poste permis sans licence.
        let code = serveur
            .base(|c| inscription::creer_code(c, "POSTE-ESSAI", Utc::now()))
            .unwrap()
            .unwrap();
        let refus = inscrire(&code);
        assert!(refus.is_err(), "{refus:?}");

        // Reprise : ancien poste retiré, nouvelle inscription acceptée.
        let code = serveur
            .base(|c| nouveau_code(c, "POSTE-ESSAI"))
            .unwrap()
            .unwrap();
        let inscrit = inscrire(&code).unwrap();
        assert_eq!(inscrit.nom, "POSTE-ESSAI");
        assert_eq!(
            serveur
                .base(|c| postes::nombre_inscrits(c))
                .unwrap()
                .unwrap(),
            1
        );
        // L'arrêt rend la main base fermée : le dossier s'efface aussitôt.
        serveur.arreter();
        std::fs::remove_dir_all(&dossier).unwrap();
        assert!(
            !dossier.exists(),
            "dossier d'essai resté : {}",
            dossier.display()
        );
    }

    /// « Repartir à neuf » : la copie est renommée avec la date et garde son
    /// contenu, annexes SQLite comprises ; rien n'est effacé. Chemins accentués.
    #[test]
    fn la_copie_est_mise_de_cote_sans_rien_effacer() {
        let dossier = std::env::temp_dir().join(format!("kadenz-côté-été-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(&dossier).unwrap();
        let copie = dossier.join("copie.db");
        std::fs::write(&copie, b"donnees d'ete").unwrap();
        std::fs::write(dossier.join("copie.db-wal"), b"annexe").unwrap();

        let cible = mettre_la_copie_de_cote(&copie).unwrap();
        assert!(!copie.exists());
        assert!(!dossier.join("copie.db-wal").exists());
        let nom = cible.file_name().unwrap().to_string_lossy().into_owned();
        assert!(nom.starts_with("copie.db.avant-"), "{nom}");
        assert_eq!(std::fs::read(&cible).unwrap(), b"donnees d'ete");
        let annexe = dossier.join(format!("{nom}-wal"));
        assert_eq!(std::fs::read(annexe).unwrap(), b"annexe");
        let _ = std::fs::remove_dir_all(&dossier);
    }

    /// Serveur intégré réel : sauvegarde, modification, arrêt, restauration sur
    /// le dossier fermé, redémarrage. L'entreprise revient à son état d'avant,
    /// noms accentués compris. Un mot de passe faux ne touche à rien.
    #[test]
    fn restauration_du_serveur_integre_depuis_une_sauvegarde() {
        let racine = std::env::temp_dir().join(format!("kadenz-m4-été-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&racine);
        let dossier = racine.join("Serveur");
        let sauvegardes_dossier = racine.join("Sauvegardes");
        let nom = |serveur: &Serveur| {
            serveur
                .base(|c| kadenz_stockage::entreprise::lire(c).map(|e| e.map(|e| e.nom)))
                .unwrap()
                .unwrap()
                .unwrap_or_default()
        };

        let serveur = Serveur::demarrer(Config::monoposte(dossier.clone())).unwrap();
        let mut donnees = donnees();
        donnees.nom = "Établissements Ouédraogo & Fils".into();
        let init = installation_depuis_assistant(&donnees).unwrap();
        serveur
            .base(move |c| admin::installer(c, &init, None).map(|_| ()))
            .unwrap()
            .unwrap();
        sauvegardes::enregistrer_reglage(
            &dossier,
            &sauvegardes_dossier,
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            Some("motdepasse-sauvegarde"),
        )
        .unwrap();
        let d = dossier.clone();
        let fichier = serveur
            .base(move |c| {
                sauvegardes::sauvegarder(c, &d, kadenz_stockage::sauvegarde::Genre::Manuelle)
            })
            .unwrap()
            .unwrap();
        assert_eq!(nom(&serveur), "Établissements Ouédraogo & Fils");
        serveur
            .base(|c| c.execute("UPDATE entreprise SET nom = 'Modifiée' WHERE id = 1", []))
            .unwrap()
            .unwrap();
        assert_eq!(nom(&serveur), "Modifiée");
        serveur.arreter();

        // Mot de passe faux : refusé, la base en place reste celle d'avant.
        assert!(admin::preparer_restauration(&dossier, &fichier, "faux-mot-de-passe").is_err());
        restaurer_dossier(&dossier, &fichier, "motdepasse-sauvegarde").unwrap();
        let serveur = Serveur::demarrer(Config::monoposte(dossier.clone())).unwrap();
        assert_eq!(nom(&serveur), "Établissements Ouédraogo & Fils");
        serveur.arreter();
        let _ = std::fs::remove_dir_all(&racine);
    }

    #[test]
    fn restauration_d_une_sauvegarde_introuvable_refusee() {
        let dossier = std::env::temp_dir().join(format!("kadenz-restau-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(&dossier).unwrap();
        let erreur = restaurer_dossier(&dossier, &dossier.join("absente.kzs"), "mdp").unwrap_err();
        assert!(erreur.contains("introuvable"), "{erreur}");
        let _ = std::fs::remove_dir_all(&dossier);
    }
}
