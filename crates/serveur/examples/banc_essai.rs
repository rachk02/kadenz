//! Banc d'essai à l'écran (jalons F5 et F6d) : serveur Kadenz au premier plan
//! sur un dossier d'essai, installé avec une licence d'abonnement signée par
//! une clé d'essai que seul ce banc accepte. Exemple de développement : il
//! n'est jamais livré, et ne sert jamais sur les données réelles.
//!
//! Usage :
//! - `banc_essai <dossier> <port> [--codes N]` : installe le serveur si le
//!   dossier est neuf (entreprise, comptes admin, compta et vente, licence de
//!   5 postes avec tous les modules), le lance et affiche N codes complets
//!   d'inscription (« Poste 1 » à « Poste N »). Arrêt propre : touche Entrée
//!   (sauvegarde d'arrêt si un mot de passe est réglé) ; Ctrl+C ou la croix
//!   arrêtent sans elle.
//! - `banc_essai <dossier> relais <adresse:port> <empreinte>` : règle le
//!   relais de l'accès à distance (F8), comme la fenêtre du serveur ; le
//!   service relit ce réglage de lui-même. L'accès se règle ensuite depuis un
//!   poste Admin (rubrique « Accès à distance »).
//! - `banc_essai <dossier> desactiver <identifiant>` : désactive un compte
//!   directement dans la base, serveur arrêté (coupure du F6d).
//! - `banc_essai <dossier> etat` : opérations reçues, suspendues, changements
//!   et tiers, en lecture seule.
//! - `banc_essai <dossier> operation <début d'identifiant>` : contenu complet
//!   des opérations dont l'identifiant commence ainsi, en lecture seule.
//! - `banc_essai <fichier.sqlite> base_v1` : crée une base V1 de démonstration
//!   (chiffrée par une clé de ce compte Windows, à côté d'elle) pour l'essai
//!   de reprise du F7d : comptes admin (écarté à la reprise, décision 53) et
//!   caisse, deux clients, un fournisseur, deux produits, un devis converti en
//!   facture validée et réglée en partie, un brouillon. Refusé si le fichier
//!   existe.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use kadenz_domaine::calcul::Taux;
use kadenz_domaine::document::TypeDocument;
use kadenz_domaine::module::Module;
use kadenz_domaine::referentiel::{FicheProduit, FicheTiers, Sens, TypeTiers};
use kadenz_domaine::role::Role;
use kadenz_licence::SigningKey;
use kadenz_licence::entreprise::{self as lic, LicenceEntreprise};
use kadenz_protocole::Entite;
use kadenz_serveur::{Config, Serveur, admin};
use kadenz_stockage::comptes::{self, NouvelUtilisateur, Utilisateur};
use kadenz_stockage::documents::{self, ChampsLivraison, LigneSaisie, Saisie};
use kadenz_stockage::entreprise::Entreprise;
use kadenz_stockage::installation::InstallationInitiale;
use kadenz_stockage::parametres::Parametres;
use kadenz_stockage::reglements::{self, SaisieReglement};
use kadenz_stockage::{abonnement, inscription, installation, produits, synchro, tiers};
use rusqlite::Connection;

const MOT_DE_PASSE: &str = "motdepasse1";

/// Clé d'essai du portail, la même que celle des essais automatiques.
fn cle_essai() -> SigningKey {
    SigningKey::from_bytes(&[5u8; 32])
}

fn compte(conn: &Connection, identifiant: &str) -> Result<Utilisateur, String> {
    let uid: String = conn
        .query_row(
            "SELECT uid FROM utilisateur WHERE identifiant = ?1",
            [identifiant],
            |r| r.get(0),
        )
        .map_err(|e| format!("compte {identifiant} : {e}"))?;
    let id = synchro::id_de(conn, Entite::Utilisateur, &uid)
        .map_err(|e| e.to_string())?
        .ok_or("compte introuvable")?;
    comptes::lire(conn, id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "compte introuvable".to_string())
}

fn installer(conn: &mut Connection) -> Result<(), String> {
    let deja: i64 = conn
        .query_row("SELECT count(*) FROM utilisateur", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if deja > 0 {
        return Ok(());
    }
    let entreprise = synchro::identifiant_entreprise(conn).map_err(|e| e.to_string())?;
    let jour = |n: i64| {
        (abonnement::horloge() + chrono::Duration::days(n))
            .format("%Y-%m-%d")
            .to_string()
    };
    let licence = lic::emettre(
        &LicenceEntreprise {
            entreprise,
            client: "Entreprise d'essai".into(),
            formule: lic::Formule::Annuelle,
            postes: 5,
            modules: Module::TOUS.to_vec(),
            debut: jour(0),
            echeance: jour(365),
            emetteur: lic::Emetteur::Editeur,
            emise_le: jour(0),
        },
        &cle_essai(),
    )
    .map_err(|e| e.to_string())?;
    let admin = admin::installer_avec(
        conn,
        &InstallationInitiale {
            entreprise: Entreprise {
                nom: "Entreprise d'essai".into(),
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
        Some(&licence),
        &[cle_essai().verifying_key()],
    )
    .map_err(|e| e.to_string())?;
    for (nom, identifiant, role) in [
        ("Comptable d'essai", "compta", Role::Comptable),
        ("Commercial d'essai", "vente", Role::Commercial),
    ] {
        comptes::creer(
            conn,
            &admin,
            &NouvelUtilisateur {
                nom_complet: nom.into(),
                identifiant: identifiant.into(),
                mot_de_passe: MOT_DE_PASSE.into(),
                role,
            },
        )
        .map_err(|e| e.to_string())?;
    }
    synchro::completer_uids(conn).map_err(|e| e.to_string())?;
    println!("Serveur d'essai installé : entreprise « Entreprise d'essai », licence de 5 postes.");
    Ok(())
}

fn regler_relais(dossier: &Path, adresse: &str, empreinte: &str) -> Result<(), String> {
    let conn = admin::ouvrir_base(dossier)?;
    admin::regler_relais(&conn, Some((adresse, empreinte)))?;
    println!("Relais réglé : {adresse}.");
    Ok(())
}

fn desactiver(dossier: &Path, identifiant: &str) -> Result<(), String> {
    let conn = admin::ouvrir_base(dossier)?;
    let admin = compte(&conn, "admin")?;
    let cible = compte(&conn, identifiant)?;
    comptes::definir_actif(&conn, &admin, cible.id, false).map_err(|e| e.to_string())?;
    println!("Compte « {identifiant} » désactivé.");
    Ok(())
}

/// Lecture seule de la base : opérations reçues, suspendues, changements et
/// tiers, pour suivre un essai.
fn etat(dossier: &Path) -> Result<(), String> {
    let conn = admin::ouvrir_base(dossier)?;
    let afficher = |titre: &str, sql: &str| -> Result<(), String> {
        println!("== {titre}");
        let mut req = conn.prepare(sql).map_err(|e| e.to_string())?;
        let n = req.column_count();
        let lignes = req
            .query_map([], |r| {
                (0..n)
                    .map(|i| {
                        r.get_ref(i).map(|v| match v {
                            rusqlite::types::ValueRef::Null => "NULL".to_string(),
                            rusqlite::types::ValueRef::Integer(x) => x.to_string(),
                            rusqlite::types::ValueRef::Real(x) => x.to_string(),
                            rusqlite::types::ValueRef::Text(t) => {
                                String::from_utf8_lossy(t).into_owned()
                            }
                            rusqlite::types::ValueRef::Blob(_) => "<blob>".to_string(),
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(|e| e.to_string())?;
        for l in lignes {
            println!("{}", l.map_err(|e| e.to_string())?.join(" | "));
        }
        Ok(())
    };
    afficher(
        "operation (recue, poste, issue, base, hors_connexion, action)",
        "SELECT recue, substr(id, 1, 13), substr(json_extract(contenu, '$.compte'), 1, 13), json_extract(contenu, '$.heure_poste'), substr(poste, 1, 12), issue, json_extract(contenu, '$.base'),
                json_extract(contenu, '$.hors_connexion'), substr(json_extract(contenu, '$.action'), 1, 160)
         FROM operation ORDER BY recue",
    )?;
    afficher(
        "operation_suspendue",
        "SELECT operation_id, motif, cible, ouverte FROM operation_suspendue",
    )?;
    afficher(
        "changement (seq, entite, entite_id, poste)",
        "SELECT seq, entite, entite_id, substr(poste, 1, 12) FROM changement ORDER BY seq",
    )?;
    afficher(
        "tiers (id, uid, nom, telephone)",
        "SELECT id, uid, nom, telephone FROM tiers",
    )
}

/// Contenu complet (JSON) des opérations dont l'identifiant commence par
/// `debut`, en lecture seule.
fn operation(dossier: &Path, debut: &str) -> Result<(), String> {
    let conn = admin::ouvrir_base(dossier)?;
    let mut req = conn
        .prepare("SELECT id, issue, contenu FROM operation WHERE id LIKE ?1 || '%' ORDER BY recue, rowid")
        .map_err(|e| e.to_string())?;
    let lignes = req
        .query_map([debut], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    for l in lignes {
        let (id, issue, contenu) = l.map_err(|e| e.to_string())?;
        println!(
            "== {id}
issue : {issue}
{contenu}
"
        );
    }
    Ok(())
}

fn lancer(dossier: PathBuf, port: u16, codes: u32) -> Result<(), String> {
    std::fs::create_dir_all(&dossier).map_err(|e| e.to_string())?;
    {
        let mut conn = admin::ouvrir_base(&dossier)?;
        installer(&mut conn)?;
    }
    let mut config = Config::nouvelle(dossier.clone());
    config.adresse = SocketAddr::from(([0, 0, 0, 0], port));
    config.controle_droits = false;
    config.cles_portail = vec![cle_essai().verifying_key()];
    let serveur = Serveur::demarrer(config)?;
    if let Some(avertissement) = serveur.avertissement() {
        eprintln!("{avertissement}");
    }
    println!(
        "Serveur d'essai à l'écoute sur {}\nEmpreinte : {}",
        serveur.adresse(),
        serveur.empreinte()
    );
    println!("Comptes : admin, compta, vente ; mot de passe : {MOT_DE_PASSE}");
    if codes > 0 {
        let conn = admin::ouvrir_base(&dossier)?;
        for i in 1..=codes {
            let code = inscription::creer_code(&conn, &format!("Poste {i}"), chrono::Utc::now())
                .map_err(|e| e.to_string())?;
            println!(
                "Code complet du Poste {i} (24 h) : {}",
                admin::code_complet(&conn, &dossier, &code)?
            );
        }
    }
    println!(
        "Arrêt propre, avec la sauvegarde d'arrêt si un mot de passe est réglé : touche Entrée.\n\
         Ctrl+C ou la croix de la fenêtre arrêtent sans cette sauvegarde."
    );
    let mut ligne = String::new();
    match std::io::stdin().read_line(&mut ligne) {
        // Entrée standard fermée (lancement sans console) : pas d'arrêt par
        // Entrée, le serveur tourne jusqu'à la fin du processus.
        Ok(0) | Err(_) => loop {
            std::thread::park();
        },
        Ok(_) => {
            println!("Arrêt en cours...");
            serveur.arreter();
            println!("Serveur arrêté.");
            Ok(())
        }
    }
}

fn base_v1(fichier: &Path) -> Result<(), String> {
    if fichier.exists() {
        return Err(format!("{} existe déjà", fichier.display()));
    }
    let e = |e: kadenz_stockage::Erreur| e.to_string();
    let mut conn = kadenz_stockage::ouvrir(fichier).map_err(e)?;
    // Modules de la licence V1 : table temporaire de cette seule connexion,
    // rien n'est écrit dans la base.
    for m in Module::TOUS {
        conn.execute(
            "INSERT OR IGNORE INTO temp.module_actif (code) VALUES (?1)",
            [m.code()],
        )
        .map_err(|e| e.to_string())?;
    }
    let admin = installation::installer(
        &mut conn,
        &InstallationInitiale {
            entreprise: Entreprise {
                nom: "Quincaillerie Démo V1".into(),
                adresse: "Secteur 15, Ouagadougou".into(),
                ..Default::default()
            },
            admin: NouvelUtilisateur {
                nom_complet: "Patron V1".into(),
                identifiant: "admin".into(),
                mot_de_passe: "motdepasse-v1".into(),
                role: Role::Admin,
            },
            parametres: Parametres::default(),
        },
    )
    .map_err(e)?;
    comptes::creer(
        &conn,
        &admin,
        &NouvelUtilisateur {
            nom_complet: "Caisse V1".into(),
            identifiant: "caisse".into(),
            mot_de_passe: "motdepasse-caisse".into(),
            role: Role::Commercial,
        },
    )
    .map_err(e)?;
    let fiche = |nom: &str, type_tiers| FicheTiers {
        nom: nom.into(),
        ..FicheTiers::vide(type_tiers)
    };
    let client_a = tiers::creer(&conn, &admin, &fiche("Client Démo A", TypeTiers::Client))
        .map_err(e)?
        .id;
    let client_b = tiers::creer(&conn, &admin, &fiche("Client Démo B", TypeTiers::Client))
        .map_err(e)?
        .id;
    tiers::creer(
        &conn,
        &admin,
        &fiche("Fournisseur Démo", TypeTiers::Fournisseur),
    )
    .map_err(e)?;
    let produit = |reference: &str, designation: &str, prix| FicheProduit {
        designation: designation.into(),
        reference: reference.into(),
        prix,
        sens: Sens::Vente,
        suivi_stock: false,
        nature_comptable: Default::default(),
    };
    let ciment = produits::creer(&conn, &admin, &produit("CIM50", "Ciment 50 kg", 6_500))
        .map_err(e)?
        .id;
    let fer = produits::creer(&conn, &admin, &produit("FER10", "Fer de 10", 4_200))
        .map_err(e)?
        .id;
    let saisie = |client, produit_id, reference: &str, designation: &str, prix, quantite| Saisie {
        date: chrono::Local::now().date_naive(),
        tiers_id: Some(client),
        libelle: "Démonstration de reprise".into(),
        taux: Taux::default(),
        mode_paiement: String::new(),
        observations: String::new(),
        date_echeance: None,
        lignes: vec![LigneSaisie {
            produit_id: Some(produit_id),
            reference: reference.into(),
            designation: designation.into(),
            prix,
            quantite,
            ..LigneSaisie::default()
        }],
        livraison: ChampsLivraison::default(),
    };
    let devis = documents::creer_brouillon(
        &mut conn,
        &admin,
        TypeDocument::Devis,
        &saisie(client_a, ciment, "CIM50", "Ciment 50 kg", 6_500, 10_000),
    )
    .map_err(e)?;
    documents::valider(&mut conn, &admin, devis).map_err(e)?;
    let facture = documents::convertir_en_facture(&mut conn, &admin, devis).map_err(e)?;
    documents::valider(&mut conn, &admin, facture).map_err(e)?;
    reglements::ajouter(
        &mut conn,
        &admin,
        facture,
        &SaisieReglement {
            date: chrono::Local::now().date_naive(),
            montant: 30_000,
            mode_paiement: "Espèces".into(),
            reference: String::new(),
        },
    )
    .map_err(e)?;
    documents::creer_brouillon(
        &mut conn,
        &admin,
        TypeDocument::Facture,
        &saisie(client_b, fer, "FER10", "Fer de 10", 4_200, 5_000),
    )
    .map_err(e)?;
    println!(
        "Base V1 de démonstration créée : {}\nComptes : admin / motdepasse-v1 (écarté à la reprise), \
         caisse / motdepasse-caisse",
        fichier.display()
    );
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let resultat = match args.as_slice() {
        [dossier, commande, identifiant] if commande == "desactiver" => {
            desactiver(&PathBuf::from(dossier), identifiant)
        }
        [dossier, commande, adresse, empreinte] if commande == "relais" => {
            regler_relais(&PathBuf::from(dossier), adresse, empreinte)
        }
        [dossier, commande] if commande == "etat" => etat(&PathBuf::from(dossier)),
        [fichier, commande] if commande == "base_v1" => base_v1(&PathBuf::from(fichier)),
        [dossier, commande, debut] if commande == "operation" => {
            operation(&PathBuf::from(dossier), debut)
        }
        [dossier, port] => port
            .parse()
            .map_err(|_| "port invalide".to_string())
            .and_then(|p| lancer(PathBuf::from(dossier), p, 0)),
        [dossier, port, option, n] if option == "--codes" => match (port.parse(), n.parse()) {
            (Ok(p), Ok(n)) => lancer(PathBuf::from(dossier), p, n),
            _ => Err("port ou nombre de codes invalide".into()),
        },
        _ => Err("usage : banc_essai <dossier> <port> [--codes N] | \
                  banc_essai <dossier> relais <adresse:port> <empreinte> | \
                  banc_essai <dossier> desactiver <identifiant> | \
                  banc_essai <dossier> etat | \
                  banc_essai <dossier> operation <début d'identifiant> | \
                  banc_essai <fichier.sqlite> base_v1"
            .into()),
    };
    match resultat {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
