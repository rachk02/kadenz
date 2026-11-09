//! Paramètres système modifiables par l'Admin (cahier des charges, section 5.4).
//!
//! Chaque paramètre est une ligne clé/valeur, la valeur étant du JSON. Une clé
//! absente retombe sur la valeur par défaut du cahier des charges.

use kadenz_domaine::calcul::Pourcentage;
use kadenz_domaine::lettres::Orthographe;
use kadenz_domaine::numerotation::{Numerotation, RemiseAZero};
use kadenz_domaine::role::Droit;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use crate::audit::{self, Entree};
use crate::comptes::Utilisateur;
use crate::droits::exiger;
use crate::{Erreur, Resultat};

const CLE_TAUX_TVA: &str = "taux.tva";
const CLE_TAUX_REMISE: &str = "taux.remise";
const CLE_TAUX_RETENUE: &str = "taux.retenue";
const CLE_MODES_PAIEMENT: &str = "paiement.modes";
const CLE_ORTHOGRAPHE: &str = "lettres.orthographe";
const CLE_VALIDITE_DEVIS: &str = "delais.validite_devis_jours";
const CLE_DELAI_PAIEMENT: &str = "delais.paiement_jours";
const CLE_NUMEROTATION: &str = "numerotation";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parametres {
    pub taux_tva: Vec<Pourcentage>,
    pub taux_remise: Vec<Pourcentage>,
    pub taux_retenue: Vec<Pourcentage>,
    pub modes_paiement: Vec<String>,
    pub orthographe: Orthographe,
    /// Durée de validité des devis, en jours. Pas de valeur par défaut dans le cahier.
    pub validite_devis_jours: Option<u32>,
    /// Délai de paiement des factures, en jours (date d'échéance). Pas de valeur par défaut.
    pub delai_paiement_jours: Option<u32>,
    /// Préfixes, remise à zéro et nombre de chiffres des numéros (section 5.4).
    pub numerotation: Numerotation,
}

impl Default for Parametres {
    /// Valeurs par défaut du cahier des charges et de l'outil Excel d'origine.
    fn default() -> Self {
        Parametres {
            taux_tva: vec![0, 1800],
            taux_remise: vec![0, 500, 1000, 1500],
            taux_retenue: vec![0, 500, 1000, 1500],
            modes_paiement: [
                "Espèce",
                "Chèque",
                "Virement bancaire",
                "Carte bancaire",
                "Moov money",
                "Orange money",
            ]
            .map(String::from)
            .to_vec(),
            orthographe: Orthographe::default(),
            validite_devis_jours: None,
            delai_paiement_jours: None,
            numerotation: Numerotation::default(),
        }
    }
}

impl Parametres {
    fn valider(&self) -> Resultat<()> {
        let verifier_taux = |nom: &str, liste: &[Pourcentage]| {
            if liste.is_empty() || liste.iter().any(|p| !(0..=10_000).contains(p)) {
                Err(Erreur::Validation(format!(
                    "la liste des taux de {nom} doit contenir des valeurs entre 0 et 100 %"
                )))
            } else {
                Ok(())
            }
        };
        verifier_taux("TVA", &self.taux_tva)?;
        verifier_taux("remise", &self.taux_remise)?;
        verifier_taux("retenue", &self.taux_retenue)?;
        if self.modes_paiement.iter().all(|m| m.trim().is_empty()) {
            return Err(Erreur::Validation(
                "au moins un mode de paiement est nécessaire".into(),
            ));
        }
        self.numerotation
            .nettoyee()
            .map_err(|e| Erreur::Validation(e.to_string()))?;
        Ok(())
    }
}

fn numerotation_en_json(n: &Numerotation) -> Value {
    json!({
        "prefixes": n.prefixes,
        "remise": n.remise.code(),
        "chiffres": n.chiffres,
    })
}

/// Valeur illisible ou incomplète : retour au réglage par défaut, comme pour une clé absente.
fn numerotation_depuis_json(valeur: Option<Value>) -> Option<Numerotation> {
    let v = valeur?;
    let prefixes: Vec<String> = liste_textes(v.get("prefixes").cloned())?;
    let prefixes: [String; 6] = prefixes.try_into().ok()?;
    let remise = RemiseAZero::depuis_code(v.get("remise")?.as_str()?)?;
    let chiffres = u8::try_from(v.get("chiffres")?.as_u64()?).ok()?;
    Numerotation {
        prefixes,
        remise,
        chiffres,
    }
    .nettoyee()
    .ok()
}

fn lire_valeur(conn: &Connection, cle: &str) -> Resultat<Option<Value>> {
    let texte: Option<String> = conn
        .query_row(
            "SELECT valeur FROM parametre_systeme WHERE cle = ?1",
            [cle],
            |r| r.get(0),
        )
        .optional()?;
    Ok(texte.and_then(|t| serde_json::from_str(&t).ok()))
}

fn ecrire_valeur(conn: &Connection, cle: &str, valeur: &Value) -> Resultat<()> {
    conn.execute(
        "INSERT INTO parametre_systeme (cle, valeur) VALUES (?1, ?2)
         ON CONFLICT (cle) DO UPDATE SET valeur = excluded.valeur",
        params![cle, valeur.to_string()],
    )?;
    Ok(())
}

fn liste_entiers(valeur: Option<Value>) -> Option<Vec<i64>> {
    valeur?.as_array()?.iter().map(Value::as_i64).collect()
}

fn jours(valeur: Option<Value>) -> Option<u32> {
    valeur?.as_u64().and_then(|j| u32::try_from(j).ok())
}

fn liste_textes(valeur: Option<Value>) -> Option<Vec<String>> {
    valeur?
        .as_array()?
        .iter()
        .map(|v| v.as_str().map(String::from))
        .collect()
}

pub fn lire(conn: &Connection) -> Resultat<Parametres> {
    let defaut = Parametres::default();
    Ok(Parametres {
        taux_tva: liste_entiers(lire_valeur(conn, CLE_TAUX_TVA)?).unwrap_or(defaut.taux_tva),
        taux_remise: liste_entiers(lire_valeur(conn, CLE_TAUX_REMISE)?)
            .unwrap_or(defaut.taux_remise),
        taux_retenue: liste_entiers(lire_valeur(conn, CLE_TAUX_RETENUE)?)
            .unwrap_or(defaut.taux_retenue),
        modes_paiement: liste_textes(lire_valeur(conn, CLE_MODES_PAIEMENT)?)
            .unwrap_or(defaut.modes_paiement),
        orthographe: lire_valeur(conn, CLE_ORTHOGRAPHE)?
            .and_then(|v| v.as_str().and_then(Orthographe::depuis_code))
            .unwrap_or(defaut.orthographe),
        validite_devis_jours: jours(lire_valeur(conn, CLE_VALIDITE_DEVIS)?),
        delai_paiement_jours: jours(lire_valeur(conn, CLE_DELAI_PAIEMENT)?),
        numerotation: numerotation_depuis_json(lire_valeur(conn, CLE_NUMEROTATION)?)
            .unwrap_or(defaut.numerotation),
    })
}

/// Réservé au droit « Gérer la fiche entreprise et les paramètres ».
pub fn enregistrer(conn: &Connection, acteur: &Utilisateur, p: &Parametres) -> Resultat<()> {
    exiger(conn, acteur, Droit::GererParametres)?;
    enregistrer_sans_controle(conn, p, Some(acteur.id))
}

/// Écriture sans contrôle de droit : réservée à l'installation initiale.
pub(crate) fn enregistrer_sans_controle(
    conn: &Connection,
    p: &Parametres,
    auteur: Option<i64>,
) -> Resultat<()> {
    p.valider()?;
    let numerotation = numerotation_en_json(
        &p.numerotation
            .nettoyee()
            .map_err(|e| Erreur::Validation(e.to_string()))?,
    );
    let modes: Vec<&str> = p
        .modes_paiement
        .iter()
        .map(|m| m.trim())
        .filter(|m| !m.is_empty())
        .collect();
    ecrire_valeur(conn, CLE_TAUX_TVA, &json!(p.taux_tva))?;
    ecrire_valeur(conn, CLE_TAUX_REMISE, &json!(p.taux_remise))?;
    ecrire_valeur(conn, CLE_TAUX_RETENUE, &json!(p.taux_retenue))?;
    ecrire_valeur(conn, CLE_MODES_PAIEMENT, &json!(modes))?;
    ecrire_valeur(conn, CLE_ORTHOGRAPHE, &json!(p.orthographe.code()))?;
    ecrire_valeur(conn, CLE_VALIDITE_DEVIS, &json!(p.validite_devis_jours))?;
    ecrire_valeur(conn, CLE_DELAI_PAIEMENT, &json!(p.delai_paiement_jours))?;
    ecrire_valeur(conn, CLE_NUMEROTATION, &numerotation)?;
    audit::journaliser(
        conn,
        Entree {
            entite: "parametres",
            entite_id: 0,
            action: "modification",
            ancienne_valeur: None,
            nouvelle_valeur: Some(json!({
                "taux_tva": p.taux_tva,
                "taux_remise": p.taux_remise,
                "taux_retenue": p.taux_retenue,
                "modes_paiement": modes,
                "orthographe": p.orthographe.code(),
                "validite_devis_jours": p.validite_devis_jours,
                "delai_paiement_jours": p.delai_paiement_jours,
                "numerotation": numerotation,
            })),
            utilisateur_id: auteur,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ouvrir_en_memoire;

    #[test]
    fn valeurs_par_defaut_sans_enregistrement() {
        let conn = ouvrir_en_memoire().unwrap();
        assert_eq!(lire(&conn).unwrap(), Parametres::default());
    }

    #[test]
    fn aller_retour() {
        let conn = ouvrir_en_memoire().unwrap();
        let p = Parametres {
            taux_tva: vec![0, 250, 1800],
            modes_paiement: vec!["Espèce".into(), "  ".into(), "Chèque".into()],
            orthographe: Orthographe::Traditionnelle,
            ..Default::default()
        };
        enregistrer_sans_controle(&conn, &p, None).unwrap();
        let lu = lire(&conn).unwrap();
        assert_eq!(lu.taux_tva, vec![0, 250, 1800]);
        assert_eq!(lu.modes_paiement, vec!["Espèce", "Chèque"]);
        assert_eq!(lu.orthographe, Orthographe::Traditionnelle);
    }

    #[test]
    fn numerotation_aller_retour() {
        let conn = ouvrir_en_memoire().unwrap();
        let mut p = Parametres::default();
        p.numerotation.remise = RemiseAZero::Annuelle;
        p.numerotation.chiffres = 5;
        p.numerotation.prefixes[4] = " fa ".into();
        enregistrer_sans_controle(&conn, &p, None).unwrap();
        let n = lire(&conn).unwrap().numerotation;
        assert_eq!(n.remise, RemiseAZero::Annuelle);
        assert_eq!(n.chiffres, 5);
        assert_eq!(n.prefixes[4], "FA");
        p.numerotation.prefixes[4] = "D".into();
        assert!(enregistrer_sans_controle(&conn, &p, None).is_err());
    }

    #[test]
    fn listes_vides_refusees() {
        let conn = ouvrir_en_memoire().unwrap();
        let sans_tva = Parametres {
            taux_tva: vec![],
            ..Default::default()
        };
        assert!(enregistrer_sans_controle(&conn, &sans_tva, None).is_err());
        let sans_mode = Parametres {
            modes_paiement: vec![" ".into()],
            ..Default::default()
        };
        assert!(enregistrer_sans_controle(&conn, &sans_mode, None).is_err());
    }
}
