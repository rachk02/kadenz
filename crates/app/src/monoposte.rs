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
//! l'inscription aussi : le serveur la traite dans son propre fil de base.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use chrono::{NaiveTime, Utc};
use kadenz_client::{Client, ErreurClient, Identite, Inscrit};
use kadenz_domaine::lettres::Orthographe;
use kadenz_domaine::role::Role;
use kadenz_domaine::saisie;
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

struct Contexte {
    integre: Integre,
    demarrer: Demarrer,
    /// Dossier du serveur intégré.
    dossier: PathBuf,
}

thread_local! {
    static CONTEXTE: RefCell<Option<Rc<Contexte>>> = const { RefCell::new(None) };
    /// Reprise de la base V1 à proposer à la prochaine connexion d'un Admin
    /// (spécification, section 3.0) : posé après l'inscription automatique.
    static REPRISE_A_PROPOSER: Cell<bool> = const { Cell::new(false) };
}

fn contexte() -> Option<Rc<Contexte>> {
    CONTEXTE.with_borrow(|c| c.clone())
}

/// Vrai dans l'édition monoposte. En attendant la marque d'édition posée par
/// l'installateur (lot M5, `kadenz_maj::edition::monoposte`), seulement en
/// version de développement avec `KADENZ_MONOPOSTE=1` : sans marque, c'est
/// l'édition multiposte, et rien ne change pour un poste actuel.
pub(crate) fn edition_monoposte() -> bool {
    cfg!(debug_assertions) && std::env::var_os("KADENZ_MONOPOSTE").is_some_and(|v| v == "1")
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
    f.set_message_demarrage(crate::phrase(message));
    f.set_ecran(Ecran::DemarrageImpossible);
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
    serveur.base(tache).map_err(|detail| {
        crate::consigner_erreur(&format!("Serveur intégré : {detail}"));
        crate::phrase(tr("le serveur de Kadenz ne répond pas"))
    })
}

/// Démarre l'édition monoposte : écran d'attente, serveur intégré démarré dans
/// un fil à part, puis l'écran qui convient à l'état trouvé.
pub(crate) fn lancer(f: &FenetrePrincipale, integre: Integre, demarrer: Demarrer) {
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
    CONTEXTE.set(Some(Rc::new(Contexte {
        integre,
        demarrer,
        dossier: dossier.clone(),
    })));
    brancher(f);
    let fa = f.as_weak();
    std::thread::spawn(move || {
        let resultat = Serveur::demarrer(Config::monoposte(dossier));
        let _ = fa.upgrade_in_event_loop(move |f| match resultat {
            Ok(serveur) => serveur_demarre(&f, serveur),
            Err(e) => {
                crate::consigner_erreur(&format!("Démarrage du serveur intégré : {e}"));
                impossible(&f, message_erreur(&e));
            }
        });
    });
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

    // Démarrage impossible : le dossier des données est celui du serveur.
    f.on_ouvrir_dossier_donnees(|| {
        if let Some(c) = contexte() {
            let _ = std::process::Command::new("explorer")
                .arg(&c.dossier)
                .spawn();
        }
    });
}

fn serveur_demarre(f: &FenetrePrincipale, serveur: Serveur) {
    let Some(c) = contexte() else { return };
    *c.integre.borrow_mut() = Some(serveur);
    match sur_le_serveur(&c, |conn| comptes::etat_installation(conn)) {
        Ok(Ok(EtatInstallation::AssistantRequis)) => {
            preparer_assistant(f, &c);
            f.set_ecran(Ecran::Presentation);
        }
        Ok(Ok(EtatInstallation::ConnexionRequise)) => {
            match poste_relie::copie(&crate::dossier_donnees()) {
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

/// Copie du poste relié présente : son adresse est mise au port effectif du
/// serveur intégré, avant le synchroniseur (spécification, section 2.1).
fn ouvrir_poste(f: &FenetrePrincipale, c: &Contexte, copie: &Path) {
    let (adresse, empreinte) = {
        let integre = c.integre.borrow();
        let Some(serveur) = integre.as_ref() else {
            return;
        };
        (
            format!("127.0.0.1:{}", serveur.adresse().port()),
            serveur.empreinte().to_string(),
        )
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
        impossible(
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
    let installation = sur_le_serveur(&c, move |conn| {
        let licence = (!cle.is_empty()).then_some(cle.as_str());
        admin::installer(conn, &init, licence).map(|_| ())
    });
    match installation {
        Ok(Ok(())) => {
            inscrire(f);
            SharedString::new()
        }
        Ok(Err(e)) => crate::phrase(e),
        Err(message) => message,
    }
}

/// Inscription automatique de ce poste sur le serveur intégré
/// (spécification, section 3, étape 4) : code créé sur le serveur, puis
/// `Client::inscrire` sur 127.0.0.1 dans un fil à part ; un échec annule le
/// code aussitôt.
fn inscrire(f: &FenetrePrincipale) {
    let Some(c) = contexte() else { return };
    attendre(f, tr("Inscription de ce poste..."));
    let (adresse, empreinte) = {
        let integre = c.integre.borrow();
        let Some(serveur) = integre.as_ref() else {
            return;
        };
        (
            format!("127.0.0.1:{}", serveur.adresse().port()),
            serveur.empreinte().to_string(),
        )
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
    let poste = PosteRelie {
        adresse,
        empreinte_serveur: empreinte,
        entreprise: inscrit.entreprise,
        nom: inscrit.nom,
        certificat: identite.certificat,
        cle: identite.cle,
    };
    let ouverture = poste_relie::creer_copie(&crate::dossier_donnees(), &poste)
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
        serveur.arreter();
        // Le fil de la base du serveur se termine après l'arrêt, sans être
        // attendu : sa connexion SQLite peut tenir encore le fichier un instant.
        for _ in 0..50 {
            if std::fs::remove_dir_all(&dossier).is_ok() || !dossier.exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        assert!(
            !dossier.exists(),
            "dossier d'essai resté : {}",
            dossier.display()
        );
    }
}
