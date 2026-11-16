//! Export comptable SYSCOHADA (`docs/spec-export-syscohada.md`, jalon C1) :
//! comptes réglables, journal des ventes et des encaissements, TVA collectée,
//! clôture d'un mois et format CSV.
//!
//! Règles appliquées ici, quel que soit l'écran :
//! - le journal part des totaux figés des factures et des avoirs en vigueur
//!   (validés ou envoyés, jamais annulés) et des règlements des factures ; rien
//!   n'est recalculé, la ventilation entre marchandises et services suit la
//!   nature figée sur chaque ligne à la validation ;
//! - une seule fonction calcule le journal, sur un poste depuis sa copie comme
//!   sur le serveur pour contrôler une clôture ;
//! - un mois clos garde les comptes en vigueur à sa clôture ; ses documents et
//!   ses règlements ne changent plus (validation, annulation, ajout ou
//!   suppression d'un règlement refusés), la correction passe par un avoir daté
//!   d'un mois ouvert ;
//! - régler les comptes et clore un mois demandent le droit « Exporter la
//!   comptabilité et clore une période ».

use std::collections::BTreeMap;
use std::fmt;

use chrono::{Datelike, NaiveDate};
use kadenz_domaine::calcul::{Montant, Pourcentage};
use kadenz_domaine::comptabilite::{NatureComptable, ventiler};
use kadenz_domaine::document::{Statut, TypeDocument};
use kadenz_domaine::role::Droit;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::audit::{self, Entree};
use crate::comptes::Utilisateur;
use crate::documents::{aujourd_hui, date_texte, lire_date, lire_type};
use crate::droits::exiger;
use crate::{Erreur, Resultat, maintenant, refus};

/// Modèles des refus de ce module, pour les traductions de l'interface.
pub const MODELES_ERREUR: &[&str] = &[
    "le mois {} est clos : une facture ou un avoir daté de ce mois ne peut plus être validé ni annulé",
    "le mois {} est clos : un règlement daté de ce mois ne peut plus être ajouté ni supprimé",
    "le mois {} n'est pas terminé : il peut être clos à partir du {}",
    "le mois {} est déjà clos",
    "copie pas à jour : synchronisez puis recommencez",
    "le compte « {} » est obligatoire",
    "le mode de paiement « {} » n'a pas de compte et le compte d'attente est vide : renseigner l'un ou l'autre avant de clore",
    "la pièce {} est déséquilibrée : son TTC n'est pas la base plus la TVA",
    "mois illisible : {}",
    "le mode de paiement « {} » a deux comptes",
];

const CLE_COMPTES: &str = "comptabilite.comptes";
const CLE_CLOTURES: &str = "comptabilite.clotures";

/// Mois civil.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Mois {
    pub annee: i32,
    pub mois: u32,
}

impl Mois {
    pub fn de_date(d: NaiveDate) -> Mois {
        Mois {
            annee: d.year(),
            mois: d.month(),
        }
    }

    /// `AAAA-MM`.
    pub fn code(&self) -> String {
        format!("{:04}-{:02}", self.annee, self.mois)
    }

    pub fn depuis_code(code: &str) -> Option<Mois> {
        let (a, m) = code.trim().split_once('-')?;
        let (annee, mois) = (a.parse().ok()?, m.parse().ok()?);
        NaiveDate::from_ymd_opt(annee, mois, 1)?;
        Some(Mois { annee, mois })
    }

    pub fn premier_jour(&self) -> NaiveDate {
        NaiveDate::from_ymd_opt(self.annee, self.mois, 1).expect("mois valide")
    }

    pub fn suivant(&self) -> Mois {
        if self.mois == 12 {
            Mois {
                annee: self.annee + 1,
                mois: 1,
            }
        } else {
            Mois {
                annee: self.annee,
                mois: self.mois + 1,
            }
        }
    }

    pub fn dernier_jour(&self) -> NaiveDate {
        self.suivant()
            .premier_jour()
            .pred_opt()
            .expect("jour précédent")
    }
}

impl fmt::Display for Mois {
    /// `MM/AAAA`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}/{:04}", self.mois, self.annee)
    }
}

/// Journal de l'export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Journal {
    Ventes,
    Encaissements,
}

/// Comptes de l'export (spec 4.3) : valeurs par défaut relevées sur le plan
/// de comptes officiel, chacune à confirmer par un comptable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComptesComptables {
    pub clients: String,
    pub ventes_marchandises: String,
    pub ventes_services: String,
    pub tva_marchandises: String,
    pub tva_services: String,
    /// Compte de trésorerie de chaque mode de paiement : (mode, compte).
    pub tresorerie: Vec<(String, String)>,
    /// Compte d'un mode de paiement absent de `tresorerie` ; vide par défaut,
    /// faute de source pour cet usage.
    pub attente: String,
    pub journal_ventes: String,
    pub journal_encaissements: String,
}

impl Default for ComptesComptables {
    fn default() -> Self {
        let c = |s: &str| s.to_string();
        ComptesComptables {
            clients: c("4111"),
            ventes_marchandises: c("7011"),
            ventes_services: c("7061"),
            tva_marchandises: c("4431"),
            tva_services: c("4432"),
            tresorerie: [
                ("Espèce", "5711"),
                ("Chèque", "513"),
                ("Virement bancaire", "5211"),
                ("Carte bancaire", "515"),
                ("Moov money", "552"),
                ("Orange money", "552"),
            ]
            .into_iter()
            .map(|(m, k)| (c(m), c(k)))
            .collect(),
            attente: String::new(),
            journal_ventes: c("VT"),
            journal_encaissements: c("EN"),
        }
    }
}

impl ComptesComptables {
    fn ventes(&self, n: NatureComptable) -> &str {
        match n {
            NatureComptable::Marchandise => &self.ventes_marchandises,
            NatureComptable::Service => &self.ventes_services,
        }
    }

    fn tva(&self, n: NatureComptable) -> &str {
        match n {
            NatureComptable::Marchandise => &self.tva_marchandises,
            NatureComptable::Service => &self.tva_services,
        }
    }

    /// Compte d'un mode de paiement, le compte d'attente à défaut (vide si
    /// lui aussi l'est).
    fn tresorerie_de(&self, mode: &str) -> &str {
        let mode = mode.trim();
        self.tresorerie
            .iter()
            .find(|(m, _)| m.trim().eq_ignore_ascii_case(mode) || m.trim() == mode)
            .map(|(_, k)| k.as_str())
            .unwrap_or(&self.attente)
    }

    fn nettoyes(&self) -> Resultat<ComptesComptables> {
        let t = |s: &String| s.trim().to_string();
        let c = ComptesComptables {
            clients: t(&self.clients),
            ventes_marchandises: t(&self.ventes_marchandises),
            ventes_services: t(&self.ventes_services),
            tva_marchandises: t(&self.tva_marchandises),
            tva_services: t(&self.tva_services),
            tresorerie: self
                .tresorerie
                .iter()
                .map(|(m, k)| (t(m), t(k)))
                .filter(|(m, _)| !m.is_empty())
                .collect(),
            attente: t(&self.attente),
            journal_ventes: t(&self.journal_ventes),
            journal_encaissements: t(&self.journal_encaissements),
        };
        for (nom, valeur) in [
            ("Compte clients", &c.clients),
            ("Ventes de marchandises", &c.ventes_marchandises),
            ("Services vendus", &c.ventes_services),
            ("TVA facturée sur ventes", &c.tva_marchandises),
            ("TVA facturée sur prestations de services", &c.tva_services),
            ("Journal des ventes", &c.journal_ventes),
            ("Journal des encaissements", &c.journal_encaissements),
        ] {
            if valeur.is_empty() {
                return Err(refus(MODELES_ERREUR[5], &[&nom]));
            }
        }
        for (i, (m, _)) in c.tresorerie.iter().enumerate() {
            if c.tresorerie[..i].iter().any(|(autre, _)| autre == m) {
                return Err(refus(MODELES_ERREUR[9], &[&m]));
            }
        }
        Ok(c)
    }
}

/// Forme JSON des comptes, pour l'action d'un poste relié.
pub fn comptes_en_json(c: &ComptesComptables) -> Value {
    json!({
        "clients": c.clients,
        "ventes_marchandises": c.ventes_marchandises,
        "ventes_services": c.ventes_services,
        "tva_marchandises": c.tva_marchandises,
        "tva_services": c.tva_services,
        "tresorerie": c.tresorerie.iter()
            .map(|(m, k)| json!({ "mode": m, "compte": k }))
            .collect::<Vec<_>>(),
        "attente": c.attente,
        "journal_ventes": c.journal_ventes,
        "journal_encaissements": c.journal_encaissements,
    })
}

/// Comptes décrits par [`comptes_en_json`] ; une valeur absente prend celle
/// par défaut.
pub fn comptes_depuis_json(v: &Value) -> ComptesComptables {
    let d = ComptesComptables::default();
    let texte = |cle: &str, defaut: String| {
        v.get(cle)
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or(defaut)
    };
    ComptesComptables {
        clients: texte("clients", d.clients),
        ventes_marchandises: texte("ventes_marchandises", d.ventes_marchandises),
        ventes_services: texte("ventes_services", d.ventes_services),
        tva_marchandises: texte("tva_marchandises", d.tva_marchandises),
        tva_services: texte("tva_services", d.tva_services),
        tresorerie: match v.get("tresorerie").and_then(Value::as_array) {
            Some(liste) => liste
                .iter()
                .map(|e| {
                    let champ =
                        |c: &str| e.get(c).and_then(Value::as_str).unwrap_or("").to_string();
                    (champ("mode"), champ("compte"))
                })
                .collect(),
            None => d.tresorerie,
        },
        attente: texte("attente", d.attente),
        journal_ventes: texte("journal_ventes", d.journal_ventes),
        journal_encaissements: texte("journal_encaissements", d.journal_encaissements),
    }
}

fn lire_cle(conn: &Connection, cle: &str) -> Resultat<Option<Value>> {
    let texte: Option<String> = conn
        .query_row(
            "SELECT valeur FROM parametre_systeme WHERE cle = ?1",
            [cle],
            |r| r.get(0),
        )
        .optional()?;
    Ok(texte.and_then(|t| serde_json::from_str(&t).ok()))
}

fn ecrire_cle(conn: &Connection, cle: &str, valeur: &Value) -> Resultat<()> {
    conn.execute(
        "INSERT INTO parametre_systeme (cle, valeur) VALUES (?1, ?2)
         ON CONFLICT (cle) DO UPDATE SET valeur = excluded.valeur",
        params![cle, valeur.to_string()],
    )?;
    Ok(())
}

/// Comptes en vigueur, ceux par défaut s'ils n'ont jamais été réglés.
pub fn comptes(conn: &Connection) -> Resultat<ComptesComptables> {
    Ok(lire_cle(conn, CLE_COMPTES)?
        .map(|v| comptes_depuis_json(&v))
        .unwrap_or_default())
}

/// Réservé au droit « Exporter la comptabilité et clore une période ». Les
/// mois déjà clos gardent leurs comptes.
pub fn enregistrer_comptes(
    conn: &Connection,
    acteur: &Utilisateur,
    c: &ComptesComptables,
) -> Resultat<()> {
    exiger(conn, acteur, Droit::ExporterComptabilite)?;
    let c = c.nettoyes()?;
    let ancien = comptes(conn)?;
    let valeur = comptes_en_json(&c);
    ecrire_cle(conn, CLE_COMPTES, &valeur)?;
    audit::journaliser(
        conn,
        Entree {
            entite: "parametres",
            entite_id: 0,
            action: "comptes_comptables",
            ancienne_valeur: Some(comptes_en_json(&ancien)),
            nouvelle_valeur: Some(valeur),
            utilisateur_id: Some(acteur.id),
        },
    )
}

/// Mois clos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cloture {
    pub mois: Mois,
    /// Empreinte des écritures du mois à sa clôture.
    pub empreinte: String,
    /// Date et heure de la clôture, ISO 8601 local à la seconde.
    pub le: String,
    /// Nom complet du compte qui a clos le mois.
    pub par: String,
    /// Comptes en vigueur à la clôture, ceux de tout export du mois.
    pub comptes: ComptesComptables,
}

fn cloture_en_json(c: &Cloture) -> Value {
    json!({
        "mois": c.mois.code(),
        "empreinte": c.empreinte,
        "le": c.le,
        "par": c.par,
        "comptes": comptes_en_json(&c.comptes),
    })
}

/// Mois clos, du plus ancien au plus récent.
pub fn clotures(conn: &Connection) -> Resultat<Vec<Cloture>> {
    let Some(Value::Array(liste)) = lire_cle(conn, CLE_CLOTURES)? else {
        return Ok(Vec::new());
    };
    let mut clotures: Vec<Cloture> = liste
        .iter()
        .filter_map(|e| {
            let texte = |c: &str| e.get(c).and_then(Value::as_str).unwrap_or("").to_string();
            Some(Cloture {
                mois: Mois::depuis_code(e.get("mois")?.as_str()?)?,
                empreinte: texte("empreinte"),
                le: texte("le"),
                par: texte("par"),
                comptes: e
                    .get("comptes")
                    .map(comptes_depuis_json)
                    .unwrap_or_default(),
            })
        })
        .collect();
    clotures.sort_by_key(|c| c.mois);
    Ok(clotures)
}

/// Vrai si le mois de `date` est clos.
pub fn est_clos(conn: &Connection, date: NaiveDate) -> Resultat<bool> {
    let mois = Mois::de_date(date);
    Ok(clotures(conn)?.iter().any(|c| c.mois == mois))
}

/// Refus d'une facture ou d'un avoir daté dans un mois clos (validation,
/// annulation).
pub(crate) fn exiger_document_ouvert(conn: &Connection, date: NaiveDate) -> Resultat<()> {
    if est_clos(conn, date)? {
        return Err(refus(MODELES_ERREUR[0], &[&Mois::de_date(date)]));
    }
    Ok(())
}

/// Refus d'un règlement de facture daté dans un mois clos (ajout,
/// suppression).
pub(crate) fn exiger_reglement_ouvert(conn: &Connection, date: NaiveDate) -> Resultat<()> {
    if est_clos(conn, date)? {
        return Err(refus(MODELES_ERREUR[1], &[&Mois::de_date(date)]));
    }
    Ok(())
}

/// Période d'un export, dates incluses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Periode {
    pub du: NaiveDate,
    pub au: NaiveDate,
}

impl Periode {
    pub fn du_mois(m: Mois) -> Periode {
        Periode {
            du: m.premier_jour(),
            au: m.dernier_jour(),
        }
    }
}

/// Ligne du journal d'écritures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ecriture {
    pub journal: Journal,
    pub code_journal: String,
    pub date: NaiveDate,
    pub piece: String,
    pub compte: String,
    pub compte_tiers: String,
    pub libelle: String,
    pub debit: Montant,
    pub credit: Montant,
}

/// Journal d'une période.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct JournalComptable {
    pub ecritures: Vec<Ecriture>,
    pub total_debit: Montant,
    pub total_credit: Montant,
    /// Pièces : un document ou un règlement.
    pub pieces: usize,
    /// Modes de paiement sans compte quand le compte d'attente est vide aussi ;
    /// leurs écritures ont un compte vide.
    pub modes_sans_compte: Vec<String>,
    /// Mois de la période qui ne sont pas clos.
    pub mois_non_clos: Vec<Mois>,
}

struct Ecrivain<'a> {
    ecritures: Vec<Ecriture>,
    journal: Journal,
    code: &'a str,
    date: NaiveDate,
    piece: &'a str,
    libelle: &'a str,
}

impl Ecrivain<'_> {
    fn ligne(&mut self, compte: &str, debit: Montant, credit: Montant) {
        if debit == 0 && credit == 0 {
            return;
        }
        self.ecritures.push(Ecriture {
            journal: self.journal,
            code_journal: self.code.to_string(),
            date: self.date,
            piece: self.piece.to_string(),
            compte: compte.to_string(),
            compte_tiers: String::new(),
            libelle: self.libelle.to_string(),
            debit,
            credit,
        });
    }
}

/// Journal des ventes et des encaissements d'une période, dans l'ordre des
/// dates (ventes avant encaissements le même jour). Un mois clos prend les
/// comptes de sa clôture, un mois ouvert ceux en vigueur.
pub fn journal(
    conn: &Connection,
    periode: Periode,
    journaux: &[Journal],
) -> Resultat<JournalComptable> {
    let clos: BTreeMap<Mois, ComptesComptables> = clotures(conn)?
        .into_iter()
        .map(|c| (c.mois, c.comptes))
        .collect();
    let courants = comptes(conn)?;
    let comptes_de = |d: NaiveDate| clos.get(&Mois::de_date(d)).unwrap_or(&courants);
    let mut j = JournalComptable::default();
    let (du, au) = (date_texte(periode.du), date_texte(periode.au));

    if journaux.contains(&Journal::Ventes) {
        let mut stmt = conn.prepare(
            "SELECT id, type, numero, date, tiers_nom_snapshot, montant_base_tva, montant_tva,
                    montant_ttc, pourcentage_remise, pourcentage_tva
             FROM document
             WHERE type IN ('facture', 'avoir') AND statut NOT IN (?1, ?2)
               AND date BETWEEN ?3 AND ?4
             ORDER BY date, type DESC, numero, id",
        )?;
        let documents = stmt
            .query_map(
                params![Statut::Brouillon.code(), Statut::Annule.code(), du, au],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        lire_type(r, 1)?,
                        r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                        lire_date(r, 3)?,
                        r.get::<_, String>(4)?,
                        [
                            r.get::<_, i64>(5)?,
                            r.get(6)?,
                            r.get(7)?,
                            r.get(8)?,
                            r.get(9)?,
                        ],
                    ))
                },
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut lignes = conn.prepare(
            "SELECT nature_snapshot, total FROM ligne_document WHERE document_id = ?1 ORDER BY ordre",
        )?;
        for (id, type_document, numero, date, tiers, [base, tva, ttc, remise, taux_tva]) in
            documents
        {
            if ttc != base + tva {
                return Err(refus(MODELES_ERREUR[7], &[&numero]));
            }
            let natures = lignes
                .query_map([id], |r| {
                    let code: String = r.get(0)?;
                    Ok((
                        NatureComptable::depuis_code(&code).unwrap_or_default(),
                        r.get::<_, Montant>(1)?,
                    ))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let parts = ventiler(base, tva, remise, taux_tva, &natures);
            let c = comptes_de(date);
            let libelle = format!("{type_document} {numero} - {tiers}");
            let mut e = Ecrivain {
                ecritures: Vec::new(),
                journal: Journal::Ventes,
                code: &c.journal_ventes,
                date,
                piece: &numero,
                libelle: &libelle,
            };
            // Facture : client au débit, ventes et TVA au crédit ; l'avoir
            // inverse (spec 2.2 et 4.2).
            let avoir = type_document == TypeDocument::Avoir;
            let sens = |m: Montant| if avoir { (0, m) } else { (m, 0) };
            let (d, k) = sens(ttc);
            e.ligne(&c.clients, d, k);
            for p in &parts {
                let (d, k) = sens(p.base);
                e.ligne(c.ventes(p.nature), k, d);
            }
            for p in &parts {
                let (d, k) = sens(p.tva);
                e.ligne(c.tva(p.nature), k, d);
            }
            if !e.ecritures.is_empty() {
                j.pieces += 1;
                j.ecritures.extend(e.ecritures);
            }
        }
    }

    if journaux.contains(&Journal::Encaissements) {
        let mut stmt = conn.prepare(
            "SELECT r.date, r.montant, r.mode_paiement, d.numero, d.tiers_nom_snapshot
             FROM reglement r JOIN document d ON d.id = r.document_id
             WHERE d.type = 'facture' AND r.date BETWEEN ?1 AND ?2
             ORDER BY r.date, r.id",
        )?;
        let reglements = stmt
            .query_map(params![du, au], |r| {
                Ok((
                    lire_date(r, 0)?,
                    r.get::<_, Montant>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    r.get::<_, String>(4)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for (date, montant, mode, numero, tiers) in reglements {
            let c = comptes_de(date);
            let tresorerie = c.tresorerie_de(&mode);
            if tresorerie.is_empty() && !j.modes_sans_compte.contains(&mode) {
                j.modes_sans_compte.push(mode.clone());
            }
            let libelle = format!("Règlement {numero} - {tiers} ({mode})");
            let mut e = Ecrivain {
                ecritures: Vec::new(),
                journal: Journal::Encaissements,
                code: &c.journal_encaissements,
                date,
                piece: &numero,
                libelle: &libelle,
            };
            e.ligne(tresorerie, montant, 0);
            e.ligne(&c.clients, 0, montant);
            j.pieces += 1;
            j.ecritures.extend(e.ecritures);
        }
    }

    // Tri stable : l'ordre de chaque journal est gardé pour un même jour.
    j.ecritures.sort_by_key(|e| (e.date, e.journal));
    j.total_debit = j.ecritures.iter().map(|e| e.debit).sum();
    j.total_credit = j.ecritures.iter().map(|e| e.credit).sum();
    let mut m = Mois::de_date(periode.du);
    while m <= Mois::de_date(periode.au) {
        if !clos.contains_key(&m) {
            j.mois_non_clos.push(m);
        }
        m = m.suivant();
    }
    Ok(j)
}

/// Empreinte SHA-256 (hexadécimal) d'une suite d'écritures, comptes compris.
fn empreinte_de(ecritures: &[Ecriture]) -> String {
    let mut h = Sha256::new();
    for e in ecritures {
        let ligne = [
            e.code_journal.as_str(),
            &date_texte(e.date),
            &e.piece,
            &e.compte,
            &e.compte_tiers,
            &e.libelle,
            &e.debit.to_string(),
            &e.credit.to_string(),
        ]
        .join("\u{1f}");
        h.update(ligne.as_bytes());
        h.update(b"\n");
    }
    h.finalize().iter().map(|o| format!("{o:02x}")).collect()
}

/// Empreinte des écritures du mois, que le poste envoie avec la clôture.
pub fn empreinte(conn: &Connection, mois: Mois) -> Resultat<String> {
    let j = journal(
        conn,
        Periode::du_mois(mois),
        &[Journal::Ventes, Journal::Encaissements],
    )?;
    Ok(empreinte_de(&j.ecritures))
}

/// Clôt `mois` : refusé pour un mois pas encore terminé ou déjà clos, une
/// écriture sans compte, ou une empreinte différente de celle de cette base
/// (copie du poste pas à jour). Pas de réouverture (spec 4.6).
pub fn clore(
    conn: &Connection,
    acteur: &Utilisateur,
    mois: Mois,
    empreinte_attendue: &str,
) -> Resultat<()> {
    exiger(conn, acteur, Droit::ExporterComptabilite)?;
    if aujourd_hui() <= mois.dernier_jour() {
        return Err(refus(
            MODELES_ERREUR[2],
            &[&mois, &mois.suivant().premier_jour().format("%d/%m/%Y")],
        ));
    }
    let mut liste = clotures(conn)?;
    if liste.iter().any(|c| c.mois == mois) {
        return Err(refus(MODELES_ERREUR[3], &[&mois]));
    }
    let j = journal(
        conn,
        Periode::du_mois(mois),
        &[Journal::Ventes, Journal::Encaissements],
    )?;
    if let Some(mode) = j.modes_sans_compte.first() {
        return Err(refus(MODELES_ERREUR[6], &[&mode]));
    }
    let calculee = empreinte_de(&j.ecritures);
    if calculee != empreinte_attendue {
        return Err(Erreur::Validation(MODELES_ERREUR[4].into()));
    }
    let cloture = Cloture {
        mois,
        empreinte: calculee,
        le: maintenant(),
        par: acteur.nom_complet.clone(),
        comptes: comptes(conn)?,
    };
    let entree = cloture_en_json(&cloture);
    liste.push(cloture);
    liste.sort_by_key(|c| c.mois);
    ecrire_cle(
        conn,
        CLE_CLOTURES,
        &Value::Array(liste.iter().map(cloture_en_json).collect()),
    )?;
    audit::journaliser(
        conn,
        Entree {
            entite: "parametres",
            entite_id: 0,
            action: "cloture_mois",
            ancienne_valeur: None,
            nouvelle_valeur: Some(entree),
            utilisateur_id: Some(acteur.id),
        },
    )
}

/// Ligne de l'état de la TVA collectée : factures moins avoirs, chacun dans
/// le mois de sa propre date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LigneTva {
    pub mois: Mois,
    /// Taux en centièmes de pour cent (1 800 pour 18 %).
    pub taux: Pourcentage,
    pub base: Montant,
    pub tva: Montant,
}

/// TVA collectée d'une période, par mois et par taux.
pub fn tva_collectee(conn: &Connection, periode: Periode) -> Resultat<Vec<LigneTva>> {
    let mut stmt = conn.prepare(
        "SELECT substr(date, 1, 7), pourcentage_tva,
                SUM(CASE type WHEN 'avoir' THEN -montant_base_tva ELSE montant_base_tva END),
                SUM(CASE type WHEN 'avoir' THEN -montant_tva ELSE montant_tva END)
         FROM document
         WHERE type IN ('facture', 'avoir') AND statut NOT IN (?1, ?2)
           AND date BETWEEN ?3 AND ?4
         GROUP BY 1, 2 ORDER BY 1, 2",
    )?;
    let lignes = stmt
        .query_map(
            params![
                Statut::Brouillon.code(),
                Statut::Annule.code(),
                date_texte(periode.du),
                date_texte(periode.au)
            ],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, i64>(3)?,
                ))
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    lignes
        .into_iter()
        .map(|(m, taux, base, tva)| {
            Ok(LigneTva {
                mois: Mois::depuis_code(&m).ok_or_else(|| refus(MODELES_ERREUR[8], &[&m]))?,
                taux,
                base,
                tva,
            })
        })
        .collect()
}

/// Champ CSV : entre guillemets s'il contient un point-virgule, un guillemet
/// ou un retour à la ligne, guillemet doublé (RFC 4180).
fn champ_csv(s: &str) -> String {
    if s.contains([';', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Journal au format CSV de la spec (4.8) : UTF-8 avec BOM, point-virgule,
/// fin de ligne CRLF, une ligne d'en-tête, dates JJ/MM/AAAA, montants entiers
/// en FCFA avec 0 dans la colonne vide, aucune ligne de total.
pub fn csv(ecritures: &[Ecriture]) -> Vec<u8> {
    let mut s =
        String::from("\u{feff}Journal;Date;Pièce;Compte;Compte tiers;Libellé;Débit;Crédit\r\n");
    for e in ecritures {
        let champs = [
            champ_csv(&e.code_journal),
            e.date.format("%d/%m/%Y").to_string(),
            champ_csv(&e.piece),
            champ_csv(&e.compte),
            champ_csv(&e.compte_tiers),
            champ_csv(&e.libelle),
            e.debit.to_string(),
            e.credit.to_string(),
        ];
        s.push_str(&champs.join(";"));
        s.push_str("\r\n");
    }
    s.into_bytes()
}

/// Nom du fichier CSV : `journal-AAAA-MM.csv` pour un mois entier,
/// `journal-AAAA-MM-JJ_AAAA-MM-JJ.csv` sinon ; `-provisoire` avant
/// l'extension si un mois de la période n'est pas clos.
pub fn nom_fichier(du: NaiveDate, au: NaiveDate, provisoire: bool) -> String {
    let m = Mois::de_date(du);
    let base = if du == m.premier_jour() && au == m.dernier_jour() {
        format!("journal-{}", m.code())
    } else {
        format!("journal-{}_{}", date_texte(du), date_texte(au))
    };
    format!("{base}{}.csv", if provisoire { "-provisoire" } else { "" })
}

#[cfg(test)]
mod tests;
