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
