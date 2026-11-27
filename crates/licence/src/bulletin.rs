//! Bulletin de licence (décision 80, `docs/spec-revocation-licences.md`,
//! section 4).
//!
//! Le portail répond à chaque état de licence qu'un poste lui envoie par un
//! bulletin signé de sa clé : `KADENZ-B-<données>.<signature>`, comme la
//! licence d'entreprise et l'attestation d'essai. Il dit si la licence de
//! l'entreprise est révoquée, quels postes retirer (empreintes de certificat)
//! et quels ordinateurs sont bloqués (identifiants d'ordinateur, liste
//! complète). Le poste le transmet au serveur de l'entreprise, qui le vérifie
//! puis l'applique ; un poste ne peut ni le forger ni le modifier. Un bulletin
//! n'est jamais pris pour une licence ou une attestation : son type et son
//! préfixe diffèrent.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use kadenz_domaine::message::MessageCompose;
use serde_json::{Value, json};

use crate::entreprise::date_lisible;
use crate::identifiant_cle;

const PREFIXE: &str = "KADENZ-B-";
const VERSION: u64 = 1;
const TYPE: &str = "bulletin";

/// Nombre maximal d'empreintes ou d'ordinateurs dans une liste : autant que de
/// postes dans une licence.
pub const MAX_LISTE: usize = 1000;

/// Modèles d'erreur de ce module, pour vérifier leurs traductions.
pub const MODELES_ERREUR: &[&str] = &[
    "bulletin de licence illisible",
    "bulletin de licence émis pour une autre entreprise",
    "bulletin de licence signé par une clé inconnue",
];

/// Bulletin émis par le portail pour une entreprise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bulletin {
    /// Identifiant de l'entreprise (UUID en minuscules).
    pub entreprise: String,
    /// Date et heure d'émission, UTC, `AAAA-MM-JJ HH:MM:SS` : un serveur
    /// refuse un bulletin qui n'est pas plus récent que le dernier appliqué.
    pub emis_le: String,
    /// Licence de l'entreprise révoquée.
    pub revoquee: bool,
    /// Empreintes de certificat des postes à retirer.
    pub retires: Vec<String>,
    /// Identifiants d'ordinateur bloqués (liste complète à jour).
    pub bloques: Vec<String>,
}

fn erreur(modele: &'static str) -> MessageCompose {
    MessageCompose::new(modele, &[])
}

/// Vrai pour `AAAA-MM-JJ HH:MM:SS`, date valide.
pub fn horodatage_lisible(texte: &str) -> bool {
    let o = texte.as_bytes();
    o.len() == 19
        && texte.is_ascii()
        && date_lisible(&texte[..10])
        && o[10] == b' '
        && o[13] == b':'
        && o[16] == b':'
        && [11, 12, 14, 15, 17, 18]
            .iter()
            .all(|&i| o[i].is_ascii_digit())
        && &texte[11..13] < "24"
        && &texte[14..16] < "60"
        && &texte[17..19] < "60"
}

/// Élément de liste acceptable : court, sans espace ni caractère de contrôle.
fn element_valide(e: &str) -> bool {
    !e.is_empty() && e.len() <= 128 && e.bytes().all(|b| b.is_ascii_graphic())
}

fn liste_valide(liste: &[String]) -> bool {
    liste.len() <= MAX_LISTE && liste.iter().all(|e| element_valide(e))
}

/// Signe le bulletin avec la clé du portail.
pub fn emettre(bulletin: &Bulletin, cle_privee: &SigningKey) -> Result<String, MessageCompose> {
    if bulletin.entreprise.trim().is_empty()
        || !horodatage_lisible(&bulletin.emis_le)
        || !liste_valide(&bulletin.retires)
        || !liste_valide(&bulletin.bloques)
    {
        return Err(erreur(MODELES_ERREUR[0]));
    }
    let donnees = json!({
        "type": TYPE,
        "version": VERSION,
        "entreprise": bulletin.entreprise.trim().to_ascii_lowercase(),
        "emis_le": bulletin.emis_le,
        "revoquee": bulletin.revoquee,
        "retires": bulletin.retires,
        "bloques": bulletin.bloques,
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

/// Vérifie un bulletin pour cette entreprise, parmi les clés du portail
/// données. La comparaison avec le dernier bulletin appliqué revient au
/// serveur.
pub fn verifier(
    texte: &str,
    entreprise: &str,
    cles_portail: &[VerifyingKey],
) -> Result<Bulletin, MessageCompose> {
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
    let liste = |nom: &str| -> Option<Vec<String>> {
        let l: Vec<String> = v
            .get(nom)?
            .as_array()?
            .iter()
            .map(|e| e.as_str().map(str::to_string))
            .collect::<Option<_>>()?;
        liste_valide(&l).then_some(l)
    };
    let bulletin = Bulletin {
        entreprise: texte_de("entreprise").ok_or_else(illisible)?,
        emis_le: texte_de("emis_le")
            .filter(|d| horodatage_lisible(d))
            .ok_or_else(illisible)?,
        revoquee: v
            .get("revoquee")
            .and_then(Value::as_bool)
            .ok_or_else(illisible)?,
        retires: liste("retires").ok_or_else(illisible)?,
        bloques: liste("bloques").ok_or_else(illisible)?,
    };
    if !bulletin.entreprise.eq_ignore_ascii_case(entreprise.trim()) {
        return Err(erreur(MODELES_ERREUR[1]));
    }
    Ok(bulletin)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENTREPRISE: &str = "3f2a6c1e-9b7d-4e21-8a55-0c4d2b1e7f90";

    fn exemple() -> Bulletin {
        Bulletin {
            entreprise: ENTREPRISE.into(),
            emis_le: "2026-11-27 06:10:42".into(),
            revoquee: true,
            retires: vec!["ab12cd34".into()],
            bloques: vec!["ABCD-EFGH-JKMN-PQRS".into()],
        }
    }

    fn modele(r: Result<Bulletin, MessageCompose>) -> String {
        r.unwrap_err().modele.to_string()
    }

    #[test]
    fn aller_retour_et_refus() {
        let cle = SigningKey::from_bytes(&[5u8; 32]);
        let publique = [cle.verifying_key()];
        let texte = emettre(&exemple(), &cle).unwrap();
        assert!(texte.starts_with(PREFIXE));
        // L'identifiant se compare sans la casse ; un retour à la ligne collé
        // au milieu ne gêne pas.
        let coupe = format!("{}\r\n{}", &texte[..40], &texte[40..]);
        let lu = verifier(&coupe, &ENTREPRISE.to_uppercase(), &publique).unwrap();
        assert_eq!(lu, exemple());

        // Autre entreprise, autre clé.
        assert_eq!(
            modele(verifier(
                &texte,
                "00000000-0000-4000-8000-000000000000",
                &publique
            )),
            MODELES_ERREUR[1]
        );
        let autre = [SigningKey::from_bytes(&[6u8; 32]).verifying_key()];
        assert_eq!(
            modele(verifier(&texte, ENTREPRISE, &autre)),
            MODELES_ERREUR[2]
        );

        // Révocation retirée après la signature : refusée.
        let (donnees, signature) = texte[PREFIXE.len()..].split_once('.').unwrap();
        let mut json: Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(donnees).unwrap()).unwrap();
        json["revoquee"] = json!(false);
        let falsifie = format!(
            "{PREFIXE}{}.{signature}",
            URL_SAFE_NO_PAD.encode(json.to_string())
        );
        assert_eq!(
            modele(verifier(&falsifie, ENTREPRISE, &publique)),
            MODELES_ERREUR[0]
        );

        // Une licence ou une attestation d'essai n'est pas un bulletin.
        assert!(verifier("KADENZ-E-abc.def", ENTREPRISE, &publique).is_err());
        assert!(verifier("KADENZ-T-abc.def", ENTREPRISE, &publique).is_err());
    }

    #[test]
    fn emission_refusee_si_mal_formee() {
        let cle = SigningKey::from_bytes(&[5u8; 32]);
        let mut b = exemple();
        b.emis_le = "2026-11-27".into();
        assert!(emettre(&b, &cle).is_err());
        let mut b = exemple();
        b.retires = vec!["avec espace".into()];
        assert!(emettre(&b, &cle).is_err());
        let mut b = exemple();
        b.bloques = vec!["X".into(); MAX_LISTE + 1];
        assert!(emettre(&b, &cle).is_err());
        let mut b = exemple();
        b.entreprise = " ".into();
        assert!(emettre(&b, &cle).is_err());
        // Listes vides, licence non révoquée : bulletin ordinaire.
        let b = Bulletin {
            revoquee: false,
            retires: Vec::new(),
            bloques: Vec::new(),
            ..exemple()
        };
        let texte = emettre(&b, &cle).unwrap();
        assert_eq!(
            verifier(&texte, ENTREPRISE, &[cle.verifying_key()]).unwrap(),
            b
        );
    }

    #[test]
    fn horodatages() {
        assert!(horodatage_lisible("2026-11-27 23:59:59"));
        assert!(!horodatage_lisible("2026-11-27 24:00:00"));
        assert!(!horodatage_lisible("2026-13-01 10:00:00"));
        assert!(!horodatage_lisible("2026-11-27T10:00:00"));
        assert!(!horodatage_lisible("2026-11-27 10:00"));
    }
}
