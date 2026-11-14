//! Essais de bout en bout du jalon F5c : un serveur réel sur 127.0.0.1, la
//! copie d'un poste et son synchroniseur.

use std::cell::RefCell;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

use kadenz_client::Identite;
use kadenz_domaine::module::Module;
use kadenz_domaine::referentiel::{FicheTiers, TypeTiers};
use kadenz_domaine::role::Role;
use kadenz_licence::SigningKey;
use kadenz_licence::entreprise::{self as lic, LicenceEntreprise};
use kadenz_poste::{ErreurPoste, EtatConnexion, Evenement, Synchroniseur};
use kadenz_protocole::{Action, Entite, Issue, Ligne, Motif, Operation, Saisie, Tiers};
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

    let serveur = relancer(&dossier_serveur, port);
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
