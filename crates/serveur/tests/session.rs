//! Essais de bout en bout du jalon F2 : un serveur réel sur 127.0.0.1, des
//! postes qui s'y connectent avec `kadenz-client`.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use kadenz_client::{Client, ErreurClient, Identite};
use kadenz_domaine::role::Role;
use kadenz_protocole::messages::{self, Reponse, TAILLE_MAX};
use kadenz_protocole::{Action, Etat, Issue, Operation, Tiers, VERSION};
use kadenz_serveur::{Config, Serveur};
use kadenz_stockage::comptes::NouvelUtilisateur;
use kadenz_stockage::entreprise::Entreprise;
use kadenz_stockage::installation::{self, InstallationInitiale};
use kadenz_stockage::parametres::Parametres;
use kadenz_stockage::{postes, synchro};

const DELAI_CLIENT: Duration = Duration::from_secs(5);
const MOT_DE_PASSE: &str = "motdepasse1";

fn dossier(nom: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("kadenz-serveur-essai-{nom}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn demarrer(nom: &str, delai: Duration, connexions_max: usize) -> Serveur {
    let mut config = Config::nouvelle(dossier(nom));
    config.adresse = SocketAddr::from(([127, 0, 0, 1], 0));
    config.delai_lecture = delai;
    config.connexions_max = connexions_max;
    // Essai sous le compte courant, sans droits d'administrateur.
    config.controle_droits = false;
    Serveur::demarrer(config).unwrap()
}

/// Entreprise installée avec son administrateur ; rend l'identifiant de
/// l'entreprise et l'`uid` du compte.
fn installer(serveur: &Serveur) -> (String, String) {
    serveur
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
            let uid = synchro::uid_de(c, kadenz_protocole::Entite::Utilisateur, admin.id)
                .unwrap()
                .unwrap();
            (synchro::identifiant_entreprise(c).unwrap(), uid)
        })
        .unwrap()
}

fn poste_inscrit(serveur: &Serveur) -> Identite {
    let identite = Identite::nouvelle("poste").unwrap();
    let empreinte = identite.empreinte();
    serveur
        .base(move |c| postes::inscrire(c, &empreinte, "Poste d'essai").unwrap())
        .unwrap();
    identite
}

fn connecter(serveur: &Serveur, identite: &Identite) -> Result<Client, ErreurClient> {
    Client::connecter(
        &serveur.adresse().to_string(),
        serveur.empreinte(),
        identite,
        DELAI_CLIENT,
    )
    .map(|(client, _)| client)
}

fn creer_tiers(entreprise: &str, poste: &str, compte: &str, nom: &str) -> Operation {
    Operation {
        id: synchro::nouvel_uid(),
        entreprise: entreprise.into(),
        poste: poste.into(),
        compte: compte.into(),
        hors_connexion: false,
        base: 0,
        action: Action::CreerTiers {
            uid: synchro::nouvel_uid(),
            fiche: Tiers {
                nom: nom.into(),
                type_tiers: "client".into(),
                ..Tiers::default()
            },
        },
    }
}

#[test]
fn session_complete() {
    let serveur = demarrer("session", DELAI_CLIENT, 8);
    let (entreprise, admin) = installer(&serveur);
    let poste = poste_inscrit(&serveur);
    let (mut client, bienvenue) = Client::connecter(
        &serveur.adresse().to_string(),
        serveur.empreinte(),
        &poste,
        DELAI_CLIENT,
    )
    .unwrap();
    assert_eq!(bienvenue.entreprise, entreprise);

    // Rien avant la connexion d'un compte.
    let op = creer_tiers(&entreprise, &poste.empreinte(), &admin, "Ouédraogo");
    assert!(matches!(client.envoyer(&op), Err(ErreurClient::Rejet(_))));
    assert!(matches!(
        client.changements(0, 10),
        Err(ErreurClient::Rejet(_))
    ));

    let connecte = client.connexion("admin", MOT_DE_PASSE).unwrap();
    assert_eq!(connecte.compte, admin);
    assert_eq!(connecte.nom_complet, "Administrateur");

    let issue = client.envoyer(&op).unwrap();
    assert!(matches!(issue, Issue::Acceptee { .. }), "{issue:?}");
    // Renvoyée après une coupure : même résultat, appliquée une seule fois.
    assert_eq!(client.envoyer(&op).unwrap(), issue);

    // Poste ou compte de l'opération autres que ceux de la session : refus.
    let autre_poste = creer_tiers(&entreprise, "autre", &admin, "Sawadogo");
    assert!(matches!(
        client.envoyer(&autre_poste),
        Err(ErreurClient::Rejet(_))
    ));
    let autre_compte = creer_tiers(&entreprise, &poste.empreinte(), "autre", "Sawadogo");
    assert!(matches!(
        client.envoyer(&autre_compte),
        Err(ErreurClient::Rejet(_))
    ));

    let (changements, dernier) = client.changements(0, 100).unwrap();
    assert!(dernier >= 1);
    let tiers: Vec<_> = changements
        .iter()
        .filter_map(|c| match &c.etat {
            Etat::Tiers { fiche, .. } => Some(fiche.nom.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(tiers, vec!["Ouédraogo"]);
}

#[test]
fn certificat_du_serveur_different() {
    let serveur = demarrer("certificat", DELAI_CLIENT, 8);
    installer(&serveur);
    let poste = poste_inscrit(&serveur);
    let faux = Identite::nouvelle("autre").unwrap().empreinte();
    let resultat = Client::connecter(&serveur.adresse().to_string(), &faux, &poste, DELAI_CLIENT);
    assert!(matches!(resultat, Err(ErreurClient::CertificatDifferent)));
}

#[test]
fn poste_non_inscrit_ou_retire() {
    let serveur = demarrer("inscription", DELAI_CLIENT, 8);
    installer(&serveur);
    let inconnu = Identite::nouvelle("poste").unwrap();
    assert!(matches!(
        connecter(&serveur, &inconnu),
        Err(ErreurClient::PosteNonInscrit)
    ));
    let poste = poste_inscrit(&serveur);
    assert!(connecter(&serveur, &poste).is_ok());
    let empreinte = poste.empreinte();
    serveur
        .base(move |c| postes::retirer(c, &empreinte).unwrap())
        .unwrap();
    assert!(matches!(
        connecter(&serveur, &poste),
        Err(ErreurClient::PosteNonInscrit)
    ));
}

#[test]
fn autre_version_du_protocole() {
    let serveur = demarrer("version", DELAI_CLIENT, 8);
    installer(&serveur);
    let poste = poste_inscrit(&serveur);
    let resultat = Client::connecter_en_version(
        &serveur.adresse().to_string(),
        serveur.empreinte(),
        &poste,
        DELAI_CLIENT,
        VERSION + 1,
    );
    assert!(matches!(
        resultat,
        Err(ErreurClient::VersionDifferente { serveur: v }) if v == VERSION
    ));
}

#[test]
fn message_trop_grand_refuse() {
    let serveur = demarrer("taille", DELAI_CLIENT, 8);
    installer(&serveur);
    let poste = poste_inscrit(&serveur);
    let mut client = connecter(&serveur, &poste).unwrap();
    let flux = client.flux();
    flux.write_all(&((TAILLE_MAX + 1) as u32).to_be_bytes())
        .unwrap();
    flux.flush().unwrap();
    match messages::lire::<Reponse>(flux).unwrap() {
        Reponse::Rejet { motif } => assert!(motif.contains("trop grand"), "{motif}"),
        autre => panic!("{autre:?}"),
    }
    // La session est close.
    assert!(client.changements(0, 10).is_err());
}

#[test]
fn delai_de_lecture() {
    let delai = Duration::from_millis(300);
    let serveur = demarrer("delai", delai, 8);
    installer(&serveur);
    let poste = poste_inscrit(&serveur);

    // Connexion TCP muette : fermée par le serveur au bout du délai.
    let mut muette = TcpStream::connect(serveur.adresse()).unwrap();
    muette
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let debut = Instant::now();
    let mut octet = [0u8; 1];
    let lu = muette.read(&mut octet);
    assert!(matches!(lu, Ok(0) | Err(_)), "{lu:?}");
    assert!(debut.elapsed() < Duration::from_secs(4));

    // Session ouverte puis inactive : fermée aussi.
    let mut client = connecter(&serveur, &poste).unwrap();
    thread::sleep(delai * 3);
    assert!(matches!(
        client.changements(0, 10),
        Err(ErreurClient::Injoignable(_))
    ));
}

#[test]
fn attente_apres_des_echecs_de_connexion() {
    let serveur = demarrer("tentatives", DELAI_CLIENT, 8);
    installer(&serveur);
    let poste = poste_inscrit(&serveur);
    let mut client = connecter(&serveur, &poste).unwrap();
    let mut attentes = Vec::new();
    for _ in 0..3 {
        match client.connexion("admin", "mauvais") {
            Err(ErreurClient::CompteRefuse { attente_secondes }) => attentes.push(attente_secondes),
            autre => panic!("{autre:?}"),
        }
    }
    assert_eq!(attentes, vec![0, 0, 5]);
    // Pendant l'attente, même le bon mot de passe est refusé sans être vérifié.
    assert!(matches!(
        client.connexion("admin", MOT_DE_PASSE),
        Err(ErreurClient::CompteRefuse { attente_secondes }) if attente_secondes > 0
    ));
    // Le même compte depuis un autre poste attend aussi.
    let autre = poste_inscrit(&serveur);
    let mut client = connecter(&serveur, &autre).unwrap();
    assert!(matches!(
        client.connexion("admin", MOT_DE_PASSE),
        Err(ErreurClient::CompteRefuse { .. })
    ));
}

#[test]
fn nombre_de_connexions_limite() {
    let serveur = demarrer("connexions", DELAI_CLIENT, 2);
    installer(&serveur);
    let poste = poste_inscrit(&serveur);
    let a = TcpStream::connect(serveur.adresse()).unwrap();
    let _b = TcpStream::connect(serveur.adresse()).unwrap();
    // Laisse le serveur accepter les deux premières.
    thread::sleep(Duration::from_millis(200));
    assert!(matches!(
        connecter(&serveur, &poste),
        Err(ErreurClient::Injoignable(_))
    ));
    // Une place libérée sert à la connexion suivante.
    drop(a);
    let fin = Instant::now() + Duration::from_secs(5);
    loop {
        match connecter(&serveur, &poste) {
            Ok(_) => break,
            Err(e) if Instant::now() > fin => panic!("{e}"),
            Err(_) => thread::sleep(Duration::from_millis(50)),
        }
    }
}

#[test]
fn redemarrage_garde_identite_et_base() {
    let nom = "redemarrage";
    let serveur = demarrer(nom, DELAI_CLIENT, 8);
    let (entreprise, _) = installer(&serveur);
    let poste = poste_inscrit(&serveur);
    let empreinte = serveur.empreinte().to_string();
    let adresse = serveur.adresse();
    serveur.arreter();
    assert!(TcpStream::connect_timeout(&adresse, Duration::from_secs(1)).is_err());

    let mut config = Config::nouvelle(
        std::env::temp_dir().join(format!("kadenz-serveur-essai-{nom}-{}", std::process::id())),
    );
    config.adresse = SocketAddr::from(([127, 0, 0, 1], 0));
    config.controle_droits = false;
    let serveur = Serveur::demarrer(config).unwrap();
    assert_eq!(serveur.empreinte(), empreinte);
    assert_eq!(
        kadenz_serveur::empreinte_du_dossier(
            &std::env::temp_dir()
                .join(format!("kadenz-serveur-essai-{nom}-{}", std::process::id()))
        )
        .unwrap(),
        empreinte
    );
    let (_, bienvenue) = Client::connecter(
        &serveur.adresse().to_string(),
        &empreinte,
        &poste,
        DELAI_CLIENT,
    )
    .unwrap();
    assert_eq!(bienvenue.entreprise, entreprise);
}
