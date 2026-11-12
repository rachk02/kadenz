//! Clé de licence de Kadenz (cahier des charges, section 15).
//!
//! Une clé contient le nom du client, l'identifiant du poste, les modules
//! activés et la date d'émission ; elle est perpétuelle. Elle est signée par
//! l'éditeur (Ed25519) : l'application ne contient que la clé publique et vérifie
//! la signature hors ligne. Format : `KADENZ-<données>.<signature>`, chaque partie
//! en base64 URL sans remplissage, les données en JSON.
//!
//! La clé privée ne vit que chez l'éditeur, hors du dépôt ; seul l'outil
//! `kadenz-editeur` s'en sert, pour émettre les clés.
//!
//! Rotation (cahier de la phase 2, pont V1, règle c) : l'application accepte
//! une liste de clés publiques, l'actuelle et la suivante. Une licence porte
//! l'identifiant de la clé qui l'a signée (champ `cle`) ; une licence sans ce
//! champ, émise avant le pont, est vérifiée par la clé actuelle.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::{Signature, Signer, Verifier};
pub use ed25519_dalek::{SigningKey, VerifyingKey};
use kadenz_domaine::message::MessageCompose;
use kadenz_domaine::module::Module;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const PREFIXE: &str = "KADENZ-";
const VERSION: u64 = 1;

/// Clé publique de vérification de l'éditeur, en hexadécimal (64 caractères),
/// produite par `kadenz-editeur generer`.
const CLE_PUBLIQUE: &str = include_str!("../cle-publique.hex");

/// Clé publique suivante, générée d'avance et gardée hors ligne : le portail
/// d'abonnement pourra signer avec elle sans nouvelle mise à jour des postes.
/// Fichier vide tant qu'elle n'est pas générée (aucune release possible alors,
/// voir [`controler_cles_embarquees`]).
const CLE_PUBLIQUE_SUIVANTE: &str = include_str!("../cle-publique-suivante.hex");

/// Contenu d'une clé de licence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Licence {
    pub client: String,
    /// Identifiant du poste (« XXXX-XXXX-XXXX-XXXX »).
    pub poste: String,
    /// Modules activés, dans l'ordre de `Module::TOUS`.
    pub modules: Vec<Module>,
    /// Date d'émission, AAAA-MM-JJ.
    pub emise_le: String,
}

/// Tous les modèles d'erreur de ce crate, pour vérifier leurs traductions.
pub const MODELES_ERREUR: &[&str] = &[
    "clé de licence illisible",
    "signature de la clé de licence invalide",
    "version de clé de licence non prise en charge : {}",
    "clé de licence émise pour un autre poste ({})",
    "module inconnu dans la clé de licence : {}",
    "le module « {} » exige le module « {} »",
    "clé publique de vérification absente : générer la paire de clés avec kadenz-editeur",
    "clé de signature de la licence inconnue : {}",
];

fn erreur(modele: &'static str) -> MessageCompose {
    MessageCompose::new(modele, &[])
}

fn illisible() -> MessageCompose {
    erreur("clé de licence illisible")
}

/// Remet les modules dans l'ordre de `Module::TOUS` et vérifie leurs dépendances.
fn normaliser(modules: &[Module]) -> Result<Vec<Module>, MessageCompose> {
    let modules: Vec<Module> = Module::TOUS
        .into_iter()
        .filter(|m| modules.contains(m))
        .collect();
    for m in &modules {
        if let Some(requis) = m.requis()
            && !modules.contains(&requis)
        {
            return Err(MessageCompose::new(
                "le module « {} » exige le module « {} »",
                &[&m.libelle(), &requis.libelle()],
            ));
        }
    }
    Ok(modules)
}

/// Clé publique embarquée dans l'application.
pub fn cle_publique() -> Result<VerifyingKey, MessageCompose> {
    cle_publique_depuis_hex(CLE_PUBLIQUE).ok_or_else(|| {
        erreur(
            "clé publique de vérification absente : générer la paire de clés avec kadenz-editeur",
        )
    })
}

/// Identifiant d'une clé publique : les 16 premiers caractères hexadécimaux de
/// l'empreinte SHA-256 de ses octets.
pub fn identifiant_cle(cle: &VerifyingKey) -> String {
    en_hex(&Sha256::digest(cle.as_bytes())[..8])
}

/// Clés publiques acceptées, l'actuelle d'abord ; un fichier vide (clé suivante
/// pas encore générée) est ignoré.
pub fn cles_publiques() -> Vec<VerifyingKey> {
    [CLE_PUBLIQUE, CLE_PUBLIQUE_SUIVANTE]
        .into_iter()
        .filter_map(cle_publique_depuis_hex)
        .collect()
}

/// Contrôle avant publication : la clé actuelle et la clé suivante embarquées
/// sont valides et distinctes.
pub fn controler_cles_embarquees() -> Result<(), String> {
    let actuelle = cle_publique_depuis_hex(CLE_PUBLIQUE)
        .ok_or("crates/licence/cle-publique.hex : clé publique absente ou invalide")?;
    let suivante = cle_publique_depuis_hex(CLE_PUBLIQUE_SUIVANTE)
        .ok_or("crates/licence/cle-publique-suivante.hex : clé publique absente ou invalide")?;
    if actuelle == suivante {
        return Err("la clé de licence suivante est identique à la clé actuelle".into());
    }
    Ok(())
}

fn octets_hex<const N: usize>(texte: &str) -> Option<[u8; N]> {
    let texte = texte.trim();
    if texte.len() != 2 * N {
        return None;
    }
    let mut octets = [0u8; N];
    for (k, o) in octets.iter_mut().enumerate() {
        *o = u8::from_str_radix(texte.get(2 * k..2 * k + 2)?, 16).ok()?;
    }
    Some(octets)
}

pub fn cle_publique_depuis_hex(texte: &str) -> Option<VerifyingKey> {
    VerifyingKey::from_bytes(&octets_hex::<32>(texte)?).ok()
}

pub fn cle_privee_depuis_hex(texte: &str) -> Option<SigningKey> {
    Some(SigningKey::from_bytes(&octets_hex::<32>(texte)?))
}

pub fn en_hex(octets: &[u8]) -> String {
    octets.iter().map(|o| format!("{o:02x}")).collect()
}

/// Émet une clé de licence signée (outil éditeur seulement).
pub fn emettre(licence: &Licence, cle_privee: &SigningKey) -> Result<String, MessageCompose> {
    let donnees = json!({
        "version": VERSION,
        "client": licence.client,
        "poste": licence.poste,
        "modules": normaliser(&licence.modules)?.iter().map(|m| m.code()).collect::<Vec<_>>(),
        "emise_le": licence.emise_le,
        "cle": identifiant_cle(&cle_privee.verifying_key()),
    })
    .to_string();
    let signature = cle_privee.sign(donnees.as_bytes());
    Ok(format!(
        "{PREFIXE}{}.{}",
        URL_SAFE_NO_PAD.encode(donnees.as_bytes()),
        URL_SAFE_NO_PAD.encode(signature.to_bytes())
    ))
}

/// Vérifie une clé avec les clés publiques embarquées, pour le poste donné.
pub fn verifier(cle: &str, poste: &str) -> Result<Licence, MessageCompose> {
    let cles = cles_publiques();
    if cles.is_empty() {
        return Err(cle_publique().unwrap_err());
    }
    verifier_parmi(cle, poste, &cles)
}

/// Vérifie une clé avec une clé publique donnée, pour le poste donné.
pub fn verifier_avec(
    cle: &str,
    poste: &str,
    cle_publique: &VerifyingKey,
) -> Result<Licence, MessageCompose> {
    verifier_parmi(cle, poste, std::slice::from_ref(cle_publique))
}

/// Vérifie une clé parmi des clés publiques, la première étant l'actuelle :
/// celle que désigne le champ `cle`, ou la première si la licence n'en a pas.
pub fn verifier_parmi(
    cle: &str,
    poste: &str,
    cles_publiques: &[VerifyingKey],
) -> Result<Licence, MessageCompose> {
    let cle: String = cle.chars().filter(|c| !c.is_whitespace()).collect();
    let corps = cle.strip_prefix(PREFIXE).ok_or_else(illisible)?;
    let (donnees, signature) = corps.split_once('.').ok_or_else(illisible)?;
    let donnees = URL_SAFE_NO_PAD.decode(donnees).map_err(|_| illisible())?;
    let signature = URL_SAFE_NO_PAD.decode(signature).map_err(|_| illisible())?;
    let signature = Signature::from_slice(&signature).map_err(|_| illisible())?;
    // Le champ `cle` ne sert qu'à choisir la clé : la signature est vérifiée
    // ensuite, sur toutes les données, ce champ compris.
    let v: Value = serde_json::from_slice(&donnees).map_err(|_| illisible())?;
    let cle_publique = match v.get("cle").and_then(Value::as_str) {
        Some(id) => cles_publiques
            .iter()
            .find(|c| identifiant_cle(c) == id)
            .ok_or_else(|| {
                MessageCompose::new("clé de signature de la licence inconnue : {}", &[&id])
            })?,
        None => cles_publiques.first().ok_or_else(illisible)?,
    };
    cle_publique
        .verify(&donnees, &signature)
        .map_err(|_| erreur("signature de la clé de licence invalide"))?;

    let version = v.get("version").and_then(Value::as_u64).unwrap_or(0);
    if version != VERSION {
        return Err(MessageCompose::new(
            "version de clé de licence non prise en charge : {}",
            &[&version],
        ));
    }
    let texte = |cle: &str| {
        v.get(cle)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(illisible)
    };
    let licence_poste = texte("poste")?;
    if licence_poste != poste.trim() {
        return Err(MessageCompose::new(
            "clé de licence émise pour un autre poste ({})",
            &[&licence_poste],
        ));
    }
    let mut modules = Vec::new();
    for code in v
        .get("modules")
        .and_then(Value::as_array)
        .ok_or_else(illisible)?
    {
        let code = code.as_str().ok_or_else(illisible)?;
        modules.push(Module::depuis_code(code).ok_or_else(|| {
            MessageCompose::new("module inconnu dans la clé de licence : {}", &[&code])
        })?);
    }
    Ok(Licence {
        client: texte("client")?,
        poste: licence_poste,
        modules: normaliser(&modules)?,
        emise_le: texte("emise_le")?,
    })
}

/// Identifiant de poste dérivé d'un identifiant machine : empreinte SHA-256,
/// 80 premiers bits en base 32 de Crockford (sans I, L, O, U), en quatre groupes
/// de quatre caractères faciles à dicter.
pub fn identifiant_poste(identifiant_machine: &str) -> String {
    const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let empreinte = Sha256::digest(
        format!("kadenz-poste:{}", identifiant_machine.trim().to_lowercase()).as_bytes(),
    );
    let mut bits: u128 = 0;
    for o in &empreinte[..10] {
        bits = (bits << 8) | u128::from(*o);
    }
    let caracteres: Vec<char> = (0..16)
        .map(|k| ALPHABET[((bits >> (75 - 5 * k)) & 31) as usize] as char)
        .collect();
    caracteres
        .chunks(4)
        .map(|g| g.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("-")
}

/// Identifiant de ce poste, tiré du MachineGuid de Windows : stable aux mises à
/// jour de Windows, il change à la réinstallation du système.
pub fn poste_local() -> Result<String, String> {
    machine_guid().map(|g| identifiant_poste(&g))
}

#[cfg(windows)]
fn machine_guid() -> Result<String, String> {
    use windows_sys::Win32::System::Registry::{
        HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RRF_SUBKEY_WOW6464KEY, RegGetValueW,
    };
    let large = |s: &str| s.encode_utf16().chain([0]).collect::<Vec<u16>>();
    let cle = large("SOFTWARE\\Microsoft\\Cryptography");
    let valeur = large("MachineGuid");
    let mut tampon = [0u16; 128];
    let mut taille = (tampon.len() * 2) as u32;
    // SAFETY : chaînes terminées par un zéro ; tampon et taille en octets cohérents.
    let code = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            cle.as_ptr(),
            valeur.as_ptr(),
            RRF_RT_REG_SZ | RRF_SUBKEY_WOW6464KEY,
            std::ptr::null_mut(),
            tampon.as_mut_ptr().cast(),
            &mut taille,
        )
    };
    if code != 0 {
        return Err(format!("identifiant machine illisible (code {code})"));
    }
    let n = (taille as usize / 2).saturating_sub(1).min(tampon.len());
    let guid = String::from_utf16_lossy(&tampon[..n]);
    if guid.trim().is_empty() {
        return Err("identifiant machine vide".into());
    }
    Ok(guid)
}

#[cfg(not(windows))]
fn machine_guid() -> Result<String, String> {
    Err("identifiant de poste disponible sous Windows seulement".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paire() -> (SigningKey, VerifyingKey) {
        let privee = SigningKey::from_bytes(&[7u8; 32]);
        let publique = privee.verifying_key();
        (privee, publique)
    }

    fn licence() -> Licence {
        Licence {
            client: "Client Essai".into(),
            poste: "ABCD-EFGH-JKMN-PQRS".into(),
            modules: vec![Module::Relances, Module::Paiements, Module::Devis],
            emise_le: "2026-11-11".into(),
        }
    }

    #[test]
    fn aller_retour() {
        let (privee, publique) = paire();
        let cle = emettre(&licence(), &privee).unwrap();
        let lue = verifier_avec(&cle, "ABCD-EFGH-JKMN-PQRS", &publique).unwrap();
        assert_eq!(
            lue.modules,
            [Module::Devis, Module::Paiements, Module::Relances]
        );
        assert_eq!(lue.client, "Client Essai");
        // Espaces et retours à la ligne d'un copier-coller ignorés.
        let coupee = format!("{}\n {}", &cle[..20], &cle[20..]);
        assert!(verifier_avec(&coupee, "ABCD-EFGH-JKMN-PQRS", &publique).is_ok());
    }

    #[test]
    fn rotation_des_cles() {
        let (actuelle, publique_actuelle) = paire();
        let suivante = SigningKey::from_bytes(&[8u8; 32]);
        let cles = [publique_actuelle, suivante.verifying_key()];
        let poste = "ABCD-EFGH-JKMN-PQRS";

        // Licence signée par la clé suivante : choisie par son identifiant.
        let nouvelle = emettre(&licence(), &suivante).unwrap();
        assert_eq!(
            verifier_parmi(&nouvelle, poste, &cles).unwrap().client,
            "Client Essai"
        );
        // Un poste qui ne connaît que la clé actuelle la refuse.
        assert_eq!(
            verifier_parmi(&nouvelle, poste, &cles[..1])
                .unwrap_err()
                .modele,
            "clé de signature de la licence inconnue : {}"
        );

        // Licence d'avant le pont, sans champ `cle` : vérifiée par la clé actuelle.
        let donnees = json!({
            "version": VERSION,
            "client": "Ancien client",
            "poste": poste,
            "modules": ["devis"],
            "emise_le": "2026-11-10",
        })
        .to_string();
        let signature = actuelle.sign(donnees.as_bytes());
        let ancienne = format!(
            "{PREFIXE}{}.{}",
            URL_SAFE_NO_PAD.encode(donnees.as_bytes()),
            URL_SAFE_NO_PAD.encode(signature.to_bytes())
        );
        assert_eq!(
            verifier_parmi(&ancienne, poste, &cles).unwrap().client,
            "Ancien client"
        );
        // Changer le champ `cle` casse la signature.
        let emise = emettre(&licence(), &actuelle).unwrap();
        let (d, sig) = emise
            .strip_prefix(PREFIXE)
            .unwrap()
            .split_once('.')
            .unwrap();
        let json = String::from_utf8(URL_SAFE_NO_PAD.decode(d).unwrap()).unwrap();
        let id_actuel = identifiant_cle(&publique_actuelle);
        let id_suivant = identifiant_cle(&suivante.verifying_key());
        let forgee = format!(
            "{PREFIXE}{}.{sig}",
            URL_SAFE_NO_PAD.encode(json.replace(&id_actuel, &id_suivant))
        );
        assert_eq!(
            verifier_parmi(&forgee, poste, &cles).unwrap_err().modele,
            "signature de la clé de licence invalide"
        );
    }

    #[test]
    fn cle_actuelle_embarquee() {
        assert!(cle_publique().is_ok());
        assert_eq!(identifiant_cle(&cle_publique().unwrap()).len(), 16);
    }

    #[test]
    fn refus() {
        let (privee, publique) = paire();
        let cle = emettre(&licence(), &privee).unwrap();
        assert_eq!(
            verifier_avec(&cle, "AAAA-AAAA-AAAA-AAAA", &publique)
                .unwrap_err()
                .modele,
            "clé de licence émise pour un autre poste ({})"
        );
        // Données modifiées : signature refusée.
        let (donnees, signature) = cle.strip_prefix(PREFIXE).unwrap().split_once('.').unwrap();
        let mut json = String::from_utf8(URL_SAFE_NO_PAD.decode(donnees).unwrap()).unwrap();
        json = json.replace("\"devis\"", "\"proforma\"");
        let forgee = format!("{PREFIXE}{}.{signature}", URL_SAFE_NO_PAD.encode(json));
        assert_eq!(
            verifier_avec(&forgee, "ABCD-EFGH-JKMN-PQRS", &publique)
                .unwrap_err()
                .modele,
            "signature de la clé de licence invalide"
        );
        // Autre éditeur.
        let autre = SigningKey::from_bytes(&[9u8; 32]).verifying_key();
        assert!(verifier_avec(&cle, "ABCD-EFGH-JKMN-PQRS", &autre).is_err());
        assert_eq!(
            verifier_avec("n'importe quoi", "X", &publique)
                .unwrap_err()
                .modele,
            "clé de licence illisible"
        );
        // Relances sans Paiements refusé à l'émission.
        let sans = Licence {
            modules: vec![Module::Relances],
            ..licence()
        };
        assert_eq!(
            emettre(&sans, &privee).unwrap_err().modele,
            "le module « {} » exige le module « {} »"
        );
    }

    #[test]
    fn identifiant_stable_et_lisible() {
        let a = identifiant_poste("1b2c3d4e-0000-1111-2222-333344445555");
        assert_eq!(
            a,
            identifiant_poste("1B2C3D4E-0000-1111-2222-333344445555 ")
        );
        assert_eq!(a.len(), 19);
        assert!(
            a.chars()
                .all(|c| c == '-' || (c.is_ascii_alphanumeric() && !"ILOU".contains(c)))
        );
        assert_ne!(a, identifiant_poste("autre"));
    }

    #[test]
    fn hex() {
        let (privee, publique) = paire();
        assert_eq!(
            cle_publique_depuis_hex(&en_hex(publique.as_bytes())),
            Some(publique)
        );
        assert_eq!(
            cle_privee_depuis_hex(&en_hex(&privee.to_bytes())).map(|k| k.verifying_key()),
            Some(publique)
        );
        assert!(cle_publique_depuis_hex("a_generer").is_none());
    }

    #[test]
    fn cle_publique_embarquee_lisible() {
        assert!(cle_publique().is_ok());
    }

    #[cfg(windows)]
    #[test]
    fn poste_de_ce_pc() {
        let p = poste_local().unwrap();
        assert_eq!(p, poste_local().unwrap());
        assert_eq!(p.len(), 19);
    }
}
