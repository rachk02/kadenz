//! Réglages d'impression (cahier des charges, section 12, jalon J8b) : gabarit,
//! marges, zones d'en-tête et de pied de page.
//!
//! Les zones sont stockées en structure (lignes, passages, styles, variables),
//! jamais dans le markup saisi ; l'éditeur reçoit un markup reconstruit. Les
//! réglages se modifient au droit « Gérer les paramètres » et sont figés dans
//! l'instantané d'impression à la validation d'un document.

use kadenz_domaine::document::TypeDocument;
use kadenz_domaine::mise_en_page::{
    BasDeDocument, Contenu, Gabarit, Ligne, LigneTotal, Marges, MiseEnPage, MiseEnPageAnalysee,
    Passage, StylePied, Variable, Zone,
};
use kadenz_domaine::role::Droit;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use crate::Resultat;
use crate::audit::{self, Entree};
use crate::comptes::Utilisateur;
use crate::droits::exiger;

const CLE: &str = "impression.mise_en_page";

fn zone_en_json(z: &Zone) -> Value {
    let lignes: Vec<Value> = z
        .lignes
        .iter()
        .map(|l| match l {
            Ligne::Logo { logo, centree } => json!({ "logo": logo.nom(), "centree": centree }),
            Ligne::Texte { centree, passages } => json!({
                "centree": centree,
                "passages": passages.iter().map(|p| {
                    let mut v = match &p.contenu {
                        Contenu::Texte(t) => json!({ "texte": t }),
                        Contenu::Variable(var) => json!({ "variable": var.nom() }),
                    };
                    if p.gras { v["gras"] = json!(true); }
                    if p.italique { v["italique"] = json!(true); }
                    v
                }).collect::<Vec<_>>(),
            }),
        })
        .collect();
    Value::Array(lignes)
}

/// Lecture tolérante : un élément illisible est sauté plutôt que d'empêcher
/// l'impression.
fn zone_depuis_json(v: &Value) -> Zone {
    let booleen = |o: &Value, cle: &str| o.get(cle).and_then(Value::as_bool).unwrap_or(false);
    let lignes = v
        .as_array()
        .map(|lignes| {
            lignes
                .iter()
                .filter_map(|l| {
                    let centree = booleen(l, "centree");
                    if let Some(nom) = l.get("logo").and_then(Value::as_str) {
                        let logo = Variable::depuis_nom(nom).filter(|v| v.est_un_logo())?;
                        return Some(Ligne::Logo { logo, centree });
                    }
                    let passages = l
                        .get("passages")
                        .and_then(Value::as_array)
                        .map(|ps| {
                            ps.iter()
                                .filter_map(|p| {
                                    let contenu =
                                        if let Some(t) = p.get("texte").and_then(Value::as_str) {
                                            Contenu::Texte(t.to_owned())
                                        } else {
                                            let nom = p.get("variable").and_then(Value::as_str)?;
                                            Contenu::Variable(
                                                Variable::depuis_nom(nom)
                                                    .filter(|v| !v.est_un_logo())?,
                                            )
                                        };
                                    Some(Passage {
                                        contenu,
                                        gras: booleen(p, "gras"),
                                        italique: booleen(p, "italique"),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    Some(Ligne::Texte { centree, passages })
                })
                .collect()
        })
        .unwrap_or_default();
    Zone { lignes }
}

/// Réglages analysés en JSON (paramètre et instantané d'impression).
pub fn en_json(m: &MiseEnPageAnalysee) -> Value {
    json!({
        "gabarit": m.gabarit.code(),
        "marges": {
            "haut": m.marges.haut,
            "bas": m.marges.bas,
            "gauche": m.marges.gauche,
            "droite": m.marges.droite,
        },
        "entete": zone_en_json(&m.entete),
        "pied": zone_en_json(&m.pied),
        "style_pied": m.style_pied.code(),
    })
}

/// Réglages depuis leur JSON ; ce qui manque reprend la valeur par défaut.
pub fn depuis_json(v: &Value) -> MiseEnPageAnalysee {
    let defaut = defaut_analyse();
    let marge = |cle: &str, d: u8| {
        v.get("marges")
            .and_then(|m| m.get(cle))
            .and_then(Value::as_u64)
            .and_then(|n| u8::try_from(n).ok())
            .filter(|n| *n <= Marges::MAX)
            .unwrap_or(d)
    };
    MiseEnPageAnalysee {
        gabarit: v
            .get("gabarit")
            .and_then(Value::as_str)
            .and_then(Gabarit::depuis_code)
            .unwrap_or(defaut.gabarit),
        marges: Marges {
            haut: marge("haut", defaut.marges.haut),
            bas: marge("bas", defaut.marges.bas),
            gauche: marge("gauche", defaut.marges.gauche),
            droite: marge("droite", defaut.marges.droite),
        },
        entete: v
            .get("entete")
            .map(zone_depuis_json)
            .unwrap_or(defaut.entete),
        pied: v.get("pied").map(zone_depuis_json).unwrap_or(defaut.pied),
        style_pied: v
            .get("style_pied")
            .and_then(Value::as_str)
            .and_then(StylePied::depuis_code)
            .unwrap_or(defaut.style_pied),
    }
}

/// Bas d'un document en JSON (paramètre et instantané d'impression).
pub fn bas_en_json(b: &BasDeDocument) -> Value {
    json!({
        "totaux": b.totaux.iter().map(|l| l.code()).collect::<Vec<_>>(),
        "masquer_zero": b.masquer_zero,
        "montant_en_lettres": b.montant_en_lettres,
        "visa_client": b.visa_client,
        "visa_fournisseur": b.visa_fournisseur,
        "date_reception": b.date_reception,
        "date_livraison": b.date_livraison,
        "signataire": b.signataire,
    })
}

/// Bas d'un document depuis son JSON ; ce qui manque reprend le réglage
/// d'origine du type.
pub fn bas_depuis_json(v: &Value, t: TypeDocument) -> BasDeDocument {
    let d = BasDeDocument::par_defaut(t);
    let b = |cle: &str, defaut: bool| v.get(cle).and_then(Value::as_bool).unwrap_or(defaut);
    let mut bas = BasDeDocument {
        totaux: v
            .get("totaux")
            .and_then(Value::as_array)
            .map(|ls| {
                ls.iter()
                    .filter_map(|l| l.as_str().and_then(LigneTotal::depuis_code))
                    .collect()
            })
            .unwrap_or(d.totaux),
        masquer_zero: b("masquer_zero", d.masquer_zero),
        montant_en_lettres: b("montant_en_lettres", d.montant_en_lettres),
        visa_client: b("visa_client", d.visa_client),
        visa_fournisseur: b("visa_fournisseur", d.visa_fournisseur),
        date_reception: b("date_reception", d.date_reception),
        date_livraison: b("date_livraison", d.date_livraison),
        signataire: b("signataire", d.signataire),
    };
    bas.normaliser();
    bas
}

fn cle_bas(t: TypeDocument) -> String {
    format!("impression.bas.{}", t.code())
}

/// Bas du moment pour un type de document.
pub fn lire_bas(conn: &Connection, t: TypeDocument) -> Resultat<BasDeDocument> {
    let texte: Option<String> = conn
        .query_row(
            "SELECT valeur FROM parametre_systeme WHERE cle = ?1",
            [cle_bas(t)],
            |r| r.get(0),
        )
        .optional()?;
    Ok(texte
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .map(|v| bas_depuis_json(&v, t))
        .unwrap_or_else(|| BasDeDocument::par_defaut(t)))
}

/// Enregistre le bas d'un type de document.
pub fn enregistrer_bas(
    conn: &Connection,
    acteur: &Utilisateur,
    t: TypeDocument,
    b: &BasDeDocument,
) -> Resultat<()> {
    exiger(conn, acteur, Droit::GererParametres)?;
    let mut b = b.clone();
    b.normaliser();
    let valeur = bas_en_json(&b);
    conn.execute(
        "INSERT INTO parametre_systeme (cle, valeur) VALUES (?1, ?2)
         ON CONFLICT (cle) DO UPDATE SET valeur = excluded.valeur",
        params![cle_bas(t), valeur.to_string()],
    )?;
    audit::journaliser(
        conn,
        Entree {
            entite: "parametres",
            entite_id: 0,
            action: "modification_bas_de_document",
            ancienne_valeur: None,
            nouvelle_valeur: Some(json!({ "type": t.code(), "bas": valeur })),
            utilisateur_id: Some(acteur.id),
        },
    )
}

/// Réglages par défaut : la mise en page d'origine.
pub fn defaut_analyse() -> MiseEnPageAnalysee {
    MiseEnPageAnalysee::default()
}

/// Réglages analysés du moment.
pub fn lire_analysee(conn: &Connection) -> Resultat<MiseEnPageAnalysee> {
    let texte: Option<String> = conn
        .query_row(
            "SELECT valeur FROM parametre_systeme WHERE cle = ?1",
            [CLE],
            |r| r.get(0),
        )
        .optional()?;
    Ok(texte
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .map(|v| depuis_json(&v))
        .unwrap_or_else(defaut_analyse))
}

/// Réglages du moment pour l'éditeur, zones en markup.
pub fn lire(conn: &Connection) -> Resultat<MiseEnPage> {
    Ok(pour_editeur(&lire_analysee(conn)?))
}

/// Réglages analysés remis en markup, pour l'éditeur ou pour `enregistrer`.
pub fn pour_editeur(m: &MiseEnPageAnalysee) -> MiseEnPage {
    MiseEnPage {
        gabarit: m.gabarit,
        marges: m.marges,
        entete: m.entete.en_markup(),
        pied: m.pied.en_markup(),
        style_pied: m.style_pied,
    }
}

/// Enregistre les réglages après analyse du markup ; refus traduisible sinon.
pub fn enregistrer(conn: &Connection, acteur: &Utilisateur, m: &MiseEnPage) -> Resultat<()> {
    exiger(conn, acteur, Droit::GererParametres)?;
    let analysee = m.analyser()?;
    let valeur = en_json(&analysee);
    conn.execute(
        "INSERT INTO parametre_systeme (cle, valeur) VALUES (?1, ?2)
         ON CONFLICT (cle) DO UPDATE SET valeur = excluded.valeur",
        params![CLE, valeur.to_string()],
    )?;
    audit::journaliser(
        conn,
        Entree {
            entite: "parametres",
            entite_id: 0,
            action: "modification_mise_en_page",
            ancienne_valeur: None,
            nouvelle_valeur: Some(valeur),
            utilisateur_id: Some(acteur.id),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::droits::tests::compte;
    use crate::ouvrir_en_memoire;
    use kadenz_domaine::role::Role;

    #[test]
    fn lecture_par_defaut_enregistrement_et_droit() {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        let vente = compte(&conn, "vente", Role::Commercial);
        assert_eq!(lire(&conn).unwrap(), MiseEnPage::default());
        let m = MiseEnPage {
            gabarit: Gabarit::PreImprime,
            marges: Marges {
                haut: 30,
                bas: 20,
                gauche: 15,
                droite: 15,
            },
            entete: "-> {logo}\n-> ***{nom}*** et \\*\n\nIFU : {ifu}".into(),
            pied: String::new(),
            style_pied: StylePied::Accentue,
        };
        assert!(
            enregistrer(&conn, &vente, &m).is_err(),
            "droit Gérer les paramètres"
        );
        enregistrer(&conn, &admin, &m).unwrap();
        let relu = lire(&conn).unwrap();
        assert_eq!(relu.gabarit, Gabarit::PreImprime);
        assert_eq!(relu.marges, m.marges);
        assert_eq!(relu.pied, "");
        assert_eq!(relu.style_pied, StylePied::Accentue);
        assert_eq!(
            relu.entete,
            "-> {logo}\n-> ***{nom}*** et \\*\n\nIFU : {ifu}"
        );
        // Stocké en structure, pas en markup.
        let brut: String = conn
            .query_row(
                "SELECT valeur FROM parametre_systeme WHERE cle = ?1",
                [CLE],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!brut.contains("**"));
        let faux = MiseEnPage {
            entete: "{nom".into(),
            ..MiseEnPage::default()
        };
        assert_eq!(
            enregistrer(&conn, &admin, &faux).unwrap_err().to_string(),
            "en-tête, ligne 1 : accolade « { » non fermée"
        );
    }

    #[test]
    fn bas_par_type() {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        let vente = compte(&conn, "vente", Role::Commercial);
        let t = TypeDocument::Livraison;
        assert_eq!(lire_bas(&conn, t).unwrap(), BasDeDocument::par_defaut(t));
        let b = BasDeDocument {
            totaux: vec![LigneTotal::Ttc, LigneTotal::Ht],
            visa_fournisseur: false,
            ..BasDeDocument::par_defaut(t)
        };
        assert!(enregistrer_bas(&conn, &vente, t, &b).is_err());
        enregistrer_bas(&conn, &admin, t, &b).unwrap();
        let relu = lire_bas(&conn, t).unwrap();
        assert_eq!(relu.totaux, [LigneTotal::Ht, LigneTotal::Ttc]);
        assert!(!relu.visa_fournisseur && relu.visa_client);
        let f = TypeDocument::Facture;
        assert_eq!(lire_bas(&conn, f).unwrap(), BasDeDocument::par_defaut(f));
    }

    #[test]
    fn json_tolerant() {
        let m = depuis_json(&json!({ "gabarit": "inconnu", "marges": { "haut": 99 } }));
        assert_eq!(m, defaut_analyse());
        let z = zone_depuis_json(
            &json!([{ "logo": "nom" }, { "passages": [{ "variable": "x" }, { "texte": "ok" }] }]),
        );
        assert_eq!(z.lignes.len(), 1);
    }
}
