//! Fichier des tiers, clients et fournisseurs (cahier des charges, section 5.2).
//!
//! Droits : « ajouter » suffit pour créer une fiche ou en importer ; modifier,
//! désactiver ou retirer demande « modifier » (section 10.1, le rôle Commercial
//! n'a que l'ajout).

pub use kadenz_domaine::referentiel::{FicheTiers, TypeTiers};
use kadenz_domaine::role::Droit;
use kadenz_domaine::texte;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use crate::audit::{self, Entree};
use crate::comptes::Utilisateur;
use crate::droits::exiger;
use crate::{BilanImport, Erreur, Resultat, Retrait};

fn en_json(f: &FicheTiers) -> Value {
    json!({
        "nom": f.nom,
        "adresse": f.adresse,
        "telephone": f.telephone,
        "localisation": f.localisation,
        "ifu": f.ifu,
        "rccm": f.rccm,
        "regime_imposition": f.regime_imposition,
        "type": f.type_tiers.code(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tiers {
    pub id: i64,
    pub fiche: FicheTiers,
    pub actif: bool,
}

/// Critères de la liste des tiers.
#[derive(Debug, Clone, Default)]
pub struct FiltreTiers {
    /// Cherché dans le nom, le téléphone, la localisation, l'adresse et l'IFU,
    /// sans tenir compte de la casse ni des accents.
    pub texte: String,
    /// `Client` retient aussi les fiches « les deux », de même pour `Fournisseur` ;
    /// `LesDeux` ne retient que les fiches « les deux ».
    pub type_tiers: Option<TypeTiers>,
    pub inclure_inactifs: bool,
}

impl FiltreTiers {
    fn retient(&self, t: &Tiers) -> bool {
        if !self.inclure_inactifs && !t.actif {
            return false;
        }
        let type_ok = match self.type_tiers {
            None => true,
            Some(TypeTiers::Client) => t.fiche.type_tiers.est_client(),
            Some(TypeTiers::Fournisseur) => t.fiche.type_tiers.est_fournisseur(),
            Some(TypeTiers::LesDeux) => t.fiche.type_tiers == TypeTiers::LesDeux,
        };
        let f = &t.fiche;
        type_ok
            && [&f.nom, &f.telephone, &f.localisation, &f.adresse, &f.ifu]
                .iter()
                .any(|champ| texte::contient(champ, &self.texte))
    }
}

const COLONNES: &str =
    "id, nom, adresse, telephone, localisation, ifu, rccm, regime_imposition, type, actif";

fn ligne_vers_tiers(r: &rusqlite::Row<'_>) -> rusqlite::Result<Tiers> {
    let code: String = r.get(8)?;
    let type_tiers = TypeTiers::depuis_code(&code).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            8,
            rusqlite::types::Type::Text,
            format!("type de tiers inconnu : {code}").into(),
        )
    })?;
    Ok(Tiers {
        id: r.get(0)?,
        fiche: FicheTiers {
            nom: r.get(1)?,
            adresse: r.get(2)?,
            telephone: r.get(3)?,
            localisation: r.get(4)?,
            ifu: r.get(5)?,
            rccm: r.get(6)?,
            regime_imposition: r.get(7)?,
            type_tiers,
        },
        actif: r.get(9)?,
    })
}

pub fn lire(conn: &Connection, id: i64) -> Resultat<Option<Tiers>> {
    Ok(conn
        .query_row(
            &format!("SELECT {COLONNES} FROM tiers WHERE id = ?1"),
            [id],
            ligne_vers_tiers,
        )
        .optional()?)
}

fn lire_existant(conn: &Connection, id: i64) -> Resultat<Tiers> {
    lire(conn, id)?.ok_or_else(|| Erreur::Validation("ce tiers n'existe plus".into()))
}

/// Liste triée par nom.
pub fn lister(conn: &Connection, filtre: &FiltreTiers) -> Resultat<Vec<Tiers>> {
    let mut requete = conn.prepare(&format!(
        "SELECT {COLONNES} FROM tiers ORDER BY nom COLLATE NOCASE, id"
    ))?;
    let tous = requete
        .query_map([], ligne_vers_tiers)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(tous.into_iter().filter(|t| filtre.retient(t)).collect())
}

fn inserer(conn: &Connection, f: &FicheTiers, auteur: i64) -> Resultat<Tiers> {
    conn.execute(
        "INSERT INTO tiers (nom, adresse, telephone, localisation, ifu, rccm,
                            regime_imposition, type)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            f.nom,
            f.adresse,
            f.telephone,
            f.localisation,
            f.ifu,
            f.rccm,
            f.regime_imposition,
            f.type_tiers.code()
        ],
    )?;
    let id = conn.last_insert_rowid();
    audit::journaliser(
        conn,
        Entree {
            entite: "tiers",
            entite_id: id,
            action: "creation",
            ancienne_valeur: None,
            nouvelle_valeur: Some(en_json(f)),
            utilisateur_id: Some(auteur),
        },
    )?;
    Ok(Tiers {
        id,
        fiche: f.clone(),
        actif: true,
    })
}

pub fn creer(conn: &Connection, acteur: &Utilisateur, fiche: &FicheTiers) -> Resultat<Tiers> {
    exiger(conn, acteur, Droit::AjouterReferentiel)?;
    inserer(
        conn,
        &fiche.nettoyee().map_err(Erreur::Validation)?,
        acteur.id,
    )
}

pub fn modifier(
    conn: &Connection,
    acteur: &Utilisateur,
    id: i64,
    fiche: &FicheTiers,
) -> Resultat<()> {
    exiger(conn, acteur, Droit::ModifierReferentiel)?;
    let f = fiche.nettoyee().map_err(Erreur::Validation)?;
    let ancien = lire_existant(conn, id)?;
    conn.execute(
        "UPDATE tiers SET nom = ?1, adresse = ?2, telephone = ?3, localisation = ?4, ifu = ?5,
                          rccm = ?6, regime_imposition = ?7, type = ?8
         WHERE id = ?9",
        params![
            f.nom,
            f.adresse,
            f.telephone,
            f.localisation,
            f.ifu,
            f.rccm,
            f.regime_imposition,
            f.type_tiers.code(),
            id
        ],
    )?;
    audit::journaliser(
        conn,
        Entree {
            entite: "tiers",
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
        "UPDATE tiers SET actif = ?1 WHERE id = ?2",
        params![actif, id],
    )?;
    audit::journaliser(
        conn,
        Entree {
            entite: "tiers",
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
        "SELECT EXISTS (SELECT 1 FROM document WHERE tiers_id = ?1)",
        [id],
        |r| r.get(0),
    )?)
}

/// Supprime la fiche si aucun document ne la cite, sinon la désactive
/// (section 5.3 : l'historique reste intact).
pub fn retirer(conn: &Connection, acteur: &Utilisateur, id: i64) -> Resultat<Retrait> {
    exiger(conn, acteur, Droit::ModifierReferentiel)?;
    let ancien = lire_existant(conn, id)?;
    if est_utilise(conn, id)? {
        definir_actif(conn, acteur, id, false)?;
        return Ok(Retrait::Desactivee);
    }
    conn.execute("DELETE FROM tiers WHERE id = ?1", [id])?;
    audit::journaliser(
        conn,
        Entree {
            entite: "tiers",
            entite_id: id,
            action: "suppression",
            ancienne_valeur: Some(en_json(&ancien.fiche)),
            nouvelle_valeur: None,
            utilisateur_id: Some(acteur.id),
        },
    )?;
    Ok(Retrait::Supprimee)
}

/// Fiche existante portant le même nom (à la casse, aux accents et aux espaces près).
/// Sert à signaler les doublons à l'import.
pub fn homonyme(conn: &Connection, nom: &str) -> Resultat<Option<Tiers>> {
    let cle = texte::cle_comparaison(nom);
    Ok(lister(
        conn,
        &FiltreTiers {
            inclure_inactifs: true,
            ..Default::default()
        },
    )?
    .into_iter()
    .find(|t| texte::cle_comparaison(&t.fiche.nom) == cle))
}

/// Ajoute les fiches en une seule transaction. Une fiche dont le nom existe déjà
/// (en base ou plus haut dans la liste) est ignorée et signalée ; une fiche
/// invalide fait tout annuler.
pub fn importer(
    conn: &mut Connection,
    acteur: &Utilisateur,
    fiches: &[FicheTiers],
) -> Resultat<BilanImport> {
    exiger(conn, acteur, Droit::AjouterReferentiel)?;
    let tx = conn.transaction()?;
    let mut noms: std::collections::HashSet<String> = lister(
        &tx,
        &FiltreTiers {
            inclure_inactifs: true,
            ..Default::default()
        },
    )?
    .iter()
    .map(|t| texte::cle_comparaison(&t.fiche.nom))
    .collect();
    let mut bilan = BilanImport::default();
    for (rang, fiche) in fiches.iter().enumerate() {
        let f = fiche
            .nettoyee()
            .map_err(|e| Erreur::Validation(format!("fiche {} : {e}", rang + 1)))?;
        if !noms.insert(texte::cle_comparaison(&f.nom)) {
            bilan
                .ignorees
                .push((rang, format!("« {} » existe déjà", f.nom)));
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

    fn fiche(nom: &str, type_tiers: TypeTiers) -> FicheTiers {
        FicheTiers {
            nom: nom.into(),
            ..FicheTiers::vide(type_tiers)
        }
    }

    #[test]
    fn creation_modification_et_filtres() {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        let a = creer(
            &conn,
            &admin,
            &fiche("  Société Générale ", TypeTiers::Client),
        )
        .unwrap();
        assert_eq!(a.fiche.nom, "Société Générale");
        creer(
            &conn,
            &admin,
            &fiche("Ciment du Faso", TypeTiers::Fournisseur),
        )
        .unwrap();
        creer(&conn, &admin, &fiche("Atelier Bâtir", TypeTiers::LesDeux)).unwrap();

        let clients = lister(
            &conn,
            &FiltreTiers {
                type_tiers: Some(TypeTiers::Client),
                ..Default::default()
            },
        )
        .unwrap();
        let noms: Vec<_> = clients.iter().map(|t| t.fiche.nom.as_str()).collect();
        assert_eq!(noms, ["Atelier Bâtir", "Société Générale"]);

        let recherche = lister(
            &conn,
            &FiltreTiers {
                texte: "generale".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(recherche.len(), 1);

        let mut f = a.fiche.clone();
        f.ifu = "00012345A".into();
        modifier(&conn, &admin, a.id, &f).unwrap();
        assert_eq!(lire(&conn, a.id).unwrap().unwrap().fiche.ifu, "00012345A");
        assert!(creer(&conn, &admin, &fiche("  ", TypeTiers::Client)).is_err());
    }

    #[test]
    fn droits_par_role() {
        let conn = ouvrir_en_memoire().unwrap();
        let vente = compte(&conn, "vente", Role::Commercial);
        let lecture = compte(&conn, "lecture", Role::Consultation);
        let t = creer(&conn, &vente, &fiche("Client A", TypeTiers::Client)).unwrap();
        assert!(matches!(
            modifier(&conn, &vente, t.id, &t.fiche),
            Err(Erreur::Interdit(Droit::ModifierReferentiel))
        ));
        assert!(retirer(&conn, &vente, t.id).is_err());
        assert!(matches!(
            creer(&conn, &lecture, &fiche("Client B", TypeTiers::Client)),
            Err(Erreur::Interdit(Droit::AjouterReferentiel))
        ));
    }

    #[test]
    fn retrait_supprime_ou_desactive() {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        let libre = creer(&conn, &admin, &fiche("Libre", TypeTiers::Client)).unwrap();
        let cite = creer(&conn, &admin, &fiche("Cité", TypeTiers::Client)).unwrap();
        conn.execute(
            "INSERT INTO document (type, date, date_creation, tiers_id)
             VALUES ('devis', '2026-11-09', '2026-11-09T10:00:00', ?1)",
            [cite.id],
        )
        .unwrap();

        assert_eq!(
            retirer(&conn, &admin, libre.id).unwrap(),
            Retrait::Supprimee
        );
        assert!(lire(&conn, libre.id).unwrap().is_none());
        assert_eq!(
            retirer(&conn, &admin, cite.id).unwrap(),
            Retrait::Desactivee
        );
        assert!(!lire(&conn, cite.id).unwrap().unwrap().actif);
        assert!(lister(&conn, &FiltreTiers::default()).unwrap().is_empty());
        definir_actif(&conn, &admin, cite.id, true).unwrap();
        assert_eq!(lister(&conn, &FiltreTiers::default()).unwrap().len(), 1);
    }

    #[test]
    fn import_avec_doublons() {
        let mut conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        creer(&conn, &admin, &fiche("Société Générale", TypeTiers::Client)).unwrap();
        let fiches = [
            fiche("SOCIETE GENERALE", TypeTiers::Client),
            fiche("Nouveau", TypeTiers::Client),
            fiche("nouveau ", TypeTiers::Fournisseur),
        ];
        let bilan = importer(&mut conn, &admin, &fiches).unwrap();
        assert_eq!(bilan.ajoutees, 1);
        assert_eq!(
            bilan.ignorees.iter().map(|(r, _)| *r).collect::<Vec<_>>(),
            [0, 2]
        );
        assert!(homonyme(&conn, "nouveau").unwrap().is_some());

        let invalide = [
            fiche("Autre", TypeTiers::Client),
            fiche("", TypeTiers::Client),
        ];
        assert!(importer(&mut conn, &admin, &invalide).is_err());
        assert!(
            homonyme(&conn, "Autre").unwrap().is_none(),
            "tout est annulé"
        );
    }
}
