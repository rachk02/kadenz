//! Essai de bout en bout du jalon F8 : relais en processus, serveur réel,
//! poste distant relié par le relais avec la session TLS de bout en bout ;
//! serveur coupé puis rallumé.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use kadenz_client::{Client, ErreurClient, Identite};
use kadenz_domaine::role::Role;
use kadenz_licence::SigningKey;
use kadenz_licence::entreprise::{self as lic, LicenceEntreprise};
use kadenz_protocole::{Action, Issue, Operation, Tiers};
use kadenz_relais::relais::{Config as ConfigRelais, Relais};
use kadenz_serveur::{Config, Serveur, admin};
use kadenz_stockage::comptes::NouvelUtilisateur;
use kadenz_stockage::entreprise::Entreprise;
use kadenz_stockage::installation::{self, InstallationInitiale};
use kadenz_stockage::parametres::Parametres;
use kadenz_stockage::{abonnement, postes, synchro};

const DELAI: Duration = Duration::from_secs(5);
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
            produit: kadenz_licence::entreprise::Produit::Entreprise,
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
    let d = std::env::temp_dir().join(format!(
        "kadenz-distance-essai-{nom}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn demarrer(dossier: &Path) -> Serveur {
    let mut config = Config::nouvelle(dossier.to_path_buf());
    config.adresse = SocketAddr::from(([127, 0, 0, 1], 0));
    config.delai_lecture = DELAI;
    config.controle_droits = false;
    config.decouverte = false;
    config.cles_portail = vec![cle_portail().verifying_key()];
    Serveur::demarrer(config).unwrap()
}

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
        .unwrap()
}

fn poste_inscrit(serveur: &Serveur, nom: &'static str) -> Identite {
    let identite = Identite::nouvelle("poste").unwrap();
    let empreinte = identite.empreinte();
    serveur
        .base(move |c| postes::inscrire(c, &empreinte, nom).unwrap())
        .unwrap();
    identite
}

fn creer_tiers(entreprise: &str, poste: &str, compte: &str, nom: &str) -> Operation {
    Operation {
        id: synchro::nouvel_uid(),
        entreprise: entreprise.into(),
        poste: poste.into(),
        compte: compte.into(),
        hors_connexion: false,
        heure_poste: None,
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

/// Attend qu'une condition devienne vraie, 15 secondes au plus.
fn attendre(mut condition: impl FnMut() -> bool) {
    let fin = Instant::now() + Duration::from_secs(15);
    while !condition() {
        assert!(Instant::now() < fin, "condition jamais remplie");
        thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn poste_distant_par_le_relais() {
    let dossier_relais = dossier("relais");
    let mut config = ConfigRelais::nouvelle(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        dossier_relais.clone(),
        vec![cle_portail().verifying_key()],
    );
    config.traces = None;
    let relais = Relais::demarrer(config).unwrap();
    let adresse_relais = relais.adresse().to_string();

    let dossier_serveur = dossier("serveur");
    let serveur = demarrer(&dossier_serveur);
    let empreinte_serveur = serveur.empreinte().to_string();
    let (entreprise, admin_uid) = installer(&serveur);
    let poste = poste_inscrit(&serveur, "Poste distant");
    let autre = poste_inscrit(&serveur, "Poste du bureau");

    // Rubrique de l'Admin, en réseau local : désactivé, aucun relais réglé.
    let (mut local, bienvenue) = Client::connecter(
        &serveur.adresse().to_string(),
        &empreinte_serveur,
        &poste,
        DELAI,
    )
    .unwrap();
    assert_eq!(bienvenue.relais, None);
    local.connexion("admin", MOT_DE_PASSE).unwrap();
    let etat = local.acces_distant().unwrap();
    assert!(!etat.actif);
    assert_eq!(etat.relais, None);
    assert!(!etat.joignable);
    assert_eq!(etat.postes.len(), 2);
    assert!(etat.postes.iter().all(|p| !p.autorise));

    // Fenêtre du serveur : relais réglé (forme contrôlée).
    let empreinte_relais = relais.empreinte().to_string();
    {
        let adresse = adresse_relais.clone();
        let empreinte = empreinte_relais.clone();
        serveur
            .base(move |c| {
                assert!(admin::regler_relais(c, Some(("sans-port", &empreinte))).is_err());
                assert!(admin::regler_relais(c, Some((&adresse, "abc"))).is_err());
                admin::regler_relais(c, Some((&adresse, &empreinte.to_uppercase()))).unwrap();
                assert_eq!(admin::lire_relais(c).unwrap().unwrap().empreinte, empreinte);
            })
            .unwrap();
    }

    // Poste coché avant l'activation (préparer la liste), puis activation.
    let etat = local.autoriser_distance(&poste.empreinte(), true).unwrap();
    assert!(
        etat.postes
            .iter()
            .any(|p| p.autorise && p.nom == "Poste distant")
    );
    let etat = local.regler_acces_distant(true).unwrap();
    assert!(etat.actif);
    assert_eq!(etat.relais.as_deref(), Some(adresse_relais.as_str()));
    attendre(|| relais.entreprises() == vec![entreprise.clone()]);
    attendre(|| local.acces_distant().unwrap().joignable);

    // Poste distant : relié par le relais, TLS de bout en bout, même
    // épinglage ; il travaille comme en réseau local.
    let (mut distant, bienvenue) = Client::connecter_par_relais(
        &adresse_relais,
        &entreprise,
        &empreinte_serveur,
        &poste,
        DELAI,
    )
    .unwrap();
    assert_eq!(bienvenue.entreprise, entreprise);
    assert_eq!(bienvenue.relais.as_deref(), Some(adresse_relais.as_str()));
    distant.connexion("admin", MOT_DE_PASSE).unwrap();
    let issue = distant
        .envoyer(&creer_tiers(
            &entreprise,
            &poste.empreinte(),
            &admin_uid,
            "Client à distance",
        ))
        .unwrap();
    assert!(matches!(issue, Issue::Acceptee { .. }), "{issue:?}");
    assert!(local.acces_distant().unwrap().derniere_distante.is_some());

    // Un serveur qui ne présente pas le certificat épinglé est refusé, même
    // par le relais.
    assert_eq!(
        Client::connecter_par_relais(
            &adresse_relais,
            &entreprise,
            &"00".repeat(32),
            &poste,
            DELAI
        )
        .err(),
        Some(ErreurClient::CertificatDifferent)
    );

    // Poste non coché : refusé par le relais, et pas de relais dans son
    // accueil en réseau local. Un refus n'est pas une adresse injoignable :
    // le motif est distinct et traduit à l'écran.
    assert!(matches!(
        Client::connecter_par_relais(
            &adresse_relais,
            &entreprise,
            &empreinte_serveur,
            &autre,
            DELAI
        ),
        Err(ErreurClient::RelaisRefuse { motif })
            if motif == kadenz_relais::motif::POSTE_NON_AUTORISE
    ));
    let (_, accueil_autre) = Client::connecter(
        &serveur.adresse().to_string(),
        &empreinte_serveur,
        &autre,
        DELAI,
    )
    .unwrap();
    assert_eq!(accueil_autre.relais, None);

    // Décoché : la session distante est coupée tout de suite.
    local.autoriser_distance(&poste.empreinte(), false).unwrap();
    thread::sleep(Duration::from_millis(800));
    assert!(distant.abonnement().is_err());
    assert!(matches!(
        Client::connecter_par_relais(
            &adresse_relais,
            &entreprise,
            &empreinte_serveur,
            &poste,
            DELAI
        ),
        Err(ErreurClient::RelaisRefuse { motif })
            if motif == kadenz_relais::motif::POSTE_NON_AUTORISE
    ));
    local.autoriser_distance(&poste.empreinte(), true).unwrap();

    // Serveur coupé : le relais l'oublie, le poste distant est refusé tout
    // de suite (il passe hors connexion).
    drop(local);
    serveur.arreter();
    attendre(|| relais.entreprises().is_empty());
    let debut = Instant::now();
    assert!(matches!(
        Client::connecter_par_relais(
            &adresse_relais,
            &entreprise,
            &empreinte_serveur,
            &poste,
            DELAI
        ),
        Err(ErreurClient::RelaisRefuse { motif })
            if motif == kadenz_relais::motif::SERVEUR_INJOIGNABLE
    ));
    assert!(debut.elapsed() < Duration::from_secs(4));

    // Serveur rallumé : il se réenregistre seul, le poste distant revient.
    let serveur = demarrer(&dossier_serveur);
    attendre(|| relais.entreprises() == vec![entreprise.clone()]);
    let derniere = serveur
        .base(|c| kadenz_stockage::distance::derniere_distante(c))
        .unwrap()
        .unwrap();
    assert!(derniere.is_some(), "dernière session distante perdue");
    let (mut distant, _) = Client::connecter_par_relais(
        &adresse_relais,
        &entreprise,
        &empreinte_serveur,
        &poste,
        DELAI,
    )
    .unwrap();
    distant.connexion("admin", MOT_DE_PASSE).unwrap();
    let changements = distant.changements(0, 500).unwrap();
    assert!(!changements.0.is_empty());

    // Accès désactivé : le serveur quitte le relais.
    let (mut local, _) = Client::connecter(
        &serveur.adresse().to_string(),
        &empreinte_serveur,
        &poste,
        DELAI,
    )
    .unwrap();
    local.connexion("admin", MOT_DE_PASSE).unwrap();
    let etat = local.regler_acces_distant(false).unwrap();
    assert!(!etat.actif);
    attendre(|| relais.entreprises().is_empty());

    drop(local);
    drop(distant);
    serveur.arreter();
    relais.arreter();
    let _ = std::fs::remove_dir_all(dossier_serveur);
    let _ = std::fs::remove_dir_all(dossier_relais);
}

#[test]
fn relais_regle_par_la_fenetre_pendant_le_recul() {
    let dossier_serveur = dossier("recul");
    let serveur = demarrer(&dossier_serveur);
    let empreinte_serveur = serveur.empreinte().to_string();
    let (entreprise, _) = installer(&serveur);
    let poste = poste_inscrit(&serveur, "Poste distant");

    // Relais réglé sur une adresse où rien n'écoute : l'enregistrement échoue
    // et le service attend avant de réessayer (5 secondes, puis plus).
    let ecoute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let morte = ecoute.local_addr().unwrap().to_string();
    drop(ecoute);
    {
        let morte = morte.clone();
        serveur
            .base(move |c| admin::regler_relais(c, Some((&morte, &"ab".repeat(32)))).unwrap())
            .unwrap();
    }
    let (mut local, _) = Client::connecter(
        &serveur.adresse().to_string(),
        &empreinte_serveur,
        &poste,
        DELAI,
    )
    .unwrap();
    local.connexion("admin", MOT_DE_PASSE).unwrap();
    local.autoriser_distance(&poste.empreinte(), true).unwrap();
    local.regler_acces_distant(true).unwrap();
    attendre(|| local.acces_distant().unwrap().erreur.is_some());

    // La fenêtre du serveur règle le bon relais, directement dans la base,
    // sans requête : le service le prend sans attendre la fin du recul.
    let dossier_relais = dossier("recul-relais");
    let mut config = ConfigRelais::nouvelle(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        dossier_relais.clone(),
        vec![cle_portail().verifying_key()],
    );
    config.traces = None;
    let relais = Relais::demarrer(config).unwrap();
    let debut = Instant::now();
    {
        let adresse = relais.adresse().to_string();
        let empreinte = relais.empreinte().to_string();
        serveur
            .base(move |c| admin::regler_relais(c, Some((&adresse, &empreinte))).unwrap())
            .unwrap();
    }
    attendre(|| relais.entreprises() == vec![entreprise.clone()]);
    assert!(
        debut.elapsed() < Duration::from_millis(3500),
        "relais pris après {:?}",
        debut.elapsed()
    );

    drop(local);
    serveur.arreter();
    relais.arreter();
    let _ = std::fs::remove_dir_all(dossier_serveur);
    let _ = std::fs::remove_dir_all(dossier_relais);
}

/// Relais par WebSocket (docs/spec-relais-websocket.md) : adresse `ws://`
/// réglée dans la fenêtre du serveur, connexion de contrôle, sessions du
/// serveur et du poste distant, tout par la passerelle du relais ; serveur
/// coupé puis rallumé, réenregistré seul par la même adresse.
#[test]
fn poste_distant_par_websocket() {
    let dossier_relais = dossier("relais-ws");
    let mut config = ConfigRelais::nouvelle(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        dossier_relais.clone(),
        vec![cle_portail().verifying_key()],
    );
    config.traces = None;
    config.ecoute_ws = Some(SocketAddr::from(([127, 0, 0, 1], 0)));
    let relais = Relais::demarrer(config).unwrap();
    let adresse_relais = format!("ws://127.0.0.1:{}", relais.adresse_ws().unwrap().port());

    let dossier_serveur = dossier("serveur-ws");
    let serveur = demarrer(&dossier_serveur);
    let empreinte_serveur = serveur.empreinte().to_string();
    let (entreprise, admin_uid) = installer(&serveur);
    let poste = poste_inscrit(&serveur, "Poste distant");

    // Fenêtre du serveur : adresses WebSocket admises et refusées.
    let empreinte_relais = relais.empreinte().to_string();
    {
        let adresse = adresse_relais.clone();
        let empreinte = empreinte_relais.clone();
        serveur
            .base(move |c| {
                for refusee in [
                    "ws://relais.exemple.fr",
                    "wss://relais.exemple.fr/chemin",
                    "wss://relais.exemple.fr:0",
                    "wss://",
                ] {
                    assert_eq!(
                        admin::regler_relais(c, Some((refusee, &empreinte))),
                        Err(admin::MODELES_ERREUR_RELAIS[3].to_string()),
                        "{refusee}"
                    );
                }
                assert_eq!(
                    admin::regler_relais(c, Some(("wss://relais.exemple.fr", "abc"))),
                    Err(admin::MODELES_ERREUR_RELAIS[2].to_string())
                );
                admin::regler_relais(c, Some(("wss://Relais.Exemple.fr:443/", &empreinte)))
                    .unwrap();
                assert_eq!(
                    admin::lire_relais(c).unwrap().unwrap().adresse,
                    "wss://relais.exemple.fr"
                );
                admin::regler_relais(c, Some((&adresse, &empreinte))).unwrap();
                assert_eq!(admin::lire_relais(c).unwrap().unwrap().adresse, adresse);
            })
            .unwrap();
    }

    let (mut local, _) = Client::connecter(
        &serveur.adresse().to_string(),
        &empreinte_serveur,
        &poste,
        DELAI,
    )
    .unwrap();
    local.connexion("admin", MOT_DE_PASSE).unwrap();
    local.autoriser_distance(&poste.empreinte(), true).unwrap();
    let etat = local.regler_acces_distant(true).unwrap();
    assert_eq!(etat.relais.as_deref(), Some(adresse_relais.as_str()));
    attendre(|| relais.entreprises() == vec![entreprise.clone()]);

    let (mut distant, bienvenue) = Client::connecter_par_relais(
        &adresse_relais,
        &entreprise,
        &empreinte_serveur,
        &poste,
        DELAI,
    )
    .unwrap();
    assert_eq!(bienvenue.relais.as_deref(), Some(adresse_relais.as_str()));
    distant.connexion("admin", MOT_DE_PASSE).unwrap();
    let issue = distant
        .envoyer(&creer_tiers(
            &entreprise,
            &poste.empreinte(),
            &admin_uid,
            "Client par WebSocket",
        ))
        .unwrap();
    assert!(matches!(issue, Issue::Acceptee { .. }), "{issue:?}");

    // Serveur coupé puis rallumé : réenregistré seul, le poste revient.
    drop(local);
    drop(distant);
    serveur.arreter();
    attendre(|| relais.entreprises().is_empty());
    let serveur = demarrer(&dossier_serveur);
    attendre(|| relais.entreprises() == vec![entreprise.clone()]);
    let (mut distant, _) = Client::connecter_par_relais(
        &adresse_relais,
        &entreprise,
        &empreinte_serveur,
        &poste,
        DELAI,
    )
    .unwrap();
    distant.connexion("admin", MOT_DE_PASSE).unwrap();
    assert!(!distant.changements(0, 500).unwrap().0.is_empty());

    drop(distant);
    serveur.arreter();
    relais.arreter();
    let _ = std::fs::remove_dir_all(dossier_serveur);
    let _ = std::fs::remove_dir_all(dossier_relais);
}
