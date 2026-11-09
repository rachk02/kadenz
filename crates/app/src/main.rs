// En version optimisée, pas de console Windows derrière la fenêtre.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod administration;
mod factures;
mod import_ecran;
mod pdf;
mod referentiels;
mod traduction;
mod utilisateurs;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use kadenz_domaine::langue::Langue;
use kadenz_domaine::lettres::Orthographe;
use kadenz_domaine::role::Droit;
use kadenz_domaine::saisie;
use kadenz_stockage::Connexion;
use kadenz_stockage::comptes::{self, EtatInstallation, NouvelUtilisateur, Utilisateur};
use kadenz_stockage::documents::TypeDocument;
use kadenz_stockage::droits;
use kadenz_stockage::entreprise::{self, Entreprise};
use kadenz_stockage::installation::{self, InstallationInitiale};
use kadenz_stockage::parametres::Parametres;
use kadenz_stockage::produits::{FiltreProduits, Produit};
use kadenz_stockage::tiers::{FiltreTiers, Tiers};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::referentiels::Vue;

slint::include_modules!();

/// Emplacement de la base : dossier de données de l'utilisateur Windows
/// (cahier des charges, section 13). La variable d'environnement `KADENZ_BASE`
/// permet de travailler sur une autre base en développement.
fn chemin_base() -> PathBuf {
    if let Some(chemin) = std::env::var_os("KADENZ_BASE") {
        return PathBuf::from(chemin);
    }
    let racine = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    racine.join("Kadenz").join("kadenz.sqlite")
}

/// Met la première lettre d'un message en majuscule et termine par un point.
/// Le message est d'abord traduit dans la langue choisie (traduction::tr).
fn phrase(message: impl std::fmt::Display) -> SharedString {
    let texte = traduction::tr(&message.to_string());
    let mut caracteres = texte.chars();
    let mut s: String = match caracteres.next() {
        Some(c) => c.to_uppercase().chain(caracteres).collect(),
        None => return SharedString::new(),
    };
    if !s.ends_with(['.', '。', '!', '?', '؟']) {
        s.push('.');
    }
    s.into()
}

struct Etat {
    conn: Connexion,
    utilisateur: Option<Utilisateur>,
    tiers: Vue<Tiers, FiltreTiers>,
    produits: Vue<Produit, FiltreProduits>,
    comptes: Vue<Utilisateur, ()>,
    factures: factures::Registre,
    facture: Option<factures::EditionFacture>,
    /// Type de document de l'écran commun des documents de vente.
    type_document: TypeDocument,
}

impl Etat {
    fn nouveau(conn: Connexion) -> Self {
        Etat {
            conn,
            utilisateur: None,
            tiers: Vue::default(),
            produits: Vue::default(),
            comptes: Vue::default(),
            factures: factures::Registre::default(),
            facture: None,
            type_document: TypeDocument::Facture,
        }
    }
}

type EtatPartage = Rc<RefCell<Etat>>;

/// Droits du compte connecté et menu correspondant, relus depuis la matrice
/// à chaque connexion (la matrice est modifiable par l'Admin).
pub(crate) fn appliquer_droits(fenetre: &FenetrePrincipale, etat: &Etat) {
    let possede = |droit| match &etat.utilisateur {
        Some(u) => droits::possede(&etat.conn, u, droit).unwrap_or(false),
        None => false,
    };
    let d = fenetre.global::<Droits>();
    d.set_ajouter_referentiel(possede(Droit::AjouterReferentiel));
    d.set_modifier_referentiel(possede(Droit::ModifierReferentiel));
    d.set_gerer_parametres(possede(Droit::GererParametres));
    d.set_gerer_utilisateurs(possede(Droit::GererUtilisateurs));

    // Les libellés et les icônes du menu sont dans fenetre-principale.slint.
    let mut menu = Vec::new();
    if possede(Droit::Consulter) {
        menu.extend([
            Page::TableauDeBord,
            Page::Devis,
            Page::Proforma,
            Page::BonsCommande,
            Page::Livraisons,
            Page::Factures,
            Page::Avoirs,
            Page::Tiers,
            Page::Produits,
        ]);
    }
    if possede(Droit::GererParametres) {
        menu.extend([Page::Entreprise, Page::Parametres]);
    }
    if possede(Droit::GererUtilisateurs) {
        menu.push(Page::Utilisateurs);
    }
    if etat.utilisateur.is_some() {
        menu.push(Page::MonCompte);
    }
    menu.push(Page::APropos);

    // La page en cours reste affichée si le menu la contient encore.
    let page = fenetre.get_page();
    fenetre.set_page(if menu.contains(&page) { page } else { menu[0] });
    fenetre.set_menu(ModelRc::new(VecModel::from(menu)));
}

/// Réglages d'affichage propres à la langue retenue (code de traduction::choisir).
fn appliquer_langue(fenetre: &FenetrePrincipale, code: &str) {
    fenetre.global::<Theme>().set_chinois(code == "zh");
}

fn afficher_session(fenetre: &FenetrePrincipale, etat: &Etat) {
    // Recalculé ici pour suivre la langue du compte.
    let chemin = etat.conn.path().unwrap_or_default().to_string();
    let afficher_etat_base = || {
        fenetre.set_etat_base(
            traduction::trf(
                "Base : {} (schéma {})",
                &[&chemin, &kadenz_stockage::version_schema_application()],
            )
            .into(),
        );
    };
    if let Some(u) = &etat.utilisateur {
        // Langue du compte, avant tout texte construit ici.
        appliquer_langue(fenetre, traduction::choisir(u.langue.code()));
        let rang = Langue::TOUTES
            .iter()
            .position(|l| *l == u.langue)
            .unwrap_or(0);
        fenetre.set_utilisateur_langue(rang as i32);
        fenetre.set_utilisateur_nom(u.nom_complet.as_str().into());
        fenetre.set_utilisateur_identifiant(u.identifiant.as_str().into());
        fenetre.set_utilisateur_role(traduction::tr(&u.role.to_string()).into());
        appliquer_droits(fenetre, etat);
        fenetre.set_ecran(Ecran::Application);
    }
    afficher_etat_base();
    if let Ok(Some(e)) = entreprise::lire(&etat.conn) {
        fenetre.set_nom_entreprise(e.nom.into());
    }
}

fn installation_depuis_assistant(d: &DonneesAssistant) -> Result<InstallationInitiale, String> {
    let taux = |libelle: &str, texte: &SharedString| {
        saisie::liste_pourcentages_depuis_texte(texte).ok_or_else(|| {
            traduction::trf(
                "taux de {} invalides : saisir des pourcentages entre 0 et 100 séparés par des points-virgules",
                &[&traduction::tr(libelle)],
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
            role: kadenz_domaine::role::Role::Admin,
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

fn brancher(fenetre: &FenetrePrincipale, etat: EtatPartage) {
    let defaut = Parametres::default();
    fenetre.set_defaut_taux_tva(saisie::liste_pourcentages_en_texte(&defaut.taux_tva).into());
    fenetre.set_defaut_taux_remise(saisie::liste_pourcentages_en_texte(&defaut.taux_remise).into());
    fenetre
        .set_defaut_taux_retenue(saisie::liste_pourcentages_en_texte(&defaut.taux_retenue).into());
    fenetre.set_defaut_modes_paiement(defaut.modes_paiement.join(" ; ").into());

    let (f, e) = (fenetre.as_weak(), etat.clone());
    fenetre.on_terminer_assistant(move |donnees| {
        let init = match installation_depuis_assistant(&donnees) {
            Ok(init) => init,
            Err(message) => return phrase(message),
        };
        let mut etat = e.borrow_mut();
        match installation::installer(&mut etat.conn, &init) {
            Ok(admin) => {
                etat.utilisateur = Some(admin);
                if let Some(f) = f.upgrade() {
                    afficher_session(&f, &etat);
                }
                SharedString::new()
            }
            Err(erreur) => phrase(erreur),
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    fenetre.on_connecter(move |identifiant, mot_de_passe| {
        let mut etat = e.borrow_mut();
        match comptes::connecter(&etat.conn, &identifiant, &mot_de_passe) {
            Ok(utilisateur) => {
                etat.utilisateur = Some(utilisateur);
                if let Some(f) = f.upgrade() {
                    afficher_session(&f, &etat);
                }
                SharedString::new()
            }
            Err(erreur) => phrase(erreur),
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    fenetre.on_deconnecter(move || {
        let mut etat = e.borrow_mut();
        etat.utilisateur = None;
        etat.tiers = Vue::default();
        etat.produits = Vue::default();
        etat.comptes = Vue::default();
        etat.factures = factures::Registre::default();
        etat.facture = None;
        let langue = traduction::choisir_demarrage();
        if let Some(f) = f.upgrade() {
            appliquer_langue(&f, langue);
            f.set_page(Page::TableauDeBord);
            appliquer_droits(&f, &etat);
            f.set_utilisateur_nom(SharedString::new());
            f.set_utilisateur_identifiant(SharedString::new());
            f.set_utilisateur_role(SharedString::new());
            afficher_session(&f, &etat);
            f.set_ecran(Ecran::Connexion);
        }
    });

    referentiels::brancher(fenetre, &etat);
    administration::brancher(fenetre, &etat);
    utilisateurs::brancher(fenetre, &etat);
    import_ecran::brancher(fenetre, &etat);
    factures::brancher(fenetre, &etat);

    let (f, e) = (fenetre.as_weak(), etat.clone());
    fenetre.on_changer_langue(move |rang| {
        let Some(langue) = usize::try_from(rang)
            .ok()
            .and_then(|r| Langue::TOUTES.get(r).copied())
        else {
            return SharedString::new();
        };
        let mut etat = e.borrow_mut();
        let Some(utilisateur) = etat.utilisateur.clone() else {
            return phrase("aucune session ouverte");
        };
        let resultat = comptes::changer_langue(&etat.conn, &utilisateur, langue);
        if resultat.is_ok()
            && let Some(u) = etat.utilisateur.as_mut()
        {
            u.langue = langue;
        }
        // Dans tous les cas, l'écran reprend la langue enregistrée du compte.
        if let Some(f) = f.upgrade() {
            afficher_session(&f, &etat);
        }
        match resultat {
            Ok(()) => SharedString::new(),
            Err(erreur) => phrase(erreur),
        }
    });

    let e = etat;
    fenetre.on_changer_mot_de_passe(move |ancien, nouveau, confirmation| {
        if nouveau != confirmation {
            return phrase("les deux nouveaux mots de passe ne correspondent pas");
        }
        let etat = e.borrow();
        let Some(utilisateur) = &etat.utilisateur else {
            return phrase("aucune session ouverte");
        };
        match comptes::changer_mot_de_passe(&etat.conn, utilisateur.id, &ancien, &nouveau) {
            Ok(()) => SharedString::new(),
            Err(erreur) => phrase(erreur),
        }
    });
}

fn main() -> Result<(), slint::PlatformError> {
    let fenetre = FenetrePrincipale::new()?;
    // Interface en français au démarrage, quelle que soit la langue de Windows, jusqu'à
    // la connexion d'un compte (voir traduction::choisir_demarrage).
    appliquer_langue(&fenetre, traduction::choisir_demarrage());
    fenetre.set_version_logiciel(env!("CARGO_PKG_VERSION").into());
    let chemin = chemin_base();

    let demarrage = kadenz_stockage::ouvrir(&chemin).and_then(|conn| {
        let ecran = comptes::etat_installation(&conn)?;
        Ok((conn, ecran))
    });

    match demarrage {
        Ok((conn, ecran)) => {
            let etat = Rc::new(RefCell::new(Etat::nouveau(conn)));
            afficher_session(&fenetre, &etat.borrow());
            fenetre.set_ecran(match ecran {
                EtatInstallation::AssistantRequis => Ecran::Assistant,
                EtatInstallation::ConnexionRequise => Ecran::Connexion,
            });
            brancher(&fenetre, etat);
        }
        Err(erreur) => {
            fenetre.set_message_demarrage(
                traduction::trf(
                    "La base de données ({}) n'a pas pu être ouverte : {}",
                    &[&chemin.display(), &traduction::tr(&erreur.to_string())],
                )
                .into(),
            );
            fenetre.set_ecran(Ecran::DemarrageImpossible);
        }
    }

    fenetre.run()
}
