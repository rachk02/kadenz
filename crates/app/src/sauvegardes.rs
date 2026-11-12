//! Sauvegardes de la base (cahier des charges, section 10.3) : rubrique des
//! Paramètres, sauvegarde automatique à la fermeture et restauration, y compris
//! depuis l'écran « Démarrage impossible ».
//!
//! Le stockage (kadenz_stockage::sauvegarde) fait les copies et la restauration ;
//! ce module contrôle le droit « Sauvegarder / exporter la base », ferme la
//! connexion avant de restaurer, puis rouvre la base et revient à la connexion.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use kadenz_domaine::role::Droit;
use kadenz_stockage::sauvegarde::{self, Genre, Reglage};
use kadenz_stockage::{Erreur, droits};
use slint::{ComponentHandle, SharedString};

use crate::referentiels::{retour_erreur, retour_ok};
use crate::traduction::{tr, trf};
use crate::{Etat, EtatPartage, FenetrePrincipale, Retour, SauvegardesApi, chemin_base};

/// Extension des fichiers de sauvegarde (noms fixés par le stockage).
const EXTENSION: &str = "kzs";

/// Réglage des sauvegardes de ce poste ; illisible, le dossier par défaut et aucun
/// mot de passe (le détail va dans erreurs.log).
fn reglage() -> Reglage {
    sauvegarde::lire_reglage(&chemin_base()).unwrap_or_else(|erreur| {
        crate::consigner_erreur(&format!("Réglage des sauvegardes illisible : {erreur}"));
        Reglage {
            dossier: sauvegarde::dossier_par_defaut(),
            mot_de_passe: None,
        }
    })
}

fn genre(g: &Genre) -> String {
    match g {
        Genre::Automatique => tr("Automatique"),
        Genre::Manuelle => tr("Manuelle"),
        Genre::AvantRestauration => tr("Avant restauration"),
    }
}

/// Taille en kilo-octets, arrondie au-dessus : pas de séparateur décimal à traduire.
fn taille(octets: u64) -> String {
    trf("{} Ko", &[&octets.div_ceil(1024)])
}

/// Relit le réglage et le dossier, et met la rubrique à jour ; garde les chemins
/// des lignes affichées.
fn afficher(fenetre: &FenetrePrincipale, chemins: &RefCell<Vec<PathBuf>>) {
    let api = fenetre.global::<SauvegardesApi>();
    let r = reglage();
    api.set_dossier(r.dossier.display().to_string().into());
    api.set_mot_de_passe_defini(r.mot_de_passe.is_some());
    let liste = sauvegarde::liste_sauvegardes(&r.dossier).unwrap_or_else(|erreur| {
        crate::consigner_erreur(&format!("Liste des sauvegardes illisible : {erreur}"));
        Vec::new()
    });
    let lignes: Vec<Vec<String>> = liste
        .iter()
        .map(|s| {
            vec![
                s.date.format("%d/%m/%Y %H:%M:%S").to_string(),
                genre(&s.genre),
                taille(s.taille),
            ]
        })
        .collect();
    *chemins.borrow_mut() = liste.into_iter().map(|s| s.chemin).collect();
    api.set_lignes(crate::referentiels::modele_lignes(lignes));
    api.set_ligne_courante(-1);
}

/// Le compte connecté peut-il sauvegarder et restaurer ?
fn autorise(etat: &Etat) -> Result<(), Retour> {
    let Some(acteur) = &etat.utilisateur else {
        return Err(retour_erreur("aucune session ouverte"));
    };
    match droits::possede(&etat.conn, acteur, Droit::SauvegarderBase) {
        Ok(true) => Ok(()),
        Ok(false) => Err(retour_erreur(Erreur::Interdit(Droit::SauvegarderBase))),
        Err(erreur) => Err(retour_erreur(erreur)),
    }
}

/// Choix d'un fichier de sauvegarde, en partant du dossier des sauvegardes.
fn choisir_fichier(fenetre: &FenetrePrincipale) -> SharedString {
    let mut boite = rfd::FileDialog::new()
        .set_title(tr("Choisir une sauvegarde"))
        .add_filter(tr("Sauvegardes Kadenz"), &[EXTENSION])
        .set_parent(&fenetre.window().window_handle());
    let dossier = reglage().dossier;
    if dossier.is_dir() {
        boite = boite.set_directory(&dossier);
    }
    match boite.pick_file() {
        Some(chemin) => chemin.display().to_string().into(),
        None => SharedString::new(),
    }
}

/// Restaure une sauvegarde pendant une session : la connexion est fermée avant
/// (Windows refuse de renommer une base ouverte), la base rouverte après, puis
/// chacun se reconnecte. En cas d'échec, la base en place est rouverte.
fn restaurer_en_session(
    fenetre: &FenetrePrincipale,
    etat: &EtatPartage,
    sauvegarde_choisie: &Path,
    mot_de_passe: &str,
) -> Retour {
    let mut e = etat.borrow_mut();
    if let Err(refus) = autorise(&e) {
        return refus;
    }
    let chemin = chemin_base();
    // Fermeture : une base en mémoire prend la place le temps de l'échange.
    match kadenz_stockage::ouvrir_en_memoire() {
        Ok(provisoire) => drop(std::mem::replace(&mut e.conn, provisoire)),
        Err(erreur) => return retour_erreur(erreur),
    }
    let resultat = sauvegarde::restaurer(&chemin, sauvegarde_choisie, mot_de_passe);
    match kadenz_stockage::ouvrir(&chemin) {
        Ok(conn) => e.conn = conn,
        Err(erreur) => {
            // La base ne se rouvre pas : l'écran de démarrage impossible prend le relais.
            crate::consigner_erreur(&format!(
                "Réouverture de la base après restauration : {erreur}"
            ));
            drop(e);
            fenetre.set_message_demarrage(message_demarrage(&erreur).into());
            fenetre.set_ecran(crate::Ecran::DemarrageImpossible);
            return retour_erreur(erreur);
        }
    }
    match resultat {
        Ok(()) => {
            crate::licence::ouvrir(fenetre, &e.conn, &e.poste);
            crate::fermer_session(fenetre, &mut e);
            retour_ok("sauvegarde restaurée ; reconnectez-vous")
        }
        Err(erreur) => retour_erreur(erreur),
    }
}

pub(crate) fn brancher(fenetre: &FenetrePrincipale, etat: &EtatPartage) {
    let api = fenetre.global::<SauvegardesApi>();
    let chemins: Rc<RefCell<Vec<PathBuf>>> = Rc::default();

    let (f, c) = (fenetre.as_weak(), chemins.clone());
    api.on_ouvrir(move || {
        if let Some(f) = f.upgrade() {
            afficher(&f, &c);
        }
    });

    let (f, e, c) = (fenetre.as_weak(), etat.clone(), chemins.clone());
    api.on_choisir_dossier(move || {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        // Dialogue modal : aucun emprunt de l'état pendant qu'il est ouvert.
        let mut boite = rfd::FileDialog::new()
            .set_title(tr("Dossier des sauvegardes"))
            .set_parent(&f.window().window_handle());
        let actuel = reglage().dossier;
        if actuel.is_dir() {
            boite = boite.set_directory(&actuel);
        }
        let Some(dossier) = boite.pick_folder() else {
            return Retour::default();
        };
        let resultat = {
            let etat = e.borrow();
            let Some(acteur) = &etat.utilisateur else {
                return retour_erreur("aucune session ouverte");
            };
            sauvegarde::enregistrer_reglage(&etat.conn, acteur, &chemin_base(), &dossier, None)
        };
        afficher(&f, &c);
        match resultat {
            Ok(()) => retour_ok("dossier des sauvegardes enregistré"),
            Err(erreur) => retour_erreur(erreur),
        }
    });

    api.on_ouvrir_dossier(|| {
        let dossier = reglage().dossier;
        if let Err(erreur) = std::fs::create_dir_all(&dossier) {
            crate::consigner_erreur(&format!("Création du dossier des sauvegardes : {erreur}"));
        }
        let _ = std::process::Command::new("explorer").arg(&dossier).spawn();
    });

    let (f, e, c) = (fenetre.as_weak(), etat.clone(), chemins.clone());
    api.on_definir_mot_de_passe(move |mot_de_passe, confirmation| {
        if mot_de_passe != confirmation {
            return retour_erreur("les deux mots de passe ne correspondent pas");
        }
        let resultat = {
            let etat = e.borrow();
            let Some(acteur) = &etat.utilisateur else {
                return retour_erreur("aucune session ouverte");
            };
            sauvegarde::enregistrer_reglage(
                &etat.conn,
                acteur,
                &chemin_base(),
                &reglage().dossier,
                Some(&mot_de_passe),
            )
        };
        if let Some(f) = f.upgrade() {
            afficher(&f, &c);
        }
        match resultat {
            Ok(()) => retour_ok("mot de passe des sauvegardes enregistré"),
            Err(erreur) => retour_erreur(erreur),
        }
    });

    let (f, e, c) = (fenetre.as_weak(), etat.clone(), chemins.clone());
    api.on_sauvegarder(move || {
        let resultat = {
            let etat = e.borrow();
            let Some(acteur) = &etat.utilisateur else {
                return retour_erreur("aucune session ouverte");
            };
            sauvegarde::sauvegarde_manuelle(&etat.conn, acteur, &reglage())
        };
        if let Some(f) = f.upgrade() {
            afficher(&f, &c);
        }
        match resultat {
            Ok(_) => retour_ok("sauvegarde enregistrée"),
            Err(erreur) => retour_erreur(erreur),
        }
    });

    let c = chemins.clone();
    api.on_chemin(move |ligne| {
        usize::try_from(ligne)
            .ok()
            .and_then(|l| c.borrow().get(l).map(|p| p.display().to_string().into()))
            .unwrap_or_default()
    });

    let (f, e, c) = (fenetre.as_weak(), etat.clone(), chemins);
    api.on_restaurer(move |ligne| {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        let Some(chemin) = usize::try_from(ligne)
            .ok()
            .and_then(|l| c.borrow().get(l).cloned())
        else {
            return retour_erreur("choisissez une sauvegarde dans la liste");
        };
        let Some(mot_de_passe) = reglage().mot_de_passe else {
            return retour_erreur("saisissez le mot de passe de la sauvegarde");
        };
        restaurer_en_session(&f, &e, &chemin, &mot_de_passe)
    });

    let f = fenetre.as_weak();
    api.on_choisir_fichier(move || match f.upgrade() {
        Some(f) => choisir_fichier(&f),
        None => SharedString::new(),
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_restaurer_fichier(move |chemin, mot_de_passe| {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        restaurer_en_session(&f, &e, Path::new(chemin.as_str()), &mot_de_passe)
    });
}

/// Callbacks de l'écran « Démarrage impossible », branchés avant toute session :
/// restaurer une sauvegarde (le mot de passe de la sauvegarde est la seule garde),
/// puis démarrer normalement ; ou ouvrir le dossier des données.
pub(crate) fn brancher_demarrage(
    fenetre: &FenetrePrincipale,
    demarrer: impl Fn(&FenetrePrincipale, kadenz_stockage::Connexion) -> Result<(), Erreur> + 'static,
) {
    let f = fenetre.as_weak();
    fenetre.on_choisir_sauvegarde_demarrage(move || match f.upgrade() {
        Some(f) => choisir_fichier(&f),
        None => SharedString::new(),
    });

    let f = fenetre.as_weak();
    fenetre.on_restaurer_au_demarrage(move |chemin, mot_de_passe| {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        let base = chemin_base();
        if let Err(erreur) = sauvegarde::restaurer(&base, Path::new(chemin.as_str()), &mot_de_passe)
        {
            return retour_erreur(erreur);
        }
        match kadenz_stockage::ouvrir(&base).and_then(|conn| demarrer(&f, conn)) {
            Ok(()) => retour_ok("sauvegarde restaurée"),
            Err(erreur) => {
                crate::consigner_erreur(&format!("Ouverture de la base restaurée : {erreur}"));
                f.set_message_demarrage(message_demarrage(&erreur).into());
                retour_erreur(erreur)
            }
        }
    });

    fenetre.on_ouvrir_dossier_donnees(|| {
        if let Some(dossier) = chemin_base().parent() {
            let _ = std::process::Command::new("explorer").arg(dossier).spawn();
        }
    });
}

/// Sauvegarde automatique à la fermeture (rotation des dix dernières dans le
/// stockage). Sans mot de passe choisi, rien n'est fait ; un échec va dans erreurs.log.
pub(crate) fn sauvegarder_a_la_fermeture(etat: &Etat) {
    if let Err(erreur) = sauvegarde::sauvegarde_auto(&etat.conn, &reglage()) {
        crate::consigner_erreur(&format!("Sauvegarde automatique à la fermeture : {erreur}"));
    }
}

/// Message de l'écran « Démarrage impossible », selon la cause.
pub(crate) fn message_demarrage(erreur: &Erreur) -> String {
    match erreur {
        Erreur::CleIntrouvable => tr(
            "La clé de la base (fichier kadenz.cle) est introuvable. Remettez ce fichier dans le dossier des données, ou restaurez une sauvegarde.",
        ),
        Erreur::CleAutreCompte => tr(
            "La clé de la base a été créée sur un autre poste ou un autre compte Windows. Ouvrez Kadenz avec le compte Windows d'origine, ou restaurez une sauvegarde.",
        ),
        Erreur::CleEndommagee => {
            tr("La clé de la base (fichier kadenz.cle) est endommagée. Restaurez une sauvegarde.")
        }
        Erreur::BaseIndechiffrable => tr(
            "La base ne s'ouvre pas avec sa clé : la clé ou le fichier de la base est endommagé. Restaurez une sauvegarde.",
        ),
        Erreur::VersionSchemaInconnue { .. } => tr(
            "La base a été enregistrée par une version plus récente de Kadenz. Installez la dernière version de Kadenz, ou restaurez une sauvegarde.",
        ),
        autre => trf(
            "La base de données n'a pas pu être ouverte : {}",
            &[&crate::referentiels::texte_erreur_ref(autre)],
        ),
    }
}
