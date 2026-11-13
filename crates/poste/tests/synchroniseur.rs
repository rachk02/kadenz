//! Essais de bout en bout du jalon F5c : un serveur réel sur 127.0.0.1, la
//! copie d'un poste et son synchroniseur.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use kadenz_client::Identite;
use kadenz_domaine::referentiel::{FicheTiers, TypeTiers};
use kadenz_domaine::role::Role;
use kadenz_licence::SigningKey;
use kadenz_licence::entreprise::{self as lic, LicenceEntreprise};
use kadenz_poste::{ErreurPoste, EtatConnexion, Evenement, Synchroniseur};
use kadenz_protocole::{Action, Entite, Issue, Operation, Tiers};
use kadenz_serveur::{Config, Serveur};
use kadenz_stockage::comptes::{self, NouvelUtilisateur};
use kadenz_stockage::entreprise::Entreprise;
use kadenz_stockage::installation::{self, InstallationInitiale};
use kadenz_stockage::parametres::Parametres;
use kadenz_stockage::poste_relie::{self, PosteRelie};
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
            modules: Vec::new(),
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
