//! Reprise d'une base V1 dans le serveur de l'entreprise (phase 2, jalon F7 ;
//! décisions 19, 45 et 53 à 55 ; écrans, section 8).
//!
//! - Sur le poste, la base V1 n'est jamais ouverte ni migrée : une copie de
//!   ses fichiers est sauvegardée en `.kzs` ([`exporter_base_v1`]), et c'est ce
//!   fichier qui voyage, avec un mot de passe à usage unique.
//! - Sur le serveur, la sauvegarde est copiée en mémoire et migrée au schéma
//!   courant ([`ouvrir_pour_reprise`]), puis versée dans la base du serveur en
//!   une seule transaction ([`reprendre`]) : tout ou rien.
//! - Une seule base par serveur : la reprise est refusée dès que le serveur a
//!   un tiers, un produit, un document ou un règlement (décision 55).
//! - Les comptes V1 sont repris, sauf ceux dont l'identifiant existe déjà sur
//!   le serveur (décision 53) ; ce qui leur était attribué passe au compte du
//!   serveur de même identifiant. La fiche de l'entreprise et les paramètres
//!   partagés de la V1 remplacent ceux du serveur ; l'identifiant d'entreprise,
//!   la licence d'abonnement et les clés propres au serveur restent (décision
//!   54).
//! - Les tiers, produits, documents et règlements gardent leurs identifiants
//!   et leurs `uid` : les tables du serveur sont vides. Chaque ligne écrite
//!   entre dans la suite des changements par les déclencheurs habituels, et
//!   les postes la reçoivent.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use kadenz_domaine::document::TypeDocument;
use kadenz_domaine::role::{Droit, Role};
use kadenz_protocole::reprise::{DocumentsRepris, ResumeReprise};
use rusqlite::types::Value;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params, params_from_iter};
use serde_json::json;

use crate::audit::{self, Entree};
use crate::chiffrement::{self, avec_suffixe, avec_suffixe_brut};
use crate::comptes::Utilisateur;
use crate::replique::CLES_PARTAGEES;
use crate::{Erreur, Resultat, ouvrir_sauvegarde, refus, sauvegarder, version_schema_application};

/// Message du refus d'une reprise sur un serveur qui a déjà des données.
pub const MODELE_PAS_NEUF: &str =
    "le serveur a déjà des données : une base V1 ne peut être reprise que sur un serveur neuf";

/// Sauvegarde en `.kzs` de la base V1 `chemin_base`, chiffrée par
/// `mot_de_passe`. La base d'origine n'est jamais ouverte : ses fichiers
/// (base, `-wal`, `-shm`) sont copiés à côté de `destination`, la copie est
/// ouverte avec la clé du poste (`kadenz.cle`, à côté de la base d'origine),
/// sauvegardée, puis effacée.
pub fn exporter_base_v1(
    chemin_base: &Path,
    destination: &Path,
    mot_de_passe: &str,
) -> Resultat<()> {
    if !chemin_base.is_file() {
        return Err(refus("base V1 introuvable : {}", &[&chemin_base.display()]));
    }
    let travail = avec_suffixe(destination, "copie");
    let _ = fs::remove_dir_all(&travail);
    fs::create_dir_all(&travail)?;
    let resultat = (|| -> Resultat<()> {
        let copie = travail.join("kadenz.sqlite");
        fs::copy(chemin_base, &copie)?;
        for suffixe in ["-wal", "-shm"] {
            let fichier = avec_suffixe_brut(chemin_base, suffixe);
            if fichier.is_file() {
                fs::copy(&fichier, avec_suffixe_brut(&copie, suffixe))?;
            }
        }
        // Base d'avant le chiffrement (V1 ancienne) : sa copie est d'abord
        // chiffrée, la sauvegarde ne copiant que d'une base chiffrée à une
        // autre.
        let cle = if chiffrement::en_clair(&copie)? {
            chiffrement::chiffrer_base_en_clair(&copie, mot_de_passe)?;
            mot_de_passe.to_string()
        } else {
            chiffrement::cle(&chiffrement::chemin_cle(chemin_base), false)?
        };
        let conn = Connection::open(&copie)?;
        chiffrement::appliquer_cle(&conn, &cle)?;
        sauvegarder(&conn, destination, mot_de_passe)
    })();
    let _ = fs::remove_dir_all(&travail);
    if resultat.is_err() {
        let _ = fs::remove_file(destination);
    }
    resultat
}

/// Copie de travail d'une sauvegarde, migrée pour la reprise ; le fichier de
/// travail est effacé avec elle.
pub struct BaseReprise {
    conn: Option<Connection>,
    travail: PathBuf,
}

impl std::ops::Deref for BaseReprise {
    type Target = Connection;

    fn deref(&self) -> &Connection {
        self.conn.as_ref().expect("copie de travail ouverte")
    }
}

impl Drop for BaseReprise {
    fn drop(&mut self) {
        // Fermée d'abord : Windows refuse d'effacer un fichier ouvert.
        drop(self.conn.take());
        effacer_copie(&self.travail);
    }
}

fn effacer_copie(travail: &Path) {
    for fichier in [
        travail.to_path_buf(),
        avec_suffixe_brut(travail, "-wal"),
        avec_suffixe_brut(travail, "-shm"),
        avec_suffixe_brut(travail, "-journal"),
    ] {
        let _ = fs::remove_file(fichier);
    }
}

/// Ouvre une sauvegarde `.kzs` pour la reprise : copiée à côté d'elle
/// (`<sauvegarde>.reprise`, chiffrée par le même mot de passe), puis migrée au
/// schéma courant ; la sauvegarde n'est pas modifiée. Une sauvegarde d'un
/// schéma plus récent que l'application est refusée.
pub fn ouvrir_pour_reprise(sauvegarde: &Path, mot_de_passe: &str) -> Resultat<BaseReprise> {
    let source = ouvrir_sauvegarde(sauvegarde, mot_de_passe)?;
    let version: u32 = source.pragma_query_value(None, "user_version", |r| r.get(0))?;
    let application = version_schema_application();
    if version > application {
        return Err(Erreur::VersionSchemaInconnue {
            base: version,
            application,
        });
    }
    let travail = avec_suffixe(sauvegarde, "reprise");
    effacer_copie(&travail);
    let ouverte = (|| -> Resultat<Connection> {
        let mut copie = Connection::open(&travail)?;
        chiffrement::appliquer_cle(&copie, mot_de_passe)?;
        rusqlite::backup::Backup::new(&source, &mut copie)?.run_to_completion(
            256,
            std::time::Duration::ZERO,
            None,
        )?;
        copie.pragma_update(None, "journal_mode", "DELETE")?;
        crate::preparer(&mut copie)?;
        Ok(copie)
    })();
    match ouverte {
        Ok(conn) => Ok(BaseReprise {
            conn: Some(conn),
            travail,
        }),
        Err(e) => {
            effacer_copie(&travail);
            Err(e)
        }
    }
}

/// Résumé d'une base, lisible sur tout schéma de la V1 (migré ou non) : nom de
/// l'entreprise, tiers, produits, règlements, documents et dernier numéro par
/// type, identifiants des comptes (tous dans `comptes_repris`, voir
/// [`separer_comptes`]).
pub fn resume(conn: &Connection) -> Resultat<ResumeReprise> {
    let compter = |sql: &str| -> Resultat<u32> { Ok(conn.query_row(sql, [], |r| r.get(0))?) };
    let entreprise = conn
        .query_row("SELECT nom FROM entreprise WHERE id = 1", [], |r| {
            r.get::<_, String>(0)
        })
        .optional()?
        .unwrap_or_default();
    let mut documents = Vec::new();
    for type_document in TypeDocument::TOUS {
        let nombre: u32 = conn.query_row(
            "SELECT count(*) FROM document WHERE type = ?1",
            [type_document.code()],
            |r| r.get(0),
        )?;
        if nombre == 0 {
            continue;
        }
        let dernier_numero = conn
            .query_row(
                "SELECT numero FROM document WHERE type = ?1 AND numero IS NOT NULL
                 ORDER BY coalesce(date_validation, date_creation) DESC, id DESC LIMIT 1",
                [type_document.code()],
                |r| r.get::<_, String>(0),
            )
            .optional()?;
        documents.push(DocumentsRepris {
            type_document: type_document.code().into(),
            nombre,
            dernier_numero,
        });
    }
    let comptes_repris = conn
        .prepare("SELECT identifiant FROM utilisateur ORDER BY id")?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ResumeReprise {
        entreprise,
        clients: compter("SELECT count(*) FROM tiers WHERE type IN ('client', 'les_deux')")?,
        fournisseurs: compter(
            "SELECT count(*) FROM tiers WHERE type IN ('fournisseur', 'les_deux')",
        )?,
        produits: compter("SELECT count(*) FROM produit")?,
        reglements: compter("SELECT count(*) FROM reglement")?,
        documents,
        comptes_repris,
        comptes_ecartes: Vec::new(),
    })
}

/// Passe dans `comptes_ecartes` les comptes du résumé dont l'identifiant existe
/// déjà dans `cible` (base du serveur, ou copie d'un poste relié), sans tenir
/// compte de la casse, comme la connexion (décision 53).
pub fn separer_comptes(resume: &mut ResumeReprise, cible: &Connection) -> Resultat<()> {
    let mut repris = Vec::new();
    for identifiant in std::mem::take(&mut resume.comptes_repris) {
        if compte_de(cible, &identifiant)?.is_some() {
            resume.comptes_ecartes.push(identifiant);
        } else {
            repris.push(identifiant);
        }
    }
    resume.comptes_repris = repris;
    Ok(())
}

fn compte_de(conn: &Connection, identifiant: &str) -> Resultat<Option<i64>> {
    Ok(conn
        .query_row(
            "SELECT id FROM utilisateur WHERE identifiant = ?1",
            [identifiant],
            |r| r.get(0),
        )
        .optional()?)
}

/// Vrai si le serveur n'a encore ni tiers, ni produit, ni document, ni
/// règlement (décision 55) : des comptes ne comptent pas.
pub fn serveur_neuf(conn: &Connection) -> Resultat<bool> {
    let lignes: i64 = conn.query_row(
        "SELECT (SELECT count(*) FROM tiers) + (SELECT count(*) FROM produit)
              + (SELECT count(*) FROM document) + (SELECT count(*) FROM reglement)",
        [],
        |r| r.get(0),
    )?;
    Ok(lignes == 0)
}

/// Verse la base V1 `source` (ouverte par [`ouvrir_pour_reprise`]) dans la
/// base du serveur `cible`, en une transaction, et rend le bilan mesuré sur
/// le serveur. Compte Admin seul ; refus si le serveur n'est pas neuf.
pub fn reprendre(
    cible: &mut Connection,
    acteur: &Utilisateur,
    source: &Connection,
) -> Resultat<ResumeReprise> {
    if acteur.role != Role::Admin {
        return Err(Erreur::Interdit(Droit::GererParametres));
    }
    // Sans licence valide, le serveur est en lecture seule (décision 40).
    crate::abonnement::exiger_ecriture(cible)?;
    let tx = cible.transaction_with_behavior(TransactionBehavior::Immediate)?;
    if !serveur_neuf(&tx)? {
        return Err(refus(MODELE_PAS_NEUF, &[]));
    }
    // Un document peut citer un document de rang plus élevé (source d'une
    // conversion) : les clés étrangères sont vérifiées à la validation.
    tx.pragma_update(None, "defer_foreign_keys", "ON")?;

    // Fiche de l'entreprise (décision 54).
    let (colonnes, valeurs) = ligne(source, "entreprise", "id = 1")?;
    if !colonnes.is_empty() {
        let affectations: Vec<String> = colonnes
            .iter()
            .filter(|c| *c != "id")
            .map(|c| format!("{c} = ?"))
            .collect();
        let valeurs: Vec<Value> = colonnes
            .iter()
            .zip(valeurs)
            .filter(|(c, _)| *c != "id")
            .map(|(_, v)| v)
            .collect();
        let modifiee = tx.execute(
            &format!(
                "UPDATE entreprise SET {} WHERE id = 1",
                affectations.join(", ")
            ),
            params_from_iter(valeurs.iter()),
        )?;
        if modifiee == 0 {
            copier(source, &tx, "entreprise", true, |_, _| Ok(()))?;
        }
    }

    // Paramètres partagés (décision 54) ; les clés propres au serveur restent.
    for cle in CLES_PARTAGEES {
        let valeur: Option<String> = source
            .query_row(
                "SELECT valeur FROM parametre_systeme WHERE cle = ?1",
                [cle],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(valeur) = valeur {
            tx.execute(
                "INSERT INTO parametre_systeme (cle, valeur) VALUES (?1, ?2)
                 ON CONFLICT (cle) DO UPDATE SET valeur = excluded.valeur",
                params![cle, valeur],
            )?;
        }
    }

    // Comptes (décision 53).
    let mut comptes: HashMap<i64, i64> = HashMap::new();
    let mut comptes_repris = Vec::new();
    let mut comptes_ecartes = Vec::new();
    let colonnes_comptes = colonnes_communes(source, &tx, "utilisateur")?;
    let rang_identifiant = colonnes_comptes
        .iter()
        .position(|c| c == "identifiant")
        .ok_or_else(|| Erreur::Validation("base V1 sans identifiant de compte".into()))?;
    for (ancien, valeurs) in lignes(source, "utilisateur", &colonnes_comptes)? {
        let Value::Text(identifiant) = valeurs[rang_identifiant].clone() else {
            continue;
        };
        if let Some(present) = compte_de(&tx, &identifiant)? {
            comptes.insert(ancien, present);
            comptes_ecartes.push(identifiant);
            continue;
        }
        let nouveau = inserer(&tx, "utilisateur", &colonnes_comptes, &valeurs, false)?;
        comptes.insert(ancien, nouveau);
        comptes_repris.push(identifiant);
    }
    let compte = |v: &mut Value| {
        if let Value::Integer(id) = v {
            *v = comptes.get(id).map_or(Value::Null, |n| Value::Integer(*n));
        }
    };

    // Instantanés de la fiche, cités par les documents validés.
    let instantanes = copier(source, &tx, "instantane_entreprise", false, |_, _| Ok(()))?;

    for table in ["tiers", "produit"] {
        copier(source, &tx, table, true, |_, _| Ok(()))?;
    }
    copier(source, &tx, "document", true, |colonnes, valeurs| {
        for (c, v) in colonnes.iter().zip(valeurs.iter_mut()) {
            match c.as_str() {
                "utilisateur_creation_id" | "utilisateur_validation_id" => compte(v),
                "instantane_entreprise_id" => {
                    if let Value::Integer(id) = v {
                        *v = instantanes
                            .get(id)
                            .map_or(Value::Null, |n| Value::Integer(*n));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    })?;
    copier(source, &tx, "ligne_document", true, |_, _| Ok(()))?;
    copier(source, &tx, "reglement", true, |colonnes, valeurs| {
        for (c, v) in colonnes.iter().zip(valeurs.iter_mut()) {
            if c == "utilisateur_id" {
                compte(v);
            }
        }
        Ok(())
    })?;

    // Numérotation continue : chaque compteur repart du plus grand rang.
    let compteurs: Vec<(String, i64, i64, i64)> = source
        .prepare("SELECT type_document, annee, mois, dernier_numero FROM compteur_numerotation")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<Result<_, _>>()?;
    for (type_document, annee, mois, dernier) in compteurs {
        tx.execute(
            "INSERT INTO compteur_numerotation (type_document, annee, mois, dernier_numero)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (type_document, annee, mois)
             DO UPDATE SET dernier_numero = max(dernier_numero, excluded.dernier_numero)",
            params![type_document, annee, mois, dernier],
        )?;
    }

    // Journal d'audit de la V1, à la suite de celui du serveur.
    copier(source, &tx, "audit_log", false, |colonnes, valeurs| {
        let utilisateur = colonnes
            .iter()
            .zip(valeurs.iter())
            .any(|(c, v)| c == "entite" && *v == Value::Text("utilisateur".into()));
        for (c, v) in colonnes.iter().zip(valeurs.iter_mut()) {
            match c.as_str() {
                "utilisateur_id" => compte(v),
                "entite_id" if utilisateur => {
                    if let Value::Integer(id) = v {
                        *v = Value::Integer(comptes.get(id).copied().unwrap_or(*id));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    })?;

    let mut bilan = resume(&tx)?;
    bilan.comptes_repris = comptes_repris;
    bilan.comptes_ecartes = comptes_ecartes;
    audit::journaliser(
        &tx,
        Entree {
            entite: "reprise",
            entite_id: 0,
            action: "reprise_v1",
            ancienne_valeur: None,
            nouvelle_valeur: Some(json!({
                "entreprise": bilan.entreprise,
                "clients": bilan.clients,
                "fournisseurs": bilan.fournisseurs,
                "produits": bilan.produits,
                "reglements": bilan.reglements,
                "documents": bilan.documents.iter().map(|d| d.nombre).sum::<u32>(),
                "comptes_repris": bilan.comptes_repris,
                "comptes_ecartes": bilan.comptes_ecartes,
            })),
            utilisateur_id: Some(acteur.id),
        },
    )?;
    tx.commit()?;
    Ok(bilan)
}

/// Colonnes de `table` présentes dans les deux bases, dans l'ordre de la cible.
fn colonnes_communes(
    source: &Connection,
    cible: &Connection,
    table: &str,
) -> Resultat<Vec<String>> {
    let noms = |conn: &Connection| -> Resultat<Vec<String>> {
        Ok(conn
            .prepare("SELECT name FROM pragma_table_info(?1)")?
            .query_map([table], |r| r.get::<_, String>(0))?
            .collect::<Result<_, _>>()?)
    };
    let dans_source = noms(source)?;
    Ok(noms(cible)?
        .into_iter()
        .filter(|c| dans_source.contains(c))
        .collect())
}

/// Une ligne de `table` choisie par `condition` : colonnes et valeurs ; aucune
/// colonne si elle n'existe pas.
fn ligne(conn: &Connection, table: &str, condition: &str) -> Resultat<(Vec<String>, Vec<Value>)> {
    let mut req = conn.prepare(&format!("SELECT * FROM {table} WHERE {condition}"))?;
    let colonnes: Vec<String> = req.column_names().map(str::to_string).collect();
    let n = colonnes.len();
    let valeurs = req
        .query_row([], |r| (0..n).map(|i| r.get::<_, Value>(i)).collect())
        .optional()?;
    Ok(match valeurs {
        Some(valeurs) => (colonnes, valeurs),
        None => (Vec::new(), Vec::new()),
    })
}

/// Lignes de `table` (colonnes données, `id` compris), par identifiant croissant.
fn lignes(conn: &Connection, table: &str, colonnes: &[String]) -> Resultat<Vec<(i64, Vec<Value>)>> {
    let mut req = conn.prepare(&format!(
        "SELECT id, {} FROM {table} ORDER BY id",
        colonnes.join(", ")
    ))?;
    let n = colonnes.len();
    let lignes = req
        .query_map([], |r| {
            let id: i64 = r.get(0)?;
            let valeurs = (1..=n)
                .map(|i| r.get::<_, Value>(i))
                .collect::<Result<_, _>>()?;
            Ok((id, valeurs))
        })?
        .collect::<Result<_, _>>()?;
    Ok(lignes)
}

/// Insère une ligne ; sans `garder_id`, la colonne `id` est laissée au
/// serveur. Rend l'identifiant de la ligne insérée.
fn inserer(
    conn: &Connection,
    table: &str,
    colonnes: &[String],
    valeurs: &[Value],
    garder_id: bool,
) -> Resultat<i64> {
    let (colonnes, valeurs): (Vec<&String>, Vec<&Value>) = colonnes
        .iter()
        .zip(valeurs)
        .filter(|(c, _)| garder_id || *c != "id")
        .unzip();
    let marques = vec!["?"; colonnes.len()].join(", ");
    let noms: Vec<&str> = colonnes.iter().map(|c| c.as_str()).collect();
    conn.execute(
        &format!(
            "INSERT INTO {table} ({}) VALUES ({marques})",
            noms.join(", ")
        ),
        params_from_iter(valeurs),
    )?;
    Ok(conn.last_insert_rowid())
}

/// Copie toutes les lignes de `table`, après `ajuster` ; rend la
/// correspondance des identifiants (ancien vers nouveau).
fn copier(
    source: &Connection,
    cible: &Connection,
    table: &str,
    garder_id: bool,
    mut ajuster: impl FnMut(&[String], &mut [Value]) -> Resultat<()>,
) -> Resultat<HashMap<i64, i64>> {
    let colonnes = colonnes_communes(source, cible, table)?;
    let mut correspondance = HashMap::new();
    for (ancien, mut valeurs) in lignes(source, table, &colonnes)? {
        ajuster(&colonnes, &mut valeurs)?;
        let nouveau = inserer(cible, table, &colonnes, &valeurs, garder_id)?;
        correspondance.insert(ancien, nouveau);
    }
    Ok(correspondance)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comptes::NouvelUtilisateur;
    use crate::documents::{LigneSaisie, Saisie};
    use crate::droits::tests::compte;
    use crate::entreprise::Entreprise;
    use crate::installation::InstallationInitiale;
    use crate::parametres::Parametres;
    use crate::reglements::SaisieReglement;
    use crate::relances::TextesRelance;
    use crate::{documents, installation, reglements, relances, synchro, tiers};
    use chrono::NaiveDate;
    use kadenz_domaine::calcul::UNE_UNITE;
    use kadenz_domaine::referentiel::{FicheTiers, TypeTiers};

    const MOT_DE_PASSE: &str = "unique-pour-la-reprise";

    struct V1 {
        dossier: PathBuf,
        base: PathBuf,
        numero: String,
    }

    impl Drop for V1 {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dossier);
        }
    }

    /// Base V1 sur fichier, au schéma 6 de la V1, en clair : entreprise avec
    /// logo, comptes admin et compta, client, fournisseur, produit, facture
    /// validée et payée en partie, textes de relance.
    fn base_v1(nom: &str) -> V1 {
        let dossier =
            std::env::temp_dir().join(format!("kadenz-reprise-{nom}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dossier);
        fs::create_dir_all(&dossier).unwrap();
        let base = dossier.join("kadenz.sqlite");
        let mut conn = Connection::open(&base).unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        for (rang, sql) in crate::MIGRATIONS[..6].iter().enumerate() {
            conn.execute_batch(sql).unwrap();
            conn.pragma_update(None, "user_version", rang as u32 + 1)
                .unwrap();
        }
        crate::licence::preparer(&conn, true).unwrap();
        conn.execute(
            "INSERT INTO entreprise (id, nom, ifu, logo) VALUES (1, 'Quincaillerie V1', 'IFU-V1', x'89504E47')",
            [],
        )
        .unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        compte(&conn, "compta", Role::Comptable);
        let mut client = FicheTiers::vide(TypeTiers::Client);
        client.nom = "Client V1".into();
        let client = tiers::creer(&conn, &admin, &client).unwrap();
        let mut fournisseur = FicheTiers::vide(TypeTiers::Fournisseur);
        fournisseur.nom = "Fournisseur V1".into();
        tiers::creer(&conn, &admin, &fournisseur).unwrap();
        // Au schéma de la V1, sans la colonne du suivi en stock : écrit à la
        // main plutôt que par `produits::creer`.
        conn.execute(
            "INSERT INTO produit (designation, reference, prix, sens) VALUES ('Sable', 'SAB', 1000, 'vente')",
            [],
        )
        .unwrap();
        let produit_id = conn.last_insert_rowid();
        relances::enregistrer_textes(
            &conn,
            &admin,
            &TextesRelance {
                titre: "Relance V1".into(),
                avant: "Texte de la V1".into(),
                ..TextesRelance::default()
            },
        )
        .unwrap();
        let mut s = Saisie::vide(NaiveDate::from_ymd_opt(2026, 11, 2).unwrap());
        s.tiers_id = Some(client.id);
        s.lignes = vec![LigneSaisie {
            produit_id: Some(produit_id),
            designation: "Sable".into(),
            prix: 1000,
            quantite: UNE_UNITE,
            ..LigneSaisie::default()
        }];
        let id = documents::creer_brouillon(&mut conn, &admin, TypeDocument::Facture, &s).unwrap();
        let numero = crate::valider_au_schema_v1(&mut conn, &admin, id);
        reglements::ajouter(
            &mut conn,
            &admin,
            id,
            &SaisieReglement {
                date: NaiveDate::from_ymd_opt(2026, 11, 3).unwrap(),
                montant: 500,
                mode_paiement: "Espèces".into(),
                reference: String::new(),
            },
        )
        .unwrap();
        V1 {
            dossier,
            base,
            numero,
        }
    }

    /// Serveur neuf : entreprise et Admin « Admin » de l'installation.
    fn serveur() -> (Connection, Utilisateur) {
        let mut conn = crate::ouvrir_en_memoire().unwrap();
        let admin = installation::installer(
            &mut conn,
            &InstallationInitiale {
                entreprise: Entreprise {
                    nom: "Serveur neuf".into(),
                    ..Default::default()
                },
                admin: NouvelUtilisateur {
                    nom_complet: "Admin du serveur".into(),
                    identifiant: "Admin".into(),
                    mot_de_passe: "motdepasse-serveur".into(),
                    role: Role::Admin,
                },
                parametres: Parametres::default(),
            },
        )
        .unwrap();
        (conn, admin)
    }

    #[test]
    fn base_v1_reprise_dans_un_serveur_neuf() {
        let v1 = base_v1("neuf");
        let avant = fs::read(&v1.base).unwrap();
        let kzs = v1.dossier.join("reprise.kzs");
        exporter_base_v1(&v1.base, &kzs, MOT_DE_PASSE).unwrap();
        // La base V1 d'origine n'est ni modifiée ni migrée (décision 45).
        assert_eq!(fs::read(&v1.base).unwrap(), avant);
        let version: u32 = Connection::open(&v1.base)
            .unwrap()
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, 6);
        assert!(!v1.dossier.join("reprise.kzs.copie").exists());

        let source = ouvrir_pour_reprise(&kzs, MOT_DE_PASSE).unwrap();
        let (mut serveur, admin) = serveur();
        let entreprise = synchro::identifiant_entreprise(&serveur).unwrap();
        let mut attendu = resume(&source).unwrap();
        separer_comptes(&mut attendu, &serveur).unwrap();
        assert_eq!(attendu.entreprise, "Quincaillerie V1");
        assert_eq!(
            (
                attendu.clients,
                attendu.fournisseurs,
                attendu.produits,
                attendu.reglements
            ),
            (1, 1, 1, 1)
        );
        assert_eq!(
            attendu.documents,
            vec![DocumentsRepris {
                type_document: "facture".into(),
                nombre: 1,
                dernier_numero: Some(v1.numero.clone()),
            }]
        );
        assert_eq!(attendu.comptes_repris, ["compta"]);
        assert_eq!(attendu.comptes_ecartes, ["admin"]);

        let seq_avant = synchro::dernier_changement(&serveur).unwrap() as i64;
        let bilan = reprendre(&mut serveur, &admin, &source).unwrap();
        assert_eq!(bilan, attendu);
        // L'identifiant d'entreprise du serveur est gardé (décision 54).
        assert_eq!(
            synchro::identifiant_entreprise(&serveur).unwrap(),
            entreprise
        );
        let (nom, ifu, logo): (String, String, Vec<u8>) = serveur
            .query_row(
                "SELECT nom, ifu, logo FROM entreprise WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(
            (nom.as_str(), ifu.as_str(), logo),
            ("Quincaillerie V1", "IFU-V1", vec![0x89, 0x50, 0x4E, 0x47])
        );
        assert_eq!(relances::lire_textes(&serveur).unwrap().titre, "Relance V1");
        // La facture et le règlement de la V1 passent au compte « Admin » du
        // serveur, de même identifiant que l'« admin » écarté.
        let (createur, payeur): (i64, i64) = serveur
            .query_row(
                "SELECT d.utilisateur_creation_id, r.utilisateur_id
                 FROM document d JOIN reglement r ON r.document_id = d.id",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((createur, payeur), (admin.id, admin.id));
        // Le compte compta se connecte avec son mot de passe de la V1.
        assert!(crate::comptes::connecter(&serveur, "compta", "motdepasse1").is_ok());

        // Chaque ligne reprise entre dans la suite des changements.
        for (entite, table) in [
            ("tiers", "tiers"),
            ("produit", "produit"),
            ("document", "document"),
            ("reglement", "reglement"),
            ("utilisateur", "utilisateur"),
            ("instantane", "instantane_entreprise"),
        ] {
            let sans: i64 = serveur
                .query_row(
                    &format!(
                        "SELECT count(*) FROM {table} t WHERE NOT EXISTS (
                             SELECT 1 FROM changement c
                             WHERE c.entite = ?1 AND c.uid = t.uid AND c.seq > ?2)"
                    ),
                    params![entite, seq_avant],
                    |r| r.get(0),
                )
                .unwrap();
            // L'Admin du serveur existait avant la reprise.
            let attendu = i64::from(entite == "utilisateur");
            assert_eq!(sans, attendu, "{entite} sans changement");
        }
        for entite in ["entreprise", "parametres"] {
            let n: i64 = serveur
                .query_row(
                    "SELECT count(*) FROM changement WHERE entite = ?1 AND seq > ?2",
                    params![entite, seq_avant],
                    |r| r.get(0),
                )
                .unwrap();
            assert!(n > 0, "{entite} sans changement");
        }

        // Numérotation continue : la facture suivante du même mois prend le
        // rang suivant de la même série.
        let client: i64 = serveur
            .query_row("SELECT id FROM tiers WHERE type = 'client'", [], |r| {
                r.get(0)
            })
            .unwrap();
        let mut s = Saisie::vide(NaiveDate::from_ymd_opt(2026, 11, 20).unwrap());
        s.tiers_id = Some(client);
        s.lignes = vec![LigneSaisie {
            designation: "Gravier".into(),
            prix: 2000,
            quantite: UNE_UNITE,
            ..LigneSaisie::default()
        }];
        let id =
            documents::creer_brouillon(&mut serveur, &admin, TypeDocument::Facture, &s).unwrap();
        let numero = documents::valider(&mut serveur, &admin, id).unwrap();
        let rang = |n: &str| n[n.len() - 4..].parse::<u32>().unwrap();
        assert_eq!(numero[..numero.len() - 4], v1.numero[..v1.numero.len() - 4]);
        assert_eq!(rang(&numero), rang(&v1.numero) + 1);

        // Une seule base par serveur (décision 55).
        let e = reprendre(&mut serveur, &admin, &source).unwrap_err();
        assert!(e.to_string().contains("serveur neuf"), "{e}");
        assert!(!serveur_neuf(&serveur).unwrap());
    }

    #[test]
    fn reprise_refusee_sans_role_admin_ou_avec_un_mot_de_passe_faux() {
        let v1 = base_v1("refus");
        let kzs = v1.dossier.join("reprise.kzs");
        exporter_base_v1(&v1.base, &kzs, MOT_DE_PASSE).unwrap();
        assert!(ouvrir_pour_reprise(&kzs, "mauvais-mot-de-passe").is_err());
        let source = ouvrir_pour_reprise(&kzs, MOT_DE_PASSE).unwrap();
        let (mut serveur, _) = serveur();
        let compta = compte(&serveur, "compta", Role::Comptable);
        assert!(matches!(
            reprendre(&mut serveur, &compta, &source),
            Err(Erreur::Interdit(_))
        ));
        // Des comptes créés après l'installation n'empêchent pas la reprise.
        assert!(serveur_neuf(&serveur).unwrap());
        assert!(exporter_base_v1(&v1.dossier.join("absente.sqlite"), &kzs, MOT_DE_PASSE).is_err());
    }

    /// Base V1 chiffrée par la clé du poste (`kadenz.cle`), au schéma courant.
    #[test]
    fn base_v1_chiffree_exportee_avec_la_cle_du_poste() {
        let dossier =
            std::env::temp_dir().join(format!("kadenz-reprise-chiffree-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dossier);
        let base = dossier.join("kadenz.sqlite");
        {
            let conn = crate::ouvrir(&base).unwrap();
            crate::licence::preparer(&conn, true).unwrap();
            let admin = compte(&conn, "patron", Role::Admin);
            let mut f = FicheTiers::vide(TypeTiers::Client);
            f.nom = "Client chiffré".into();
            tiers::creer(&conn, &admin, &f).unwrap();
        }
        assert!(chiffrement::en_clair(&base).is_ok_and(|clair| !clair));
        let kzs = dossier.join("reprise.kzs");
        exporter_base_v1(&base, &kzs, MOT_DE_PASSE).unwrap();
        let source = ouvrir_pour_reprise(&kzs, MOT_DE_PASSE).unwrap();
        let lu = resume(&source).unwrap();
        assert_eq!(lu.clients, 1);
        assert_eq!(lu.comptes_repris, ["patron"]);
        let _ = fs::remove_dir_all(&dossier);
    }
}
