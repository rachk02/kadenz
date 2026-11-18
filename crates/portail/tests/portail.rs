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

const MOT_DE_PASSE: &str = "motdepasse12";
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
    // Même le bon mot de passe attend, depuis cette adresse ou pour cet
    // identifiant.
    assert!(matches!(
        p.connecter("editeur", MOT_DE_PASSE, "10.0.0.2"),
        Err(ErreurPortail::Freine(_))
    ));
    assert!(matches!(
        p.connecter("rev-a", MOT_DE_PASSE, "10.0.0.1"),
        Err(ErreurPortail::Freine(_))
    ));
    assert!(p.connecter("rev-a", MOT_DE_PASSE, "10.0.0.3").is_ok());
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
    let mut tcp = TcpStream::connect(adresse).unwrap();
    tcp.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    let mut texte =
        format!("{methode} {chemin} HTTP/1.1\r\nHost: {adresse}\r\nConnection: close\r\n");
    if let Some(c) = cookie {
        texte.push_str(&format!("Cookie: {c}\r\n"));
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

#[test]
fn routes_sessions_et_jetons() {
    let d = dossier("web");
    let p = Portail::ouvrir(&d, cle_essai()).unwrap();
    p.creer_editeur("editeur", "Éditeur", MOT_DE_PASSE).unwrap();
    let lance = web::lancer(
        p,
        SocketAddr::from(([127, 0, 0, 1], 0)),
        ConfigWeb::default(),
    )
    .unwrap();
    let a = lance.adresse();

    // Console privée : rien à indexer, en-têtes de sécurité partout.
    let r = requete(a, "GET", "/robots.txt", None, "");
    assert_eq!(r.statut, 200);
    assert!(r.corps.contains("Disallow: /"));
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

    // Mauvais mot de passe, puis le bon.
    let r = requete(
        a,
        "POST",
        "/connexion",
        None,
        "identifiant=editeur&mot_de_passe=faux",
    );
    assert_eq!(r.statut, 401);
    let r = requete(
        a,
        "POST",
        "/connexion",
        None,
        &format!("identifiant=editeur&mot_de_passe={MOT_DE_PASSE}"),
    );
    assert_eq!(r.statut, 303);
    let cookie = r.entete("set-cookie").unwrap().to_string();
    assert!(
        cookie.contains("HttpOnly")
            && cookie.contains("Secure")
            && cookie.contains("SameSite=Strict")
    );
    let session = cookie.split(';').next().unwrap().to_string();
    assert_eq!(requete(a, "GET", "/", Some(&session), "").statut, 200);

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

// ----- Codes d'activation (décision 74) -----

fn demande(code: &str, entreprise: &str) -> DemandeActivation {
    DemandeActivation {
        code: code.into(),
        entreprise: entreprise.into(),
        client: "Boulangerie Wendé".into(),
        version: "1.0.1".into(),
    }
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
    // Même entreprise depuis une autre adresse : freinée aussi.
    assert!(matches!(
        p.activer(&demande(&faux, ENTREPRISE_A), "10.0.0.10"),
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
    p.creer_editeur("editeur", "Éditeur", MOT_DE_PASSE).unwrap();
    let lance = web::lancer(
        p,
        SocketAddr::from(([127, 0, 0, 1], 0)),
        ConfigWeb::default(),
    )
    .unwrap();
    let a = lance.adresse();
    let r = requete(
        a,
        "POST",
        "/connexion",
        None,
        &format!("identifiant=editeur&mot_de_passe={MOT_DE_PASSE}"),
    );
    let session = r
        .entete("set-cookie")
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();

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
