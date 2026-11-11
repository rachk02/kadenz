//! Outil de l'éditeur de Kadenz (cahier des charges, section 15), jamais
//! distribué aux clients.
//!
//! Commandes :
//! - `generer <dossier>` : crée la paire de clés ; la clé privée est écrite dans
//!   `<dossier>` (hors du dépôt), la clé publique est affichée pour être copiée
//!   dans `crates/licence/cle-publique.hex`.
//! - `poste` : affiche l'identifiant de ce poste.
//! - `verifier <clé> <poste>` : vérifie une clé avec la clé publique embarquée
//!   et affiche son contenu.
//! - `emettre <fichier de clé privée> <poste> <client> <modules>` : affiche une
//!   clé de licence ; `<modules>` est une liste de codes séparés par des
//!   virgules (devis,proforma,bons_commande,livraisons,paiements,
//!   tableau_de_bord,relances), ou `tous`, ou `aucun`.

use std::path::Path;
use std::process::ExitCode;

use ed25519_dalek::SigningKey;
use kadenz_domaine::module::Module;
use kadenz_licence::{Licence, cle_privee_depuis_hex, emettre, en_hex, poste_local, verifier};

const FICHIER_PRIVE: &str = "kadenz-cle-privee.hex";
const FICHIER_PUBLIC: &str = "kadenz-cle-publique.hex";

fn generer(dossier: &str) -> Result<(), String> {
    let dossier = Path::new(dossier);
    std::fs::create_dir_all(dossier).map_err(|e| format!("dossier {} : {e}", dossier.display()))?;
    let prive = dossier.join(FICHIER_PRIVE);
    if prive.exists() {
        return Err(format!(
            "{} existe déjà : une nouvelle paire rendrait invalides toutes les clés émises",
            prive.display()
        ));
    }
    let mut graine = [0u8; 32];
    getrandom::fill(&mut graine).map_err(|e| format!("aléa du système indisponible : {e}"))?;
    let cle = SigningKey::from_bytes(&graine);
    let publique = en_hex(cle.verifying_key().as_bytes());
    std::fs::write(&prive, en_hex(&graine)).map_err(|e| format!("{} : {e}", prive.display()))?;
    std::fs::write(dossier.join(FICHIER_PUBLIC), &publique)
        .map_err(|e| format!("{} : {e}", dossier.display()))?;
    println!(
        "Clé privée écrite dans {} : à garder secrète, jamais dans le dépôt.",
        prive.display()
    );
    println!("Clé publique (à copier dans crates/licence/cle-publique.hex) :");
    println!("{publique}");
    Ok(())
}

fn modules(liste: &str) -> Result<Vec<Module>, String> {
    match liste.trim() {
        "tous" => Ok(Module::TOUS.to_vec()),
        "aucun" | "" => Ok(Vec::new()),
        l => l
            .split(',')
            .map(|c| Module::depuis_code(c.trim()).ok_or_else(|| format!("module inconnu : {c}")))
            .collect(),
    }
}

fn emettre_cle(fichier: &str, poste: &str, client: &str, liste: &str) -> Result<(), String> {
    let texte = std::fs::read_to_string(fichier).map_err(|e| format!("{fichier} : {e}"))?;
    let cle = cle_privee_depuis_hex(&texte).ok_or("clé privée illisible")?;
    let licence = Licence {
        client: client.trim().to_owned(),
        poste: poste.trim().to_uppercase(),
        modules: modules(liste)?,
        emise_le: chrono::Local::now()
            .date_naive()
            .format("%Y-%m-%d")
            .to_string(),
    };
    if licence.client.is_empty() {
        return Err("nom du client vide".into());
    }
    let cle = emettre(&licence, &cle).map_err(|e| e.to_string())?;
    println!("{cle}");
    Ok(())
}

fn verifier_cle(cle: &str, poste: &str) -> Result<(), String> {
    let l = verifier(cle, poste).map_err(|e| e.to_string())?;
    let modules: Vec<&str> = l.modules.iter().map(|m| m.code()).collect();
    println!(
        "Clé valide : client « {} », poste {}, émise le {}",
        l.client, l.poste, l.emise_le
    );
    println!("Modules : {}", modules.join(", "));
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let resultat = match args.as_slice() {
        ["generer", dossier] => generer(dossier),
        ["poste"] => poste_local().map(|p| println!("{p}")),
        ["verifier", cle, poste] => verifier_cle(cle, poste),
        ["emettre", fichier, poste, client, liste] => emettre_cle(fichier, poste, client, liste),
        _ => Err(
            "usage : kadenz-editeur generer <dossier> | poste | verifier <clé> <poste> | \
                  emettre <fichier de clé privée> <poste> <client> <modules>"
                .into(),
        ),
    };
    match resultat {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
