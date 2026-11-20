//! Relais en processus : serveur enregistré, poste distant relié, refus.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

use ed25519_dalek::SigningKey;
use kadenz_licence::entreprise::{self as lic, LicenceEntreprise};
use kadenz_relais::controle::{Controle, Enregistrement, ouvrir_session};
use kadenz_relais::relais::{Config, Relais};
use kadenz_relais::{ErreurRelais, IdentiteRelais, motif, poste};

const ENTREPRISE: &str = "0192f000-0000-7000-8000-000000000001";
const DELAI: Duration = Duration::from_secs(5);

fn cle_portail() -> SigningKey {
    SigningKey::from_bytes(&[5u8; 32])
}

fn licence(entreprise: &str, echeance_jours: i64) -> String {
    let jour = |n: i64| {
        (chrono::Utc::now() + chrono::Duration::days(n))
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
            debut: jour(-30),
            echeance: jour(echeance_jours),
            emetteur: lic::Emetteur::Editeur,
            emise_le: jour(-30),
        },
        &cle_portail(),
    )
    .unwrap()
}

fn dossier(nom: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "kadenz-relais-{nom}-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
    ));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn relais(nom: &str) -> (Relais, PathBuf) {
    let d = dossier(nom);
    let mut config = Config::nouvelle(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        d.clone(),
        vec![cle_portail().verifying_key()],
    );
    config.battement = Duration::from_millis(200);
    config.silence = Duration::from_secs(2);
    (Relais::demarrer(config).unwrap(), d)
}

fn identite(nom: &str) -> IdentiteRelais {
    kadenz_relais::nouvelle_identite(nom).unwrap()
}

fn enregistrement<'a>(
    r: &'a str,
    empreinte_relais: &'a str,
    licence: &'a str,
    serveur: &'a IdentiteRelais,
    postes: Vec<String>,
) -> Enregistrement<'a> {
    Enregistrement {
        relais: r,
        empreinte_relais,
        entreprise: ENTREPRISE,
        licence,
        certificat: &serveur.certificat,
        cle: &serveur.cle,
        postes,
    }
}

/// Serveur d'essai : répond à chaque demande de session par un écho.
struct ServeurEcho {
    arret: Arc<AtomicBool>,
    fil: Option<thread::JoinHandle<()>>,
    postes: mpsc::Sender<Vec<String>>,
}

impl ServeurEcho {
    fn lancer(mut controle: Controle, adresse: String) -> ServeurEcho {
        controle.regler_battement(Duration::from_millis(200), Duration::from_secs(2));
        let arret = Arc::new(AtomicBool::new(false));
        let (postes, a_envoyer) = mpsc::channel::<Vec<String>>();
        let fil = {
            let arret = arret.clone();
            thread::spawn(move || {
                while !arret.load(Ordering::SeqCst) {
                    while let Ok(p) = a_envoyer.try_recv() {
                        controle.envoyer_postes(p).unwrap();
                    }
                    match controle.attendre(Duration::from_millis(50)) {
                        Ok(Some(session)) => {
                            let tcp = ouvrir_session(&adresse, &session, DELAI).unwrap();
                            thread::spawn(move || echo(tcp));
                        }
                        Ok(None) => {}
                        Err(_) => break,
                    }
                }
                controle.fermer();
            })
        };
        ServeurEcho {
            arret,
            fil: Some(fil),
            postes,
        }
    }

    fn arreter(mut self) {
        self.arret.store(true, Ordering::SeqCst);
        if let Some(f) = self.fil.take() {
            f.join().unwrap();
        }
    }
}

fn echo(mut tcp: TcpStream) {
    let mut tampon = [0u8; 1024];
    loop {
        match tcp.read(&mut tampon) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                if tcp.write_all(&tampon[..n]).is_err() {
                    break;
                }
            }
        }
    }
}

fn refus(e: Result<TcpStream, ErreurRelais>) -> String {
    match e {
        Err(ErreurRelais::Refus { motif, .. }) => motif,
        Err(autre) => panic!("refus attendu, obtenu {autre:?}"),
        Ok(_) => panic!("refus attendu, connexion reliée"),
    }
}

#[test]
fn poste_relie_au_serveur_de_son_entreprise() {
    let (r, d) = relais("relie");
    let adresse = r.adresse().to_string();
    let serveur = identite("serveur");
    let poste_autorise = identite("poste");
    let intrus = identite("intrus");
    let lic = licence(ENTREPRISE, 30);

    // Avant l'enregistrement : serveur injoignable.
    assert_eq!(
        refus(poste::ouvrir(
            &adresse,
            ENTREPRISE,
            &poste_autorise.certificat,
            &poste_autorise.cle,
            DELAI
        )),
        motif::SERVEUR_INJOIGNABLE
    );

    let controle = Controle::enregistrer(
        &enregistrement(
            &adresse,
            r.empreinte(),
            &lic,
            &serveur,
            vec![poste_autorise.empreinte()],
        ),
        DELAI,
    )
    .unwrap();
    assert_eq!(r.entreprises(), vec![ENTREPRISE.to_string()]);
    let echo = ServeurEcho::lancer(controle, adresse.clone());

    let mut tcp = poste::ouvrir(
        &adresse,
        ENTREPRISE,
        &poste_autorise.certificat,
        &poste_autorise.cle,
        DELAI,
    )
    .unwrap();
    tcp.write_all(b"\x16bonjour").unwrap();
    let mut lu = [0u8; 8];
    tcp.read_exact(&mut lu).unwrap();
    assert_eq!(&lu, b"\x16bonjour");

    // Un poste non autorisé, ou qui présente le certificat d'un autre sans
    // sa clé, est refusé.
    assert_eq!(
        refus(poste::ouvrir(
            &adresse,
            ENTREPRISE,
            &intrus.certificat,
            &intrus.cle,
            DELAI
        )),
        motif::POSTE_NON_AUTORISE
    );
    assert_eq!(
        refus(poste::ouvrir(
            &adresse,
            ENTREPRISE,
            &poste_autorise.certificat,
            &intrus.cle,
            DELAI
        )),
        motif::POSTE_NON_AUTORISE
    );

    // Poste retiré : sa session est coupée et il est refusé ensuite.
    echo.postes.send(Vec::new()).unwrap();
    thread::sleep(Duration::from_millis(500));
    let mut reste = Vec::new();
    let _ = tcp.read_to_end(&mut reste);
    assert!(reste.is_empty());
    // Le poste ferme de son côté, comme le fait la session du poste.
    drop(tcp);
    assert_eq!(
        refus(poste::ouvrir(
            &adresse,
            ENTREPRISE,
            &poste_autorise.certificat,
            &poste_autorise.cle,
            DELAI
        )),
        motif::POSTE_NON_AUTORISE
    );

    // Serveur coupé : l'enregistrement disparaît, le poste est refusé tout
    // de suite.
    echo.arreter();
    thread::sleep(Duration::from_millis(300));
    assert!(r.entreprises().is_empty());
    assert_eq!(
        refus(poste::ouvrir(
            &adresse,
            ENTREPRISE,
            &poste_autorise.certificat,
            &poste_autorise.cle,
            DELAI
        )),
        motif::SERVEUR_INJOIGNABLE
    );

    // Traces : enregistrement, session et refus, sans contenu.
    r.arreter();
    let traces: String = std::fs::read_dir(d.join("traces"))
        .unwrap()
        .flatten()
        .map(|e| std::fs::read_to_string(e.path()).unwrap())
        .collect();
    assert!(traces.contains("enregistrement entreprise="));
    assert!(traces.contains("session entreprise="));
    assert!(traces.contains("motif=poste_non_autorise"));
    assert!(!traces.contains("bonjour"));
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn enregistrements_refuses() {
    let (r, d) = relais("refus");
    let adresse = r.adresse().to_string();
    let serveur = identite("serveur");
    let refus_controle = |e: Result<Controle, ErreurRelais>| match e {
        Err(ErreurRelais::Refus { motif, valeurs }) => (motif, valeurs),
        Err(autre) => panic!("refus attendu, obtenu {autre:?}"),
        Ok(_) => panic!("refus attendu"),
    };

    // Licence échue (décision 58).
    let echue = licence(ENTREPRISE, -1);
    let (m, valeurs) = refus_controle(Controle::enregistrer(
        &enregistrement(&adresse, r.empreinte(), &echue, &serveur, Vec::new()),
        DELAI,
    ));
    assert_eq!(m, motif::LICENCE_ECHUE);
    assert_eq!(valeurs.len(), 1);

    // Licence d'une autre entreprise.
    let autre = licence("0192f000-0000-7000-8000-000000000002", 30);
    let (m, _) = refus_controle(Controle::enregistrer(
        &enregistrement(&adresse, r.empreinte(), &autre, &serveur, Vec::new()),
        DELAI,
    ));
    assert_eq!(m, motif::LICENCE_REFUSEE);

    // Relais qui ne présente pas le certificat épinglé.
    let lic = licence(ENTREPRISE, 30);
    let faux = "00".repeat(32);
    assert!(matches!(
        Controle::enregistrer(
            &enregistrement(&adresse, &faux, &lic, &serveur, Vec::new()),
            DELAI
        ),
        Err(ErreurRelais::CertificatDifferent)
    ));

    // Un autre serveur de la même entreprise est refusé tant que le premier
    // est vivant ; le même serveur (même certificat) le remplace.
    let premier = Controle::enregistrer(
        &enregistrement(&adresse, r.empreinte(), &lic, &serveur, Vec::new()),
        DELAI,
    )
    .unwrap();
    let autre_serveur = identite("autre-serveur");
    let (m, _) = refus_controle(Controle::enregistrer(
        &enregistrement(&adresse, r.empreinte(), &lic, &autre_serveur, Vec::new()),
        DELAI,
    ));
    assert_eq!(m, motif::DEJA_ENREGISTREE);
    let second = Controle::enregistrer(
        &enregistrement(&adresse, r.empreinte(), &lic, &serveur, Vec::new()),
        DELAI,
    )
    .unwrap();
    assert_eq!(r.entreprises(), vec![ENTREPRISE.to_string()]);
    drop(premier);
    second.fermer();
    r.arreter();
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn trames_au_compte_gouttes_coupees() {
    let (r, d) = relais("gouttes");
    let mut tcp = TcpStream::connect(r.adresse()).unwrap();
    let debut = std::time::Instant::now();
    // Longueur annoncée, puis un octet par seconde : chaque lecture du relais
    // reçoit quelque chose avant 5 secondes, mais le total les dépasse.
    let _ = tcp.write_all(&[0x00, 0x10]);
    for _ in 0..7 {
        thread::sleep(Duration::from_secs(1));
        let _ = tcp.write_all(b"x");
    }
    tcp.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    let mut tampon = [0u8; 16];
    match tcp.read(&mut tampon) {
        Ok(0) => {}
        Ok(n) => panic!("{n} octets reçus"),
        Err(e) => assert!(
            !matches!(
                e.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ),
            "connexion encore ouverte après {:?}",
            debut.elapsed()
        ),
    }
    r.arreter();
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn licence_echue_pendant_la_connexion() {
    let d = dossier("echeance");
    let jour = |n: i64| {
        (chrono::Utc::now() + chrono::Duration::days(n))
            .format("%Y-%m-%d")
            .to_string()
    };
    let date = Arc::new(std::sync::Mutex::new(jour(0)));
    let mut config = Config::nouvelle(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        d.clone(),
        vec![cle_portail().verifying_key()],
    );
    config.traces = None;
    config.battement = Duration::from_millis(200);
    {
        let date = date.clone();
        config.date_utc = Arc::new(move || date.lock().unwrap().clone());
    }
    let r = Relais::demarrer(config).unwrap();
    let adresse = r.adresse().to_string();
    let serveur = identite("serveur");
    let lic = licence(ENTREPRISE, 1);
    let mut controle = Controle::enregistrer(
        &enregistrement(&adresse, r.empreinte(), &lic, &serveur, Vec::new()),
        DELAI,
    )
    .unwrap();
    controle.regler_battement(Duration::from_millis(200), Duration::from_secs(2));
    assert!(matches!(
        controle.attendre(Duration::from_millis(500)),
        Ok(None)
    ));

    // Le lendemain de l'échéance, sans couper la connexion : refus et fin.
    *date.lock().unwrap() = jour(2);
    let fin = std::time::Instant::now() + Duration::from_secs(5);
    let refus = loop {
        match controle.attendre(Duration::from_millis(200)) {
            Ok(_) => assert!(std::time::Instant::now() < fin, "aucun refus"),
            Err(e) => break e,
        }
    };
    assert_eq!(
        refus,
        ErreurRelais::Refus {
            motif: motif::LICENCE_ECHUE.into(),
            valeurs: vec![jour(1)],
        }
    );
    thread::sleep(Duration::from_millis(200));
    assert!(r.entreprises().is_empty());
    r.arreter();
    let _ = std::fs::remove_dir_all(d);
}

/// Lot R1 : un fil qui ne peut pas être lancé (limite de fils du conteneur)
/// refuse la connexion sans tuer l'écoute ; le relais répond de nouveau dès
/// que des fils peuvent être lancés.
#[test]
fn fil_impossible_sans_arret_de_l_ecoute() {
    let d = dossier("fils");
    let refuser = Arc::new(AtomicBool::new(false));
    let mut config = Config::nouvelle(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        d.clone(),
        vec![cle_portail().verifying_key()],
    );
    let ordinaire = kadenz_relais::relais::lanceur_ordinaire();
    let drapeau = refuser.clone();
    config.lanceur = Arc::new(move |travail: Box<dyn FnOnce() + Send>| {
        if drapeau.load(Ordering::SeqCst) {
            Err(std::io::Error::other("limite de fils simulée"))
        } else {
            ordinaire(travail)
        }
    });
    let r = Relais::demarrer(config).unwrap();
    let adresse = r.adresse().to_string();
    let empreinte = r.empreinte().to_string();
    kadenz_relais::controle::verifier_sante(&adresse, &empreinte, DELAI).unwrap();

    refuser.store(true, Ordering::SeqCst);
    for _ in 0..20 {
        // Connexion acceptée puis fermée aussitôt, sans réponse.
        let mut tcp = TcpStream::connect(r.adresse()).unwrap();
        tcp.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut tampon = [0u8; 1];
        assert!(
            !matches!(tcp.read(&mut tampon), Ok(n) if n > 0),
            "le relais a répondu sans fil"
        );
    }
    assert!(kadenz_relais::controle::verifier_sante(&adresse, &empreinte, DELAI).is_err());

    refuser.store(false, Ordering::SeqCst);
    kadenz_relais::controle::verifier_sante(&adresse, &empreinte, DELAI)
        .expect("l'écoute doit avoir survécu aux fils refusés");
    r.arreter();
    let _ = std::fs::remove_dir_all(d);
}

/// Lot R1 : une restauration partielle (certificat ou clé seul) ne crée pas
/// d'autre identité ; l'identité complète est relue telle quelle.
#[test]
fn identite_partielle_refusee() {
    let d = dossier("identite");
    let premiere = kadenz_relais::identite_relais(&d).unwrap();
    assert_eq!(
        kadenz_relais::identite_relais(&d).unwrap().empreinte(),
        premiere.empreinte()
    );
    assert_eq!(
        kadenz_relais::empreinte_enregistree(&d).unwrap(),
        premiere.empreinte()
    );
    let cle = std::fs::read(d.join("relais.cle")).unwrap();

    // Certificat absent, clé présente : refus, clé intacte.
    let certificat = std::fs::read(d.join("relais.crt")).unwrap();
    std::fs::remove_file(d.join("relais.crt")).unwrap();
    let erreur = kadenz_relais::identite_relais(&d).unwrap_err();
    assert!(erreur.contains("certificat absent"), "{erreur}");
    assert!(!d.join("relais.crt").exists());
    assert_eq!(std::fs::read(d.join("relais.cle")).unwrap(), cle);
    let mut config = Config::nouvelle(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        d.clone(),
        vec![cle_portail().verifying_key()],
    );
    config.traces = None;
    assert!(Relais::demarrer(config).is_err());

    // Clé absente, certificat présent : refus aussi.
    std::fs::write(d.join("relais.crt"), &certificat).unwrap();
    std::fs::remove_file(d.join("relais.cle")).unwrap();
    let erreur = kadenz_relais::identite_relais(&d).unwrap_err();
    assert!(erreur.contains("clé absente"), "{erreur}");

    // Les deux restaurés : même empreinte qu'avant.
    std::fs::write(d.join("relais.cle"), &cle).unwrap();
    assert_eq!(
        kadenz_relais::identite_relais(&d).unwrap().empreinte(),
        premiere.empreinte()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&d).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700);
    }
    let _ = std::fs::remove_dir_all(d);
}

/// Lot R1 : la sonde de santé refuse un relais qui ne présente pas le
/// certificat attendu.
#[test]
fn sante_avec_autre_empreinte_refusee() {
    let (r, d) = relais("sante");
    let autre = identite("autre").empreinte();
    assert_eq!(
        kadenz_relais::controle::verifier_sante(&r.adresse().to_string(), &autre, DELAI),
        Err(ErreurRelais::CertificatDifferent)
    );
    r.arreter();
    let _ = std::fs::remove_dir_all(d);
}

/// Relais d'essai avec, en plus, une écoute WebSocket locale.
fn relais_ws(nom: &str) -> (Relais, PathBuf, String) {
    let d = dossier(nom);
    let mut config = Config::nouvelle(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        d.clone(),
        vec![cle_portail().verifying_key()],
    );
    config.battement = Duration::from_millis(200);
    config.silence = Duration::from_secs(2);
    config.ecoute_ws = Some(SocketAddr::from(([127, 0, 0, 1], 0)));
    let r = Relais::demarrer(config).unwrap();
    let ws = format!("ws://127.0.0.1:{}", r.adresse_ws().unwrap().port());
    (r, d, ws)
}

/// Passage par WebSocket (docs/spec-relais-websocket.md) : enregistrement du
/// serveur, sessions du serveur et du poste, et sonde de santé, tous par
/// `ws://`, sans rien changer au-dessus du transport.
#[test]
fn poste_relie_par_websocket() {
    let (r, d, adresse) = relais_ws("websocket");
    let serveur = identite("serveur");
    let poste_autorise = identite("poste");
    let lic = licence(ENTREPRISE, 30);

    kadenz_relais::controle::verifier_sante(&adresse, r.empreinte(), DELAI).unwrap();

    let controle = Controle::enregistrer(
        &enregistrement(
            &adresse,
            r.empreinte(),
            &lic,
            &serveur,
            vec![poste_autorise.empreinte()],
        ),
        DELAI,
    )
    .unwrap();
    assert_eq!(r.entreprises(), vec![ENTREPRISE.to_string()]);
    let echo = ServeurEcho::lancer(controle, adresse.clone());

    let tcp = poste::ouvrir(
        &adresse,
        ENTREPRISE,
        &poste_autorise.certificat,
        &poste_autorise.cle,
        DELAI,
    )
    .unwrap();
    // 300 Kio dans un sens pendant qu'on les relit dans l'autre : plusieurs
    // messages de 16 Kio, à travers deux ponts et le relais.
    let envoi: Vec<u8> = (0..300 * 1024).map(|i| (i * 7 % 251) as u8).collect();
    let attendu = envoi.clone();
    let mut ecriture = tcp.try_clone().unwrap();
    let fil = thread::spawn(move || ecriture.write_all(&envoi).unwrap());
    let mut lu = vec![0u8; attendu.len()];
    let mut lecture = tcp;
    lecture.read_exact(&mut lu).unwrap();
    fil.join().unwrap();
    assert!(lu == attendu);

    // Un poste non autorisé est refusé de la même façon.
    let intrus = identite("intrus");
    assert_eq!(
        refus(poste::ouvrir(
            &adresse,
            ENTREPRISE,
            &intrus.certificat,
            &intrus.cle,
            DELAI
        )),
        motif::POSTE_NON_AUTORISE
    );
    drop(lecture);
    echo.arreter();
    r.arreter();
    let _ = std::fs::remove_dir_all(d);
}

/// Adresses WebSocket mal formées : refusées sans rien ouvrir ; `ws://` vers
/// une autre machine que la machine locale n'est pas admis.
#[test]
fn adresse_websocket_invalide() {
    let poste_autorise = identite("poste");
    for adresse in ["ws://relais.exemple.fr", "wss://relais.exemple.fr/chemin"] {
        match poste::ouvrir(
            adresse,
            ENTREPRISE,
            &poste_autorise.certificat,
            &poste_autorise.cle,
            DELAI,
        ) {
            Err(ErreurRelais::Injoignable(e)) => assert_eq!(e, "adresse WebSocket invalide"),
            autre => panic!("{adresse} : {:?}", autre.map(|_| ())),
        }
    }
}

/// La passerelle demande un relais qui écoute en 0.0.0.0 ou 127.0.0.1.
#[test]
fn ecoute_websocket_sans_boucle_locale_refusee() {
    let d = dossier("ws-refus");
    let mut config = Config::nouvelle(
        SocketAddr::from(([127, 0, 0, 2], 0)),
        d.clone(),
        vec![cle_portail().verifying_key()],
    );
    config.ecoute_ws = Some(SocketAddr::from(([127, 0, 0, 1], 0)));
    // 127.0.0.2 n'est pas joignable partout (macOS) : seul compte le refus.
    if let Err(e) = Relais::demarrer(config) {
        assert!(
            e.contains("0.0.0.0 ou 127.0.0.1") || e.contains("impossible"),
            "{e}"
        );
    } else {
        panic!("relais démarré avec une écoute WebSocket sur 127.0.0.2");
    }
    let _ = std::fs::remove_dir_all(d);
}

/// Frein derrière la passerelle (docs/spec-relais-websocket.md, section 5) :
/// deux postes derrière le même mandataire sont comptés chacun sur son
/// `CF-Connecting-IP`. Le premier dépasse la rafale permise et voit des
/// connexions refusées ; le second, ouvert juste après, passe encore. Comptés
/// sur l'adresse de cloudflared, il aurait été refusé avec le premier.
#[test]
fn frein_par_poste_derriere_le_mandataire() {
    use futures_util::StreamExt;
    use tokio_tungstenite::WebSocketStream;
    use tokio_tungstenite::tungstenite::Message;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;

    let d = dossier("ws-frein");
    let mut config = Config::nouvelle(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        d.clone(),
        vec![cle_portail().verifying_key()],
    );
    config.traces = None;
    config.ecoute_ws = Some(SocketAddr::from(([127, 0, 0, 1], 0)));
    config.mandataire = Some(std::net::IpAddr::from([127, 0, 0, 1]));
    let r = Relais::demarrer(config).unwrap();
    let ws = r.adresse_ws().unwrap();

    async fn ouvrir(
        ws: SocketAddr,
        client: &'static str,
    ) -> WebSocketStream<tokio::net::TcpStream> {
        let mut requete = format!("ws://{ws}/").into_client_request().unwrap();
        requete
            .headers_mut()
            .insert("cf-connecting-ip", client.parse().unwrap());
        let tcp = tokio::net::TcpStream::connect(ws).await.unwrap();
        tokio_tungstenite::client_async(requete, tcp)
            .await
            .unwrap()
            .0
    }
    /// Fermée par le relais : la passerelle a rendu la fermeture. Une
    /// connexion admise attend ses trames d'ouverture et ne dit rien.
    async fn fermee(ws: &mut WebSocketStream<tokio::net::TcpStream>) -> bool {
        matches!(
            tokio::time::timeout(Duration::from_millis(5), ws.next()).await,
            Ok(None | Some(Ok(Message::Close(_))) | Some(Err(_)))
        )
    }

    let execution = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    execution.block_on(async move {
        // 220 connexions en une seconde environ : la rafale (160) et le rythme
        // (16 par seconde) en admettent moins. Le second poste s'ouvre aussitôt
        // après, seau du premier vide : compté avec lui, il serait refusé.
        let mut premier = Vec::new();
        for _ in 0..220 {
            premier.push(ouvrir(ws, "203.0.113.1").await);
        }
        let mut second = ouvrir(ws, "203.0.113.2").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert!(
            !fermee(&mut second).await,
            "second poste compté avec le premier"
        );
        let mut refusees = 0;
        for w in premier.iter_mut() {
            if fermee(w).await {
                refusees += 1;
            }
        }
        assert!(refusees > 0, "aucune connexion du premier poste freinée");
    });
    r.arreter();
    let _ = std::fs::remove_dir_all(d);
}

/// Essai à la main, par le réseau, contre le relais en service derrière
/// Cloudflare (docs/spec-relais-websocket.md) :
/// `KADENZ_EMPREINTE_RELAIS=<empreinte> cargo test -p kadenz-relais --test relais
/// relais_en_service_par_cloudflare -- --ignored --nocapture`.
///
/// Rien n'est enregistré : la sonde de santé ouvre la WebSocket, fait la
/// poignée de main TLS épinglée avec le relais à travers elle, reçoit le défi
/// et ferme. Puis la même ouverture avec une autre empreinte doit être
/// refusée : l'épinglage tient de bout en bout à travers Cloudflare.
#[test]
#[ignore = "réseau : relais en service derrière Cloudflare"]
fn relais_en_service_par_cloudflare() {
    let adresse =
        std::env::var("KADENZ_ADRESSE_RELAIS").unwrap_or_else(|_| "wss://relais.mcarre.dev".into());
    let empreinte = std::env::var("KADENZ_EMPREINTE_RELAIS")
        .expect("KADENZ_EMPREINTE_RELAIS : empreinte du relais en service");
    let delai = Duration::from_secs(15);
    let debut = std::time::Instant::now();
    kadenz_relais::controle::verifier_sante(&adresse, &empreinte, delai).unwrap();
    eprintln!("santé par {adresse} : défi reçu en {:?}", debut.elapsed());
    assert_eq!(
        kadenz_relais::controle::verifier_sante(&adresse, &"00".repeat(32), delai),
        Err(ErreurRelais::CertificatDifferent)
    );
    eprintln!("autre empreinte : refusée (certificat différent)");
}
