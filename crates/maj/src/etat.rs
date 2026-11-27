//! État de licence relayé au portail (décision 80,
//! `docs/spec-revocation-licences.md`, section 3).
//!
//! Un poste demande à son serveur l'état de la licence (entreprise, édition,
//! postes inscrits), l'envoie au portail, et rend au serveur le bulletin signé
//! que le portail répond. Le serveur ne joint jamais Internet lui-même. Aucune
//! donnée de gestion ne part : seulement le nom, l'empreinte du certificat,
//! l'identifiant de l'ordinateur et les dates d'inscription et de dernier
//! contact de chaque poste.

use std::io::Read;

use kadenz_domaine::message::MessageCompose;

use crate::VERSION;
use crate::activation::{MODELES_ERREUR as ERREURS_PORTAIL, adresse_portail};

/// Réponse du portail lue au plus : un bulletin peut lister jusqu'à 1000
/// postes retirés et 1000 ordinateurs bloqués.
const TAILLE_MAX_REPONSE: u64 = 512 * 1024;

/// Poste inscrit, tel que le serveur le rapporte.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PosteEtat {
    pub empreinte: String,
    /// Identifiant de l'ordinateur ; vide pour un poste ancien.
    pub ordinateur: String,
    pub nom: String,
    pub inscrit_le: String,
    /// Dernier contact avec le serveur ; vide s'il n'est pas connu.
    pub vu_le: String,
}

fn fixe(modele: &'static str) -> MessageCompose {
    MessageCompose::new(modele, &[])
}

/// Envoie l'état de licence au portail configuré ; rend le bulletin reçu, non
/// encore vérifié (le serveur le vérifie). Bloquant (20 secondes au plus) : à
/// appeler hors du fil de l'interface.
pub fn echanger(
    entreprise: &str,
    produit: &str,
    postes: &[PosteEtat],
) -> Result<String, MessageCompose> {
    let adresse = adresse_portail().ok_or_else(|| fixe(ERREURS_PORTAIL[0]))?;
    if !cfg!(debug_assertions) && !adresse.starts_with("https://") {
        return Err(fixe(ERREURS_PORTAIL[0]));
    }
    echanger_aupres(&adresse, entreprise, produit, postes)
}

/// Comme [`echanger`], avec le portail à `adresse`.
pub fn echanger_aupres(
    adresse: &str,
    entreprise: &str,
    produit: &str,
    postes: &[PosteEtat],
) -> Result<String, MessageCompose> {
    let corps = corps(entreprise, produit, postes);
    let reponse = crate::activation::agent()
        .post(&format!("{adresse}/api/licence/etat"))
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

fn corps(entreprise: &str, produit: &str, postes: &[PosteEtat]) -> String {
    let postes: Vec<serde_json::Value> = postes
        .iter()
        .map(|p| {
            serde_json::json!({
                "empreinte": p.empreinte,
                "ordinateur": p.ordinateur,
                "nom": p.nom,
                "inscrit_le": p.inscrit_le,
                "vu_le": p.vu_le,
            })
        })
        .collect();
    serde_json::json!({
        "entreprise": entreprise,
        "produit": produit,
        "version": VERSION,
        "postes": postes,
    })
    .to_string()
}

/// Bulletin de la réponse du portail ; un refus du portail donne une réponse
/// inattendue (l'échange repart plus tard).
fn lire_reponse(texte: &str) -> Result<String, MessageCompose> {
    let v: serde_json::Value = serde_json::from_str(texte).map_err(|_| fixe(ERREURS_PORTAIL[2]))?;
    v.get("bulletin")
        .and_then(serde_json::Value::as_str)
        .filter(|b| b.starts_with("KADENZ-B-"))
        .map(str::to_string)
        .ok_or_else(|| fixe(ERREURS_PORTAIL[2]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corps_et_reponses() {
        let c: serde_json::Value = serde_json::from_str(&corps(
            "e1",
            "entreprise",
            &[PosteEtat {
                empreinte: "ab".into(),
                nom: "Caisse".into(),
                ..Default::default()
            }],
        ))
        .unwrap();
        assert_eq!(c["entreprise"], "e1");
        assert_eq!(c["produit"], "entreprise");
        assert_eq!(c["version"], VERSION);
        assert_eq!(c["postes"][0]["nom"], "Caisse");
        assert_eq!(c["postes"][0]["vu_le"], "");
        assert_eq!(
            lire_reponse(r#"{"bulletin":"KADENZ-B-abc.def"}"#).unwrap(),
            "KADENZ-B-abc.def"
        );
        assert!(lire_reponse(r#"{"erreur":"entreprise inconnue du portail"}"#).is_err());
        assert!(lire_reponse(r#"{"bulletin":"KADENZ-E-abc"}"#).is_err());
        assert!(lire_reponse("pas du json").is_err());
    }
}
