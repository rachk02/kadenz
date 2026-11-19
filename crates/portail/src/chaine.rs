//! Chaîne de hachage du journal (relecture d'OpenCode, défaut 12) : chaque
//! entrée reçoit une empreinte SHA-256 qui couvre ses champs et l'empreinte
//! de l'entrée précédente. Modifier, retirer ou intercaler une entrée rompt
//! la chaîne à cet endroit.
//!
//! Limite : quelqu'un qui peut écrire dans la base peut aussi recalculer
//! toute la chaîne. La chaîne ne prouve donc l'historique que jusqu'à la
//! dernière empreinte notée hors du serveur (export du journal, sceau relevé
//! sur la page du journal) : une entrée antérieure à ce sceau ne peut plus
//! être changée sans que la vérification le voie.
//!
//! Les empreintes sont dans la table `journal_chaine` (schéma 6), en ajout
//! seul comme le journal ; `precedente` est unique, si bien que deux entrées
//! ne peuvent pas suivre la même (deux écritures concurrentes échouent au
//! lieu de faire bifurquer la chaîne).
//!
//! Archivage (audit d'OpenCode, lot C, schéma 11) : les entrées anciennes,
//! déjà exportées hors du serveur et vérifiées avec la base, sont retirées
//! de la base ; la table `journal_depart` garde la dernière entrée retirée
//! et son empreinte, d'où la chaîne repart. Les entrées retirées ne vivent
//! plus que dans l'export (`journal.jsonl` des sauvegardes).

use std::collections::{HashMap, HashSet};

use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use sha2::{Digest, Sha256};

/// Empreinte qui précède la première entrée.
pub(crate) const ORIGINE: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Séparation de domaine : cette empreinte ne sert à rien d'autre.
const DOMAINE: &[u8] = b"kadenz-portail-journal-1";

/// État de la chaîne du journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EtatChaine {
    /// Entrées vérifiées, depuis la première.
    pub entrees: i64,
    /// Empreinte de la dernière entrée vérifiée : le sceau à relever.
    pub sceau: String,
    /// Première entrée où la chaîne est rompue (identifiant du journal).
    pub rupture: Option<i64>,
    /// Dernière entrée archivée (retirée de la base), d'où la chaîne de la
    /// base repart ; `None` si rien n'a été archivé.
    pub depart: Option<i64>,
}

/// Résultat d'un archivage du journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Archivage {
    /// Dernière entrée retirée de la base (départ de la chaîne).
    pub jusqua: i64,
    /// Entrées retirées par cet archivage (0 : rien de plus à archiver).
    pub retirees: i64,
}

/// Entrée du journal avec son empreinte, pour l'export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntreeChainee {
    pub id: i64,
    pub heure: String,
    pub compte: Option<i64>,
    pub action: String,
    pub cible: Option<String>,
    pub details: String,
    pub precedente: i64,
    pub empreinte: String,
}

/// Champ encodé sans ambiguïté : absent, ou présent avec sa longueur.
fn champ(h: &mut Sha256, valeur: Option<&[u8]>) {
    match valeur {
        None => h.update([0u8]),
        Some(v) => {
            h.update([1u8]);
            h.update((v.len() as u64).to_le_bytes());
            h.update(v);
        }
    }
}

/// Empreinte d'une entrée, à la suite de l'empreinte `precedente`.
pub(crate) fn empreinte(
    precedente: &str,
    id: i64,
    heure: &str,
    compte: Option<i64>,
    action: &str,
    cible: Option<&str>,
    details: &str,
) -> String {
    let mut h = Sha256::new();
    h.update(DOMAINE);
    champ(&mut h, Some(precedente.as_bytes()));
    champ(&mut h, Some(&id.to_le_bytes()));
    champ(&mut h, Some(heure.as_bytes()));
    let compte = compte.map(i64::to_le_bytes);
    champ(&mut h, compte.as_ref().map(|c| c.as_slice()));
    champ(&mut h, Some(action.as_bytes()));
    champ(&mut h, cible.map(str::as_bytes));
    champ(&mut h, Some(details.as_bytes()));
    h.finalize().iter().map(|o| format!("{o:02x}")).collect()
}

/// Dernière entrée archivée et son empreinte, s'il y en a une.
fn depart(conn: &Connection) -> rusqlite::Result<Option<(i64, String)>> {
    // Avant le schéma 11 (chaîne posée par la migration vers le schéma 6),
    // la table n'existe pas : rien n'a pu être archivé.
    let table: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'journal_depart'",
        [],
        |r| r.get(0),
    )?;
    if table == 0 {
        return Ok(None);
    }
    conn.query_row(
        "SELECT id, empreinte FROM journal_depart ORDER BY id DESC LIMIT 1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()
}

/// Point de départ de la chaîne de la base : la dernière entrée archivée,
/// ou 0 et l'origine.
fn depart_ou_origine(conn: &Connection) -> rusqlite::Result<(i64, String)> {
    Ok(depart(conn)?.unwrap_or((0, ORIGINE.to_string())))
}

/// Dernier maillon : identifiant et empreinte ; sans maillon, le point de
/// départ (tout archivé, ou journal vide).
fn dernier(conn: &Connection) -> rusqlite::Result<(i64, String)> {
    match conn
        .query_row(
            "SELECT id, empreinte FROM journal_chaine ORDER BY id DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
    {
        Some(d) => Ok(d),
        None => depart_ou_origine(conn),
    }
}

/// Ajoute à la chaîne l'entrée `id` du journal, qui vient d'être écrite.
pub(crate) fn chainer(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    let (precedente, empreinte_precedente) = dernier(conn)?;
    let (heure, compte, action, cible, details): (
        String,
        Option<i64>,
        String,
        Option<String>,
        String,
    ) = conn.query_row(
        "SELECT heure, compte, action, cible, details FROM journal WHERE id = ?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
    )?;
    let e = empreinte(
        &empreinte_precedente,
        id,
        &heure,
        compte,
        &action,
        cible.as_deref(),
        &details,
    );
    conn.execute(
        "INSERT INTO journal_chaine (id, precedente, empreinte) VALUES (?1, ?2, ?3)",
        params![id, precedente, e],
    )?;
    Ok(())
}

/// Chaîne les entrées écrites avant la chaîne (migration vers le schéma 6),
/// dans l'ordre du journal.
pub(crate) fn rattraper(conn: &Connection) -> rusqlite::Result<()> {
    let ids: Vec<i64> = conn
        .prepare(
            "SELECT id FROM journal WHERE id NOT IN (SELECT id FROM journal_chaine) ORDER BY id",
        )?
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    for id in ids {
        chainer(conn, id)?;
    }
    Ok(())
}

/// Entrées du journal à partir de `depuis` (exclu), avec leur maillon.
pub(crate) fn entrees(conn: &Connection, depuis: i64) -> rusqlite::Result<Vec<EntreeChainee>> {
    conn.prepare(
        "SELECT j.id, j.heure, j.compte, j.action, j.cible, j.details,
                coalesce(c.precedente, -1), coalesce(c.empreinte, '')
         FROM journal j LEFT JOIN journal_chaine c ON c.id = j.id
         WHERE j.id > ?1 ORDER BY j.id",
    )?
    .query_map([depuis], |r| {
        Ok(EntreeChainee {
            id: r.get(0)?,
            heure: r.get(1)?,
            compte: r.get(2)?,
            action: r.get(3)?,
            cible: r.get(4)?,
            details: r.get(5)?,
            precedente: r.get(6)?,
            empreinte: r.get(7)?,
        })
    })?
    .collect()
}

/// Recalcule toute la chaîne de la base, depuis la première entrée ou
/// depuis la dernière entrée archivée.
pub(crate) fn verifier(conn: &Connection) -> rusqlite::Result<EtatChaine> {
    let depart = depart(conn)?;
    let depart_id = depart.as_ref().map(|d| d.0);
    let (mut precedente, mut sceau) = depart.unwrap_or((0, ORIGINE.to_string()));
    let mut compte = 0i64;
    for e in entrees(conn, precedente)? {
        let attendue = empreinte(
            &sceau,
            e.id,
            &e.heure,
            e.compte,
            &e.action,
            e.cible.as_deref(),
            &e.details,
        );
        if e.precedente != precedente || e.empreinte != attendue {
            return Ok(EtatChaine {
                entrees: compte,
                sceau,
                rupture: Some(e.id),
                depart: depart_id,
            });
        }
        precedente = e.id;
        sceau = e.empreinte;
        compte += 1;
    }
    // Maillon sans entrée du journal (entrée retirée malgré le déclencheur).
    let orphelin: Option<i64> = conn
        .query_row(
            "SELECT min(id) FROM journal_chaine WHERE id NOT IN (SELECT id FROM journal)",
            [],
            |r| r.get(0),
        )
        .optional()?
        .flatten();
    Ok(EtatChaine {
        entrees: compte,
        sceau,
        rupture: orphelin,
        depart: depart_id,
    })
}

/// Vérifie l'export du journal (`journal-exporter`, depuis la première
/// entrée) et la base ensemble : la chaîne de l'export est recalculée
/// depuis l'origine ; ses identifiants croissent strictement ; il passe par
/// le départ de la base avec la même empreinte ; chaque entrée encore en
/// base et dans l'export y est identique (champs et empreinte), aucune
/// entrée de l'export postérieure au départ ne manque à la base ; la chaîne
/// continue dans la base après la dernière ligne de l'export. `entrees`
/// compte l'export et les entrées de la base qui le suivent.
pub(crate) fn verifier_avec_export(
    conn: &Connection,
    export: &[EntreeChainee],
) -> rusqlite::Result<EtatChaine> {
    let base = verifier(conn)?;
    if base.rupture.is_some() {
        return Ok(base);
    }
    let depart = depart(conn)?;
    let depart_id = depart.as_ref().map(|d| d.0);
    let rompue = |id: i64, entrees: i64, sceau: &str| EtatChaine {
        entrees,
        sceau: sceau.to_string(),
        rupture: Some(id),
        depart: depart_id,
    };
    let (mut precedente, mut sceau) = (0i64, ORIGINE.to_string());
    let mut compte = 0i64;
    let mut par_id: HashMap<i64, &EntreeChainee> = HashMap::new();
    for e in export {
        let attendue = empreinte(
            &sceau,
            e.id,
            &e.heure,
            e.compte,
            &e.action,
            e.cible.as_deref(),
            &e.details,
        );
        if e.id <= precedente || e.precedente != precedente || e.empreinte != attendue {
            return Ok(rompue(e.id, compte, &sceau));
        }
        precedente = e.id;
        sceau = e.empreinte.clone();
        compte += 1;
        par_id.insert(e.id, e);
    }
    let dernier_export = precedente;
    if let Some((d, empreinte_d)) = &depart
        && par_id.get(d).is_none_or(|e| &e.empreinte != empreinte_d)
    {
        return Ok(rompue(*d, compte, &sceau));
    }
    let depuis = depart_id.unwrap_or(0);
    let en_base = entrees(conn, depuis)?;
    let ids_base: HashSet<i64> = en_base.iter().map(|b| b.id).collect();
    if let Some(manque) = export
        .iter()
        .find(|e| e.id > depuis && !ids_base.contains(&e.id))
    {
        return Ok(rompue(manque.id, compte, &sceau));
    }
    for b in en_base {
        if b.id <= dernier_export {
            if par_id.get(&b.id).is_none_or(|e| **e != b) {
                return Ok(rompue(b.id, compte, &sceau));
            }
            continue;
        }
        let attendue = empreinte(
            &sceau,
            b.id,
            &b.heure,
            b.compte,
            &b.action,
            b.cible.as_deref(),
            &b.details,
        );
        if b.precedente != precedente || b.empreinte != attendue {
            return Ok(rompue(b.id, compte, &sceau));
        }
        precedente = b.id;
        sceau = b.empreinte;
        compte += 1;
    }
    Ok(EtatChaine {
        entrees: compte,
        sceau,
        rupture: None,
        depart: depart_id,
    })
}

/// Retire de la base les entrées jusqu'à `jusqua`, dont `sceau` est
/// l'empreinte, quand toutes datent d'avant `limite` (heure UTC au format
/// du journal). Refusé, sans rien changer, si la chaîne de la base est
/// rompue, si `jusqua` n'y est pas, si le sceau ne correspond pas à son
/// empreinte recalculée, ou si une entrée jusqu'à `jusqua` est trop
/// récente. Rien à faire si `jusqua` est déjà archivé.
///
/// Une seule transaction, prise en écriture dès le début (le portail peut
/// écrire en même temps) : entrée « journal_archive » notée et chaînée,
/// nouveau départ, déclencheurs levés, maillons puis entrées effacés,
/// déclencheurs remis, chaîne revérifiée.
pub(crate) fn archiver(
    conn: &Connection,
    jusqua: i64,
    sceau: &str,
    limite: &str,
) -> crate::Resultat<Archivage> {
    let refus = |m: String| Err(crate::ErreurPortail::Validation(m));
    let transaction = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    let etat = verifier(&transaction)?;
    if let Some(id) = etat.rupture {
        return refus(format!(
            "chaîne du journal rompue à l'entrée {id} : rien n'est archivé"
        ));
    }
    let (depuis, sceau_depart) = depart_ou_origine(&transaction)?;
    if jusqua <= depuis {
        return Ok(Archivage {
            jusqua: depuis,
            retirees: 0,
        });
    }
    let mut courant = sceau_depart;
    let mut recalculee = None;
    for e in entrees(&transaction, depuis)? {
        courant = empreinte(
            &courant,
            e.id,
            &e.heure,
            e.compte,
            &e.action,
            e.cible.as_deref(),
            &e.details,
        );
        if e.id == jusqua {
            recalculee = Some(courant.clone());
            break;
        }
    }
    let Some(recalculee) = recalculee else {
        return refus(format!(
            "entrée {jusqua} absente du journal : rien n'est archivé"
        ));
    };
    if recalculee != sceau {
        return refus(format!(
            "le sceau donné n'est pas celui de l'entrée {jusqua} : rien n'est archivé"
        ));
    }
    let (retirees, plus_recente): (i64, Option<String>) = transaction.query_row(
        "SELECT count(*), max(heure) FROM journal WHERE id > ?1 AND id <= ?2",
        [depuis, jusqua],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if let Some(heure) = plus_recente.as_deref()
        && heure >= limite
    {
        return refus(format!(
            "une entrée jusqu'à {jusqua} date de {heure}, pas d'avant {limite} : rien n'est archivé"
        ));
    }
    crate::noter(
        &transaction,
        None,
        "journal_archive",
        None,
        &format!(
            "{retirees} entrées retirées de la base jusqu'à l'entrée {jusqua}, sceau {sceau} ; elles restent dans l'export du journal"
        ),
    )?;
    transaction.execute(
        "INSERT INTO journal_depart (id, empreinte, archive_le) VALUES (?1, ?2, ?3)",
        params![jusqua, sceau, crate::maintenant()],
    )?;
    transaction.execute_batch(
        "DROP TRIGGER journal_chaine_sans_effacement; DROP TRIGGER journal_sans_effacement;",
    )?;
    // Les maillons d'abord : ils désignent les entrées (clé étrangère).
    transaction.execute("DELETE FROM journal_chaine WHERE id <= ?1", [jusqua])?;
    transaction.execute("DELETE FROM journal WHERE id <= ?1", [jusqua])?;
    transaction.execute_batch(crate::base::JOURNAL_SANS_EFFACEMENT)?;
    if let Some(id) = verifier(&transaction)?.rupture {
        return Err(crate::ErreurPortail::Interne(format!(
            "chaîne rompue à l'entrée {id} après l'archivage : rien n'est archivé"
        )));
    }
    transaction.commit()?;
    Ok(Archivage { jusqua, retirees })
}

/// Lit l'export du journal (une ligne JSON par entrée, `journal-exporter`).
pub fn lire_export(texte: &str) -> Result<Vec<EntreeChainee>, String> {
    texte
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(n, l)| {
            ligne_export(l).ok_or_else(|| format!("export du journal illisible, ligne {}", n + 1))
        })
        .collect()
}

fn ligne_export(ligne: &str) -> Option<EntreeChainee> {
    let v: serde_json::Value = serde_json::from_str(ligne).ok()?;
    let texte = |c: &str| -> Option<String> { Some(v.get(c)?.as_str()?.to_string()) };
    let entier = |c: &str| -> Option<i64> { v.get(c)?.as_i64() };
    let compte = match v.get("compte")? {
        serde_json::Value::Null => None,
        c => Some(c.as_i64()?),
    };
    let cible = match v.get("cible")? {
        serde_json::Value::Null => None,
        c => Some(c.as_str()?.to_string()),
    };
    Some(EntreeChainee {
        id: entier("id")?,
        heure: texte("heure")?,
        compte,
        action: texte("action")?,
        cible,
        details: texte("details")?,
        precedente: entier("precedente")?,
        empreinte: texte("empreinte")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::base::migrer(&conn).unwrap();
        conn
    }

    fn noter(conn: &Connection, action: &str, cible: Option<&str>, details: &str) -> i64 {
        conn.execute(
            "INSERT INTO journal (heure, compte, action, cible, details)
             VALUES ('2026-11-19 10:00:00', NULL, ?1, ?2, ?3)",
            params![action, cible, details],
        )
        .unwrap();
        let id = conn.last_insert_rowid();
        chainer(conn, id).unwrap();
        id
    }

    #[test]
    fn champs_sans_ambiguite() {
        let a = empreinte(ORIGINE, 1, "h", None, "x", None, "");
        let b = empreinte(ORIGINE, 1, "h", None, "x", Some(""), "");
        let c = empreinte(ORIGINE, 1, "h", None, "x", Some("a"), "");
        let d = empreinte(ORIGINE, 1, "h", None, "xa", None, "");
        assert_eq!(a.len(), 64);
        assert!(a != b && b != c && c != d && a != d);
    }

    #[test]
    fn chaine_intacte_puis_rompue() {
        let conn = base();
        assert_eq!(
            verifier(&conn).unwrap(),
            EtatChaine {
                entrees: 0,
                sceau: ORIGINE.into(),
                rupture: None,
                depart: None,
            }
        );
        noter(&conn, "connexion", None, "adresse 10.0.0.1");
        let deux = noter(&conn, "licence_emise", Some("ent"), "3 postes");
        noter(&conn, "deconnexion", None, "");
        let etat = verifier(&conn).unwrap();
        assert_eq!((etat.entrees, etat.rupture), (3, None));
        let sceau = etat.sceau.clone();
        // Deux entrées ne peuvent pas suivre la même.
        assert!(
            conn.execute(
                "INSERT INTO journal_chaine (id, precedente, empreinte) VALUES (99, ?1, 'x')",
                [deux],
            )
            .is_err()
        );
        // Ni modification ni effacement par les voies normales.
        assert!(
            conn.execute("UPDATE journal_chaine SET empreinte = 'x'", [])
                .is_err()
        );
        assert!(conn.execute("DELETE FROM journal_chaine", []).is_err());
        // Une modification qui contourne les déclencheurs rompt la chaîne à
        // l'entrée modifiée.
        conn.execute_batch(
            "DROP TRIGGER journal_sans_modification;
             UPDATE journal SET details = '30 postes' WHERE action = 'licence_emise';",
        )
        .unwrap();
        let etat = verifier(&conn).unwrap();
        assert_eq!((etat.entrees, etat.rupture), (1, Some(deux)));
        assert_ne!(etat.sceau, sceau);
    }

    #[test]
    fn entree_sans_maillon_rompt_la_chaine() {
        let conn = base();
        noter(&conn, "connexion", None, "");
        conn.execute(
            "INSERT INTO journal (heure, compte, action, cible, details)
             VALUES ('2026-11-19 10:00:01', NULL, 'connexion', NULL, 'glissée')",
            [],
        )
        .unwrap();
        let glissee = conn.last_insert_rowid();
        let etat = verifier(&conn).unwrap();
        assert_eq!((etat.entrees, etat.rupture), (1, Some(glissee)));
    }

    /// Entrée chaînée datée (heure choisie), comme le fait `noter`.
    fn noter_a(conn: &Connection, heure: &str, details: &str) -> i64 {
        conn.execute(
            "INSERT INTO journal (heure, compte, action, cible, details)
             VALUES (?1, NULL, 'essai', NULL, ?2)",
            params![heure, details],
        )
        .unwrap();
        let id = conn.last_insert_rowid();
        chainer(conn, id).unwrap();
        id
    }

    fn sceau_de(conn: &Connection, id: i64) -> String {
        conn.query_row(
            "SELECT empreinte FROM journal_chaine WHERE id = ?1",
            [id],
            |r| r.get(0),
        )
        .unwrap()
    }

    const LIMITE: &str = "2026-06-01 00:00:00";

    #[test]
    fn archivage_garde_la_chaine_et_les_declencheurs() {
        let conn = base();
        let a = noter_a(&conn, "2026-01-01 10:00:00", "a");
        let b = noter_a(&conn, "2026-02-01 10:00:00", "b");
        let c = noter_a(&conn, "2026-11-19 10:00:00", "c");
        let export = entrees(&conn, 0).unwrap();
        let sceau_b = sceau_de(&conn, b);
        let fait = archiver(&conn, b, &sceau_b, LIMITE).unwrap();
        assert_eq!(
            fait,
            Archivage {
                jusqua: b,
                retirees: 2
            }
        );
        let etat = verifier(&conn).unwrap();
        assert_eq!(etat.rupture, None);
        assert_eq!(etat.depart, Some(b));
        // c, puis l'entrée de l'archivage.
        assert_eq!(etat.entrees, 2);
        let ids: Vec<i64> = entrees(&conn, 0).unwrap().iter().map(|e| e.id).collect();
        assert_eq!(ids[0], c);
        assert!(!ids.contains(&a));
        // Déclencheurs remis.
        assert!(
            conn.execute("DELETE FROM journal WHERE id = ?1", [c])
                .is_err()
        );
        assert!(
            conn.execute("DELETE FROM journal_chaine WHERE id = ?1", [c])
                .is_err()
        );
        assert!(conn.execute("DELETE FROM journal_depart", []).is_err());
        // L'export d'avant, complété par la suite, passe avec la base.
        let mut complet = export.clone();
        complet.extend(entrees(&conn, c).unwrap());
        let ensemble = verifier_avec_export(&conn, &complet).unwrap();
        assert_eq!(ensemble.rupture, None);
        assert_eq!(ensemble.entrees, 4);
        assert_eq!(ensemble.sceau, verifier(&conn).unwrap().sceau);
        // Le même archivage relancé ne fait rien.
        assert_eq!(archiver(&conn, b, &sceau_b, LIMITE).unwrap().retirees, 0);
        // Une nouvelle entrée se chaîne à la suite.
        noter_a(&conn, "2026-11-19 11:00:00", "d");
        assert_eq!(verifier(&conn).unwrap().rupture, None);
    }

    /// Tout archivé : l'entrée de l'archivage reste, la chaîne repart du
    /// départ ; deux archivages de suite.
    #[test]
    fn tout_archive_puis_encore() {
        let conn = base();
        noter_a(&conn, "2026-01-01 10:00:00", "a");
        let b = noter_a(&conn, "2026-01-02 10:00:00", "b");
        let mut export = entrees(&conn, 0).unwrap();
        archiver(&conn, b, &sceau_de(&conn, b), LIMITE).unwrap();
        assert_eq!(verifier(&conn).unwrap().rupture, None);
        // L'entrée de l'archivage date d'aujourd'hui : on en écrit d'autres,
        // anciennes, pour un second archivage.
        let premiere_archive = entrees(&conn, 0).unwrap()[0].id;
        let e = noter_a(&conn, "2026-01-03 10:00:00", "e");
        export.extend(entrees(&conn, b).unwrap());
        // L'entrée de l'archivage est trop récente : refus.
        assert!(archiver(&conn, e, &sceau_de(&conn, e), LIMITE).is_err());
        // Avec une limite plus tardive, tout part.
        let tard = "2999-01-01 00:00:00";
        let fait = archiver(&conn, e, &sceau_de(&conn, e), tard).unwrap();
        assert_eq!(fait.retirees, 2);
        assert!(premiere_archive < e);
        let etat = verifier(&conn).unwrap();
        assert_eq!(etat.rupture, None);
        assert_eq!(etat.depart, Some(e));
        export.extend(entrees(&conn, e).unwrap());
        assert_eq!(verifier_avec_export(&conn, &export).unwrap().rupture, None);
        noter_a(&conn, "2026-11-19 12:00:00", "f");
        assert_eq!(verifier(&conn).unwrap().rupture, None);
    }

    #[test]
    fn archivage_refuse_sans_rien_changer() {
        let conn = base();
        let a = noter_a(&conn, "2026-01-01 10:00:00", "a");
        let b = noter_a(&conn, "2026-11-19 10:00:00", "b");
        let avant = entrees(&conn, 0).unwrap();
        // Sceau faux, entrée absente, entrée trop récente.
        assert!(archiver(&conn, a, &sceau_de(&conn, b), LIMITE).is_err());
        assert!(archiver(&conn, 99, &sceau_de(&conn, b), LIMITE).is_err());
        assert!(archiver(&conn, b, &sceau_de(&conn, b), LIMITE).is_err());
        assert_eq!(entrees(&conn, 0).unwrap(), avant);
        assert_eq!(verifier(&conn).unwrap().depart, None);
    }

    #[test]
    fn export_qui_ne_couvre_pas_la_base_refuse() {
        let conn = base();
        let a = noter_a(&conn, "2026-01-01 10:00:00", "a");
        let b = noter_a(&conn, "2026-01-02 10:00:00", "b");
        let c = noter_a(&conn, "2026-11-19 10:00:00", "c");
        let export = entrees(&conn, 0).unwrap();
        archiver(&conn, b, &sceau_de(&conn, b), LIMITE).unwrap();
        // Export qui s'arrête avant le départ.
        let court = verifier_avec_export(&conn, &export[..1]).unwrap();
        assert_eq!(court.rupture, Some(b));
        // Export altéré (détails changés) : rupture à cette entrée.
        let mut altere = export.clone();
        altere[0].details = "autre".into();
        assert_eq!(
            verifier_avec_export(&conn, &altere).unwrap().rupture,
            Some(a)
        );
        // Doublon (export lancé deux fois) : refusé.
        let mut double = export.clone();
        double.push(export[2].clone());
        assert_eq!(
            verifier_avec_export(&conn, &double).unwrap().rupture,
            Some(c)
        );
        // Export sans départ ni archive, base intacte : il suffit qu'il
        // commence à la première entrée.
        let conn2 = base();
        noter_a(&conn2, "2026-01-01 10:00:00", "a");
        let x = entrees(&conn2, 0).unwrap();
        assert_eq!(verifier_avec_export(&conn2, &x).unwrap().rupture, None);
        assert_eq!(verifier_avec_export(&conn2, &[]).unwrap().rupture, None);
    }

    #[test]
    fn export_lu_ligne_a_ligne() {
        let conn = base();
        noter_a(&conn, "2026-01-01 10:00:00", "a \"guillemets\"");
        let e = &entrees(&conn, 0).unwrap()[0];
        let ligne = serde_json::json!({
            "id": e.id, "heure": e.heure, "compte": e.compte, "action": e.action,
            "cible": e.cible, "details": e.details, "precedente": e.precedente,
            "empreinte": e.empreinte,
        });
        let lu = lire_export(&format!("{ligne}\n\n")).unwrap();
        assert_eq!(&lu[0], e);
        assert!(lire_export("{\"id\": 1}").is_err());
    }
}
