//! Tableau de bord : indicateurs des ventes et des achats sur une période
//! (cahier des charges, section 9), recalculés à chaque appel.
//!
//! Règles appliquées (jalon J8) :
//! - une facture ou un bon de commande compte s'il est en vigueur (ni brouillon
//!   ni annulé) et daté dans la période, dates incluses ;
//! - les avoirs validés sont déduits de leur facture, quelle que soit leur date :
//!   CA HT, TVA et CA TTC sont nets de leurs avoirs, et payé + impayé = CA TTC ;
//! - le CA HT est le HT après remise (base de la TVA), le CA TTC le montant dû ;
//! - le payé d'un document est plafonné à son montant dû (un trop-perçu ne compte
//!   pas), l'impayé est la somme des restes à payer ;
//! - le CA par mode de paiement reprend les règlements des factures de la période.

use std::collections::HashMap;

use chrono::{Datelike, NaiveDate};
use kadenz_domaine::calcul::Montant;
use kadenz_domaine::paiement::EtatPaiement;
use rusqlite::{Connection, params};

use crate::Resultat;
use crate::documents::{date_texte, lire_date};
use crate::reglements::{Situation, situation_de};

/// Indicateurs d'une période.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Indicateurs {
    pub ventes: Ventes,
    pub achats: Achats,
}

/// Ventes : factures en vigueur de la période, nettes de leurs avoirs validés.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ventes {
    pub ca_ht: Montant,
    pub tva: Montant,
    pub ca_ttc: Montant,
    pub paye: Montant,
    pub impaye: Montant,
    /// Montant dû par état de paiement : Impayé, Partiel, Payé, toujours dans
    /// cet ordre, à zéro quand aucune facture n'est dans l'état.
    pub par_etat: Vec<(EtatPaiement, Montant)>,
    /// Règlements des factures de la période par mode, dans l'ordre alphabétique.
    pub par_mode: Vec<(String, Montant)>,
    /// Un point par mois, du mois du début au mois de la fin de la période
    /// (premier jour du mois, CA TTC des factures du mois), mois vides à zéro.
    pub tendance: Vec<(NaiveDate, Montant)>,
    /// Cinq premiers clients par CA TTC, puis par nom à égalité.
    pub top_clients: Vec<(String, Montant)>,
    /// Quatre factures les plus récentes de la période.
    pub dernieres_factures: Vec<FactureRecente>,
}

/// Facture du tableau de bord.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactureRecente {
    pub id: i64,
    pub numero: String,
    pub date: NaiveDate,
    pub tiers: String,
    pub montant_du: Montant,
    pub reste_a_payer: Montant,
    pub etat: EtatPaiement,
}

/// Achats : bons de commande en vigueur de la période.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Achats {
    pub total: Montant,
    pub paye: Montant,
    pub reste: Montant,
}

const TOP_CLIENTS: usize = 5;
const DERNIERES_FACTURES: usize = 4;

/// Facture lue avec ses avoirs validés et ses règlements.
struct FactureLue {
    id: i64,
    numero: String,
    date: NaiveDate,
    tiers_id: Option<i64>,
    tiers: String,
    ht: Montant,
    tva: Montant,
    situation: Situation,
}

fn payee(s: &Situation) -> Montant {
    s.regle.min(s.montant_du).max(0)
}

fn factures(conn: &Connection, du: NaiveDate, au: NaiveDate) -> Resultat<Vec<FactureLue>> {
    let mut requete = conn.prepare(
        "SELECT d.id, COALESCE(d.numero, ''), d.date, d.tiers_id, d.tiers_nom_snapshot,
                d.montant_base_tva - COALESCE(a.base_tva, 0),
                d.montant_tva - COALESCE(a.tva, 0),
                d.montant_ttc - COALESCE(a.ttc, 0),
                (SELECT COALESCE(SUM(r.montant), 0) FROM reglement r WHERE r.document_id = d.id)
         FROM document d
         LEFT JOIN (SELECT document_source_id AS facture,
                           SUM(montant_base_tva) AS base_tva, SUM(montant_tva) AS tva,
                           SUM(montant_ttc) AS ttc
                      FROM document
                     WHERE type = 'avoir' AND statut IN ('valide', 'envoye')
                     GROUP BY document_source_id) a ON a.facture = d.id
         WHERE d.type = 'facture' AND d.statut NOT IN ('brouillon', 'annule')
           AND d.date >= ?1 AND d.date <= ?2
         ORDER BY d.date DESC, d.id DESC",
    )?;
    let lignes = requete
        .query_map(params![date_texte(du), date_texte(au)], |r| {
            Ok(FactureLue {
                id: r.get(0)?,
                numero: r.get(1)?,
                date: lire_date(r, 2)?,
                tiers_id: r.get(3)?,
                tiers: r.get(4)?,
                ht: r.get(5)?,
                tva: r.get(6)?,
                situation: situation_de(r.get(7)?, r.get(8)?),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(lignes)
}

fn par_mode(conn: &Connection, du: NaiveDate, au: NaiveDate) -> Resultat<Vec<(String, Montant)>> {
    let mut requete = conn.prepare(
        "SELECT r.mode_paiement, SUM(r.montant)
         FROM reglement r JOIN document d ON d.id = r.document_id
         WHERE d.type = 'facture' AND d.statut NOT IN ('brouillon', 'annule')
           AND d.date >= ?1 AND d.date <= ?2
         GROUP BY r.mode_paiement
         ORDER BY r.mode_paiement",
    )?;
    let lignes = requete
        .query_map(params![date_texte(du), date_texte(au)], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(lignes)
}

fn premier_du_mois(d: NaiveDate) -> NaiveDate {
    d.with_day(1).expect("le premier du mois existe toujours")
}

fn mois_suivant(d: NaiveDate) -> NaiveDate {
    let (annee, mois) = if d.month() == 12 {
        (d.year() + 1, 1)
    } else {
        (d.year(), d.month() + 1)
    };
    NaiveDate::from_ymd_opt(annee, mois, 1).expect("le premier du mois existe toujours")
}

fn ventes(conn: &Connection, du: NaiveDate, au: NaiveDate) -> Resultat<Ventes> {
    let liste = factures(conn, du, au)?;
    let mut v = Ventes::default();
    let mut par_etat = [
        (EtatPaiement::Impaye, 0),
        (EtatPaiement::Partiel, 0),
        (EtatPaiement::Paye, 0),
    ];
    let mut par_mois: HashMap<NaiveDate, Montant> = HashMap::new();
    // Clé du client : son identifiant, ou son nom pour une facture sans tiers lié.
    // Le nom retenu est celui de la facture la plus récente (liste en date
    // décroissante : la première rencontrée).
    let mut clients: Vec<((Option<i64>, String), String, Montant)> = Vec::new();
    for f in &liste {
        let s = &f.situation;
        v.ca_ht += f.ht;
        v.tva += f.tva;
        v.ca_ttc += s.montant_du;
        v.paye += payee(s);
        v.impaye += s.reste_a_payer;
        if let Some(e) = par_etat.iter_mut().find(|(e, _)| *e == s.etat) {
            e.1 += s.montant_du;
        }
        *par_mois.entry(premier_du_mois(f.date)).or_default() += s.montant_du;
        let cle = (
            f.tiers_id,
            if f.tiers_id.is_some() {
                String::new()
            } else {
                f.tiers.clone()
            },
        );
        match clients.iter_mut().find(|(c, _, _)| *c == cle) {
            Some((_, _, total)) => *total += s.montant_du,
            None => clients.push((cle, f.tiers.clone(), s.montant_du)),
        }
    }
    v.par_etat = par_etat.to_vec();
    v.par_mode = par_mode(conn, du, au)?;
    let mut mois = premier_du_mois(du);
    while mois <= au {
        v.tendance
            .push((mois, par_mois.get(&mois).copied().unwrap_or_default()));
        mois = mois_suivant(mois);
    }
    clients.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.1.cmp(&b.1)));
    v.top_clients = clients
        .into_iter()
        .take(TOP_CLIENTS)
        .map(|(_, nom, total)| (nom, total))
        .collect();
    v.dernieres_factures = liste
        .into_iter()
        .take(DERNIERES_FACTURES)
        .map(|f| FactureRecente {
            id: f.id,
            numero: f.numero,
            date: f.date,
            tiers: f.tiers,
            montant_du: f.situation.montant_du,
            reste_a_payer: f.situation.reste_a_payer,
            etat: f.situation.etat,
        })
        .collect();
    Ok(v)
}

fn achats(conn: &Connection, du: NaiveDate, au: NaiveDate) -> Resultat<Achats> {
    let mut requete = conn.prepare(
        "SELECT d.montant_ttc,
                (SELECT COALESCE(SUM(r.montant), 0) FROM reglement r WHERE r.document_id = d.id)
         FROM document d
         WHERE d.type = 'bon_commande' AND d.statut NOT IN ('brouillon', 'annule')
           AND d.date >= ?1 AND d.date <= ?2",
    )?;
    let mut a = Achats::default();
    let situations = requete
        .query_map(params![date_texte(du), date_texte(au)], |r| {
            Ok(situation_de(r.get(0)?, r.get(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for s in situations {
        a.total += s.montant_du;
        a.paye += payee(&s);
        a.reste += s.reste_a_payer;
    }
    Ok(a)
}

/// Indicateurs des ventes et des achats du `du` au `au`, dates incluses. Une
/// période vide (fin avant le début) donne des indicateurs à zéro.
pub fn indicateurs(conn: &Connection, du: NaiveDate, au: NaiveDate) -> Resultat<Indicateurs> {
    crate::licence::exiger(conn, kadenz_domaine::module::Module::TableauDeBord)?;
    Ok(Indicateurs {
        ventes: ventes(conn, du, au)?,
        achats: achats(conn, du, au)?,
    })
}

/// Années des factures et des bons de commande en vigueur, de la plus récente à
/// la plus ancienne (filtre « par année » du tableau de bord).
pub fn annees(conn: &Connection) -> Resultat<Vec<i32>> {
    let mut requete = conn.prepare(
        "SELECT DISTINCT CAST(substr(date, 1, 4) AS INTEGER) FROM document
         WHERE type IN ('facture', 'bon_commande') AND statut NOT IN ('brouillon', 'annule')
         ORDER BY 1 DESC",
    )?;
    let lignes = requete
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<Vec<i32>>>()?;
    Ok(lignes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comptes::Utilisateur;
    use crate::documents::{self, LigneSaisie, Saisie};
    use crate::droits::tests::compte;
    use crate::ouvrir_en_memoire;
    use crate::reglements::{self, SaisieReglement};
    use crate::tiers::{self, FicheTiers, TypeTiers};
    use kadenz_domaine::calcul::{Taux, UNE_UNITE};
    use kadenz_domaine::document::TypeDocument;
    use kadenz_domaine::role::Role;

    fn date(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    fn tiers(conn: &Connection, admin: &Utilisateur, t: TypeTiers, nom: &str) -> i64 {
        let mut fiche = FicheTiers::vide(t);
        fiche.nom = nom.into();
        tiers::creer(conn, admin, &fiche).unwrap().id
    }

    /// Document validé : une ligne au prix donné, remise 10 %, TVA 18 %.
    fn valide(
        conn: &mut Connection,
        admin: &Utilisateur,
        t: TypeDocument,
        tiers_id: i64,
        jour: NaiveDate,
        prix: Montant,
    ) -> i64 {
        let s = Saisie {
            tiers_id: Some(tiers_id),
            taux: Taux {
                remise: 1_000,
                tva: 1_800,
                retenue: 0,
            },
            lignes: vec![LigneSaisie {
                designation: "Pelle".into(),
                prix,
                quantite: UNE_UNITE,
                ..Default::default()
            }],
            ..Saisie::vide(jour)
        };
        let id = documents::creer_brouillon(conn, admin, t, &s).unwrap();
        documents::valider(conn, admin, id).unwrap();
        id
    }

    fn regler(conn: &mut Connection, admin: &Utilisateur, id: i64, montant: Montant, mode: &str) {
        let jour = documents::lire(conn, id).unwrap().unwrap().saisie.date;
        let s = SaisieReglement {
            date: jour,
            montant,
            mode_paiement: mode.into(),
            reference: String::new(),
        };
        reglements::ajouter(conn, admin, id, &s).unwrap();
    }

    #[test]
    fn indicateurs_d_une_periode() {
        let mut conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        let alpha = tiers(&conn, &admin, TypeTiers::Client, "Alpha");
        let beta = tiers(&conn, &admin, TypeTiers::Client, "Beta");
        let fournisseur = tiers(&conn, &admin, TypeTiers::Fournisseur, "Gamma");
        let facture = TypeDocument::Facture;
        // 100 000 HT, remise 10 000, base 90 000, TVA 16 200, TTC 106 200.
        let f1 = valide(
            &mut conn,
            &admin,
            facture,
            alpha,
            date(2026, 1, 15),
            100_000,
        );
        let f2 = valide(&mut conn, &admin, facture, beta, date(2026, 3, 2), 100_000);
        let f3 = valide(
            &mut conn,
            &admin,
            facture,
            alpha,
            date(2026, 3, 20),
            100_000,
        );
        // Hors période, brouillon et annulée : ignorées.
        valide(
            &mut conn,
            &admin,
            facture,
            beta,
            date(2025, 12, 31),
            100_000,
        );
        let annulee = valide(&mut conn, &admin, facture, beta, date(2026, 2, 1), 50_000);
        documents::annuler(&mut conn, &admin, annulee).unwrap();
        let s = Saisie {
            tiers_id: Some(beta),
            ..Saisie::vide(date(2026, 2, 2))
        };
        documents::creer_brouillon(&mut conn, &admin, facture, &s).unwrap();
        // Avoir validé sur f2 : une ligne de 10 000 HT, soit 10 620 TTC.
        let a = documents::creer_avoir(&mut conn, &admin, f2).unwrap();
        let mut d = documents::lire(&conn, a).unwrap().unwrap();
        d.saisie.lignes[0].prix = 10_000;
        documents::modifier_brouillon(&mut conn, &admin, a, &d.saisie).unwrap();
        documents::valider(&mut conn, &admin, a).unwrap();
        // f1 payée en deux modes, f2 en partie, f3 impayée.
        regler(&mut conn, &admin, f1, 6_200, "Chèque");
        regler(&mut conn, &admin, f1, 100_000, "Espèce");
        regler(&mut conn, &admin, f2, 50_000, "Espèce");
        // Achats : un bon de commande réglé en partie, un hors période.
        let bc = valide(
            &mut conn,
            &admin,
            TypeDocument::BonCommande,
            fournisseur,
            date(2026, 2, 10),
            10_000,
        );
        regler(&mut conn, &admin, bc, 1_000, "Virement");
        valide(
            &mut conn,
            &admin,
            TypeDocument::BonCommande,
            fournisseur,
            date(2027, 1, 1),
            10_000,
        );

        let i = indicateurs(&conn, date(2026, 1, 1), date(2026, 4, 30)).unwrap();
        let v = &i.ventes;
        assert_eq!(v.ca_ht, 3 * 90_000 - 9_000);
        assert_eq!(v.tva, 3 * 16_200 - 1_620);
        assert_eq!(v.ca_ttc, 3 * 106_200 - 10_620);
        assert_eq!(v.paye, 106_200 + 50_000);
        assert_eq!(v.impaye, (95_580 - 50_000) + 106_200);
        assert_eq!(v.paye + v.impaye, v.ca_ttc);
        assert_eq!(
            v.par_etat,
            [
                (EtatPaiement::Impaye, 106_200),
                (EtatPaiement::Partiel, 95_580),
                (EtatPaiement::Paye, 106_200),
            ]
        );
        assert_eq!(
            v.par_mode,
            [
                ("Chèque".to_string(), 6_200),
                ("Espèce".to_string(), 150_000)
            ]
        );
        assert_eq!(
            v.tendance,
            [
                (date(2026, 1, 1), 106_200),
                (date(2026, 2, 1), 0),
                (date(2026, 3, 1), 95_580 + 106_200),
                (date(2026, 4, 1), 0),
            ]
        );
        assert_eq!(
            v.top_clients,
            [("Alpha".to_string(), 212_400), ("Beta".to_string(), 95_580)]
        );
        let ids: Vec<i64> = v.dernieres_factures.iter().map(|f| f.id).collect();
        assert_eq!(ids, [f3, f2, f1]);
        assert_eq!(v.dernieres_factures[1].reste_a_payer, 45_580);
        assert_eq!(v.dernieres_factures[1].etat, EtatPaiement::Partiel);
        // 10 000 HT, remise 1 000, TVA 1 620 : 10 620 TTC.
        assert_eq!(
            i.achats,
            Achats {
                total: 10_620,
                paye: 1_000,
                reste: 9_620,
            }
        );
        assert_eq!(annees(&conn).unwrap(), [2027, 2026, 2025]);
    }

    #[test]
    fn periode_vide_et_bornes() {
        let mut conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        assert!(annees(&conn).unwrap().is_empty());
        let vide = indicateurs(&conn, date(2026, 1, 1), date(2026, 12, 31)).unwrap();
        assert_eq!(vide.ventes.ca_ttc, 0);
        assert_eq!(vide.ventes.tendance.len(), 12);
        assert_eq!(vide.ventes.par_etat.len(), 3);
        assert!(vide.ventes.top_clients.is_empty());
        // Dates incluses aux deux bornes ; tendance d'un mois de décembre à février.
        let client = tiers(&conn, &admin, TypeTiers::Client, "Alpha");
        valide(
            &mut conn,
            &admin,
            TypeDocument::Facture,
            client,
            date(2025, 12, 31),
            100_000,
        );
        valide(
            &mut conn,
            &admin,
            TypeDocument::Facture,
            client,
            date(2026, 2, 1),
            100_000,
        );
        let i = indicateurs(&conn, date(2025, 12, 31), date(2026, 2, 1)).unwrap();
        assert_eq!(i.ventes.ca_ttc, 2 * 106_200);
        let mois: Vec<NaiveDate> = i.ventes.tendance.iter().map(|t| t.0).collect();
        assert_eq!(
            mois,
            [date(2025, 12, 1), date(2026, 1, 1), date(2026, 2, 1)]
        );
        // Fin avant le début : rien.
        let i = indicateurs(&conn, date(2026, 2, 1), date(2025, 12, 31)).unwrap();
        assert_eq!(i.ventes.ca_ttc, 0);
        assert!(i.ventes.tendance.is_empty());
    }

    #[test]
    fn top_cinq_clients() {
        let mut conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        for (n, nom) in ["F", "E", "D", "C", "B", "A"].iter().enumerate() {
            let id = tiers(&conn, &admin, TypeTiers::Client, nom);
            // A et B à égalité, tous au-dessus de F.
            let prix = 10_000 * (n as Montant).min(4) + 10_000;
            valide(
                &mut conn,
                &admin,
                TypeDocument::Facture,
                id,
                date(2026, 5, 3),
                prix,
            );
        }
        let v = indicateurs(&conn, date(2026, 1, 1), date(2026, 12, 31))
            .unwrap()
            .ventes;
        let noms: Vec<&str> = v.top_clients.iter().map(|c| c.0.as_str()).collect();
        assert_eq!(noms, ["A", "B", "C", "D", "E"]);
    }
}
