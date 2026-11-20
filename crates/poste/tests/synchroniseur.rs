//! Essais de bout en bout du jalon F5c : un serveur réel sur 127.0.0.1, la
//! copie d'un poste et son synchroniseur.

use std::cell::RefCell;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use kadenz_client::Identite;
use kadenz_domaine::module::Module;
use kadenz_domaine::referentiel::{FicheTiers, TypeTiers};
use kadenz_domaine::role::Role;
use kadenz_licence::SigningKey;
use kadenz_licence::entreprise::{self as lic, LicenceEntreprise};
use kadenz_poste::{
    AvertissementSauvegarde, DELAI, ErreurPoste, EtatConnexion, Evenement, SourceV1, Synchroniseur,
};
use kadenz_protocole::{Action, Entite, Issue, Ligne, Motif, Operation, Saisie, Tiers};
use kadenz_serveur::admin::{self, EtatService};
use kadenz_serveur::{Config, Serveur, sauvegardes};
use kadenz_stockage::comptes::{self, NouvelUtilisateur};
use kadenz_stockage::entreprise::Entreprise;
use kadenz_stockage::installation::{self, InstallationInitiale};
use kadenz_stockage::parametres::Parametres;
use kadenz_stockage::poste_relie::{self, PosteRelie};
use kadenz_stockage::reprise::MODELE_PAS_NEUF;
use kadenz_stockage::{abonnement, postes, replique, synchro, tiers};

const MOT_DE_PASSE: &str = "motdepasse1";

fn cle_portail() -> SigningKey {
    SigningKey::from_bytes(&[5u8; 32])
}

fn licence(entreprise: &str) -> String {
    let jour = |n: i64| {
        (abonnement::horloge() + chrono::Duration::days(n))
            .format("%Y-%m-%d")
            .to_string()
    };
    lic::emettre(
        &LicenceEntreprise {
            entreprise: entreprise.into(),
            client: "Entreprise Essai".into(),
            formule: lic::Formule::Annuelle,
            postes: 5,
            modules: Module::TOUS.to_vec(),
            debut: jour(0),
            echeance: jour(365),
            emetteur: lic::Emetteur::Editeur,
            emise_le: jour(0),
        },
        &cle_portail(),
    )
    .unwrap()
}

fn dossier(nom: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("kadenz-poste-essai-{nom}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn config(dossier: &Path, port: u16) -> Config {
    let mut config = Config::nouvelle(dossier.to_path_buf());
    config.adresse = SocketAddr::from(([127, 0, 0, 1], port));
    config.controle_droits = false;
    config.cles_portail = vec![cle_portail().verifying_key()];
    config
}

struct Banc {
    serveur: Serveur,
    dossier_serveur: PathBuf,
    entreprise: String,
    admin: String,
    copie: PathBuf,
    identite: Identite,
}

/// Serveur installé avec son abonnement, poste inscrit (ou non) et sa copie
/// créée, vide.
fn banc(nom: &str, inscrit: bool) -> Banc {
    let dossier_serveur = dossier(&format!("{nom}-serveur"));
    let serveur = Serveur::demarrer(config(&dossier_serveur, 0)).unwrap();
    let (entreprise, admin) = serveur
        .base(|c| {
            let admin = installation::installer(
                c,
                &InstallationInitiale {
                    entreprise: Entreprise {
                        nom: "Entreprise Essai".into(),
                        ..Default::default()
                    },
                    admin: NouvelUtilisateur {
                        nom_complet: "Administrateur".into(),
                        identifiant: "admin".into(),
                        mot_de_passe: MOT_DE_PASSE.into(),
                        role: Role::Admin,
                    },
                    parametres: Parametres::default(),
                },
            )
            .unwrap();
            synchro::completer_uids(c).unwrap();
            let uid = synchro::uid_de(c, Entite::Utilisateur, admin.id)
                .unwrap()
                .unwrap();
            let entreprise = synchro::identifiant_entreprise(c).unwrap();
            abonnement::enregistrer_avec(
                c,
                &admin,
                &licence(&entreprise),
                abonnement::horloge(),
                &[cle_portail().verifying_key()],
            )
            .unwrap();
            (entreprise, uid)
        })
        .unwrap();
    let identite = Identite::nouvelle("poste").unwrap();
    if inscrit {
        let empreinte = identite.empreinte();
        serveur
            .base(move |c| postes::inscrire(c, &empreinte, "Poste d'essai").unwrap())
            .unwrap();
    }
    let copie = poste_relie::creer_copie(
        &dossier(&format!("{nom}-poste")),
        &PosteRelie {
            adresse: serveur.adresse().to_string(),
            empreinte_serveur: serveur.empreinte().to_string(),
            entreprise: entreprise.clone(),
            nom: "Poste d'essai".into(),
            certificat: identite.certificat.clone(),
            cle: identite.cle.clone(),
        },
    )
    .unwrap();
    Banc {
        serveur,
        dossier_serveur,
        entreprise,
        admin,
        copie,
        identite,
    }
}

fn demarrer(copie: &Path) -> (Synchroniseur, Receiver<Evenement>) {
    let (envoi, reception) = mpsc::channel();
    let s = Synchroniseur::demarrer(
        copie.to_path_buf(),
        Box::new(move |e| {
            let _ = envoi.send(e);
        }),
    )
    .unwrap();
    (s, reception)
}

fn attendre(quoi: &str, condition: impl Fn() -> bool) {
    let debut = Instant::now();
    while !condition() {
        assert!(
            debut.elapsed() < Duration::from_secs(15),
            "attente : {quoi}"
        );
        thread::sleep(Duration::from_millis(50));
    }
}

fn present(copie: &Path, entite: Entite, uid: &str) -> bool {
    let conn = poste_relie::ouvrir_copie(copie).unwrap();
    synchro::id_de(&conn, entite, uid).unwrap().is_some()
}

fn fiche(nom: &str) -> Tiers {
    Tiers {
        nom: nom.into(),
        type_tiers: "client".into(),
        ..Tiers::default()
    }
}

#[test]
fn ecriture_puis_changements_du_serveur() {
    let b = banc("ecriture", true);
    let (s, evenements) = demarrer(&b.copie);
    attendre("connexion", || s.etat() == EtatConnexion::Connecte);
    let compte = s.connecter("admin", MOT_DE_PASSE).unwrap();
    assert_eq!(compte.compte, b.admin);
    // Première synchronisation : le compte arrive dans la copie.
    attendre("compte dans la copie", || {
        present(&b.copie, Entite::Utilisateur, &b.admin)
    });

    // La ligne créée est dans la copie dès le retour de l'écriture.
    let t = synchro::nouvel_uid();
    let issue = s
        .ecrire(Action::CreerTiers {
            uid: t.clone(),
            fiche: fiche("Client Poste"),
        })
        .unwrap();
    assert_eq!(issue, Issue::acceptee());
    assert!(present(&b.copie, Entite::Tiers, &t));

    // Un refus du serveur revient comme résultat, à traduire.
    let issue = s
        .ecrire(Action::ModifierTiers {
            uid: synchro::nouvel_uid(),
            fiche: fiche("Inconnu"),
        })
        .unwrap();
    assert!(matches!(issue, Issue::Refusee { .. }), "{issue:?}");

    // Changement fait sur la machine du serveur : relevé dans les 2 s.
    let admin = b.admin.clone();
    let cree = b
        .serveur
        .base(move |c| {
            let id = synchro::id_de(c, Entite::Utilisateur, &admin)
                .unwrap()
                .unwrap();
            let qui = comptes::lire(c, id).unwrap().unwrap();
            let t = tiers::creer(
                c,
                &qui,
                &FicheTiers {
                    nom: "Client Serveur".into(),
                    ..FicheTiers::vide(TypeTiers::Client)
                },
            )
            .unwrap();
            synchro::completer_uids(c).unwrap();
            synchro::uid_de(c, Entite::Tiers, t.id).unwrap().unwrap()
        })
        .unwrap();
    attendre("tiers du serveur dans la copie", || {
        present(&b.copie, Entite::Tiers, &cree)
    });
    let recus: Vec<Evenement> = evenements.try_iter().collect();
    assert!(recus.iter().any(
        |e| matches!(e, Evenement::Changements { sortes } if sortes.contains(&Entite::Tiers))
    ));
    assert!(recus.iter().any(|e| matches!(e, Evenement::Abonnement(_))));
    s.arreter();
}

#[test]
fn mot_de_passe_refuse() {
    let b = banc("refus", true);
    let (s, _evenements) = demarrer(&b.copie);
    attendre("connexion", || s.etat() == EtatConnexion::Connecte);
    assert!(matches!(
        s.connecter("admin", "mauvais"),
        Err(ErreurPoste::CompteRefuse { .. })
    ));
    // Sans compte connecté, rien n'est envoyé.
    assert!(matches!(
        s.ecrire(Action::CreerTiers {
            uid: synchro::nouvel_uid(),
            fiche: fiche("X"),
        }),
        Err(ErreurPoste::Base(_))
    ));
    s.arreter();
}

#[test]
fn poste_retire_etat_final() {
    let b = banc("retire", false);
    let (s, _evenements) = demarrer(&b.copie);
    attendre("poste retiré", || s.etat() == EtatConnexion::PosteRetire);
    assert_eq!(
        s.connecter("admin", MOT_DE_PASSE),
        Err(ErreurPoste::Injoignable)
    );
    s.arreter();
}

/// Serveur relancé avec une configuration donnée, le port pouvant tarder à
/// se libérer.
fn relancer_avec(config: Config) -> Serveur {
    for _ in 0..40 {
        match Serveur::demarrer(config.clone()) {
            Ok(sv) => return sv,
            Err(_) => thread::sleep(Duration::from_millis(250)),
        }
    }
    panic!("serveur non relancé sur {}", config.adresse);
}

/// Serveur d'une autre version du protocole (F10) : plus ancienne, le poste
/// attend sa mise à jour en réessayant ; plus récente, le poste est à mettre
/// à jour (état final). La version logicielle du serveur est retenue.
#[test]
fn version_du_serveur_differente() {
    let b = banc("version", true);
    let port = b.serveur.adresse().port();
    b.serveur.arreter();
    let mut ancien = config(&b.dossier_serveur, port);
    ancien.version_protocole = kadenz_protocole::VERSION - 1;
    let serveur = relancer_avec(ancien);
    let (s, _evenements) = demarrer(&b.copie);
    let attendu = EtatConnexion::AttendServeur {
        serveur: kadenz_protocole::VERSION - 1,
        logiciel: Some(env!("CARGO_PKG_VERSION").into()),
    };
    attendre("attente du serveur", || s.etat() == attendu);
    assert!(!s.etat().definitif());
    assert_eq!(
        s.logiciel_serveur().as_deref(),
        Some(env!("CARGO_PKG_VERSION"))
    );
    // Hors connexion en attendant : la connexion d'un compte le dit.
    assert_eq!(
        s.connecter("admin", MOT_DE_PASSE),
        Err(ErreurPoste::HorsConnexion)
    );
    s.arreter();
    serveur.arreter();

    let mut recent = config(&b.dossier_serveur, port);
    recent.version_protocole = kadenz_protocole::VERSION + 1;
    let serveur = relancer_avec(recent);
    let (s, _evenements) = demarrer(&b.copie);
    let attendu = EtatConnexion::VersionDifferente {
        serveur: kadenz_protocole::VERSION + 1,
        logiciel: Some(env!("CARGO_PKG_VERSION").into()),
    };
    attendre("poste à mettre à jour", || s.etat() == attendu);
    assert!(s.etat().definitif());
    s.arreter();
    serveur.arreter();
}

#[test]
fn ecriture_sans_reponse_confirmee_a_la_connexion() {
    let b = banc("en-cours", true);
    // Le poste s'est arrêté après l'envoi, sans recevoir la réponse.
    let t = synchro::nouvel_uid();
    let op = Operation {
        id: synchro::nouvel_uid(),
        entreprise: b.entreprise.clone(),
        poste: b.identite.empreinte(),
        compte: b.admin.clone(),
        hors_connexion: false,
        heure_poste: None,
        base: 0,
        action: Action::CreerTiers {
            uid: t.clone(),
            fiche: fiche("Client Repris"),
        },
    };
    {
        let conn = poste_relie::ouvrir_copie(&b.copie).unwrap();
        replique::garder_en_cours(&conn, Some(&op)).unwrap();
    }
    let (s, evenements) = demarrer(&b.copie);
    attendre("connexion", || s.etat() == EtatConnexion::Connecte);
    s.connecter("admin", MOT_DE_PASSE).unwrap();
    attendre("tiers repris dans la copie", || {
        present(&b.copie, Entite::Tiers, &t)
    });
    let confirmee = evenements.try_iter().find_map(|e| match e {
        Evenement::Confirmee { issue, .. } => Some(issue),
        _ => None,
    });
    assert_eq!(confirmee, Some(Issue::acceptee()));
    let conn = poste_relie::ouvrir_copie(&b.copie).unwrap();
    assert!(replique::operation_en_cours(&conn).unwrap().is_none());
    s.arreter();
}

#[test]
fn compte_connecte_au_retour_du_serveur() {
    let b = banc("differe", true);
    let port = b.serveur.adresse().port();
    b.serveur.arreter();
    // Serveur arrêté : le compte attend (décision 48).
    let (s, evenements) = demarrer(&b.copie);
    attendre("serveur injoignable", || {
        matches!(
            s.etat(),
            EtatConnexion::Reconnexion { .. } | EtatConnexion::Injoignable
        )
    });
    assert_eq!(
        s.connecter("admin", MOT_DE_PASSE),
        Err(ErreurPoste::HorsConnexion)
    );
    assert_eq!(
        s.ecrire(Action::CreerTiers {
            uid: synchro::nouvel_uid(),
            fiche: fiche("Hors connexion"),
        }),
        Err(ErreurPoste::HorsConnexion)
    );
    // Le serveur revient sur le même port : le compte se connecte seul.
    let mut serveur = None;
    for _ in 0..20 {
        match Serveur::demarrer(config(&b.dossier_serveur, port)) {
            Ok(sv) => {
                serveur = Some(sv);
                break;
            }
            Err(_) => thread::sleep(Duration::from_millis(250)),
        }
    }
    let _serveur = serveur.expect("serveur redémarré");
    let debut = Instant::now();
    let issue = loop {
        let e = evenements
            .recv_timeout(Duration::from_secs(20))
            .expect("événement Compte attendu");
        if let Evenement::Compte(issue) = e {
            break issue;
        }
        assert!(debut.elapsed() < Duration::from_secs(25));
    };
    assert_eq!(issue.map(|c| c.compte), Ok(b.admin.clone()));
    attendre("compte dans la copie", || {
        present(&b.copie, Entite::Utilisateur, &b.admin)
    });
    // L'abonnement reçu est gardé dans la copie, et ses modules s'appliquent.
    let conn = poste_relie::ouvrir_copie(&b.copie).unwrap();
    attendre("abonnement gardé", || {
        abonnement::dernier_recu(&conn).unwrap().is_some()
    });
    poste_relie::appliquer_modules(&conn).unwrap();
    s.arreter();
}

/// Essai F6d : le serveur est relancé pendant que le poste garde une session
/// TLS ouverte sans compte. La connexion du compte la trouve morte ; le
/// compte doit alors attendre le retour du serveur, et s'y connecter seul.
#[test]
fn compte_connecte_apres_session_perdue_en_route() {
    let b = banc("perdue", true);
    let port = b.serveur.adresse().port();
    let (s, evenements) = demarrer(&b.copie);
    attendre("connexion", || s.etat() == EtatConnexion::Connecte);
    let dossier_serveur = b.dossier_serveur.clone();
    b.serveur.arreter();
    let _serveur = relancer(&dossier_serveur, port);
    // La session ouverte au démarrage est refermée par l'ancien serveur ; si
    // le poste l'a déjà vu, il rouvre seul et le compte se connecte tout de
    // suite.
    match s.connecter("admin", MOT_DE_PASSE) {
        Ok(c) => assert_eq!(c.compte, b.admin),
        Err(e) => {
            assert_eq!(e, ErreurPoste::HorsConnexion);
            let debut = Instant::now();
            let issue = loop {
                let e = evenements
                    .recv_timeout(Duration::from_secs(20))
                    .expect("événement Compte attendu");
                if let Evenement::Compte(issue) = e {
                    break issue;
                }
                assert!(debut.elapsed() < Duration::from_secs(25));
            };
            assert_eq!(issue.map(|c| c.compte), Ok(b.admin.clone()));
        }
    }
    attendre("connexion", || s.etat() == EtatConnexion::Connecte);
    // Le compte est bien connecté : une écriture passe au serveur.
    let t = synchro::nouvel_uid();
    assert!(matches!(
        s.ecrire(Action::CreerTiers {
            uid: t.clone(),
            fiche: fiche("Après relance"),
        }),
        Ok(Issue::Acceptee { .. })
    ));
    s.arreter();
}

/// Essai F6d : la coupure n'est vue qu'à l'envoi d'une écriture permise hors
/// connexion. Elle passe dans la file, appliquée dans la copie, au lieu de
/// rester sans réponse (`Delai`), puis part au retour du serveur.
#[test]
fn coupure_vue_a_l_envoi_ecriture_en_file() {
    let b = banc("envoi", true);
    let port = b.serveur.adresse().port();
    let (s, evenements) = demarrer(&b.copie);
    attendre("connexion", || s.etat() == EtatConnexion::Connecte);
    s.connecter("admin", MOT_DE_PASSE).unwrap();
    attendre("compte dans la copie", || {
        present(&b.copie, Entite::Utilisateur, &b.admin)
    });
    let dossier_serveur = b.dossier_serveur.clone();
    // Le serveur s'arrête ; le poste ne le voit qu'à sa prochaine requête.
    b.serveur.arreter();
    let t = synchro::nouvel_uid();
    assert_eq!(
        s.ecrire(Action::CreerTiers {
            uid: t.clone(),
            fiche: fiche("Client de la coupure"),
        }),
        Ok(Issue::EnFile)
    );
    assert!(present(&b.copie, Entite::Tiers, &t));
    assert_eq!(s.file(), 1);
    let conn = poste_relie::ouvrir_copie(&b.copie).unwrap();
    assert!(replique::operation_en_cours(&conn).unwrap().is_none());
    // Non permise hors connexion : rendue tout de suite.
    assert_eq!(
        s.ecrire(Action::CreerCompte {
            uid: synchro::nouvel_uid(),
            nom_complet: "Nouveau".into(),
            identifiant: "nouveau".into(),
            mot_de_passe: MOT_DE_PASSE.into(),
            role: "commercial".into(),
        }),
        Err(ErreurPoste::HorsConnexion)
    );
    // Serveur arrêté : la file ne se vide pas, le délai est tenu et rien
    // n'est perdu (fermeture du monoposte).
    let debut = std::time::Instant::now();
    assert!(!s.vider(Duration::from_millis(300)));
    assert!(debut.elapsed() < Duration::from_secs(2));
    assert_eq!(s.file(), 1);

    let serveur = relancer(&dossier_serveur, port);
    assert!(s.vider(Duration::from_secs(30)));
    attendre("file envoyée", || s.file() == 0);
    let vus = RefCell::new(Vec::new());
    attendre("écriture confirmée", || {
        confirmees(&evenements, &vus)
            .iter()
            .any(|i| matches!(i, Issue::Acceptee { .. }))
    });
    let t2 = t.clone();
    let sur_serveur = serveur
        .base(move |c| synchro::id_de(c, Entite::Tiers, &t2).unwrap().is_some())
        .unwrap();
    assert!(sur_serveur, "le tiers est arrivé au serveur");
    s.arreter();
}

/// Essai F6d : hors connexion, une écriture d'un module de la licence (un
/// devis) est appliquée dans la copie avec les modules de la licence gardée,
/// et non refusée comme si le module manquait.
#[test]
fn devis_hors_connexion_avec_les_modules_gardes() {
    let b = banc("modules", true);
    {
        let (s, _e) = demarrer(&b.copie);
        attendre("connexion", || s.etat() == EtatConnexion::Connecte);
        s.connecter("admin", MOT_DE_PASSE).unwrap();
        attendre("compte dans la copie", || {
            present(&b.copie, Entite::Utilisateur, &b.admin)
        });
        let conn = poste_relie::ouvrir_copie(&b.copie).unwrap();
        attendre("abonnement gardé", || {
            abonnement::dernier_recu(&conn).unwrap().is_some()
        });
        s.arreter();
    }
    b.serveur.arreter();

    let (s, _e) = demarrer(&b.copie);
    attendre("serveur injoignable", || injoignable(&s));
    assert_eq!(
        s.connecter("admin", MOT_DE_PASSE),
        Err(ErreurPoste::HorsConnexion)
    );
    let t = synchro::nouvel_uid();
    assert_eq!(
        s.ecrire(Action::CreerTiers {
            uid: t.clone(),
            fiche: fiche("Client du devis"),
        }),
        Ok(Issue::EnFile)
    );
    let d = synchro::nouvel_uid();
    assert_eq!(
        s.ecrire(Action::CreerBrouillon {
            uid: d.clone(),
            type_document: "devis".into(),
            saisie: saisie(&t),
        }),
        Ok(Issue::EnFile)
    );
    assert_eq!(s.file(), 2);
    s.arreter();
}

/// Essai F6d : une tentative de connexion qui traîne (le serveur accepte la
/// connexion TCP et ne répond jamais) ne bloque plus le fil. Une écriture
/// permise part en file tout de suite, et la connexion d'un compte attend
/// cette seule tentative, sans en empiler une seconde.
#[test]
fn tentative_qui_traine_sans_bloquer_le_fil() {
    let b = banc("traine", true);
    {
        let (s, _e) = demarrer(&b.copie);
        attendre("connexion", || s.etat() == EtatConnexion::Connecte);
        s.connecter("admin", MOT_DE_PASSE).unwrap();
        attendre("compte dans la copie", || {
            present(&b.copie, Entite::Utilisateur, &b.admin)
        });
        let conn = poste_relie::ouvrir_copie(&b.copie).unwrap();
        attendre("abonnement gardé", || {
            abonnement::dernier_recu(&conn).unwrap().is_some()
        });
        s.arreter();
    }
    b.serveur.arreter();
    // Écoute muette : chaque connexion est acceptée et gardée ouverte, sans
    // réponse ; la poignée de main TLS attend donc `DELAI`.
    let muet = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let adresse = muet.local_addr().unwrap().to_string();
    let gardees = Arc::new(Mutex::new(Vec::new()));
    {
        let gardees = gardees.clone();
        thread::spawn(move || {
            for flux in muet.incoming().flatten() {
                gardees.lock().unwrap().push(flux);
            }
        });
    }
    {
        let conn = poste_relie::ouvrir_copie(&b.copie).unwrap();
        let mut poste = poste_relie::lire(&conn).unwrap().unwrap();
        poste.adresse = adresse;
        poste_relie::garder(&conn, &poste).unwrap();
    }

    let (s, _e) = demarrer(&b.copie);
    attendre("première tentative en cours", || {
        !gardees.lock().unwrap().is_empty()
    });
    let debut = Instant::now();
    assert_eq!(
        s.connecter("admin", MOT_DE_PASSE),
        Err(ErreurPoste::HorsConnexion)
    );
    assert!(debut.elapsed() < DELAI + Duration::from_secs(2));
    // La tentative suivante part une seconde après l'échec, puis traîne.
    let deja = gardees.lock().unwrap().len();
    attendre("seconde tentative en cours", || {
        gardees.lock().unwrap().len() > deja
    });
    let debut = Instant::now();
    let t = synchro::nouvel_uid();
    assert_eq!(
        s.ecrire(Action::CreerTiers {
            uid: t.clone(),
            fiche: fiche("Client de la tentative"),
        }),
        Ok(Issue::EnFile)
    );
    assert!(debut.elapsed() < Duration::from_secs(1));
    assert!(present(&b.copie, Entite::Tiers, &t));
    assert_eq!(s.file(), 1);
    s.arreter();
}

/// Second poste inscrit sur le serveur du banc, avec sa propre copie.
fn second_poste(b: &Banc, nom: &str) -> PathBuf {
    let identite = Identite::nouvelle("poste2").unwrap();
    let empreinte = identite.empreinte();
    b.serveur
        .base(move |c| postes::inscrire(c, &empreinte, "Second poste").unwrap())
        .unwrap();
    poste_relie::creer_copie(
        &dossier(&format!("{nom}-poste2")),
        &PosteRelie {
            adresse: b.serveur.adresse().to_string(),
            empreinte_serveur: b.serveur.empreinte().to_string(),
            entreprise: b.entreprise.clone(),
            nom: "Second poste".into(),
            certificat: identite.certificat.clone(),
            cle: identite.cle.clone(),
        },
    )
    .unwrap()
}

fn saisie(tiers: &str) -> Saisie {
    Saisie {
        date: chrono::Local::now()
            .date_naive()
            .format("%Y-%m-%d")
            .to_string(),
        tiers: Some(tiers.into()),
        libelle: "Essai F5e".into(),
        lignes: vec![Ligne {
            designation: "Ciment".into(),
            prix: 5000,
            quantite: 2_000,
            ..Ligne::default()
        }],
        ..Saisie::default()
    }
}

/// Numéro d'un document dans une copie, s'il y est validé.
fn numero(copie: &Path, uid: &str) -> Option<String> {
    let conn = poste_relie::ouvrir_copie(copie).unwrap();
    let id = synchro::id_de(&conn, Entite::Document, uid).unwrap()?;
    conn.query_row("SELECT numero FROM document WHERE id = ?1", [id], |r| {
        r.get(0)
    })
    .unwrap()
}

/// Jalon F5e : deux postes sur le même serveur. Une fiche créée sur l'un
/// paraît sur l'autre ; deux validations simultanées reçoivent deux numéros
/// consécutifs de la même série.
#[test]
fn deux_postes_numeros_consecutifs() {
    let b = banc("deux", true);
    let copie2 = second_poste(&b, "deux");
    let (s1, _e1) = demarrer(&b.copie);
    let (s2, _e2) = demarrer(&copie2);
    attendre("connexion du premier poste", || {
        s1.etat() == EtatConnexion::Connecte
    });
    attendre("connexion du second poste", || {
        s2.etat() == EtatConnexion::Connecte
    });
    s1.connecter("admin", MOT_DE_PASSE).unwrap();
    s2.connecter("admin", MOT_DE_PASSE).unwrap();

    // Client créé sur le premier poste, reçu par le second à la relève.
    let t = synchro::nouvel_uid();
    let issue = s1
        .ecrire(Action::CreerTiers {
            uid: t.clone(),
            fiche: fiche("Client Partagé"),
        })
        .unwrap();
    assert_eq!(issue, Issue::acceptee());
    attendre("client sur le second poste", || {
        present(&copie2, Entite::Tiers, &t)
    });
    let conn2 = poste_relie::ouvrir_copie(&copie2).unwrap();
    let id = synchro::id_de(&conn2, Entite::Tiers, &t).unwrap().unwrap();
    assert_eq!(
        tiers::lire(&conn2, id).unwrap().unwrap().fiche.nom,
        "Client Partagé"
    );
    drop(conn2);

    // Un brouillon de facture sur chaque poste.
    let (d1, d2) = (synchro::nouvel_uid(), synchro::nouvel_uid());
    for (s, d) in [(&s1, &d1), (&s2, &d2)] {
        let issue = s
            .ecrire(Action::CreerBrouillon {
                uid: d.clone(),
                type_document: "facture".into(),
                saisie: saisie(&t),
            })
            .unwrap();
        assert_eq!(issue, Issue::acceptee());
    }

    // Les deux validations partent en même temps.
    let depart = Arc::new(Barrier::new(2));
    let valider = |s: Synchroniseur, d: String| {
        let depart = depart.clone();
        thread::spawn(move || {
            depart.wait();
            let issue = s.ecrire(Action::Valider { uid: d });
            (s, issue)
        })
    };
    let f1 = valider(s1, d1.clone());
    let f2 = valider(s2, d2.clone());
    let (s1, i1) = f1.join().unwrap();
    let (s2, i2) = f2.join().unwrap();
    let numero_de = |i: Result<Issue, ErreurPoste>| match i {
        Ok(Issue::Acceptee {
            numero: Some(n), ..
        }) => n,
        autre => panic!("numéro attendu : {autre:?}"),
    };
    let (n1, n2) = (numero_de(i1), numero_de(i2));
    assert_ne!(n1, n2);
    assert_eq!(n1[..n1.len() - 4], n2[..n2.len() - 4], "même série");
    let rang = |n: &str| n[n.len() - 4..].parse::<u32>().unwrap();
    assert_eq!(rang(&n1).abs_diff(rang(&n2)), 1, "{n1} et {n2} consécutifs");

    // Chaque poste reçoit le numéro attribué sur l'autre.
    attendre("numéro du second sur le premier", || {
        numero(&b.copie, &d2).as_deref() == Some(n2.as_str())
    });
    attendre("numéro du premier sur le second", || {
        numero(&copie2, &d1).as_deref() == Some(n1.as_str())
    });
    s1.arreter();
    s2.arreter();
}

/// Serveur relancé sur le même dossier : sur un autre port (injoignable pour
/// le poste) avec `Some(0)`, ou sur son port d'origine.
fn relancer(dossier: &Path, port: u16) -> Serveur {
    for _ in 0..40 {
        match Serveur::demarrer(config(dossier, port)) {
            Ok(sv) => return sv,
            Err(_) => thread::sleep(Duration::from_millis(250)),
        }
    }
    panic!("serveur non relancé sur le port {port}");
}

fn nom_du_tiers(conn: &rusqlite::Connection, uid: &str) -> Option<String> {
    let id = synchro::id_de(conn, Entite::Tiers, uid).unwrap()?;
    Some(tiers::lire(conn, id).unwrap().unwrap().fiche.nom)
}

fn nom_dans_la_copie(copie: &Path, uid: &str) -> Option<String> {
    nom_du_tiers(&poste_relie::ouvrir_copie(copie).unwrap(), uid)
}

fn injoignable(s: &Synchroniseur) -> bool {
    matches!(
        s.etat(),
        EtatConnexion::Reconnexion { .. } | EtatConnexion::Injoignable
    )
}

/// Résultats appris plus tard (`Confirmee`), reçus jusqu'ici.
fn confirmees(evenements: &Receiver<Evenement>, vus: &RefCell<Vec<Evenement>>) -> Vec<Issue> {
    vus.borrow_mut().extend(evenements.try_iter());
    vus.borrow()
        .iter()
        .filter_map(|e| match e {
            Evenement::Confirmee { issue, .. } => Some(issue.clone()),
            _ => None,
        })
        .collect()
}

/// Jalon F6b : un compte connecté en local, serveur arrêté (décisions 48, 50
/// et 51), saisit hors connexion ; au retour du serveur, la file part dans
/// l'ordre et le brouillon noté se valide.
#[test]
fn ecritures_hors_connexion_envoyees_au_retour() {
    let b = banc("file", true);
    let port = b.serveur.adresse().port();
    {
        let (s, _e) = demarrer(&b.copie);
        attendre("connexion", || s.etat() == EtatConnexion::Connecte);
        s.connecter("admin", MOT_DE_PASSE).unwrap();
        attendre("compte dans la copie", || {
            present(&b.copie, Entite::Utilisateur, &b.admin)
        });
        s.arreter();
    }
    let dossier_serveur = b.dossier_serveur.clone();
    b.serveur.arreter();

    // Démarrage sans serveur : compte connecté en local.
    let (s, evenements) = demarrer(&b.copie);
    attendre("serveur injoignable", || injoignable(&s));
    assert_eq!(
        s.connecter("admin", MOT_DE_PASSE),
        Err(ErreurPoste::HorsConnexion)
    );
    let t = synchro::nouvel_uid();
    assert_eq!(
        s.ecrire(Action::CreerTiers {
            uid: t.clone(),
            fiche: fiche("Client Hors Ligne"),
        }),
        Ok(Issue::EnFile)
    );
    assert!(present(&b.copie, Entite::Tiers, &t));
    let d = synchro::nouvel_uid();
    assert_eq!(
        s.ecrire(Action::CreerBrouillon {
            uid: d.clone(),
            type_document: "facture".into(),
            saisie: saisie(&t),
        }),
        Ok(Issue::EnFile)
    );
    assert_eq!(s.file(), 2);

    // Refusée par la copie : rendue tout de suite, rien en file.
    let refus = s.ecrire(Action::ModifierTiers {
        uid: synchro::nouvel_uid(),
        fiche: fiche("Inconnu"),
    });
    assert!(matches!(refus, Ok(Issue::Refusee { .. })), "{refus:?}");
    // Validation, comptes et réglages attendent le serveur.
    assert_eq!(
        s.ecrire(Action::Valider { uid: d.clone() }),
        Err(ErreurPoste::HorsConnexion)
    );
    assert_eq!(
        s.ecrire(Action::CreerCompte {
            uid: synchro::nouvel_uid(),
            nom_complet: "Nouveau".into(),
            identifiant: "nouveau".into(),
            mot_de_passe: MOT_DE_PASSE.into(),
            role: "commercial".into(),
        }),
        Err(ErreurPoste::HorsConnexion)
    );
    assert_eq!(s.file(), 2);
    s.valider_au_retour(&d).unwrap();
    let a_valider =
        || poste_relie::a_valider(&poste_relie::ouvrir_copie(&b.copie).unwrap()).unwrap();
    assert_eq!(a_valider().len(), 1);

    // Retour du serveur : le compte se connecte, la file part.
    let serveur = relancer(&dossier_serveur, port);
    attendre("file envoyée", || s.file() == 0);
    let vus = RefCell::new(Vec::new());
    attendre("deux écritures confirmées", || {
        confirmees(&evenements, &vus).len() == 2
    });
    assert!(
        confirmees(&evenements, &vus)
            .iter()
            .all(|i| matches!(i, Issue::Acceptee { .. }))
    );
    assert!(vus.borrow().iter().any(|e| matches!(e, Evenement::File(2))));
    assert!(vus.borrow().iter().any(|e| matches!(e, Evenement::File(0))));
    assert!(
        vus.borrow()
            .iter()
            .any(|e| matches!(e, Evenement::AValider(1)))
    );
    let (t2, d2) = (t.clone(), d.clone());
    let (sur_serveur, heure) = serveur
        .base(move |c| {
            let heure: Option<String> = c
                .query_row(
                    "SELECT json_extract(contenu, '$.heure_poste') FROM operation
                     WHERE json_extract(contenu, '$.hors_connexion') = 1 LIMIT 1",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            (
                synchro::id_de(c, Entite::Tiers, &t2).unwrap().is_some()
                    && synchro::id_de(c, Entite::Document, &d2).unwrap().is_some(),
                heure,
            )
        })
        .unwrap();
    assert!(sur_serveur);
    assert!(heure.is_some(), "heure du poste gardée par le serveur");

    // Validation au retour : numéro attribué, brouillon hors de la liste.
    let issue = s.ecrire(Action::Valider { uid: d.clone() }).unwrap();
    assert!(
        matches!(
            &issue,
            Issue::Acceptee {
                numero: Some(_),
                ..
            }
        ),
        "{issue:?}"
    );
    assert!(a_valider().is_empty());
    s.arreter();
    serveur.arreter();
}

/// Stock (S1, `docs/spec-stock.md`) : la copie d'un poste ne crée aucun
/// mouvement, même hors connexion ; au retour du serveur, l'ajustement (avec
/// l'identifiant choisi par le poste) et la livraison sont créés par le
/// serveur seul, et la copie les reçoit une seule fois.
#[test]
fn stock_hors_connexion_calcule_par_le_serveur() {
    use kadenz_stockage::stock::{self, FiltreMouvements, NatureMouvement};

    let b = banc("stock", true);
    let port = b.serveur.adresse().port();
    let (p, l, m0) = (
        synchro::nouvel_uid(),
        synchro::nouvel_uid(),
        synchro::nouvel_uid(),
    );
    {
        let (s, _e) = demarrer(&b.copie);
        attendre("connexion", || s.etat() == EtatConnexion::Connecte);
        s.connecter("admin", MOT_DE_PASSE).unwrap();
        let ok = |action: Action| {
            let issue = s.ecrire(action.clone()).unwrap();
            assert!(
                matches!(issue, Issue::Acceptee { .. }),
                "{action:?} : {issue:?}"
            );
        };
        ok(Action::CreerProduit {
            uid: p.clone(),
            fiche: kadenz_protocole::Produit {
                designation: "Ciment 50 kg".into(),
                reference: "CIM".into(),
                prix: 6_500,
                sens: "les_deux".into(),
                suivi_stock: true,
                nature_comptable: Default::default(),
            },
        });
        let t = synchro::nouvel_uid();
        ok(Action::CreerTiers {
            uid: t.clone(),
            fiche: fiche("Client Stock"),
        });
        // 5 sacs à 100 en stock d'ouverture.
        ok(Action::AjusterStock {
            uid: m0.clone(),
            produit: p.clone(),
            quantite: 5_000,
            cout_unitaire: Some(100),
            motif: "Ouverture".into(),
        });
        // Bordereau vierge validé : 2 sacs à livrer.
        ok(Action::CreerBrouillon {
            uid: l.clone(),
            type_document: "livraison".into(),
            saisie: Saisie {
                lignes: vec![Ligne {
                    produit: Some(p.clone()),
                    reference: "CIM".into(),
                    designation: "Ciment 50 kg".into(),
                    prix: 6_500,
                    quantite: 2_000,
                    quantite_livree: 2_000,
                    ..Ligne::default()
                }],
                ..saisie(&t)
            },
        });
        ok(Action::Valider { uid: l.clone() });
        attendre("ouverture dans la copie", || {
            present(&b.copie, Entite::Mouvement, &m0)
        });
        s.arreter();
    }
    let dossier_serveur = b.dossier_serveur.clone();
    b.serveur.arreter();

    // Hors connexion : un sac cassé, puis la livraison confirmée.
    let (s, _evenements) = demarrer(&b.copie);
    attendre("serveur injoignable", || injoignable(&s));
    assert_eq!(
        s.connecter("admin", MOT_DE_PASSE),
        Err(ErreurPoste::HorsConnexion)
    );
    let m1 = synchro::nouvel_uid();
    assert_eq!(
        s.ecrire(Action::AjusterStock {
            uid: m1.clone(),
            produit: p.clone(),
            quantite: -1_000,
            cout_unitaire: None,
            motif: "Casse".into(),
        }),
        Ok(Issue::EnFile)
    );
    assert_eq!(
        s.ecrire(Action::ConfirmerLivraison { uid: l.clone() }),
        Ok(Issue::EnFile)
    );
    // L'inventaire attend le serveur.
    assert_eq!(
        s.ecrire(Action::SaisirInventaire {
            uid: synchro::nouvel_uid(),
            motif: "Inventaire".into(),
            lignes: Vec::new(),
        }),
        Err(ErreurPoste::HorsConnexion)
    );
    let copie = || poste_relie::ouvrir_copie(&b.copie).unwrap();
    let produit_copie = synchro::id_de(&copie(), Entite::Produit, &p)
        .unwrap()
        .unwrap();
    let tous = |conn: &rusqlite::Connection| {
        stock::mouvements(conn, &FiltreMouvements::default()).unwrap()
    };
    assert_eq!(tous(&copie()).len(), 1, "aucun mouvement créé par la copie");
    assert_eq!(
        stock::lire(&copie(), produit_copie)
            .unwrap()
            .unwrap()
            .quantite,
        5_000
    );
    assert_eq!(
        stock::ajustements_en_attente(&copie())
            .unwrap()
            .get(&produit_copie),
        Some(&1)
    );

    // Retour du serveur : un mouvement par écriture, reçu une seule fois.
    let serveur = relancer(&dossier_serveur, port);
    attendre("file envoyée", || s.file() == 0);
    attendre("mouvements reçus", || tous(&copie()).len() == 3);
    let m1_serveur = m1.clone();
    let (m1_cree, sur_serveur) = serveur
        .base(move |c| {
            (
                synchro::id_de(c, Entite::Mouvement, &m1_serveur)
                    .unwrap()
                    .is_some(),
                tous(c)
                    .iter()
                    .map(|m| (m.nature, m.quantite, m.valeur))
                    .collect::<Vec<_>>(),
            )
        })
        .unwrap();
    assert!(m1_cree, "mouvement du serveur avec l'identifiant du poste");
    assert_eq!(
        sur_serveur,
        [
            (NatureMouvement::Ajustement, 5_000, 500),
            (NatureMouvement::Ajustement, -1_000, -100),
            (NatureMouvement::Livraison, -2_000, -200),
        ]
    );
    let recus: Vec<_> = tous(&copie())
        .iter()
        .map(|m| (m.nature, m.quantite, m.valeur))
        .collect();
    assert_eq!(recus, sur_serveur);
    assert!(present(&b.copie, Entite::Mouvement, &m1));
    let s_copie = stock::lire(&copie(), produit_copie).unwrap().unwrap();
    assert_eq!((s_copie.quantite, s_copie.valeur), (2_000, 200));
    assert!(stock::ajustements_en_attente(&copie()).unwrap().is_empty());
    s.arreter();
    serveur.arreter();
}

/// Jalon F6b : une fiche modifiée hors connexion et sur le serveur pendant la
/// coupure ; au retour, la modification du poste est suspendue et la fiche
/// remise à la version du serveur, puis l'Admin garde la version du poste.
#[test]
fn conflit_hors_connexion_suspendu_puis_tranche() {
    let b = banc("conflit", true);
    let port = b.serveur.adresse().port();
    let (s, evenements) = demarrer(&b.copie);
    attendre("connexion", || s.etat() == EtatConnexion::Connecte);
    s.connecter("admin", MOT_DE_PASSE).unwrap();
    let t = synchro::nouvel_uid();
    assert_eq!(
        s.ecrire(Action::CreerTiers {
            uid: t.clone(),
            fiche: fiche("Client"),
        }),
        Ok(Issue::acceptee())
    );
    let dossier_serveur = b.dossier_serveur.clone();
    b.serveur.arreter();
    attendre("serveur injoignable", || injoignable(&s));

    let modifier = Action::ModifierTiers {
        uid: t.clone(),
        fiche: fiche("Version du poste"),
    };
    assert_eq!(s.ecrire(modifier.clone()), Ok(Issue::EnFile));
    assert_eq!(
        nom_dans_la_copie(&b.copie, &t).as_deref(),
        Some("Version du poste")
    );

    // Pendant la coupure, la fiche change sur la machine du serveur (relancé
    // sur un autre port, que le poste ne connaît pas).
    let ailleurs = relancer(&dossier_serveur, 0);
    let (t2, admin) = (t.clone(), b.admin.clone());
    ailleurs
        .base(move |c| {
            let qui = comptes::lire(
                c,
                synchro::id_de(c, Entite::Utilisateur, &admin)
                    .unwrap()
                    .unwrap(),
            )
            .unwrap()
            .unwrap();
            let id = synchro::id_de(c, Entite::Tiers, &t2).unwrap().unwrap();
            tiers::modifier(
                c,
                &qui,
                id,
                &FicheTiers {
                    nom: "Version du serveur".into(),
                    ..FicheTiers::vide(TypeTiers::Client)
                },
            )
            .unwrap();
        })
        .unwrap();
    ailleurs.arreter();

    // Retour : conflit, fiche remise à la version du serveur.
    let serveur = relancer(&dossier_serveur, port);
    let vus = RefCell::new(Vec::new());
    attendre("conflit confirmé", || {
        confirmees(&evenements, &vus).contains(&Issue::EnConflit)
    });
    attendre("version du serveur dans la copie", || {
        nom_dans_la_copie(&b.copie, &t).as_deref() == Some("Version du serveur")
    });
    assert_eq!(s.file(), 0, "une écriture suspendue n'est plus à envoyer");
    attendre("un conflit signalé", || {
        vus.borrow_mut().extend(evenements.try_iter());
        vus.borrow().iter().any(|e| {
            matches!(
                e,
                Evenement::Suspendues {
                    conflits: 1,
                    attentes: 0
                }
            )
        })
    });
    // Liste demandée sans attendre : elle arrive par un événement.
    s.demander_suspendues();
    attendre("liste reçue", || {
        vus.borrow_mut().extend(evenements.try_iter());
        vus.borrow()
            .iter()
            .any(|e| matches!(e, Evenement::ListeSuspendues(Ok(l)) if l.len() == 1))
    });
    let liste = s.suspendues().unwrap();
    assert_eq!(liste.len(), 1);
    assert_eq!(liste[0].motif, Motif::Conflit);
    assert!(liste[0].operation.hors_connexion);

    // L'Admin garde la version du poste : l'écriture suspendue est renvoyée
    // et acceptée.
    assert_eq!(
        s.ecrire(Action::TrancherConflit {
            operation: liste[0].operation.id.clone(),
            resultat: Some(Box::new(modifier)),
        }),
        Ok(Issue::acceptee())
    );
    attendre("version du poste dans la copie", || {
        nom_dans_la_copie(&b.copie, &t).as_deref() == Some("Version du poste")
    });
    attendre("décision confirmée", || {
        confirmees(&evenements, &vus)
            .last()
            .is_some_and(|i| matches!(i, Issue::Acceptee { .. }))
    });
    let conn = poste_relie::ouvrir_copie(&b.copie).unwrap();
    assert!(replique::file(&conn).unwrap().is_empty());
    assert!(s.suspendues().unwrap().is_empty());
    attendre("plus aucun conflit signalé", || {
        vus.borrow_mut().extend(evenements.try_iter());
        matches!(
            vus.borrow()
                .iter()
                .rev()
                .find(|e| matches!(e, Evenement::Suspendues { .. })),
            Some(Evenement::Suspendues {
                conflits: 0,
                attentes: 0
            })
        )
    });
    s.arreter();
    serveur.arreter();
}

/// Décision 52 : un compte connecté en local saisit hors connexion, puis il
/// est désactivé sur le serveur pendant la coupure. Au retour, sa connexion
/// est refusée ; ses écritures partent sous la session de l'Admin, en
/// attente, et l'Admin les accepte.
#[test]
fn compte_desactive_pendant_la_coupure_en_attente() {
    let b = banc("relaie", true);
    let port = b.serveur.adresse().port();
    let admin = b.admin.clone();
    let vente = b
        .serveur
        .base(move |c| {
            let qui = comptes::lire(
                c,
                synchro::id_de(c, Entite::Utilisateur, &admin)
                    .unwrap()
                    .unwrap(),
            )
            .unwrap()
            .unwrap();
            let u = comptes::creer(
                c,
                &qui,
                &NouvelUtilisateur {
                    nom_complet: "Vente".into(),
                    identifiant: "vente".into(),
                    mot_de_passe: MOT_DE_PASSE.into(),
                    role: Role::Commercial,
                },
            )
            .unwrap();
            synchro::completer_uids(c).unwrap();
            synchro::uid_de(c, Entite::Utilisateur, u.id)
                .unwrap()
                .unwrap()
        })
        .unwrap();
    {
        let (s, _e) = demarrer(&b.copie);
        attendre("connexion", || s.etat() == EtatConnexion::Connecte);
        s.connecter("admin", MOT_DE_PASSE).unwrap();
        attendre("compte du commercial dans la copie", || {
            present(&b.copie, Entite::Utilisateur, &vente)
        });
        s.arreter();
    }
    let dossier_serveur = b.dossier_serveur.clone();
    b.serveur.arreter();

    let (s, evenements) = demarrer(&b.copie);
    attendre("serveur injoignable", || injoignable(&s));
    assert_eq!(
        s.connecter("vente", MOT_DE_PASSE),
        Err(ErreurPoste::HorsConnexion)
    );
    let t = synchro::nouvel_uid();
    assert_eq!(
        s.ecrire(Action::CreerTiers {
            uid: t.clone(),
            fiche: fiche("Client du commercial"),
        }),
        Ok(Issue::EnFile)
    );

    // Pendant la coupure, l'Admin désactive le commercial sur le serveur.
    let ailleurs = relancer(&dossier_serveur, 0);
    let (admin, v) = (b.admin.clone(), vente.clone());
    ailleurs
        .base(move |c| {
            let qui = comptes::lire(
                c,
                synchro::id_de(c, Entite::Utilisateur, &admin)
                    .unwrap()
                    .unwrap(),
            )
            .unwrap()
            .unwrap();
            let id = synchro::id_de(c, Entite::Utilisateur, &v).unwrap().unwrap();
            comptes::definir_actif(c, &qui, id, false).unwrap();
        })
        .unwrap();
    ailleurs.arreter();

    // Retour : la connexion du commercial est refusée, rien n'est parti.
    let serveur = relancer(&dossier_serveur, port);
    let debut = Instant::now();
    let refus = loop {
        let e = evenements
            .recv_timeout(Duration::from_secs(20))
            .expect("événement Compte attendu");
        if let Evenement::Compte(r) = e {
            break r;
        }
        assert!(debut.elapsed() < Duration::from_secs(25));
    };
    assert!(
        matches!(refus, Err(ErreurPoste::CompteRefuse { .. })),
        "{refus:?}"
    );
    assert_eq!(s.file(), 1);

    // L'Admin se connecte sur ce poste : l'écriture part, en attente, et la
    // fiche provisoire quitte la copie.
    s.connecter("admin", MOT_DE_PASSE).unwrap();
    let vus = RefCell::new(Vec::new());
    attendre("attente confirmée", || {
        confirmees(&evenements, &vus).contains(&Issue::EnAttente)
    });
    attendre("fiche provisoire retirée", || {
        !present(&b.copie, Entite::Tiers, &t)
    });
    assert_eq!(s.file(), 0);
    attendre("une attente signalée à l'Admin", || {
        vus.borrow_mut().extend(evenements.try_iter());
        vus.borrow()
            .iter()
            .any(|e| matches!(e, Evenement::Suspendues { attentes: 1, .. }))
    });
    let liste = s.suspendues().unwrap();
    assert_eq!(liste.len(), 1);
    assert_eq!(liste[0].motif, Motif::Attente);
    assert_eq!(liste[0].operation.compte, vente);

    // Acceptée par l'Admin : la fiche revient, la file est vide.
    assert_eq!(
        s.ecrire(Action::DeciderAttente {
            operation: liste[0].operation.id.clone(),
            accepter: true,
        }),
        Ok(Issue::acceptee())
    );
    attendre("fiche acceptée dans la copie", || {
        present(&b.copie, Entite::Tiers, &t)
    });
    attendre("file vide", || {
        replique::file(&poste_relie::ouvrir_copie(&b.copie).unwrap())
            .unwrap()
            .is_empty()
    });
    s.arreter();
    serveur.arreter();
}

/// Essai F7a : un Admin reprend la base V1 de son poste dans un serveur neuf.
/// Le résumé lu sur le poste et le bilan du serveur portent les mêmes nombres,
/// la base V1 reste intacte, les lignes reprises arrivent dans la copie, et
/// une seconde reprise est refusée (décision 55).
#[test]
fn reprise_d_une_base_v1() {
    let b = banc("reprise", true);
    // Base V1 du poste, chiffrée par la clé du poste comme en V1.
    let v1 = dossier("reprise-v1").join("kadenz.sqlite");
    let client_v1 = {
        let mut conn = kadenz_stockage::ouvrir(&v1).unwrap();
        let admin = installation::installer(
            &mut conn,
            &InstallationInitiale {
                entreprise: Entreprise {
                    nom: "Quincaillerie V1".into(),
                    ..Default::default()
                },
                admin: NouvelUtilisateur {
                    nom_complet: "Patron V1".into(),
                    identifiant: "admin".into(),
                    mot_de_passe: "motdepasse-v1".into(),
                    role: Role::Admin,
                },
                parametres: Parametres::default(),
            },
        )
        .unwrap();
        comptes::creer(
            &conn,
            &admin,
            &NouvelUtilisateur {
                nom_complet: "Vendeur V1".into(),
                identifiant: "vente".into(),
                mot_de_passe: "motdepasse-vente".into(),
                role: Role::Commercial,
            },
        )
        .unwrap();
        let mut f = FicheTiers::vide(TypeTiers::Client);
        f.nom = "Client de la V1".into();
        let t = tiers::creer(&conn, &admin, &f).unwrap();
        synchro::completer_uids(&conn).unwrap();
        synchro::uid_de(&conn, Entite::Tiers, t.id)
            .unwrap()
            .unwrap()
    };
    let avant = std::fs::read(&v1).unwrap();

    let (s, _e) = demarrer(&b.copie);
    attendre("connexion", || s.etat() == EtatConnexion::Connecte);
    s.connecter("admin", MOT_DE_PASSE).unwrap();
    attendre("compte dans la copie", || {
        present(&b.copie, Entite::Utilisateur, &b.admin)
    });
    assert_eq!(s.reprise_possible(), Ok(true));

    let source = SourceV1::Base(v1.clone());
    let resume = s.resume_reprise(&source).unwrap();
    assert_eq!(resume.entreprise, "Quincaillerie V1");
    assert_eq!(resume.clients, 1);
    assert_eq!(resume.comptes_repris, ["vente"]);
    assert_eq!(resume.comptes_ecartes, ["admin"]);

    // La reprise passe par la poignée, depuis un autre fil que l'écran.
    let poignee = s.poignee();
    let envoi = source.clone();
    let bilan = thread::spawn(move || poignee.reprendre(&envoi))
        .join()
        .unwrap()
        .unwrap();
    assert_eq!(bilan, resume);
    // La base V1 n'est pas modifiée, la sauvegarde envoyée est effacée.
    assert_eq!(std::fs::read(&v1).unwrap(), avant);
    assert!(!b.copie.with_file_name("reprise-v1.kzs").exists());
    attendre("client de la V1 dans la copie", || {
        present(&b.copie, Entite::Tiers, &client_v1)
    });

    assert_eq!(s.reprise_possible(), Ok(false));
    match s.reprendre(&source) {
        Err(ErreurPoste::Refus { modele, .. }) => assert_eq!(modele, MODELE_PAS_NEUF),
        autre => panic!("refus attendu : {autre:?}"),
    }
    // Mot de passe faux d'une sauvegarde : refus à traduire, lu sur le poste.
    let faux = SourceV1::Sauvegarde {
        chemin: v1.clone(),
        mot_de_passe: "pas-le-bon-mot".into(),
    };
    assert!(matches!(
        s.resume_reprise(&faux),
        Err(ErreurPoste::Refus { .. })
    ));
    s.arreter();
}

#[test]
fn avertissement_des_sauvegardes_pour_un_admin() {
    let b = banc("sauvegardes", true);
    let (s, e) = demarrer(&b.copie);
    let vus = RefCell::new(Vec::new());
    let dernier = |vus: &RefCell<Vec<Evenement>>| {
        vus.borrow_mut().extend(e.try_iter());
        vus.borrow().iter().rev().find_map(|ev| match ev {
            Evenement::Sauvegarde(a) => Some(*a),
            _ => None,
        })
    };
    attendre("connexion", || s.etat() == EtatConnexion::Connecte);
    // Sans compte connecté, rien n'est signalé.
    assert_eq!(dernier(&vus), None);
    s.connecter("admin", MOT_DE_PASSE).unwrap();
    // Serveur sans mot de passe des sauvegardes : bandeau permanent.
    attendre("avertissement sans mot de passe", || {
        dernier(&vus) == Some(Some(AvertissementSauvegarde::SansMotDePasse))
    });
    // Mot de passe réglé, dossier sur le même disque que les données.
    let a_cote = b.dossier_serveur.with_file_name(format!(
        "kadenz-poste-essai-sauvegardes-{}",
        std::process::id()
    ));
    sauvegardes::enregistrer_reglage(
        &b.dossier_serveur,
        &a_cote,
        chrono::NaiveTime::from_hms_opt(7, 30, 0).unwrap(),
        Some("motdepasse-sauvegardes"),
    )
    .unwrap();
    s.deconnecter();
    vus.borrow_mut().clear();
    s.connecter("admin", MOT_DE_PASSE).unwrap();
    attendre("avertissement même disque", || {
        dernier(&vus) == Some(Some(AvertissementSauvegarde::MemeDisque))
    });
    s.arreter();
}

#[test]
fn copie_refaite_apres_une_restauration_du_serveur() {
    const MDP: &str = "motdepasse-sauvegardes";
    let Banc {
        serveur,
        dossier_serveur,
        copie,
        admin: uid_admin,
        ..
    } = banc("restauration", true);
    let a_cote = dossier_serveur.with_file_name(format!(
        "kadenz-poste-essai-restauration-sauvegardes-{}",
        std::process::id()
    ));
    sauvegardes::enregistrer_reglage(
        &dossier_serveur,
        &a_cote,
        chrono::NaiveTime::from_hms_opt(7, 30, 0).unwrap(),
        Some(MDP),
    )
    .unwrap();
    let (s, _e) = demarrer(&copie);
    attendre("connexion", || s.etat() == EtatConnexion::Connecte);
    s.connecter("admin", MOT_DE_PASSE).unwrap();
    attendre("compte dans la copie", || {
        present(&copie, Entite::Utilisateur, &uid_admin)
    });
    let creer = |s: &Synchroniseur, nom: &str| {
        let uid = synchro::nouvel_uid();
        let issue = s
            .ecrire(Action::CreerTiers {
                uid: uid.clone(),
                fiche: fiche(nom),
            })
            .unwrap();
        assert_eq!(issue, Issue::acceptee());
        uid
    };
    let garde = creer(&s, "Client gardé");
    let d = dossier_serveur.clone();
    let sauvegarde = serveur
        .base(move |c| {
            sauvegardes::sauvegarder(c, &d, kadenz_stockage::sauvegarde::Genre::Manuelle)
        })
        .unwrap()
        .unwrap();
    let perdu = creer(&s, "Client perdu");
    assert!(present(&copie, Entite::Tiers, &perdu));
    s.arreter();
    let port = serveur.adresse().port();
    serveur.arreter();

    let preparee = admin::preparer_restauration(&dossier_serveur, &sauvegarde, MDP).unwrap();
    admin::restaurer_si(&dossier_serveur, &preparee, EtatService::NonInstalle).unwrap();
    drop(preparee);
    let _serveur = relancer(&dossier_serveur, port);

    // Le poste voit une autre génération : il refait sa copie, le tiers créé
    // après la sauvegarde en est retiré.
    let (s, _e) = demarrer(&copie);
    attendre("connexion", || s.etat() == EtatConnexion::Connecte);
    s.connecter("admin", MOT_DE_PASSE).unwrap();
    attendre("tiers perdu retiré de la copie", || {
        !present(&copie, Entite::Tiers, &perdu)
    });
    assert!(present(&copie, Entite::Tiers, &garde));
    let nouveau = creer(&s, "Client après la restauration");
    assert!(present(&copie, Entite::Tiers, &nouveau));
    s.arreter();
    let _ = std::fs::remove_dir_all(&a_cote);
}

#[test]
fn poste_hors_du_bureau_par_le_relais() {
    let b = banc("distance", true);
    let dossier_relais = dossier("distance-relais");
    let mut config_relais = kadenz_relais::relais::Config::nouvelle(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        dossier_relais.clone(),
        vec![cle_portail().verifying_key()],
    );
    config_relais.traces = None;
    let relais = kadenz_relais::relais::Relais::demarrer(config_relais).unwrap();
    let adresse_relais = relais.adresse().to_string();
    let empreinte_relais = relais.empreinte().to_string();
    let empreinte_poste = b.identite.empreinte();
    {
        let adresse = adresse_relais.clone();
        b.serveur
            .base(move |c| {
                admin::regler_relais(c, Some((&adresse, &empreinte_relais))).unwrap();
                kadenz_stockage::distance::autoriser(c, &empreinte_poste, true, None).unwrap();
                kadenz_stockage::distance::activer(c, true, None).unwrap();
            })
            .unwrap();
    }
    attendre("serveur enregistré", || !relais.entreprises().is_empty());

    // Au bureau : session directe ; le serveur donne l'adresse du relais.
    let (s, _e) = demarrer(&b.copie);
    attendre("connexion directe", || s.etat() == EtatConnexion::Connecte);
    assert!(!s.distant());
    s.connecter("admin", MOT_DE_PASSE).unwrap();
    attendre("relais appris", || {
        let conn = poste_relie::ouvrir_copie(&b.copie).unwrap();
        kadenz_stockage::distance::relais_du_poste(&conn).unwrap() == Some(adresse_relais.clone())
    });
    s.arreter();

    // Hors du bureau : l'adresse du serveur ne répond plus, le poste passe
    // par le relais.
    {
        let conn = poste_relie::ouvrir_copie(&b.copie).unwrap();
        let mut poste = poste_relie::lire(&conn).unwrap().unwrap();
        poste.adresse = "127.0.0.1:1".into();
        poste_relie::garder(&conn, &poste).unwrap();
    }
    let (s, _e) = demarrer(&b.copie);
    attendre("connexion par le relais", || {
        s.etat() == EtatConnexion::Connecte && s.distant()
    });
    s.connecter("admin", MOT_DE_PASSE).unwrap();
    let issue = s
        .ecrire(Action::CreerTiers {
            uid: synchro::nouvel_uid(),
            fiche: fiche("Client à distance"),
        })
        .unwrap();
    assert!(matches!(issue, Issue::Acceptee { .. }), "{issue:?}");

    // Serveur coupé puis rallumé : le poste distant passe hors connexion,
    // puis revient par le relais.
    let Banc {
        serveur,
        dossier_serveur,
        copie: b_copie,
        ..
    } = b;
    serveur.arreter();
    attendre("hors connexion", || injoignable(&s));
    assert!(!s.distant());
    let serveur = relancer(&dossier_serveur, 0);
    attendre("retour par le relais", || {
        s.etat() == EtatConnexion::Connecte && s.distant()
    });
    s.arreter();

    // Revenu au bureau : l'adresse du serveur répond de nouveau, le poste ne
    // passe plus par le relais.
    {
        let conn = poste_relie::ouvrir_copie(&b_copie).unwrap();
        let mut poste = poste_relie::lire(&conn).unwrap().unwrap();
        poste.adresse = serveur.adresse().to_string();
        poste_relie::garder(&conn, &poste).unwrap();
    }
    let (s, _e) = demarrer(&b_copie);
    attendre("connexion directe au retour", || {
        s.etat() == EtatConnexion::Connecte
    });
    assert!(!s.distant());
    s.arreter();
    serveur.arreter();
    relais.arreter();
    let _ = std::fs::remove_dir_all(dossier_relais);
}

/// Mandataire TCP qu'on peut couper, devant la passerelle WebSocket du
/// relais : simule une coupure côté Cloudflare (déploiement, cloudflared
/// arrêté), qui ferme d'un coup toutes les WebSocket en cours.
struct Coupeur {
    adresse: SocketAddr,
    ouvertes: Arc<Mutex<Vec<std::net::TcpStream>>>,
}

impl Coupeur {
    fn lancer(cible: SocketAddr) -> Coupeur {
        let ecoute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let adresse = ecoute.local_addr().unwrap();
        let ouvertes = Arc::new(Mutex::new(Vec::new()));
        let liste = ouvertes.clone();
        thread::spawn(move || {
            for flux in ecoute.incoming() {
                let Ok(entree) = flux else { continue };
                let Ok(sortie) = std::net::TcpStream::connect(cible) else {
                    continue;
                };
                let (e2, s2) = (entree.try_clone().unwrap(), sortie.try_clone().unwrap());
                liste
                    .lock()
                    .unwrap()
                    .extend([entree.try_clone().unwrap(), sortie.try_clone().unwrap()]);
                thread::spawn(move || copier_flux(entree, s2));
                thread::spawn(move || copier_flux(sortie, e2));
            }
        });
        Coupeur { adresse, ouvertes }
    }

    /// Coupe toutes les connexions en cours ; rend leur nombre.
    fn couper(ouvertes: &Mutex<Vec<std::net::TcpStream>>) -> usize {
        let mut liste = ouvertes.lock().unwrap();
        let nombre = liste.len() / 2;
        for t in liste.drain(..) {
            let _ = t.shutdown(std::net::Shutdown::Both);
        }
        nombre
    }
}

fn copier_flux(mut de: std::net::TcpStream, mut vers: std::net::TcpStream) {
    let _ = std::io::copy(&mut de, &mut vers);
    let _ = vers.shutdown(std::net::Shutdown::Write);
}

/// Relais par WebSocket (docs/spec-relais-websocket.md, section 7) : toutes
/// les WebSocket sont coupées en pleine série d'écritures d'un poste distant
/// (contrôle du serveur et session du poste). Le serveur se réenregistre, le
/// poste revient par le relais et vide sa file : chaque écriture arrive au
/// serveur une fois, ni perdue ni doublée.
#[test]
fn coupure_des_websocket_en_pleine_session() {
    let b = banc("ws-coupure", true);
    let dossier_relais = dossier("ws-coupure-relais");
    let mut config_relais = kadenz_relais::relais::Config::nouvelle(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        dossier_relais.clone(),
        vec![cle_portail().verifying_key()],
    );
    config_relais.traces = None;
    config_relais.ecoute_ws = Some(SocketAddr::from(([127, 0, 0, 1], 0)));
    let relais = kadenz_relais::relais::Relais::demarrer(config_relais).unwrap();
    let coupeur = Coupeur::lancer(relais.adresse_ws().unwrap());
    let adresse_relais = format!("ws://{}", coupeur.adresse);
    let empreinte_relais = relais.empreinte().to_string();
    let empreinte_poste = b.identite.empreinte();
    {
        let adresse = adresse_relais.clone();
        b.serveur
            .base(move |c| {
                admin::regler_relais(c, Some((&adresse, &empreinte_relais))).unwrap();
                kadenz_stockage::distance::autoriser(c, &empreinte_poste, true, None).unwrap();
                kadenz_stockage::distance::activer(c, true, None).unwrap();
            })
            .unwrap();
    }
    attendre("serveur enregistré", || !relais.entreprises().is_empty());

    // Au bureau, le poste apprend l'adresse du relais ; puis il en sort.
    let (s, _e) = demarrer(&b.copie);
    attendre("connexion directe", || s.etat() == EtatConnexion::Connecte);
    s.connecter("admin", MOT_DE_PASSE).unwrap();
    attendre("relais appris", || {
        let conn = poste_relie::ouvrir_copie(&b.copie).unwrap();
        kadenz_stockage::distance::relais_du_poste(&conn).unwrap() == Some(adresse_relais.clone())
    });
    s.arreter();
    {
        let conn = poste_relie::ouvrir_copie(&b.copie).unwrap();
        let mut poste = poste_relie::lire(&conn).unwrap().unwrap();
        poste.adresse = "127.0.0.1:1".into();
        poste_relie::garder(&conn, &poste).unwrap();
    }
    let (s, _e) = demarrer(&b.copie);
    attendre("connexion par le relais", || {
        s.etat() == EtatConnexion::Connecte && s.distant()
    });
    s.connecter("admin", MOT_DE_PASSE).unwrap();

    // Écritures en série ; la coupure part dès la dixième rendue, pendant la
    // onzième.
    let ouvertes = coupeur.ouvertes.clone();
    let faites = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let coupure = {
        let faites = faites.clone();
        thread::spawn(move || {
            while faites.load(std::sync::atomic::Ordering::SeqCst) < 10 {
                thread::sleep(Duration::from_millis(1));
            }
            Coupeur::couper(&ouvertes)
        })
    };
    let mut uids = Vec::new();
    for i in 0..30 {
        let uid = synchro::nouvel_uid();
        let issue = s
            .ecrire(Action::CreerTiers {
                uid: uid.clone(),
                fiche: fiche(&format!("Client coupure {i:02}")),
            })
            .unwrap();
        assert!(
            matches!(issue, Issue::Acceptee { .. } | Issue::EnFile),
            "{issue:?}"
        );
        uids.push(uid);
        faites.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
    assert!(coupure.join().unwrap() >= 2, "rien n'a été coupé");

    attendre("serveur réenregistré", || {
        !relais.entreprises().is_empty()
    });
    attendre("retour par le relais", || {
        s.etat() == EtatConnexion::Connecte && s.distant()
    });
    assert!(s.vider(Duration::from_secs(30)));
    attendre("file envoyée", || s.file() == 0);

    let (comptes, presents) = b
        .serveur
        .base(move |c| {
            let mut requete = c
                .prepare(
                    "SELECT nom, COUNT(*) FROM tiers WHERE nom LIKE 'Client coupure %' GROUP BY nom",
                )
                .unwrap();
            let comptes: Vec<(String, i64)> = requete
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
                .unwrap()
                .map(Result::unwrap)
                .collect();
            let presents = uids
                .iter()
                .filter(|u| synchro::id_de(c, Entite::Tiers, u).unwrap().is_some())
                .count();
            (comptes, presents)
        })
        .unwrap();
    assert_eq!(presents, 30, "écritures perdues");
    assert_eq!(comptes.len(), 30, "{comptes:?}");
    assert!(
        comptes.iter().all(|(_, n)| *n == 1),
        "doublées : {comptes:?}"
    );

    s.arreter();
    b.serveur.arreter();
    relais.arreter();
    let _ = std::fs::remove_dir_all(dossier_relais);
}
