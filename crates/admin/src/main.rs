//! Fenêtre d'administration du serveur Kadenz (phase 2, jalon F4 ; décision 30 ;
//! écrans, section 1). Elle tourne en administrateur sur la machine du serveur
//! (manifeste, build.rs) et ouvre la base directement, comme le service : chaque
//! écriture passe par les fonctions du stockage (`kadenz_serveur::admin`).

// En version optimisée, pas de console Windows derrière la fenêtre.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Même module de traduction que l'application, sur les mêmes fichiers .po ; ses
// vérifications parcourent aussi les textes de cette fenêtre.
#[allow(dead_code)]
#[path = "../../app/src/traduction.rs"]
mod traduction;

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use chrono::{DateTime, Local, NaiveDateTime, Utc};
use kadenz_domaine::lettres::Orthographe;
use kadenz_domaine::role::Role;
use kadenz_domaine::saisie;
use kadenz_protocole::inscription::presenter_secret;
use kadenz_serveur::admin::{self, EtatService};
use kadenz_stockage::Erreur;
use kadenz_stockage::comptes::{self, EtatInstallation, NouvelUtilisateur};
use kadenz_stockage::entreprise::{self, Entreprise};
use kadenz_stockage::installation::InstallationInitiale;
use kadenz_stockage::parametres::Parametres;
use kadenz_stockage::{abonnement, inscription, postes, synchro};
use rusqlite::Connection;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::traduction::{tr, trf};

slint::include_modules!();

/// Texte traduit d'une erreur du stockage.
fn texte_erreur(erreur: &Erreur) -> String {
    match erreur {
        Erreur::ValidationComposee { modele, valeurs } => {
            let valeurs: Vec<String> = valeurs.iter().map(|v| tr(v)).collect();
            let arguments: Vec<&dyn std::fmt::Display> = valeurs
                .iter()
                .map(|v| v as &dyn std::fmt::Display)
                .collect();
            trf(modele, &arguments)
        }
        autre => tr(&autre.to_string()),
    }
}

fn retour_ok(message: String) -> Retour {
    Retour {
        ok: true,
        message: message.into(),
    }
}

fn retour_erreur(message: String) -> Retour {
    Retour {
        ok: false,
        message: message.into(),
    }
}

/// Horodatage UTC du stockage (`…Z`) en heure locale, JJ/MM/AAAA HH:MM.
fn heure_locale_utc(texte: &str) -> String {
    DateTime::parse_from_rfc3339(texte)
        .map(|d| d.with_timezone(&Local).format("%d/%m/%Y %H:%M").to_string())
        .unwrap_or_else(|_| texte.to_string())
}

/// Horodatage local du stockage (`AAAA-MM-JJTHH:MM:SS`), JJ/MM/AAAA HH:MM.
fn heure_locale(texte: &str) -> String {
    NaiveDateTime::parse_from_str(texte, "%Y-%m-%dT%H:%M:%S")
        .map(|d| d.format("%d/%m/%Y %H:%M").to_string())
        .unwrap_or_else(|_| texte.to_string())
}

/// Date `AAAA-MM-JJ` en JJ/MM/AAAA.
fn date_jour(texte: &str) -> String {
    chrono::NaiveDate::parse_from_str(texte, "%Y-%m-%d")
        .map(|d| d.format("%d/%m/%Y").to_string())
        .unwrap_or_else(|_| texte.to_string())
}

/// Empreinte en groupes de quatre caractères, plus facile à comparer à l'oeil.
fn empreinte_lisible(empreinte: &str) -> String {
    empreinte
        .to_uppercase()
        .as_bytes()
        .chunks(4)
        .map(|g| String::from_utf8_lossy(g).into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

fn texte_etat_service(etat: Result<EtatService, String>) -> (String, bool) {
    match etat {
        Ok(EtatService::Demarre) => (tr("Démarré"), true),
        Ok(EtatService::Demarrage) => (tr("Démarrage en cours"), false),
        Ok(EtatService::Arret) => (tr("Arrêt en cours"), false),
        Ok(EtatService::Arrete) => (tr("Arrêté"), false),
        Ok(EtatService::NonInstalle) => (tr("Service non installé"), false),
        Ok(EtatService::Autre) => (tr("État inconnu"), false),
        Err(e) => (e, false),
    }
}

/// Dossier des données du serveur. En développement, `KADENZ_SERVEUR_DOSSIER`
/// le remplace, pour essayer la fenêtre sur un dossier d'essai ; une version
/// publiée ne lit jamais cette variable.
fn dossier() -> PathBuf {
    #[cfg(debug_assertions)]
    if let Some(d) = std::env::var_os("KADENZ_SERVEUR_DOSSIER") {
        return PathBuf::from(d);
    }
    kadenz_serveur::dossier_par_defaut()
}

/// Ouvre la base du serveur, sans jamais la créer : c'est le service qui crée
/// le dossier, ses droits, le certificat et la base à son premier démarrage.
fn ouvrir(dossier: &Path) -> Result<Connection, String> {
    if !dossier.join(kadenz_serveur::NOM_BASE).is_file() {
        return Err(trf(
            "Aucune base du serveur dans {} : démarrez le service KadenzServeur une première fois, puis rouvrez cette fenêtre.",
            &[&dossier.display()],
        ));
    }
    admin::ouvrir_base(dossier).map_err(|e| {
        trf(
            "La base du serveur ne s'ouvre pas : {}. Cette fenêtre doit être lancée en administrateur, sur la machine du serveur.",
            &[&e],
        )
    })
}

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

/// Recopie l'état du serveur dans le tableau.
fn actualiser(f: &FenetreAdmin, conn: &Connection, dossier: &Path) {
    let api = f.global::<AdminApi>();
    let (etat, demarre) = texte_etat_service(admin::etat_service());
    api.set_etat_service(etat.into());
    api.set_service_demarre(demarre);
    api.set_derniere_erreur(admin::derniere_erreur(dossier).unwrap_or_default().into());
    api.set_empreinte(
        kadenz_serveur::empreinte_du_dossier(dossier)
            .map(|e| empreinte_lisible(&e))
            .unwrap_or_else(|e| e)
            .into(),
    );
    if let Ok(Some(e)) = entreprise::lire(conn) {
        api.set_nom_entreprise(e.nom.into());
    }
    let inscrits = postes::liste(conn).unwrap_or_default();
    let permis = abonnement::etat(conn, abonnement::horloge())
        .map(|a| abonnement::postes_permis(&a))
        .unwrap_or(abonnement::POSTES_SANS_ABONNEMENT_ACTIF);
    api.set_postes_permis(
        trf(
            "{} inscrits sur {} permis par l'abonnement",
            &[&inscrits.len(), &permis],
        )
        .into(),
    );
    api.set_postes(ModelRc::new(VecModel::from(
        inscrits
            .into_iter()
            .map(|p| PosteUi {
                empreinte: p.empreinte.into(),
                nom: p.nom.into(),
                inscrit_le: heure_locale(&p.inscrit_le).into(),
            })
            .collect::<Vec<_>>(),
    )));
    api.set_codes(ModelRc::new(VecModel::from(
        inscription::codes_en_cours(conn, Utc::now())
            .unwrap_or_default()
            .into_iter()
            .map(|c| CodeUi {
                id: c.id as i32,
                nom: c.nom_poste.into(),
                expire_le: heure_locale_utc(&c.expire_le).into(),
            })
            .collect::<Vec<_>>(),
    )));
}

fn brancher_tableau(f: &FenetreAdmin, conn: Rc<RefCell<Connection>>, dossier: Rc<PathBuf>) {
    let api = f.global::<AdminApi>();
    api.set_dossier(dossier.display().to_string().into());
    actualiser(f, &conn.borrow(), &dossier);

    let (fa, c, d) = (f.as_weak(), conn.clone(), dossier.clone());
    api.on_actualiser(move || {
        if let Some(f) = fa.upgrade() {
            actualiser(&f, &c.borrow(), &d);
        }
    });

    let (fa, c, d) = (f.as_weak(), conn.clone(), dossier.clone());
    api.on_creer_code(move |nom| {
        let Some(f) = fa.upgrade() else {
            return retour_erreur(String::new());
        };
        let conn = c.borrow();
        let code = match inscription::creer_code(&conn, &nom, Utc::now()) {
            Ok(code) => code,
            Err(e) => return retour_erreur(texte_erreur(&e)),
        };
        let complet = match admin::code_complet(&conn, &d, &code) {
            Ok(complet) => complet,
            Err(e) => return retour_erreur(e),
        };
        let api = f.global::<AdminApi>();
        api.set_nouveau_nom(nom.trim().into());
        api.set_nouveau_secret(presenter_secret(&code.secret).into());
        api.set_nouveau_complet(complet.into());
        api.set_nouveau_expire(heure_locale_utc(&code.expire_le).into());
        actualiser(&f, &conn, &d);
        retour_ok(String::new())
    });

    let (fa, c, d) = (f.as_weak(), conn.clone(), dossier.clone());
    api.on_annuler_code(move |id| {
        let Some(f) = fa.upgrade() else {
            return retour_erreur(String::new());
        };
        let conn = c.borrow();
        let retour = match inscription::annuler_code(&conn, i64::from(id), Utc::now()) {
            Ok(()) => retour_ok(tr("Code annulé.")),
            Err(e) => retour_erreur(texte_erreur(&e)),
        };
        actualiser(&f, &conn, &d);
        retour
    });

    let (fa, c, d) = (f.as_weak(), conn, dossier);
    api.on_retirer_poste(move |empreinte| {
        let Some(f) = fa.upgrade() else {
            return retour_erreur(String::new());
        };
        let conn = c.borrow();
        let retour = match postes::retirer(&conn, &empreinte) {
            Ok(()) => retour_ok(tr("Poste retiré.")),
            Err(e) => retour_erreur(texte_erreur(&e)),
        };
        actualiser(&f, &conn, &d);
        retour
    });
}

fn brancher_assistant(f: &FenetreAdmin, conn: Rc<RefCell<Connection>>, dossier: Rc<PathBuf>) {
    let defaut = Parametres::default();
    f.set_defaut_taux_tva(saisie::liste_pourcentages_en_texte(&defaut.taux_tva).into());
    f.set_defaut_taux_remise(saisie::liste_pourcentages_en_texte(&defaut.taux_remise).into());
    f.set_defaut_taux_retenue(saisie::liste_pourcentages_en_texte(&defaut.taux_retenue).into());
    f.set_defaut_modes_paiement(defaut.modes_paiement.join(" ; ").into());

    // L'étape de la licence montre l'identifiant de l'entreprise, auquel la
    // licence d'abonnement est liée (champ du poste dans l'application).
    let entreprise = synchro::identifiant_entreprise(&conn.borrow()).unwrap_or_default();
    let licence = f.global::<LicenceApi>();
    licence.set_poste(entreprise.clone().into());
    licence.on_verifier(move |cle| {
        match kadenz_licence::entreprise::verifier(
            &cle,
            &entreprise,
            &kadenz_licence::entreprise::cles_portail(),
        ) {
            Ok(l) => VerificationUi {
                ok: true,
                message: trf(
                    "{}, {} postes, jusqu'au {}",
                    &[&l.client, &l.postes, &date_jour(&l.echeance)],
                )
                .into(),
            },
            Err(m) => VerificationUi {
                ok: false,
                message: trf(
                    m.modele,
                    &m.valeurs
                        .iter()
                        .map(|v| v as &dyn std::fmt::Display)
                        .collect::<Vec<_>>(),
                )
                .into(),
            },
        }
    });

    let fa = f.as_weak();
    f.on_terminer(move |d| {
        let Some(f) = fa.upgrade() else {
            return SharedString::new();
        };
        let init = match installation_depuis_assistant(&d) {
            Ok(init) => init,
            Err(message) => return message.into(),
        };
        // Licence facultative : sans elle, lecture seule et un seul poste
        // permis, le temps de l'installer depuis ce poste (décisions 40 et 41).
        let cle = d.cle_licence.trim().to_string();
        let licence = (!cle.is_empty()).then_some(cle.as_str());
        let resultat = admin::installer(&mut conn.borrow_mut(), &init, licence);
        match resultat {
            Ok(_) => {
                brancher_tableau(&f, conn.clone(), dossier.clone());
                f.set_ecran(EcranAdmin::Tableau);
                SharedString::new()
            }
            Err(e) => texte_erreur(&e).into(),
        }
    });
}

fn main() -> Result<(), slint::PlatformError> {
    traduction::choisir_demarrage();
    let fenetre = FenetreAdmin::new()?;
    let dossier = Rc::new(dossier());
    match ouvrir(&dossier) {
        Err(message) => {
            fenetre.set_message_erreur(message.into());
            fenetre.set_ecran(EcranAdmin::Erreur);
        }
        Ok(conn) => {
            let installation = comptes::etat_installation(&conn);
            let conn = Rc::new(RefCell::new(conn));
            match installation {
                Ok(EtatInstallation::AssistantRequis) => {
                    brancher_assistant(&fenetre, conn, dossier);
                    fenetre.set_ecran(EcranAdmin::Assistant);
                }
                Ok(EtatInstallation::ConnexionRequise) => {
                    brancher_tableau(&fenetre, conn, dossier);
                    fenetre.set_ecran(EcranAdmin::Tableau);
                }
                Err(e) => {
                    fenetre.set_message_erreur(texte_erreur(&e).into());
                    fenetre.set_ecran(EcranAdmin::Erreur);
                }
            }
        }
    }

    // L'état du service change sans la fenêtre : relu toutes les cinq secondes.
    let minuterie = slint::Timer::default();
    let fa = fenetre.as_weak();
    minuterie.start(
        slint::TimerMode::Repeated,
        Duration::from_secs(5),
        move || {
            if let Some(f) = fa.upgrade()
                && f.get_ecran() == EcranAdmin::Tableau
            {
                f.global::<AdminApi>().invoke_actualiser();
            }
        },
    );

    fenetre.run()
}
