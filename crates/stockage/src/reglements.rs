//! Règlements des factures et des bons de commande, registres et relances
//! (cahier des charges, section 8).
//!
//! Règles appliquées ici, quel que soit l'écran :
//! - un règlement vise une facture ou un bon de commande en vigueur (validé et
//!   non annulé) ; il ne porte jamais le total réglé au-delà du montant dû
//!   (TTC moins les avoirs validés d'une facture) ;
//! - sa date est comprise entre la date du document et aujourd'hui (décision du
//!   2026-11-10, jalon J7) ;
//! - saisir et supprimer un règlement demandent le droit « Saisir un règlement » ;
//!   l'ajout et la suppression sont tracés dans le journal d'audit du document,
//!   la suppression avec la valeur supprimée ;
//! - une facture est échue quand sa date d'échéance est passée et qu'il lui reste
//!   un montant à payer ; une facture sans échéance n'est jamais échue.

use chrono::NaiveDate;
use kadenz_domaine::calcul::Montant;
use kadenz_domaine::document::{Statut, TypeDocument};
use kadenz_domaine::paiement::{EtatPaiement, ReglementRefuse, etat_paiement, verifier_reglement};
use kadenz_domaine::role::Droit;
use kadenz_domaine::saisie::montant_en_texte;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use crate::audit::{self, Entree};
use crate::comptes::Utilisateur;
use crate::documents::{
    Document, aujourd_hui, avoirs_valides, date_texte, lire_date, lire_date_facultative,
    lire_existant, lire_type,
};
use crate::droits::exiger;
use crate::{Erreur, Resultat, refus};

/// Règlement enregistré.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reglement {
    pub id: i64,
    pub document_id: i64,
    pub date: NaiveDate,
    pub montant: Montant,
    pub mode_paiement: String,
    pub reference: String,
    /// Nom complet du compte qui l'a saisi, vide si inconnu.
    pub auteur: String,
}

/// Saisie d'un nouveau règlement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaisieReglement {
    pub date: NaiveDate,
    pub montant: Montant,
    pub mode_paiement: String,
    pub reference: String,
}

/// Situation de paiement d'un document en vigueur qui reçoit des règlements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Situation {
    /// TTC moins les avoirs validés (facture), TTC (bon de commande).
    pub montant_du: Montant,
    pub regle: Montant,
    /// Jamais négatif : un avoir validé après un paiement complet laisse un
    /// trop-perçu, pas un reste négatif.
    pub reste_a_payer: Montant,
    pub etat: EtatPaiement,
}

/// Encaissement (facture, client) ou décaissement (bon de commande, fournisseur).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sens {
    Encaissement,
    Decaissement,
}

impl Sens {
    pub fn du_type(t: TypeDocument) -> Option<Sens> {
        match t {
            TypeDocument::Facture => Some(Sens::Encaissement),
            TypeDocument::BonCommande => Some(Sens::Decaissement),
            _ => None,
        }
    }
}

/// Critères du registre des règlements ; un critère absent ne filtre pas.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FiltreRegistre {
    pub sens: Option<Sens>,
    pub du: Option<NaiveDate>,
    pub au: Option<NaiveDate>,
    pub mode: Option<String>,
}

/// Ligne du registre : le règlement et le document réglé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LigneRegistre {
    pub reglement: Reglement,
    pub sens: Sens,
    pub type_document: TypeDocument,
    pub numero: String,
    pub tiers: String,
}

/// Total d'un sens (encaissements ou décaissements) dans la sélection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TotalSens {
    pub total: Montant,
    /// Sous-total par mode de paiement, dans l'ordre alphabétique des modes.
    pub par_mode: Vec<(String, Montant)>,
}

/// Registre filtré, du plus ancien au plus récent, avec ses totaux : les
/// encaissements et les décaissements ne s'additionnent pas.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Registre {
    pub lignes: Vec<LigneRegistre>,
    pub encaissements: TotalSens,
    pub decaissements: TotalSens,
}

/// Document qui reste à payer (suivi fournisseurs, relances).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentDu {
    pub id: i64,
    pub type_document: TypeDocument,
    pub numero: String,
    pub statut: Statut,
    pub date: NaiveDate,
    pub echeance: Option<NaiveDate>,
    pub tiers: String,
    pub montant_du: Montant,
    pub regle: Montant,
    pub reste_a_payer: Montant,
    /// Jours écoulés depuis l'échéance, pour une facture échue ; rien sinon.
    pub jours_de_retard: Option<i64>,
}

fn total_regle(conn: &Connection, document_id: i64) -> Resultat<Montant> {
    Ok(conn.query_row(
        "SELECT COALESCE(SUM(montant), 0) FROM reglement WHERE document_id = ?1",
        [document_id],
        |r| r.get(0),
    )?)
}

/// Nombre de règlements d'un document (refus d'annulation, section 8).
pub(crate) fn nombre(conn: &Connection, document_id: i64) -> Resultat<i64> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM reglement WHERE document_id = ?1",
        [document_id],
        |r| r.get(0),
    )?)
}

pub(crate) fn situation_de(montant_du: Montant, regle: Montant) -> Situation {
    Situation {
        montant_du,
        regle,
        reste_a_payer: (montant_du - regle).max(0),
        etat: etat_paiement(montant_du, regle),
    }
}

fn recoit(d: &Document) -> bool {
    d.type_document.recoit_des_reglements() && d.statut.en_vigueur()
}

/// Situation de paiement d'un document : rien hors facture et bon de commande,
/// ni pour un brouillon ou un document annulé.
pub fn situation(conn: &Connection, d: &Document) -> Resultat<Option<Situation>> {
    if !recoit(d) {
        return Ok(None);
    }
    let avoirs = if d.type_document == TypeDocument::Facture {
        avoirs_valides(conn, d.id)?
    } else {
        0
    };
    Ok(Some(situation_de(
        d.totaux.ttc - avoirs,
        total_regle(conn, d.id)?,
    )))
}

const COLONNES: &str = "r.id, r.document_id, r.date, r.montant, r.mode_paiement, r.reference,
    COALESCE(u.nom_complet, '')";

fn ligne_vers_reglement(r: &rusqlite::Row<'_>) -> rusqlite::Result<Reglement> {
    Ok(Reglement {
        id: r.get(0)?,
        document_id: r.get(1)?,
        date: lire_date(r, 2)?,
        montant: r.get(3)?,
        mode_paiement: r.get(4)?,
        reference: r.get(5)?,
        auteur: r.get(6)?,
    })
}

/// Règlements d'un document, du plus ancien au plus récent.
pub fn reglements(conn: &Connection, document_id: i64) -> Resultat<Vec<Reglement>> {
    let mut requete = conn.prepare(&format!(
        "SELECT {COLONNES} FROM reglement r LEFT JOIN utilisateur u ON u.id = r.utilisateur_id
         WHERE r.document_id = ?1 ORDER BY r.date, r.id"
    ))?;
    let lignes = requete
        .query_map([document_id], ligne_vers_reglement)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(lignes)
}

fn reglement_refuse(e: ReglementRefuse) -> Erreur {
    match e {
        ReglementRefuse::MontantNonPositif => {
            Erreur::Validation("le montant du règlement doit être positif".into())
        }
        ReglementRefuse::DepasseLeMontantDu { reste_a_payer } => refus(
            "le règlement dépasse le reste à payer ({} FCFA)",
            &[&montant_en_texte(reste_a_payer.max(0))],
        ),
    }
}

fn en_json(r: &Reglement) -> Value {
    json!({
        "reglement": r.id,
        "date": date_texte(r.date),
        "montant": r.montant,
        "mode_paiement": r.mode_paiement,
        "reference": r.reference,
    })
}

fn journaliser(
    conn: &Connection,
    document_id: i64,
    action: &str,
    ancien: Option<Value>,
    nouveau: Option<Value>,
    acteur: &Utilisateur,
) -> Resultat<()> {
    audit::journaliser(
        conn,
        Entree {
            entite: "document",
            entite_id: document_id,
            action,
            ancienne_valeur: ancien,
            nouvelle_valeur: nouveau,
            utilisateur_id: Some(acteur.id),
        },
    )
}

/// Enregistre un règlement sur une facture ou un bon de commande en vigueur.
pub fn ajouter(
    conn: &mut Connection,
    acteur: &Utilisateur,
    document_id: i64,
    saisie: &SaisieReglement,
) -> Resultat<i64> {
    crate::licence::exiger(conn, kadenz_domaine::module::Module::Paiements)?;
    let tx = conn.transaction()?;
    exiger(&tx, acteur, Droit::SaisirReglement)?;
    let d = lire_existant(&tx, document_id)?;
    if !d.type_document.recoit_des_reglements() {
        return Err(Erreur::Validation(
            "seuls une facture ou un bon de commande reçoivent des règlements".into(),
        ));
    }
    if !d.statut.en_vigueur() {
        return Err(Erreur::Validation(
            "un règlement se saisit sur un document validé et non annulé".into(),
        ));
    }
    if saisie.date < d.saisie.date {
        return Err(refus(
            "la date du règlement ne peut pas précéder celle du document ({})",
            &[&d.saisie.date.format("%d/%m/%Y")],
        ));
    }
    if saisie.date > aujourd_hui() {
        return Err(Erreur::Validation(
            "la date du règlement ne peut pas être dans le futur".into(),
        ));
    }
    let mode = saisie.mode_paiement.trim();
    if mode.is_empty() {
        return Err(Erreur::Validation(
            "le mode de paiement du règlement est obligatoire".into(),
        ));
    }
    let s = situation(&tx, &d)?.expect("document en vigueur qui reçoit des règlements");
    verifier_reglement(s.montant_du, s.regle, saisie.montant).map_err(reglement_refuse)?;
    tx.execute(
        "INSERT INTO reglement (document_id, date, montant, mode_paiement, reference, utilisateur_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            document_id,
            date_texte(saisie.date),
            saisie.montant,
            mode,
            saisie.reference.trim(),
            acteur.id
        ],
    )?;
    let id = tx.last_insert_rowid();
    let r = Reglement {
        id,
        document_id,
        date: saisie.date,
        montant: saisie.montant,
        mode_paiement: mode.to_owned(),
        reference: saisie.reference.trim().to_owned(),
        auteur: String::new(),
    };
    journaliser(
        &tx,
        document_id,
        "reglement",
        None,
        Some(en_json(&r)),
        acteur,
    )?;
    tx.commit()?;
    Ok(id)
}

/// Supprime un règlement saisi par erreur ; la valeur supprimée reste au journal.
pub fn supprimer(conn: &mut Connection, acteur: &Utilisateur, reglement_id: i64) -> Resultat<()> {
    crate::licence::exiger(conn, kadenz_domaine::module::Module::Paiements)?;
    let tx = conn.transaction()?;
    exiger(&tx, acteur, Droit::SaisirReglement)?;
    let r = tx
        .query_row(
            &format!(
                "SELECT {COLONNES} FROM reglement r
                 LEFT JOIN utilisateur u ON u.id = r.utilisateur_id WHERE r.id = ?1"
            ),
            [reglement_id],
            ligne_vers_reglement,
        )
        .optional()?
        .ok_or_else(|| Erreur::Validation("ce règlement n'existe plus".into()))?;
    tx.execute("DELETE FROM reglement WHERE id = ?1", [reglement_id])?;
    journaliser(
        &tx,
        r.document_id,
        "suppression_reglement",
        Some(en_json(&r)),
        None,
        acteur,
    )?;
    tx.commit()?;
    Ok(())
}

/// Registre des encaissements et des décaissements, filtré par sens, période
/// (dates incluses) et mode de paiement.
pub fn registre(conn: &Connection, filtre: &FiltreRegistre) -> Resultat<Registre> {
    let mut requete = conn.prepare(&format!(
        "SELECT {COLONNES}, d.type, COALESCE(d.numero, ''), d.tiers_nom_snapshot
         FROM reglement r
         JOIN document d ON d.id = r.document_id
         LEFT JOIN utilisateur u ON u.id = r.utilisateur_id
         WHERE (?1 IS NULL OR d.type = ?1)
           AND (?2 IS NULL OR r.date >= ?2)
           AND (?3 IS NULL OR r.date <= ?3)
           AND (?4 IS NULL OR r.mode_paiement = ?4)
         ORDER BY r.date, r.id"
    ))?;
    let type_du_sens = filtre.sens.map(|s| match s {
        Sens::Encaissement => TypeDocument::Facture.code(),
        Sens::Decaissement => TypeDocument::BonCommande.code(),
    });
    let lignes = requete
        .query_map(
            params![
                type_du_sens,
                filtre.du.map(date_texte),
                filtre.au.map(date_texte),
                filtre.mode.as_deref().map(str::trim)
            ],
            |r| {
                let type_document = lire_type(r, 7)?;
                Ok(LigneRegistre {
                    reglement: ligne_vers_reglement(r)?,
                    sens: Sens::du_type(type_document).unwrap_or(Sens::Encaissement),
                    type_document,
                    numero: r.get(8)?,
                    tiers: r.get(9)?,
                })
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let total_du_sens = |sens: Sens| {
        let mut t = TotalSens::default();
        for l in lignes.iter().filter(|l| l.sens == sens) {
            let mode = &l.reglement.mode_paiement;
            t.total += l.reglement.montant;
            match t.par_mode.iter_mut().find(|(m, _)| m == mode) {
                Some((_, total)) => *total += l.reglement.montant,
                None => t.par_mode.push((mode.clone(), l.reglement.montant)),
            }
        }
        t.par_mode.sort_by(|a, b| a.0.cmp(&b.0));
        t
    };
    Ok(Registre {
        encaissements: total_du_sens(Sens::Encaissement),
        decaissements: total_du_sens(Sens::Decaissement),
        lignes,
    })
}

/// Documents d'un type, en vigueur, qui restent à payer.
fn documents_dus(conn: &Connection, type_document: TypeDocument) -> Resultat<Vec<DocumentDu>> {
    let mut requete = conn.prepare(
        "SELECT d.id, d.type, COALESCE(d.numero, ''), d.statut, d.date, d.date_echeance,
                d.tiers_nom_snapshot, d.montant_ttc,
                (SELECT COALESCE(SUM(a.montant_ttc), 0) FROM document a
                  WHERE a.document_source_id = d.id AND a.type = 'avoir'
                    AND a.statut IN ('valide', 'envoye')),
                (SELECT COALESCE(SUM(r.montant), 0) FROM reglement r WHERE r.document_id = d.id)
         FROM document d
         WHERE d.type = ?1 AND d.statut NOT IN ('brouillon', 'annule')
         ORDER BY d.date, d.id",
    )?;
    let lignes = requete
        .query_map([type_document.code()], |r| {
            let code: String = r.get(3)?;
            let statut = Statut::depuis_code(&code).unwrap_or(Statut::Valide);
            let ttc: Montant = r.get(7)?;
            let avoirs: Montant = r.get(8)?;
            let regle: Montant = r.get(9)?;
            let s = situation_de(ttc - avoirs, regle);
            Ok(DocumentDu {
                id: r.get(0)?,
                type_document: lire_type(r, 1)?,
                numero: r.get(2)?,
                statut,
                date: lire_date(r, 4)?,
                echeance: lire_date_facultative(r, 5)?,
                tiers: r.get(6)?,
                montant_du: s.montant_du,
                regle: s.regle,
                reste_a_payer: s.reste_a_payer,
                jours_de_retard: None,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(lignes.into_iter().filter(|d| d.reste_a_payer > 0).collect())
}

/// Suivi fournisseurs : bons de commande en vigueur non payés ou payés en
/// partie, du plus ancien au plus récent.
pub fn bons_de_commande_non_payes(conn: &Connection) -> Resultat<Vec<DocumentDu>> {
    documents_dus(conn, TypeDocument::BonCommande)
}

/// Relances : factures validées ou envoyées dont l'échéance est passée au jour
/// donné et qui restent à payer, de la plus en retard à la moins en retard.
pub fn factures_echues(conn: &Connection, aujourd_hui: NaiveDate) -> Resultat<Vec<DocumentDu>> {
    let mut echues: Vec<DocumentDu> = documents_dus(conn, TypeDocument::Facture)?
        .into_iter()
        .filter_map(|mut d| {
            let echeance = d.echeance?;
            let retard = (aujourd_hui - echeance).num_days();
            (retard > 0).then(|| {
                d.jours_de_retard = Some(retard);
                d
            })
        })
        .collect();
    echues.sort_by(|a, b| {
        b.jours_de_retard
            .cmp(&a.jours_de_retard)
            .then(a.id.cmp(&b.id))
    });
    Ok(echues)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::documents::{self, LigneSaisie, Saisie};
    use crate::droits::tests::compte;
    use crate::ouvrir_en_memoire;
    use crate::tiers::{self, FicheTiers, TypeTiers};
    use kadenz_domaine::calcul::{Taux, UNE_UNITE};
    use kadenz_domaine::role::Role;

    struct Contexte {
        conn: Connection,
        admin: Utilisateur,
        vente: Utilisateur,
        client: i64,
        fournisseur: i64,
    }

    fn contexte() -> Contexte {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        let vente = compte(&conn, "vente", Role::Commercial);
        let mut fiche = FicheTiers::vide(TypeTiers::Client);
        fiche.nom = "Client Alpha".into();
        let client = tiers::creer(&conn, &admin, &fiche).unwrap().id;
        let mut fiche = FicheTiers::vide(TypeTiers::Fournisseur);
        fiche.nom = "Fournisseur Beta".into();
        let fournisseur = tiers::creer(&conn, &admin, &fiche).unwrap().id;
        Contexte {
            conn,
            admin,
            vente,
            client,
            fournisseur,
        }
    }

    fn date(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    /// Document validé de 100 000 FCFA TTC (TVA 0), daté du 3 mai 2026.
    fn valide(c: &mut Contexte, t: TypeDocument, echeance: Option<NaiveDate>) -> i64 {
        let admin = c.admin.clone();
        let s = Saisie {
            tiers_id: Some(if t == TypeDocument::BonCommande {
                c.fournisseur
            } else {
                c.client
            }),
            taux: Taux {
                remise: 0,
                tva: 0,
                retenue: 0,
            },
            date_echeance: echeance,
            lignes: vec![LigneSaisie {
                designation: "Pelle".into(),
                prix: 100_000,
                quantite: UNE_UNITE,
                ..Default::default()
            }],
            ..Saisie::vide(date(2026, 5, 3))
        };
        let id = documents::creer_brouillon(&mut c.conn, &admin, t, &s).unwrap();
        documents::valider(&mut c.conn, &admin, id).unwrap();
        id
    }

    fn saisie(jour: NaiveDate, montant: Montant) -> SaisieReglement {
        SaisieReglement {
            date: jour,
            montant,
            mode_paiement: " Espèce ".into(),
            reference: String::new(),
        }
    }

    fn situation_de_id(c: &Contexte, id: i64) -> Situation {
        let d = documents::lire(&c.conn, id).unwrap().unwrap();
        situation(&c.conn, &d).unwrap().unwrap()
    }

    #[test]
    fn reglements_echelonnes_jusqu_au_paiement_complet() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let f = valide(&mut c, TypeDocument::Facture, None);
        let s = situation_de_id(&c, f);
        assert_eq!(
            (s.montant_du, s.regle, s.reste_a_payer, s.etat),
            (100_000, 0, 100_000, EtatPaiement::Impaye)
        );
        ajouter(&mut c.conn, &admin, f, &saisie(date(2026, 5, 3), 40_000)).unwrap();
        let s = situation_de_id(&c, f);
        assert_eq!((s.reste_a_payer, s.etat), (60_000, EtatPaiement::Partiel));
        // Au-delà du reste à payer : refusé.
        let e = ajouter(&mut c.conn, &admin, f, &saisie(date(2026, 5, 4), 60_001)).unwrap_err();
        assert_eq!(
            e.to_string(),
            "le règlement dépasse le reste à payer (60\u{a0}000 FCFA)"
        );
        ajouter(&mut c.conn, &admin, f, &saisie(date(2026, 5, 4), 60_000)).unwrap();
        let s = situation_de_id(&c, f);
        assert_eq!((s.reste_a_payer, s.etat), (0, EtatPaiement::Paye));
        let liste = reglements(&c.conn, f).unwrap();
        assert_eq!(liste.len(), 2);
        assert_eq!(liste[0].mode_paiement, "Espèce", "mode nettoyé");
        assert_eq!(liste[0].auteur, "Compte admin");
        assert!(ajouter(&mut c.conn, &admin, f, &saisie(date(2026, 5, 4), 1)).is_err());
    }

    #[test]
    fn reglement_refuse_hors_cas_prevus() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let vente = c.vente.clone();
        let f = valide(&mut c, TypeDocument::Facture, None);
        // Droit « Saisir un règlement » : pas pour un commercial.
        assert!(matches!(
            ajouter(&mut c.conn, &vente, f, &saisie(date(2026, 5, 3), 1)),
            Err(Erreur::Interdit(Droit::SaisirReglement))
        ));
        // Date avant le document, dans le futur ; montant nul ; mode vide.
        assert!(ajouter(&mut c.conn, &admin, f, &saisie(date(2026, 5, 2), 1)).is_err());
        let demain = aujourd_hui().succ_opt().unwrap();
        assert!(ajouter(&mut c.conn, &admin, f, &saisie(demain, 1)).is_err());
        assert!(ajouter(&mut c.conn, &admin, f, &saisie(date(2026, 5, 3), 0)).is_err());
        let sans_mode = SaisieReglement {
            mode_paiement: " ".into(),
            ..saisie(date(2026, 5, 3), 1)
        };
        assert!(ajouter(&mut c.conn, &admin, f, &sans_mode).is_err());
        // Brouillon, devis : refusés.
        let s = Saisie {
            tiers_id: Some(c.client),
            ..Saisie::vide(date(2026, 5, 3))
        };
        let brouillon =
            documents::creer_brouillon(&mut c.conn, &admin, TypeDocument::Facture, &s).unwrap();
        assert!(ajouter(&mut c.conn, &admin, brouillon, &saisie(date(2026, 5, 3), 1)).is_err());
        let devis = valide(&mut c, TypeDocument::Devis, None);
        assert!(ajouter(&mut c.conn, &admin, devis, &saisie(date(2026, 5, 3), 1)).is_err());
        assert!(reglements(&c.conn, f).unwrap().is_empty());
    }

    #[test]
    fn suppression_tracee_et_annulation_refusee_avec_reglements() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let vente = c.vente.clone();
        let f = valide(&mut c, TypeDocument::Facture, None);
        let r = ajouter(&mut c.conn, &admin, f, &saisie(date(2026, 5, 3), 30_000)).unwrap();
        assert!(documents::annuler(&mut c.conn, &admin, f).is_err());
        assert!(supprimer(&mut c.conn, &vente, r).is_err());
        supprimer(&mut c.conn, &admin, r).unwrap();
        assert!(supprimer(&mut c.conn, &admin, r).is_err(), "déjà supprimé");
        assert!(reglements(&c.conn, f).unwrap().is_empty());
        let historique = documents::historique(&c.conn, f).unwrap();
        let libelles: Vec<&str> = historique.iter().map(|e| e.libelle.as_str()).collect();
        assert!(libelles.contains(&"Règlement de 30\u{a0}000 FCFA enregistré"));
        assert!(libelles.contains(&"Règlement de 30\u{a0}000 FCFA supprimé"));
        let entree = audit::lister_pour(&c.conn, "document", f)
            .unwrap()
            .into_iter()
            .find(|e| e.action == "suppression_reglement")
            .unwrap();
        assert_eq!(entree.ancienne_valeur.unwrap()["montant"], 30_000);
        documents::annuler(&mut c.conn, &admin, f).unwrap();
        let d = documents::lire(&c.conn, f).unwrap().unwrap();
        assert_eq!(situation(&c.conn, &d).unwrap(), None);
    }

    #[test]
    fn avoir_et_bon_de_commande() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let f = valide(&mut c, TypeDocument::Facture, None);
        let a = documents::creer_avoir(&mut c.conn, &admin, f).unwrap();
        let mut d = documents::lire(&c.conn, a).unwrap().unwrap();
        d.saisie.lignes[0].prix = 25_000;
        documents::modifier_brouillon(&mut c.conn, &admin, a, &d.saisie).unwrap();
        documents::valider(&mut c.conn, &admin, a).unwrap();
        let s = situation_de_id(&c, f);
        assert_eq!((s.montant_du, s.reste_a_payer), (75_000, 75_000));
        assert!(ajouter(&mut c.conn, &admin, f, &saisie(date(2026, 5, 3), 75_001)).is_err());
        assert!(ajouter(&mut c.conn, &admin, a, &saisie(date(2026, 5, 3), 1)).is_err());
        let bc = valide(&mut c, TypeDocument::BonCommande, None);
        ajouter(&mut c.conn, &admin, bc, &saisie(date(2026, 5, 5), 10_000)).unwrap();
        let dus = bons_de_commande_non_payes(&c.conn).unwrap();
        assert_eq!(dus.len(), 1);
        assert_eq!(
            (dus[0].id, dus[0].tiers.as_str(), dus[0].reste_a_payer),
            (bc, "Fournisseur Beta", 90_000)
        );
        ajouter(&mut c.conn, &admin, bc, &saisie(date(2026, 5, 6), 90_000)).unwrap();
        assert!(bons_de_commande_non_payes(&c.conn).unwrap().is_empty());
    }

    #[test]
    fn registre_filtre_et_totaux_par_mode() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let f = valide(&mut c, TypeDocument::Facture, None);
        let bc = valide(&mut c, TypeDocument::BonCommande, None);
        let mut cheque = saisie(date(2026, 5, 10), 20_000);
        cheque.mode_paiement = "Chèque".into();
        cheque.reference = "CHQ 0001".into();
        ajouter(&mut c.conn, &admin, f, &saisie(date(2026, 5, 3), 10_000)).unwrap();
        ajouter(&mut c.conn, &admin, f, &cheque).unwrap();
        ajouter(&mut c.conn, &admin, bc, &saisie(date(2026, 5, 4), 5_000)).unwrap();
        let tout = registre(&c.conn, &FiltreRegistre::default()).unwrap();
        assert_eq!(tout.lignes.len(), 3);
        assert_eq!(tout.encaissements.total, 30_000);
        assert_eq!(
            tout.encaissements.par_mode,
            [
                ("Chèque".to_string(), 20_000),
                ("Espèce".to_string(), 10_000)
            ]
        );
        assert_eq!(
            tout.decaissements,
            TotalSens {
                total: 5_000,
                par_mode: vec![("Espèce".to_string(), 5_000)]
            }
        );
        let encaissements = registre(
            &c.conn,
            &FiltreRegistre {
                sens: Some(Sens::Encaissement),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(encaissements.encaissements.total, 30_000);
        assert_eq!(encaissements.decaissements, TotalSens::default());
        assert!(
            encaissements
                .lignes
                .iter()
                .all(|l| l.sens == Sens::Encaissement && l.tiers == "Client Alpha")
        );
        let periode = registre(
            &c.conn,
            &FiltreRegistre {
                du: Some(date(2026, 5, 4)),
                au: Some(date(2026, 5, 10)),
                mode: Some("Chèque".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(periode.lignes.len(), 1);
        assert_eq!(periode.lignes[0].reglement.reference, "CHQ 0001");
        assert!(periode.lignes[0].numero.starts_with('F'));
    }

    #[test]
    fn factures_echues_selon_l_echeance() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let sans_echeance = valide(&mut c, TypeDocument::Facture, None);
        let f1 = valide(&mut c, TypeDocument::Facture, Some(date(2026, 6, 1)));
        let f2 = valide(&mut c, TypeDocument::Facture, Some(date(2026, 6, 10)));
        let payee = valide(&mut c, TypeDocument::Facture, Some(date(2026, 6, 1)));
        ajouter(
            &mut c.conn,
            &admin,
            payee,
            &saisie(date(2026, 5, 3), 100_000),
        )
        .unwrap();
        ajouter(&mut c.conn, &admin, f2, &saisie(date(2026, 5, 3), 30_000)).unwrap();
        // Le jour de l'échéance, la facture n'est pas encore échue.
        assert!(
            factures_echues(&c.conn, date(2026, 6, 1))
                .unwrap()
                .is_empty()
        );
        let echues = factures_echues(&c.conn, date(2026, 6, 15)).unwrap();
        let resume: Vec<(i64, Option<i64>, Montant)> = echues
            .iter()
            .map(|d| (d.id, d.jours_de_retard, d.reste_a_payer))
            .collect();
        assert_eq!(resume, [(f1, Some(14), 100_000), (f2, Some(5), 70_000)]);
        assert!(echues.iter().all(|d| d.id != sans_echeance));
    }
}
