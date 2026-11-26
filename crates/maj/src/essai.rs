//! Essai gratuit confirmé par le portail (décision 78,
//! `docs/spec-telechargements-essai.md`, section 6).
//!
//! À l'installation sans code, le programme envoie au portail l'édition et
//! l'identifiant du poste (empreinte du `MachineGuid`, jamais le `MachineGuid`
//! lui-même), aucune donnée personnelle ; le portail rend la date de début de
//! l'essai de ce poste, signée. Sans connexion, l'essai démarre quand même ;
//! la demande repart à chaque lancement jusqu'à la réponse. L'attestation est gardée dans
//! `%ProgramData%\Kadenz\essai-<édition>.txt`, que la désinstallation ne
//! supprime pas : une réinstallation retrouve la date d'origine sans appeler
//! le portail. Ce fichier n'est qu'un cache, nommé et documenté : effacé, il
//! se reconstitue depuis le portail ; modifié, sa signature est refusée.

use std::io::Read;
use std::path::PathBuf;

use kadenz_domaine::message::MessageCompose;
use kadenz_licence::entreprise::{Produit, cles_portail};
use kadenz_licence::essai::{self as attestation, Essai};

use crate::VERSION;
use crate::activation::{MODELES_ERREUR as ERREURS_PORTAIL, adresse_portail};

/// Réponse du portail lue au plus.
const TAILLE_MAX_REPONSE: u64 = 16 * 1024;

fn fixe(modele: &'static str) -> MessageCompose {
    MessageCompose::new(modele, &[])
}

/// Fichier de l'attestation d'une édition, dans `%ProgramData%\Kadenz`.
pub fn chemin_cache(produit: Produit) -> Option<PathBuf> {
    let racine = std::env::var_os("ProgramData").filter(|v| !v.is_empty())?;
    Some(
        PathBuf::from(racine)
            .join("Kadenz")
            .join(format!("essai-{}.txt", produit.code())),
    )
}

/// Essai attesté gardé sur ce poste, s'il est valable pour ce poste et cette
/// édition.
pub fn lire_cache(produit: Produit, poste: &str) -> Option<Essai> {
    let texte = std::fs::read_to_string(chemin_cache(produit)?).ok()?;
    attestation::verifier(&texte, poste, produit, &cles_portail()).ok()
}

fn ecrire_cache(produit: Produit, texte: &str) {
    if let Some(chemin) = chemin_cache(produit) {
        if let Some(dossier) = chemin.parent() {
            let _ = std::fs::create_dir_all(dossier);
        }
        // Cache seulement : un échec d'écriture ne change rien à l'essai.
        let _ = std::fs::write(chemin, texte);
    }
}

/// Essai de ce poste pour cette édition : l'attestation gardée si elle est
/// valable, sinon celle du portail, vérifiée puis gardée. Bloquant (20
/// secondes au plus) : à appeler hors du fil de l'interface.
pub fn obtenir(produit: Produit) -> Result<Essai, MessageCompose> {
    let poste = kadenz_licence::poste_local().map_err(|_| fixe(ERREURS_PORTAIL[2]))?;
    if let Some(essai) = lire_cache(produit, &poste) {
        return Ok(essai);
    }
    let adresse = adresse_portail().ok_or_else(|| fixe(ERREURS_PORTAIL[0]))?;
    if !cfg!(debug_assertions) && !adresse.starts_with("https://") {
        return Err(fixe(ERREURS_PORTAIL[0]));
    }
    let texte = demander_aupres(&adresse, produit, &poste)?;
    let essai = attestation::verifier(&texte, &poste, produit, &cles_portail())
        .map_err(|_| fixe(ERREURS_PORTAIL[2]))?;
    ecrire_cache(produit, &texte);
    Ok(essai)
}

/// Demande l'essai au portail à `adresse` ; rend l'attestation reçue, non
/// encore vérifiée.
pub fn demander_aupres(
    adresse: &str,
    produit: Produit,
    poste: &str,
) -> Result<String, MessageCompose> {
    let corps = serde_json::json!({
        "produit": produit.code(),
        "poste": poste,
        "version": VERSION,
    })
    .to_string();
    let reponse = crate::activation::agent()
        .post(&format!("{adresse}/api/essai"))
        .header("Content-Type", "application/json")
        .send(corps.as_str())
        .map_err(|e| match e {
            ureq::Error::Tls(_) | ureq::Error::NativeTls(_) => fixe(ERREURS_PORTAIL[3]),
            _ => fixe(ERREURS_PORTAIL[1]),
        })?;
    let mut texte = String::new();
    reponse
        .into_body()
        .into_with_config()
        .limit(TAILLE_MAX_REPONSE)
        .reader()
        .read_to_string(&mut texte)
        .map_err(|_| fixe(ERREURS_PORTAIL[2]))?;
    lire_reponse(&texte)
}

/// Attestation de la réponse du portail ; un refus du portail donne une
/// réponse inattendue (le programme garde alors l'essai à confirmer).
fn lire_reponse(texte: &str) -> Result<String, MessageCompose> {
    let v: serde_json::Value = serde_json::from_str(texte).map_err(|_| fixe(ERREURS_PORTAIL[2]))?;
    v.get("attestation")
        .and_then(serde_json::Value::as_str)
        .filter(|a| a.starts_with("KADENZ-T-"))
        .map(str::to_string)
        .ok_or_else(|| fixe(ERREURS_PORTAIL[2]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reponses_du_portail() {
        assert_eq!(
            lire_reponse(r#"{"attestation":"KADENZ-T-abc.def","debut":"2026-11-26"}"#).unwrap(),
            "KADENZ-T-abc.def"
        );
        assert!(lire_reponse(r#"{"erreur":"demande d'essai invalide","valeurs":[]}"#).is_err());
        assert!(lire_reponse(r#"{"attestation":"KADENZ-E-abc.def"}"#).is_err());
        assert!(lire_reponse("pas du json").is_err());
    }

    #[test]
    fn cache_par_edition() {
        let chemin = chemin_cache(Produit::Monoposte);
        if std::env::var_os("ProgramData").is_some() {
            let chemin = chemin.unwrap();
            assert!(
                chemin.ends_with("Kadenz\\essai-monoposte.txt")
                    || chemin.ends_with("Kadenz/essai-monoposte.txt")
            );
        }
    }
}
