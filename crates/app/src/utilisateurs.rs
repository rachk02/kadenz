//! Écran Utilisateurs : comptes et matrice des droits par rôle.

use std::cell::RefCell;
use std::rc::Rc;

use kadenz_domaine::role::{Droit, MatriceDroits, Role};
use kadenz_protocole::{Action, Entite};
use kadenz_stockage::comptes::{self, NouvelUtilisateur, Utilisateur};
use kadenz_stockage::droits;
use kadenz_stockage::synchro;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::referentiels::{modele_lignes, retour_erreur, retour_ok, texte_erreur};
use crate::traduction::tr;
use crate::{
    CompteUi, EtatPartage, FenetrePrincipale, LigneDroitUi, UtilisateursApi, appliquer_droits,
    phrase, relie,
};

fn rang_role(role: Role) -> i32 {
    Role::TOUS.iter().position(|r| *r == role).unwrap_or(0) as i32
}

fn role_de_rang(rang: i32) -> Role {
    Role::TOUS[usize::try_from(rang).unwrap_or(0).min(Role::TOUS.len() - 1)]
}

fn compte_ui(u: Option<&Utilisateur>, session: Option<&Utilisateur>) -> CompteUi {
    match u {
        Some(u) => CompteUi {
            nom: u.nom_complet.as_str().into(),
            identifiant: u.identifiant.as_str().into(),
            role: rang_role(u.role),
            actif: u.actif,
            existe: true,
            soi_meme: session.is_some_and(|s| s.id == u.id),
        },
        None => CompteUi {
            // Rôle proposé pour un nouveau compte : le plus restreint.
            role: rang_role(Role::Consultation),
            actif: true,
            ..Default::default()
        },
    }
}

fn afficher_comptes(fenetre: &FenetrePrincipale, etat: &mut crate::Etat) {
    let api = fenetre.global::<UtilisateursApi>();
    // Relus à chaque ouverture : ils suivent la langue du compte connecté.
    let roles: Vec<SharedString> = Role::TOUS
        .iter()
        .map(|r| tr(&r.to_string()).into())
        .collect();
    api.set_roles(ModelRc::new(VecModel::from(roles)));
    match comptes::lister(&etat.conn) {
        Ok(lignes) => {
            let cellules = lignes
                .iter()
                .map(|u| {
                    vec![
                        u.nom_complet.clone(),
                        u.identifiant.clone(),
                        tr(&u.role.to_string()),
                        if u.actif {
                            String::new()
                        } else {
                            tr("Désactivé")
                        },
                    ]
                })
                .collect();
            api.set_lignes(modele_lignes(cellules));
            let courante = etat
                .comptes
                .selection
                .and_then(|id| lignes.iter().position(|u| u.id == id))
                .map_or(-1, |i| i as i32);
            api.set_ligne_courante(courante);
            etat.comptes.lignes = lignes;
        }
        Err(e) => {
            api.set_lignes(modele_lignes(vec![vec![phrase(e).to_string()]]));
        }
    }
}

fn verifier_confirmation(mot_de_passe: &str, confirmation: &str) -> Result<(), String> {
    if mot_de_passe != confirmation {
        return Err("les deux mots de passe ne correspondent pas".into());
    }
    Ok(())
}

fn brancher_comptes(fenetre: &FenetrePrincipale, etat: &EtatPartage) {
    let api = fenetre.global::<UtilisateursApi>();

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_ouvrir(move || {
        let mut etat = e.borrow_mut();
        etat.comptes = Default::default();
        if let Some(f) = f.upgrade() {
            afficher_comptes(&f, &mut etat);
        }
    });

    let e = etat.clone();
    api.on_selectionner(move |ligne| {
        let mut etat = e.borrow_mut();
        let choisi = usize::try_from(ligne)
            .ok()
            .and_then(|i| etat.comptes.lignes.get(i))
            .cloned();
        etat.comptes.selection = choisi.as_ref().map(|u| u.id);
        compte_ui(choisi.as_ref(), etat.utilisateur.as_ref())
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_nouveau(move || {
        e.borrow_mut().comptes.selection = None;
        if let Some(f) = f.upgrade() {
            f.global::<UtilisateursApi>().set_ligne_courante(-1);
        }
        compte_ui(None, None)
    });

    let e = etat.clone();
    api.on_compte_courant(move || {
        let mut etat = e.borrow_mut();
        let compte = match etat.comptes.selection {
            Some(id) => comptes::lire(&etat.conn, id).ok().flatten(),
            None => None,
        };
        if compte.is_none() {
            etat.comptes.selection = None;
        }
        compte_ui(compte.as_ref(), etat.utilisateur.as_ref())
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_creer(move |nom, identifiant, role, mot_de_passe, confirmation| {
        if let Err(message) = verifier_confirmation(&mot_de_passe, &confirmation) {
            return retour_erreur(message);
        }
        let mut etat = e.borrow_mut();
        let Some(acteur) = etat.utilisateur.clone() else {
            return retour_erreur("aucune session ouverte");
        };
        let nouveau = NouvelUtilisateur {
            nom_complet: nom.to_string(),
            identifiant: identifiant.to_string(),
            mot_de_passe: mot_de_passe.to_string(),
            role: role_de_rang(role),
        };
        // Poste relié : création envoyée au serveur, sous un nouvel uid.
        let action = Action::CreerCompte {
            uid: synchro::nouvel_uid(),
            nom_complet: nouveau.nom_complet.clone(),
            identifiant: nouveau.identifiant.clone(),
            mot_de_passe: nouveau.mot_de_passe.clone(),
            role: nouveau.role.code().into(),
        };
        let resultat = match relie::ecrire(&etat, action.clone()) {
            Some(r) => r.map(|_| relie::id_cree(&etat, &action)),
            None => comptes::creer(&etat.conn, &acteur, &nouveau)
                .map(|u| Some(u.id))
                .map_err(texte_erreur),
        };
        if let Ok(Some(id)) = &resultat {
            etat.comptes.selection = Some(*id);
        }
        if let Some(f) = f.upgrade() {
            afficher_comptes(&f, &mut etat);
        }
        match resultat {
            Ok(_) => retour_ok("compte créé"),
            Err(erreur) => retour_erreur(erreur),
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_changer_role(move |role| {
        let mut etat = e.borrow_mut();
        let (Some(acteur), Some(id)) = (etat.utilisateur.clone(), etat.comptes.selection) else {
            return retour_erreur("aucun compte sélectionné");
        };
        let role = role_de_rang(role);
        let resultat =
            match relie::ecrire_sur(&etat, Entite::Utilisateur, id, |uid| Action::ChangerRole {
                uid,
                role: role.code().into(),
            }) {
                Some(r) => r.map(|_| ()),
                None => comptes::changer_role(&etat.conn, &acteur, id, role).map_err(texte_erreur),
            };
        if let Some(f) = f.upgrade() {
            afficher_comptes(&f, &mut etat);
        }
        match resultat {
            Ok(()) => {
                retour_ok("rôle changé ; le menu de ce compte suivra à sa prochaine connexion")
            }
            Err(erreur) => retour_erreur(erreur),
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_basculer_actif(move || {
        let mut etat = e.borrow_mut();
        let (Some(acteur), Some(id)) = (etat.utilisateur.clone(), etat.comptes.selection) else {
            return retour_erreur("aucun compte sélectionné");
        };
        let actif = match comptes::lire(&etat.conn, id) {
            Ok(Some(u)) => u.actif,
            Ok(None) => return retour_erreur("ce compte n'existe plus"),
            Err(erreur) => return retour_erreur(erreur),
        };
        let resultat = match relie::ecrire_sur(&etat, Entite::Utilisateur, id, |uid| {
            Action::DefinirActifCompte { uid, actif: !actif }
        }) {
            Some(r) => r.map(|_| ()),
            None => comptes::definir_actif(&etat.conn, &acteur, id, !actif).map_err(texte_erreur),
        };
        if let Some(f) = f.upgrade() {
            afficher_comptes(&f, &mut etat);
        }
        match resultat {
            Ok(()) if actif => retour_ok("compte désactivé"),
            Ok(()) => retour_ok("compte réactivé"),
            Err(erreur) => retour_erreur(erreur),
        }
    });

    let e = etat.clone();
    api.on_reinitialiser(move |mot_de_passe, confirmation| {
        if let Err(message) = verifier_confirmation(&mot_de_passe, &confirmation) {
            return retour_erreur(message);
        }
        let etat = e.borrow();
        let (Some(acteur), Some(id)) = (&etat.utilisateur, etat.comptes.selection) else {
            return retour_erreur("aucun compte sélectionné");
        };
        let resultat = match relie::ecrire_sur(&etat, Entite::Utilisateur, id, |uid| {
            Action::ReinitialiserMotDePasse {
                uid,
                nouveau: mot_de_passe.to_string(),
            }
        }) {
            Some(r) => r.map(|_| ()),
            None => comptes::reinitialiser_mot_de_passe(&etat.conn, acteur, id, &mot_de_passe)
                .map_err(texte_erreur),
        };
        match resultat {
            Ok(()) => retour_ok("mot de passe réinitialisé"),
            Err(erreur) => retour_erreur(erreur),
        }
    });
}

// ---------------------------------------------------------------- Matrice

fn afficher_matrice(fenetre: &FenetrePrincipale, m: &MatriceDroits) {
    let lignes: Vec<LigneDroitUi> = Droit::TOUS
        .iter()
        .map(|&droit| {
            let autorisations: Vec<bool> =
                Role::TOUS.iter().map(|&r| m.autorise(r, droit)).collect();
            let verrous: Vec<bool> = Role::TOUS
                .iter()
                .map(|&r| r == Role::Admin && droit == Droit::GererUtilisateurs)
                .collect();
            LigneDroitUi {
                libelle: tr(&droit.to_string()).into(),
                autorisations: ModelRc::new(VecModel::from(autorisations)),
                verrous: ModelRc::new(VecModel::from(verrous)),
            }
        })
        .collect();
    fenetre
        .global::<UtilisateursApi>()
        .set_matrice(ModelRc::new(VecModel::from(lignes)));
}

fn brancher_matrice(fenetre: &FenetrePrincipale, etat: &EtatPartage) {
    let api = fenetre.global::<UtilisateursApi>();
    // Matrice en cours d'édition, enregistrée seulement sur demande.
    let en_cours = Rc::new(RefCell::new(MatriceDroits::default()));

    let (f, e, m) = (fenetre.as_weak(), etat.clone(), en_cours.clone());
    api.on_ouvrir_matrice(move || {
        let lue = droits::lire_matrice(&e.borrow().conn).unwrap_or_default();
        *m.borrow_mut() = lue;
        if let Some(f) = f.upgrade() {
            afficher_matrice(&f, &m.borrow());
        }
    });

    let m = en_cours.clone();
    api.on_basculer_droit(move |ligne, colonne, autorise| {
        let (Ok(l), Ok(c)) = (usize::try_from(ligne), usize::try_from(colonne)) else {
            return;
        };
        if let (Some(&droit), Some(&role)) = (Droit::TOUS.get(l), Role::TOUS.get(c)) {
            m.borrow_mut().definir(role, droit, autorise);
        }
    });

    let (f, m) = (fenetre.as_weak(), en_cours.clone());
    api.on_matrice_par_defaut(move || {
        *m.borrow_mut() = MatriceDroits::default();
        if let Some(f) = f.upgrade() {
            afficher_matrice(&f, &m.borrow());
        }
    });

    let (f, e, m) = (fenetre.as_weak(), etat.clone(), en_cours);
    api.on_enregistrer_matrice(move || {
        let etat = e.borrow();
        let Some(acteur) = &etat.utilisateur else {
            return retour_erreur("aucune session ouverte");
        };
        let action = Action::EnregistrerMatrice {
            contenu: droits::matrice_en_json(&m.borrow()),
        };
        let resultat = match relie::ecrire(&etat, action) {
            Some(r) => r.map(|_| ()),
            None => {
                droits::enregistrer_matrice(&etat.conn, acteur, &m.borrow()).map_err(texte_erreur)
            }
        };
        match resultat {
            Ok(()) => {
                if let Some(f) = f.upgrade() {
                    // Les droits de la session en cours suivent tout de suite.
                    appliquer_droits(&f, &etat);
                }
                retour_ok("droits enregistrés")
            }
            Err(erreur) => retour_erreur(erreur),
        }
    });
}

pub(crate) fn brancher(fenetre: &FenetrePrincipale, etat: &EtatPartage) {
    brancher_comptes(fenetre, etat);
    brancher_matrice(fenetre, etat);
}
