//! Serveur intégré du monoposte, lot M1 (spécification du monoposte,
//! section 2) : le multiposte ne change pas, et le serveur intégré protège
//! ses clés au niveau du compte, réserve son dossier au compte, prend un
//! verrou et reprend son port.

#![cfg(windows)]

use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use std::path::{Path, PathBuf};

use kadenz_serveur::{
    Config, ErreurDemarrage, NOM_BASE, NOM_CERTIFICAT, NOM_MARQUE_COMPTE, PORT_PAR_DEFAUT, Serveur,
    admin, droits, portee_du_dossier, sauvegardes,
};
use kadenz_stockage::chiffrement::{self, Portee};

/// Clé TLS du serveur dans son dossier.
const NOM_CLE_TLS: &str = "serveur.cle-tls";

fn dossier(nom: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "kadenz-serveur-monoposte-{nom}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    d
}

/// Drapeaux d'une donnée protégée par Windows (DPAPI), lus dans l'en-tête :
/// version, fournisseur, version et identifiant de la clé maîtresse, puis
/// drapeaux, à l'octet 40.
fn drapeaux(protegee: &[u8]) -> u32 {
    u32::from_le_bytes(protegee[40..44].try_into().unwrap())
}

/// Portée de protection d'un fichier de clé, par comparaison avec deux
/// données protégées à l'instant, une par portée.
fn portee_de(fichier: &Path) -> Portee {
    let compte = drapeaux(&chiffrement::proteger(b"essai", Portee::Compte).unwrap());
    let machine = drapeaux(&chiffrement::proteger(b"essai", Portee::Machine).unwrap());
    assert_ne!(compte, machine, "les deux portées doivent se distinguer");
    let lu = drapeaux(&std::fs::read(fichier).unwrap());
    if lu == compte {
        Portee::Compte
    } else if lu == machine {
        Portee::Machine
    } else {
        panic!("drapeaux inattendus {lu:#x} dans {}", fichier.display())
    }
}

fn cles(d: &Path) -> [PathBuf; 2] {
    [
        chiffrement::chemin_cle(&d.join(NOM_BASE)),
        d.join(NOM_CLE_TLS),
    ]
}

/// Le multiposte ne change pas : configuration par défaut à la portée
/// machine, sans marque, clés protégées au niveau de la machine.
#[test]
fn multiposte_inchange() {
    let d = dossier("service");
    let mut config = Config::nouvelle(d.clone());
    assert_eq!(config.portee, Portee::Machine);
    assert!(!config.reprendre_port);
    assert!(config.decouverte);
    assert!(config.controle_droits);
    assert_eq!(
        config.adresse,
        SocketAddr::from(([0, 0, 0, 0], PORT_PAR_DEFAUT))
    );
    // Essai sous le compte courant, sans droits d'administrateur (les droits
    // du service sont essayés dans `droits`).
    config.adresse = SocketAddr::from(([127, 0, 0, 1], 0));
    config.controle_droits = false;
    config.decouverte = false;
    let serveur = Serveur::demarrer(config).unwrap();
    assert!(d.join(NOM_CERTIFICAT).is_file());
    assert!(!d.join(NOM_MARQUE_COMPTE).exists());
    assert_eq!(portee_du_dossier(&d), Portee::Machine);
    for cle in cles(&d) {
        assert_eq!(portee_de(&cle), Portee::Machine, "{}", cle.display());
    }
    serveur.arreter();

    // Un dossier de service n'est pas pris par le serveur intégré.
    assert_eq!(
        sauvegardes::dossier_par_defaut_de(&d),
        sauvegardes::dossier_par_defaut()
    );
    let mut integre = Config::monoposte(d.clone());
    integre.controle_droits = false;
    let refus = Serveur::demarrer(integre).err().unwrap();
    assert!(matches!(refus, ErreurDemarrage::AutreEdition), "{refus}");
    let _ = std::fs::remove_dir_all(&d);
}

/// Serveur intégré : 127.0.0.1 sans découverte, clés au niveau du compte,
/// dossier réservé au compte (sans droits d'administrateur), verrou,
/// port repris puis remplacé s'il est pris.
#[test]
fn serveur_integre_du_monoposte() {
    let d = dossier("integre");
    let config = Config::monoposte(d.clone());
    assert_eq!(config.portee, Portee::Compte);
    assert!(config.reprendre_port);
    assert!(!config.decouverte);
    assert!(config.controle_droits);
    assert_eq!(config.adresse.ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));

    let serveur = Serveur::demarrer(config).unwrap();
    assert_eq!(serveur.adresse().ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
    assert_ne!(serveur.adresse().port(), 0);
    assert!(d.join(NOM_MARQUE_COMPTE).is_file());
    assert_eq!(portee_du_dossier(&d), Portee::Compte);
    assert!(droits::verifier(&d, Portee::Compte).unwrap().is_empty());
    for cle in cles(&d) {
        assert_eq!(portee_de(&cle), Portee::Compte, "{}", cle.display());
    }

    // Un second serveur sur le même dossier est refusé.
    let refus = Serveur::demarrer(Config::monoposte(d.clone()))
        .err()
        .unwrap();
    assert!(matches!(refus, ErreurDemarrage::Verrouille), "{refus}");
    // Sauvegardes à part de celles de la V1, sous son dossier.
    let sauvegardes_v1 = kadenz_stockage::sauvegarde::dossier_par_defaut();
    let par_defaut = sauvegardes::dossier_par_defaut_de(&d);
    assert_eq!(par_defaut.parent(), Some(sauvegardes_v1.as_path()));
    assert_eq!(sauvegardes::lire_reglage(&d).dossier, par_defaut);

    // Relancé : clés relues, même port.
    let port = serveur.adresse().port();
    serveur.arreter();
    let serveur = Serveur::demarrer(Config::monoposte(d.clone())).unwrap();
    assert_eq!(serveur.adresse().port(), port);
    serveur.arreter();
    // Base fermée au retour d'`arreter` (restauration du lot M4, remarque de
    // kadenz-74) : sous Windows, un fichier ouvert par SQLite ne se renomme
    // pas.
    let base = d.join(NOM_BASE);
    let deplacee = d.join("base-deplacee.sqlite");
    std::fs::rename(&base, &deplacee).unwrap();
    std::fs::rename(&deplacee, &base).unwrap();

    // Port pris par un autre programme : un port libre, noté dans la base.
    let occupant = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port))).unwrap();
    let serveur = Serveur::demarrer(Config::monoposte(d.clone())).unwrap();
    let nouveau = serveur.adresse().port();
    assert_ne!(nouveau, port);
    assert_eq!(
        serveur.base(|c| admin::port_note(c)).unwrap(),
        Some(nouveau)
    );
    serveur.arreter();
    drop(occupant);

    // Un dossier du serveur intégré n'est pas pris par le service.
    let mut service = Config::nouvelle(d.clone());
    service.adresse = SocketAddr::from(([127, 0, 0, 1], 0));
    service.controle_droits = false;
    service.decouverte = false;
    let refus = Serveur::demarrer(service).err().unwrap();
    assert!(matches!(refus, ErreurDemarrage::AutreEdition), "{refus}");
    std::fs::remove_dir_all(&d).unwrap();
}
