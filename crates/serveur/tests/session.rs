//! Essais de bout en bout des jalons F2 à F4 : un serveur réel sur
//! 127.0.0.1, des postes qui s'y connectent avec `kadenz-client`.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use kadenz_client::{Client, ErreurClient, Identite, Inscrit};
use kadenz_domaine::role::Role;
use kadenz_licence::SigningKey;
use kadenz_licence::entreprise::{self as lic, LicenceEntreprise};
use kadenz_protocole::abonnement::Statut;
use kadenz_protocole::inscription::{CodeComplet, presenter_secret};
use kadenz_protocole::messages::{self, Reponse, TAILLE_MAX};
use kadenz_protocole::{Action, Entite, Etat, Issue, Operation, Tiers, VERSION};
use kadenz_serveur::{Config, Serveur, admin};
use kadenz_stockage::comptes::NouvelUtilisateur;
use kadenz_stockage::entreprise::Entreprise;
use kadenz_stockage::installation::{self, InstallationInitiale};
use kadenz_stockage::parametres::Parametres;
use kadenz_stockage::{abonnement, comptes, inscription, postes, synchro};

const DELAI_CLIENT: Duration = Duration::from_secs(5);
const MOT_DE_PASSE: &str = "motdepasse1";

/// Clé du portail des essais, à la place de la clé embarquée.
fn cle_portail() -> SigningKey {
    SigningKey::from_bytes(&[5u8; 32])
}

/// Licence d'abonnement signée par la clé des essais ; dates en jours depuis
/// aujourd'hui.
fn licence(entreprise: &str, echeance: i64, emise_le: i64) -> String {
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
            debut: jour(emise_le),
            echeance: jour(echeance),
            emetteur: lic::Emetteur::Editeur,
            emise_le: jour(emise_le),
        },
        &cle_portail(),
    )
    .unwrap()
}

fn chemin(nom: &str) -> PathBuf {
    std::env::temp_dir().join(format!("kadenz-serveur-essai-{nom}-{}", std::process::id()))
}

fn dossier(nom: &str) -> PathBuf {
    let d = chemin(nom);
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
    config.cles_portail = vec![cle_portail().verifying_key()];
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
            let entreprise = synchro::identifiant_entreprise(c).unwrap();
            // Abonnement d'un an : sans licence, le serveur est en lecture seule.
            abonnement::enregistrer_avec(
                c,
                &admin,
                &licence(&entreprise, 365, 0),
                abonnement::horloge(),
                &[cle_portail().verifying_key()],
            )
            .unwrap();
            (entreprise, uid)
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
            Etat::Ligne { valeurs, .. } if c.entite == Entite::Tiers => valeurs["nom"].as_str(),
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
    // Un autre poste n'est pas freiné : le poste fautif ne bloque pas le compte.
    let autre = poste_inscrit(&serveur);
    let mut client = connecter(&serveur, &autre).unwrap();
    assert!(client.connexion("admin", MOT_DE_PASSE).is_ok());
}

#[test]
fn sessions_paralleles_sans_essais_en_plus() {
    let serveur = demarrer("paralleles", DELAI_CLIENT, 16);
    installer(&serveur);
    let poste = poste_inscrit(&serveur);
    let clients: Vec<Client> = (0..10)
        .map(|_| connecter(&serveur, &poste).unwrap())
        .collect();
    let fils: Vec<_> = clients
        .into_iter()
        .map(|mut client| {
            thread::spawn(move || match client.connexion("admin", "mauvais") {
                Err(ErreurClient::CompteRefuse { attente_secondes }) => attente_secondes,
                autre => panic!("{autre:?}"),
            })
        })
        .collect();
    let attentes: Vec<u64> = fils.into_iter().map(|f| f.join().unwrap()).collect();
    // Deux essais libres au plus, quel que soit le nombre de sessions ; les
    // autres sont refusés avec une attente.
    let libres = attentes.iter().filter(|&&a| a == 0).count();
    assert!(libres <= 2, "{attentes:?}");
}

#[test]
fn ouverture_de_session_bornee() {
    let mut config = Config::nouvelle(dossier("ouverture"));
    config.adresse = SocketAddr::from(([127, 0, 0, 1], 0));
    config.delai_lecture = Duration::from_secs(2);
    config.delai_ouverture = Duration::from_millis(500);
    config.controle_droits = false;
    config.cles_portail = vec![cle_portail().verifying_key()];
    let serveur = Serveur::demarrer(config).unwrap();
    // Début d'enregistrement TLS, puis un octet toutes les 200 ms, chaque fois
    // sous le délai de lecture : la connexion est coupée au délai d'ouverture.
    let mut lent = TcpStream::connect(serveur.adresse()).unwrap();
    lent.write_all(&[0x16, 0x03, 0x01, 0x00, 0xff]).unwrap();
    let mut envoi = lent.try_clone().unwrap();
    thread::spawn(move || {
        for _ in 0..25 {
            thread::sleep(Duration::from_millis(200));
            if envoi.write_all(&[0]).is_err() {
                break;
            }
        }
    });
    lent.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let debut = Instant::now();
    let mut octet = [0u8; 1];
    let lu = lent.read(&mut octet);
    assert!(matches!(lu, Ok(0) | Err(_)), "{lu:?}");
    assert!(
        debut.elapsed() < Duration::from_millis(1800),
        "{:?}",
        debut.elapsed()
    );
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
    config.cles_portail = vec![cle_portail().verifying_key()];
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

#[test]
fn abonnement_et_lecture_seule() {
    let serveur = demarrer("abonnement", DELAI_CLIENT, 8);
    let (entreprise, admin) = installer(&serveur);
    serveur
        .base(|c| {
            let admin = comptes::connecter(c, "admin", MOT_DE_PASSE).unwrap();
            comptes::creer(
                c,
                &admin,
                &NouvelUtilisateur {
                    nom_complet: "Lecteur".into(),
                    identifiant: "lecteur".into(),
                    mot_de_passe: MOT_DE_PASSE.into(),
                    role: Role::Consultation,
                },
            )
            .unwrap();
        })
        .unwrap();
    let poste = poste_inscrit(&serveur);
    let mut client = connecter(&serveur, &poste).unwrap();
    client.connexion("admin", MOT_DE_PASSE).unwrap();
    let a = client.abonnement().unwrap();
    assert_eq!(a.statut, Statut::Actif);
    assert_eq!(a.jours_restants, Some(365));
    assert_eq!(a.postes_utilises, 1);

    // Licence échue hier : lecture seule, la consultation reste possible.
    let a = client.renouveler(&licence(&entreprise, -1, -40)).unwrap();
    assert_eq!(a.statut, Statut::Echu);
    let op = creer_tiers(&entreprise, &poste.empreinte(), &admin, "Kaboré");
    assert!(matches!(
        client.envoyer(&op),
        Err(ErreurClient::LectureSeule { echeance: Some(_) })
    ));
    assert!(client.changements(0, 10).is_ok());

    // Licence refusée : message à traduire, celle en place reste.
    match client.renouveler(&licence("autre", 365, 0)) {
        Err(ErreurClient::Refus { modele, .. }) => assert_eq!(
            modele,
            "licence d'abonnement émise pour une autre entreprise ({})"
        ),
        autre => panic!("{autre:?}"),
    }
    assert_eq!(client.abonnement().unwrap().statut, Statut::Echu);

    // Renouvellement : l'opération gardée par le poste passe.
    let a = client.renouveler(&licence(&entreprise, 30, 0)).unwrap();
    assert_eq!(a.statut, Statut::Actif);
    assert!(a.avertissement.is_some());
    assert!(matches!(client.envoyer(&op), Ok(Issue::Acceptee { .. })));

    // Sans le droit « Gérer les paramètres » : refus.
    let mut lecteur = connecter(&serveur, &poste).unwrap();
    lecteur.connexion("lecteur", MOT_DE_PASSE).unwrap();
    assert!(lecteur.abonnement().is_ok());
    assert!(matches!(
        lecteur.renouveler(&licence(&entreprise, 365, 0)),
        Err(ErreurClient::Refus { .. })
    ));
}

/// Code d'inscription créé sur le serveur, il y a `heures` heures.
fn code(serveur: &Serveur, nom: &str, heures: i64) -> inscription::CodeCree {
    let nom = nom.to_string();
    serveur
        .base(move |c| {
            let quand = chrono::Utc::now() - chrono::Duration::hours(heures);
            inscription::creer_code(c, &nom, quand).unwrap()
        })
        .unwrap()
}

fn inscrire(serveur: &Serveur, identite: &Identite, secret: &str) -> Result<Inscrit, ErreurClient> {
    Client::inscrire(
        &serveur.adresse().to_string(),
        serveur.empreinte(),
        identite,
        secret,
        DELAI_CLIENT,
    )
}

fn modele(resultat: Result<Inscrit, ErreurClient>) -> String {
    match resultat {
        Err(ErreurClient::Refus { modele, .. }) => modele,
        autre => panic!("refus attendu : {autre:?}"),
    }
}

#[test]
fn inscription_par_code() {
    let serveur = demarrer("code-complet", Duration::from_secs(5), 8);
    let (entreprise, _) = installer(&serveur);
    let cree = code(&serveur, "Comptabilité", 0);

    // Code complet donné par la fenêtre du serveur.
    let complet = {
        let dossier = chemin("code-complet");
        let cree = cree.clone();
        serveur
            .base(move |c| admin::code_complet(c, &dossier, &cree).unwrap())
            .unwrap()
    };
    let complet = CodeComplet::decoder(&complet).unwrap();
    assert_eq!(complet.entreprise, entreprise);
    assert_eq!(complet.empreinte, serveur.empreinte());
    assert_eq!(complet.secret, cree.secret);
    let port = format!(":{}", serveur.adresse().port());
    assert!(!complet.adresses.is_empty());
    assert!(complet.adresses.iter().all(|a| a.ends_with(&port)));

    // Inscription avec le secret tapé en minuscules, puis connexion.
    let identite = Identite::nouvelle("poste").unwrap();
    let tape = presenter_secret(&cree.secret).to_lowercase();
    assert_eq!(
        inscrire(&serveur, &identite, &tape).unwrap(),
        Inscrit {
            entreprise: entreprise.clone(),
            nom: "Comptabilité".into()
        }
    );
    assert!(connecter(&serveur, &identite).is_ok());
    let liste = serveur.base(|c| postes::liste(c).unwrap()).unwrap();
    assert_eq!(liste.len(), 1);
    assert_eq!(liste[0].nom, "Comptabilité");
    assert_eq!(liste[0].empreinte, identite.empreinte());

    // Un code ne sert qu'une fois.
    let autre = Identite::nouvelle("poste").unwrap();
    assert_eq!(
        modele(inscrire(&serveur, &autre, &cree.secret)),
        "code d'inscription inconnu ou déjà utilisé"
    );
    assert!(matches!(
        connecter(&serveur, &autre),
        Err(ErreurClient::PosteNonInscrit)
    ));
}

#[test]
fn codes_expire_annule_ou_mauvais_serveur() {
    let serveur = demarrer("codes", Duration::from_secs(5), 8);
    installer(&serveur);
    let identite = Identite::nouvelle("poste").unwrap();

    let expire = code(&serveur, "A", 25);
    assert_eq!(
        modele(inscrire(&serveur, &identite, &expire.secret)),
        "code d'inscription expiré"
    );

    let annule = code(&serveur, "B", 0);
    let id = annule.id;
    serveur
        .base(move |c| inscription::annuler_code(c, id, chrono::Utc::now()).unwrap())
        .unwrap();
    assert_eq!(
        modele(inscrire(&serveur, &identite, &annule.secret)),
        "code d'inscription annulé"
    );

    // Empreinte du serveur différente : rien n'est envoyé, le code reste bon.
    let bon = code(&serveur, "C", 0);
    let faux = "00".repeat(32);
    assert!(matches!(
        Client::inscrire(
            &serveur.adresse().to_string(),
            &faux,
            &identite,
            &bon.secret,
            DELAI_CLIENT
        ),
        Err(ErreurClient::CertificatDifferent)
    ));
    assert!(inscrire(&serveur, &identite, &bon.secret).is_ok());

    // Un poste déjà inscrit ne s'inscrit pas une seconde fois.
    let encore = code(&serveur, "D", 0);
    assert!(matches!(
        inscrire(&serveur, &identite, &encore.secret),
        Err(ErreurClient::Rejet(_))
    ));
}

#[test]
fn un_seul_poste_sans_licence() {
    // Serveur neuf, sans licence (décision 41) : un seul poste.
    let serveur = demarrer("sans-licence", Duration::from_secs(5), 8);
    let a = code(&serveur, "A", 0);
    let b = code(&serveur, "B", 0);
    assert!(inscrire(&serveur, &Identite::nouvelle("poste").unwrap(), &a.secret).is_ok());
    assert_eq!(
        modele(inscrire(
            &serveur,
            &Identite::nouvelle("poste").unwrap(),
            &b.secret
        )),
        "nombre de postes atteint : {} poste permis par l'abonnement"
    );
}

#[test]
fn attente_apres_des_echecs_d_inscription() {
    let serveur = demarrer("echecs-inscription", Duration::from_secs(5), 8);
    let identite = Identite::nouvelle("poste").unwrap();
    for _ in 0..5 {
        assert_eq!(
            modele(inscrire(&serveur, &identite, "AAAA-BBBB-CCCC")),
            "code d'inscription inconnu ou déjà utilisé"
        );
    }
    // Même un bon code attend.
    let bon = code(&serveur, "A", 0);
    assert_eq!(
        modele(inscrire(&serveur, &identite, &bon.secret)),
        "trop d'essais d'inscription : nouvel essai dans {} s"
    );
}

#[test]
fn decouverte_du_serveur() {
    let serveur = demarrer("decouverte", Duration::from_secs(5), 8);
    let trouves = kadenz_client::decouvrir_vers(serveur.adresse(), Duration::from_secs(1));
    assert_eq!(trouves.len(), 1);
    assert_eq!(trouves[0].0, serveur.adresse());
    assert_eq!(trouves[0].1.port, serveur.adresse().port());
}

#[test]
fn installation_avec_licence_tout_ou_rien() {
    let serveur = demarrer("installation-licence", Duration::from_secs(5), 8);
    let cles = vec![cle_portail().verifying_key()];
    let init = || InstallationInitiale {
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
    };
    serveur
        .base(move |c| {
            // Licence d'une autre entreprise : rien n'est installé.
            let autre = licence("autre-entreprise", 365, 0);
            assert!(admin::installer_avec(c, &init(), Some(&autre), &cles).is_err());
            assert_eq!(
                comptes::etat_installation(c).unwrap(),
                comptes::EtatInstallation::AssistantRequis
            );
            let entreprise = synchro::identifiant_entreprise(c).unwrap();
            let bonne = licence(&entreprise, 365, 0);
            admin::installer_avec(c, &init(), Some(&bonne), &cles).unwrap();
            assert_eq!(
                comptes::etat_installation(c).unwrap(),
                comptes::EtatInstallation::ConnexionRequise
            );
            let etat = abonnement::appliquer_avec(c, abonnement::horloge(), &cles).unwrap();
            assert_eq!(etat.statut, Statut::Actif);
        })
        .unwrap();
}

#[test]
fn installation_sans_licence() {
    let serveur = demarrer("installation-sans-licence", Duration::from_secs(5), 8);
    let cles = vec![cle_portail().verifying_key()];
    serveur
        .base(move |c| {
            let init = InstallationInitiale {
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
            };
            admin::installer_avec(c, &init, None, &cles).unwrap();
            let etat = abonnement::appliquer_avec(c, abonnement::horloge(), &cles).unwrap();
            assert_eq!(etat.statut, Statut::Absent);
            assert_eq!(abonnement::postes_permis(&etat), 1);
        })
        .unwrap();
}

#[test]
fn inscription_bornee_par_le_delai_d_ouverture() {
    let mut config = Config::nouvelle(dossier("inscription-lente"));
    config.adresse = SocketAddr::from(([127, 0, 0, 1], 0));
    config.delai_lecture = Duration::from_secs(2);
    config.delai_ouverture = Duration::from_millis(500);
    config.controle_droits = false;
    config.cles_portail = vec![cle_portail().verifying_key()];
    let serveur = Serveur::demarrer(config).unwrap();
    // Poste non inscrit : poignée de main, longueur de trame annoncée, puis
    // plus rien. La connexion est coupée au délai d'ouverture.
    let identite = Identite::nouvelle("poste").unwrap();
    let mut client = Client::ouvrir_sans_message(
        &serveur.adresse().to_string(),
        serveur.empreinte(),
        &identite,
        DELAI_CLIENT,
    )
    .unwrap();
    let flux = client.flux();
    flux.write_all(&100u32.to_be_bytes()).unwrap();
    flux.flush().unwrap();
    let debut = Instant::now();
    let mut octet = [0u8; 1];
    let lu = flux.read(&mut octet);
    assert!(matches!(lu, Ok(0) | Err(_)), "{lu:?}");
    assert!(
        debut.elapsed() < Duration::from_millis(1800),
        "{:?}",
        debut.elapsed()
    );
}

#[test]
fn empreinte_presentee_sans_confiance() {
    let serveur = demarrer("empreinte-presentee", Duration::from_secs(5), 8);
    installer(&serveur);
    let adresse = serveur.adresse().to_string();
    assert_eq!(
        kadenz_client::empreinte_presentee(&adresse, DELAI_CLIENT).unwrap(),
        serveur.empreinte()
    );
    // L'empreinte confirmée sert ensuite à l'inscription.
    let cree = code(&serveur, "Accueil", 0);
    let identite = Identite::nouvelle("poste").unwrap();
    assert!(inscrire(&serveur, &identite, &cree.secret).is_ok());
    assert!(connecter(&serveur, &identite).is_ok());
}

#[test]
fn demarrage_sans_decouverte_si_le_port_udp_est_pris() {
    // Le port UDP est pris avant le démarrage (décision 44). Le numéro vient
    // d'un port TCP libre : Windows réserve des plages au TCP seul (Hyper-V,
    // WinNAT), où un port UDP libre au hasard ne pourrait pas écouter en TCP.
    // Un port repris entre-temps par un autre programme fait réessayer.
    let dossier = dossier("udp-pris");
    let mut essais = 0;
    let (serveur, _pris) = loop {
        essais += 1;
        assert!(essais <= 10, "aucun port TCP et UDP libre trouvé");
        let port = std::net::TcpListener::bind(("127.0.0.1", 0))
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let Ok(pris) = std::net::UdpSocket::bind(("127.0.0.1", port)) else {
            continue;
        };
        let mut config = Config::nouvelle(dossier.clone());
        config.adresse = pris.local_addr().unwrap();
        config.controle_droits = false;
        config.cles_portail = vec![cle_portail().verifying_key()];
        match Serveur::demarrer(config) {
            Ok(serveur) => break (serveur, pris),
            Err(e) if e.contains("écoute") => continue,
            Err(e) => panic!("{e}"),
        }
    };
    let avertissement = serveur.avertissement().unwrap();
    assert!(avertissement.contains("découverte"), "{avertissement}");
    assert!(avertissement.contains("impossible"), "{avertissement}");
    // Les sessions marchent quand même.
    installer(&serveur);
    let identite = poste_inscrit(&serveur);
    assert!(connecter(&serveur, &identite).is_ok());
}
