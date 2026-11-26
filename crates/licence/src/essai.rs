//! Attestation d'essai gratuit (décision 78,
//! `docs/spec-telechargements-essai.md`, section 6).
//!
//! Le portail connaît l'essai de chaque poste et rend sa date de début, signée
//! par sa clé : `KADENZ-T-<données>.<signature>`, comme la licence
//! d'entreprise. Les données portent `type: "essai"`, l'identifiant du poste,
//! l'édition et la date de début. Le programme garde l'attestation (cache
//! hors ligne, `%ProgramData%\Kadenz\essai.json`) ; modifiée, elle est
//! refusée. Une attestation n'est jamais prise pour une licence : son type et
//! son préfixe diffèrent.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use kadenz_domaine::message::MessageCompose;
use serde_json::{Value, json};

use crate::entreprise::{Produit, date_lisible};
use crate::identifiant_cle;

const PREFIXE: &str = "KADENZ-T-";
const VERSION: u64 = 1;
const TYPE: &str = "essai";

/// Modèles d'erreur de ce module, pour vérifier leurs traductions.
pub const MODELES_ERREUR: &[&str] = &[
    "attestation d'essai illisible",
    "attestation d'essai émise pour un autre poste",
    "attestation d'essai signée par une clé inconnue",
];

/// Essai attesté par le portail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Essai {
    /// Identifiant du poste (`identifiant_poste`, `XXXX-XXXX-XXXX-XXXX`).
    pub poste: String,
    pub produit: Produit,
    /// Date de début de l'essai, AAAA-MM-JJ.
    pub debut: String,
    /// Date d'émission de l'attestation, AAAA-MM-JJ.
    pub emise_le: String,
}

fn erreur(modele: &'static str) -> MessageCompose {
    MessageCompose::new(modele, &[])
}

/// Signe l'attestation d'essai avec la clé du portail.
pub fn emettre(essai: &Essai, cle_privee: &SigningKey) -> Result<String, MessageCompose> {
    if !date_lisible(&essai.debut) || !date_lisible(&essai.emise_le) {
        return Err(erreur(MODELES_ERREUR[0]));
    }
    let donnees = json!({
        "type": TYPE,
        "version": VERSION,
        "poste": essai.poste,
        "produit": essai.produit.code(),
        "debut": essai.debut,
        "emise_le": essai.emise_le,
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

/// Vérifie une attestation pour ce poste et cette édition, parmi les clés du
/// portail données.
pub fn verifier(
    texte: &str,
    poste: &str,
    produit: Produit,
    cles_portail: &[VerifyingKey],
) -> Result<Essai, MessageCompose> {
    let illisible = || erreur(MODELES_ERREUR[0]);
    let texte: String = texte.chars().filter(|c| !c.is_whitespace()).collect();
    let corps = texte.strip_prefix(PREFIXE).ok_or_else(illisible)?;
    let (donnees, signature) = corps.split_once('.').ok_or_else(illisible)?;
    let donnees = URL_SAFE_NO_PAD.decode(donnees).map_err(|_| illisible())?;
    let signature = URL_SAFE_NO_PAD.decode(signature).map_err(|_| illisible())?;
    let signature = Signature::from_slice(&signature).map_err(|_| illisible())?;
    let v: Value = serde_json::from_slice(&donnees).map_err(|_| illisible())?;
    let id = v.get("cle").and_then(Value::as_str).ok_or_else(illisible)?;
    let cle = cles_portail
        .iter()
        .find(|c| identifiant_cle(c) == id)
        .ok_or_else(|| erreur(MODELES_ERREUR[2]))?;
    cle.verify(&donnees, &signature).map_err(|_| illisible())?;
    let texte_de = |nom: &str| v.get(nom).and_then(Value::as_str).map(str::to_string);
    if texte_de("type").as_deref() != Some(TYPE)
        || v.get("version").and_then(Value::as_u64) != Some(VERSION)
    {
        return Err(illisible());
    }
    let essai = Essai {
        poste: texte_de("poste").ok_or_else(illisible)?,
        produit: texte_de("produit")
            .and_then(|p| Produit::depuis_code(&p))
            .ok_or_else(illisible)?,
        debut: texte_de("debut")
            .filter(|d| date_lisible(d))
            .ok_or_else(illisible)?,
        emise_le: texte_de("emise_le")
            .filter(|d| date_lisible(d))
            .ok_or_else(illisible)?,
    };
    if !essai.poste.eq_ignore_ascii_case(poste.trim()) || essai.produit != produit {
        return Err(erreur(MODELES_ERREUR[1]));
    }
    Ok(essai)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exemple() -> Essai {
        Essai {
            poste: "ABCD-EFGH-JKMN-PQRS".into(),
            produit: Produit::Monoposte,
            debut: "2026-11-20".into(),
            emise_le: "2026-11-26".into(),
        }
    }

    #[test]
    fn aller_retour_et_refus() {
        let cle = SigningKey::from_bytes(&[5u8; 32]);
        let publique = [cle.verifying_key()];
        let texte = emettre(&exemple(), &cle).unwrap();
        assert!(texte.starts_with(PREFIXE));
        let lu = verifier(&texte, "abcd-efgh-jkmn-pqrs", Produit::Monoposte, &publique).unwrap();
        assert_eq!(lu, exemple());
        let modele = |r: Result<Essai, MessageCompose>| r.unwrap_err().modele.to_string();
        // Autre poste, autre édition.
        assert_eq!(
            modele(verifier(
                &texte,
                "WXYZ-EFGH-JKMN-PQRS",
                Produit::Monoposte,
                &publique
            )),
            MODELES_ERREUR[1]
        );
        assert_eq!(
            modele(verifier(
                &texte,
                &exemple().poste,
                Produit::Entreprise,
                &publique
            )),
            MODELES_ERREUR[1]
        );
        // Autre clé.
        let autre = [SigningKey::from_bytes(&[6u8; 32]).verifying_key()];
        assert_eq!(
            modele(verifier(
                &texte,
                &exemple().poste,
                Produit::Monoposte,
                &autre
            )),
            MODELES_ERREUR[2]
        );
        // Date avancée après la signature.
        let (donnees, signature) = texte[PREFIXE.len()..].split_once('.').unwrap();
        let mut json: Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(donnees).unwrap()).unwrap();
        json["debut"] = json!("2027-01-01");
        let falsifiee = format!(
            "{PREFIXE}{}.{signature}",
            URL_SAFE_NO_PAD.encode(json.to_string())
        );
        assert_eq!(
            modele(verifier(
                &falsifiee,
                &exemple().poste,
                Produit::Monoposte,
                &publique
            )),
            MODELES_ERREUR[0]
        );
        // Une licence d'entreprise n'est pas une attestation.
        assert!(
            verifier(
                "KADENZ-E-abc.def",
                &exemple().poste,
                Produit::Monoposte,
                &publique
            )
            .is_err()
        );
    }
}
