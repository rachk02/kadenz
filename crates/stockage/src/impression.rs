//! Ce qui s'imprime autour d'un document : données de l'entreprise, logos, textes
//! et couleur (section 12).
//!
//! À la validation, ces éléments sont figés dans un instantané référencé par le
//! document : une réimpression reste identique à l'original, même si la fiche
//! entreprise ou les réglages changent ensuite. Un brouillon s'imprime (en aperçu)
//! avec les valeurs du moment.

use kadenz_domaine::document::TypeDocument;
use kadenz_domaine::lettres::Orthographe;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use crate::entreprise::{self, Entreprise, Logo};
use crate::parametres::{self, TextesDocument, couleur_depuis_texte, couleur_en_texte};
use crate::{Erreur, Resultat};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Impression {
    pub entreprise: Entreprise,
    pub logo: Option<Vec<u8>>,
    pub logo2: Option<Vec<u8>>,
    pub textes: TextesDocument,
    pub couleur: [u8; 3],
    /// Orthographe du montant en lettres.
    pub orthographe: Orthographe,
}

/// Valeurs du moment pour un type de document (aperçu d'un brouillon).
pub fn courante(conn: &Connection, t: TypeDocument) -> Resultat<Impression> {
    let p = parametres::lire(conn)?;
    Ok(Impression {
        entreprise: entreprise::lire(conn)?.unwrap_or_default(),
        logo: entreprise::lire_logo(conn, Logo::Principal)?,
        logo2: entreprise::lire_logo(conn, Logo::Secondaire)?,
        textes: p.textes(t).clone(),
        couleur: p.couleur_documents,
        orthographe: p.orthographe,
    })
}

fn en_json(i: &Impression) -> Value {
    let e = &i.entreprise;
    json!({
        "nom": e.nom,
        "adresse": e.adresse,
        "telephone": e.telephone,
        "email": e.email,
        "site_web": e.site_web,
        "localisation": e.localisation,
        "regime_imposition": e.regime_imposition,
        "rccm": e.rccm,
        "ifu": e.ifu,
        "banque1": e.banque1,
        "banque2": e.banque2,
        "responsable_facturation": e.responsable_facturation,
        "responsable_comptabilite": e.responsable_comptabilite,
        "titre": i.textes.titre,
        "phrase_somme": i.textes.phrase_somme,
        "signataire": i.textes.signataire,
        "couleur": couleur_en_texte(i.couleur),
        "orthographe": i.orthographe.code(),
    })
}

fn depuis_json(v: &Value, logo: Option<Vec<u8>>, logo2: Option<Vec<u8>>) -> Impression {
    let t = |nom: &str| {
        v.get(nom)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    Impression {
        entreprise: Entreprise {
            nom: t("nom"),
            adresse: t("adresse"),
            telephone: t("telephone"),
            email: t("email"),
            site_web: t("site_web"),
            localisation: t("localisation"),
            regime_imposition: t("regime_imposition"),
            rccm: t("rccm"),
            ifu: t("ifu"),
            banque1: t("banque1"),
            banque2: t("banque2"),
            responsable_facturation: t("responsable_facturation"),
            responsable_comptabilite: t("responsable_comptabilite"),
        },
        logo,
        logo2,
        textes: TextesDocument {
            titre: t("titre"),
            phrase_somme: t("phrase_somme"),
            signataire: t("signataire"),
        },
        couleur: couleur_depuis_texte(&t("couleur")).unwrap_or(parametres::COULEUR_PAR_DEFAUT),
        orthographe: Orthographe::depuis_code(&t("orthographe")).unwrap_or_default(),
    }
}

/// Fige l'impression du moment pour un document, dans la transaction de
/// validation. Réutilise l'instantané identique le plus récent s'il existe.
pub(crate) fn figer(conn: &Connection, document_id: i64, t: TypeDocument) -> Resultat<()> {
    let i = courante(conn, t)?;
    let donnees = en_json(&i).to_string();
    let existant: Option<i64> = conn
        .query_row(
            "SELECT id FROM instantane_entreprise
             WHERE donnees = ?1 AND logo IS ?2 AND logo2 IS ?3
             ORDER BY id DESC LIMIT 1",
            params![donnees, i.logo, i.logo2],
            |r| r.get(0),
        )
        .optional()?;
    let id = match existant {
        Some(id) => id,
        None => {
            conn.execute(
                "INSERT INTO instantane_entreprise (donnees, logo, logo2) VALUES (?1, ?2, ?3)",
                params![donnees, i.logo, i.logo2],
            )?;
            conn.last_insert_rowid()
        }
    };
    conn.execute(
        "UPDATE document SET instantane_entreprise_id = ?1 WHERE id = ?2",
        params![id, document_id],
    )?;
    Ok(())
}

/// Impression d'un document : l'instantané figé s'il a été validé, sinon les
/// valeurs du moment.
pub fn pour_document(conn: &Connection, document_id: i64) -> Resultat<Impression> {
    let (code, instantane): (String, Option<i64>) = conn
        .query_row(
            "SELECT type, instantane_entreprise_id FROM document WHERE id = ?1",
            [document_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| Erreur::Validation("document introuvable".into()))?;
    let t = TypeDocument::depuis_code(&code)
        .ok_or_else(|| Erreur::Validation(format!("type de document inconnu : {code}")))?;
    let Some(id) = instantane else {
        return courante(conn, t);
    };
    let (donnees, logo, logo2): (String, Option<Vec<u8>>, Option<Vec<u8>>) = conn.query_row(
        "SELECT donnees, logo, logo2 FROM instantane_entreprise WHERE id = ?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let v: Value = serde_json::from_str(&donnees).unwrap_or(Value::Null);
    Ok(depuis_json(&v, logo, logo2))
}
