//! Licence d'abonnement d'une entreprise (phase 2, jalon F3 ; décisions 21 et
//! 29).
//!
//! Liée à l'identifiant de l'entreprise (et non à un poste), datée (début et
//! échéance), avec un nombre de postes, les modules et l'émetteur. Signée par
//! la clé du portail, la clé suivante embarquée par le pont V1, jamais par la
//! clé actuelle, qui reste hors ligne pour les licences V1. En attendant le
//! portail (F9), `kadenz-editeur` l'émet avec la clé privée du portail.
//!
//! Format : `KADENZ-E-<données>.<signature>`, comme la clé V1. Les données
//! signées portent `type: "entreprise"` et `version: 2` : une licence V1 n'est
//! jamais prise pour une licence d'entreprise, ni l'inverse.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use kadenz_domaine::message::MessageCompose;
use kadenz_domaine::module::Module;
use serde_json::{Value, json};

use crate::{CLE_PUBLIQUE_SUIVANTE, cle_publique_depuis_hex, identifiant_cle, normaliser};

pub const PREFIXE: &str = "KADENZ-E-";
const VERSION: u64 = 2;
const TYPE: &str = "entreprise";

/// Modèles d'erreur propres à la licence d'entreprise, pour leurs traductions.
pub const MODELES_ERREUR: &[&str] = &[
    "ce n'est pas une licence d'abonnement d'entreprise",
    "licence d'abonnement émise pour une autre entreprise ({})",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Formule {
    Mensuelle,
    Annuelle,
}

impl Formule {
    pub fn code(self) -> &'static str {
        match self {
            Formule::Mensuelle => "mensuelle",
            Formule::Annuelle => "annuelle",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Formule> {
        match code {
            "mensuelle" => Some(Formule::Mensuelle),
            "annuelle" => Some(Formule::Annuelle),
            _ => None,
        }
    }
}

/// Émetteur de la licence : l'éditeur ou un revendeur (décision 29).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Emetteur {
    Editeur,
    Revendeur { identifiant: String, nom: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LicenceEntreprise {
    /// Identifiant de l'entreprise (UUID tiré à la création de sa base).
    pub entreprise: String,
    /// Raison sociale du client.
    pub client: String,
    pub formule: Formule,
    /// Postes permis.
    pub postes: u32,
    /// Modules activés, dans l'ordre de `Module::TOUS`.
    pub modules: Vec<Module>,
    /// Premier jour couvert, AAAA-MM-JJ.
    pub debut: String,
    /// Dernier jour couvert, AAAA-MM-JJ.
    pub echeance: String,
    pub emetteur: Emetteur,
    /// Date d'émission, AAAA-MM-JJ.
    pub emise_le: String,
}

fn erreur(modele: &'static str) -> MessageCompose {
    MessageCompose::new(modele, &[])
}

fn illisible() -> MessageCompose {
    erreur("clé de licence illisible")
}

/// Date AAAA-MM-JJ bien formée (mois et jour plausibles) ; le calendrier exact
/// est contrôlé par le stockage.
fn date_lisible(texte: &str) -> bool {
    let o = texte.as_bytes();
    let chiffres = |r: std::ops::Range<usize>| o[r].iter().all(u8::is_ascii_digit);
    if o.len() != 10 || o[4] != b'-' || o[7] != b'-' {
        return false;
    }
    if !(chiffres(0..4) && chiffres(5..7) && chiffres(8..10)) {
        return false;
    }
    let mois: u32 = texte[5..7].parse().unwrap_or(0);
    let jour: u32 = texte[8..10].parse().unwrap_or(0);
    (1..=12).contains(&mois) && (1..=31).contains(&jour)
}

/// Clés du portail acceptées : la clé suivante embarquée ; une liste, pour
/// une rotation future. Vide tant qu'elle n'est pas générée.
pub fn cles_portail() -> Vec<VerifyingKey> {
    cle_publique_depuis_hex(CLE_PUBLIQUE_SUIVANTE)
        .into_iter()
        .collect()
}

/// Émet une licence d'entreprise signée (outil éditeur, puis portail).
pub fn emettre(
    licence: &LicenceEntreprise,
    cle_privee: &SigningKey,
) -> Result<String, MessageCompose> {
    let mut donnees = json!({
        "type": TYPE,
        "version": VERSION,
        "entreprise": licence.entreprise,
        "client": licence.client,
        "formule": licence.formule.code(),
        "postes": licence.postes,
        "modules": normaliser(&licence.modules)?.iter().map(|m| m.code()).collect::<Vec<_>>(),
        "debut": licence.debut,
        "echeance": licence.echeance,
        "emise_le": licence.emise_le,
        "cle": identifiant_cle(&cle_privee.verifying_key()),
    });
    donnees["emetteur"] = match &licence.emetteur {
        Emetteur::Editeur => json!({ "type": "editeur" }),
        Emetteur::Revendeur { identifiant, nom } => {
            json!({ "type": "revendeur", "identifiant": identifiant, "nom": nom })
        }
    };
    let donnees = donnees.to_string();
    let signature = cle_privee.sign(donnees.as_bytes());
    Ok(format!(
        "{PREFIXE}{}.{}",
        URL_SAFE_NO_PAD.encode(donnees.as_bytes()),
        URL_SAFE_NO_PAD.encode(signature.to_bytes())
    ))
}

/// Vérifie une licence d'entreprise parmi les clés du portail données, pour
/// l'entreprise donnée.
pub fn verifier(
    cle: &str,
    entreprise: &str,
    cles_portail: &[VerifyingKey],
) -> Result<LicenceEntreprise, MessageCompose> {
    let cle: String = cle.chars().filter(|c| !c.is_whitespace()).collect();
    let corps = cle
        .strip_prefix(PREFIXE)
        .ok_or_else(|| erreur("ce n'est pas une licence d'abonnement d'entreprise"))?;
    let (donnees, signature) = corps.split_once('.').ok_or_else(illisible)?;
    let donnees = URL_SAFE_NO_PAD.decode(donnees).map_err(|_| illisible())?;
    let signature = URL_SAFE_NO_PAD.decode(signature).map_err(|_| illisible())?;
    let signature = Signature::from_slice(&signature).map_err(|_| illisible())?;
    let v: Value = serde_json::from_slice(&donnees).map_err(|_| illisible())?;
    // Le champ `cle` ne fait que choisir la clé ; la signature couvre tout.
    let id = v.get("cle").and_then(Value::as_str).ok_or_else(illisible)?;
    let cle_publique = cles_portail
        .iter()
        .find(|c| identifiant_cle(c) == id)
        .ok_or_else(|| {
            MessageCompose::new("clé de signature de la licence inconnue : {}", &[&id])
        })?;
    cle_publique
        .verify(&donnees, &signature)
        .map_err(|_| erreur("signature de la clé de licence invalide"))?;

    if v.get("type").and_then(Value::as_str) != Some(TYPE) {
        return Err(erreur("ce n'est pas une licence d'abonnement d'entreprise"));
    }
    let version = v.get("version").and_then(Value::as_u64).unwrap_or(0);
    if version != VERSION {
        return Err(MessageCompose::new(
            "version de clé de licence non prise en charge : {}",
            &[&version],
        ));
    }
    let texte = |champ: &str| {
        v.get(champ)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(illisible)
    };
    let date = |champ: &str| {
        texte(champ).and_then(|d| date_lisible(&d).then_some(d).ok_or_else(illisible))
    };
    let licence_entreprise = texte("entreprise")?;
    if licence_entreprise != entreprise.trim() {
        return Err(MessageCompose::new(
            "licence d'abonnement émise pour une autre entreprise ({})",
            &[&licence_entreprise],
        ));
    }
    let formule = Formule::depuis_code(&texte("formule")?).ok_or_else(illisible)?;
    let postes = v
        .get("postes")
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
        .filter(|&n| n >= 1)
        .ok_or_else(illisible)?;
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
    let e = v.get("emetteur").ok_or_else(illisible)?;
    let champ_emetteur = |champ: &str| {
        e.get(champ)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(illisible)
    };
    let emetteur = match champ_emetteur("type")?.as_str() {
        "editeur" => Emetteur::Editeur,
        "revendeur" => Emetteur::Revendeur {
            identifiant: champ_emetteur("identifiant")?,
            nom: champ_emetteur("nom")?,
        },
        _ => return Err(illisible()),
    };
    let (debut, echeance) = (date("debut")?, date("echeance")?);
    if debut > echeance {
        return Err(illisible());
    }
    Ok(LicenceEntreprise {
        entreprise: licence_entreprise,
        client: texte("client")?,
        formule,
        postes,
        modules: normaliser(&modules)?,
        debut,
        echeance,
        emetteur,
        emise_le: date("emise_le")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paire() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    fn exemple() -> LicenceEntreprise {
        LicenceEntreprise {
            entreprise: "0190a1b2-0000-7000-8000-000000000001".into(),
            client: "Quincaillerie du Centre".into(),
            formule: Formule::Annuelle,
            postes: 5,
            modules: vec![Module::Paiements, Module::Devis],
            debut: "2026-11-12".into(),
            echeance: "2027-11-11".into(),
            emetteur: Emetteur::Revendeur {
                identifiant: "rev-01".into(),
                nom: "Informatique Plus".into(),
            },
            emise_le: "2026-11-12".into(),
        }
    }

    #[test]
    fn aller_retour() {
        let cle = paire();
        let texte = emettre(&exemple(), &cle).unwrap();
        assert!(texte.starts_with(PREFIXE));
        let lue = verifier(&texte, &exemple().entreprise, &[cle.verifying_key()]).unwrap();
        let mut attendue = exemple();
        attendue.modules = vec![Module::Devis, Module::Paiements];
        assert_eq!(lue, attendue);
    }

    #[test]
    fn refus() {
        let cle = paire();
        let publique = [cle.verifying_key()];
        let texte = emettre(&exemple(), &cle).unwrap();
        let modele =
            |r: Result<LicenceEntreprise, MessageCompose>| r.unwrap_err().modele.to_string();
        assert_eq!(
            modele(verifier(&texte, "autre", &publique)),
            "licence d'abonnement émise pour une autre entreprise ({})"
        );
        // Signée par une autre clé que celles du portail.
        let autre = SigningKey::from_bytes(&[9u8; 32]);
        assert_eq!(
            modele(verifier(
                &emettre(&exemple(), &autre).unwrap(),
                &exemple().entreprise,
                &publique
            )),
            "clé de signature de la licence inconnue : {}"
        );
        // Licence V1, même signée par la clé du portail.
        let v1 = crate::emettre(
            &crate::Licence {
                client: "X".into(),
                poste: "AAAA-BBBB-CCCC-DDDD".into(),
                modules: vec![],
                emise_le: "2026-11-12".into(),
            },
            &cle,
        )
        .unwrap();
        assert_eq!(
            modele(verifier(&v1, &exemple().entreprise, &publique)),
            "ce n'est pas une licence d'abonnement d'entreprise"
        );
        // Et une licence d'entreprise n'est pas une licence V1.
        assert!(crate::verifier_parmi(&texte, "AAAA-BBBB-CCCC-DDDD", &publique).is_err());
        // Données modifiées après la signature.
        let (donnees, signature) = texte[PREFIXE.len()..].split_once('.').unwrap();
        let mut json: Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(donnees).unwrap()).unwrap();
        json["postes"] = json!(50);
        let falsifiee = format!(
            "{PREFIXE}{}.{signature}",
            URL_SAFE_NO_PAD.encode(json.to_string())
        );
        assert_eq!(
            modele(verifier(&falsifiee, &exemple().entreprise, &publique)),
            "signature de la clé de licence invalide"
        );
    }

    #[test]
    fn dates() {
        assert!(date_lisible("2026-02-28"));
        assert!(!date_lisible("2026-13-01"));
        assert!(!date_lisible("26-01-01"));
        let mut l = exemple();
        l.echeance = "2026-01-01".into();
        let cle = paire();
        let texte = emettre(&l, &cle).unwrap();
        assert!(verifier(&texte, &l.entreprise, &[cle.verifying_key()]).is_err());
    }
}
