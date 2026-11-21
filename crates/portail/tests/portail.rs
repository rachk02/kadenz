//! Essais du portail (jalon F9a) : droits des revendeurs, paiements,
//! émission acceptée par un serveur Kadenz, journal en ajout seul, routes
//! HTTP (sessions, jeton anti-falsification, en-têtes), codes d'activation
//! (décision 74).

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use chrono::NaiveDate;
use ed25519_dalek::SigningKey;
use kadenz_domaine::module::Module;
use kadenz_domaine::role::Role as RoleKadenz;
use kadenz_portail::web::{self, ConfigWeb};
use kadenz_portail::{
    Activation, Compte, DemandeActivation, ErreurPortail, EtatCode, MODELES_ACTIVATION,
    NouveauPaiement, Portail,
};
use kadenz_stockage::comptes::NouvelUtilisateur;
use kadenz_stockage::entreprise::Entreprise as FicheKadenz;
use kadenz_stockage::installation::{self, InstallationInitiale};
use kadenz_stockage::parametres::Parametres;
use kadenz_stockage::{abonnement, synchro};

const MOT_DE_PASSE: &str = "phrase-de-passe-essai-12";
const ENTREPRISE_A: &str = "01a1dfaf-e51a-747a-92f6-37b52b38b16b";
const ENTREPRISE_B: &str = "01a1dfaf-e51a-747a-92f6-37b52b38b16c";

fn cle_essai() -> SigningKey {
    SigningKey::from_bytes(&[5u8; 32])
}

fn dossier(nom: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("kadenz-portail-essai-{nom}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn jour(texte: &str) -> NaiveDate {
    NaiveDate::parse_from_str(texte, "%Y-%m-%d").unwrap()
}

/// Portail d'essai à une date choisie, avec un éditeur et deux revendeurs.
fn portail(nom: &str, aujourdhui: &str) -> (Portail, PathBuf, Compte, Compte, Compte) {
    let d = dossier(nom);
    let date = jour(aujourdhui);
    let p = Portail::ouvrir_avec(&d, cle_essai(), Arc::new(move || date)).unwrap();
    let editeur = p.creer_editeur("editeur", "Éditeur", MOT_DE_PASSE).unwrap();
    let a = p
        .creer_revendeur(&editeur, "rev-a", "Revendeur A", MOT_DE_PASSE)
        .unwrap();
    let b = p
        .creer_revendeur(&editeur, "rev-b", "Revendeur B", MOT_DE_PASSE)
        .unwrap();
    (p, d, editeur, a, b)
}

fn saisie(postes: &str, formule: &str) -> NouveauPaiement {
    NouveauPaiement {
        recu_le: "2026-11-15".into(),
        mode: "virement".into(),
        montant: "150 000".into(),
        devise: "xof".into(),
        reference: "VIR-0042".into(),
        postes: postes.into(),
        formule: formule.into(),
    }
}

#[test]
fn revendeur_limite_a_ses_clients() {
    let (p, d, editeur, a, b) = portail("droits", "2026-11-15");
    p.creer_entreprise(&a, ENTREPRISE_A, "Client de A", Some(b.id))
        .unwrap();
    p.creer_entreprise(&b, ENTREPRISE_B, "Client de B", None)
        .unwrap();

    // Un revendeur rattache toujours son entreprise à lui-même.
    assert_eq!(
        p.entreprise(&a, ENTREPRISE_A).unwrap().revendeur.unwrap().0,
        a.id
    );
    // Chacun ne voit que ses clients ; l'éditeur voit tout.
    let vus = |c: &Compte| -> Vec<String> {
        p.entreprises(c)
            .unwrap()
            .into_iter()
            .map(|v| v.entreprise.client)
            .collect()
    };
    assert_eq!(vus(&a), vec!["Client de A"]);
    assert_eq!(vus(&b), vec!["Client de B"]);
    assert_eq!(vus(&editeur).len(), 2);

    // A ne peut ni lire, ni payer, ni émettre pour le client de B.
    assert_eq!(p.fiche(&a, ENTREPRISE_B), Err(ErreurPortail::Introuvable));
    assert_eq!(
        p.enregistrer_paiement(&a, ENTREPRISE_B, &saisie("3", "annuelle")),
        Err(ErreurPortail::Introuvable)
    );
    let paiement_b = p
        .enregistrer_paiement(&b, ENTREPRISE_B, &saisie("3", "annuelle"))
        .unwrap();
    assert_eq!(
        p.paiement(&a, paiement_b.id),
        Err(ErreurPortail::Introuvable)
    );
    assert_eq!(
        p.emettre(&a, paiement_b.id, &Module::TOUS, "2027-11-15"),
        Err(ErreurPortail::Introuvable)
    );
    let licence_b = p
        .emettre(&b, paiement_b.id, &Module::TOUS, "2027-11-15")
        .unwrap();
    assert_eq!(p.licence(&a, licence_b.id), Err(ErreurPortail::Introuvable));

    // Comptes et journal complet : à l'éditeur seulement.
    assert_eq!(p.comptes(&a), Err(ErreurPortail::Interdit));
    assert_eq!(
        p.creer_revendeur(&a, "rev-c", "Revendeur C", MOT_DE_PASSE),
        Err(ErreurPortail::Interdit)
    );
    assert!(
        p.journal(&a)
            .unwrap()
            .iter()
            .all(|j| j.compte.as_deref() == Some("Revendeur A"))
    );

    // Revendeur désactivé : plus aucun droit, plus de connexion.
    p.regler_actif(&editeur, b.id, false).unwrap();
    assert_eq!(p.entreprises(&b), Err(ErreurPortail::Interdit));
    assert_eq!(
        p.connecter("rev-b", MOT_DE_PASSE, "127.0.0.1").map(|r| r.1),
        Err(ErreurPortail::Identifiants)
    );
    drop(p);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn paiement_et_emission() {
    let (p, d, editeur, a, _) = portail("emission", "2026-11-15");
    p.creer_entreprise(&editeur, ENTREPRISE_A, "Boulangerie Wendé", Some(a.id))
        .unwrap();

    // Saisies refusées.
    let mut mauvaise = saisie("3", "annuelle");
    mauvaise.montant = "0".into();
    assert!(matches!(
        p.enregistrer_paiement(&a, ENTREPRISE_A, &mauvaise),
        Err(ErreurPortail::Validation(_))
    ));
    let mut mauvaise = saisie("3", "annuelle");
    mauvaise.recu_le = "2026-11-16".into();
    assert!(matches!(
        p.enregistrer_paiement(&a, ENTREPRISE_A, &mauvaise),
        Err(ErreurPortail::Validation(_))
    ));
    assert!(matches!(
        p.creer_entreprise(&editeur, "pas-un-uuid", "X", None),
        Err(ErreurPortail::Validation(_))
    ));

    let paiement = p
        .enregistrer_paiement(&a, ENTREPRISE_A, &saisie("3", "annuelle"))
        .unwrap();
    assert_eq!(paiement.montant, 150_000);
    assert_eq!(paiement.devise, "XOF");

    // Première licence : tous les modules, un an à partir du jour.
    let prop = p.proposition(&a, paiement.id).unwrap();
    assert_eq!(prop.modules, Module::TOUS.to_vec());
    assert_eq!(prop.echeance, "2027-11-14");
    assert!(matches!(
        p.emettre(&a, paiement.id, &prop.modules, "2026-11-14"),
        Err(ErreurPortail::Validation(_))
    ));
    let premiere = p
        .emettre(
            &a,
            paiement.id,
            &[Module::Devis, Module::Proforma],
            &prop.echeance,
        )
        .unwrap();
    assert_eq!(premiere.emetteur, "Revendeur A");
    assert_eq!(premiere.postes, 3);
    assert!(premiere.texte.starts_with("KADENZ-E-"));
    // Un paiement ne sert qu'une fois.
    assert!(matches!(
        p.emettre(&a, paiement.id, &prop.modules, &prop.echeance),
        Err(ErreurPortail::Validation(_))
    ));

    // Renouvellement mensuel : modules repris, à la suite de l'échéance.
    let second = p
        .enregistrer_paiement(&a, ENTREPRISE_A, &saisie("5", "mensuelle"))
        .unwrap();
    let prop = p.proposition(&a, second.id).unwrap();
    assert_eq!(prop.modules, vec![Module::Devis, Module::Proforma]);
    assert_eq!(prop.echeance, "2027-12-14");

    // La licence porte bien l'émetteur, signée par la clé du portail.
    let lue =
        kadenz_licence::entreprise::verifier(&premiere.texte, ENTREPRISE_A, &[p.cle_publique()])
            .unwrap();
    assert_eq!(
        lue.emetteur,
        kadenz_licence::entreprise::Emetteur::Revendeur {
            identifiant: format!("revendeur-{}", a.id),
            nom: "Revendeur A".into()
        }
    );
    let actions: Vec<String> = p
        .journal(&editeur)
        .unwrap()
        .into_iter()
        .map(|j| j.action)
        .collect();
    assert!(actions.contains(&"licence_emise".to_string()));
    assert!(actions.contains(&"paiement_enregistre".to_string()));
    drop(p);

    // Journal en ajout seul, même directement dans la base.
    let conn = rusqlite::Connection::open(d.join("portail.sqlite")).unwrap();
    assert!(conn.execute("UPDATE journal SET action = 'x'", []).is_err());
    assert!(conn.execute("DELETE FROM journal", []).is_err());
    drop(conn);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn licence_acceptee_par_un_serveur() {
    // Base Kadenz installée : son identifiant d'entreprise est celui que
    // l'Admin copie dans le portail.
    let d = dossier("serveur");
    std::fs::create_dir_all(&d).unwrap();
    let mut conn = kadenz_stockage::ouvrir(&d.join("kadenz.sqlite")).unwrap();
    let admin = installation::installer(
        &mut conn,
        &InstallationInitiale {
            entreprise: FicheKadenz {
                nom: "Pharmacie Nabiiga".into(),
                ..Default::default()
            },
            admin: NouvelUtilisateur {
                nom_complet: "Administrateur".into(),
                identifiant: "admin".into(),
                mot_de_passe: "motdepasse1".into(),
                role: RoleKadenz::Admin,
            },
            parametres: Parametres::default(),
        },
    )
    .unwrap();
    synchro::completer_uids(&conn).unwrap();
    let identifiant = synchro::identifiant_entreprise(&conn).unwrap();

    let aujourdhui = abonnement::horloge();
    let dp = dossier("serveur-portail");
    let p = Portail::ouvrir_avec(&dp, cle_essai(), Arc::new(move || aujourdhui)).unwrap();
    let editeur = p.creer_editeur("editeur", "Éditeur", MOT_DE_PASSE).unwrap();
    p.creer_entreprise(
        &editeur,
        &identifiant.to_uppercase(),
        "Pharmacie Nabiiga",
        None,
    )
    .unwrap();
    let mut s = saisie("2", "annuelle");
    s.recu_le = aujourdhui.format("%Y-%m-%d").to_string();
    let paiement = p.enregistrer_paiement(&editeur, &identifiant, &s).unwrap();
    let prop = p.proposition(&editeur, paiement.id).unwrap();
    let licence = p
        .emettre(&editeur, paiement.id, &prop.modules, &prop.echeance)
        .unwrap();
    assert_eq!(licence.emetteur, "Éditeur");

    let installee = abonnement::enregistrer_avec(
        &conn,
        &admin,
        &licence.texte,
        aujourdhui,
        &[cle_essai().verifying_key()],
    )
    .unwrap();
    assert!(installee.erreur.is_none(), "{:?}", installee.erreur);
    assert_eq!(installee.licence.unwrap().postes, 2);

    // Seconde licence le même jour : acceptée aussi.
    let second = p.enregistrer_paiement(&editeur, &identifiant, &s).unwrap();
    let prop = p.proposition(&editeur, second.id).unwrap();
    let autre = p
        .emettre(&editeur, second.id, &prop.modules, &prop.echeance)
        .unwrap();
    abonnement::enregistrer_avec(
        &conn,
        &admin,
        &autre.texte,
        aujourdhui,
        &[cle_essai().verifying_key()],
    )
    .unwrap();
    drop(conn);
    drop(p);
    let _ = std::fs::remove_dir_all(d);
    let _ = std::fs::remove_dir_all(dp);
}

#[test]
fn connexions_freinees() {
    let (p, d, ..) = portail("frein", "2026-11-15");
    for _ in 0..5 {
        assert_eq!(
            p.connecter("editeur", "mauvais", "10.0.0.1").map(|r| r.1),
            Err(ErreurPortail::Identifiants)
        );
    }
    // Depuis cette adresse, même le bon mot de passe attend, pour tout compte.
    assert!(matches!(
        p.connecter("editeur", MOT_DE_PASSE, "10.0.0.1"),
        Err(ErreurPortail::Freine(_))
    ));
    assert!(matches!(
        p.connecter("rev-a", MOT_DE_PASSE, "10.0.0.1"),
        Err(ErreurPortail::Freine(_))
    ));
    // Le titulaire qui se connecte d'ailleurs n'est pas bloqué par les
    // échecs d'un tiers (défaut 5 de la relecture d'OpenCode).
    assert!(p.connecter("editeur", MOT_DE_PASSE, "10.0.0.2").is_ok());
    assert!(p.connecter("rev-a", MOT_DE_PASSE, "10.0.0.3").is_ok());
    drop(p);
    let _ = std::fs::remove_dir_all(d);
}

/// Une attaque répartie sur beaucoup d'adresses finit par freiner
/// l'identifiant lui-même, pour une minute au plus.
#[test]
fn identifiant_freine_apres_une_attaque_repartie() {
    let (p, d, ..) = portail("frein-reparti", "2026-11-15");
    for k in 0..50 {
        let adresse = format!("10.1.{}.{}", k / 200, k % 200);
        assert_eq!(
            p.connecter("editeur", "mauvais", &adresse).map(|r| r.1),
            Err(ErreurPortail::Identifiants)
        );
    }
    match p.connecter("editeur", MOT_DE_PASSE, "10.9.9.9") {
        Err(ErreurPortail::Freine(s)) => assert!(s <= 60, "{s}"),
        autre => panic!("attendu un frein, reçu {:?}", autre.map(|r| r.1)),
    }
    drop(p);
    let _ = std::fs::remove_dir_all(d);
}

/// Le frein est gardé dans la base : un redémarrage du portail ne l'efface pas.
#[test]
fn frein_garde_apres_un_redemarrage() {
    let (p, d, ..) = portail("frein-garde", "2026-11-15");
    for _ in 0..5 {
        let _ = p.connecter("editeur", "mauvais", "10.0.0.7");
    }
    drop(p);
    let p = Portail::ouvrir(&d, cle_essai()).unwrap();
    assert!(matches!(
        p.connecter("editeur", MOT_DE_PASSE, "10.0.0.7"),
        Err(ErreurPortail::Freine(_))
    ));
    drop(p);
    let _ = std::fs::remove_dir_all(d);
}

/// Refus pendant l'attente du frein : une seule entrée au journal par
/// attente, puis une nouvelle à l'attente suivante (remarque de kadenz-74).
#[test]
fn refus_du_frein_au_journal() {
    let (p, d, editeur, ..) = portail("frein-journal", "2026-11-15");
    let freinees = |p: &Portail| {
        p.journal(&editeur)
            .unwrap()
            .into_iter()
            .filter(|j| j.action == "connexion_freinee")
            .count()
    };
    for _ in 0..5 {
        let _ = p.connecter("editeur", "mauvais", "10.0.0.9");
    }
    assert_eq!(freinees(&p), 0);
    for _ in 0..4 {
        assert!(matches!(
            p.connecter("editeur", MOT_DE_PASSE, "10.0.0.9"),
            Err(ErreurPortail::Freine(_))
        ));
    }
    assert_eq!(freinees(&p), 1);
    let j = p
        .journal(&editeur)
        .unwrap()
        .into_iter()
        .find(|j| j.action == "connexion_freinee")
        .unwrap();
    assert!(j.details.contains("10.0.0.9"), "{}", j.details);
    // Chaîne intacte.
    assert_eq!(p.verifier_journal().unwrap().rupture, None);
    drop(p);
    let _ = std::fs::remove_dir_all(d);
}

/// Le verrou de la base n'est pas tenu pendant Argon2 (défaut 1) : une
/// lecture de la base pendant une connexion répond sans attendre la fin du
/// hachage.
#[test]
fn base_libre_pendant_le_hachage() {
    let (p, d, editeur, ..) = portail("hachage", "2026-11-15");
    let debut = std::time::Instant::now();
    let (jeton, _) = p.connecter("editeur", MOT_DE_PASSE, "10.0.0.1").unwrap();
    let une_connexion = debut.elapsed();
    let p2 = p.clone();
    let fil = std::thread::spawn(move || p2.connecter("rev-a", MOT_DE_PASSE, "10.0.0.2"));
    // Laisse au fil le temps d'entrer dans le hachage.
    std::thread::sleep(une_connexion / 4);
    let debut = std::time::Instant::now();
    assert!(p.session(&jeton).is_some());
    assert!(p.comptes(&editeur).is_ok());
    let lecture = debut.elapsed();
    assert!(fil.join().unwrap().is_ok());
    assert!(
        lecture < une_connexion / 2,
        "lecture {lecture:?} contre une connexion {une_connexion:?}"
    );
    drop(p);
    let _ = std::fs::remove_dir_all(d);
}

/// Deux connexions en même temps réussissent toutes les deux.
#[test]
fn connexions_simultanees() {
    let (p, d, ..) = portail("simultanees", "2026-11-15");
    let fils: Vec<_> = ["editeur", "rev-a", "rev-b"]
        .into_iter()
        .enumerate()
        .map(|(k, id)| {
            let p = p.clone();
            std::thread::spawn(move || p.connecter(id, MOT_DE_PASSE, &format!("10.0.1.{k}")))
        })
        .collect();
    for fil in fils {
        assert!(fil.join().unwrap().is_ok());
    }
    drop(p);
    let _ = std::fs::remove_dir_all(d);
}

// ----- Routes HTTP -----

struct Reponse {
    statut: u16,
    entetes: Vec<(String, String)>,
    corps: String,
}

impl Reponse {
    fn entete(&self, nom: &str) -> Option<&str> {
        self.entetes
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(nom))
            .map(|(_, v)| v.as_str())
    }
}

fn requete(
    adresse: SocketAddr,
    methode: &str,
    chemin: &str,
    cookie: Option<&str>,
    corps: &str,
) -> Reponse {
    requete_avec(adresse, methode, chemin, cookie, &[], corps)
}

/// Comme `requete`, avec des en-têtes en plus.
fn requete_avec(
    adresse: SocketAddr,
    methode: &str,
    chemin: &str,
    cookie: Option<&str>,
    en_plus: &[(&str, &str)],
    corps: &str,
) -> Reponse {
    let mut tcp = TcpStream::connect(adresse).unwrap();
    tcp.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    let mut texte =
        format!("{methode} {chemin} HTTP/1.1\r\nHost: {adresse}\r\nConnection: close\r\n");
    if let Some(c) = cookie {
        texte.push_str(&format!("Cookie: {c}\r\n"));
    }
    for (nom, valeur) in en_plus {
        texte.push_str(&format!("{nom}: {valeur}\r\n"));
    }
    if methode == "POST" {
        texte.push_str(&format!(
            "Content-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n",
            corps.len()
        ));
    }
    texte.push_str("\r\n");
    texte.push_str(corps);
    tcp.write_all(texte.as_bytes()).unwrap();
    let mut brut = Vec::new();
    tcp.read_to_end(&mut brut).unwrap();
    let brut = String::from_utf8_lossy(&brut).to_string();
    let (tete, corps) = brut.split_once("\r\n\r\n").unwrap_or((&brut, ""));
    let mut lignes = tete.lines();
    let statut = lignes
        .next()
        .unwrap()
        .split(' ')
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    let entetes = lignes
        .filter_map(|l| l.split_once(':'))
        .map(|(n, v)| (n.trim().to_string(), v.trim().to_string()))
        .collect();
    Reponse {
        statut,
        entetes,
        corps: corps.to_string(),
    }
}

/// Jeton anti-falsification d'une page : valeur du champ `csrf`.
fn csrf(page: &str) -> String {
    let debut = page.find("name=\"csrf\"").expect("champ csrf");
    let reste = &page[debut..];
    let valeur = &reste[reste.find("value=\"").unwrap() + 7..];
    valeur[..valeur.find('"').unwrap()].to_string()
}

/// Premier morceau `nom=valeur` d'un en-tête Set-Cookie.
fn morceau_cookie(set_cookie: &str) -> String {
    set_cookie.split(';').next().unwrap().to_string()
}

/// Jeton de la page de connexion : cookie à renvoyer et champ du formulaire.
fn jeton_connexion(a: SocketAddr) -> (String, String) {
    let page = requete(a, "GET", "/connexion", None, "");
    assert_eq!(page.statut, 200);
    let cookie = morceau_cookie(page.entete("set-cookie").expect("cookie de connexion"));
    assert!(cookie.starts_with("__Host-kadenz_connexion="));
    (cookie, csrf(&page.corps))
}

/// Connexion par le formulaire, avec son jeton ; `session` : cookie de
/// session déjà porté par le navigateur, envoyé avec le jeton.
fn se_connecter(
    a: SocketAddr,
    identifiant: &str,
    mot_de_passe: &str,
    session: Option<&str>,
) -> Reponse {
    let (cookie, jeton) = jeton_connexion(a);
    let cookies = match session {
        Some(s) => format!("{s}; {cookie}"),
        None => cookie,
    };
    requete(
        a,
        "POST",
        "/connexion",
        Some(&cookies),
        &format!("identifiant={identifiant}&mot_de_passe={mot_de_passe}&csrf={jeton}"),
    )
}

#[test]
fn routes_sessions_et_jetons() {
    let d = dossier("web");
    let p = Portail::ouvrir(&d, cle_essai()).unwrap();
    // Un revendeur : un éditeur n'a de session complète qu'avec la double
    // authentification (essai double_authentification_des_editeurs).
    let editeur = p.creer_editeur("editeur", "Éditeur", MOT_DE_PASSE).unwrap();
    p.creer_revendeur(&editeur, "revendeur", "Revendeur", MOT_DE_PASSE)
        .unwrap();
    let lance = web::lancer(
        p,
        SocketAddr::from(([127, 0, 0, 1], 0)),
        ConfigWeb::default(),
    )
    .unwrap();
    let a = lance.adresse();

    // Console privée : rien à indexer, en-têtes de sécurité partout.
    // Seule la page de connexion peut être lue, pour que son noindex soit vu.
    let r = requete(a, "GET", "/robots.txt", None, "");
    assert_eq!(r.statut, 200);
    assert!(r.corps.contains(
        "Disallow: /
"
    ));
    assert!(r.corps.contains(
        "Allow: /connexion$
"
    ));
    assert!(!r.corps.contains("Sitemap"));
    assert!(r.corps.contains("Allow: /style.css\n"));
    let connexion = requete(a, "GET", "/connexion", None, "");
    assert!(connexion.corps.contains("content=\"noindex, nofollow\""));
    assert!(connexion.corps.contains("name=\"description\""));
    assert_eq!(connexion.entete("x-robots-tag"), Some("noindex, nofollow"));
    // Feuille de style à empreinte : un CSS changé est rechargé malgré le cache.
    let lien = kadenz_portail::pages::lien_style();
    assert!(lien.starts_with("/style.css?v=") && lien.len() == "/style.css?v=".len() + 8);
    assert!(connexion.corps.contains(&format!("href=\"{lien}\"")));
    assert!(!connexion.corps.contains("href=\"/style.css\""));
    let feuille = requete(a, "GET", &lien, None, "");
    assert_eq!(feuille.statut, 200);
    assert_eq!(
        feuille.entete("content-type"),
        Some("text/css; charset=utf-8")
    );
    // Sans adresse publique, aucun aperçu de partage.
    assert!(!connexion.corps.contains("og:image"));
    let image = requete(a, "GET", kadenz_portail::pages::IMAGE_PARTAGE, None, "");
    assert_eq!(image.statut, 200);
    assert_eq!(image.entete("content-type"), Some("image/png"));
    assert!(
        r.entete("content-security-policy")
            .unwrap()
            .contains("default-src 'self'")
    );
    assert_eq!(r.entete("x-robots-tag"), Some("noindex, nofollow"));
    assert_eq!(r.entete("x-content-type-options"), Some("nosniff"));
    assert_eq!(
        requete(a, "GET", "/police/inter-regular.ttf", None, "").statut,
        200
    );

    // Sans session : vers la connexion.
    let r = requete(a, "GET", "/", None, "");
    assert_eq!(r.statut, 303);
    assert_eq!(r.entete("location"), Some("/connexion"));
    assert_eq!(requete(a, "GET", "/connexion", None, "").statut, 200);
    // Santé : sans session, réponse « ok ».
    let r = requete(a, "GET", "/sante", None, "");
    assert_eq!((r.statut, r.corps.as_str()), (200, "ok"));

    // Mauvais mot de passe, puis le bon.
    let r = se_connecter(a, "revendeur", "faux", None);
    assert_eq!(r.statut, 401);
    let r = se_connecter(a, "revendeur", MOT_DE_PASSE, None);
    assert_eq!(r.statut, 303);
    let cookie = r.entete("set-cookie").unwrap().to_string();
    assert!(
        cookie.starts_with("__Host-kadenz_portail=")
            && cookie.contains("Path=/")
            && !cookie.contains("Domain")
            && cookie.contains("HttpOnly")
            && cookie.contains("Secure")
            && cookie.contains("SameSite=Strict")
    );
    let session = morceau_cookie(&cookie);
    let accueil = requete(a, "GET", "/", Some(&session), "");
    assert_eq!(accueil.statut, 200);

    // Coque animée du gabarit (2026-11-21) : couverture de transition,
    // en-tête en pilules, pied au C Tournant, feuille et scripts du gabarit ;
    // page à formulaire en écran partagé, panneau animé à gauche.
    assert!(accueil.corps.contains("<body class=\"inverted console\">"));
    assert!(accueil.corps.contains("<div class=\"pt\""));
    assert!(
        accueil
            .corps
            .contains("<header class=\"navigation inverted\"")
    );
    assert!(accueil.corps.contains("<footer class=\"footer inverted\""));
    assert!(!accueil.corps.contains("contenu-partage"));
    assert!(!accueil.corps.contains("Diatype"));
    // Aucune entreprise : le lien reste simple, sans panneau ni exemple.
    assert!(!accueil.corps.contains("data-mega-cle=\"entreprises\""));
    for &(chemin, type_contenu, _) in kadenz_portail::pages::gabarit::FICHIERS {
        let f = requete(a, "GET", chemin, None, "");
        assert_eq!(f.statut, 200, "{chemin}");
        assert_eq!(f.entete("content-type"), Some(type_contenu), "{chemin}");
        assert_eq!(
            f.entete("cache-control"),
            Some("public, max-age=31536000, immutable")
        );
    }
    let gsap = kadenz_portail::pages::gabarit::JS_GSAP;
    let coque = kadenz_portail::pages::gabarit::JS_COQUE;
    assert!(
        accueil
            .corps
            .contains(&format!("<script src=\"{gsap}\" defer>"))
    );
    assert!(accueil.corps.find(gsap) < accueil.corps.find(coque));
    let formulaire = requete(a, "GET", "/entreprises/nouvelle", Some(&session), "").corps;
    assert!(formulaire.contains("class=\"contenu contenu-partage\""));
    assert!(formulaire.contains("class=\"panneau-illustration\" aria-hidden=\"true\""));
    assert!(formulaire.contains("data-motif=\"P\""));
    assert_eq!(formulaire.matches("<h1").count(), 1);
    assert!(!formulaire.contains("style="));

    // Formulaire sans jeton ou avec un faux : refusé, rien n'est écrit.
    let corps = format!("identifiant={ENTREPRISE_A}&client=Client");
    assert_eq!(
        requete(a, "POST", "/entreprises/nouvelle", Some(&session), &corps).statut,
        403
    );
    let faux = format!("{corps}&csrf={}", "0".repeat(64));
    assert_eq!(
        requete(a, "POST", "/entreprises/nouvelle", Some(&session), &faux).statut,
        403
    );
    let page = requete(a, "GET", "/entreprises/nouvelle", Some(&session), "").corps;
    let jeton = csrf(&page);
    let r = requete(
        a,
        "POST",
        "/entreprises/nouvelle",
        Some(&session),
        &format!("{corps}&csrf={jeton}"),
    );
    assert_eq!(r.statut, 303);
    assert_eq!(
        r.entete("location"),
        Some(format!("/entreprises/{ENTREPRISE_A}?m=entreprise").as_str())
    );
    // Le menu se construit depuis les vraies données : l'entreprise créée.
    let accueil = requete(a, "GET", "/", Some(&session), "").corps;
    assert!(accueil.contains("data-mega-cle=\"entreprises\""));
    assert!(accueil.contains("<span class=\"mega-title\">Client</span>"));
    assert!(accueil.contains(&format!("href=\"/entreprises/{ENTREPRISE_A}\"")));
    assert_eq!(
        requete(
            a,
            "GET",
            &format!("/entreprises/{ENTREPRISE_A}"),
            Some(&session),
            ""
        )
        .statut,
        200
    );
    assert_eq!(
        requete(a, "GET", "/entreprises/inconnue", Some(&session), "").statut,
        404
    );

    // Déconnexion : la session ne sert plus.
    let r = requete(
        a,
        "POST",
        "/deconnexion",
        Some(&session),
        &format!("csrf={jeton}"),
    );
    assert_eq!(r.statut, 303);
    assert_eq!(requete(a, "GET", "/", Some(&session), "").statut, 303);

    lance.arreter();
    let _ = std::fs::remove_dir_all(d);
}

/// Lot D2 : connexion protégée contre la falsification, rotation de la
/// session, politique de contenu sans script étranger ni objet.
#[test]
fn connexion_sans_falsification_et_rotation() {
    let d = dossier("connexion-d2");
    let p = Portail::ouvrir(&d, cle_essai()).unwrap();
    // Un revendeur : un éditeur n'a de session complète qu'avec la double
    // authentification (essai double_authentification_des_editeurs).
    let editeur = p.creer_editeur("editeur", "Éditeur", MOT_DE_PASSE).unwrap();
    p.creer_revendeur(&editeur, "revendeur", "Revendeur", MOT_DE_PASSE)
        .unwrap();
    let lance = web::lancer(
        p,
        SocketAddr::from(([127, 0, 0, 1], 0)),
        ConfigWeb::default(),
    )
    .unwrap();
    let a = lance.adresse();
    let politique = requete(a, "GET", "/connexion", None, "");
    let politique = politique.entete("content-security-policy").unwrap();
    // Scripts du portail seulement (le code en six cases) : ni en ligne, ni
    // évalués, ni d'ailleurs.
    assert!(politique.contains("script-src 'self'") && politique.contains("object-src 'none'"));
    assert!(!politique.contains("unsafe-inline") && !politique.contains("unsafe-eval"));
    let identifiants = format!("identifiant=revendeur&mot_de_passe={MOT_DE_PASSE}");

    // Sans jeton, sans cookie, ou avec un jeton qui n'est pas celui du
    // cookie : refusé, sans session.
    let r = requete(a, "POST", "/connexion", None, &identifiants);
    assert_eq!(r.statut, 403);
    let (cookie, jeton) = jeton_connexion(a);
    let r = requete(
        a,
        "POST",
        "/connexion",
        None,
        &format!("{identifiants}&csrf={jeton}"),
    );
    assert_eq!(r.statut, 403);
    let (_, autre) = jeton_connexion(a);
    let r = requete(
        a,
        "POST",
        "/connexion",
        Some(&cookie),
        &format!("{identifiants}&csrf={autre}"),
    );
    assert_eq!(r.statut, 403);
    assert!(
        !r.entete("set-cookie")
            .unwrap_or("")
            .starts_with("__Host-kadenz_portail=")
    );
    // Formulaire envoyé depuis un autre site : refusé même avec le bon jeton.
    let r = requete_avec(
        a,
        "POST",
        "/connexion",
        Some(&cookie),
        &[("Sec-Fetch-Site", "cross-site")],
        &format!("{identifiants}&csrf={jeton}"),
    );
    assert_eq!(r.statut, 403);
    // Depuis le portail lui-même : accepté, le cookie de connexion est effacé.
    let r = requete_avec(
        a,
        "POST",
        "/connexion",
        Some(&cookie),
        &[("Sec-Fetch-Site", "same-origin")],
        &format!("{identifiants}&csrf={jeton}"),
    );
    assert_eq!(r.statut, 303);
    let premiere = morceau_cookie(r.entete("set-cookie").unwrap());
    assert!(
        r.entetes
            .iter()
            .any(|(n, v)| n.eq_ignore_ascii_case("set-cookie")
                && v.starts_with("__Host-kadenz_connexion=;")
                && v.contains("Max-Age=0"))
    );
    assert_eq!(requete(a, "GET", "/", Some(&premiere), "").statut, 200);

    // Nouvelle connexion depuis le même navigateur : session neuve,
    // l'ancienne est fermée.
    let r = se_connecter(a, "revendeur", MOT_DE_PASSE, Some(&premiere));
    assert_eq!(r.statut, 303);
    let seconde = morceau_cookie(r.entete("set-cookie").unwrap());
    assert_ne!(seconde, premiere);
    assert_eq!(requete(a, "GET", "/", Some(&premiere), "").statut, 303);
    assert_eq!(requete(a, "GET", "/", Some(&seconde), "").statut, 200);
    lance.arreter();
    let _ = std::fs::remove_dir_all(d);
}

/// Chaîne du journal : vérifiée par l'API, visible de l'éditeur seul,
/// vérifiée et exportée par le programme.
#[test]
fn chaine_du_journal() {
    let (p, d, editeur, a, _) = portail("chaine", "2026-11-15");
    p.creer_entreprise(&editeur, ENTREPRISE_A, "Boulangerie Wendé", None)
        .unwrap();
    let etat = p.verifier_journal().unwrap();
    assert_eq!(etat.rupture, None);
    assert!(etat.entrees >= 3, "{etat:?}");
    assert_eq!(p.chaine_du_journal(&editeur).unwrap(), Some(etat.clone()));
    assert_eq!(p.chaine_du_journal(&a).unwrap(), None);
    let export = p.exporter_journal(0).unwrap();
    assert_eq!(export.len() as i64, etat.entrees);
    assert_eq!(export.last().unwrap().empreinte, etat.sceau);
    let suite = p.exporter_journal(export[0].id).unwrap();
    assert_eq!(suite.len(), export.len() - 1);
    drop(p);

    let programme = env!("CARGO_BIN_EXE_kadenz-portail");
    let sortie = std::process::Command::new(programme)
        .args(["--dossier", d.to_str().unwrap(), "journal-verifier"])
        .output()
        .unwrap();
    assert!(sortie.status.success());
    let texte = String::from_utf8_lossy(&sortie.stdout).to_string();
    assert!(
        texte.contains("intacte") && texte.contains(&etat.sceau),
        "{texte}"
    );
    let sortie = std::process::Command::new(programme)
        .args(["--dossier", d.to_str().unwrap(), "journal-exporter"])
        .output()
        .unwrap();
    assert!(sortie.status.success());
    let lignes: Vec<serde_json::Value> = String::from_utf8_lossy(&sortie.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lignes.len() as i64, etat.entrees);
    assert_eq!(lignes.last().unwrap()["empreinte"], etat.sceau.as_str());
    let _ = std::fs::remove_dir_all(&d);
}

/// Secondes depuis 1970, pour les codes de l'application d'authentification.
fn maintenant_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Valeur d'un champ d'une page (`name` ou `id`), comme `csrf`.
fn valeur_champ(page: &str, attribut: &str) -> String {
    let debut = page.find(attribut).expect("champ");
    let reste = &page[debut..];
    let valeur = &reste[reste.find("value=\"").unwrap() + 7..];
    valeur[..valeur.find('"').unwrap()].to_string()
}

/// Double authentification des éditeurs : obligatoire avant tout autre
/// usage, mise en place, connexion en deux étapes, code à usage unique,
/// codes de secours, retrait par le programme.
#[test]
fn double_authentification_des_editeurs() {
    let d = dossier("double");
    let p = Portail::ouvrir(&d, cle_essai()).unwrap();
    let editeur = p.creer_editeur("editeur", "Éditeur", MOT_DE_PASSE).unwrap();
    let revendeur = p
        .creer_revendeur(&editeur, "revendeur", "Revendeur", MOT_DE_PASSE)
        .unwrap();
    // Réservée aux éditeurs pour l'instant, et exigée d'eux seuls.
    assert_eq!(
        p.preparer_double_authentification(&revendeur),
        Err(ErreurPortail::Interdit)
    );
    assert!(p.mise_en_place_exigee(&editeur));
    assert!(!p.mise_en_place_exigee(&revendeur));
    let lance = web::lancer(
        p.clone(),
        SocketAddr::from(([127, 0, 0, 1], 0)),
        ConfigWeb::default(),
    )
    .unwrap();
    let a = lance.adresse();

    // Sans double authentification : session limitée à la mise en place.
    let r = se_connecter(a, "editeur", MOT_DE_PASSE, None);
    assert_eq!(r.statut, 303);
    let limitee = morceau_cookie(r.entete("set-cookie").unwrap());
    for chemin in ["/", "/codes", "/journal", "/comptes"] {
        let r = requete(a, "GET", chemin, Some(&limitee), "");
        assert_eq!(r.statut, 303, "{chemin}");
        assert_eq!(r.entete("location"), Some("/double-authentification"));
    }
    let page = requete(a, "GET", "/double-authentification", Some(&limitee), "");
    assert_eq!(page.statut, 200);
    let jeton = csrf(&page.corps);
    // Un POST qui agit avec les droits de l'éditeur est renvoyé aussi.
    let r = requete(
        a,
        "POST",
        "/codes",
        Some(&limitee),
        &format!("gamme=entreprise&nombre=1&postes=2&formule=annuelle&csrf={jeton}"),
    );
    assert_eq!(r.entete("location"), Some("/double-authentification"));
    assert!(p.codes_a_vendre(&editeur).unwrap().is_empty());
    let secret = valeur_champ(&page.corps, "id=\"cle\"");
    // Page rechargée : même clé.
    let encore = requete(a, "GET", "/double-authentification", Some(&limitee), "");
    assert_eq!(valeur_champ(&encore.corps, "id=\"cle\""), secret);
    // Code faux, puis le bon : codes de secours montrés.
    let r = requete(
        a,
        "POST",
        "/double-authentification",
        Some(&limitee),
        &format!("code=000000&csrf={jeton}"),
    );
    assert!(r.corps.contains("code incorrect"), "{}", r.corps);
    let code = kadenz_portail::totp::code_pour(&secret, maintenant_unix()).unwrap();
    let r = requete(
        a,
        "POST",
        "/double-authentification",
        Some(&limitee),
        &format!("code={code}&csrf={jeton}"),
    );
    assert_eq!(r.statut, 200);
    let secours: Vec<String> = r
        .corps
        .split("<span class=\"code-activation\">")
        .skip(1)
        .map(|m| m[..m.find('<').unwrap()].to_string())
        .collect();
    assert_eq!(secours.len(), kadenz_portail::CODES_SECOURS);
    assert!(!p.mise_en_place_exigee(&editeur));
    assert_eq!(requete(a, "GET", "/", Some(&limitee), "").statut, 200);

    // Connexion suivante : le mot de passe mène à l'étape du code, sans
    // session.
    let etape_de = |r: &Reponse| -> String {
        assert_eq!(r.statut, 303);
        assert_eq!(r.entete("location"), Some("/connexion/code"));
        let cookie = morceau_cookie(r.entete("set-cookie").unwrap());
        assert!(cookie.starts_with("__Host-kadenz_etape="), "{cookie}");
        cookie
    };
    let etape = etape_de(&se_connecter(a, "editeur", MOT_DE_PASSE, None));
    let page = requete(a, "GET", "/connexion/code", Some(&etape), "");
    assert_eq!(page.statut, 200);
    // Six cases par le script du portail ; le champ unique reste sans lui.
    let script = kadenz_portail::pages::SCRIPT_CODE;
    assert!(page.corps.contains(&format!("src=\"{script}\"")));
    assert!(page.corps.contains("id=\"code\""));
    let js = requete(a, "GET", script, None, "");
    assert_eq!(js.statut, 200);
    assert_eq!(
        js.entete("content-type"),
        Some("text/javascript; charset=utf-8")
    );
    assert!(js.corps.contains("/connexion/code"));
    let champ_etape = valeur_champ(&page.corps, "name=\"etape\"");
    let poster = |etape: &str, champ: &str, code: &str, en_plus: &[(&str, &str)]| {
        requete_avec(
            a,
            "POST",
            "/connexion/code",
            Some(etape),
            en_plus,
            &format!("etape={champ}&code={code}"),
        )
    };
    // Sans le jeton d'étape dans le formulaire, ou depuis un autre site :
    // refusé.
    let suivant = kadenz_portail::totp::code_pour(&secret, maintenant_unix() + 30).unwrap();
    assert_eq!(poster(&etape, "autre", &suivant, &[]).statut, 403);
    assert_eq!(
        poster(
            &etape,
            &champ_etape,
            &suivant,
            &[("Sec-Fetch-Site", "cross-site")]
        )
        .statut,
        403
    );
    // Code faux : la page du code revient avec son message.
    let r = poster(&etape, &champ_etape, "123 456", &[]);
    assert_eq!(r.statut, 401);
    assert!(r.corps.contains("code incorrect"));
    // Bon code (pas suivant, dans la fenêtre permise) : session ouverte.
    let r = poster(&etape, &champ_etape, &suivant, &[]);
    assert_eq!(r.statut, 303);
    let session = morceau_cookie(r.entete("set-cookie").unwrap());
    assert!(session.starts_with("__Host-kadenz_portail="));
    assert_eq!(requete(a, "GET", "/", Some(&session), "").statut, 200);

    // Le même code ne sert pas deux fois.
    let etape = etape_de(&se_connecter(a, "editeur", MOT_DE_PASSE, None));
    let champ_etape = morceau_cookie(&etape)
        .split_once('=')
        .unwrap()
        .1
        .to_string();
    let r = poster(&etape, &champ_etape, &suivant, &[]);
    assert_eq!(r.statut, 401);
    // Code de secours, saisi en minuscules : accepté une fois.
    let saisi = secours[0].to_lowercase();
    let r = poster(&etape, &champ_etape, &saisi, &[]);
    assert_eq!(r.statut, 303);
    let etape = etape_de(&se_connecter(a, "editeur", MOT_DE_PASSE, None));
    let champ_etape = etape.split_once('=').unwrap().1.to_string();
    assert_eq!(poster(&etape, &champ_etape, &saisi, &[]).statut, 401);

    // Le revendeur se connecte toujours en une étape.
    let r = se_connecter(a, "revendeur", MOT_DE_PASSE, None);
    let cookie = morceau_cookie(r.entete("set-cookie").unwrap());
    assert!(cookie.starts_with("__Host-kadenz_portail="));
    assert_eq!(requete(a, "GET", "/", Some(&cookie), "").statut, 200);

    // Retrait par le programme sur le serveur : journalisé, chaîne intacte,
    // mise en place de nouveau exigée.
    let programme = env!("CARGO_BIN_EXE_kadenz-portail");
    let sortie = std::process::Command::new(programme)
        .args([
            "--dossier",
            d.to_str().unwrap(),
            "retirer-double-authentification",
            "editeur",
        ])
        .output()
        .unwrap();
    assert!(sortie.status.success(), "{sortie:?}");
    assert!(p.mise_en_place_exigee(&editeur));
    assert_eq!(requete(a, "GET", "/", Some(&session), "").statut, 303);
    assert_eq!(
        p.retirer_double_authentification("editeur"),
        Err(ErreurPortail::Validation(
            "aucune double authentification pour ce compte".into()
        ))
    );
    let etat = p.verifier_journal().unwrap();
    assert_eq!(etat.rupture, None);
    let journal = p.journal(&editeur);
    // Session de l'éditeur sans double authentification : le journal reste
    // lisible par l'API, qui n'est pas une route.
    let actions: Vec<String> = journal.unwrap().into_iter().map(|j| j.action).collect();
    for attendue in [
        "double_authentification_activee",
        "connexion_etape",
        "code_refuse",
        "double_authentification_retiree",
    ] {
        assert!(actions.iter().any(|x| x == attendue), "{attendue}");
    }
    lance.arreter();
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn programme_refuse_sans_cle() {
    // Le vrai programme ne démarre jamais sans fichier de clé.
    let d = dossier("sans-cle");
    let sortie = std::process::Command::new(env!("CARGO_BIN_EXE_kadenz-portail"))
        .args(["--dossier", d.to_str().unwrap(), "--ecoute", "127.0.0.1:0"])
        .output()
        .unwrap();
    assert!(!sortie.status.success());
    assert!(String::from_utf8_lossy(&sortie.stderr).contains("clé du portail manquante"));
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn programme_refuse_une_cle_inconnue() {
    // Clé valide mais inconnue de Kadenz (la clé d'essai) : ses licences
    // seraient refusées partout, le programme ne démarre pas.
    let d = dossier("cle-inconnue");
    std::fs::create_dir_all(&d).unwrap();
    let fichier = d.join("cle.hex");
    std::fs::write(&fichier, kadenz_licence::en_hex(&cle_essai().to_bytes())).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fichier, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let sortie = std::process::Command::new(env!("CARGO_BIN_EXE_kadenz-portail"))
        .args([
            "--dossier",
            d.to_str().unwrap(),
            "--cle",
            fichier.to_str().unwrap(),
            "--ecoute",
            "127.0.0.1:0",
        ])
        .output()
        .unwrap();
    assert!(!sortie.status.success());
    assert!(String::from_utf8_lossy(&sortie.stderr).contains("pas une clé du portail reconnue"));
    let _ = std::fs::remove_dir_all(d);
}

/// Sous Unix, un fichier de clé lisible par le groupe ou par tous est refusé
/// avant d'être lu.
#[cfg(unix)]
#[test]
fn programme_refuse_une_cle_lisible_par_d_autres() {
    use std::os::unix::fs::PermissionsExt;
    let d = dossier("cle-ouverte");
    std::fs::create_dir_all(&d).unwrap();
    let fichier = d.join("cle.hex");
    std::fs::write(&fichier, kadenz_licence::en_hex(&cle_essai().to_bytes())).unwrap();
    std::fs::set_permissions(&fichier, std::fs::Permissions::from_mode(0o644)).unwrap();
    let sortie = std::process::Command::new(env!("CARGO_BIN_EXE_kadenz-portail"))
        .args([
            "--dossier",
            d.to_str().unwrap(),
            "--cle",
            fichier.to_str().unwrap(),
            "--ecoute",
            "127.0.0.1:0",
        ])
        .output()
        .unwrap();
    assert!(!sortie.status.success());
    assert!(String::from_utf8_lossy(&sortie.stderr).contains("chmod 400"));
    let _ = std::fs::remove_dir_all(d);
}

/// « kadenz-portail sante » : succès contre un portail qui répond, échec sur
/// un port fermé.
#[test]
fn programme_sante() {
    let d = dossier("sante");
    let p = Portail::ouvrir(&d, cle_essai()).unwrap();
    let lance = web::lancer(
        p,
        SocketAddr::from(([127, 0, 0, 1], 0)),
        ConfigWeb::default(),
    )
    .unwrap();
    let adresse = lance.adresse().to_string();
    let sante = |cible: &str| {
        std::process::Command::new(env!("CARGO_BIN_EXE_kadenz-portail"))
            .args(["sante", "--ecoute", cible])
            .output()
            .unwrap()
    };
    assert!(sante(&adresse).status.success());
    // Port libéré juste avant : personne n'y écoute.
    let libre = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let ferme = libre.local_addr().unwrap().to_string();
    drop(libre);
    assert!(!sante(&ferme).status.success());
    let _ = std::fs::remove_dir_all(d);
}

// ----- Codes d'activation (décision 74) -----

fn demande(code: &str, entreprise: &str) -> DemandeActivation {
    DemandeActivation {
        code: code.into(),
        entreprise: entreprise.into(),
        client: "Boulangerie Wendé".into(),
        version: "1.0.1".into(),
        poste: String::new(),
    }
}

/// Demande d'un poste de la ligne V1 (0.2.3).
fn demande_v1(code: &str, poste: &str) -> DemandeActivation {
    DemandeActivation {
        code: code.into(),
        entreprise: String::new(),
        client: "Pharmacie Nabiiga".into(),
        version: "0.2.3".into(),
        poste: poste.into(),
    }
}

const POSTE_1: &str = "ABCD-EFGH-JKMN-PQRS";
const POSTE_2: &str = "0123-4567-89AB-CDEF";

/// Licence V1 reçue : datée, pour ce poste, signée par la clé du portail et
/// acceptée par la règle de clé par version (`verifier_selon`).
fn licence_v1_lue(p: &Portail, texte: &str, poste: &str) -> kadenz_licence::Licence {
    kadenz_licence::verifier_selon(texte, poste, &[], &[p.cle_publique()]).unwrap()
}

#[test]
fn code_v1_active_puis_libere() {
    let (p, d, editeur, a, _) = portail("codes-v1", "2026-11-15");
    // Un code V1 vaut un poste.
    assert!(matches!(
        p.creer_codes_gamme(&a, "v1", "1", "3", "annuelle", &Module::TOUS),
        Err(ErreurPortail::Validation(_))
    ));
    let code = p
        .creer_codes_gamme(&a, "v1", "1", "1", "annuelle", &[Module::Devis])
        .unwrap()
        .remove(0);
    assert_eq!(code.fiche.gamme, kadenz_portail::Gamme::V1);

    // Code V1 présenté comme un code d'entreprise (logiciel 1.x) : refus [5].
    assert_eq!(
        refus(p.activer(&demande(&code.code, ENTREPRISE_A), "10.0.3.1")),
        MODELES_ACTIVATION[5]
    );
    // Demande avec les deux, ou sans aucun : invalide [4].
    let mut les_deux = demande_v1(&code.code, POSTE_1);
    les_deux.entreprise = ENTREPRISE_A.into();
    assert_eq!(
        refus(p.activer(&les_deux, "10.0.3.1")),
        MODELES_ACTIVATION[4]
    );
    assert_eq!(
        refus(p.activer(&demande_v1(&code.code, "pas-un-poste"), "10.0.3.1")),
        MODELES_ACTIVATION[4]
    );

    // Première activation : licence V1 datée, une année moins un jour.
    let premiere = p
        .activer(&demande_v1(&code.code, POSTE_1), "10.0.3.2")
        .unwrap();
    assert_eq!(
        (
            premiere.postes,
            premiere.debut.as_str(),
            premiere.echeance.as_str()
        ),
        (1, "2026-11-15", "2027-11-14")
    );
    let lue = licence_v1_lue(&p, &premiere.licence, POSTE_1);
    assert_eq!(lue.echeance.as_deref(), Some("2027-11-14"));
    assert_eq!(lue.modules, vec![Module::Devis]);
    assert_eq!(lue.client, "Pharmacie Nabiiga");
    // Le poste en minuscules est le même poste : même licence.
    let encore = p
        .activer(&demande_v1(&code.code, &POSTE_1.to_lowercase()), "10.0.3.3")
        .unwrap();
    assert_eq!(encore.licence, premiere.licence);
    // Autre poste pendant que le code est lié : refus [6].
    assert_eq!(
        refus(p.activer(&demande_v1(&code.code, POSTE_2), "10.0.3.4")),
        MODELES_ACTIVATION[6]
    );
    // Un revendeur qui n'a pas tiré le code ne le voit pas ; l'éditeur et le
    // revendeur qui l'a tiré le libèrent.
    let fiche = p.code(&a, code.fiche.id).unwrap();
    assert_eq!(fiche.poste.as_deref(), Some(POSTE_1));
    let rev_b = p.connecter("rev-b", MOT_DE_PASSE, "10.0.3.5").unwrap().1;
    assert_eq!(
        p.liberer_poste(&rev_b, code.fiche.id),
        Err(ErreurPortail::Introuvable)
    );
    let liberee = p.liberer_poste(&a, code.fiche.id).unwrap();
    assert_eq!(liberee.poste, None);
    assert!(matches!(
        p.liberer_poste(&editeur, code.fiche.id),
        Err(ErreurPortail::Validation(_))
    ));

    // Nouveau poste : licence pour lui, même échéance que la première.
    let seconde = p
        .activer(&demande_v1(&code.code, POSTE_2), "10.0.3.6")
        .unwrap();
    assert_eq!(seconde.echeance, "2027-11-14");
    assert_eq!(seconde.debut, "2026-11-15");
    let lue = licence_v1_lue(&p, &seconde.licence, POSTE_2);
    assert_eq!(lue.poste, POSTE_2);
    // L'ancien poste est refusé à son tour.
    assert_eq!(
        refus(p.activer(&demande_v1(&code.code, POSTE_1), "10.0.3.7")),
        MODELES_ACTIVATION[6]
    );
    // Un code V1 activé ne s'annule pas ; un code d'entreprise ne se libère pas.
    assert!(matches!(
        p.annuler_code(&a, code.fiche.id),
        Err(ErreurPortail::Validation(_))
    ));
    let entreprise = p
        .creer_codes(&a, "1", "2", "annuelle", &Module::TOUS)
        .unwrap()
        .remove(0);
    assert_eq!(entreprise.fiche.gamme, kadenz_portail::Gamme::Entreprise);
    assert!(matches!(
        p.liberer_poste(&a, entreprise.fiche.id),
        Err(ErreurPortail::Validation(_))
    ));
    // Code d'entreprise présenté par un poste V1 : refus [5].
    assert_eq!(
        refus(p.activer(&demande_v1(&entreprise.code, POSTE_1), "10.0.3.8")),
        MODELES_ACTIVATION[5]
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Route JSON : un poste V1 active son code avec le champ `poste`.
#[test]
fn activation_v1_par_la_route_json() {
    let (p, d, _, a, _) = portail("codes-v1-json", "2026-11-15");
    let code = p
        .creer_codes_gamme(&a, "v1", "1", "1", "mensuelle", &Module::TOUS)
        .unwrap()
        .remove(0);
    let cle = p.cle_publique();
    let lance = web::lancer(
        p,
        SocketAddr::from(([127, 0, 0, 1], 0)),
        ConfigWeb::default(),
    )
    .unwrap();
    let corps = serde_json::json!({
        "code": code.code,
        "poste": POSTE_1,
        "client": "Pharmacie Nabiiga",
        "version": "0.2.3",
    })
    .to_string();
    let r = requete(lance.adresse(), "POST", "/api/activation", None, &corps);
    assert_eq!(r.statut, 200, "{}", r.corps);
    let v: serde_json::Value = serde_json::from_str(&r.corps).unwrap();
    assert_eq!(v["postes"], 1);
    assert_eq!(v["echeance"], "2026-12-14");
    let texte = v["licence"].as_str().unwrap();
    assert!(kadenz_licence::verifier_selon(texte, POSTE_1, &[], &[cle]).is_ok());
    lance.arreter();
    let _ = std::fs::remove_dir_all(&d);
}

fn refus(r: Result<Activation, ErreurPortail>) -> String {
    match r {
        Err(ErreurPortail::Validation(m)) => m,
        autre => panic!("refus attendu : {autre:?}"),
    }
}

#[test]
fn code_tire_puis_active() {
    let (p, d, editeur, a, _) = portail("codes", "2026-11-15");
    p.creer_entreprise(&editeur, ENTREPRISE_A, "Boulangerie Wendé", Some(a.id))
        .unwrap();
    let paiement = p
        .enregistrer_paiement(&a, ENTREPRISE_A, &saisie("3", "annuelle"))
        .unwrap();
    let modules = [Module::TOUS[0]];
    let cree = p.creer_code(&a, paiement.id, &modules).unwrap();
    assert_eq!(cree.code.len(), 29);
    assert!(kadenz_licence::code_court::verifier(&cree.code).is_ok());
    assert_eq!(cree.fiche.etat, EtatCode::Libre);
    assert_eq!(cree.fiche.entreprise.as_deref(), Some(ENTREPRISE_A));
    assert_eq!(cree.fiche.postes, 3);
    assert_eq!(cree.code[24..], cree.fiche.fin);

    // Un paiement ne sert qu'une fois : ni second code, ni licence longue.
    assert!(p.paiement(&a, paiement.id).unwrap().a_servi());
    assert!(matches!(
        p.creer_code(&a, paiement.id, &modules),
        Err(ErreurPortail::Validation(_))
    ));
    assert!(matches!(
        p.emettre(&a, paiement.id, &modules, "2027-11-14"),
        Err(ErreurPortail::Validation(_))
    ));
    assert_eq!(p.entreprises(&a).unwrap()[0].paiements_libres, 0);

    // Refus : code inconnu, code mal saisi, autre entreprise.
    let inconnu = kadenz_licence::code_court::tirer().unwrap();
    assert_eq!(
        refus(p.activer(&demande(&inconnu, ENTREPRISE_A), "10.0.0.1")),
        MODELES_ACTIVATION[0]
    );
    assert_eq!(
        refus(p.activer(&demande("ABCDE", ENTREPRISE_A), "10.0.0.1")),
        MODELES_ACTIVATION[0]
    );
    assert_eq!(
        refus(p.activer(&demande(&cree.code, "pas-un-uuid"), "10.0.0.1")),
        MODELES_ACTIVATION[4]
    );
    assert_eq!(
        refus(p.activer(&demande(&cree.code, ENTREPRISE_B), "10.0.0.1")),
        MODELES_ACTIVATION[1]
    );

    // Activation : code collé en minuscules sans tirets ; échéance d'une
    // première licence, un an moins un jour.
    let colle = cree.code.to_lowercase().replace('-', "");
    let activation = p
        .activer(&demande(&colle, ENTREPRISE_A), "10.0.0.2")
        .unwrap();
    assert_eq!(activation.postes, 3);
    assert_eq!(activation.debut, "2026-11-15");
    assert_eq!(activation.echeance, "2027-11-14");
    let lue = kadenz_licence::entreprise::verifier(
        &activation.licence,
        ENTREPRISE_A,
        &[p.cle_publique()],
    )
    .unwrap();
    assert_eq!(lue.postes, 3);
    assert_eq!(lue.modules, modules.to_vec());

    // Le code est activé, lié à la licence émise au nom du revendeur.
    let fiche = p.fiche(&a, ENTREPRISE_A).unwrap();
    assert_eq!(fiche.codes.len(), 1);
    assert_eq!(fiche.codes[0].etat, EtatCode::Active);
    assert_eq!(fiche.licences.len(), 1);
    assert_eq!(fiche.codes[0].licence, Some(fiche.licences[0].id));
    assert_eq!(fiche.licences[0].emetteur, "Revendeur A");
    assert_eq!(fiche.licences[0].par, "Revendeur A");

    // Réinstallation par la même entreprise : la même licence, rien de neuf.
    let encore = p
        .activer(&demande(&cree.code, ENTREPRISE_A), "10.0.0.3")
        .unwrap();
    assert_eq!(encore, activation);
    assert_eq!(p.fiche(&a, ENTREPRISE_A).unwrap().licences.len(), 1);
    assert_eq!(
        refus(p.activer(&demande(&cree.code, ENTREPRISE_B), "10.0.0.1")),
        MODELES_ACTIVATION[1]
    );

    // Renouvellement : le code suivant prolonge l'échéance qui court.
    let second = p
        .enregistrer_paiement(&a, ENTREPRISE_A, &saisie("5", "mensuelle"))
        .unwrap();
    let code2 = p.creer_code(&editeur, second.id, &Module::TOUS).unwrap();
    let renouvellement = p
        .activer(&demande(&code2.code, ENTREPRISE_A), "10.0.0.2")
        .unwrap();
    assert_eq!(renouvellement.echeance, "2027-12-14");
    assert_eq!(renouvellement.postes, 5);

    // Journal : tirages, activation, réinstallation, refus.
    let actions: Vec<String> = p
        .journal(&editeur)
        .unwrap()
        .into_iter()
        .map(|a| a.action)
        .collect();
    for attendu in [
        "code_cree",
        "code_active",
        "code_reactive",
        "activation_refusee",
    ] {
        assert!(
            actions.iter().any(|a| a == attendu),
            "{attendu} absent du journal"
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn activation_freinee() {
    let (p, d, _, _, _) = portail("codes-frein", "2026-11-15");
    let faux = kadenz_licence::code_court::tirer().unwrap();
    for _ in 0..5 {
        assert!(matches!(
            p.activer(&demande(&faux, ENTREPRISE_A), "10.0.0.9"),
            Err(ErreurPortail::Validation(_))
        ));
    }
    assert!(matches!(
        p.activer(&demande(&faux, ENTREPRISE_A), "10.0.0.9"),
        Err(ErreurPortail::Freine(_))
    ));
    // Même entreprise depuis une autre adresse : pas freinée par les échecs
    // de la première (un tiers ne bloque pas l'activation d'un client).
    assert!(matches!(
        p.activer(&demande(&faux, ENTREPRISE_A), "10.0.0.10"),
        Err(ErreurPortail::Validation(_))
    ));
    // Identifiants d'entreprise inventés : seule l'adresse est freinée, aucune
    // clé n'est créée pour un texte qui n'est pas un identifiant.
    for k in 0..5 {
        let _ = p.activer(&demande(&faux, &format!("pas-un-uuid-{k}")), "10.0.0.11");
    }
    assert!(matches!(
        p.activer(&demande(&faux, ENTREPRISE_B), "10.0.0.11"),
        Err(ErreurPortail::Freine(_))
    ));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn activation_par_la_route_json() {
    let (p, d, editeur, _, _) = portail("codes-web", "2026-11-15");
    p.creer_entreprise(&editeur, ENTREPRISE_A, "Boulangerie Wendé", None)
        .unwrap();
    let paiement = p
        .enregistrer_paiement(&editeur, ENTREPRISE_A, &saisie("2", "annuelle"))
        .unwrap();
    let cree = p.creer_code(&editeur, paiement.id, &Module::TOUS).unwrap();
    let lance = web::lancer(
        p,
        SocketAddr::from(([127, 0, 0, 1], 0)),
        ConfigWeb::default(),
    )
    .unwrap();
    let a = lance.adresse();

    let r = requete(a, "POST", "/api/activation", None, "pas du json");
    assert_eq!(r.statut, 400);
    let v: serde_json::Value = serde_json::from_str(&r.corps).unwrap();
    assert_eq!(v["erreur"], MODELES_ACTIVATION[4]);

    let corps = serde_json::json!({
        "code": cree.code, "entreprise": ENTREPRISE_B, "client": "X", "version": "1.0.1"
    })
    .to_string();
    let r = requete(a, "POST", "/api/activation", None, &corps);
    assert_eq!(r.statut, 422);
    let v: serde_json::Value = serde_json::from_str(&r.corps).unwrap();
    assert_eq!(v["erreur"], MODELES_ACTIVATION[1]);

    let corps = serde_json::json!({
        "code": cree.code, "entreprise": ENTREPRISE_A,
        "client": "Boulangerie Wendé", "version": "1.0.1"
    })
    .to_string();
    let r = requete(a, "POST", "/api/activation", None, &corps);
    assert_eq!(r.statut, 200, "{}", r.corps);
    assert!(
        r.entete("content-type")
            .unwrap()
            .starts_with("application/json")
    );
    assert_eq!(r.entete("cache-control"), Some("no-store"));
    let v: serde_json::Value = serde_json::from_str(&r.corps).unwrap();
    assert!(v["licence"].as_str().unwrap().starts_with("KADENZ-E-"));
    assert_eq!(v["postes"], 2);
    assert_eq!(v["formule"], "annuelle");
    assert_eq!(v["echeance"], "2027-11-14");
    lance.arreter();
    let _ = std::fs::remove_dir_all(&d);
}

/// Bout en bout : le client d'activation des programmes de Kadenz contre le
/// portail lancé sur 127.0.0.1 avec une clé d'essai.
#[test]
fn client_d_activation_contre_le_portail() {
    let (p, d, editeur, _, _) = portail("codes-client", "2026-11-15");
    p.creer_entreprise(&editeur, ENTREPRISE_A, "Boulangerie Wendé", None)
        .unwrap();
    let paiement = p
        .enregistrer_paiement(&editeur, ENTREPRISE_A, &saisie("4", "annuelle"))
        .unwrap();
    let cree = p.creer_code(&editeur, paiement.id, &Module::TOUS).unwrap();
    let cle = p.cle_publique();
    let lance = web::lancer(
        p,
        SocketAddr::from(([127, 0, 0, 1], 0)),
        ConfigWeb::default(),
    )
    .unwrap();
    let adresse = format!("http://{}", lance.adresse());
    use kadenz_maj::activation::activer_aupres;

    // Refus du portail, rendus avec leur modèle.
    let e = activer_aupres(&adresse, &cree.code, ENTREPRISE_B, "Autre").unwrap_err();
    assert_eq!(e.modele, MODELES_ACTIVATION[1]);

    // Code saisi avec des espaces et en minuscules.
    let saisi = cree.code.to_lowercase().replace('-', " ");
    let a = activer_aupres(&adresse, &saisi, ENTREPRISE_A, "Boulangerie Wendé").unwrap();
    assert_eq!((a.postes, a.formule.as_str()), (4, "annuelle"));
    assert_eq!(
        (a.debut.as_str(), a.echeance.as_str()),
        ("2026-11-15", "2027-11-14")
    );
    let lue = kadenz_licence::entreprise::verifier(&a.licence, ENTREPRISE_A, &[cle]).unwrap();
    assert_eq!(lue.postes, 4);
    lance.arreter();
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn codes_a_vendre_et_annulation() {
    let (p, d, editeur, a, b) = portail("codes-vente", "2026-11-15");

    // Saisies refusées.
    for (nombre, postes, formule) in [
        ("0", "3", "annuelle"),
        ("51", "3", "annuelle"),
        ("2", "0", "annuelle"),
        ("2", "3", "trimestrielle"),
    ] {
        assert!(matches!(
            p.creer_codes(&a, nombre, postes, formule, &Module::TOUS),
            Err(ErreurPortail::Validation(_))
        ));
    }

    // Trois codes de 2 postes tirés par le revendeur A.
    let codes = p
        .creer_codes(&a, "3", "2", "annuelle", &Module::TOUS)
        .unwrap();
    assert_eq!(codes.len(), 3);
    for k in &codes {
        assert!(kadenz_licence::code_court::verifier(&k.code).is_ok());
        assert_eq!(
            (k.fiche.paiement, k.fiche.entreprise.as_deref()),
            (None, None)
        );
        assert_eq!(k.fiche.etat, EtatCode::Libre);
    }
    assert_eq!(p.codes_a_vendre(&a).unwrap().len(), 3);
    assert_eq!(p.codes_a_vendre(&editeur).unwrap().len(), 3);
    // Le revendeur B ne les voit pas et ne peut pas les annuler.
    assert!(p.codes_a_vendre(&b).unwrap().is_empty());
    assert_eq!(
        p.code(&b, codes[0].fiche.id),
        Err(ErreurPortail::Introuvable)
    );
    assert_eq!(
        p.annuler_code(&b, codes[0].fiche.id),
        Err(ErreurPortail::Introuvable)
    );

    // Annulation par celui qui l'a tiré : le code n'est plus reconnu.
    let annule = p.annuler_code(&a, codes[0].fiche.id).unwrap();
    assert_eq!(annule.etat, EtatCode::Annule);
    assert_eq!(
        refus(p.activer(&demande(&codes[0].code, ENTREPRISE_B), "10.0.0.1")),
        MODELES_ACTIVATION[0]
    );
    assert!(matches!(
        p.annuler_code(&a, codes[0].fiche.id),
        Err(ErreurPortail::Validation(_))
    ));

    // Activation par une entreprise inconnue du portail : créée avec la
    // raison sociale reçue, rattachée au revendeur A ; licence sans paiement.
    let activation = p
        .activer(&demande(&codes[1].code, ENTREPRISE_B), "10.0.0.2")
        .unwrap();
    assert_eq!(
        (activation.postes, activation.echeance.as_str()),
        (2, "2027-11-14")
    );
    let fiche = p.fiche(&a, ENTREPRISE_B).unwrap();
    assert_eq!(fiche.entreprise.client, "Boulangerie Wendé");
    assert_eq!(fiche.entreprise.revendeur.as_ref().map(|r| r.0), Some(a.id));
    assert_eq!(fiche.licences.len(), 1);
    assert_eq!(fiche.licences[0].paiement, None);
    assert_eq!(fiche.codes.len(), 1);
    // Un code activé ne s'annule plus, et ne sert pas à une autre entreprise.
    assert!(matches!(
        p.annuler_code(&editeur, codes[1].fiche.id),
        Err(ErreurPortail::Validation(_))
    ));
    assert_eq!(
        refus(p.activer(&demande(&codes[1].code, ENTREPRISE_A), "10.0.0.3")),
        MODELES_ACTIVATION[1]
    );

    // Code d'un paiement annulé : le paiement redevient libre.
    p.creer_entreprise(&editeur, ENTREPRISE_A, "Client A", None)
        .unwrap();
    let paiement = p
        .enregistrer_paiement(&editeur, ENTREPRISE_A, &saisie("1", "mensuelle"))
        .unwrap();
    let du_paiement = p.creer_code(&editeur, paiement.id, &Module::TOUS).unwrap();
    assert!(p.paiement(&editeur, paiement.id).unwrap().a_servi());
    p.annuler_code(&editeur, du_paiement.fiche.id).unwrap();
    assert!(!p.paiement(&editeur, paiement.id).unwrap().a_servi());
    assert!(p.creer_code(&editeur, paiement.id, &Module::TOUS).is_ok());

    let actions: Vec<String> = p
        .journal(&editeur)
        .unwrap()
        .into_iter()
        .map(|a| a.action)
        .collect();
    for attendu in ["codes_crees", "code_annule", "code_active"] {
        assert!(
            actions.iter().any(|a| a == attendu),
            "{attendu} absent du journal"
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn pages_des_codes_a_vendre() {
    let d = dossier("codes-pages");
    let p = Portail::ouvrir(&d, cle_essai()).unwrap();
    // Un revendeur : un éditeur n'a de session complète qu'avec la double
    // authentification (essai double_authentification_des_editeurs).
    let editeur = p.creer_editeur("editeur", "Éditeur", MOT_DE_PASSE).unwrap();
    p.creer_revendeur(&editeur, "revendeur", "Revendeur", MOT_DE_PASSE)
        .unwrap();
    let lance = web::lancer(
        p,
        SocketAddr::from(([127, 0, 0, 1], 0)),
        ConfigWeb::default(),
    )
    .unwrap();
    let a = lance.adresse();
    let r = se_connecter(a, "revendeur", MOT_DE_PASSE, None);
    let session = morceau_cookie(r.entete("set-cookie").unwrap());

    let page = requete(a, "GET", "/codes", Some(&session), "");
    assert_eq!(page.statut, 200);
    assert!(page.corps.contains("Aucun code à vendre"));
    let jeton = csrf(&page.corps);

    // Saisie refusée : la page revient avec son message.
    let r = requete(
        a,
        "POST",
        "/codes",
        Some(&session),
        &format!("nombre=0&postes=2&formule=annuelle&csrf={jeton}"),
    );
    assert_eq!(r.statut, 422);

    let r = requete(
        a,
        "POST",
        "/codes",
        Some(&session),
        &format!("nombre=2&postes=2&formule=annuelle&module_devis=1&csrf={jeton}"),
    );
    assert_eq!(r.statut, 200);
    assert_eq!(r.entete("cache-control"), Some("no-store"));
    assert_eq!(r.corps.matches("class=\"code-activation\"").count(), 2);

    // Liste : les fins seulement ; annulation du premier code.
    let liste = requete(a, "GET", "/codes", Some(&session), "").corps;
    assert!(liste.contains("href=\"/codes/1\""));
    let r = requete(
        a,
        "POST",
        "/codes/1/annuler",
        Some(&session),
        &format!("csrf={jeton}"),
    );
    assert_eq!(r.statut, 303);
    assert_eq!(r.entete("location"), Some("/codes/1?m=code_annule"));
    let fiche = requete(a, "GET", "/codes/1?m=code_annule", Some(&session), "").corps;
    assert!(fiche.contains("annulé"));
    assert!(!fiche.contains("Annuler ce code"));
    // Sans jeton : refusé.
    assert_eq!(
        requete(a, "POST", "/codes/2/annuler", Some(&session), "").statut,
        403
    );
    lance.arreter();
    let _ = std::fs::remove_dir_all(&d);
}

// ----- Mots de passe (lot D1) -----

#[test]
fn mot_de_passe_long_et_pas_courant() {
    let (p, d, editeur, ..) = portail("mots-de-passe", "2026-11-15");
    // Trop court (14 caractères), puis courant (liste embarquée, majuscules
    // ignorées) : refusés.
    for refuse in ["quatorze-carac", "123456789987654321", "1QAZ2WSX3EDC4RFV"] {
        assert!(
            matches!(
                p.creer_revendeur(&editeur, "rev-z", "Revendeur Z", refuse),
                Err(ErreurPortail::Validation(_))
            ),
            "{refuse}"
        );
    }
    assert!(
        p.creer_revendeur(&editeur, "rev-z", "Revendeur Z", "quinze-caracter")
            .is_ok()
    );
    drop(p);
    let _ = std::fs::remove_dir_all(d);
}

/// Changer son mot de passe ferme toutes les sessions du compte et en rend
/// une nouvelle ; l'ancien mot de passe ne sert plus.
#[test]
fn changement_de_mot_de_passe_ferme_les_sessions() {
    let (p, d, ..) = portail("changement", "2026-11-15");
    let (premiere, editeur) = p.connecter("editeur", MOT_DE_PASSE, "10.0.2.1").unwrap();
    let (seconde, _) = p.connecter("editeur", MOT_DE_PASSE, "10.0.2.2").unwrap();
    let (autre, _) = p.connecter("rev-a", MOT_DE_PASSE, "10.0.2.3").unwrap();
    let nouveau = "nouvelle-phrase-de-passe-34";
    assert!(matches!(
        p.changer_mot_de_passe(&editeur, "mauvais-ancien-mot", nouveau, nouveau),
        Err(ErreurPortail::Validation(_))
    ));
    let neuve = p
        .changer_mot_de_passe(&editeur, MOT_DE_PASSE, nouveau, nouveau)
        .unwrap();
    assert!(p.session(&premiere).is_none());
    assert!(p.session(&seconde).is_none());
    assert!(p.session(&neuve).is_some());
    assert!(p.session(&autre).is_some());
    assert_eq!(
        p.connecter("editeur", MOT_DE_PASSE, "10.0.2.4")
            .map(|r| r.1),
        Err(ErreurPortail::Identifiants)
    );
    assert!(p.connecter("editeur", nouveau, "10.0.2.5").is_ok());
    drop(p);
    let _ = std::fs::remove_dir_all(d);
}

/// Identifiant qu'aucun compte ne peut porter : refusé avant le frein et le
/// journal (audit d'OpenCode, 2026-11-19).
#[test]
fn identifiant_impossible_ni_freine_ni_journal() {
    let (p, d, editeur, ..) = portail("identifiant-impossible", "2026-11-15");
    let avant = p.journal(&editeur).unwrap().len();
    let long = "a".repeat(10_000);
    for _ in 0..10 {
        for identifiant in [long.as_str(), "rev a", "ré-a", ""] {
            assert!(matches!(
                p.connecter(identifiant, "mauvais", "10.0.4.1"),
                Err(ErreurPortail::Identifiants)
            ));
        }
    }
    assert_eq!(p.journal(&editeur).unwrap().len(), avant);
    // Le frein n'a rien compté : connexion valide depuis la même adresse.
    assert!(p.connecter("rev-a", MOT_DE_PASSE, "10.0.4.1").is_ok());
    drop(p);
    let _ = std::fs::remove_dir_all(d);
}

/// Entreprise et version envoyées par le programme client : bornées au
/// journal, activation refusée comme réussie.
#[test]
fn activation_textes_bornes_au_journal() {
    let (p, d, editeur, a, _) = portail("activation-bornee", "2026-11-15");
    let long = "x".repeat(10_000);
    let mut refusee = demande("AAAA", &long);
    refusee.version = long.clone();
    assert!(p.activer(&refusee, "10.0.5.1").is_err());
    let code = p
        .creer_codes_gamme(&a, "v1", "1", "1", "mensuelle", &Module::TOUS)
        .unwrap()
        .remove(0);
    let mut reussie = demande_v1(&code.code, POSTE_1);
    reussie.version = long.clone();
    p.activer(&reussie, "10.0.5.2").unwrap();
    let journal = p.journal(&editeur).unwrap();
    for action in ["activation_refusee", "code_v1_active"] {
        let j = journal
            .iter()
            .find(|j| j.action == action)
            .unwrap_or_else(|| panic!("{action} absent du journal"));
        let longueur = j.details.chars().count();
        assert!(longueur < 300, "{action} : {longueur} caractères");
        assert!(j.details.contains('…'), "{}", j.details);
    }
    assert_eq!(p.verifier_journal().unwrap().rupture, None);
    drop(p);
    let _ = std::fs::remove_dir_all(d);
}

/// Corps annoncé plus long que la limite : 413, sans attendre le corps.
#[test]
fn corps_trop_long_refuse_sans_lecture() {
    let (p, d, ..) = portail("corps-trop-long", "2026-11-15");
    let lance = web::lancer(
        p,
        SocketAddr::from(([127, 0, 0, 1], 0)),
        ConfigWeb::default(),
    )
    .unwrap();
    for chemin in ["/api/activation", "/connexion"] {
        let mut tcp = TcpStream::connect(lance.adresse()).unwrap();
        tcp.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        let tete = format!(
            "POST {chemin} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
            lance.adresse(),
            web::CORPS_MAX + 1
        );
        tcp.write_all(tete.as_bytes()).unwrap();
        let mut brut = Vec::new();
        tcp.read_to_end(&mut brut).unwrap();
        let brut = String::from_utf8_lossy(&brut);
        assert!(brut.starts_with("HTTP/1.1 413"), "{chemin} : {brut}");
    }
    lance.arreter();
    let _ = std::fs::remove_dir_all(d);
}

/// Valeur stockée du secret de double authentification d'un compte.
fn secret_stocke(d: &std::path::Path, compte: i64) -> String {
    let conn = rusqlite::Connection::open(d.join("portail.sqlite")).unwrap();
    conn.query_row(
        "SELECT secret FROM double_authentification WHERE compte = ?1",
        [compte],
        |r| r.get(0),
    )
    .unwrap()
}

/// Lot B : le secret est stocké chiffré ; recopié sur un autre compte dans
/// la base, il ne sert à rien (donnée associée : l'identifiant du compte).
#[test]
fn secret_chiffre_et_lie_a_son_compte() {
    let d = dossier("secret-chiffre");
    let p = Portail::ouvrir(&d, cle_essai()).unwrap();
    let editeur = p.creer_editeur("editeur", "Éditeur", MOT_DE_PASSE).unwrap();
    let revendeur = p
        .creer_revendeur(&editeur, "revendeur", "Revendeur", MOT_DE_PASSE)
        .unwrap();
    let secret = p.preparer_double_authentification(&editeur).unwrap();
    let stocke = secret_stocke(&d, editeur.id);
    assert!(stocke.starts_with("c1:"), "{stocke}");
    assert!(!stocke.contains(&secret));
    // Page rechargée : le même secret, en clair pour l'application.
    assert_eq!(
        p.preparer_double_authentification(&editeur).unwrap(),
        secret
    );
    let code = kadenz_portail::totp::code_pour(&secret, maintenant_unix()).unwrap();
    p.activer_double_authentification(&editeur, &code).unwrap();

    // Le secret de l'éditeur recopié sur le revendeur.
    {
        let conn = rusqlite::Connection::open(d.join("portail.sqlite")).unwrap();
        conn.execute(
            "INSERT INTO double_authentification (compte, secret, active_le, dernier_pas)
             SELECT ?2, secret, active_le, 0 FROM double_authentification WHERE compte = ?1",
            [editeur.id, revendeur.id],
        )
        .unwrap();
    }
    let etape = match p.connecter("revendeur", MOT_DE_PASSE, "10.0.6.1") {
        Err(ErreurPortail::DeuxiemeFacteur(e)) => e,
        autre => panic!("étape du code attendue : {autre:?}"),
    };
    let code = kadenz_portail::totp::code_pour(&secret, maintenant_unix() + 30).unwrap();
    assert!(matches!(
        p.confirmer_code(&etape, &code, "10.0.6.1"),
        Err(ErreurPortail::Interne(_))
    ));
    drop(p);
    let _ = std::fs::remove_dir_all(d);
}

/// Lot B : une base d'avant le chiffrement (secret en clair) est chiffrée au
/// démarrage, sans perte : le même secret reste valable.
#[test]
fn secret_en_clair_chiffre_au_demarrage() {
    let d = dossier("secret-en-clair");
    let p = Portail::ouvrir(&d, cle_essai()).unwrap();
    let editeur = p.creer_editeur("editeur", "Éditeur", MOT_DE_PASSE).unwrap();
    drop(p);
    let secret = "JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP";
    {
        let conn = rusqlite::Connection::open(d.join("portail.sqlite")).unwrap();
        conn.execute(
            "INSERT INTO double_authentification (compte, secret) VALUES (?1, ?2)",
            rusqlite::params![editeur.id, secret],
        )
        .unwrap();
    }
    let p = Portail::ouvrir(&d, cle_essai()).unwrap();
    let stocke = secret_stocke(&d, editeur.id);
    assert!(stocke.starts_with("c1:"), "{stocke}");
    assert_eq!(
        p.preparer_double_authentification(&editeur).unwrap(),
        secret
    );
    let code = kadenz_portail::totp::code_pour(secret, maintenant_unix()).unwrap();
    assert!(p.activer_double_authentification(&editeur, &code).is_ok());
    // Une seconde ouverture ne rechiffre rien.
    drop(p);
    let p = Portail::ouvrir(&d, cle_essai()).unwrap();
    assert_eq!(secret_stocke(&d, editeur.id), stocke);
    drop(p);
    let _ = std::fs::remove_dir_all(d);
}

/// Lot B : une commande d'administration ouvre la base sans la clé du
/// portail ; elle ne doit pas chiffrer un secret en clair avec sa clé
/// jetable (il serait perdu), ni lire un secret chiffré.
#[test]
fn ouverture_sans_cle_laisse_les_secrets() {
    let d = dossier("sans-cle-secrets");
    let p = Portail::ouvrir(&d, cle_essai()).unwrap();
    let editeur = p.creer_editeur("editeur", "Éditeur", MOT_DE_PASSE).unwrap();
    drop(p);
    let secret = "JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP";
    {
        let conn = rusqlite::Connection::open(d.join("portail.sqlite")).unwrap();
        conn.execute(
            "INSERT INTO double_authentification (compte, secret) VALUES (?1, ?2)",
            rusqlite::params![editeur.id, secret],
        )
        .unwrap();
    }
    let outil = Portail::ouvrir_sans_cle(&d).unwrap();
    assert!(outil.verifier_journal().unwrap().rupture.is_none());
    assert_eq!(secret_stocke(&d, editeur.id), secret);
    assert!(matches!(
        outil.preparer_double_authentification(&editeur),
        Err(ErreurPortail::Interne(_))
    ));
    drop(outil);
    // Avec la clé : chiffré, et toujours le même secret.
    let p = Portail::ouvrir(&d, cle_essai()).unwrap();
    let stocke = secret_stocke(&d, editeur.id);
    assert!(stocke.starts_with("c1:"));
    assert_eq!(
        p.preparer_double_authentification(&editeur).unwrap(),
        secret
    );
    drop(p);
    // Avec une autre clé : la valeur chiffrée n'est jamais réécrite, ni au
    // démarrage ni à la mise en place ; la lecture est refusée.
    let autre = Portail::ouvrir(&d, SigningKey::from_bytes(&[6u8; 32])).unwrap();
    assert_eq!(secret_stocke(&d, editeur.id), stocke);
    assert!(matches!(
        autre.preparer_double_authentification(&editeur),
        Err(ErreurPortail::Interne(_))
    ));
    assert_eq!(secret_stocke(&d, editeur.id), stocke);
    drop(autre);
    let outil = Portail::ouvrir_sans_cle(&d).unwrap();
    assert!(matches!(
        outil.preparer_double_authentification(&editeur),
        Err(ErreurPortail::Interne(_))
    ));
    assert_eq!(secret_stocke(&d, editeur.id), stocke);
    // Le retrait reste possible sans la clé (téléphone perdu, clé changée).
    outil.retirer_double_authentification("editeur").unwrap();
    let reste: i64 = rusqlite::Connection::open(d.join("portail.sqlite"))
        .unwrap()
        .query_row("SELECT count(*) FROM double_authentification", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(reste, 0);
    drop(outil);
    let _ = std::fs::remove_dir_all(d);
}
