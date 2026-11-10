//! Catalogue des produits et services (cahier des charges, section 5.3).
//!
//! La référence est unique sans tenir compte de la casse. Mêmes droits que les
//! tiers : « ajouter » pour créer et importer, « modifier » pour le reste.

pub use kadenz_domaine::referentiel::{FicheProduit, Sens};
use kadenz_domaine::role::Droit;
use kadenz_domaine::texte;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use crate::audit::{self, Entree};
use crate::comptes::Utilisateur;
use crate::droits::exiger;
use crate::{BilanImport, Erreur, Resultat, Retrait, refus};

fn en_json(f: &FicheProduit) -> Value {
    json!({
        "designation": f.designation,
        "reference": f.reference,
        "prix": f.prix,
        "sens": f.sens.code(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Produit {
    pub id: i64,
    pub fiche: FicheProduit,
    pub actif: bool,
}

/// Critères de la liste des produits.
#[derive(Debug, Clone, Default)]
pub struct FiltreProduits {
    /// Cherché dans la désignation et la référence, sans tenir compte de la casse
    /// ni des accents.
    pub texte: String,
    /// `Vente` retient aussi les produits « les deux », de même pour `Achat` ;
    /// `LesDeux` ne retient que les produits « les deux ».
    pub sens: Option<Sens>,
    pub inclure_inactifs: bool,
}

impl FiltreProduits {
    fn retient(&self, p: &Produit) -> bool {
        if !self.inclure_inactifs && !p.actif {
            return false;
        }
        let sens_ok = match self.sens {
            None => true,
            Some(Sens::Vente) => p.fiche.sens.en_vente(),
            Some(Sens::Achat) => p.fiche.sens.en_achat(),
            Some(Sens::LesDeux) => p.fiche.sens == Sens::LesDeux,
        };
        sens_ok
            && (texte::contient(&p.fiche.designation, &self.texte)
                || texte::contient(&p.fiche.reference, &self.texte))
    }
}

const COLONNES: &str = "id, designation, reference, prix, sens, actif";

fn ligne_vers_produit(r: &rusqlite::Row<'_>) -> rusqlite::Result<Produit> {
    let code: String = r.get(4)?;
    let sens = Sens::depuis_code(&code).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            4,
            rusqlite::types::Type::Text,
            format!("sens inconnu : {code}").into(),
        )
    })?;
    Ok(Produit {
        id: r.get(0)?,
        fiche: FicheProduit {
            designation: r.get(1)?,
            reference: r.get(2)?,
            prix: r.get(3)?,
            sens,
        },
        actif: r.get(5)?,
    })
}

pub fn lire(conn: &Connection, id: i64) -> Resultat<Option<Produit>> {
    Ok(conn
        .query_row(
            &format!("SELECT {COLONNES} FROM produit WHERE id = ?1"),
            [id],
            ligne_vers_produit,
        )
        .optional()?)
}

/// Produit portant cette référence (casse ignorée), actif ou non.
pub fn par_reference(conn: &Connection, reference: &str) -> Resultat<Option<Produit>> {
    Ok(conn
        .query_row(
            &format!("SELECT {COLONNES} FROM produit WHERE reference = ?1"),
            [reference.trim()],
            ligne_vers_produit,
        )
        .optional()?)
}

fn lire_existant(conn: &Connection, id: i64) -> Resultat<Produit> {
    lire(conn, id)?.ok_or_else(|| Erreur::Validation("ce produit n'existe plus".into()))
}

/// Liste triée par désignation.
pub fn lister(conn: &Connection, filtre: &FiltreProduits) -> Resultat<Vec<Produit>> {
    let mut requete = conn.prepare(&format!(
        "SELECT {COLONNES} FROM produit ORDER BY designation COLLATE NOCASE, id"
    ))?;
    let tous = requete
        .query_map([], ligne_vers_produit)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(tous.into_iter().filter(|p| filtre.retient(p)).collect())
}

fn reference_prise(conn: &Connection, reference: &str, sauf: Option<i64>) -> Resultat<()> {
    match par_reference(conn, reference)? {
        Some(p) if Some(p.id) != sauf => Err(refus(
            "la référence « {} » est déjà utilisée par « {} »",
            &[&p.fiche.reference, &p.fiche.designation],
        )),
        _ => Ok(()),
    }
}

fn inserer(conn: &Connection, f: &FicheProduit, auteur: i64) -> Resultat<Produit> {
    conn.execute(
        "INSERT INTO produit (designation, reference, prix, sens) VALUES (?1, ?2, ?3, ?4)",
        params![f.designation, f.reference, f.prix, f.sens.code()],
    )?;
    let id = conn.last_insert_rowid();
    audit::journaliser(
        conn,
        Entree {
            entite: "produit",
            entite_id: id,
            action: "creation",
            ancienne_valeur: None,
            nouvelle_valeur: Some(en_json(f)),
            utilisateur_id: Some(auteur),
        },
    )?;
    Ok(Produit {
        id,
        fiche: f.clone(),
        actif: true,
    })
}

pub fn creer(conn: &Connection, acteur: &Utilisateur, fiche: &FicheProduit) -> Resultat<Produit> {
    exiger(conn, acteur, Droit::AjouterReferentiel)?;
    let f = fiche.nettoyee().map_err(Erreur::Validation)?;
    reference_prise(conn, &f.reference, None)?;
    inserer(conn, &f, acteur.id)
}

pub fn modifier(
    conn: &Connection,
    acteur: &Utilisateur,
    id: i64,
    fiche: &FicheProduit,
) -> Resultat<()> {
    exiger(conn, acteur, Droit::ModifierReferentiel)?;
    let f = fiche.nettoyee().map_err(Erreur::Validation)?;
    let ancien = lire_existant(conn, id)?;
    reference_prise(conn, &f.reference, Some(id))?;
    conn.execute(
        "UPDATE produit SET designation = ?1, reference = ?2, prix = ?3, sens = ?4 WHERE id = ?5",
        params![f.designation, f.reference, f.prix, f.sens.code(), id],
    )?;
    audit::journaliser(
        conn,
        Entree {
            entite: "produit",
            entite_id: id,
            action: "modification",
            ancienne_valeur: Some(en_json(&ancien.fiche)),
            nouvelle_valeur: Some(en_json(&f)),
            utilisateur_id: Some(acteur.id),
        },
    )
}

pub fn definir_actif(
    conn: &Connection,
    acteur: &Utilisateur,
    id: i64,
    actif: bool,
) -> Resultat<()> {
    exiger(conn, acteur, Droit::ModifierReferentiel)?;
    let ancien = lire_existant(conn, id)?;
    if ancien.actif == actif {
        return Ok(());
    }
    conn.execute(
        "UPDATE produit SET actif = ?1 WHERE id = ?2",
        params![actif, id],
    )?;
    audit::journaliser(
        conn,
        Entree {
            entite: "produit",
            entite_id: id,
            action: if actif { "activation" } else { "desactivation" },
            ancienne_valeur: None,
            nouvelle_valeur: None,
            utilisateur_id: Some(acteur.id),
        },
    )
}

fn est_utilise(conn: &Connection, id: i64) -> Resultat<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM ligne_document WHERE produit_id = ?1)",
        [id],
        |r| r.get(0),
    )?)
}

/// Supprime le produit si aucune ligne de document ne le cite, sinon le désactive.
pub fn retirer(conn: &Connection, acteur: &Utilisateur, id: i64) -> Resultat<Retrait> {
    exiger(conn, acteur, Droit::ModifierReferentiel)?;
    let ancien = lire_existant(conn, id)?;
    if est_utilise(conn, id)? {
        definir_actif(conn, acteur, id, false)?;
        return Ok(Retrait::Desactivee);
    }
    conn.execute("DELETE FROM produit WHERE id = ?1", [id])?;
    audit::journaliser(
        conn,
        Entree {
            entite: "produit",
            entite_id: id,
            action: "suppression",
            ancienne_valeur: Some(en_json(&ancien.fiche)),
            nouvelle_valeur: None,
            utilisateur_id: Some(acteur.id),
        },
    )?;
    Ok(Retrait::Supprimee)
}

/// Ajoute les produits en une seule transaction. Une référence déjà présente (en
/// base ou plus haut dans la liste) est ignorée et signalée ; une fiche invalide
/// fait tout annuler.
pub fn importer(
    conn: &mut Connection,
    acteur: &Utilisateur,
    fiches: &[FicheProduit],
) -> Resultat<BilanImport> {
    exiger(conn, acteur, Droit::AjouterReferentiel)?;
    let tx = conn.transaction()?;
    let mut bilan = BilanImport::default();
    for (rang, fiche) in fiches.iter().enumerate() {
        let f = fiche
            .nettoyee()
            .map_err(|e| refus("fiche {} : {}", &[&(rang + 1), &e]))?;
        if let Some(existant) = par_reference(&tx, &f.reference)? {
            bilan.ignorees.push((
                rang,
                format!(
                    "la référence « {} » existe déjà (« {} »)",
                    existant.fiche.reference, existant.fiche.designation
                ),
            ));
            continue;
        }
        inserer(&tx, &f, acteur.id)?;
        bilan.ajoutees += 1;
    }
    tx.commit()?;
    Ok(bilan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::droits::tests::compte;
    use crate::ouvrir_en_memoire;
    use kadenz_domaine::role::Role;

    fn fiche(reference: &str, designation: &str, sens: Sens) -> FicheProduit {
        FicheProduit {
            designation: designation.into(),
            reference: reference.into(),
            prix: 1_500,
            sens,
        }
    }

    #[test]
    fn reference_unique_sans_casse() {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        let p = creer(
            &conn,
            &admin,
            &fiche(" CIM-42 ", "Ciment 42,5", Sens::Vente),
        )
        .unwrap();
        assert_eq!(p.fiche.reference, "CIM-42");
        assert!(matches!(
            creer(&conn, &admin, &fiche("cim-42", "Autre", Sens::Vente)),
            Err(Erreur::ValidationComposee { .. })
        ));
        let autre = creer(&conn, &admin, &fiche("FER-8", "Fer de 8", Sens::Achat)).unwrap();
        assert!(
            modifier(
                &conn,
                &admin,
                autre.id,
                &fiche("Cim-42", "Fer", Sens::Achat)
            )
            .is_err()
        );
        // Changer la casse de sa propre référence reste permis.
        modifier(
            &conn,
            &admin,
            p.id,
            &fiche("cim-42", "Ciment 42,5", Sens::Vente),
        )
        .unwrap();
        let mut negatif = fiche("X", "X", Sens::Vente);
        negatif.prix = -1;
        assert!(creer(&conn, &admin, &negatif).is_err());
    }

    #[test]
    fn filtres_par_sens_et_texte() {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        creer(&conn, &admin, &fiche("A", "Pelle", Sens::Vente)).unwrap();
        creer(&conn, &admin, &fiche("B", "Brouette", Sens::Achat)).unwrap();
        creer(&conn, &admin, &fiche("C", "Tôle ondulée", Sens::LesDeux)).unwrap();
        let achats = lister(
            &conn,
            &FiltreProduits {
                sens: Some(Sens::Achat),
                ..Default::default()
            },
        )
        .unwrap();
        let noms: Vec<_> = achats
            .iter()
            .map(|p| p.fiche.designation.as_str())
            .collect();
        assert_eq!(noms, ["Brouette", "Tôle ondulée"]);
        let tole = lister(
            &conn,
            &FiltreProduits {
                texte: "TOLE".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(tole.len(), 1);
    }

    #[test]
    fn droits_et_retrait() {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        let vente = compte(&conn, "vente", Role::Commercial);
        let p = creer(&conn, &vente, &fiche("A", "Pelle", Sens::Vente)).unwrap();
        assert!(matches!(
            modifier(&conn, &vente, p.id, &p.fiche),
            Err(Erreur::Interdit(Droit::ModifierReferentiel))
        ));
        conn.execute(
            "INSERT INTO document (type, date, date_creation)
             VALUES ('devis', '2026-11-09', '2026-11-09T10:00:00')",
            [],
        )
        .unwrap();
        let doc = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO ligne_document (document_id, produit_id, quantite, ordre)
             VALUES (?1, ?2, 1000, 1)",
            [doc, p.id],
        )
        .unwrap();
        assert_eq!(retirer(&conn, &admin, p.id).unwrap(), Retrait::Desactivee);
        let libre = creer(&conn, &admin, &fiche("B", "Brouette", Sens::Vente)).unwrap();
        assert_eq!(
            retirer(&conn, &admin, libre.id).unwrap(),
            Retrait::Supprimee
        );
    }

    #[test]
    fn import_avec_doublons() {
        let mut conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        creer(&conn, &admin, &fiche("A", "Pelle", Sens::Vente)).unwrap();
        let fiches = [
            fiche("a", "Pelle bis", Sens::Vente),
            fiche("B", "Brouette", Sens::Vente),
            fiche("b", "Brouette bis", Sens::Vente),
        ];
        let bilan = importer(&mut conn, &admin, &fiches).unwrap();
        assert_eq!(bilan.ajoutees, 1);
        assert_eq!(
            bilan.ignorees.iter().map(|(r, _)| *r).collect::<Vec<_>>(),
            [0, 2]
        );
    }
}
