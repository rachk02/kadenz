// En version optimisée, pas de console Windows derrière la fenêtre.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod abonnement;
mod administration;
mod excel;
mod factures;
mod graphiques;
mod import_ecran;
mod impression;
mod instance;
mod licence;
mod maj;
mod paiements;
mod pdf;
mod pilotage;
mod referentiels;
mod relie;
mod sauvegardes;
mod serveur;
mod traduction;
mod utilisateurs;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use kadenz_domaine::langue::Langue;
use kadenz_domaine::lettres::Orthographe;
use kadenz_domaine::module::Module;
use kadenz_domaine::role::Droit;
use kadenz_domaine::saisie;
use kadenz_stockage::Connexion;
use kadenz_stockage::comptes::{
    self, EtatInstallation, Navigation, NouvelUtilisateur, Utilisateur,
};
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
/// Dossier des données : celui de la base V1, où se trouve aussi la copie du
/// serveur d'un poste relié (décision 45).
pub(crate) fn dossier_donnees() -> PathBuf {
    let chemin = chemin_base();
    chemin
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn chemin_base() -> PathBuf {
    if let Some(chemin) = std::env::var_os("KADENZ_BASE") {
        return PathBuf::from(chemin);
    }
    let racine = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    racine.join("Kadenz").join("kadenz.sqlite")
}

/// Détail technique d'une erreur, gardé hors de l'écran : ajouté au fichier
/// erreurs.log, à côté de la base. Au-delà d'un mégaoctet, le fichier repart de zéro.
pub(crate) fn consigner_erreur(detail: &str) {
    use std::io::Write;
    if cfg!(test) {
        return;
    }
    let chemin = chemin_base().with_file_name("erreurs.log");
    let trop_gros = std::fs::metadata(&chemin).is_ok_and(|m| m.len() > 1_048_576);
    let mut options = std::fs::OpenOptions::new();
    if trop_gros {
        options.write(true).create(true).truncate(true);
    } else {
        options.append(true).create(true);
    }
    if let Ok(mut fichier) = options.open(&chemin) {
        let horodatage = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(fichier, "[{horodatage}] {detail}");
    }
}

/// Met la première lettre d'un message en majuscule et termine par un point.
/// Le message est d'abord traduit dans la langue choisie (traduction::tr) ; une
/// erreur du stockage passe par referentiels::texte_erreur (jamais de requête SQL
/// à l'écran).
pub(crate) fn phrase(message: impl std::fmt::Display + 'static) -> SharedString {
    let texte = match (&message as &dyn std::any::Any).downcast_ref::<kadenz_stockage::Erreur>() {
        Some(erreur) => referentiels::texte_erreur_ref(erreur),
        None => traduction::tr(&message.to_string()),
    };
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
    /// Document à ouvrir dès l'affichage de la page de son type (document créé
    /// ou lien suivi depuis la fiche d'un autre type).
    document_a_ouvrir: Option<i64>,
    /// Identifiant de ce poste pour la licence (vide s'il n'a pas pu être lu).
    poste: String,
    /// Poste relié (phase 2) : synchroniseur et état de la connexion.
    relie: Option<relie::Relie>,
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
            document_a_ouvrir: None,
            poste: licence::poste_local(),
            relie: None,
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
    // Lecture seule (décision 40) : plus d'ajout ni de modification des
    // référentiels. Gérer les paramètres et les utilisateurs reste vrai : ces
    // droits ouvrent aussi des pages, dont « Abonnement », seule sortie de la
    // lecture seule ; leurs boutons d'enregistrement suivent `Ecriture`.
    let ecrire = relie::blocage(etat).is_none();
    fenetre.global::<Ecriture>().set_permise(ecrire);
    let d = fenetre.global::<Droits>();
    d.set_ajouter_referentiel(ecrire && possede(Droit::AjouterReferentiel));
    d.set_modifier_referentiel(ecrire && possede(Droit::ModifierReferentiel));
    d.set_gerer_parametres(possede(Droit::GererParametres));
    d.set_gerer_utilisateurs(possede(Droit::GererUtilisateurs));

    // Les libellés et les icônes du menu sont dans navigation.slint.
    // Les pages des modules que la licence n'active pas sont retirées du menu.
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
            Page::Reglements,
            Page::SuiviFournisseurs,
            Page::Relances,
            Page::Tiers,
            Page::Produits,
        ]);
        menu.retain(|p| licence::page_active(&etat.conn, *p));
    }
    // Les Paramètres sont ouverts à tous : Mon compte, la navigation et À propos
    // en font partie.
    menu.push(Page::Parametres);

    let groupe = |pages: &[Page]| -> ModelRc<Page> {
        let retenues: Vec<Page> = pages.iter().copied().filter(|p| menu.contains(p)).collect();
        ModelRc::new(VecModel::from(retenues))
    };
    let m = fenetre.global::<Menu>();
    m.set_tableau_de_bord(menu.contains(&Page::TableauDeBord));
    m.set_documents(groupe(&[
        Page::Devis,
        Page::Proforma,
        Page::BonsCommande,
        Page::Livraisons,
        Page::Factures,
        Page::Avoirs,
    ]));
    m.set_paiements(groupe(&[
        Page::Reglements,
        Page::SuiviFournisseurs,
        Page::Relances,
    ]));
    m.set_referentiels(groupe(&[Page::Tiers, Page::Produits]));

    // La page en cours reste affichée si le menu la contient encore ; sinon les
    // Factures (socle), ou à défaut la première page du menu.
    let page = fenetre.get_page();
    fenetre.set_page(if menu.contains(&page) {
        page
    } else if menu.contains(&Page::Factures) {
        Page::Factures
    } else {
        menu[0]
    });

    // Rubriques des Paramètres, selon les droits et la licence.
    let gere = possede(Droit::GererParametres);
    let mut rubriques = vec![Rubrique::MonCompte, Rubrique::Navigation];
    if gere {
        rubriques.extend([Rubrique::Entreprise, Rubrique::Documents]);
        if licence::actif(&etat.conn, Module::Paiements) {
            rubriques.push(Rubrique::Paiements);
        }
        rubriques.extend([Rubrique::Textes, Rubrique::Impression]);
    }
    if possede(Droit::GererUtilisateurs) {
        rubriques.push(Rubrique::Utilisateurs);
    }
    // Poste relié (phase 2) : la rubrique « Abonnement » remplace « Licence ».
    let relie = abonnement::afficher(fenetre, &etat.conn, etat.utilisateur.is_some(), gere);
    // Décision 47 : un poste inscrit sur un serveur ne fait pas de sauvegarde,
    // les données sont sauvegardées sur le serveur.
    let inscrit = relie || serveur::inscrit(&etat.conn);
    if possede(Droit::SauvegarderBase) && !inscrit {
        rubriques.push(Rubrique::Sauvegardes);
    }
    if gere {
        rubriques.push(if relie {
            Rubrique::Abonnement
        } else {
            Rubrique::Licence
        });
        rubriques.push(Rubrique::MisesAJour);
        // Phase 2 : inscription de ce poste sur le serveur de l'entreprise.
        rubriques.push(Rubrique::Serveur);
        serveur::afficher(fenetre, &etat.conn);
    }
    rubriques.push(Rubrique::APropos);
    let r = fenetre.global::<ReglagesApi>();
    if !rubriques.contains(&r.get_rubrique()) {
        r.set_rubrique(Rubrique::MonCompte);
    }
    r.set_rubriques(ModelRc::new(VecModel::from(rubriques)));
}

/// Le compte connecté gère-t-il les paramètres ? (Invite de mise à jour, J10.)
fn gere_parametres(etat: &Etat) -> bool {
    etat.utilisateur
        .as_ref()
        .is_some_and(|u| droits::possede(&etat.conn, u, Droit::GererParametres).unwrap_or(false))
}

/// Préférences d'affichage du compte : navigation et animations.
fn appliquer_affichage(fenetre: &FenetrePrincipale, u: &Utilisateur) {
    let a = fenetre.global::<Affichage>();
    a.set_navigation(match u.navigation {
        Navigation::Bandeau => 0,
        Navigation::Barre => 1,
    });
    a.set_animations(u.animations);
}

/// Réglages d'affichage propres à la langue retenue (code de traduction::choisir).
fn appliquer_langue(fenetre: &FenetrePrincipale, code: &str) {
    fenetre.global::<Theme>().set_chinois(code == "zh");
}

/// Ouvre la session d'un compte reconnu : écrans du compte, puis invite de
/// mise à jour pour qui gère les paramètres.
pub(crate) fn ouvrir_session(f: &FenetrePrincipale, etat: &mut Etat, utilisateur: Utilisateur) {
    etat.utilisateur = Some(utilisateur);
    afficher_session(f, etat);
    maj::verifier_apres_connexion(f, gere_parametres(etat));
}

/// Compte, droits ou entreprise changés sur un autre poste (poste relié) :
/// la session relit son compte dans la copie, puis ses droits et son menu.
pub(crate) fn relire_session(f: &FenetrePrincipale, etat: &EtatPartage) {
    let mut e = etat.borrow_mut();
    if let Some(id) = e.utilisateur.as_ref().map(|u| u.id)
        && let Ok(Some(u)) = comptes::lire(&e.conn, id)
    {
        e.utilisateur = Some(u);
    }
    afficher_session(f, &e);
}

fn afficher_session(fenetre: &FenetrePrincipale, etat: &Etat) {
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
        appliquer_affichage(fenetre, u);
        appliquer_droits(fenetre, etat);
        fenetre.set_ecran(Ecran::Application);
    }
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

/// Fin de session (déconnexion, ou restauration d'une sauvegarde) : état de la
/// session vidé, langue de démarrage, retour à l'écran de connexion.
pub(crate) fn fermer_session(f: &FenetrePrincipale, etat: &mut Etat) {
    etat.utilisateur = None;
    etat.tiers = Vue::default();
    etat.produits = Vue::default();
    etat.comptes = Vue::default();
    etat.factures = factures::Registre::default();
    etat.facture = None;
    etat.type_document = TypeDocument::Facture;
    etat.document_a_ouvrir = None;
    appliquer_langue(f, traduction::choisir_demarrage());
    f.set_page(Page::TableauDeBord);
    appliquer_droits(f, etat);
    f.set_utilisateur_nom(SharedString::new());
    f.set_utilisateur_identifiant(SharedString::new());
    f.set_utilisateur_role(SharedString::new());
    afficher_session(f, etat);
    f.set_ecran(Ecran::Connexion);
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
                if let Some(f) = f.upgrade() {
                    licence::enregistrer_cle_assistant(
                        &f,
                        &etat.conn,
                        &admin,
                        &donnees.cle_licence,
                        &etat.poste,
                    );
                }
                etat.utilisateur = Some(admin);
                if let Some(f) = f.upgrade() {
                    afficher_session(&f, &etat);
                    maj::verifier_apres_connexion(&f, gere_parametres(&etat));
                }
                SharedString::new()
            }
            Err(erreur) => phrase(erreur),
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    fenetre.on_connecter(move |identifiant, mot_de_passe| {
        // Poste relié : connexion par le serveur (ou locale, décision 48).
        if let Some(f) = f.upgrade()
            && let Some(message) = relie::connecter(&f, &e, &identifiant, &mot_de_passe)
        {
            return message;
        }
        let mut etat = e.borrow_mut();
        match comptes::connecter(&etat.conn, &identifiant, &mot_de_passe) {
            Ok(utilisateur) => {
                etat.utilisateur = Some(utilisateur);
                if let Some(f) = f.upgrade() {
                    afficher_session(&f, &etat);
                    maj::verifier_apres_connexion(&f, gere_parametres(&etat));
                }
                SharedString::new()
            }
            Err(erreur) => phrase(erreur),
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    fenetre.on_deconnecter(move || {
        if let Some(f) = f.upgrade() {
            let mut etat = e.borrow_mut();
            relie::deconnecter(&mut etat);
            fermer_session(&f, &mut etat);
        }
    });

    referentiels::brancher(fenetre, &etat);
    administration::brancher(fenetre, &etat);
    utilisateurs::brancher(fenetre, &etat);
    import_ecran::brancher(fenetre, &etat);
    factures::brancher(fenetre, &etat);
    paiements::brancher(fenetre, &etat);
    pilotage::brancher(fenetre, &etat);
    impression::brancher(fenetre, &etat);
    licence::brancher(fenetre, &etat);
    sauvegardes::brancher(fenetre, &etat);
    serveur::brancher(fenetre, &etat);
    relie::brancher(fenetre, &etat);
    maj::brancher(fenetre);

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
        let resultat = match relie::ecrire(
            &etat,
            kadenz_protocole::Action::ChangerLangue {
                langue: langue.code().into(),
            },
        ) {
            Some(r) => r.map(|_| ()).map_err(phrase),
            None => comptes::changer_langue(&etat.conn, &utilisateur, langue).map_err(phrase),
        };
        if resultat.is_ok()
            && let Some(u) = etat.utilisateur.as_mut()
        {
            u.langue = langue;
        }
        // Dans tous les cas, l'écran reprend la langue enregistrée du compte.
        if let Some(f) = f.upgrade() {
            afficher_session(&f, &etat);
        }
        resultat.err().unwrap_or_default()
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    fenetre
        .global::<ReglagesApi>()
        .on_changer_affichage(move |navigation, animations| {
            let navigation = if navigation == 1 {
                Navigation::Barre
            } else {
                Navigation::Bandeau
            };
            let mut etat = e.borrow_mut();
            let Some(utilisateur) = etat.utilisateur.clone() else {
                return phrase("aucune session ouverte");
            };
            let resultat = match relie::ecrire(
                &etat,
                kadenz_protocole::Action::ChangerAffichage {
                    navigation: navigation.code().into(),
                    animations,
                },
            ) {
                Some(r) => r.map(|_| ()).map_err(phrase),
                None => {
                    comptes::changer_affichage(&etat.conn, &utilisateur, navigation, animations)
                        .map_err(phrase)
                }
            };
            if resultat.is_ok()
                && let Some(u) = etat.utilisateur.as_mut()
            {
                u.navigation = navigation;
                u.animations = animations;
            }
            // Dans tous les cas, l'écran reprend les préférences enregistrées du compte.
            if let (Some(f), Some(u)) = (f.upgrade(), etat.utilisateur.as_ref()) {
                appliquer_affichage(&f, u);
            }
            resultat.err().unwrap_or_default()
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
        let action = kadenz_protocole::Action::ChangerMotDePasse {
            ancien: ancien.to_string(),
            nouveau: nouveau.to_string(),
        };
        match relie::ecrire(&etat, action) {
            Some(Ok(_)) => SharedString::new(),
            Some(Err(texte)) => phrase(texte),
            None => {
                match comptes::changer_mot_de_passe(&etat.conn, utilisateur.id, &ancien, &nouveau) {
                    Ok(()) => SharedString::new(),
                    Err(erreur) => phrase(erreur),
                }
            }
        }
    });
}

fn main() -> Result<(), slint::PlatformError> {
    // Base déjà ouverte par une autre instance : sa fenêtre revient au premier plan.
    if !instance::prendre(&chemin_base()) {
        return Ok(());
    }
    let fenetre = FenetrePrincipale::new()?;
    // Interface en français au démarrage, quelle que soit la langue de Windows, jusqu'à
    // la connexion d'un compte (voir traduction::choisir_demarrage).
    appliquer_langue(&fenetre, traduction::choisir_demarrage());
    fenetre.set_version_logiciel(env!("CARGO_PKG_VERSION").into());
    let chemin = chemin_base();

    // Session ouverte, gardée pour la sauvegarde automatique à la fermeture.
    let session: Rc<RefCell<Option<EtatPartage>>> = Rc::default();
    let demarrer = {
        let session = session.clone();
        move |fenetre: &FenetrePrincipale, conn: Connexion| -> Result<(), kadenz_stockage::Erreur> {
            // Poste relié : jamais l'assistant de création d'entreprise, même
            // sur une copie neuve sans comptes.
            let inscrit = serveur::inscrit(&conn);
            let ecran = if inscrit {
                EtatInstallation::ConnexionRequise
            } else {
                comptes::etat_installation(&conn)?
            };
            let etat = Rc::new(RefCell::new(Etat::nouveau(conn)));
            {
                // Licence d'abord : le menu de la session en dépend.
                let e = etat.borrow();
                licence::ouvrir(fenetre, &e.conn, &e.poste);
                if inscrit {
                    relie::appliquer_modules(fenetre, &e.conn);
                }
            }
            afficher_session(fenetre, &etat.borrow());
            fenetre.set_ecran(match ecran {
                // Premier lancement : présentation, puis assistant.
                EtatInstallation::AssistantRequis => Ecran::Presentation,
                EtatInstallation::ConnexionRequise => Ecran::Connexion,
            });
            brancher(fenetre, etat.clone());
            if let Some(copie) = inscrit
                .then(|| kadenz_stockage::poste_relie::copie(&dossier_donnees()))
                .flatten()
            {
                relie::demarrer(fenetre, &etat, copie);
            }
            *session.borrow_mut() = Some(etat);
            Ok(())
        }
    };

    // Poste relié : la copie du serveur ; la base V1 reste intacte (décision 45).
    let demarrage = match kadenz_stockage::poste_relie::copie(&dossier_donnees()) {
        Some(copie) => kadenz_stockage::poste_relie::ouvrir_copie(&copie)
            .and_then(|conn| demarrer(&fenetre, conn)),
        None => kadenz_stockage::ouvrir(&chemin).and_then(|conn| demarrer(&fenetre, conn)),
    };
    if let Err(erreur) = demarrage {
        // L'emplacement des données ne s'affiche pas : il va dans erreurs.log.
        consigner_erreur(&format!(
            "Ouverture de la base {} impossible : {erreur}",
            chemin.display()
        ));
        fenetre.set_message_demarrage(sauvegardes::message_demarrage(&erreur).into());
        fenetre.set_ecran(Ecran::DemarrageImpossible);
        sauvegardes::brancher_demarrage(&fenetre, demarrer);
    }

    let resultat = fenetre.run();
    // Fenêtre fermée : sauvegarde automatique de la base ouverte.
    // Poste relié : synchroniseur arrêté, et pas de sauvegarde de la copie, les
    // données sont sauvegardées sur le serveur (décision 47).
    if let Some(etat) = session.borrow().as_ref() {
        let mut e = etat.borrow_mut();
        relie::arreter(&mut e);
        if !serveur::inscrit(&e.conn) {
            sauvegardes::sauvegarder_a_la_fermeture(&e);
        }
    }
    resultat
}
