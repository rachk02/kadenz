//! Gestion de stock (étape 2 de la phase 2, `docs/spec-stock.md` ; décisions
//! 64 à 67).
//!
//! Un mouvement est créé par le serveur seul, en appliquant l'opération qui le
//! cause : réception d'un bon de commande, confirmation d'un bordereau,
//! ajustement ou inventaire. La copie d'un poste n'en crée jamais : elle les
//! reçoit du serveur avec leur identifiant global, ce qui évite des mouvements
//! en double après une écriture faite hors connexion. Elle contrôle quand même
//! un ajustement (droit, produit suivi, motif), pour le refuser tout de suite.
//!
//! Le stock d'un produit est celui que garde son dernier mouvement ; le stock à
//! une date, celui du dernier mouvement de ce jour ou d'avant.

use std::collections::{HashMap, HashSet};

use chrono::NaiveDate;
use kadenz_domaine::calcul::{Montant, Quantite, total_ligne};
use kadenz_domaine::role::Droit;
pub use kadenz_domaine::stock::{NatureMouvement, Stock, cout_entree};
use rusqlite::{Connection, OptionalExtension, params};

use crate::comptes::Utilisateur;
use crate::documents::{Document, TypeDocument, aujourd_hui};
use crate::droits::exiger;
use crate::{Erreur, Resultat, maintenant, refus};

const FORMAT_DATE: &str = "%Y-%m-%d";

/// Modèles des refus du stock, pour leurs traductions.
pub const MODELES_ERREUR: &[&str] = &[
    "le produit « {} » n'est pas suivi en stock",
    "le motif est obligatoire",
    "la quantité de l'ajustement ne peut pas être nulle",
    "le coût unitaire ne peut pas être négatif",
    "la quantité comptée de « {} » ne peut pas être négative",
    "l'inventaire ne compte aucun produit",
    "le produit « {} » est compté deux fois",
    "le stock de « {} » n'est pas nul : le ramener à zéro par un ajustement avant d'arrêter son suivi",
];

/// Base qui est la copie d'un poste relié : aucun mouvement n'y est créé.
fn sur_copie(conn: &Connection) -> Resultat<bool> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM parametre_systeme WHERE cle = 'poste_relie'",
            [],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn stock_de_ligne(r: &rusqlite::Row<'_>, depuis: usize) -> rusqlite::Result<Stock> {
    Ok(Stock {
        quantite: r.get::<_, Option<i64>>(depuis)?.unwrap_or(0),
        valeur: r.get::<_, Option<i64>>(depuis + 1)?.unwrap_or(0),
        dernier_cout: r.get::<_, Option<i64>>(depuis + 2)?.unwrap_or(0),
    })
}

/// Stock du produit après son dernier mouvement (zéro sans mouvement).
fn courant(conn: &Connection, produit_id: i64) -> Resultat<Stock> {
    Ok(conn
        .query_row(
            "SELECT quantite_apres, valeur_apres, cout_apres FROM mouvement_stock
             WHERE produit_id = ?1 ORDER BY rang DESC LIMIT 1",
            [produit_id],
            |r| stock_de_ligne(r, 0),
        )
        .optional()?
        .unwrap_or_default())
}

/// Désignation et suivi d'un produit, ou `None` s'il n'existe pas.
fn produit(conn: &Connection, produit_id: i64) -> Resultat<Option<(String, bool)>> {
    Ok(conn
        .query_row(
            "SELECT designation, suivi_stock FROM produit WHERE id = ?1",
            [produit_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?)
}

/// Produit existant et suivi, ou le refus.
fn exiger_suivi(conn: &Connection, produit_id: i64) -> Resultat<String> {
    match produit(conn, produit_id)? {
        None => Err(Erreur::Validation("ce produit n'existe plus".into())),
        Some((designation, false)) => Err(refus(MODELES_ERREUR[0], &[&designation])),
        Some((designation, true)) => Ok(designation),
    }
}

/// Stock d'un produit : `None` s'il n'est pas suivi, zéro s'il est suivi sans
/// mouvement.
pub fn lire(conn: &Connection, produit_id: i64) -> Resultat<Option<Stock>> {
    match produit(conn, produit_id)? {
        Some((_, true)) => Ok(Some(courant(conn, produit_id)?)),
        _ => Ok(None),
    }
}

/// Stock de chaque produit suivi, en une seule lecture.
pub fn tous(conn: &Connection) -> Resultat<HashMap<i64, Stock>> {
    let mut requete = conn.prepare(
        "SELECT p.id, m.quantite_apres, m.valeur_apres, m.cout_apres FROM produit p
         LEFT JOIN mouvement_stock m ON m.id = (
             SELECT id FROM mouvement_stock WHERE produit_id = p.id ORDER BY rang DESC LIMIT 1)
         WHERE p.suivi_stock = 1",
    )?;
    let lignes = requete
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, stock_de_ligne(r, 1)?)))?
        .collect::<rusqlite::Result<HashMap<_, _>>>()?;
    Ok(lignes)
}

/// Nombre de produits suivis en stock négatif (tableau de bord).
pub fn nombre_negatifs(conn: &Connection) -> Resultat<usize> {
    Ok(tous(conn)?.values().filter(|s| s.quantite < 0).count())
}

/// Ligne de l'état du stock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LigneEtat {
    pub produit_id: i64,
    pub reference: String,
    pub designation: String,
    pub stock: Stock,
}

/// État du stock, trié par désignation : au moment présent (`None`) ou à la
/// fin du jour donné. Il porte les produits suivis et ceux qui ont des
/// mouvements.
pub fn etat(conn: &Connection, au: Option<NaiveDate>) -> Resultat<Vec<LigneEtat>> {
    let jour = au
        .map(|d| d.format(FORMAT_DATE).to_string())
        .unwrap_or_else(|| "9999-12-31".into());
    let mut requete = conn.prepare(
        "SELECT p.id, p.reference, p.designation, m.quantite_apres, m.valeur_apres, m.cout_apres
         FROM produit p
         LEFT JOIN mouvement_stock m ON m.id = (
             SELECT id FROM mouvement_stock WHERE produit_id = p.id AND date <= ?1
             ORDER BY rang DESC LIMIT 1)
         WHERE p.suivi_stock = 1 OR EXISTS (SELECT 1 FROM mouvement_stock WHERE produit_id = p.id)
         ORDER BY p.designation COLLATE NOCASE, p.id",
    )?;
    let lignes = requete
        .query_map([jour], |r| {
            Ok(LigneEtat {
                produit_id: r.get(0)?,
                reference: r.get(1)?,
                designation: r.get(2)?,
                stock: stock_de_ligne(r, 3)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(lignes)
}

/// Critères de la liste des mouvements.
#[derive(Debug, Clone, Default)]
pub struct FiltreMouvements {
    pub produit: Option<i64>,
    pub du: Option<NaiveDate>,
    pub au: Option<NaiveDate>,
    pub nature: Option<NatureMouvement>,
}

/// Document à l'origine d'un mouvement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentLie {
    pub id: i64,
    pub type_document: TypeDocument,
    pub numero: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mouvement {
    pub id: i64,
    pub produit_id: i64,
    pub date: NaiveDate,
    pub nature: NatureMouvement,
    /// Quantité signée, en millièmes.
    pub quantite: Quantite,
    /// Valeur signée, en FCFA.
    pub valeur: Montant,
    pub quantite_apres: Quantite,
    pub valeur_apres: Montant,
    pub document: Option<DocumentLie>,
    pub motif: String,
    /// Nom complet du compte qui a agi (vide s'il n'existe plus).
    pub compte: String,
}

/// Mouvements retenus par le filtre, dans leur ordre sur le serveur.
pub fn mouvements(conn: &Connection, filtre: &FiltreMouvements) -> Resultat<Vec<Mouvement>> {
    let du = filtre
        .du
        .map(|d| d.format(FORMAT_DATE).to_string())
        .unwrap_or_default();
    let au = filtre
        .au
        .map(|d| d.format(FORMAT_DATE).to_string())
        .unwrap_or_else(|| "9999-12-31".into());
    let mut requete = conn.prepare(
        "SELECT m.id, m.produit_id, m.date, m.nature, m.quantite, m.valeur, m.quantite_apres,
                m.valeur_apres, d.id, d.type, d.numero, m.motif, coalesce(u.nom_complet, '')
         FROM mouvement_stock m
         LEFT JOIN document d ON d.id = m.document_id
         LEFT JOIN utilisateur u ON u.id = m.utilisateur_id
         WHERE (?1 IS NULL OR m.produit_id = ?1) AND (?2 IS NULL OR m.nature = ?2)
           AND m.date >= ?3 AND m.date <= ?4
         ORDER BY m.rang",
    )?;
    let brutes = requete
        .query_map(
            params![filtre.produit, filtre.nature.map(|n| n.code()), du, au],
            |r| {
                Ok((
                    (
                        r.get::<_, i64>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                    ),
                    (
                        r.get::<_, i64>(4)?,
                        r.get::<_, i64>(5)?,
                        r.get::<_, i64>(6)?,
                        r.get::<_, i64>(7)?,
                    ),
                    (
                        r.get::<_, Option<i64>>(8)?,
                        r.get::<_, Option<String>>(9)?,
                        r.get::<_, Option<String>>(10)?,
                    ),
                    (r.get::<_, String>(11)?, r.get::<_, String>(12)?),
                ))
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let illisible = || Erreur::Validation("mouvement de stock illisible".into());
    brutes
        .into_iter()
        .map(
            |(
                (id, produit_id, date, nature),
                (quantite, valeur, quantite_apres, valeur_apres),
                (doc_id, doc_type, numero),
                (motif, compte),
            )| {
                let document = match (doc_id, doc_type) {
                    (Some(id), Some(code)) => Some(DocumentLie {
                        id,
                        type_document: TypeDocument::depuis_code(&code).ok_or_else(illisible)?,
                        numero,
                    }),
                    _ => None,
                };
                Ok(Mouvement {
                    id,
                    produit_id,
                    date: NaiveDate::parse_from_str(&date, FORMAT_DATE).map_err(|_| illisible())?,
                    nature: NatureMouvement::depuis_code(&nature).ok_or_else(illisible)?,
                    quantite,
                    valeur,
                    quantite_apres,
                    valeur_apres,
                    document,
                    motif,
                    compte,
                })
            },
        )
        .collect()
}

/// Origine d'un mouvement.
struct Origine<'a> {
    nature: NatureMouvement,
    /// Document et rang de sa ligne.
    document: Option<(i64, usize)>,
    motif: &'a str,
    acteur: &'a Utilisateur,
}

/// Écrit un mouvement de `quantite` (signée) et `valeur` (signée) qui mène au
/// stock `apres` ; rend son identifiant.
fn inscrire(
    conn: &Connection,
    produit_id: i64,
    quantite: Quantite,
    valeur: Montant,
    apres: Stock,
    origine: &Origine<'_>,
) -> Resultat<i64> {
    let rang: i64 = conn.query_row(
        "SELECT coalesce(max(rang), 0) + 1 FROM mouvement_stock",
        [],
        |r| r.get(0),
    )?;
    conn.execute(
        "INSERT INTO mouvement_stock (rang, produit_id, nature, date, heure, quantite, valeur,
             quantite_apres, valeur_apres, cout_apres, document_id, ligne, motif, utilisateur_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            rang,
            produit_id,
            origine.nature.code(),
            aujourd_hui().format(FORMAT_DATE).to_string(),
            maintenant(),
            quantite,
            valeur,
            apres.quantite,
            apres.valeur,
            apres.dernier_cout,
            origine.document.map(|(id, _)| id),
            origine.document.map(|(_, ligne)| ligne as i64),
            origine.motif,
            origine.acteur.id,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Entrée au coût `cout_unitaire` (le coût moyen du moment à défaut) ou
/// sortie au coût moyen, selon le signe de `quantite`.
fn bouger(
    conn: &Connection,
    produit_id: i64,
    quantite: Quantite,
    cout_unitaire: Option<Montant>,
    origine: &Origine<'_>,
) -> Resultat<i64> {
    let avant = courant(conn, produit_id)?;
    if quantite >= 0 {
        let valeur = total_ligne(
            cout_unitaire.unwrap_or_else(|| avant.cout_moyen()),
            quantite,
        );
        inscrire(
            conn,
            produit_id,
            quantite,
            valeur,
            avant.entree(quantite, valeur),
            origine,
        )
    } else {
        let (apres, valeur) = avant.sortie(-quantite);
        inscrire(conn, produit_id, quantite, -valeur, apres, origine)
    }
}

/// Produits suivis cités par les lignes d'un document : rang de la ligne et
/// produit.
fn lignes_suivies(conn: &Connection, d: &Document) -> Resultat<Vec<(usize, i64)>> {
    let mut sortie = Vec::new();
    for (rang, ligne) in d.saisie.lignes.iter().enumerate() {
        if let Some(pid) = ligne.produit_id
            && matches!(produit(conn, pid)?, Some((_, true)))
        {
            sortie.push((rang, pid));
        }
    }
    Ok(sortie)
}

/// Réception d'un bon de commande (`documents::enregistrer_reception`) :
/// l'écart avec la quantité déjà reçue entre au coût de la ligne, remise du
/// document déduite ; une correction à la baisse sort au même coût.
pub(crate) fn sur_reception(
    conn: &Connection,
    acteur: &Utilisateur,
    d: &Document,
    recues: &[Quantite],
) -> Resultat<()> {
    if sur_copie(conn)? {
        return Ok(());
    }
    for (rang, pid) in lignes_suivies(conn, d)? {
        let ligne = &d.saisie.lignes[rang];
        let ecart = recues[rang] - ligne.quantite_livree;
        if ecart == 0 {
            continue;
        }
        let origine = Origine {
            nature: NatureMouvement::Reception,
            document: Some((d.id, rang)),
            motif: "",
            acteur,
        };
        let cout = cout_entree(ligne.prix, ecart.abs(), d.saisie.taux.remise);
        let avant = courant(conn, pid)?;
        if ecart > 0 {
            inscrire(conn, pid, ecart, cout, avant.entree(ecart, cout), &origine)?;
        } else {
            inscrire(
                conn,
                pid,
                ecart,
                -cout,
                avant.sortie_a(-ecart, cout),
                &origine,
            )?;
        }
    }
    Ok(())
}

/// Confirmation d'un bordereau (`documents::confirmer_livraison`) : chaque
/// quantité livrée d'un produit suivi sort au coût moyen, même à découvert
/// (décision 66).
pub(crate) fn sur_livraison(conn: &Connection, acteur: &Utilisateur, d: &Document) -> Resultat<()> {
    if sur_copie(conn)? {
        return Ok(());
    }
    for (rang, pid) in lignes_suivies(conn, d)? {
        let livree = d.saisie.lignes[rang].quantite_livree;
        if livree <= 0 {
            continue;
        }
        let origine = Origine {
            nature: NatureMouvement::Livraison,
            document: Some((d.id, rang)),
            motif: "",
            acteur,
        };
        bouger(conn, pid, -livree, None, &origine)?;
    }
    Ok(())
}

fn motif_nettoye(motif: &str) -> Resultat<&str> {
    let motif = motif.trim();
    if motif.is_empty() {
        return Err(refus(MODELES_ERREUR[1], &[]));
    }
    Ok(motif)
}

fn exiger_cout(cout_unitaire: Option<Montant>) -> Resultat<()> {
    if cout_unitaire.is_some_and(|c| c < 0) {
        return Err(refus(MODELES_ERREUR[3], &[]));
    }
    Ok(())
}

/// Ajustement d'un produit suivi : entrée au coût unitaire donné (le coût
/// moyen du moment à défaut) ou sortie au coût moyen. Rend le mouvement créé,
/// `None` sur la copie d'un poste, qui ne fait que contrôler.
pub fn ajuster(
    conn: &Connection,
    acteur: &Utilisateur,
    produit_id: i64,
    quantite: Quantite,
    cout_unitaire: Option<Montant>,
    motif: &str,
) -> Resultat<Option<i64>> {
    exiger(conn, acteur, Droit::AjusterStock)?;
    exiger_suivi(conn, produit_id)?;
    let motif = motif_nettoye(motif)?;
    if quantite == 0 {
        return Err(refus(MODELES_ERREUR[2], &[]));
    }
    exiger_cout(cout_unitaire)?;
    if sur_copie(conn)? {
        return Ok(None);
    }
    let origine = Origine {
        nature: NatureMouvement::Ajustement,
        document: None,
        motif,
        acteur,
    };
    bouger(conn, produit_id, quantite, cout_unitaire, &origine).map(Some)
}

/// Ligne d'un inventaire à valider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LigneInventaire {
    pub produit_id: i64,
    pub quantite_comptee: Quantite,
    /// Coût unitaire d'un écart positif ; le coût moyen du moment à défaut.
    pub cout_unitaire: Option<Montant>,
}

/// Validation d'un inventaire : un mouvement d'écart par produit dont la
/// quantité comptée diffère de celle de la base. Rend, pour chaque produit qui
/// a bougé, son mouvement, dans l'ordre des lignes.
pub fn saisir_inventaire(
    conn: &mut Connection,
    acteur: &Utilisateur,
    motif: &str,
    lignes: &[LigneInventaire],
) -> Resultat<Vec<(i64, i64)>> {
    exiger(conn, acteur, Droit::AjusterStock)?;
    let motif = motif_nettoye(motif)?;
    if lignes.is_empty() {
        return Err(refus(MODELES_ERREUR[5], &[]));
    }
    let mut vus = HashSet::new();
    for l in lignes {
        let designation = exiger_suivi(conn, l.produit_id)?;
        if !vus.insert(l.produit_id) {
            return Err(refus(MODELES_ERREUR[6], &[&designation]));
        }
        if l.quantite_comptee < 0 {
            return Err(refus(MODELES_ERREUR[4], &[&designation]));
        }
        exiger_cout(l.cout_unitaire)?;
    }
    if sur_copie(conn)? {
        return Ok(Vec::new());
    }
    let tx = conn.savepoint()?;
    let origine = Origine {
        nature: NatureMouvement::Inventaire,
        document: None,
        motif,
        acteur,
    };
    let mut crees = Vec::new();
    for l in lignes {
        let ecart = l.quantite_comptee - courant(&tx, l.produit_id)?.quantite;
        if ecart != 0 {
            let id = bouger(&tx, l.produit_id, ecart, l.cout_unitaire, &origine)?;
            crees.push((l.produit_id, id));
        }
    }
    tx.commit()?;
    Ok(crees)
}

/// Refus d'arrêter le suivi d'un produit dont la quantité n'est pas nulle
/// (`produits::modifier`).
pub(crate) fn exiger_arret_possible(conn: &Connection, produit_id: i64) -> Resultat<()> {
    let s = courant(conn, produit_id)?;
    if s.quantite != 0 {
        let designation = produit(conn, produit_id)?.map(|p| p.0).unwrap_or_default();
        return Err(refus(MODELES_ERREUR[7], &[&designation]));
    }
    Ok(())
}

/// Produit cité par au moins un mouvement (il ne peut plus être supprimé).
pub(crate) fn a_des_mouvements(conn: &Connection, produit_id: i64) -> Resultat<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM mouvement_stock WHERE produit_id = ?1)",
        [produit_id],
        |r| r.get(0),
    )?)
}

/// Ajustements en file d'envoi sur la copie d'un poste, par produit : ils ne
/// changent la quantité affichée qu'une fois appliqués par le serveur.
pub fn ajustements_en_attente(conn: &Connection) -> Resultat<HashMap<i64, usize>> {
    let mut sortie = HashMap::new();
    for entree in crate::replique::file(conn)? {
        if let kadenz_protocole::Action::AjusterStock { produit, .. } = &entree.operation.action
            && let Some(id) =
                crate::synchro::id_de(conn, kadenz_protocole::Entite::Produit, produit)?
        {
            *sortie.entry(id).or_insert(0) += 1;
        }
    }
    Ok(sortie)
}

/// Quantité comptée d'un produit dans l'inventaire en cours de cette base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comptage {
    pub produit_id: i64,
    pub quantite_comptee: Option<Quantite>,
    pub cout_unitaire: Option<Montant>,
}

/// Note la quantité comptée d'un produit dans l'inventaire en cours ; sans
/// quantité ni coût, la ligne est effacée.
pub fn noter_comptage(
    conn: &Connection,
    produit_id: i64,
    quantite_comptee: Option<Quantite>,
    cout_unitaire: Option<Montant>,
) -> Resultat<()> {
    if quantite_comptee.is_none() && cout_unitaire.is_none() {
        conn.execute(
            "DELETE FROM comptage_stock WHERE produit_id = ?1",
            [produit_id],
        )?;
    } else {
        conn.execute(
            "INSERT INTO comptage_stock (produit_id, quantite_comptee, cout_unitaire)
             VALUES (?1, ?2, ?3)
             ON CONFLICT (produit_id) DO UPDATE SET
                 quantite_comptee = excluded.quantite_comptee,
                 cout_unitaire = excluded.cout_unitaire",
            params![produit_id, quantite_comptee, cout_unitaire],
        )?;
    }
    Ok(())
}

/// Inventaire en cours, par produit.
pub fn comptage(conn: &Connection) -> Resultat<Vec<Comptage>> {
    let mut requete = conn.prepare(
        "SELECT produit_id, quantite_comptee, cout_unitaire FROM comptage_stock ORDER BY produit_id",
    )?;
    let lignes = requete
        .query_map([], |r| {
            Ok(Comptage {
                produit_id: r.get(0)?,
                quantite_comptee: r.get(1)?,
                cout_unitaire: r.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(lignes)
}

pub fn effacer_comptage(conn: &Connection) -> Resultat<()> {
    conn.execute("DELETE FROM comptage_stock", [])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::documents::{self, LigneSaisie, Saisie};
    use crate::droits::tests::compte;
    use crate::ouvrir_en_memoire;
    use crate::produits::{self, FicheProduit, FiltreProduits, Sens};
    use crate::tiers;
    use kadenz_domaine::calcul::{Taux, UNE_UNITE};
    use kadenz_domaine::referentiel::{FicheTiers, TypeTiers};
    use kadenz_domaine::role::Role;

    const U: Quantite = UNE_UNITE;

    struct Contexte {
        conn: Connection,
        admin: Utilisateur,
        client: i64,
        fournisseur: i64,
    }

    fn contexte() -> Contexte {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        let mut f = FicheTiers::vide(TypeTiers::Client);
        f.nom = "Client Alpha".into();
        let client = tiers::creer(&conn, &admin, &f).unwrap().id;
        let mut f = FicheTiers::vide(TypeTiers::Fournisseur);
        f.nom = "Fournisseur Beta".into();
        let fournisseur = tiers::creer(&conn, &admin, &f).unwrap().id;
        Contexte {
            conn,
            admin,
            client,
            fournisseur,
        }
    }

    fn produit(c: &Contexte, reference: &str, suivi: bool) -> i64 {
        let f = FicheProduit {
            designation: format!("Produit {reference}"),
            reference: reference.into(),
            prix: 1_500,
            sens: Sens::LesDeux,
            suivi_stock: suivi,
        };
        produits::creer(&c.conn, &c.admin, &f).unwrap().id
    }

    fn ligne(produit_id: i64, prix: Montant, quantite: Quantite) -> LigneSaisie {
        LigneSaisie {
            produit_id: Some(produit_id),
            reference: format!("R{produit_id}"),
            designation: format!("Produit {produit_id}"),
            prix,
            quantite,
            ..Default::default()
        }
    }

    /// Bon de commande envoyé, remise de 10 %.
    fn bon_commande(c: &mut Contexte, lignes: Vec<LigneSaisie>) -> i64 {
        let s = Saisie {
            tiers_id: Some(c.fournisseur),
            taux: Taux {
                remise: 1_000,
                tva: 1_800,
                retenue: 500,
            },
            lignes,
            ..Saisie::vide(aujourd_hui())
        };
        let admin = c.admin.clone();
        let id =
            documents::creer_brouillon(&mut c.conn, &admin, TypeDocument::BonCommande, &s).unwrap();
        documents::valider(&mut c.conn, &admin, id).unwrap();
        documents::marquer_envoye(&mut c.conn, &admin, id).unwrap();
        id
    }

    /// Bordereau vierge validé puis confirmé : chaque ligne livrée en entier.
    fn livrer(c: &mut Contexte, lignes: Vec<LigneSaisie>) -> i64 {
        let lignes = lignes
            .into_iter()
            .map(|l| LigneSaisie {
                quantite_livree: l.quantite,
                ..l
            })
            .collect();
        let s = Saisie {
            tiers_id: Some(c.client),
            lignes,
            ..Saisie::vide(aujourd_hui())
        };
        let admin = c.admin.clone();
        let id =
            documents::creer_brouillon(&mut c.conn, &admin, TypeDocument::Livraison, &s).unwrap();
        documents::valider(&mut c.conn, &admin, id).unwrap();
        documents::confirmer_livraison(&mut c.conn, &admin, id).unwrap();
        id
    }

    fn stock(c: &Contexte, produit_id: i64) -> (Quantite, Montant) {
        let s = lire(&c.conn, produit_id).unwrap().unwrap();
        (s.quantite, s.valeur)
    }

    fn nombre(c: &Contexte, filtre: FiltreMouvements) -> usize {
        mouvements(&c.conn, &filtre).unwrap().len()
    }

    #[test]
    fn reception_correction_et_livraison_a_decouvert() {
        let mut c = contexte();
        let p = produit(&c, "P", true);
        let service = produit(&c, "S", false);
        assert_eq!(lire(&c.conn, service).unwrap(), None);
        assert_eq!(lire(&c.conn, p).unwrap(), Some(Stock::default()));
        let admin = c.admin.clone();
        let bc = bon_commande(
            &mut c,
            vec![ligne(p, 1_000, 10 * U), ligne(service, 5_000, U)],
        );
        // 6 reçus à 1 000, remise de 10 % : 5 400.
        documents::enregistrer_reception(&mut c.conn, &admin, bc, &[6 * U, 0]).unwrap();
        assert_eq!(stock(&c, p), (6 * U, 5_400));
        // Correction à la baisse : 1 sort à son coût d'entrée.
        documents::enregistrer_reception(&mut c.conn, &admin, bc, &[5 * U, U]).unwrap();
        assert_eq!(stock(&c, p), (5 * U, 4_500));
        assert_eq!(
            nombre(&c, FiltreMouvements::default()),
            2,
            "service non suivi"
        );
        let m = mouvements(
            &c.conn,
            &FiltreMouvements {
                produit: Some(p),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            m.iter().map(|m| (m.quantite, m.valeur)).collect::<Vec<_>>(),
            [(6 * U, 5_400), (-U, -900)]
        );
        let doc = m[0].document.clone().unwrap();
        assert_eq!((doc.id, doc.type_document), (bc, TypeDocument::BonCommande));
        assert_eq!(m[0].nature, NatureMouvement::Reception);
        assert_eq!(m[0].compte, admin.nom_complet);

        // Livraison de 7 : 5 pour 4 500, 2 au dernier coût (900), sans refus.
        let l = livrer(&mut c, vec![ligne(p, 2_000, 7 * U)]);
        assert_eq!(stock(&c, p), (-2 * U, -1_800));
        let livraisons = mouvements(
            &c.conn,
            &FiltreMouvements {
                nature: Some(NatureMouvement::Livraison),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(livraisons.len(), 1);
        assert_eq!(livraisons[0].document.as_ref().unwrap().id, l);
        assert_eq!(livraisons[0].valeur, -6_300);
        assert_eq!(nombre_negatifs(&c.conn).unwrap(), 1);
        let negatifs = produits::lister(
            &c.conn,
            &FiltreProduits {
                negatifs: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(negatifs.iter().map(|p| p.id).collect::<Vec<_>>(), [p]);
    }

    #[test]
    fn ajustement_controle_puis_entree_et_sortie() {
        let c = contexte();
        let p = produit(&c, "P", true);
        let service = produit(&c, "S", false);
        let vente = compte(&c.conn, "vente", Role::Commercial);
        let admin = c.admin.clone();
        assert!(matches!(
            ajuster(&c.conn, &vente, p, U, None, "Casse"),
            Err(Erreur::Interdit(Droit::AjusterStock))
        ));
        assert!(ajuster(&c.conn, &admin, service, U, None, "Casse").is_err());
        assert!(ajuster(&c.conn, &admin, p, U, None, "  ").is_err());
        assert!(ajuster(&c.conn, &admin, p, 0, None, "Casse").is_err());
        assert!(ajuster(&c.conn, &admin, p, U, Some(-1), "Casse").is_err());
        assert_eq!(nombre(&c, FiltreMouvements::default()), 0);

        // Entrée de 4 à 250, puis sortie de 1 au coût moyen.
        ajuster(&c.conn, &admin, p, 4 * U, Some(250), "Stock d'ouverture")
            .unwrap()
            .unwrap();
        ajuster(&c.conn, &admin, p, -U, None, " Casse ").unwrap();
        assert_eq!(stock(&c, p), (3 * U, 750));
        let m = mouvements(&c.conn, &FiltreMouvements::default()).unwrap();
        assert_eq!(m[1].motif, "Casse");
        assert_eq!(m[1].nature, NatureMouvement::Ajustement);
        // Entrée sans coût : au coût moyen du moment.
        ajuster(&c.conn, &admin, p, U, None, "Retour client").unwrap();
        assert_eq!(stock(&c, p), (4 * U, 1_000));
    }

    #[test]
    fn inventaire_ecarts_seulement() {
        let mut c = contexte();
        let a = produit(&c, "A", true);
        let b = produit(&c, "B", true);
        let service = produit(&c, "S", false);
        let admin = c.admin.clone();
        ajuster(&c.conn, &admin, a, 5 * U, Some(100), "Ouverture").unwrap();
        let ligne = |produit_id, quantite_comptee, cout_unitaire| LigneInventaire {
            produit_id,
            quantite_comptee,
            cout_unitaire,
        };
        assert!(saisir_inventaire(&mut c.conn, &admin, "Inventaire", &[]).is_err());
        assert!(
            saisir_inventaire(
                &mut c.conn,
                &admin,
                "Inventaire",
                &[ligne(a, U, None), ligne(a, U, None)]
            )
            .is_err()
        );
        assert!(
            saisir_inventaire(&mut c.conn, &admin, "Inventaire", &[ligne(a, -U, None)]).is_err()
        );
        assert!(
            saisir_inventaire(
                &mut c.conn,
                &admin,
                "Inventaire",
                &[ligne(service, U, None)]
            )
            .is_err()
        );
        // A : 5 comptés sur 5, aucun mouvement ; B : 3 comptés à 200.
        let crees = saisir_inventaire(
            &mut c.conn,
            &admin,
            "Inventaire de fin d'année",
            &[ligne(a, 5 * U, None), ligne(b, 3 * U, Some(200))],
        )
        .unwrap();
        assert_eq!(crees.len(), 1);
        assert_eq!(crees[0].0, b);
        assert_eq!(stock(&c, b), (3 * U, 600));
        // A : 4 comptés, l'écart sort au coût moyen.
        saisir_inventaire(&mut c.conn, &admin, "Recomptage", &[ligne(a, 4 * U, None)]).unwrap();
        assert_eq!(stock(&c, a), (4 * U, 400));
        assert_eq!(
            nombre(
                &c,
                FiltreMouvements {
                    nature: Some(NatureMouvement::Inventaire),
                    ..Default::default()
                }
            ),
            2
        );
    }

    #[test]
    fn suivi_arrete_seulement_a_zero_et_produit_garde() {
        let c = contexte();
        let p = produit(&c, "P", true);
        let admin = c.admin.clone();
        ajuster(&c.conn, &admin, p, 2 * U, Some(100), "Ouverture").unwrap();
        let mut f = produits::lire(&c.conn, p).unwrap().unwrap().fiche;
        f.suivi_stock = false;
        assert!(produits::modifier(&c.conn, &admin, p, &f).is_err());
        ajuster(&c.conn, &admin, p, -2 * U, None, "Retour fournisseur").unwrap();
        produits::modifier(&c.conn, &admin, p, &f).unwrap();
        assert_eq!(lire(&c.conn, p).unwrap(), None);
        // Un produit qui a des mouvements n'est plus supprimé.
        assert_eq!(
            produits::retirer(&c.conn, &admin, p).unwrap(),
            crate::Retrait::Desactivee
        );
        // L'état garde un produit qui a des mouvements, même non suivi.
        assert_eq!(etat(&c.conn, None).unwrap().len(), 1);
    }

    #[test]
    fn copie_d_un_poste_sans_mouvement() {
        let mut c = contexte();
        let p = produit(&c, "P", true);
        let admin = c.admin.clone();
        c.conn
            .execute(
                "INSERT INTO parametre_systeme (cle, valeur) VALUES ('poste_relie', '{}')",
                [],
            )
            .unwrap();
        // Contrôlé, mais rien d'écrit.
        assert!(ajuster(&c.conn, &admin, p, U, None, "").is_err());
        assert_eq!(ajuster(&c.conn, &admin, p, U, None, "Casse").unwrap(), None);
        let bc = bon_commande(&mut c, vec![ligne(p, 1_000, 2 * U)]);
        documents::enregistrer_reception(&mut c.conn, &admin, bc, &[2 * U]).unwrap();
        livrer(&mut c, vec![ligne(p, 1_000, U)]);
        assert_eq!(nombre(&c, FiltreMouvements::default()), 0);
        assert_eq!(stock(&c, p), (0, 0));
    }

    #[test]
    fn etat_a_une_date() {
        let c = contexte();
        let p = produit(&c, "P", true);
        let admin = c.admin.clone();
        ajuster(&c.conn, &admin, p, 3 * U, Some(100), "Ouverture").unwrap();
        let hier = aujourd_hui().pred_opt().unwrap();
        let avant = etat(&c.conn, Some(hier)).unwrap();
        assert_eq!(avant.len(), 1);
        assert_eq!(avant[0].stock, Stock::default());
        let present = etat(&c.conn, None).unwrap();
        assert_eq!(present[0].stock.quantite, 3 * U);
        assert_eq!(etat(&c.conn, Some(aujourd_hui())).unwrap(), present);
        assert_eq!(
            nombre(
                &c,
                FiltreMouvements {
                    au: Some(hier),
                    ..Default::default()
                }
            ),
            0
        );
    }

    #[test]
    fn comptage_en_cours() {
        let c = contexte();
        let a = produit(&c, "A", true);
        let b = produit(&c, "B", true);
        noter_comptage(&c.conn, a, Some(2 * U), None).unwrap();
        noter_comptage(&c.conn, b, Some(U), Some(300)).unwrap();
        noter_comptage(&c.conn, a, Some(3 * U), None).unwrap();
        assert_eq!(
            comptage(&c.conn).unwrap(),
            [
                Comptage {
                    produit_id: a,
                    quantite_comptee: Some(3 * U),
                    cout_unitaire: None
                },
                Comptage {
                    produit_id: b,
                    quantite_comptee: Some(U),
                    cout_unitaire: Some(300)
                }
            ]
        );
        noter_comptage(&c.conn, b, None, None).unwrap();
        assert_eq!(comptage(&c.conn).unwrap().len(), 1);
        effacer_comptage(&c.conn).unwrap();
        assert!(comptage(&c.conn).unwrap().is_empty());
    }
}
