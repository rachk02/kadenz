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

// Activation et champ des codes de licence courts, partagés avec l'application.
#[path = "../../app/src/activation_ui.rs"]
mod activation_ui;
#[path = "../../app/src/code_saisie.rs"]
mod code_saisie;

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Local, NaiveDateTime, NaiveTime, Utc};
use kadenz_domaine::lettres::Orthographe;
use kadenz_domaine::message::MessageCompose;
use kadenz_domaine::role::Role;
use kadenz_domaine::saisie;
use kadenz_maj::{MiseAJour, Produit, Recherche};
use kadenz_protocole::inscription::presenter_secret;
use kadenz_protocole::sauvegarde::AvertissementSauvegarde;
use kadenz_serveur::admin::{self, BilanRestauration, EtatService, Restauration};
use kadenz_serveur::sauvegardes;
use kadenz_stockage::Erreur;
use kadenz_stockage::comptes::{self, EtatInstallation, NouvelUtilisateur};
use kadenz_stockage::entreprise::{self, Entreprise};
use kadenz_stockage::installation::InstallationInitiale;
use kadenz_stockage::parametres::Parametres;
use kadenz_stockage::sauvegarde::Genre;
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
        attente: false,
    }
}

fn retour_erreur(message: String) -> Retour {
    Retour {
        ok: false,
        message: message.into(),
        attente: false,
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

/// Vrai si un serveur répond sur ce port de la machine (connexion TCP seule,
/// sans rien envoyer) : le serveur peut tourner hors du service Windows, en
/// console (essais, dépannage).
fn serveur_en_marche(port: u16) -> bool {
    std::net::TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_millis(300),
    )
    .is_ok()
}

fn texte_etat_service(etat: Result<EtatService, String>, port: u16) -> (String, bool) {
    match etat {
        Ok(EtatService::NonInstalle | EtatService::Arrete) if serveur_en_marche(port) => (
            tr("Serveur en marche hors du service Windows (console)"),
            true,
        ),
        Ok(EtatService::Demarre) => (tr("Démarré"), true),
        Ok(EtatService::Demarrage) => (tr("Démarrage en cours"), false),
        Ok(EtatService::Arret) => (tr("Arrêt en cours"), false),
        Ok(EtatService::Arrete) => (tr("Arrêté"), false),
        Ok(EtatService::NonInstalle) => (tr("Service non installé"), false),
        Ok(EtatService::Autre) => (tr("État inconnu"), false),
        Err(e) => (e, false),
    }
}

/// Dossier des données du serveur. En développement, l'option `--dossier D`
/// ou `KADENZ_SERVEUR_DOSSIER` le remplace, pour essayer la fenêtre sur un
/// dossier d'essai (un processus élevé ne reçoit pas toujours les variables
/// d'environnement de son parent) ; une version publiée ne lit ni l'une ni
/// l'autre.
fn dossier() -> PathBuf {
    dossier_essai().unwrap_or_else(kadenz_serveur::dossier_par_defaut)
}

/// Dossier d'essai donné en développement. La fenêtre ne pilote alors
/// jamais le service du serveur : une restauration d'essai se fait serveur
/// d'essai arrêté, puis relancé à la main.
fn dossier_essai() -> Option<PathBuf> {
    #[cfg(debug_assertions)]
    {
        let arguments: Vec<String> = std::env::args().collect();
        if let Some(d) = arguments
            .iter()
            .position(|a| a == "--dossier")
            .and_then(|i| arguments.get(i + 1))
        {
            return Some(PathBuf::from(d));
        }
        if let Some(d) = std::env::var_os("KADENZ_SERVEUR_DOSSIER") {
            return Some(PathBuf::from(d));
        }
    }
    None
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

/// Heure de la sauvegarde quotidienne, saisie HH:MM (12:00, ou 12h00).
fn heure_sauvegarde(texte: &str) -> Result<NaiveTime, String> {
    NaiveTime::parse_from_str(&texte.trim().replace(['h', 'H'], ":"), "%H:%M").map_err(|_| {
        tr("heure des sauvegardes invalide : la saisir sous la forme HH:MM, par exemple 12:00")
    })
}

fn genre_sauvegarde(genre: Genre) -> String {
    match genre {
        Genre::Automatique => tr("Automatique"),
        Genre::Manuelle => tr("Manuelle"),
        Genre::AvantRestauration => tr("Avant restauration"),
        Genre::Arret => tr("À l'arrêt du service"),
    }
}

/// Bandeau permanent des sauvegardes (décision 56) ; vide si tout va bien.
fn texte_avertissement(avertissement: Option<AvertissementSauvegarde>) -> String {
    match avertissement {
        None => String::new(),
        Some(AvertissementSauvegarde::SansMotDePasse) => tr(
            "Aucune sauvegarde n'est faite : leur mot de passe n'est pas choisi. Choisissez-le dans la section Sauvegardes.",
        ),
        Some(AvertissementSauvegarde::MemeDisque) => tr(
            "Les sauvegardes sont sur le même disque que la base du serveur : une panne de ce disque les perdrait toutes. Choisissez un dossier sur un autre disque (disque externe ou partage réseau).",
        ),
    }
}

/// Dossier choisi dans la boîte de Windows ; `actuel` si le choix est abandonné.
fn choisir_dossier(f: &FenetreAdmin, actuel: &str) -> SharedString {
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

/// Recopie le réglage des sauvegardes dans les champs (dossier et heure),
/// à l'ouverture et après un enregistrement : jamais pendant une saisie.
fn montrer_reglage_sauvegardes(f: &FenetreAdmin, dossier: &Path) {
    let reglage = sauvegardes::lire_reglage(dossier);
    let api = f.global::<AdminApi>();
    api.set_sauvegarde_dossier(reglage.dossier.display().to_string().into());
    api.set_sauvegarde_heure(reglage.heure.format("%H:%M").to_string().into());
}

/// Relais d'accès à distance réglé (F8) : recopié dans les champs, vides
/// s'il n'y en a pas. Une erreur de lecture est montrée sous les champs.
fn montrer_relais(f: &FenetreAdmin, conn: &Connection) {
    let api = f.global::<AdminApi>();
    match admin::lire_relais(conn) {
        Ok(relais) => {
            api.set_relais_regle(relais.is_some());
            let (adresse, empreinte) = relais
                .map(|r| (r.adresse, empreinte_lisible(&r.empreinte)))
                .unwrap_or_default();
            api.set_relais_adresse(adresse.into());
            api.set_relais_empreinte(empreinte.into());
        }
        Err(e) => api.set_relais_retour(retour_erreur(tr(&e))),
    }
}

/// Délai de l'arrêt du service : il fait une sauvegarde en s'arrêtant.
const DELAI_ARRET: Duration = Duration::from_secs(300);
const DELAI_DEMARRAGE: Duration = Duration::from_secs(120);

/// Échec d'une restauration, selon l'étape.
enum EchecRestauration {
    /// Le service ne s'est pas arrêté : rien n'a été touché.
    Arret(String),
    /// La restauration a échoué ; la base d'origine est en place, sauf si le
    /// message commence par RESTAURATION_INTERROMPUE.
    Restauration(String),
}

/// Lignes du bilan d'une restauration réussie.
fn lignes_bilan(bilan: &BilanRestauration, entreprise: &str) -> Vec<SharedString> {
    let mut lignes = Vec::new();
    if let Some(avant) = &bilan.avant {
        lignes.push(trf("Base précédente sauvegardée : {}", &[&avant.display()]).into());
    }
    if let Some(cote) = &bilan.mise_de_cote {
        lignes.push(
            trf(
                "Base précédente mise de côté, non effacée : {}",
                &[&cote.display()],
            )
            .into(),
        );
    }
    if bilan.entreprise_changee {
        lignes.push(trf("Entreprise du serveur : {}", &[&entreprise]).into());
    }
    lignes.push(tr("Chaque poste refait sa copie de la base à sa prochaine connexion.").into());
    lignes
}

/// Restauration (décision 35) : vérification dans un fil, puis arrêt du
/// service, restauration et redémarrage dans un autre. La connexion de la
/// fenêtre est fermée pendant la restauration (Windows refuse de renommer un
/// fichier ouvert), puis rouverte.
fn brancher_restauration(f: &FenetreAdmin, conn: Rc<RefCell<Connection>>, dossier: Rc<PathBuf>) {
    let api = f.global::<AdminApi>();
    // Sauvegarde vérifiée, gardée jusqu'à la fin de la restauration : son
    // fichier de travail disparaît avec elle.
    let preparee: Arc<Mutex<Option<Restauration>>> = Arc::new(Mutex::new(None));

    let (fa, p, d) = (f.as_weak(), preparee.clone(), dossier.clone());
    api.on_restauration_choisir(move |chemin| {
        let Some(f) = fa.upgrade() else { return };
        let chemin = if chemin.is_empty() {
            let mut boite = rfd::FileDialog::new()
                .set_title(tr("Choisir une sauvegarde"))
                .add_filter(tr("Sauvegardes Kadenz"), &["kzs"])
                .set_parent(&f.window().window_handle());
            let dossier_sauvegardes = sauvegardes::lire_reglage(&d).dossier;
            if dossier_sauvegardes.is_dir() {
                boite = boite.set_directory(dossier_sauvegardes);
            }
            match boite.pick_file() {
                Some(c) => c.display().to_string(),
                None => return,
            }
        } else {
            chemin.to_string()
        };
        if let Ok(mut p) = p.lock() {
            *p = None;
        }
        let api = f.global::<AdminApi>();
        api.set_restauration_fichier(chemin.into());
        api.set_restauration_resume(SharedString::new());
        api.set_restauration_alerte(SharedString::new());
        api.set_restauration_bilan(ModelRc::default());
        api.set_restauration_retour(retour_ok(String::new()));
        api.set_restauration_etat(1);
    });

    let (fa, p) = (f.as_weak(), preparee.clone());
    api.on_restauration_abandonner(move || {
        let Some(f) = fa.upgrade() else { return };
        if let Ok(mut p) = p.lock() {
            *p = None;
        }
        let api = f.global::<AdminApi>();
        api.set_restauration_etat(0);
        api.set_restauration_fichier(SharedString::new());
        api.set_restauration_retour(retour_ok(String::new()));
    });

    let (fa, p, d) = (f.as_weak(), preparee.clone(), dossier.clone());
    api.on_restauration_verifier(move |mot_de_passe| {
        let Some(f) = fa.upgrade() else { return };
        let api = f.global::<AdminApi>();
        let chemin = PathBuf::from(api.get_restauration_fichier().as_str());
        api.set_restauration_occupee(true);
        api.set_restauration_etape(tr("Vérification de la sauvegarde...").into());
        api.set_restauration_retour(retour_ok(String::new()));
        let (fa, p, d) = (f.as_weak(), p.clone(), (*d).clone());
        std::thread::spawn(move || {
            let resultat = admin::preparer_restauration(&d, &chemin, &mot_de_passe);
            let _ = fa.upgrade_in_event_loop(move |f| {
                let api = f.global::<AdminApi>();
                api.set_restauration_occupee(false);
                match resultat {
                    Ok(r) => {
                        api.set_restauration_resume(
                            trf("Sauvegarde de l'entreprise « {} ».", &[&r.entreprise]).into(),
                        );
                        let alerte = match &r.entreprise_serveur {
                            Some(serveur) if !r.meme_entreprise && !serveur.is_empty() => trf(
                                "Cette sauvegarde est celle d'une autre entreprise : l'entreprise du serveur, « {} », sera remplacée par « {} ».",
                                &[serveur, &r.entreprise],
                            ),
                            _ => String::new(),
                        };
                        api.set_restauration_alerte(alerte.into());
                        if let Ok(mut p) = p.lock() {
                            *p = Some(r);
                        }
                        api.set_restauration_etat(2);
                    }
                    Err(e) => api.set_restauration_retour(retour_erreur(tr(&e))),
                }
            });
        });
    });

    // Fin de la restauration, sur le fil de l'interface : rouvre la base.
    let (fa, c, d) = (f.as_weak(), conn.clone(), dossier.clone());
    api.on_restauration_rouvrir(move || {
        let Some(f) = fa.upgrade() else { return };
        match ouvrir(&d) {
            Ok(nouvelle) => {
                *c.borrow_mut() = nouvelle;
                // L'empreinte et le code complet changent avec l'identité :
                // le dernier code affiché ne vaut plus.
                let api = f.global::<AdminApi>();
                api.set_nouveau_secret(SharedString::new());
                api.set_nouveau_complet(SharedString::new());
                actualiser(&f, &c.borrow(), &d);
            }
            Err(message) => f
                .global::<AdminApi>()
                .set_restauration_retour(retour_erreur(message)),
        }
    });

    let (fa, p, c, d) = (f.as_weak(), preparee, conn, dossier);
    api.on_restauration_lancer(move || {
        let Some(f) = fa.upgrade() else { return };
        let Some(r) = p.lock().ok().and_then(|mut p| p.take()) else {
            return;
        };
        let api = f.global::<AdminApi>();
        api.set_restauration_etat(3);
        api.set_restauration_alerte(SharedString::new());
        api.set_restauration_retour(retour_ok(String::new()));
        api.set_restauration_etape(
            tr("Arrêt du service ; il fait une dernière sauvegarde en s'arrêtant...").into(),
        );
        // La connexion de la fenêtre est fermée jusqu'à la fin.
        match Connection::open_in_memory() {
            Ok(vide) => drop(std::mem::replace(&mut *c.borrow_mut(), vide)),
            Err(e) => {
                api.set_restauration_etat(2);
                api.set_restauration_retour(retour_erreur(e.to_string()));
                if let Ok(mut p) = p.lock() {
                    *p = Some(r);
                }
                return;
            }
        }
        let (fa, d) = (f.as_weak(), (*d).clone());
        let essai = dossier_essai().is_some();
        std::thread::spawn(move || {
            let etape = |texte: String| {
                let _ = fa.upgrade_in_event_loop(move |f| {
                    f.global::<AdminApi>()
                        .set_restauration_etape(texte.into());
                });
            };
            let entreprise = r.entreprise.clone();
            let resultat = if essai {
                etape(tr("Restauration de la base..."));
                admin::restaurer_si(&d, &r, EtatService::Arrete)
                    .map_err(EchecRestauration::Restauration)
            } else {
                admin::arreter_service(DELAI_ARRET)
                    .map_err(EchecRestauration::Arret)
                    .and_then(|()| {
                        etape(tr("Restauration de la base..."));
                        admin::restaurer(&d, &r).map_err(EchecRestauration::Restauration)
                    })
            };
            drop(r);
            let redemarrage = match &resultat {
                _ if essai => None,
                Err(EchecRestauration::Arret(_)) => None,
                Err(EchecRestauration::Restauration(m))
                    if m.starts_with(admin::RESTAURATION_INTERROMPUE) =>
                {
                    None
                }
                _ => {
                    etape(tr("Démarrage du service..."));
                    Some(admin::demarrer_service(DELAI_DEMARRAGE))
                }
            };
            let _ = fa.upgrade_in_event_loop(move |f| {
                let api = f.global::<AdminApi>();
                let service = match &redemarrage {
                    Some(Err(e)) => trf(
                        "Le service n'a pas redémarré : {}. Démarrez le service KadenzServeur depuis les services de Windows.",
                        &[&tr(e)],
                    ),
                    _ => String::new(),
                };
                match resultat {
                    Ok(bilan) => {
                        api.set_restauration_bilan(ModelRc::new(VecModel::from(lignes_bilan(
                            &bilan,
                            &entreprise,
                        ))));
                        if bilan.identite_remplacee {
                            api.set_restauration_alerte(tr(
                                "Le certificat du serveur est revenu à celui de la sauvegarde : les postes inscrits avant elle se reconnectent sans rien faire ; un poste inscrit depuis doit être inscrit de nouveau, avec un nouveau code.",
                            ).into());
                        }
                        api.set_restauration_retour(if essai {
                            // Développement seulement, d'où le texte non traduit.
                            retour_ok(format!(
                                "{} Dossier d'essai : relancez le serveur d'essai sur ce dossier.",
                                tr("Sauvegarde restaurée.")
                            ))
                        } else if service.is_empty() {
                            retour_ok(tr("Sauvegarde restaurée ; le service a redémarré."))
                        } else {
                            retour_erreur(format!("{} {service}", tr("Sauvegarde restaurée.")))
                        });
                        api.set_restauration_etat(4);
                    }
                    Err(EchecRestauration::Arret(e)) => {
                        api.set_restauration_retour(retour_erreur(trf(
                            "Le service ne s'est pas arrêté : {}. Rien n'a été restauré.",
                            &[&tr(&e)],
                        )));
                        api.set_restauration_etat(1);
                    }
                    Err(EchecRestauration::Restauration(e)) => {
                        let message = if e.starts_with(admin::RESTAURATION_INTERROMPUE) {
                            // Message du stockage tel quel : il nomme les
                            // fichiers à remettre en place.
                            e
                        } else {
                            format!(
                                "{} {service}",
                                trf(
                                    "La restauration n'a pas abouti : {}. La base du serveur est restée telle quelle.",
                                    &[&tr(&e)],
                                )
                            )
                        };
                        api.set_restauration_retour(retour_erreur(message));
                        api.set_restauration_etat(4);
                    }
                }
                api.invoke_restauration_rouvrir();
            });
        });
    });
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
    let (etat, demarre) = texte_etat_service(admin::etat_service(), admin::port(conn));
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
    // Accord en français : singulier pour 0 et 1.
    let texte = if inscrits.len() <= 1 {
        trf(
            "{} inscrit sur {} permis par l'abonnement",
            &[&inscrits.len(), &permis],
        )
    } else {
        trf(
            "{} inscrits sur {} permis par l'abonnement",
            &[&inscrits.len(), &permis],
        )
    };
    api.set_postes_permis(texte.into());
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
    api.set_avertissement_sauvegarde(
        texte_avertissement(sauvegardes::avertissement(dossier)).into(),
    );
    api.set_sauvegarde_mot_de_passe_choisi(
        sauvegardes::lire_reglage(dossier).mot_de_passe.is_some(),
    );
    api.set_sauvegardes(ModelRc::new(VecModel::from(
        sauvegardes::liste(dossier)
            .unwrap_or_default()
            .into_iter()
            .map(|s| SauvegardeUi {
                chemin: s.chemin.display().to_string().into(),
                date: s.date.format("%d/%m/%Y %H:%M").to_string().into(),
                genre: genre_sauvegarde(s.genre).into(),
                taille: trf("{} Ko", &[&s.taille.div_ceil(1024)]).into(),
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

thread_local! {
    /// Mise à jour du serveur proposée, gardée sur le fil de l'interface.
    static MAJ_PROPOSEE: RefCell<Option<MiseAJour>> = const { RefCell::new(None) };
}

/// Mégaoctets entiers.
fn mo(octets: u64) -> String {
    (octets / 1_048_576).to_string()
}

/// Avancement du téléchargement (mêmes textes que l'application).
fn avancement(recus: u64, total: Option<u64>) -> String {
    match total.filter(|t| *t > 0) {
        Some(t) => trf(
            "Téléchargement : {} Mo sur {} Mo ({} %)",
            &[&mo(recus), &mo(t), &(recus * 100 / t)],
        ),
        None => trf("Téléchargement : {} Mo", &[&mo(recus)]),
    }
}

/// Issue d'une recherche de mise à jour du serveur : texte et erreur.
fn texte_recherche(recherche: &Recherche) -> (String, bool) {
    match recherche {
        // La version proposée s'affiche déjà au-dessus des boutons.
        Recherche::Disponible(_) => (String::new(), false),
        // Un poste attend son serveur ; le serveur, lui, n'attend personne.
        Recherche::AJour | Recherche::AttendServeur { .. } => (tr("Le serveur est à jour."), false),
        Recherche::Refusee => (
            tr(
                "Une version plus récente a été trouvée, mais sa signature n'est pas valide : elle n'est pas proposée.",
            ),
            true,
        ),
        Recherche::AutreGeneration(version) => (
            trf(
                "Kadenz {} est une nouvelle génération : elle s'installe séparément.",
                &[version],
            ),
            false,
        ),
        Recherche::HorsLigne => (
            tr(
                "Recherche impossible : le serveur des mises à jour ne répond pas. Vérifiez la connexion à Internet.",
            ),
            true,
        ),
    }
}

fn echec_maj(f: &FenetreAdmin, message: MessageCompose) {
    let api = f.global::<AdminApi>();
    api.set_maj_etat(texte_erreur(&Erreur::from(message)).into());
    api.set_maj_erreur(true);
    api.set_maj_en_cours(false);
}

/// Mise à jour du serveur (F10) : recherche à la demande sur un fil
/// secondaire, puis téléchargement vérifié et lancement de l'installateur ;
/// la fenêtre se ferme quand il démarre (Windows demande l'élévation).
fn brancher_maj(f: &FenetreAdmin) {
    let api = f.global::<AdminApi>();
    api.set_version(kadenz_maj::version_installee().into());

    let fa = f.as_weak();
    api.on_rechercher_maj(move || {
        let Some(f) = fa.upgrade() else {
            return;
        };
        let api = f.global::<AdminApi>();
        api.set_maj_en_cours(true);
        api.set_maj_erreur(false);
        api.set_maj_version("".into());
        api.set_maj_notes("".into());
        api.set_maj_etat(tr("Recherche en cours").into());
        MAJ_PROPOSEE.with(|p| *p.borrow_mut() = None);
        let fa = fa.clone();
        std::thread::spawn(move || {
            let recherche = kadenz_maj::rechercher_pour(Produit::Serveur);
            let _ = fa.upgrade_in_event_loop(move |f| {
                let api = f.global::<AdminApi>();
                let (texte, erreur) = texte_recherche(&recherche);
                api.set_maj_en_cours(false);
                api.set_maj_etat(texte.into());
                api.set_maj_erreur(erreur);
                if let Recherche::Disponible(m) = recherche {
                    api.set_maj_version(m.version.as_str().into());
                    api.set_maj_notes(m.notes.trim().into());
                    MAJ_PROPOSEE.with(|p| *p.borrow_mut() = Some(m));
                }
            });
        });
    });

    let fa = f.as_weak();
    api.on_installer_maj(move || {
        let Some(f) = fa.upgrade() else {
            return;
        };
        let Some(m) = MAJ_PROPOSEE.with(|p| p.borrow().clone()) else {
            return;
        };
        let api = f.global::<AdminApi>();
        api.set_maj_en_cours(true);
        api.set_maj_erreur(false);
        api.set_maj_etat(tr("Téléchargement en cours").into());
        let fa = fa.clone();
        std::thread::spawn(move || {
            // L'écran n'est rafraîchi qu'à chaque pour cent (ou chaque mégaoctet).
            let dernier = std::cell::Cell::new(u64::MAX);
            let suivi = fa.clone();
            let resultat = kadenz_maj::telecharger(&m, |recus, total| {
                let palier = match total.filter(|t| *t > 0) {
                    Some(t) => recus * 100 / t,
                    None => recus / 1_048_576,
                };
                if palier != dernier.replace(palier) {
                    let texte = avancement(recus, total);
                    let _ = suivi.upgrade_in_event_loop(move |f| {
                        f.global::<AdminApi>().set_maj_etat(texte.into());
                    });
                }
            });
            let _ = fa.upgrade_in_event_loop(move |f| match resultat {
                // Essai (développement seulement) : rien n'est lancé.
                Ok(chemin) if kadenz_maj::lancement_simule() => {
                    let api = f.global::<AdminApi>();
                    api.set_maj_en_cours(false);
                    api.set_maj_etat(
                        trf(
                            "Installateur téléchargé et vérifié (essai : lancement désactivé) : {}",
                            &[&chemin.display()],
                        )
                        .into(),
                    );
                }
                Ok(chemin) => match kadenz_maj::lancer(&chemin) {
                    // L'installateur a démarré : la fenêtre se ferme pour le
                    // laisser remplacer ses fichiers.
                    Ok(()) => {
                        let _ = slint::quit_event_loop();
                    }
                    Err(message) => echec_maj(&f, message),
                },
                Err(message) => echec_maj(&f, message),
            });
        });
    });
}

fn brancher_tableau(f: &FenetreAdmin, conn: Rc<RefCell<Connection>>, dossier: Rc<PathBuf>) {
    let api = f.global::<AdminApi>();
    api.set_dossier(dossier.display().to_string().into());
    brancher_maj(f);
    montrer_reglage_sauvegardes(f, &dossier);
    montrer_relais(f, &conn.borrow());
    actualiser(f, &conn.borrow(), &dossier);

    // Relais d'accès à distance (F8) : regler_relais valide l'adresse et
    // l'empreinte ; le service relit le réglage seul.
    let (fa, c) = (f.as_weak(), conn.clone());
    api.on_enregistrer_relais(move |adresse, empreinte| {
        let Some(f) = fa.upgrade() else {
            return retour_erreur(String::new());
        };
        let resultat = admin::regler_relais(&c.borrow(), Some((adresse.trim(), empreinte.trim())));
        match resultat {
            Ok(()) => {
                montrer_relais(&f, &c.borrow());
                retour_ok(tr(
                    "Relais enregistré. Le service le prend en compte dans quelques secondes.",
                ))
            }
            Err(e) => retour_erreur(trf("Relais non enregistré : {}.", &[&tr(&e)])),
        }
    });

    let (fa, c) = (f.as_weak(), conn.clone());
    api.on_effacer_relais(move || {
        let Some(f) = fa.upgrade() else {
            return retour_erreur(String::new());
        };
        let resultat = admin::regler_relais(&c.borrow(), None);
        match resultat {
            Ok(()) => {
                montrer_relais(&f, &c.borrow());
                retour_ok(tr(
                    "Relais effacé : l'accès à distance n'est plus possible.",
                ))
            }
            Err(e) => retour_erreur(trf("Relais non effacé : {}.", &[&tr(&e)])),
        }
    });

    // Réglage des sauvegardes (décision 56) : mot de passe vide, il est gardé.
    let (fa, c, d) = (f.as_weak(), conn.clone(), dossier.clone());
    api.on_enregistrer_sauvegardes(move |mot_de_passe, confirmation| {
        let Some(f) = fa.upgrade() else {
            return retour_erreur(String::new());
        };
        if mot_de_passe != confirmation {
            return retour_erreur(tr("Les deux mots de passe ne correspondent pas."));
        }
        let api = f.global::<AdminApi>();
        let heure = match heure_sauvegarde(&api.get_sauvegarde_heure()) {
            Ok(heure) => heure,
            Err(message) => return retour_erreur(message),
        };
        let dossier_sauvegardes = PathBuf::from(api.get_sauvegarde_dossier().trim());
        let nouveau = (!mot_de_passe.is_empty()).then_some(mot_de_passe.as_str());
        match sauvegardes::enregistrer_reglage(&d, &dossier_sauvegardes, heure, nouveau) {
            Ok(()) => {
                montrer_reglage_sauvegardes(&f, &d);
                actualiser(&f, &c.borrow(), &d);
                retour_ok(tr(
                    "Réglage des sauvegardes enregistré. Le service le prend en compte dans la minute.",
                ))
            }
            Err(e) => retour_erreur(tr(&e)),
        }
    });

    // Sauvegarde immédiate, sur une connexion à part dans un fil : la copie et
    // son chiffrement peuvent prendre quelques secondes.
    let (fa, d) = (f.as_weak(), dossier.clone());
    api.on_sauvegarder_maintenant(move || {
        let Some(f) = fa.upgrade() else { return };
        let api = f.global::<AdminApi>();
        api.set_sauvegarde_en_cours(true);
        api.set_sauvegarde_retour(retour_ok(String::new()));
        let (fa, d) = (f.as_weak(), (*d).clone());
        std::thread::spawn(move || {
            let resultat = admin::ouvrir_base(&d)
                .and_then(|conn| sauvegardes::sauvegarder(&conn, &d, Genre::Manuelle));
            let _ = fa.upgrade_in_event_loop(move |f| {
                let api = f.global::<AdminApi>();
                api.set_sauvegarde_en_cours(false);
                api.set_sauvegarde_retour(match resultat {
                    Ok(chemin) => retour_ok(trf("Sauvegarde faite : {}", &[&chemin.display()])),
                    Err(e) => retour_erreur(tr(&e)),
                });
                api.invoke_actualiser();
            });
        });
    });

    let (fa, c, d) = (f.as_weak(), conn.clone(), dossier.clone());
    api.on_actualiser(move || {
        if let Some(f) = fa.upgrade()
            && f.global::<AdminApi>().get_restauration_etat() != 3
        {
            actualiser(&f, &c.borrow(), &d);
        }
    });

    brancher_restauration(f, conn.clone(), dossier.clone());

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

/// Activation d'un code de licence court (décision 74) : appel au portail dans
/// un fil à part, résultat rendu à `ActivationUi` dans la boucle de l'interface.
fn brancher_activation(f: &FenetreAdmin, entreprise: String) {
    let fa = f.as_weak();
    f.global::<ActivationUi>().on_activer(move |code, client| {
        let Some(f) = fa.upgrade() else {
            return;
        };
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
            let resultat = activation_ui::activer(&code, &entreprise, &client);
            let _ = slint::invoke_from_event_loop(move || {
                let Some(f) = fa.upgrade() else {
                    return;
                };
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
                        ui.set_message(activation_ui::message(&m).into());
                        ui.set_etat(3);
                    }
                }
            });
        });
    });
}

fn brancher_assistant(f: &FenetreAdmin, conn: Rc<RefCell<Connection>>, dossier: Rc<PathBuf>) {
    montrer_reglage_sauvegardes(f, &dossier);
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
    brancher_activation(f, entreprise.clone());
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
        // Sauvegardes d'abord (décision 56) : un réglage refusé n'installe rien.
        let heure = match heure_sauvegarde(&d.sauvegarde_heure) {
            Ok(heure) => heure,
            Err(message) => return message.into(),
        };
        if let Err(e) = sauvegardes::enregistrer_reglage(
            &dossier,
            Path::new(d.sauvegarde_dossier.trim()),
            heure,
            Some(d.sauvegarde_mot_de_passe.as_str()),
        ) {
            return tr(&e).into();
        }
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
    fenetre
        .global::<CodeCourtUi>()
        .on_formater(|precedent, nouveau| code_saisie::formater(&precedent, &nouveau));
    let dossier = Rc::new(dossier());
    let fa = fenetre.as_weak();
    fenetre
        .global::<AdminApi>()
        .on_choisir_dossier(move |actuel| match fa.upgrade() {
            Some(f) => choisir_dossier(&f, &actuel),
            None => actuel,
        });
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
