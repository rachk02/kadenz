//! Activation d'une licence par un code court (décision 74,
//! `docs/spec-codes-licence.md`, section 5).
//!
//! Le programme où le code est tapé (KadenzAdmin, ou la rubrique
//! « Abonnement » d'un poste Admin) l'envoie au portail avec l'identifiant de
//! l'entreprise et sa raison sociale ; le portail renvoie la licence signée,
//! qui suit ensuite le chemin d'installation habituel. Le service du serveur,
//! qui tourne sous SYSTEM, n'appelle jamais le portail.
//!
//! Rien d'autre n'est envoyé que le code, l'identifiant de l'entreprise, sa
//! raison sociale et la version du programme.

use std::io::Read;
use std::time::Duration;

use kadenz_domaine::message::MessageCompose;
use kadenz_licence::code_court::{self, MODELES_ACTIVATION};
use ureq::Agent;
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

use crate::VERSION;

/// Adresse du portail (par exemple `https://portail.exemple`), sans `/` final.
/// Vide tant que le nom de domaine n'est pas choisi (décision 72) : l'activation
/// répond alors qu'elle est indisponible.
const ADRESSE_PORTAIL: &str = include_str!("../portail-url.txt");

/// Variable des versions de développement : adresse d'un portail d'essai
/// (par exemple `http://127.0.0.1:47382`), qui remplace l'adresse embarquée.
/// Sans effet dans une version optimisée.
pub const PORTAIL_ESSAI: &str = "KADENZ_PORTAIL";

/// Délai d'une activation, connexion comprise.
const DELAI: Duration = Duration::from_secs(20);

/// Réponse du portail lue au plus (une licence fait moins de 2 Ko).
const TAILLE_MAX_REPONSE: u64 = 64 * 1024;

/// Modèles d'erreur propres au client, pour vérifier leurs traductions ; les
/// refus du portail sont dans [`MODELES_ACTIVATION`].
pub const MODELES_ERREUR: &[&str] = &[
    "activation indisponible : adresse du portail non configurée",
    "portail d'activation injoignable : vérifiez la connexion à Internet",
    "réponse inattendue du portail d'activation",
];

/// Licence reçue et son résumé, pour l'écran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Activation {
    /// Licence signée `KADENZ-E-...`, à installer comme aujourd'hui.
    pub licence: String,
    pub postes: u32,
    /// `mensuelle` ou `annuelle`.
    pub formule: String,
    /// Dates AAAA-MM-JJ.
    pub debut: String,
    pub echeance: String,
}

fn fixe(modele: &'static str) -> MessageCompose {
    MessageCompose::new(modele, &[])
}

/// Adresse du portail à appeler ; `None` si elle n'est pas configurée.
pub fn adresse_portail() -> Option<String> {
    #[cfg(debug_assertions)]
    if let Some(a) = std::env::var(PORTAIL_ESSAI).ok().filter(|v| !v.trim().is_empty()) {
        return Some(a.trim().trim_end_matches('/').to_string());
    }
    Some(ADRESSE_PORTAIL.trim().trim_end_matches('/').to_string()).filter(|a| !a.is_empty())
}

/// Active `code` pour l'entreprise auprès du portail configuré. Bloquant (20
/// secondes au plus) : à appeler hors du fil de l'interface.
pub fn activer(code: &str, entreprise: &str, client: &str) -> Result<Activation, MessageCompose> {
    // Contrôle local d'abord : un code mal saisi ne part pas sur le réseau.
    let code = code_court::verifier(code)?;
    let adresse = adresse_portail().ok_or_else(|| fixe(MODELES_ERREUR[0]))?;
    activer_aupres(&adresse, &code, entreprise, client)
}

/// Active `code` auprès du portail à `adresse`. Une version optimisée
/// n'appelle qu'une adresse `https://`.
pub fn activer_aupres(
    adresse: &str,
    code: &str,
    entreprise: &str,
    client: &str,
) -> Result<Activation, MessageCompose> {
    let code = code_court::verifier(code)?;
    if !cfg!(debug_assertions) && !adresse.starts_with("https://") {
        return Err(fixe(MODELES_ERREUR[0]));
    }
    let corps = serde_json::json!({
        "code": code,
        "entreprise": entreprise.trim(),
        "client": client.trim(),
        "version": VERSION,
    })
    .to_string();
    let reponse = agent()
        .post(&format!("{adresse}/api/activation"))
        .header("Content-Type", "application/json")
        .send(corps.as_str())
        .map_err(|_| fixe(MODELES_ERREUR[1]))?;
    let mut texte = String::new();
    reponse
        .into_body()
        .into_with_config()
        .limit(TAILLE_MAX_REPONSE)
        .reader()
        .read_to_string(&mut texte)
        .map_err(|_| fixe(MODELES_ERREUR[2]))?;
    lire_reponse(&texte)
}

fn agent() -> Agent {
    Agent::config_builder()
        .user_agent(format!("Kadenz/{VERSION}"))
        .timeout_connect(Some(Duration::from_secs(8)))
        .timeout_global(Some(DELAI))
        // Les refus arrivent en 4xx avec un corps JSON à lire.
        .http_status_as_error(false)
        .tls_config(
            TlsConfig::builder()
                .provider(TlsProvider::NativeTls)
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .into()
}

/// Lit la réponse du portail : la licence et son résumé, ou un refus connu
/// (modèle de [`MODELES_ACTIVATION`] et ses valeurs).
fn lire_reponse(texte: &str) -> Result<Activation, MessageCompose> {
    let inattendue = || fixe(MODELES_ERREUR[2]);
    let v: serde_json::Value = serde_json::from_str(texte).map_err(|_| inattendue())?;
    if let Some(erreur) = v.get("erreur").and_then(serde_json::Value::as_str) {
        let modele = MODELES_ACTIVATION
            .iter()
            .find(|m| **m == erreur)
            .ok_or_else(inattendue)?;
        let valeurs: Vec<String> = v
            .get("valeurs")
            .and_then(serde_json::Value::as_array)
            .map(|l| {
                l.iter()
                    .map(|x| x.as_str().map(str::to_string).unwrap_or_else(|| x.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let valeurs: Vec<&dyn std::fmt::Display> =
            valeurs.iter().map(|x| x as &dyn std::fmt::Display).collect();
        return Err(MessageCompose::new(modele, &valeurs));
    }
    let texte_de = |nom: &str| v.get(nom).and_then(serde_json::Value::as_str).map(str::to_string);
    let licence = texte_de("licence")
        .filter(|l| l.starts_with("KADENZ-E-"))
        .ok_or_else(inattendue)?;
    Ok(Activation {
        licence,
        postes: v
            .get("postes")
            .and_then(serde_json::Value::as_u64)
            .and_then(|p| u32::try_from(p).ok())
            .ok_or_else(inattendue)?,
        formule: texte_de("formule").ok_or_else(inattendue)?,
        debut: texte_de("debut").ok_or_else(inattendue)?,
        echeance: texte_de("echeance").ok_or_else(inattendue)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reponses_lues() {
        let ok = lire_reponse(
            r#"{"licence":"KADENZ-E-abc.def","postes":3,"formule":"annuelle","debut":"2026-11-15","echeance":"2027-11-14"}"#,
        )
        .unwrap();
        assert_eq!(ok.postes, 3);
        assert_eq!(ok.echeance, "2027-11-14");

        let refus = lire_reponse(&format!(
            r#"{{"erreur":"{}","valeurs":["40"]}}"#,
            MODELES_ACTIVATION[2]
        ))
        .unwrap_err();
        assert_eq!(refus.modele, MODELES_ACTIVATION[2]);
        assert_eq!(refus.valeurs, vec!["40".to_string()]);

        // Modèle inconnu, licence d'une autre sorte, texte illisible.
        for texte in [
            r#"{"erreur":"autre chose"}"#,
            r#"{"licence":"KADENZ-abc","postes":3,"formule":"annuelle","debut":"a","echeance":"b"}"#,
            "<html>",
        ] {
            assert_eq!(lire_reponse(texte).unwrap_err().modele, MODELES_ERREUR[2], "{texte}");
        }
    }

    #[test]
    fn code_mal_saisi_refuse_avant_le_reseau() {
        let e = activer_aupres("http://127.0.0.1:9", "ABCDE", "x", "y").unwrap_err();
        assert_eq!(e.modele, code_court::MODELES_ERREUR[0]);
    }

    #[test]
    fn portail_injoignable() {
        let code = code_court::tirer().unwrap();
        // Port 9 (discard) : rien n'écoute en local.
        let e = activer_aupres("http://127.0.0.1:9", &code, "x", "y").unwrap_err();
        assert_eq!(e.modele, MODELES_ERREUR[1]);
    }
}
