//! Réplique du serveur sur un poste relié (phase 2, jalon F5 ;
//! `docs/spec-synchronisation.md`, section 6).
//!
//! Le serveur décrit chaque ligne changée par ses valeurs, colonne par colonne
//! ([`etat`]) ; le poste les écrit telles quelles dans sa copie ([`recevoir`]),
//! sans repasser par les règles métier : elles ont déjà été appliquées par le
//! serveur, qui seul numérote. Les références entre lignes voyagent par `uid`.
//!
//! Chaque colonne des tables répliquées est classée : transmise, référence à
//! une autre ligne, contenu binaire, ou propre à chaque base. Un essai fait
//! échouer toute colonne nouvelle qui ne l'est pas, de même pour les clés de
//! `parametre_systeme`.
//!
//! Un changement qui ne peut pas encore s'écrire (référence à une ligne pas
//! encore reçue, valeur unique encore tenue par une ligne qui va changer, ligne
//! supprimée encore citée) est mis en attente avec la page, puis réessayé : le
//! serveur étant cohérent, la ligne qui gêne a toujours un changement plus
//! récent dans la suite. Une fois le poste à jour, plus rien n'attend.

use std::collections::{BTreeMap, BTreeSet};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use kadenz_protocole::{Changement, Entite, Etat, Valeurs};
use rusqlite::types::{Value as ValeurSql, ValueRef};
use rusqlite::{Connection, ErrorCode, OptionalExtension, Transaction, params};
use serde_json::Value;

use crate::{Erreur, Resultat};

/// Clés de `parametre_systeme` partagées par tous les postes. La même liste est
/// dans la migration 0011 (déclencheurs du suivi).
pub const CLES_PARTAGEES: [&str; 19] = [
    "taux.tva",
    "taux.remise",
    "taux.retenue",
    "paiement.modes",
    "lettres.orthographe",
    "delais.validite_devis_jours",
    "delais.paiement_jours",
    "numerotation",
    "documents.textes",
    "documents.couleur",
    "droits.matrice",
    "relance.textes",
    "impression.mise_en_page",
    "impression.bas.devis",
    "impression.bas.proforma",
    "impression.bas.bon_commande",
    "impression.bas.livraison",
    "impression.bas.facture",
    "impression.bas.avoir",
];

/// Clés propres à chaque base, jamais transmises ni écrasées : identité de
/// l'entreprise et du poste, licence V1, abonnement, port du service, dossiers
/// d'export de la machine, suivi de la réplique.
pub const CLES_LOCALES: [&str; 11] = [
    "identifiant_entreprise",
    "poste_relie",
    "licence.cle",
    "abonnement.cle",
    "abonnement.date_vue",
    "abonnement.emission_vue",
    "abonnement.recu",
    "port_serveur",
    "documents.dossiers",
    CLE_DERNIER_RANG,
    CLE_EN_COURS,
];

/// Dernier rang de la suite des changements reçu du serveur (sur un poste).
const CLE_DERNIER_RANG: &str = "synchro.dernier_rang";
/// Opération envoyée au serveur sans réponse reçue (sur un poste).
const CLE_EN_COURS: &str = "synchro.operation_en_cours";

/// Classement des colonnes d'une table répliquée. `id` et `uid` sont à part.
pub(crate) struct Table {
    pub entite: Entite,
    pub nom: &'static str,
    /// Colonnes transmises telles quelles.
    pub colonnes: &'static [&'static str],
    /// Colonnes binaires, transmises en base64.
    pub binaires: &'static [&'static str],
    /// Références à une autre ligne, transmises par `uid`.
    pub references: &'static [(&'static str, Entite)],
    /// Colonnes propres à chaque base : jamais lues ni écrites par la
    /// réplique, listées pour l'essai du classement.
    #[cfg_attr(not(test), allow(dead_code))]
    pub locales: &'static [&'static str],
    /// Colonnes uniques qu'une autre ligne peut tenir le temps de la réplique.
    pub uniques: &'static [&'static str],
}

pub(crate) const TABLES: [Table; 7] = [
    Table {
        entite: Entite::Utilisateur,
        nom: "utilisateur",
        colonnes: &[
            "nom_complet",
            "identifiant",
            "mot_de_passe_hache",
            "role",
            "actif",
            "date_creation",
            "langue",
            "navigation",
            "animations",
        ],
        binaires: &[],
        references: &[],
        locales: &[],
        uniques: &["identifiant"],
    },
    Table {
        entite: Entite::Tiers,
        nom: "tiers",
        colonnes: &[
            "nom",
            "adresse",
            "telephone",
            "localisation",
            "ifu",
            "rccm",
            "regime_imposition",
            "type",
            "actif",
        ],
        binaires: &[],
        references: &[],
        locales: &[],
        uniques: &[],
    },
    Table {
        entite: Entite::Produit,
        nom: "produit",
        colonnes: &["designation", "reference", "prix", "sens", "actif"],
        binaires: &[],
        references: &[],
        locales: &[],
        uniques: &["reference"],
    },
    Table {
        entite: Entite::Instantane,
        nom: "instantane_entreprise",
        colonnes: &["donnees"],
        binaires: &["logo", "logo2"],
        references: &[],
        locales: &[],
        uniques: &[],
    },
    Table {
        entite: Entite::Entreprise,
        nom: "entreprise",
        colonnes: &[
            "nom",
            "adresse",
            "telephone",
            "email",
            "site_web",
            "localisation",
            "regime_imposition",
            "rccm",
            "ifu",
            "banque1",
            "banque2",
            "responsable_facturation",
            "responsable_comptabilite",
        ],
        binaires: &["logo", "logo2"],
        references: &[],
        locales: &[],
        uniques: &[],
    },
    Table {
        entite: Entite::Document,
        nom: "document",
        colonnes: &[
            "type",
            "numero",
            "date",
            "tiers_nom_snapshot",
            "tiers_adresse_snapshot",
            "tiers_ifu_snapshot",
            "tiers_rccm_snapshot",
            "tiers_regime_snapshot",
            "statut",
            "libelle",
            "montant_ht",
            "montant_remise",
            "montant_base_tva",
            "montant_tva",
            "montant_retenue",
            "montant_ttc",
            "pourcentage_remise",
            "pourcentage_tva",
            "pourcentage_retenue",
            "mode_paiement",
            "observations",
            "date_echeance",
            "date_creation",
            "date_validation",
            "date_envoi",
            "nature_source",
            "visa_client",
            "visa_fournisseur",
            "date_livraison_texte",
            "date_reception_texte",
        ],
        binaires: &[],
        references: &[
            ("tiers_id", Entite::Tiers),
            ("document_source_id", Entite::Document),
            ("utilisateur_creation_id", Entite::Utilisateur),
            ("utilisateur_validation_id", Entite::Utilisateur),
            ("instantane_entreprise_id", Entite::Instantane),
        ],
        // Chemin du PDF sur la machine qui l'a produit : sans objet ailleurs.
        locales: &["chemin_pdf"],
        uniques: &[],
    },
    Table {
        entite: Entite::Reglement,
        nom: "reglement",
        colonnes: &["date", "montant", "mode_paiement", "reference"],
        binaires: &[],
        references: &[
            ("document_id", Entite::Document),
            ("utilisateur_id", Entite::Utilisateur),
        ],
        locales: &[],
        uniques: &[],
    },
];

/// Lignes d'un document : elles voyagent avec lui et sont remplacées en bloc.
pub(crate) const LIGNES: Table = Table {
    entite: Entite::Document,
    nom: "ligne_document",
    colonnes: &[
        "reference_snapshot",
        "designation_snapshot",
        "prix_snapshot",
        "quantite",
        "quantite_livree",
        "total",
        "observations",
        "ordre",
    ],
    binaires: &[],
    references: &[("produit_id", Entite::Produit)],
    // `document_id` désigne le document qui les porte.
    locales: &["document_id"],
    uniques: &[],
};

pub(crate) fn table(entite: Entite) -> Option<&'static Table> {
    TABLES.iter().find(|t| t.entite == entite)
}

/// Nom de la table d'une entité (`None` pour les paramètres, rangés par clé).
pub(crate) fn nom_table(entite: Entite) -> Option<&'static str> {
    table(entite).map(|t| t.nom)
}

// ---------------------------------------------------------------------------
// Côté serveur : description d'une ligne.

fn valeur_json(v: ValueRef<'_>) -> Value {
    match v {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => Value::from(i),
        ValueRef::Real(f) => Value::from(f),
        ValueRef::Text(t) => Value::from(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Blob(b) => Value::from(BASE64.encode(b)),
    }
}

fn lire_valeurs(conn: &Connection, t: &Table, filtre: &str, cle: i64) -> Resultat<Vec<Valeurs>> {
    let mut colonnes: Vec<&str> = t.colonnes.to_vec();
    colonnes.extend(t.binaires);
    colonnes.extend(t.references.iter().map(|(c, _)| *c));
    let sql = format!(
        "SELECT {} FROM {} WHERE {filtre} ORDER BY id",
        colonnes.join(", "),
        t.nom
    );
    let mut requete = conn.prepare(&sql)?;
    let brutes: Vec<Vec<(String, Value, Option<Entite>)>> = requete
        .query_map([cle], |r| {
            let mut ligne = Vec::with_capacity(colonnes.len());
            for (i, c) in colonnes.iter().enumerate() {
                let reference = t.references.iter().find(|(nom, _)| nom == c).map(|r| r.1);
                ligne.push((c.to_string(), valeur_json(r.get_ref(i)?), reference));
            }
            Ok(ligne)
        })?
        .collect::<Result<_, _>>()?;
    let mut sortie = Vec::with_capacity(brutes.len());
    for brute in brutes {
        let mut valeurs = Valeurs::new();
        for (colonne, valeur, reference) in brute {
            let valeur = match (reference, valeur.as_i64()) {
                (Some(entite), Some(id)) => crate::synchro::uid_de(conn, entite, id)?
                    .map(Value::from)
                    .unwrap_or(Value::Null),
                _ => valeur,
            };
            valeurs.insert(colonne, valeur);
        }
        sortie.push(valeurs);
    }
    Ok(sortie)
}

/// État présent d'une ligne, ou `None` si elle n'existe plus.
pub(crate) fn etat(conn: &Connection, entite: Entite, id: i64) -> Resultat<Option<Etat>> {
    if entite == Entite::Parametres {
        let mut valeurs = Valeurs::new();
        let marques = CLES_PARTAGEES.map(|_| "?").join(", ");
        let mut requete = conn.prepare(&format!(
            "SELECT cle, valeur FROM parametre_systeme WHERE cle IN ({marques}) ORDER BY cle"
        ))?;
        let lignes: Vec<(String, String)> = requete
            .query_map(rusqlite::params_from_iter(CLES_PARTAGEES), |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?
            .collect::<Result<_, _>>()?;
        for (cle, valeur) in lignes {
            let valeur = serde_json::from_str(&valeur).unwrap_or(Value::Null);
            valeurs.insert(cle, valeur);
        }
        return Ok(Some(Etat::Ligne {
            valeurs,
            lignes: Vec::new(),
        }));
    }
    let Some(t) = table(entite) else {
        return Ok(None);
    };
    let Some(valeurs) = lire_valeurs(conn, t, "id = ?1", id)?.into_iter().next() else {
        return Ok(None);
    };
    let lignes = if entite == Entite::Document {
        lire_valeurs(conn, &LIGNES, "document_id = ?1", id)?
    } else {
        Vec::new()
    };
    Ok(Some(Etat::Ligne { valeurs, lignes }))
}

// ---------------------------------------------------------------------------
// Côté poste : écriture dans la copie.

/// Bilan d'une page reçue.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reception {
    /// Changements écrits dans la copie (ceux de la page et ceux qui attendaient).
    pub appliques: usize,
    /// Changements encore en attente après la page.
    pub en_attente: usize,
    /// Entités touchées, pour que l'écran recharge ce qui est ouvert.
    pub sortes: BTreeSet<Entite>,
    /// Dernier rang reçu, gardé dans la copie.
    pub dernier: u64,
}

/// Échec d'un changement : à réessayer plus tard, ou erreur de la base.
enum Echec {
    Attente,
    Base(Erreur),
}

impl From<rusqlite::Error> for Echec {
    fn from(e: rusqlite::Error) -> Echec {
        match &e {
            rusqlite::Error::SqliteFailure(f, _) if f.code == ErrorCode::ConstraintViolation => {
                Echec::Attente
            }
            _ => Echec::Base(Erreur::Sqlite(e)),
        }
    }
}

impl From<Erreur> for Echec {
    fn from(e: Erreur) -> Echec {
        match e {
            Erreur::Sqlite(e) => Echec::from(e),
            autre => Echec::Base(autre),
        }
    }
}

fn valeur_sql(
    conn: &Connection,
    t: &Table,
    colonne: &str,
    valeur: Option<&Value>,
) -> Result<ValeurSql, Echec> {
    let valeur = valeur.unwrap_or(&Value::Null);
    if let Some((_, entite)) = t.references.iter().find(|(c, _)| *c == colonne) {
        return match valeur.as_str() {
            None => Ok(ValeurSql::Null),
            // Ligne citée pas encore reçue : le changement attend.
            Some(uid) => crate::synchro::id_de(conn, *entite, uid)?
                .map(ValeurSql::Integer)
                .ok_or(Echec::Attente),
        };
    }
    Ok(match valeur {
        Value::Null => ValeurSql::Null,
        Value::Bool(b) => ValeurSql::Integer(i64::from(*b)),
        Value::Number(n) => match n.as_i64() {
            Some(i) => ValeurSql::Integer(i),
            None => ValeurSql::Real(n.as_f64().unwrap_or_default()),
        },
        Value::String(s) if t.binaires.contains(&colonne) => {
            ValeurSql::Blob(BASE64.decode(s).map_err(|_| Echec::Attente)?)
        }
        Value::String(s) => ValeurSql::Text(s.clone()),
        autre => ValeurSql::Text(autre.to_string()),
    })
}

fn colonnes_ecrites(t: &Table) -> Vec<&'static str> {
    let mut colonnes: Vec<&str> = t.colonnes.to_vec();
    colonnes.extend(t.binaires);
    colonnes.extend(t.references.iter().map(|(c, _)| *c));
    colonnes
}

/// Écrit une ligne par son `uid` (ou la ligne unique de l'entreprise) ; rend
/// son identifiant local.
fn ecrire_ligne(conn: &Connection, t: &Table, uid: &str, valeurs: &Valeurs) -> Result<i64, Echec> {
    let colonnes = colonnes_ecrites(t);
    let mut parametres = Vec::with_capacity(colonnes.len() + 1);
    for c in &colonnes {
        parametres.push(valeur_sql(conn, t, c, valeurs.get(*c))?);
    }
    let maj = colonnes
        .iter()
        .map(|c| format!("{c} = excluded.{c}"))
        .collect::<Vec<_>>()
        .join(", ");
    let marques = (1..=colonnes.len() + 1)
        .map(|i| format!("?{i}"))
        .collect::<Vec<_>>()
        .join(", ");
    if t.entite == Entite::Entreprise {
        parametres.insert(0, ValeurSql::Integer(1));
        conn.execute(
            &format!(
                "INSERT INTO entreprise (id, {}) VALUES ({marques})
                 ON CONFLICT (id) DO UPDATE SET {maj}",
                colonnes.join(", ")
            ),
            rusqlite::params_from_iter(parametres),
        )?;
        return Ok(1);
    }
    parametres.insert(0, ValeurSql::Text(uid.to_string()));
    conn.execute(
        &format!(
            "INSERT INTO {} (uid, {}) VALUES ({marques})
             ON CONFLICT (uid) DO UPDATE SET {maj}",
            t.nom,
            colonnes.join(", ")
        ),
        rusqlite::params_from_iter(parametres),
    )?;
    Ok(conn.query_row(
        &format!("SELECT id FROM {} WHERE uid = ?1", t.nom),
        [uid],
        |r| r.get(0),
    )?)
}

fn ecrire_lignes(conn: &Connection, document: i64, lignes: &[Valeurs]) -> Result<(), Echec> {
    conn.execute(
        "DELETE FROM ligne_document WHERE document_id = ?1",
        [document],
    )?;
    let colonnes = colonnes_ecrites(&LIGNES);
    let marques = (1..=colonnes.len() + 1)
        .map(|i| format!("?{i}"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "INSERT INTO ligne_document (document_id, {}) VALUES ({marques})",
        colonnes.join(", ")
    );
    for ligne in lignes {
        let mut parametres = vec![ValeurSql::Integer(document)];
        for c in &colonnes {
            parametres.push(valeur_sql(conn, &LIGNES, c, ligne.get(*c))?);
        }
        conn.execute(&sql, rusqlite::params_from_iter(parametres))?;
    }
    Ok(())
}

fn ecrire_parametres(conn: &Connection, valeurs: &Valeurs) -> Result<(), Echec> {
    for cle in CLES_PARTAGEES {
        match valeurs.get(cle) {
            Some(valeur) => conn.execute(
                "INSERT INTO parametre_systeme (cle, valeur) VALUES (?1, ?2)
                 ON CONFLICT (cle) DO UPDATE SET valeur = excluded.valeur",
                params![cle, valeur.to_string()],
            )?,
            None => conn.execute("DELETE FROM parametre_systeme WHERE cle = ?1", [cle])?,
        };
    }
    Ok(())
}

fn appliquer_un(conn: &Connection, ch: &Changement) -> Result<(), Echec> {
    match (&ch.etat, ch.entite) {
        (Etat::Ligne { valeurs, .. }, Entite::Parametres) => ecrire_parametres(conn, valeurs),
        (Etat::Ligne { valeurs, lignes }, entite) => {
            let t = table(entite).ok_or(Echec::Attente)?;
            let id = ecrire_ligne(conn, t, &ch.uid, valeurs)?;
            if entite == Entite::Document {
                ecrire_lignes(conn, id, lignes)?;
            }
            Ok(())
        }
        (Etat::Supprime, entite) => match table(entite) {
            Some(t) if !entite.unique() => {
                conn.execute(&format!("DELETE FROM {} WHERE uid = ?1", t.nom), [&ch.uid])?;
                Ok(())
            }
            _ => Ok(()),
        },
    }
}

/// Ordre d'écriture : les lignes citées avant celles qui les citent, les
/// suppressions à la fin dans l'ordre inverse.
fn rang(ch: &Changement) -> (u8, u8, u64) {
    let ordre = match ch.entite {
        Entite::Utilisateur => 0,
        Entite::Tiers => 1,
        Entite::Produit => 2,
        Entite::Instantane => 3,
        Entite::Entreprise => 4,
        Entite::Parametres => 5,
        Entite::Document => 6,
        Entite::Reglement => 7,
    };
    match ch.etat {
        Etat::Ligne { .. } => (0, ordre, ch.seq),
        Etat::Supprime => (1, 7 - ordre, ch.seq),
    }
}

/// Une valeur unique voulue par un changement en attente est tenue par une
/// autre ligne, elle-même en attente d'un changement : cette ligne prend son
/// `uid` comme valeur provisoire, que son propre changement remplacera.
fn liberer_uniques(
    tx: &Transaction<'_>,
    attente: &BTreeMap<(Entite, String), Changement>,
) -> Resultat<bool> {
    let mut libere = false;
    for ch in attente.values() {
        let (Etat::Ligne { valeurs, .. }, Some(t)) = (&ch.etat, table(ch.entite)) else {
            continue;
        };
        for colonne in t.uniques {
            let Some(voulue) = valeurs.get(*colonne).and_then(Value::as_str) else {
                continue;
            };
            let tenue: Option<String> = tx
                .query_row(
                    &format!(
                        "SELECT uid FROM {} WHERE {colonne} = ?1 AND uid IS NOT ?2",
                        t.nom
                    ),
                    params![voulue, ch.uid],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(autre) = tenue
                && attente.contains_key(&(ch.entite, autre.clone()))
            {
                tx.execute(
                    &format!("UPDATE {} SET {colonne} = uid WHERE uid = ?1", t.nom),
                    [&autre],
                )?;
                libere = true;
            }
        }
    }
    Ok(libere)
}

fn lire_attente(tx: &Transaction<'_>) -> Resultat<BTreeMap<(Entite, String), Changement>> {
    let mut requete = tx.prepare("SELECT entite, uid, seq, etat FROM synchro_attente")?;
    let lignes: Vec<(String, String, i64, String)> = requete
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<Result<_, _>>()?;
    let mut attente = BTreeMap::new();
    for (code, uid, seq, etat) in lignes {
        let (Some(entite), Ok(etat)) = (Entite::depuis_code(&code), serde_json::from_str(&etat))
        else {
            continue;
        };
        attente.insert(
            (entite, uid.clone()),
            Changement {
                seq: seq as u64,
                entite,
                uid,
                etat,
            },
        );
    }
    Ok(attente)
}

/// Écrit une page de changements reçue du serveur dans la copie du poste, avec
/// ceux qui attendaient, et garde le dernier rang reçu : tout ou rien.
pub fn recevoir(conn: &mut Connection, page: &[Changement]) -> Resultat<Reception> {
    let mut tx = conn.transaction()?;
    let avant = crate::synchro::dernier_changement(&tx)?;
    let mut attente = lire_attente(&tx)?;
    let mut dernier = dernier_recu(&tx)?;
    for ch in page {
        dernier = dernier.max(ch.seq);
        let cle = (ch.entite, ch.uid.clone());
        if attente.get(&cle).is_none_or(|ancien| ancien.seq <= ch.seq) {
            attente.insert(cle, ch.clone());
        }
    }
    let mut bilan = Reception::default();
    loop {
        let mut ordre: Vec<(Entite, String)> = attente.keys().cloned().collect();
        ordre.sort_by_key(|cle| rang(&attente[cle]));
        let mut progres = false;
        for cle in ordre {
            let ch = &attente[&cle];
            let point = tx.savepoint()?;
            match appliquer_un(&point, ch) {
                Ok(()) => {
                    point.commit()?;
                    bilan.sortes.insert(ch.entite);
                    bilan.appliques += 1;
                    attente.remove(&cle);
                    progres = true;
                }
                // Le point de sauvegarde est annulé en sortant.
                Err(Echec::Attente) => {}
                Err(Echec::Base(e)) => return Err(e),
            }
        }
        if attente.is_empty() || (!progres && !liberer_uniques(&tx, &attente)?) {
            break;
        }
    }
    tx.execute("DELETE FROM synchro_attente", [])?;
    for ch in attente.values() {
        let etat =
            serde_json::to_string(&ch.etat).map_err(|e| Erreur::Validation(e.to_string()))?;
        tx.execute(
            "INSERT INTO synchro_attente (entite, uid, seq, etat) VALUES (?1, ?2, ?3, ?4)",
            params![ch.entite.code(), ch.uid, ch.seq as i64, etat],
        )?;
    }
    // Les écritures de la réplique ne sont pas des changements de ce poste.
    tx.execute("DELETE FROM changement WHERE seq > ?1", [avant as i64])?;
    tx.execute(
        "INSERT INTO parametre_systeme (cle, valeur) VALUES (?1, ?2)
         ON CONFLICT (cle) DO UPDATE SET valeur = excluded.valeur",
        params![CLE_DERNIER_RANG, dernier.to_string()],
    )?;
    tx.commit()?;
    bilan.en_attente = attente.len();
    bilan.dernier = dernier;
    Ok(bilan)
}

/// Garde (ou oublie, avec `None`) l'opération envoyée au serveur dont la
/// réponse n'est pas encore reçue : elle sera renvoyée avec le même
/// identifiant, et le serveur rendra le résultat d'origine.
pub fn garder_en_cours(
    conn: &Connection,
    op: Option<&kadenz_protocole::Operation>,
) -> Resultat<()> {
    match op {
        Some(op) => {
            let texte = serde_json::to_string(op).map_err(|e| Erreur::Validation(e.to_string()))?;
            conn.execute(
                "INSERT INTO parametre_systeme (cle, valeur) VALUES (?1, ?2)
                 ON CONFLICT (cle) DO UPDATE SET valeur = excluded.valeur",
                params![CLE_EN_COURS, texte],
            )?;
        }
        None => {
            conn.execute(
                "DELETE FROM parametre_systeme WHERE cle = ?1",
                [CLE_EN_COURS],
            )?;
        }
    }
    Ok(())
}

/// Opération envoyée au serveur sans réponse reçue, s'il y en a une.
pub fn operation_en_cours(conn: &Connection) -> Resultat<Option<kadenz_protocole::Operation>> {
    let texte: Option<String> = conn
        .query_row(
            "SELECT valeur FROM parametre_systeme WHERE cle = ?1",
            [CLE_EN_COURS],
            |r| r.get(0),
        )
        .optional()?;
    Ok(texte.and_then(|t| serde_json::from_str(&t).ok()))
}

/// Dernier rang reçu du serveur par ce poste (0 avant la première page).
pub fn dernier_recu(conn: &Connection) -> Resultat<u64> {
    Ok(conn
        .query_row(
            "SELECT valeur FROM parametre_systeme WHERE cle = ?1",
            [CLE_DERNIER_RANG],
            |r| r.get::<_, String>(0),
        )
        .optional()?
        .and_then(|v| v.parse().ok())
        .unwrap_or(0))
}

/// Nombre de changements reçus qui attendent encore de pouvoir s'écrire. Il doit
/// être nul une fois le poste à jour.
pub fn en_attente(conn: &Connection) -> Resultat<u64> {
    Ok(
        conn.query_row("SELECT count(*) FROM synchro_attente", [], |r| {
            r.get::<_, i64>(0)
        })? as u64,
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::comptes::Utilisateur;
    use crate::documents::{self, ChampsLivraison, LigneSaisie, Saisie};
    use crate::droits::tests::compte;
    use crate::entreprise::{self, Entreprise, Logo};
    use crate::reglements::{self, SaisieReglement};
    use crate::synchro::{changements_depuis, completer_uids, uid_de};
    use crate::{ouvrir_en_memoire, parametres, produits, tiers};
    use chrono::NaiveDate;
    use kadenz_domaine::calcul::Taux;
    use kadenz_domaine::document::TypeDocument;
    use kadenz_domaine::referentiel::{FicheProduit, FicheTiers, Sens, TypeTiers};
    use kadenz_domaine::role::Role;

    const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0];

    fn colonnes_de(conn: &Connection, table: &str) -> BTreeSet<String> {
        conn.prepare(&format!("SELECT name FROM pragma_table_info('{table}')"))
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    #[test]
    fn chaque_colonne_est_classee() {
        let conn = ouvrir_en_memoire().unwrap();
        for t in TABLES.iter().chain([&LIGNES]) {
            let mut classees: BTreeSet<String> = ["id", "uid"].map(String::from).into();
            for c in t.colonnes.iter().chain(t.binaires).chain(t.locales) {
                assert!(
                    classees.insert(c.to_string()),
                    "{} : {c} classée deux fois",
                    t.nom
                );
            }
            for (c, _) in t.references {
                assert!(
                    classees.insert(c.to_string()),
                    "{} : {c} classée deux fois",
                    t.nom
                );
            }
            for c in colonnes_de(&conn, t.nom) {
                assert!(
                    classees.contains(&c),
                    "colonne {}.{c} à classer dans replique.rs (transmise, référence, binaire ou locale)",
                    t.nom
                );
            }
        }
    }

    #[test]
    fn chaque_cle_est_classee() {
        // Les déclencheurs de la migration portent la même liste.
        let migration = include_str!("migrations/0011_parametres_bas.sql");
        for cle in CLES_PARTAGEES {
            assert_eq!(migration.matches(&format!("'{cle}'")).count(), 3, "{cle}");
            assert!(!CLES_LOCALES.contains(&cle), "{cle}");
        }
        // Toute clé écrite par une base remplie est classée.
        let mut conn = ouvrir_en_memoire().unwrap();
        let r = remplir(&mut conn);
        regler(&conn, &r.admin);
        crate::abonnement::garder(
            &conn,
            &kadenz_protocole::Abonnement {
                statut: kadenz_protocole::abonnement::Statut::Absent,
                aujourdhui: "2026-11-12".into(),
                jours_restants: None,
                avertissement: None,
                postes_utilises: 0,
                licence: None,
                erreur: None,
            },
        )
        .unwrap();
        let cles: Vec<String> = conn
            .prepare("SELECT cle FROM parametre_systeme")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        for cle in cles {
            assert!(
                CLES_PARTAGEES.contains(&cle.as_str()) || CLES_LOCALES.contains(&cle.as_str()),
                "clé {cle} à classer dans replique.rs (partagée ou locale)"
            );
        }
    }

    fn fiche_tiers(nom: &str) -> FicheTiers {
        FicheTiers {
            nom: nom.into(),
            adresse: String::new(),
            telephone: String::new(),
            localisation: String::new(),
            ifu: String::new(),
            rccm: String::new(),
            regime_imposition: String::new(),
            type_tiers: TypeTiers::Client,
        }
    }

    fn fiche_produit(reference: &str) -> FicheProduit {
        FicheProduit {
            designation: format!("Produit {reference}"),
            reference: reference.into(),
            prix: 5000,
            sens: Sens::Vente,
        }
    }

    fn saisie(tiers: i64, produit: i64) -> Saisie {
        Saisie {
            date: NaiveDate::from_ymd_opt(2026, 11, 12).unwrap(),
            tiers_id: Some(tiers),
            libelle: "Essai".into(),
            taux: Taux::default(),
            mode_paiement: String::new(),
            observations: String::new(),
            date_echeance: None,
            lignes: vec![LigneSaisie {
                produit_id: Some(produit),
                reference: "A".into(),
                designation: "Produit A".into(),
                prix: 5000,
                quantite: 2_000,
                ..LigneSaisie::default()
            }],
            livraison: ChampsLivraison::default(),
        }
    }

    /// Tous les réglages partagés écrits au moins une fois.
    fn regler(conn: &Connection, admin: &Utilisateur) {
        let m = crate::mise_en_page::lire(conn).unwrap();
        crate::mise_en_page::enregistrer(conn, admin, &m).unwrap();
        for t in TypeDocument::TOUS {
            let b = crate::mise_en_page::lire_bas(conn, t).unwrap();
            crate::mise_en_page::enregistrer_bas(conn, admin, t, &b).unwrap();
        }
        let matrice = crate::droits::lire_matrice(conn).unwrap();
        crate::droits::enregistrer_matrice(conn, admin, &matrice).unwrap();
        let textes = crate::relances::lire_textes(conn).unwrap();
        crate::relances::enregistrer_textes(conn, admin, &textes).unwrap();
    }

    pub(crate) struct Remplie {
        pub admin: Utilisateur,
        pub t1: i64,
        pub p1: i64,
        pub p2: i64,
        pub facture: i64,
        pub brouillon: i64,
    }

    /// Base de serveur avec de tout : comptes, entreprise et logo, paramètres,
    /// tiers, produits, devis converti en facture validée, règlement, brouillon.
    pub(crate) fn remplir(conn: &mut Connection) -> Remplie {
        let admin = compte(conn, "admin", Role::Admin);
        compte(conn, "vendeur", Role::Commercial);
        entreprise::enregistrer(
            conn,
            &admin,
            &Entreprise {
                nom: "Entreprise Essai".into(),
                ..Default::default()
            },
        )
        .unwrap();
        entreprise::definir_logo(conn, &admin, Logo::Principal, Some(PNG)).unwrap();
        let p = parametres::lire(conn).unwrap();
        parametres::enregistrer(conn, &admin, &p).unwrap();
        let t1 = tiers::creer(conn, &admin, &fiche_tiers("Client Alpha"))
            .unwrap()
            .id;
        let t2 = tiers::creer(conn, &admin, &fiche_tiers("Client Beta"))
            .unwrap()
            .id;
        let p1 = produits::creer(conn, &admin, &fiche_produit("A"))
            .unwrap()
            .id;
        let p2 = produits::creer(conn, &admin, &fiche_produit("B"))
            .unwrap()
            .id;
        let devis =
            documents::creer_brouillon(conn, &admin, TypeDocument::Devis, &saisie(t1, p1)).unwrap();
        documents::valider(conn, &admin, devis).unwrap();
        let facture = documents::convertir_en_facture(conn, &admin, devis).unwrap();
        documents::valider(conn, &admin, facture).unwrap();
        reglements::ajouter(
            conn,
            &admin,
            facture,
            &SaisieReglement {
                date: chrono::Local::now().date_naive(),
                montant: 1000,
                mode_paiement: "Espèces".into(),
                reference: String::new(),
            },
        )
        .unwrap();
        let brouillon =
            documents::creer_brouillon(conn, &admin, TypeDocument::Facture, &saisie(t2, p2))
                .unwrap();
        Remplie {
            admin,
            t1,
            p1,
            p2,
            facture,
            brouillon,
        }
    }

    /// Contenu comparable d'une base : chaque ligne répliquée par son `uid`,
    /// références en `uid`, colonnes locales exclues.
    pub(crate) fn vidage(conn: &Connection) -> BTreeMap<(Entite, String), Etat> {
        completer_uids(conn).unwrap();
        let mut sortie = BTreeMap::new();
        for t in &TABLES {
            let ids: Vec<i64> = conn
                .prepare(&format!("SELECT id FROM {}", t.nom))
                .unwrap()
                .query_map([], |r| r.get(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap();
            for id in ids {
                let uid = uid_de(conn, t.entite, id).unwrap().unwrap();
                sortie.insert((t.entite, uid), etat(conn, t.entite, id).unwrap().unwrap());
            }
        }
        sortie.insert(
            (Entite::Parametres, "parametres".into()),
            etat(conn, Entite::Parametres, 1).unwrap().unwrap(),
        );
        sortie
    }

    /// Rejoue sur le poste les changements du serveur, par pages de `limite`.
    pub(crate) fn synchroniser(serveur: &Connection, poste: &mut Connection, limite: u32) {
        loop {
            let page = changements_depuis(serveur, dernier_recu(poste).unwrap(), limite).unwrap();
            if page.is_empty() {
                break;
            }
            recevoir(poste, &page).unwrap();
        }
        assert_eq!(
            en_attente(poste).unwrap(),
            0,
            "changements restés en attente"
        );
        assert_eq!(vidage(poste), vidage(serveur));
    }

    #[test]
    fn replique_complete_puis_suivi() {
        for limite in [1, 2, 3, 500] {
            let mut serveur = ouvrir_en_memoire().unwrap();
            let r = remplir(&mut serveur);
            let mut poste = ouvrir_en_memoire().unwrap();
            synchroniser(&serveur, &mut poste, limite);
            // Le numéro attribué par le serveur est celui de la copie.
            let numero = documents::lire(&serveur, r.facture)
                .unwrap()
                .unwrap()
                .numero;
            assert!(numero.is_some());

            // Tiers modifié après ses documents ; références de deux produits
            // échangées ; brouillon supprimé ; second règlement ; clé partagée
            // retirée ; compte désactivé ; tiers créé puis retiré.
            let mut f = fiche_tiers("Client Alpha modifié");
            f.adresse = "Ouagadougou".into();
            tiers::modifier(&serveur, &r.admin, r.t1, &f).unwrap();
            produits::modifier(&serveur, &r.admin, r.p1, &fiche_produit("C")).unwrap();
            produits::modifier(&serveur, &r.admin, r.p2, &fiche_produit("A")).unwrap();
            produits::modifier(&serveur, &r.admin, r.p1, &fiche_produit("B")).unwrap();
            documents::supprimer_brouillon(&mut serveur, &r.admin, r.brouillon).unwrap();
            reglements::ajouter(
                &mut serveur,
                &r.admin,
                r.facture,
                &SaisieReglement {
                    date: chrono::Local::now().date_naive(),
                    montant: 500,
                    mode_paiement: "Espèces".into(),
                    reference: "R2".into(),
                },
            )
            .unwrap();
            serveur
                .execute("DELETE FROM parametre_systeme WHERE cle = 'taux.tva'", [])
                .unwrap();
            serveur
                .execute(
                    "UPDATE utilisateur SET actif = 0 WHERE identifiant = 'vendeur'",
                    [],
                )
                .unwrap();
            let t3 = tiers::creer(&serveur, &r.admin, &fiche_tiers("Client Gamma"))
                .unwrap()
                .id;
            tiers::retirer(&serveur, &r.admin, t3).unwrap();
            synchroniser(&serveur, &mut poste, limite);
            // Les écritures de la réplique ne sont pas des changements du poste.
            assert_eq!(crate::synchro::dernier_changement(&poste).unwrap(), 0);
        }
    }

    #[test]
    fn valeur_unique_echangee_dans_une_meme_page() {
        let mut serveur = ouvrir_en_memoire().unwrap();
        let r = remplir(&mut serveur);
        let mut poste = ouvrir_en_memoire().unwrap();
        synchroniser(&serveur, &mut poste, 500);
        // Échange direct des références par une valeur provisoire : les deux
        // produits arrivent dans la même page, chacun voulant la valeur tenue
        // par l'autre dans la copie.
        produits::modifier(&serveur, &r.admin, r.p1, &fiche_produit("X")).unwrap();
        produits::modifier(&serveur, &r.admin, r.p2, &fiche_produit("A")).unwrap();
        produits::modifier(&serveur, &r.admin, r.p1, &fiche_produit("B")).unwrap();
        synchroniser(&serveur, &mut poste, 500);
    }

    #[test]
    fn pages_bornees_en_octets() {
        let mut serveur = ouvrir_en_memoire().unwrap();
        let r = remplir(&mut serveur);
        // Deux logos de la taille maximale : l'entreprise et chaque instantané
        // figé ensuite pèsent près de 5,4 Mo en base64.
        let mut gros = vec![0u8; entreprise::TAILLE_MAX_LOGO];
        gros[..PNG.len()].copy_from_slice(PNG);
        for logo in [Logo::Principal, Logo::Secondaire] {
            entreprise::definir_logo(&serveur, &r.admin, logo, Some(&gros)).unwrap();
        }
        documents::valider(&mut serveur, &r.admin, r.brouillon).unwrap();
        let mut depuis = 0;
        let mut pages = 0;
        loop {
            let page = changements_depuis(&serveur, depuis, 500).unwrap();
            let Some(dernier) = page.last() else { break };
            depuis = dernier.seq;
            pages += 1;
            let json = serde_json::to_vec(&page).unwrap();
            assert!(json.len() <= kadenz_protocole::messages::TAILLE_MAX - 1024);
        }
        assert!(pages >= 2, "{pages} page(s)");
        let mut poste = ouvrir_en_memoire().unwrap();
        synchroniser(&serveur, &mut poste, 500);
    }

    #[test]
    fn page_avec_reference_a_venir_attend() {
        let mut serveur = ouvrir_en_memoire().unwrap();
        let r = remplir(&mut serveur);
        let mut poste = ouvrir_en_memoire().unwrap();
        // Le tiers est modifié en dernier : son rang passe après la facture.
        tiers::modifier(&serveur, &r.admin, r.t1, &fiche_tiers("Alpha")).unwrap();
        let tout = changements_depuis(&serveur, 0, 500).unwrap();
        let rang_facture = tout
            .iter()
            .position(|c| c.entite == Entite::Document)
            .unwrap();
        // Première page coupée avant le tiers : la facture attend.
        let coupe = tout
            .iter()
            .position(|c| {
                c.entite == Entite::Tiers
                    && c.uid == uid_de(&serveur, Entite::Tiers, r.t1).unwrap().unwrap()
            })
            .unwrap();
        assert!(rang_facture < coupe);
        let bilan = recevoir(&mut poste, &tout[..coupe]).unwrap();
        assert!(bilan.en_attente > 0);
        assert_eq!(en_attente(&poste).unwrap(), bilan.en_attente as u64);
        // La suite débloque tout.
        let bilan = recevoir(&mut poste, &tout[coupe..]).unwrap();
        assert_eq!(bilan.en_attente, 0);
        assert!(bilan.sortes.contains(&Entite::Document));
        assert_eq!(vidage(&poste), vidage(&serveur));
    }
}
