//! Documents commerciaux : brouillon, validation numérotée, envoi, annulation
//! (cahier des charges, sections 4.2, 6.1, 7 et 11).
//!
//! Règles appliquées ici, quel que soit l'écran :
//! - les totaux sont toujours recalculés par le domaine, à chaque enregistrement
//!   du brouillon et dans la transaction de validation ;
//! - les lignes enregistrent exactement la saisie (référence, désignation, prix) ;
//!   le lien vers le produit sert seulement à la navigation ;
//! - le snapshot du tiers est repris de sa fiche à chaque enregistrement du
//!   brouillon, puis figé à la validation ;
//! - le numéro est attribué à la validation, dans la même transaction que le
//!   compteur, d'après l'année et le mois de la date du document.
//!
//! Droits (section 10.1, complétée pour les actions que la matrice ne nomme pas) :
//! créer, modifier, supprimer ou annuler un brouillon demande le droit de
//! brouillon du type ; valider et marquer envoyé demandent « Valider un document » ;
//! annuler un document validé demande « Annuler un document validé » ; les champs
//! administratifs d'un document validé (mode de paiement, observations) suivent
//! le droit de brouillon du type.
//!
//! Documents liés (jalon J5, sections 6.7, 7.2 et 7.7) : conversion d'un devis
//! ou d'une proforma en facture (la source passe en Converti à la validation de
//! la facture), duplication simple ou correction (l'original est annulé), avoir
//! créé depuis une facture validée (lignes reprises, client et taux de la
//! facture, total des avoirs validés plafonné au TTC de la facture).

use chrono::NaiveDate;
use kadenz_domaine::calcul::{
    CENT_POUR_CENT, Montant, Quantite, Taux, Totaux, calculer_totaux, total_ligne,
};
use kadenz_domaine::document::{Avancement, TransitionInterdite, avancement, verifier_transition};
pub use kadenz_domaine::document::{Statut, TypeDocument};
pub use kadenz_domaine::paiement::EtatPaiement;
use kadenz_domaine::paiement::etat_paiement;
use kadenz_domaine::role::Droit;
use kadenz_domaine::saisie::montant_en_texte;
use kadenz_domaine::texte;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use crate::audit::{self, Entree, EntreeLue};
use crate::comptes::Utilisateur;
use crate::droits::exiger;
use crate::{Erreur, Resultat, composer, maintenant, refus};

const FORMAT_DATE: &str = "%Y-%m-%d";

/// Ligne telle que saisie sur la fiche.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LigneSaisie {
    /// Produit d'origine, pour la navigation ; `None` pour une ligne libre.
    pub produit_id: Option<i64>,
    pub reference: String,
    pub designation: String,
    pub prix: Montant,
    pub quantite: Quantite,
    /// Quantité livrée d'un bordereau, saisie dans le brouillon ; quantité reçue
    /// d'un bon de commande, mise à jour par `enregistrer_reception` seulement.
    /// Toujours 0 pour les autres types.
    pub quantite_livree: Quantite,
    pub observations: String,
}

/// Contenu modifiable d'un brouillon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Saisie {
    pub date: NaiveDate,
    pub tiers_id: Option<i64>,
    pub libelle: String,
    pub taux: Taux,
    pub mode_paiement: String,
    pub observations: String,
    pub date_echeance: Option<NaiveDate>,
    pub lignes: Vec<LigneSaisie>,
    /// Visas et dates d'un bordereau de livraison ; vides pour les autres types.
    pub livraison: ChampsLivraison,
}

/// Zones de visa et dates d'un bordereau, en texte libre (section 6.5).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChampsLivraison {
    pub visa_client: String,
    pub visa_fournisseur: String,
    pub date_livraison: String,
    pub date_reception: String,
}

impl Saisie {
    pub fn vide(date: NaiveDate) -> Saisie {
        Saisie {
            date,
            tiers_id: None,
            libelle: String::new(),
            taux: Taux::default(),
            mode_paiement: String::new(),
            observations: String::new(),
            date_echeance: None,
            lignes: Vec::new(),
            livraison: ChampsLivraison::default(),
        }
    }
}

/// Informations du tiers copiées dans le document (section 7.6).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SnapshotTiers {
    pub nom: String,
    pub adresse: String,
    pub ifu: String,
    pub rccm: String,
    pub regime: String,
}

/// Nature du lien entre un document et sa source (migration 0004).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NatureLien {
    /// Facture issue d'un devis ou d'une proforma.
    Conversion,
    /// Copie d'un devis ou d'une proforma dont l'original a été annulé.
    Correction,
    /// Simple duplication, l'original ne change pas.
    Copie,
    /// Avoir rattaché à sa facture.
    Avoir,
    /// Bordereau de livraison créé depuis sa facture (jalon J6).
    Livraison,
}

impl NatureLien {
    pub fn code(self) -> &'static str {
        match self {
            NatureLien::Conversion => "conversion",
            NatureLien::Correction => "correction",
            NatureLien::Copie => "copie",
            NatureLien::Avoir => "avoir",
            NatureLien::Livraison => "livraison",
        }
    }

    /// Valeur de la colonne `nature_source`. Le bordereau n'y a pas de code :
    /// la contrainte de la migration 0004, déjà appliquée, ne connaît pas
    /// « livraison ». Il est reconnu au type des deux documents (voir
    /// `nature_lue`).
    fn code_en_base(self) -> Option<&'static str> {
        match self {
            NatureLien::Livraison => None,
            n => Some(n.code()),
        }
    }

    pub fn depuis_code(code: &str) -> Option<Self> {
        [
            NatureLien::Conversion,
            NatureLien::Correction,
            NatureLien::Copie,
            NatureLien::Avoir,
            NatureLien::Livraison,
        ]
        .into_iter()
        .find(|n| n.code() == code)
    }
}

/// Nature d'un lien lue en base : sans code, c'est un bordereau rattaché à sa
/// facture (seul lien écrit sans code depuis J5).
fn nature_lue(code: Option<&str>) -> NatureLien {
    code.and_then(NatureLien::depuis_code)
        .unwrap_or(NatureLien::Livraison)
}

/// Document lié, pour la navigation depuis la fiche.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lien {
    pub id: i64,
    pub type_document: TypeDocument,
    pub numero: Option<String>,
    pub statut: Statut,
    /// Nature du lien entre le document dérivé et sa source.
    pub nature: NatureLien,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub id: i64,
    pub type_document: TypeDocument,
    pub numero: Option<String>,
    pub statut: Statut,
    pub saisie: Saisie,
    /// Total de chaque ligne, dans l'ordre de `saisie.lignes`.
    pub totaux_lignes: Vec<Montant>,
    pub tiers: SnapshotTiers,
    pub totaux: Totaux,
    pub date_creation: String,
    pub date_validation: Option<String>,
    pub date_envoi: Option<String>,
    /// Document dont celui-ci est issu (devis converti, original copié, facture
    /// d'un avoir).
    pub source: Option<Lien>,
    /// Documents issus de celui-ci, du plus ancien au plus récent.
    pub derives: Vec<Lien>,
    /// Montant dû d'un document qui reçoit des règlements, une fois validé et
    /// tant qu'il n'est pas annulé : TTC moins les avoirs validés (section 7.1).
    pub montant_du: Option<Montant>,
    /// Facture Validée ou Envoyée : TTC moins les avoirs déjà validés, plafond
    /// d'un nouvel avoir.
    pub reste_a_crediter: Option<Montant>,
    /// Reste à livrer de chaque ligne, dans l'ordre de `saisie.lignes` (section
    /// 6.5) : sur une facture en vigueur, quantité facturée moins les bordereaux
    /// validés ; sur un bordereau issu d'une facture, la même chose sans compter
    /// ce bordereau. Vide pour les autres documents.
    pub restes_a_livrer: Vec<Quantite>,
}

/// Ligne de registre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumeDocument {
    pub id: i64,
    pub numero: Option<String>,
    pub date: NaiveDate,
    pub tiers_nom: String,
    pub libelle: String,
    pub montant_ttc: Montant,
    pub statut: Statut,
    pub mode_paiement: String,
    /// Seulement pour un document qui reçoit des règlements, Validé ou Envoyé.
    pub etat_paiement: Option<EtatPaiement>,
    pub date_echeance: Option<NaiveDate>,
}

/// Critères du registre d'un type de document.
#[derive(Debug, Clone, Default)]
pub struct FiltreDocuments {
    /// Cherché dans le numéro, le nom du tiers et le libellé, sans tenir compte
    /// de la casse ni des accents.
    pub texte: String,
    pub statut: Option<Statut>,
    pub du: Option<NaiveDate>,
    pub au: Option<NaiveDate>,
    pub etat_paiement: Option<EtatPaiement>,
}

impl FiltreDocuments {
    fn retient(&self, d: &ResumeDocument) -> bool {
        self.statut.is_none_or(|s| s == d.statut)
            && self.du.is_none_or(|du| d.date >= du)
            && self.au.is_none_or(|au| d.date <= au)
            && self
                .etat_paiement
                .is_none_or(|e| d.etat_paiement == Some(e))
            && [d.numero.as_deref().unwrap_or(""), &d.tiers_nom, &d.libelle]
                .iter()
                .any(|champ| texte::contient(champ, &self.texte))
    }
}

/// Changement de statut ou autre action notable, pour l'historique de la fiche.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evenement {
    pub date_action: String,
    pub utilisateur: String,
    /// Libellé composé, en français : `modele` où chaque `{}` reçoit la valeur suivante.
    pub libelle: String,
    /// Libellé fixe, à traduire, avec un `{}` par valeur (ex. « Validation, numéro {} »).
    pub modele: String,
    /// Valeurs à placer dans `modele`, dans l'ordre : numéro de document (tel quel) ou
    /// libellé de statut (à traduire, pour « {} vers {} »).
    pub valeurs: Vec<String>,
}

fn invalide(message: impl Into<String>) -> Erreur {
    Erreur::Validation(message.into())
}

/// Transition de statut refusée par le domaine, en message composé : type de
/// document et statuts sont des libellés à traduire.
fn transition_refusee(e: TransitionInterdite) -> Erreur {
    refus(
        "{} : passage de « {} » à « {} » non autorisé",
        &[&e.type_document, &e.depuis, &e.vers],
    )
}

pub(crate) fn date_texte(d: NaiveDate) -> String {
    d.format(FORMAT_DATE).to_string()
}

fn conversion(colonne: usize, message: String) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(colonne, rusqlite::types::Type::Text, message.into())
}

pub(crate) fn lire_date(r: &rusqlite::Row<'_>, colonne: usize) -> rusqlite::Result<NaiveDate> {
    let texte: String = r.get(colonne)?;
    NaiveDate::parse_from_str(&texte, FORMAT_DATE)
        .map_err(|_| conversion(colonne, format!("date illisible : {texte}")))
}

pub(crate) fn lire_date_facultative(
    r: &rusqlite::Row<'_>,
    colonne: usize,
) -> rusqlite::Result<Option<NaiveDate>> {
    match r.get::<_, Option<String>>(colonne)? {
        None => Ok(None),
        Some(_) => lire_date(r, colonne).map(Some),
    }
}

fn lire_statut(r: &rusqlite::Row<'_>, colonne: usize) -> rusqlite::Result<Statut> {
    let code: String = r.get(colonne)?;
    Statut::depuis_code(&code)
        .ok_or_else(|| conversion(colonne, format!("statut inconnu : {code}")))
}

pub(crate) fn lire_type(r: &rusqlite::Row<'_>, colonne: usize) -> rusqlite::Result<TypeDocument> {
    let code: String = r.get(colonne)?;
    TypeDocument::depuis_code(&code)
        .ok_or_else(|| conversion(colonne, format!("type de document inconnu : {code}")))
}

/// Nettoie et contrôle la saisie d'un brouillon : textes sans espaces superflus,
/// lignes complètes, taux possibles, échéance postérieure à la date.
fn nettoyer(type_document: TypeDocument, saisie: &Saisie) -> Resultat<Saisie> {
    let mut s = saisie.clone();
    s.libelle = s.libelle.trim().to_string();
    s.mode_paiement = s.mode_paiement.trim().to_string();
    s.observations = s.observations.trim().to_string();
    if type_document == TypeDocument::Livraison {
        let l = &mut s.livraison;
        for champ in [
            &mut l.visa_client,
            &mut l.visa_fournisseur,
            &mut l.date_livraison,
            &mut l.date_reception,
        ] {
            *champ = champ.trim().to_string();
        }
    } else {
        s.livraison = ChampsLivraison::default();
    }
    if !type_document.retenue_applicable() {
        s.taux.retenue = 0;
    }
    for (message, taux) in [
        (
            "le taux de remise doit être compris entre 0 et 100 %",
            s.taux.remise,
        ),
        (
            "le taux de TVA doit être compris entre 0 et 100 %",
            s.taux.tva,
        ),
        (
            "le taux de retenue doit être compris entre 0 et 100 %",
            s.taux.retenue,
        ),
    ] {
        if !(0..=CENT_POUR_CENT).contains(&taux) {
            return Err(invalide(message));
        }
    }
    if let Some(echeance) = s.date_echeance
        && echeance < s.date
    {
        return Err(invalide(
            "la date d'échéance ne peut pas précéder la date du document",
        ));
    }
    for (rang, ligne) in s.lignes.iter_mut().enumerate() {
        let n = rang + 1;
        ligne.reference = ligne.reference.trim().to_string();
        ligne.designation = ligne.designation.trim().to_string();
        ligne.observations = ligne.observations.trim().to_string();
        if ligne.designation.is_empty() {
            return Err(refus("ligne {} : la désignation est obligatoire", &[&n]));
        }
        if ligne.prix < 0 {
            return Err(refus("ligne {} : le prix ne peut pas être négatif", &[&n]));
        }
        if ligne.quantite <= 0 {
            return Err(refus(
                "ligne {} : la quantité doit être supérieure à zéro",
                &[&n],
            ));
        }
        if type_document != TypeDocument::Livraison {
            // La réception d'un bon de commande ne passe pas par le brouillon.
            ligne.quantite_livree = 0;
        } else if !(0..=ligne.quantite).contains(&ligne.quantite_livree) {
            return Err(refus(
                "ligne {} : la quantité livrée doit être comprise entre 0 et la quantité commandée",
                &[&n],
            ));
        }
    }
    Ok(s)
}

fn calculer(type_document: TypeDocument, s: &Saisie) -> Totaux {
    calculer_totaux(
        s.lignes.iter().map(|l| (l.prix, l.quantite)),
        s.taux,
        type_document.retenue_applicable(),
    )
}

/// Le tiers convient-il au type de document ? Fournisseur pour le bon de
/// commande, client pour tous les autres, bordereau compris (décision du
/// 2026-11-10 : la réception d'un fournisseur passe par le bon de commande).
fn tiers_compatible(type_document: TypeDocument, t: &crate::tiers::Tiers) -> bool {
    match type_document {
        TypeDocument::BonCommande => t.fiche.type_tiers.est_fournisseur(),
        _ => t.fiche.type_tiers.est_client(),
    }
}

/// Snapshot du tiers choisi. Un tiers désactivé n'est accepté que s'il était déjà
/// celui du document (il a pu être désactivé depuis).
fn snapshot_tiers(
    conn: &Connection,
    type_document: TypeDocument,
    tiers_id: Option<i64>,
    tiers_actuel: Option<i64>,
) -> Resultat<SnapshotTiers> {
    let Some(id) = tiers_id else {
        return Ok(SnapshotTiers::default());
    };
    let t = crate::tiers::lire(conn, id)?.ok_or_else(|| invalide("ce tiers n'existe plus"))?;
    if !t.actif && tiers_actuel != Some(id) {
        return Err(refus("le tiers « {} » est désactivé", &[&t.fiche.nom]));
    }
    if !tiers_compatible(type_document, &t) {
        let modele = if type_document == TypeDocument::BonCommande {
            "le tiers « {} » n'est pas enregistré comme fournisseur"
        } else {
            "le tiers « {} » n'est pas enregistré comme client"
        };
        return Err(refus(modele, &[&t.fiche.nom]));
    }
    Ok(SnapshotTiers {
        nom: t.fiche.nom,
        adresse: t.fiche.adresse,
        ifu: t.fiche.ifu,
        rccm: t.fiche.rccm,
        regime: t.fiche.regime_imposition,
    })
}

/// Écrit l'en-tête calculé et remplace les lignes d'un brouillon.
fn ecrire_contenu(
    conn: &Connection,
    id: i64,
    type_document: TypeDocument,
    s: &Saisie,
    tiers: &SnapshotTiers,
) -> Resultat<()> {
    let t = calculer(type_document, s);
    conn.execute(
        "UPDATE document SET
             date = ?1, tiers_id = ?2, tiers_nom_snapshot = ?3, tiers_adresse_snapshot = ?4,
             tiers_ifu_snapshot = ?5, tiers_rccm_snapshot = ?6, tiers_regime_snapshot = ?7,
             libelle = ?8, montant_ht = ?9, montant_remise = ?10, montant_base_tva = ?11,
             montant_tva = ?12, montant_retenue = ?13, montant_ttc = ?14,
             pourcentage_remise = ?15, pourcentage_tva = ?16, pourcentage_retenue = ?17,
             mode_paiement = ?18, observations = ?19, date_echeance = ?20,
             visa_client = ?21, visa_fournisseur = ?22, date_livraison_texte = ?23,
             date_reception_texte = ?24
         WHERE id = ?25",
        params![
            date_texte(s.date),
            s.tiers_id,
            tiers.nom,
            tiers.adresse,
            tiers.ifu,
            tiers.rccm,
            tiers.regime,
            s.libelle,
            t.ht,
            t.remise,
            t.base_tva,
            t.tva,
            t.retenue,
            t.ttc,
            s.taux.remise,
            s.taux.tva,
            s.taux.retenue,
            s.mode_paiement,
            s.observations,
            s.date_echeance.map(date_texte),
            s.livraison.visa_client,
            s.livraison.visa_fournisseur,
            s.livraison.date_livraison,
            s.livraison.date_reception,
            id
        ],
    )?;
    conn.execute("DELETE FROM ligne_document WHERE document_id = ?1", [id])?;
    let mut insertion = conn.prepare(
        "INSERT INTO ligne_document (document_id, produit_id, reference_snapshot,
             designation_snapshot, prix_snapshot, quantite, quantite_livree, total,
             observations, ordre)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
    )?;
    for (ordre, l) in s.lignes.iter().enumerate() {
        insertion.execute(params![
            id,
            l.produit_id,
            l.reference,
            l.designation,
            l.prix,
            l.quantite,
            l.quantite_livree,
            total_ligne(l.prix, l.quantite),
            l.observations,
            ordre as i64
        ])?;
    }
    Ok(())
}

const COLONNES: &str = "id, type, numero, statut, date, tiers_id, libelle,
    pourcentage_remise, pourcentage_tva, pourcentage_retenue, mode_paiement, observations,
    date_echeance, tiers_nom_snapshot, tiers_adresse_snapshot, tiers_ifu_snapshot,
    tiers_rccm_snapshot, tiers_regime_snapshot, montant_ht, montant_remise, montant_base_tva,
    montant_tva, montant_retenue, montant_ttc, date_creation, date_validation, date_envoi,
    visa_client, visa_fournisseur, date_livraison_texte, date_reception_texte";

fn ligne_vers_document(r: &rusqlite::Row<'_>) -> rusqlite::Result<Document> {
    Ok(Document {
        id: r.get(0)?,
        type_document: lire_type(r, 1)?,
        numero: r.get(2)?,
        statut: lire_statut(r, 3)?,
        saisie: Saisie {
            date: lire_date(r, 4)?,
            tiers_id: r.get(5)?,
            libelle: r.get(6)?,
            taux: Taux {
                remise: r.get(7)?,
                tva: r.get(8)?,
                retenue: r.get(9)?,
            },
            mode_paiement: r.get(10)?,
            observations: r.get(11)?,
            date_echeance: lire_date_facultative(r, 12)?,
            lignes: Vec::new(),
            livraison: ChampsLivraison {
                visa_client: r.get(27)?,
                visa_fournisseur: r.get(28)?,
                date_livraison: r.get(29)?,
                date_reception: r.get(30)?,
            },
        },
        totaux_lignes: Vec::new(),
        tiers: SnapshotTiers {
            nom: r.get(13)?,
            adresse: r.get(14)?,
            ifu: r.get(15)?,
            rccm: r.get(16)?,
            regime: r.get(17)?,
        },
        totaux: Totaux {
            ht: r.get(18)?,
            remise: r.get(19)?,
            base_tva: r.get(20)?,
            tva: r.get(21)?,
            retenue: r.get(22)?,
            ttc: r.get(23)?,
        },
        date_creation: r.get(24)?,
        date_validation: r.get(25)?,
        date_envoi: r.get(26)?,
        source: None,
        derives: Vec::new(),
        montant_du: None,
        reste_a_crediter: None,
        restes_a_livrer: Vec::new(),
    })
}

fn ligne_vers_lien(r: &rusqlite::Row<'_>) -> rusqlite::Result<Lien> {
    let nature: Option<String> = r.get(4)?;
    Ok(Lien {
        id: r.get(0)?,
        type_document: lire_type(r, 1)?,
        numero: r.get(2)?,
        statut: lire_statut(r, 3)?,
        nature: nature_lue(nature.as_deref()),
    })
}

/// Total TTC des avoirs validés (ou envoyés) d'une facture.
pub(crate) fn avoirs_valides(conn: &Connection, facture_id: i64) -> Resultat<Montant> {
    Ok(conn.query_row(
        "SELECT COALESCE(SUM(montant_ttc), 0) FROM document
         WHERE document_source_id = ?1 AND type = 'avoir' AND statut IN ('valide', 'envoye')",
        [facture_id],
        |r| r.get(0),
    )?)
}

/// Facture brouillon issue de la conversion d'un devis ou d'une proforma.
fn facture_en_preparation(conn: &Connection, source_id: i64) -> Resultat<Option<i64>> {
    Ok(conn
        .query_row(
            "SELECT id FROM document
             WHERE document_source_id = ?1 AND type = 'facture'
               AND nature_source = 'conversion' AND statut = 'brouillon'
             ORDER BY id LIMIT 1",
            [source_id],
            |r| r.get(0),
        )
        .optional()?)
}

/// Source, documents dérivés et montants liés aux avoirs.
fn completer_liens(conn: &Connection, d: &mut Document) -> Resultat<()> {
    d.source = conn
        .query_row(
            "SELECT s.id, s.type, s.numero, s.statut, d.nature_source
             FROM document d JOIN document s ON s.id = d.document_source_id
             WHERE d.id = ?1",
            [d.id],
            ligne_vers_lien,
        )
        .optional()?;
    let mut requete = conn.prepare(
        "SELECT id, type, numero, statut, nature_source FROM document
         WHERE document_source_id = ?1 ORDER BY id",
    )?;
    d.derives = requete
        .query_map([d.id], ligne_vers_lien)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if d.statut.en_vigueur() {
        let avoirs = if d.type_document == TypeDocument::Facture {
            avoirs_valides(conn, d.id)?
        } else {
            0
        };
        let reste = d.totaux.ttc - avoirs;
        if d.type_document.recoit_des_reglements() {
            d.montant_du = Some(reste);
        }
        if d.type_document == TypeDocument::Facture {
            d.reste_a_crediter = Some(reste);
            d.restes_a_livrer = restes_a_livrer(conn, d.id, None)?;
        }
    }
    if d.type_document == TypeDocument::Livraison
        && let Some(l) = &d.source
        && l.nature == NatureLien::Livraison
    {
        d.restes_a_livrer = restes_a_livrer(conn, l.id, Some(d.id))?;
    }
    Ok(())
}

pub fn lire(conn: &Connection, id: i64) -> Resultat<Option<Document>> {
    let Some(mut d) = conn
        .query_row(
            &format!("SELECT {COLONNES} FROM document WHERE id = ?1"),
            [id],
            ligne_vers_document,
        )
        .optional()?
    else {
        return Ok(None);
    };
    let mut requete = conn.prepare(
        "SELECT produit_id, reference_snapshot, designation_snapshot, prix_snapshot, quantite,
                observations, total, quantite_livree
         FROM ligne_document WHERE document_id = ?1 ORDER BY ordre, id",
    )?;
    let lignes = requete
        .query_map([id], |r| {
            Ok((
                LigneSaisie {
                    produit_id: r.get(0)?,
                    reference: r.get(1)?,
                    designation: r.get(2)?,
                    prix: r.get(3)?,
                    quantite: r.get(4)?,
                    quantite_livree: r.get(7)?,
                    observations: r.get(5)?,
                },
                r.get::<_, Montant>(6)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    (d.saisie.lignes, d.totaux_lignes) = lignes.into_iter().unzip();
    completer_liens(conn, &mut d)?;
    Ok(Some(d))
}

pub(crate) fn lire_existant(conn: &Connection, id: i64) -> Resultat<Document> {
    lire(conn, id)?.ok_or_else(|| invalide("ce document n'existe plus"))
}

fn exiger_brouillon(d: &Document) -> Resultat<()> {
    if d.statut == Statut::Brouillon {
        Ok(())
    } else {
        Err(refus(
            "ce document est « {} » : ses lignes et montants ne se modifient plus",
            &[&d.statut],
        ))
    }
}

/// Registre d'un type de document, du plus récent au plus ancien.
pub fn lister(
    conn: &Connection,
    type_document: TypeDocument,
    filtre: &FiltreDocuments,
) -> Resultat<Vec<ResumeDocument>> {
    let mut requete = conn.prepare(
        "SELECT d.id, d.numero, d.date, d.tiers_nom_snapshot, d.libelle, d.montant_ttc,
                d.statut, d.mode_paiement, d.date_echeance,
                COALESCE((SELECT SUM(r.montant) FROM reglement r WHERE r.document_id = d.id), 0),
                COALESCE((SELECT SUM(a.montant_ttc) FROM document a
                          WHERE a.document_source_id = d.id AND a.type = 'avoir'
                            AND a.statut IN ('valide', 'envoye')), 0)
         FROM document d
         WHERE d.type = ?1
         ORDER BY d.date DESC, d.numero IS NULL DESC, d.numero DESC, d.id DESC",
    )?;
    let recoit = type_document.recoit_des_reglements();
    let tous = requete
        .query_map([type_document.code()], |r| {
            let statut = lire_statut(r, 6)?;
            let ttc: Montant = r.get(5)?;
            let regle: Montant = r.get(9)?;
            let avoirs: Montant = r.get(10)?;
            let etat = (recoit && statut.en_vigueur()).then(|| etat_paiement(ttc - avoirs, regle));
            Ok(ResumeDocument {
                id: r.get(0)?,
                numero: r.get(1)?,
                date: lire_date(r, 2)?,
                tiers_nom: r.get(3)?,
                libelle: r.get(4)?,
                montant_ttc: ttc,
                statut,
                mode_paiement: r.get(7)?,
                etat_paiement: etat,
                date_echeance: lire_date_facultative(r, 8)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(tous.into_iter().filter(|d| filtre.retient(d)).collect())
}

fn journaliser_statut(
    conn: &Connection,
    id: i64,
    action: &str,
    ancien: Option<Value>,
    nouveau: Value,
    acteur: &Utilisateur,
) -> Resultat<()> {
    audit::journaliser(
        conn,
        Entree {
            entite: "document",
            entite_id: id,
            action,
            ancienne_valeur: ancien,
            nouvelle_valeur: Some(nouveau),
            utilisateur_id: Some(acteur.id),
        },
    )
}

/// Insère un brouillon, éventuellement lié à sa source. Le droit est contrôlé
/// par l'appelant. `tiers_actuel` accepte le tiers de la source même s'il a été
/// désactivé depuis.
fn inserer_brouillon(
    tx: &Connection,
    acteur: &Utilisateur,
    type_document: TypeDocument,
    saisie: &Saisie,
    source: Option<(i64, NatureLien)>,
    tiers_actuel: Option<i64>,
) -> Resultat<i64> {
    let s = nettoyer(type_document, saisie)?;
    let tiers = snapshot_tiers(tx, type_document, s.tiers_id, tiers_actuel)?;
    tx.execute(
        "INSERT INTO document (type, date, date_creation, utilisateur_creation_id,
                               document_source_id, nature_source)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            type_document.code(),
            date_texte(s.date),
            maintenant(),
            acteur.id,
            source.map(|(id, _)| id),
            source.and_then(|(_, n)| n.code_en_base()),
        ],
    )?;
    let id = tx.last_insert_rowid();
    ecrire_contenu(tx, id, type_document, &s, &tiers)?;
    let mut valeur = json!({ "type": type_document.code(), "statut": Statut::Brouillon.code() });
    if let Some((source_id, nature)) = source {
        valeur["source_id"] = json!(source_id);
        valeur["nature"] = json!(nature.code());
    }
    journaliser_statut(tx, id, "creation", None, valeur, acteur)?;
    Ok(id)
}

/// Crée un brouillon et renvoie son identifiant. Un avoir ne se crée pas ici :
/// voir `creer_avoir`.
pub fn creer_brouillon(
    conn: &mut Connection,
    acteur: &Utilisateur,
    type_document: TypeDocument,
    saisie: &Saisie,
) -> Resultat<i64> {
    let tx = conn.transaction()?;
    exiger(&tx, acteur, Droit::brouillon(type_document))?;
    if type_document == TypeDocument::Avoir {
        return Err(invalide(
            "un avoir se crée depuis la fiche d'une facture validée",
        ));
    }
    let id = inserer_brouillon(&tx, acteur, type_document, saisie, None, None)?;
    tx.commit()?;
    Ok(id)
}

/// Date du jour, pour les documents créés à partir d'un autre.
pub(crate) fn aujourd_hui() -> NaiveDate {
    chrono::Local::now().date_naive()
}

/// Échéance proposée pour un nouveau document : délai de paiement pour une
/// facture, validité des devis pour un devis ou une proforma, rien sinon.
fn echeance_du_type(
    conn: &Connection,
    type_document: TypeDocument,
    date: NaiveDate,
) -> Resultat<Option<NaiveDate>> {
    let p = crate::parametres::lire(conn)?;
    let delai = match type_document {
        TypeDocument::Facture => p.delai_paiement_jours,
        TypeDocument::Devis | TypeDocument::Proforma => p.validite_devis_jours,
        _ => None,
    };
    Ok(echeance_par_defaut(date, delai))
}

/// Saisie d'un nouveau document reprise d'un autre : lignes copiées telles
/// qu'enregistrées (copie figée, section 7.7), date du jour, échéance du type.
fn saisie_reprise(conn: &Connection, d: &Document, type_cible: TypeDocument) -> Resultat<Saisie> {
    let date = aujourd_hui();
    let mut s = d.saisie.clone();
    s.date = date;
    s.date_echeance = echeance_du_type(conn, type_cible, date)?;
    Ok(s)
}

/// Convertit un devis ou une proforma Validé ou Envoyé en facture brouillon et
/// renvoie l'identifiant de la facture. La source passe en Converti à la
/// validation de la facture (décision du 2026-11-09) ; tant que la facture est
/// en brouillon, une seconde conversion est refusée.
pub fn convertir_en_facture(
    conn: &mut Connection,
    acteur: &Utilisateur,
    source_id: i64,
) -> Resultat<i64> {
    let tx = conn.transaction()?;
    exiger(&tx, acteur, Droit::brouillon(TypeDocument::Facture))?;
    let src = lire_existant(&tx, source_id)?;
    if !src.type_document.convertible_en_facture() {
        return Err(invalide(
            "seuls un devis ou une proforma se convertissent en facture",
        ));
    }
    if !matches!(src.statut, Statut::Valide | Statut::Envoye) {
        return Err(refus(
            "seul un document validé ou envoyé se convertit en facture ; celui-ci est « {} »",
            &[&src.statut],
        ));
    }
    if facture_en_preparation(&tx, source_id)?.is_some() {
        return Err(invalide(
            "une facture est déjà en préparation pour ce document",
        ));
    }
    let s = saisie_reprise(&tx, &src, TypeDocument::Facture)?;
    let id = inserer_brouillon(
        &tx,
        acteur,
        TypeDocument::Facture,
        &s,
        Some((source_id, NatureLien::Conversion)),
        src.saisie.tiers_id,
    )?;
    tx.commit()?;
    Ok(id)
}

/// Duplique un document en nouveau brouillon du même type et renvoie son
/// identifiant. Avec `annuler_original`, correction d'un devis ou d'une
/// proforma Validé ou Envoyé : l'original passe en Annulé dans la même
/// transaction (section 7.2). Un avoir ne se duplique pas.
pub fn dupliquer(
    conn: &mut Connection,
    acteur: &Utilisateur,
    id: i64,
    annuler_original: bool,
) -> Resultat<i64> {
    let tx = conn.transaction()?;
    let d = lire_existant(&tx, id)?;
    exiger(&tx, acteur, Droit::brouillon(d.type_document))?;
    if d.type_document == TypeDocument::Avoir {
        return Err(invalide(
            "un avoir ne se duplique pas : il se crée depuis la fiche de sa facture",
        ));
    }
    if d.source.as_ref().map(|l| l.nature) == Some(NatureLien::Livraison) {
        // La copie ne serait plus rattachée à la facture et échapperait au
        // contrôle du reste à livrer.
        return Err(invalide(
            "un bordereau issu d'une facture ne se duplique pas : en créer un autre depuis la facture",
        ));
    }
    if annuler_original {
        if !d.type_document.convertible_en_facture() {
            return Err(invalide(
                "la correction avec annulation de l'original vaut pour un devis ou une proforma ; une facture se corrige par un avoir",
            ));
        }
        if !matches!(d.statut, Statut::Valide | Statut::Envoye) {
            return Err(refus(
                "seul un document validé ou envoyé se corrige ; celui-ci est « {} »",
                &[&d.statut],
            ));
        }
        exiger(&tx, acteur, Droit::AnnulerDocumentValide)?;
        if facture_en_preparation(&tx, id)?.is_some() {
            return Err(invalide(
                "une facture est en préparation pour ce document : supprimer d'abord ce brouillon",
            ));
        }
        verifier_transition(d.type_document, d.statut, Statut::Annule)
            .map_err(transition_refusee)?;
    }
    let nature = if annuler_original {
        NatureLien::Correction
    } else {
        NatureLien::Copie
    };
    let s = saisie_reprise(&tx, &d, d.type_document)?;
    let nouveau = inserer_brouillon(
        &tx,
        acteur,
        d.type_document,
        &s,
        Some((id, nature)),
        d.saisie.tiers_id,
    )?;
    if annuler_original {
        tx.execute(
            "UPDATE document SET statut = ?1 WHERE id = ?2",
            params![Statut::Annule.code(), id],
        )?;
        journaliser_statut(
            &tx,
            id,
            "annulation",
            Some(json!({ "statut": d.statut.code() })),
            json!({ "statut": Statut::Annule.code(), "numero": d.numero, "corrige_par": nouveau }),
            acteur,
        )?;
    }
    tx.commit()?;
    Ok(nouveau)
}

/// Crée un avoir brouillon depuis une facture Validée ou Envoyée et renvoie son
/// identifiant : lignes, client et taux repris de la facture (décision du
/// 2026-11-09) ; un avoir partiel retire des lignes ou baisse quantités et prix.
pub fn creer_avoir(conn: &mut Connection, acteur: &Utilisateur, facture_id: i64) -> Resultat<i64> {
    let tx = conn.transaction()?;
    exiger(&tx, acteur, Droit::brouillon(TypeDocument::Avoir))?;
    let f = lire_existant(&tx, facture_id)?;
    if f.type_document != TypeDocument::Facture {
        return Err(invalide("un avoir se crée depuis une facture"));
    }
    if !matches!(f.statut, Statut::Valide | Statut::Envoye) {
        return Err(refus(
            "un avoir se crée depuis une facture validée ou envoyée ; celle-ci est « {} »",
            &[&f.statut],
        ));
    }
    if f.reste_a_crediter.unwrap_or(0) <= 0 {
        return Err(invalide(
            "cette facture est entièrement couverte par ses avoirs",
        ));
    }
    let s = saisie_reprise(&tx, &f, TypeDocument::Avoir)?;
    let id = inserer_brouillon(
        &tx,
        acteur,
        TypeDocument::Avoir,
        &s,
        Some((facture_id, NatureLien::Avoir)),
        f.saisie.tiers_id,
    )?;
    tx.commit()?;
    Ok(id)
}

/// Facture d'origine d'un avoir.
fn facture_de_l_avoir(conn: &Connection, avoir: &Document) -> Resultat<Document> {
    let source = avoir
        .source
        .as_ref()
        .ok_or_else(|| invalide("cet avoir n'est rattaché à aucune facture"))?;
    lire_existant(conn, source.id)
}

/// Un avoir garde le client et les taux de sa facture : un autre client n'aurait
/// pas de sens, d'autres taux rendraient son montant incohérent avec la facture.
fn controler_avoir(facture: &Document, s: &Saisie) -> Resultat<()> {
    if s.tiers_id != facture.saisie.tiers_id {
        return Err(invalide("le client d'un avoir est celui de sa facture"));
    }
    if s.taux != facture.saisie.taux {
        return Err(invalide(
            "la remise et la TVA d'un avoir sont celles de sa facture",
        ));
    }
    Ok(())
}

/// Statuts d'un bordereau dont les quantités comptent dans le reste à livrer de
/// sa facture : émis (Validé) ou confirmé.
const BORDEREAU_EMIS: &str = "'valide', 'livraison_partielle', 'livraison_complete'";
/// Statuts d'un bordereau dont la livraison est confirmée.
const BORDEREAU_CONFIRME: &str = "'livraison_partielle', 'livraison_complete'";

/// Quantité livrée par rang de ligne, cumulée sur les bordereaux d'une facture
/// aux statuts donnés, sans compter le bordereau `sauf`.
fn livrees_par_rang(
    conn: &Connection,
    facture_id: i64,
    statuts: &str,
    sauf: Option<i64>,
) -> Resultat<Vec<(usize, Quantite)>> {
    let mut requete = conn.prepare(&format!(
        "SELECT l.ordre, SUM(l.quantite_livree)
         FROM ligne_document l JOIN document b ON b.id = l.document_id
         WHERE b.document_source_id = ?1 AND b.type = 'livraison'
           AND b.statut IN ({statuts}) AND b.id <> ?2
         GROUP BY l.ordre"
    ))?;
    let lignes = requete
        .query_map(params![facture_id, sauf.unwrap_or(-1)], |r| {
            Ok((r.get::<_, i64>(0)? as usize, r.get(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(lignes)
}

/// Nombre de bordereaux d'une facture en brouillon ou validés, pas encore
/// confirmés ni annulés.
fn bordereaux_en_cours(conn: &Connection, facture_id: i64) -> Resultat<i64> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM document
         WHERE document_source_id = ?1 AND type = 'livraison'
           AND statut IN ('brouillon', 'valide')",
        [facture_id],
        |r| r.get(0),
    )?)
}

/// Quantités facturées, dans l'ordre des lignes de la facture (figées depuis sa
/// validation : le rang d'une ligne ne change plus).
fn quantites_facturees(conn: &Connection, facture_id: i64) -> Resultat<Vec<Quantite>> {
    let mut requete = conn
        .prepare("SELECT quantite FROM ligne_document WHERE document_id = ?1 ORDER BY ordre, id")?;
    let quantites = requete
        .query_map([facture_id], |r| r.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(quantites)
}

/// Reste à livrer de chaque ligne d'une facture : quantité facturée moins les
/// bordereaux émis, hors `sauf` (section 6.5). Les avoirs n'y changent rien
/// (décision du 2026-11-10).
fn restes_a_livrer(
    conn: &Connection,
    facture_id: i64,
    sauf: Option<i64>,
) -> Resultat<Vec<Quantite>> {
    let mut restes = quantites_facturees(conn, facture_id)?;
    for (rang, livree) in livrees_par_rang(conn, facture_id, BORDEREAU_EMIS, sauf)? {
        if let Some(reste) = restes.get_mut(rang) {
            *reste = (*reste - livree).max(0);
        }
    }
    Ok(restes)
}

/// Facture d'origine d'un bordereau créé depuis une facture ; `None` pour un
/// bordereau vierge.
fn facture_du_bordereau(conn: &Connection, bordereau: &Document) -> Resultat<Option<Document>> {
    match &bordereau.source {
        Some(l) if l.nature == NatureLien::Livraison => Ok(Some(lire_existant(conn, l.id)?)),
        _ => Ok(None),
    }
}

/// Un bordereau issu d'une facture reprend ses lignes et son client : seules la
/// quantité livrée et les observations se modifient. Les prix, qui ne
/// s'affichent pas sur un bordereau, sont repris de la facture. Une quantité
/// livrée au-delà du reste à livrer est refusée (décision du 2026-11-10).
fn controler_bordereau(
    conn: &Connection,
    bordereau_id: i64,
    facture: &Document,
    s: &mut Saisie,
) -> Resultat<()> {
    if s.tiers_id != facture.saisie.tiers_id {
        return Err(invalide(
            "le client d'un bordereau issu d'une facture est celui de la facture",
        ));
    }
    if s.lignes.len() != facture.saisie.lignes.len() {
        return Err(invalide(
            "un bordereau issu d'une facture reprend toutes les lignes de la facture",
        ));
    }
    let restes = restes_a_livrer(conn, facture.id, Some(bordereau_id))?;
    for (rang, (ligne, origine)) in s.lignes.iter_mut().zip(&facture.saisie.lignes).enumerate() {
        let n = rang + 1;
        if (&ligne.reference, &ligne.designation, ligne.quantite)
            != (&origine.reference, &origine.designation, origine.quantite)
        {
            return Err(refus(
                "ligne {} : un bordereau issu d'une facture reprend la ligne de la facture ; seules la quantité livrée et les observations se modifient",
                &[&n],
            ));
        }
        ligne.produit_id = origine.produit_id;
        ligne.prix = origine.prix;
        if ligne.quantite_livree > restes.get(rang).copied().unwrap_or(0) {
            return Err(refus(
                "ligne {} : la quantité livrée dépasse le reste à livrer, la corriger",
                &[&n],
            ));
        }
    }
    Ok(())
}

/// Crée un bordereau brouillon depuis une facture Validée ou Envoyée et renvoie
/// son identifiant (section 6.5) : lignes et client de la facture, quantité
/// commandée égale à la quantité facturée, quantité livrée proposée égale au
/// reste à livrer. Refusé quand plus rien ne reste à livrer.
pub fn creer_bordereau(
    conn: &mut Connection,
    acteur: &Utilisateur,
    facture_id: i64,
) -> Resultat<i64> {
    let tx = conn.transaction()?;
    exiger(&tx, acteur, Droit::brouillon(TypeDocument::Livraison))?;
    let f = lire_existant(&tx, facture_id)?;
    if f.type_document != TypeDocument::Facture {
        return Err(invalide("un bordereau se crée depuis une facture"));
    }
    if !matches!(f.statut, Statut::Valide | Statut::Envoye) {
        return Err(refus(
            "un bordereau se crée depuis une facture validée ou envoyée ; celle-ci est « {} »",
            &[&f.statut],
        ));
    }
    let restes = restes_a_livrer(&tx, facture_id, None)?;
    if restes.iter().all(|r| *r <= 0) {
        return Err(invalide(
            "tout est déjà livré ou en cours de livraison sur cette facture",
        ));
    }
    let date = aujourd_hui();
    let s = Saisie {
        tiers_id: f.saisie.tiers_id,
        libelle: f.saisie.libelle.clone(),
        lignes: f
            .saisie
            .lignes
            .iter()
            .zip(&restes)
            .map(|(l, reste)| LigneSaisie {
                quantite_livree: *reste,
                observations: String::new(),
                ..l.clone()
            })
            .collect(),
        ..Saisie::vide(date)
    };
    let id = inserer_brouillon(
        &tx,
        acteur,
        TypeDocument::Livraison,
        &s,
        Some((facture_id, NatureLien::Livraison)),
        f.saisie.tiers_id,
    )?;
    tx.commit()?;
    Ok(id)
}

/// Droit de confirmer la livraison d'un bordereau : « Valider un document »,
/// comme la réception d'un bon de commande (décision du 2026-11-10, section 10.1).
fn droit_confirmation_livraison() -> Droit {
    Droit::ValiderDocument
}

/// Confirme la livraison d'un bordereau Validé, après le visa du client
/// (décision du 2026-11-10). Bordereau issu d'une facture : Livraison complète
/// si les bordereaux déjà confirmés et celui-ci couvrent chaque ligne de la
/// facture, partielle sinon. Bordereau vierge : quantités livrées comparées à
/// ses quantités commandées. Le statut ne change plus ensuite.
pub fn confirmer_livraison(conn: &mut Connection, acteur: &Utilisateur, id: i64) -> Resultat<()> {
    let tx = conn.transaction()?;
    let d = lire_existant(&tx, id)?;
    exiger(&tx, acteur, droit_confirmation_livraison())?;
    if d.type_document != TypeDocument::Livraison {
        return Err(invalide("seul un bordereau de livraison se confirme"));
    }
    if d.statut != Statut::Valide {
        return Err(refus(
            "seul un bordereau validé se confirme ; celui-ci est « {} »",
            &[&d.statut],
        ));
    }
    let paires: Vec<(Quantite, Quantite)> = match facture_du_bordereau(&tx, &d)? {
        Some(f) => {
            let mut livrees: Vec<Quantite> =
                d.saisie.lignes.iter().map(|l| l.quantite_livree).collect();
            for (rang, livree) in livrees_par_rang(&tx, f.id, BORDEREAU_CONFIRME, Some(id))? {
                if let Some(cumul) = livrees.get_mut(rang) {
                    *cumul += livree;
                }
            }
            quantites_facturees(&tx, f.id)?
                .into_iter()
                .zip(livrees)
                .collect()
        }
        None => d
            .saisie
            .lignes
            .iter()
            .map(|l| (l.quantite, l.quantite_livree))
            .collect(),
    };
    let nouveau = match avancement(paires) {
        Avancement::Complet => Statut::LivraisonComplete,
        _ => Statut::LivraisonPartielle,
    };
    verifier_transition(d.type_document, d.statut, nouveau).map_err(transition_refusee)?;
    tx.execute(
        "UPDATE document SET statut = ?1 WHERE id = ?2",
        params![nouveau.code(), id],
    )?;
    journaliser_statut(
        &tx,
        id,
        "livraison",
        Some(json!({ "statut": d.statut.code() })),
        json!({ "statut": nouveau.code() }),
        acteur,
    )?;
    tx.commit()?;
    Ok(())
}

/// Remplace le contenu d'un brouillon. Pas d'audit ligne à ligne en brouillon
/// (section 7.4).
pub fn modifier_brouillon(
    conn: &mut Connection,
    acteur: &Utilisateur,
    id: i64,
    saisie: &Saisie,
) -> Resultat<()> {
    let tx = conn.transaction()?;
    let d = lire_existant(&tx, id)?;
    exiger(&tx, acteur, Droit::brouillon(d.type_document))?;
    exiger_brouillon(&d)?;
    let mut s = nettoyer(d.type_document, saisie)?;
    if d.type_document == TypeDocument::Avoir {
        controler_avoir(&facture_de_l_avoir(&tx, &d)?, &s)?;
    }
    if let Some(f) = facture_du_bordereau(&tx, &d)? {
        controler_bordereau(&tx, id, &f, &mut s)?;
    }
    let tiers = snapshot_tiers(&tx, d.type_document, s.tiers_id, d.saisie.tiers_id)?;
    ecrire_contenu(&tx, id, d.type_document, &s, &tiers)?;
    tx.commit()?;
    Ok(())
}

fn en_json(d: &Document) -> Value {
    json!({
        "type": d.type_document.code(),
        "statut": d.statut.code(),
        "date": date_texte(d.saisie.date),
        "tiers": d.tiers.nom,
        "libelle": d.saisie.libelle,
        "montant_ttc": d.totaux.ttc,
        "lignes": d.saisie.lignes.iter().map(|l| json!({
            "reference": l.reference,
            "designation": l.designation,
            "prix": l.prix,
            "quantite": l.quantite,
        })).collect::<Vec<_>>(),
    })
}

/// Suppression physique, réservée aux brouillons jamais validés (section 7.3).
/// Le contenu supprimé reste dans le journal d'audit.
pub fn supprimer_brouillon(conn: &mut Connection, acteur: &Utilisateur, id: i64) -> Resultat<()> {
    let tx = conn.transaction()?;
    let d = lire_existant(&tx, id)?;
    exiger(&tx, acteur, Droit::brouillon(d.type_document))?;
    if !d.statut.suppression_physique_autorisee() {
        return Err(refus(
            "seul un brouillon peut être supprimé ; ce document est « {} »",
            &[&d.statut],
        ));
    }
    tx.execute("DELETE FROM document WHERE id = ?1", [id])?;
    audit::journaliser(
        &tx,
        Entree {
            entite: "document",
            entite_id: id,
            action: "suppression",
            ancienne_valeur: Some(en_json(&d)),
            nouvelle_valeur: None,
            utilisateur_id: Some(acteur.id),
        },
    )?;
    tx.commit()?;
    Ok(())
}

/// Valide un brouillon : contrôles, recalcul des totaux, numéro, en une seule
/// transaction. Renvoie le numéro attribué.
pub fn valider(conn: &mut Connection, acteur: &Utilisateur, id: i64) -> Resultat<String> {
    let tx = conn.transaction()?;
    let d = lire_existant(&tx, id)?;
    exiger(&tx, acteur, Droit::ValiderDocument)?;
    verifier_transition(d.type_document, d.statut, Statut::Valide).map_err(transition_refusee)?;
    let mut s = nettoyer(d.type_document, &d.saisie)?;
    if s.tiers_id.is_none() {
        return Err(invalide("choisir le tiers avant de valider"));
    }
    if s.lignes.is_empty() {
        return Err(invalide("le document doit compter au moins une ligne"));
    }
    // Bordereau : quelque chose à livrer ; reste à livrer revérifié, un autre
    // bordereau de la même facture a pu être validé entre-temps.
    if d.type_document == TypeDocument::Livraison {
        if s.lignes.iter().all(|l| l.quantite_livree == 0) {
            return Err(invalide(
                "le bordereau ne livre rien : saisir au moins une quantité livrée",
            ));
        }
        if let Some(f) = facture_du_bordereau(&tx, &d)? {
            if !matches!(f.statut, Statut::Valide | Statut::Envoye) {
                return Err(refus(
                    "la facture d'origine est « {} » : le bordereau ne peut plus être validé",
                    &[&f.statut],
                ));
            }
            controler_bordereau(&tx, id, &f, &mut s)?;
        }
    }
    // Documents liés : la source d'une facture convertie doit encore pouvoir
    // passer en Converti ; un avoir reste dans le reste à créditer de sa facture.
    let source_convertie = match (&d.source, d.type_document) {
        (Some(l), TypeDocument::Facture) if l.nature == NatureLien::Conversion => {
            let src = lire_existant(&tx, l.id)?;
            verifier_transition(src.type_document, src.statut, Statut::Converti).map_err(|_| {
                refus(
                    "le document d'origine est « {} » : il ne peut plus être converti",
                    &[&src.statut],
                )
            })?;
            Some(src)
        }
        _ => None,
    };
    let facture_creditee = if d.type_document == TypeDocument::Avoir {
        let f = facture_de_l_avoir(&tx, &d)?;
        if !matches!(f.statut, Statut::Valide | Statut::Envoye) {
            return Err(refus(
                "la facture d'origine est « {} » : l'avoir ne peut plus être validé",
                &[&f.statut],
            ));
        }
        controler_avoir(&f, &s)?;
        let montant = calculer(TypeDocument::Avoir, &s).ttc;
        let reste = f.reste_a_crediter.unwrap_or(0);
        if montant <= 0 {
            return Err(invalide("le montant d'un avoir doit être positif"));
        }
        if montant > reste {
            return Err(refus(
                "le montant de l'avoir ({} FCFA) dépasse le reste à créditer de la facture ({} FCFA)",
                &[&montant_en_texte(montant), &montant_en_texte(reste)],
            ));
        }
        Some(f)
    } else {
        None
    };
    // Contrôle du tiers (existence, type) ; le snapshot enregistré au brouillon
    // est rafraîchi une dernière fois puis figé.
    let tiers = snapshot_tiers(&tx, d.type_document, s.tiers_id, d.saisie.tiers_id)?;
    ecrire_contenu(&tx, id, d.type_document, &s, &tiers)?;

    use chrono::Datelike;
    let annee = u16::try_from(s.date.year())
        .map_err(|_| invalide("année de la date du document hors limites"))?;
    let mois = s.date.month() as u8;
    // Réglage de la numérotation lu dans la transaction : préfixe, remise à zéro
    // mensuelle ou annuelle (compteur au mois 0), nombre de chiffres.
    let reglage = crate::parametres::lire(&tx)?.numerotation;
    let rang = crate::prochain_rang(&tx, d.type_document, annee, reglage.mois_du_compteur(mois))?;
    let numero = reglage.formater(d.type_document, annee, mois, rang);
    let modifiees = tx.execute(
        "UPDATE document SET statut = ?1, numero = ?2, date_validation = ?3,
                             utilisateur_validation_id = ?4
         WHERE id = ?5 AND statut = ?6",
        params![
            Statut::Valide.code(),
            numero,
            maintenant(),
            acteur.id,
            id,
            Statut::Brouillon.code()
        ],
    )?;
    if modifiees != 1 {
        return Err(invalide("ce document n'est plus un brouillon"));
    }
    // Entreprise, logos, textes et couleur figés avec le document (section 12).
    crate::impression::figer(&tx, id, d.type_document)?;
    let totaux = calculer(d.type_document, &s);
    journaliser_statut(
        &tx,
        id,
        "validation",
        Some(json!({ "statut": Statut::Brouillon.code() })),
        json!({ "statut": Statut::Valide.code(), "numero": numero, "montant_ttc": totaux.ttc }),
        acteur,
    )?;
    if let Some(src) = source_convertie {
        tx.execute(
            "UPDATE document SET statut = ?1 WHERE id = ?2",
            params![Statut::Converti.code(), src.id],
        )?;
        journaliser_statut(
            &tx,
            src.id,
            "conversion",
            Some(json!({ "statut": src.statut.code() })),
            json!({ "statut": Statut::Converti.code(), "facture_id": id, "facture": numero }),
            acteur,
        )?;
    }
    if let Some(f) = facture_creditee {
        journaliser_statut(
            &tx,
            f.id,
            "avoir",
            None,
            json!({ "avoir_id": id, "avoir": numero, "montant_ttc": totaux.ttc }),
            acteur,
        )?;
    }
    tx.commit()?;
    Ok(numero)
}

/// Passage en Envoyé, avec la date d'envoi.
pub fn marquer_envoye(conn: &mut Connection, acteur: &Utilisateur, id: i64) -> Resultat<()> {
    let tx = conn.transaction()?;
    let d = lire_existant(&tx, id)?;
    exiger(&tx, acteur, Droit::ValiderDocument)?;
    verifier_transition(d.type_document, d.statut, Statut::Envoye).map_err(transition_refusee)?;
    tx.execute(
        "UPDATE document SET statut = ?1, date_envoi = ?2 WHERE id = ?3",
        params![Statut::Envoye.code(), maintenant(), id],
    )?;
    journaliser_statut(
        &tx,
        id,
        "envoi",
        Some(json!({ "statut": d.statut.code() })),
        json!({ "statut": Statut::Envoye.code() }),
        acteur,
    )?;
    tx.commit()?;
    Ok(())
}

/// Annulation logique : le numéro éventuel est conservé (section 7.2).
pub fn annuler(conn: &mut Connection, acteur: &Utilisateur, id: i64) -> Resultat<()> {
    let tx = conn.transaction()?;
    let d = lire_existant(&tx, id)?;
    let droit = if d.statut == Statut::Brouillon {
        Droit::brouillon(d.type_document)
    } else {
        Droit::AnnulerDocumentValide
    };
    exiger(&tx, acteur, droit)?;
    verifier_transition(d.type_document, d.statut, Statut::Annule).map_err(transition_refusee)?;
    if d.type_document.convertible_en_facture() && facture_en_preparation(&tx, id)?.is_some() {
        return Err(invalide(
            "une facture est en préparation pour ce document : supprimer d'abord ce brouillon",
        ));
    }
    if d.type_document == TypeDocument::Facture && avoirs_valides(&tx, id)? > 0 {
        return Err(invalide(
            "cette facture a des avoirs validés : annuler d'abord ces avoirs",
        ));
    }
    // Bordereau en brouillon ou validé, pas encore confirmé : refus (décision du
    // 2026-11-10). Une fois les bordereaux confirmés ou annulés, la facture
    // s'annule ; un bordereau confirmé ne s'annule plus (section 7.1).
    if d.type_document == TypeDocument::Facture && bordereaux_en_cours(&tx, id)? > 0 {
        return Err(invalide(
            "cette facture a des bordereaux non confirmés : les annuler ou confirmer leur livraison d'abord",
        ));
    }
    // Règlements : refus tant qu'il en reste (décision du 2026-11-10, jalon J7).
    if d.type_document.recoit_des_reglements() && crate::reglements::nombre(&tx, id)? > 0 {
        return Err(invalide(
            "ce document a des règlements : les supprimer d'abord",
        ));
    }
    tx.execute(
        "UPDATE document SET statut = ?1 WHERE id = ?2",
        params![Statut::Annule.code(), id],
    )?;
    journaliser_statut(
        &tx,
        id,
        "annulation",
        Some(json!({ "statut": d.statut.code() })),
        json!({ "statut": Statut::Annule.code(), "numero": d.numero }),
        acteur,
    )?;
    tx.commit()?;
    Ok(())
}

/// Champs administratifs d'un document Validé ou Envoyé (section 7.2).
pub fn modifier_administratif(
    conn: &mut Connection,
    acteur: &Utilisateur,
    id: i64,
    mode_paiement: &str,
    observations: &str,
) -> Resultat<()> {
    let tx = conn.transaction()?;
    let d = lire_existant(&tx, id)?;
    exiger(&tx, acteur, Droit::brouillon(d.type_document))?;
    if matches!(d.statut, Statut::Brouillon | Statut::Annule) {
        return Err(refus(
            "les champs administratifs se modifient sur un document validé ; celui-ci est « {} »",
            &[&d.statut],
        ));
    }
    let (mode, obs) = (mode_paiement.trim(), observations.trim());
    if mode == d.saisie.mode_paiement && obs == d.saisie.observations {
        return Ok(());
    }
    tx.execute(
        "UPDATE document SET mode_paiement = ?1, observations = ?2 WHERE id = ?3",
        params![mode, obs, id],
    )?;
    audit::journaliser(
        &tx,
        Entree {
            entite: "document",
            entite_id: id,
            action: "modification_administrative",
            ancienne_valeur: Some(json!({
                "mode_paiement": d.saisie.mode_paiement,
                "observations": d.saisie.observations,
            })),
            nouvelle_valeur: Some(json!({ "mode_paiement": mode, "observations": obs })),
            utilisateur_id: Some(acteur.id),
        },
    )?;
    tx.commit()?;
    Ok(())
}

/// Visas et dates d'un bordereau Validé, avant la confirmation de la livraison :
/// le visa du client et la date de réception se remplissent une fois la
/// marchandise remise (décision du 2026-11-10 : confirmation après le visa).
/// En brouillon, ils passent par `modifier_brouillon` ; après la confirmation,
/// ils sont figés. Même droit que les champs administratifs (section 10.1).
pub fn modifier_livraison(
    conn: &mut Connection,
    acteur: &Utilisateur,
    id: i64,
    champs: &ChampsLivraison,
) -> Resultat<()> {
    let tx = conn.transaction()?;
    let d = lire_existant(&tx, id)?;
    exiger(&tx, acteur, Droit::brouillon(d.type_document))?;
    if d.type_document != TypeDocument::Livraison {
        return Err(invalide(
            "les visas et dates ne concernent que les bordereaux de livraison",
        ));
    }
    if d.statut != Statut::Valide {
        return Err(refus(
            "les visas et dates se modifient sur un bordereau validé, avant la confirmation ; celui-ci est « {} »",
            &[&d.statut],
        ));
    }
    let nouveaux = ChampsLivraison {
        visa_client: champs.visa_client.trim().to_string(),
        visa_fournisseur: champs.visa_fournisseur.trim().to_string(),
        date_livraison: champs.date_livraison.trim().to_string(),
        date_reception: champs.date_reception.trim().to_string(),
    };
    let anciens = &d.saisie.livraison;
    if &nouveaux == anciens {
        return Ok(());
    }
    tx.execute(
        "UPDATE document SET visa_client = ?1, visa_fournisseur = ?2,
             date_livraison_texte = ?3, date_reception_texte = ?4
         WHERE id = ?5",
        params![
            nouveaux.visa_client,
            nouveaux.visa_fournisseur,
            nouveaux.date_livraison,
            nouveaux.date_reception,
            id
        ],
    )?;
    let en_json = |c: &ChampsLivraison| {
        json!({
            "visa_client": c.visa_client,
            "visa_fournisseur": c.visa_fournisseur,
            "date_livraison": c.date_livraison,
            "date_reception": c.date_reception,
        })
    };
    audit::journaliser(
        &tx,
        Entree {
            entite: "document",
            entite_id: id,
            action: "modification_livraison",
            ancienne_valeur: Some(en_json(anciens)),
            nouvelle_valeur: Some(en_json(&nouveaux)),
            utilisateur_id: Some(acteur.id),
        },
    )?;
    tx.commit()?;
    Ok(())
}

/// Droit d'enregistrer la réception d'un bon de commande : « Valider un
/// document », comme le marquage envoyé (décision du 2026-11-10, section 10.1).
fn droit_reception() -> Droit {
    Droit::ValiderDocument
}

/// Réception d'un bon de commande Envoyé ou en Réception partielle (section 7.7) :
/// quantité reçue cumulée de chaque ligne, dans l'ordre des lignes. Le statut suit :
/// Réception complète quand chaque ligne a reçu sa quantité commandée, Réception
/// partielle dès qu'une quantité est arrivée. Une saisie qui ferait revenir en
/// arrière (tout à zéro après une réception partielle) est refusée ; Réception
/// complète est définitive.
pub fn enregistrer_reception(
    conn: &mut Connection,
    acteur: &Utilisateur,
    id: i64,
    recues: &[Quantite],
) -> Resultat<()> {
    let tx = conn.transaction()?;
    let d = lire_existant(&tx, id)?;
    exiger(&tx, acteur, droit_reception())?;
    if d.type_document != TypeDocument::BonCommande {
        return Err(invalide(
            "la réception ne concerne que les bons de commande",
        ));
    }
    if !matches!(d.statut, Statut::Envoye | Statut::ReceptionPartielle) {
        return Err(refus(
            "la réception s'enregistre sur un bon de commande envoyé ; celui-ci est « {} »",
            &[&d.statut],
        ));
    }
    if recues.len() != d.saisie.lignes.len() {
        return Err(invalide(
            "la réception doit donner une quantité pour chaque ligne du bon de commande",
        ));
    }
    for (rang, (ligne, &recue)) in d.saisie.lignes.iter().zip(recues).enumerate() {
        let n = rang + 1;
        if recue < 0 {
            return Err(refus(
                "ligne {} : la quantité reçue ne peut pas être négative",
                &[&n],
            ));
        }
        // Dépassement refusé, pas simple avertissement (décision du 2026-11-10).
        if recue > ligne.quantite {
            return Err(refus(
                "ligne {} : la quantité reçue dépasse la quantité commandée, la corriger",
                &[&n],
            ));
        }
    }
    let anciennes: Vec<Quantite> = d.saisie.lignes.iter().map(|l| l.quantite_livree).collect();
    if anciennes == recues {
        return Ok(());
    }
    let nouveau = match avancement(
        d.saisie
            .lignes
            .iter()
            .map(|l| l.quantite)
            .zip(recues.iter().copied()),
    ) {
        Avancement::Aucun if d.statut == Statut::Envoye => Statut::Envoye,
        Avancement::Aucun => {
            return Err(invalide(
                "une réception partielle est déjà enregistrée : au moins une quantité reçue reste nécessaire",
            ));
        }
        Avancement::Partiel => Statut::ReceptionPartielle,
        Avancement::Complet => Statut::ReceptionComplete,
    };
    if nouveau != d.statut {
        verifier_transition(d.type_document, d.statut, nouveau).map_err(transition_refusee)?;
    }
    let mut maj = tx.prepare(
        "UPDATE ligne_document SET quantite_livree = ?1
         WHERE id = (SELECT id FROM ligne_document WHERE document_id = ?2
                     ORDER BY ordre, id LIMIT 1 OFFSET ?3)",
    )?;
    for (rang, recue) in recues.iter().enumerate() {
        maj.execute(params![recue, id, rang as i64])?;
    }
    drop(maj);
    tx.execute(
        "UPDATE document SET statut = ?1 WHERE id = ?2",
        params![nouveau.code(), id],
    )?;
    journaliser_statut(
        &tx,
        id,
        "reception",
        Some(json!({ "statut": d.statut.code(), "quantites": anciennes })),
        json!({ "statut": nouveau.code(), "quantites": recues }),
        acteur,
    )?;
    tx.commit()?;
    Ok(())
}

fn libelle_statut(valeur: &Option<Value>) -> Option<String> {
    let code = valeur.as_ref()?.get("statut")?.as_str()?;
    Statut::depuis_code(code).map(|s| s.to_string())
}

fn texte_de<'a>(valeur: &'a Option<Value>, cle: &str) -> Option<&'a str> {
    valeur.as_ref()?.get(cle)?.as_str()
}

/// Libellé fixe de l'événement et valeurs à y placer, une par `{}`.
fn libelle_evenement(e: &EntreeLue) -> (String, Vec<String>) {
    let fixe = |modele: &str| (modele.to_string(), Vec::new());
    let avec = |modele: &str, valeur: &str| (modele.to_string(), vec![valeur.to_string()]);
    let numero = e
        .nouvelle_valeur
        .as_ref()
        .and_then(|v| v.get("numero"))
        .and_then(Value::as_str);
    match e.action.as_str() {
        "creation" => fixe("Création du brouillon"),
        "validation" => match numero {
            Some(n) => avec("Validation, numéro {}", n),
            None => fixe("Validation"),
        },
        "modification_administrative" => {
            fixe("Modification du mode de paiement ou des observations")
        }
        "modification_livraison" => fixe("Modification des visas et dates de livraison"),
        "conversion" => match texte_de(&e.nouvelle_valeur, "facture") {
            Some(f) => avec("Converti en facture {}", f),
            None => fixe("Converti en facture"),
        },
        "avoir" => match texte_de(&e.nouvelle_valeur, "avoir") {
            Some(a) => avec("Avoir {} validé", a),
            None => fixe("Avoir validé"),
        },
        "reception" => match libelle_statut(&e.nouvelle_valeur) {
            Some(statut) => avec("Réception enregistrée, {}", &statut),
            None => fixe("Réception enregistrée"),
        },
        "reglement" | "suppression_reglement" => {
            let valeur = if e.action == "reglement" {
                &e.nouvelle_valeur
            } else {
                &e.ancienne_valeur
            };
            let montant = valeur
                .as_ref()
                .and_then(|v| v.get("montant"))
                .and_then(Value::as_i64)
                .map(montant_en_texte);
            match (e.action.as_str(), montant) {
                ("reglement", Some(m)) => avec("Règlement de {} FCFA enregistré", &m),
                ("reglement", None) => fixe("Règlement enregistré"),
                (_, Some(m)) => avec("Règlement de {} FCFA supprimé", &m),
                (_, None) => fixe("Règlement supprimé"),
            }
        }
        "livraison" => match libelle_statut(&e.nouvelle_valeur) {
            Some(statut) => avec("Livraison confirmée, {}", &statut),
            None => fixe("Livraison confirmée"),
        },
        _ => match libelle_statut(&e.ancienne_valeur) {
            Some(ancien) => (
                "{} vers {}".to_string(),
                vec![
                    ancien,
                    libelle_statut(&e.nouvelle_valeur).unwrap_or_default(),
                ],
            ),
            None => fixe(&e.action),
        },
    }
}

/// Historique de la fiche, tiré du journal d'audit.
pub fn historique(conn: &Connection, id: i64) -> Resultat<Vec<Evenement>> {
    Ok(audit::lister_pour(conn, "document", id)?
        .iter()
        .map(|e| {
            let (modele, valeurs) = libelle_evenement(e);
            Evenement {
                date_action: e.date_action.clone(),
                utilisateur: e.utilisateur.clone(),
                libelle: composer(&modele, &valeurs),
                modele,
                valeurs,
            }
        })
        .collect())
}

/// État de paiement d'un document : seulement pour un type qui reçoit des
/// règlements, une fois validé et tant qu'il n'est pas annulé (section 8), y
/// compris un bon de commande en réception.
pub fn etat_paiement_de(conn: &Connection, d: &Document) -> Resultat<Option<EtatPaiement>> {
    if !d.type_document.recoit_des_reglements() || !d.statut.en_vigueur() {
        return Ok(None);
    }
    let regle: Montant = conn.query_row(
        "SELECT COALESCE(SUM(montant), 0) FROM reglement WHERE document_id = ?1",
        [d.id],
        |r| r.get(0),
    )?;
    let avoirs = if d.type_document == TypeDocument::Facture {
        avoirs_valides(conn, d.id)?
    } else {
        0
    };
    Ok(Some(etat_paiement(d.totaux.ttc - avoirs, regle)))
}

/// Date d'échéance proposée : date du document plus le délai de paiement réglé,
/// rien si aucun délai n'est réglé (question 10 du cahier).
pub fn echeance_par_defaut(date: NaiveDate, delai_jours: Option<u32>) -> Option<NaiveDate> {
    date.checked_add_days(chrono::Days::new(u64::from(delai_jours?)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::droits::tests::compte;
    use crate::ouvrir_en_memoire;
    use crate::tiers::{self, FicheTiers, TypeTiers};
    use kadenz_domaine::calcul::UNE_UNITE;
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
        fiche.adresse = "01 BP 1".into();
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

    fn ligne(designation: &str, prix: Montant, quantite: Quantite) -> LigneSaisie {
        LigneSaisie {
            designation: designation.into(),
            prix,
            quantite,
            ..Default::default()
        }
    }

    fn saisie(c: &Contexte, jour: NaiveDate) -> Saisie {
        Saisie {
            tiers_id: Some(c.client),
            taux: Taux {
                remise: 500,
                tva: 1800,
                retenue: 1000,
            },
            lignes: vec![
                ligne("Pelle", 100_000, 2 * UNE_UNITE),
                ligne("Brouette", 50_000, UNE_UNITE),
            ],
            ..Saisie::vide(jour)
        }
    }

    fn facture(c: &mut Contexte, jour: NaiveDate) -> i64 {
        let s = saisie(c, jour);
        creer_brouillon(&mut c.conn, &c.admin.clone(), TypeDocument::Facture, &s).unwrap()
    }

    #[test]
    fn brouillon_totaux_recalcules_et_snapshot() {
        let mut c = contexte();
        let id = facture(&mut c, date(2026, 5, 12));
        let d = lire(&c.conn, id).unwrap().unwrap();
        assert_eq!(d.statut, Statut::Brouillon);
        assert_eq!(d.numero, None);
        assert_eq!(d.totaux_lignes, [200_000, 50_000]);
        assert_eq!(d.totaux.ht, 250_000);
        assert_eq!(d.totaux.ttc, 280_250);
        assert_eq!(d.saisie.taux.retenue, 0, "pas de retenue sur une facture");
        assert_eq!(d.tiers.nom, "Client Alpha");
        assert_eq!(d.tiers.adresse, "01 BP 1");
        // Montants stockés : ceux du domaine, jamais ceux d'un écran.
        let ttc: Montant = c
            .conn
            .query_row(
                "SELECT montant_ttc FROM document WHERE id = ?1",
                [id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(ttc, 280_250);
    }

    #[test]
    fn saisie_controlee() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let mut s = saisie(&c, date(2026, 5, 12));
        s.lignes.push(ligne("  ", 1, UNE_UNITE));
        let e = creer_brouillon(&mut c.conn, &admin, TypeDocument::Facture, &s).unwrap_err();
        assert_eq!(e.to_string(), "ligne 3 : la désignation est obligatoire");

        let mut s = saisie(&c, date(2026, 5, 12));
        s.lignes[0].quantite = 0;
        assert!(creer_brouillon(&mut c.conn, &admin, TypeDocument::Facture, &s).is_err());

        let mut s = saisie(&c, date(2026, 5, 12));
        s.date_echeance = Some(date(2026, 5, 1));
        assert!(creer_brouillon(&mut c.conn, &admin, TypeDocument::Facture, &s).is_err());

        let mut s = saisie(&c, date(2026, 5, 12));
        s.tiers_id = Some(c.fournisseur);
        let e = creer_brouillon(&mut c.conn, &admin, TypeDocument::Facture, &s).unwrap_err();
        assert!(e.to_string().contains("n'est pas enregistré comme client"));
    }

    #[test]
    fn numerotation_par_mois_de_la_date() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let mai1 = facture(&mut c, date(2026, 5, 3));
        let juin = facture(&mut c, date(2026, 6, 1));
        let mai2 = facture(&mut c, date(2026, 5, 28));
        assert_eq!(valider(&mut c.conn, &admin, mai1).unwrap(), "F2026-05-0001");
        assert_eq!(valider(&mut c.conn, &admin, juin).unwrap(), "F2026-06-0001");
        assert_eq!(valider(&mut c.conn, &admin, mai2).unwrap(), "F2026-05-0002");
        let d = lire(&c.conn, mai2).unwrap().unwrap();
        assert_eq!(d.statut, Statut::Valide);
        assert!(d.date_validation.is_some());
        assert!(valider(&mut c.conn, &admin, mai2).is_err(), "déjà validé");
    }

    #[test]
    fn impression_figee_a_la_validation() {
        use crate::impression::pour_document;
        let mut c = contexte();
        let admin = c.admin.clone();
        c.conn
            .execute(
                "INSERT OR REPLACE INTO entreprise (id, nom) VALUES (1, 'Avant')",
                [],
            )
            .unwrap();
        let validee = facture(&mut c, date(2026, 5, 3));
        let deuxieme = facture(&mut c, date(2026, 5, 4));
        valider(&mut c.conn, &admin, validee).unwrap();
        valider(&mut c.conn, &admin, deuxieme).unwrap();
        let brouillon = facture(&mut c, date(2026, 5, 5));

        c.conn
            .execute("UPDATE entreprise SET nom = 'Après' WHERE id = 1", [])
            .unwrap();
        let mut p = crate::parametres::lire(&c.conn).unwrap();
        p.textes_documents[4].titre = "FACTURE DEFINITIVE".into();
        crate::parametres::enregistrer(&c.conn, &admin, &p).unwrap();
        let compacte = kadenz_domaine::mise_en_page::MiseEnPage {
            gabarit: kadenz_domaine::mise_en_page::Gabarit::Compact,
            ..Default::default()
        };
        crate::mise_en_page::enregistrer(&c.conn, &admin, &compacte).unwrap();

        let figee = pour_document(&c.conn, validee).unwrap();
        assert_eq!(figee.mise_en_page, crate::mise_en_page::defaut_analyse());
        assert_eq!(figee.entreprise.nom, "Avant");
        assert_eq!(figee.textes.titre, "FACTURE");
        assert_eq!(
            figee.orthographe,
            kadenz_domaine::lettres::Orthographe::default()
        );
        let apercu = pour_document(&c.conn, brouillon).unwrap();
        assert_eq!(apercu.entreprise.nom, "Après");
        assert_eq!(apercu.textes.titre, "FACTURE DEFINITIVE");
        assert_eq!(
            apercu.mise_en_page.gabarit,
            kadenz_domaine::mise_en_page::Gabarit::Compact
        );
        let instantanes: i64 = c
            .conn
            .query_row("SELECT COUNT(*) FROM instantane_entreprise", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(instantanes, 1, "instantané identique réutilisé");
        // Un instantané antérieur à J8b, sans mise en page, garde celle d'origine.
        c.conn
            .execute(
                "UPDATE instantane_entreprise SET donnees = json_remove(donnees, '$.mise_en_page')",
                [],
            )
            .unwrap();
        let ancienne = pour_document(&c.conn, validee).unwrap();
        assert_eq!(ancienne.mise_en_page, crate::mise_en_page::defaut_analyse());
        assert_eq!(ancienne.entreprise.nom, "Avant");
    }

    #[test]
    fn numerotation_annuelle_reglee() {
        use kadenz_domaine::numerotation::RemiseAZero;
        let mut c = contexte();
        let admin = c.admin.clone();
        let mai = facture(&mut c, date(2026, 5, 3));
        assert_eq!(valider(&mut c.conn, &admin, mai).unwrap(), "F2026-05-0001");

        let mut p = crate::parametres::lire(&c.conn).unwrap();
        p.numerotation.remise = RemiseAZero::Annuelle;
        crate::parametres::enregistrer(&c.conn, &admin, &p).unwrap();
        let juin = facture(&mut c, date(2026, 6, 1));
        let dec = facture(&mut c, date(2026, 12, 30));
        let an_suivant = facture(&mut c, date(2027, 1, 2));
        assert_eq!(valider(&mut c.conn, &admin, juin).unwrap(), "F2026-0001");
        assert_eq!(valider(&mut c.conn, &admin, dec).unwrap(), "F2026-0002");
        assert_eq!(
            valider(&mut c.conn, &admin, an_suivant).unwrap(),
            "F2027-0001"
        );
    }

    #[test]
    fn validation_refusee_sans_toucher_au_compteur() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let mut s = saisie(&c, date(2026, 5, 3));
        s.tiers_id = None;
        let sans_tiers = creer_brouillon(&mut c.conn, &admin, TypeDocument::Facture, &s).unwrap();
        let e = valider(&mut c.conn, &admin, sans_tiers).unwrap_err();
        assert_eq!(e.to_string(), "choisir le tiers avant de valider");
        let mut s = saisie(&c, date(2026, 5, 3));
        s.lignes.clear();
        let vide = creer_brouillon(&mut c.conn, &admin, TypeDocument::Facture, &s).unwrap();
        assert!(valider(&mut c.conn, &admin, vide).is_err());
        let compteurs: i64 = c
            .conn
            .query_row("SELECT COUNT(*) FROM compteur_numerotation", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(compteurs, 0);
        let bonne = facture(&mut c, date(2026, 5, 9));
        assert_eq!(
            valider(&mut c.conn, &admin, bonne).unwrap(),
            "F2026-05-0001"
        );
    }

    #[test]
    fn document_valide_fige() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let id = facture(&mut c, date(2026, 5, 3));
        valider(&mut c.conn, &admin, id).unwrap();
        let s = saisie(&c, date(2026, 5, 3));
        assert!(modifier_brouillon(&mut c.conn, &admin, id, &s).is_err());
        assert!(supprimer_brouillon(&mut c.conn, &admin, id).is_err());

        // Modifier la fiche du tiers ne change pas le document.
        let mut fiche = tiers::lire(&c.conn, c.client).unwrap().unwrap().fiche;
        fiche.nom = "Client Alpha renommé".into();
        tiers::modifier(&c.conn, &admin, c.client, &fiche).unwrap();
        assert_eq!(
            lire(&c.conn, id).unwrap().unwrap().tiers.nom,
            "Client Alpha"
        );

        // Champs administratifs seulement.
        modifier_administratif(&mut c.conn, &admin, id, " Chèque ", "remis en main propre")
            .unwrap();
        let d = lire(&c.conn, id).unwrap().unwrap();
        assert_eq!(d.saisie.mode_paiement, "Chèque");
        assert_eq!(d.totaux.ttc, 280_250);
    }

    #[test]
    fn envoi_annulation_et_numero_conserve() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let id = facture(&mut c, date(2026, 5, 3));
        assert!(
            marquer_envoye(&mut c.conn, &admin, id).is_err(),
            "brouillon"
        );
        let numero = valider(&mut c.conn, &admin, id).unwrap();
        marquer_envoye(&mut c.conn, &admin, id).unwrap();
        assert!(lire(&c.conn, id).unwrap().unwrap().date_envoi.is_some());
        annuler(&mut c.conn, &admin, id).unwrap();
        let d = lire(&c.conn, id).unwrap().unwrap();
        assert_eq!(d.statut, Statut::Annule);
        assert_eq!(d.numero.as_deref(), Some(numero.as_str()));
        assert!(annuler(&mut c.conn, &admin, id).is_err(), "déjà annulé");
        assert!(modifier_administratif(&mut c.conn, &admin, id, "Espèce", "").is_err());

        // Le numéro annulé n'est jamais réutilisé.
        let suivante = facture(&mut c, date(2026, 5, 4));
        assert_eq!(
            valider(&mut c.conn, &admin, suivante).unwrap(),
            "F2026-05-0002"
        );
    }

    #[test]
    fn suppression_du_brouillon_tracee() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let id = facture(&mut c, date(2026, 5, 3));
        supprimer_brouillon(&mut c.conn, &admin, id).unwrap();
        assert!(lire(&c.conn, id).unwrap().is_none());
        let lignes: i64 = c
            .conn
            .query_row("SELECT COUNT(*) FROM ligne_document", [], |r| r.get(0))
            .unwrap();
        assert_eq!(lignes, 0);
        let entrees = audit::lister_pour(&c.conn, "document", id).unwrap();
        let derniere = entrees.last().unwrap();
        assert_eq!(derniere.action, "suppression");
        assert_eq!(
            derniere.ancienne_valeur.as_ref().unwrap()["montant_ttc"],
            json!(280_250)
        );
    }

    #[test]
    fn identifiant_jamais_reutilise_apres_suppression() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let supprime = facture(&mut c, date(2026, 5, 3));
        supprimer_brouillon(&mut c.conn, &admin, supprime).unwrap();
        let nouveau = facture(&mut c, date(2026, 5, 4));
        assert_ne!(nouveau, supprime);
        let libelles: Vec<String> = historique(&c.conn, nouveau)
            .unwrap()
            .into_iter()
            .map(|e| e.libelle)
            .collect();
        assert_eq!(libelles, ["Création du brouillon"]);

        // Même règle pour un tiers retiré puis recréé.
        let mut fiche = FicheTiers::vide(TypeTiers::Client);
        fiche.nom = "Client éphémère".into();
        let retire = tiers::creer(&c.conn, &admin, &fiche).unwrap().id;
        tiers::retirer(&c.conn, &admin, retire).unwrap();
        fiche.nom = "Client suivant".into();
        assert_ne!(tiers::creer(&c.conn, &admin, &fiche).unwrap().id, retire);
    }

    #[test]
    fn droits_du_commercial() {
        let mut c = contexte();
        let vente = c.vente.clone();
        let s = saisie(&c, date(2026, 5, 3));
        assert!(matches!(
            creer_brouillon(&mut c.conn, &vente, TypeDocument::Facture, &s),
            Err(Erreur::Interdit(Droit::BrouillonFacture))
        ));
        let id = facture(&mut c, date(2026, 5, 3));
        assert!(matches!(
            valider(&mut c.conn, &vente, id),
            Err(Erreur::Interdit(Droit::ValiderDocument))
        ));
        assert!(matches!(
            supprimer_brouillon(&mut c.conn, &vente, id),
            Err(Erreur::Interdit(Droit::BrouillonFacture))
        ));
    }

    #[test]
    fn historique_des_transitions() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let id = facture(&mut c, date(2026, 5, 3));
        let s = saisie(&c, date(2026, 5, 4));
        modifier_brouillon(&mut c.conn, &admin, id, &s).unwrap();
        valider(&mut c.conn, &admin, id).unwrap();
        marquer_envoye(&mut c.conn, &admin, id).unwrap();
        annuler(&mut c.conn, &admin, id).unwrap();
        let libelles: Vec<String> = historique(&c.conn, id)
            .unwrap()
            .into_iter()
            .map(|e| e.libelle)
            .collect();
        assert_eq!(
            libelles,
            [
                "Création du brouillon",
                "Validation, numéro F2026-05-0001",
                "Validé vers Envoyé",
                "Envoyé vers Annulé",
            ]
        );
        // Libellé fixe et valeurs à part, pour la traduction côté écran.
        let evenements = historique(&c.conn, id).unwrap();
        assert_eq!(
            (evenements[0].modele.as_str(), evenements[0].valeurs.len()),
            ("Création du brouillon", 0)
        );
        assert_eq!(evenements[1].modele, "Validation, numéro {}");
        assert_eq!(evenements[1].valeurs, ["F2026-05-0001"]);
        assert_eq!(evenements[2].modele, "{} vers {}");
        assert_eq!(evenements[2].valeurs, ["Validé", "Envoyé"]);
        assert_eq!(
            historique(&c.conn, id).unwrap()[0].utilisateur,
            "Compte admin"
        );
    }

    #[test]
    fn registre_filtre_et_etat_de_paiement() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let a = facture(&mut c, date(2026, 5, 3));
        let b = facture(&mut c, date(2026, 6, 3));
        valider(&mut c.conn, &admin, a).unwrap();
        c.conn
            .execute(
                "INSERT INTO reglement (document_id, date, montant) VALUES (?1, '2026-05-10', 1000)",
                [a],
            )
            .unwrap();
        let tous = lister(&c.conn, TypeDocument::Facture, &FiltreDocuments::default()).unwrap();
        assert_eq!(tous.iter().map(|d| d.id).collect::<Vec<_>>(), [b, a]);
        assert_eq!(
            tous[0].etat_paiement, None,
            "brouillon : pas d'état de paiement"
        );
        assert_eq!(tous[1].etat_paiement, Some(EtatPaiement::Partiel));

        let filtre = FiltreDocuments {
            texte: "f2026-05".into(),
            ..Default::default()
        };
        assert_eq!(
            lister(&c.conn, TypeDocument::Facture, &filtre)
                .unwrap()
                .len(),
            1
        );
        let filtre = FiltreDocuments {
            du: Some(date(2026, 6, 1)),
            ..Default::default()
        };
        assert_eq!(
            lister(&c.conn, TypeDocument::Facture, &filtre).unwrap()[0].id,
            b
        );
        let filtre = FiltreDocuments {
            statut: Some(Statut::Valide),
            texte: "alpha".into(),
            ..Default::default()
        };
        assert_eq!(
            lister(&c.conn, TypeDocument::Facture, &filtre).unwrap()[0].id,
            a
        );
        assert!(
            lister(&c.conn, TypeDocument::Devis, &FiltreDocuments::default())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn tiers_desactive_garde_sur_le_brouillon() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let id = facture(&mut c, date(2026, 5, 3));
        tiers::definir_actif(&c.conn, &admin, c.client, false).unwrap();
        let s = saisie(&c, date(2026, 5, 3));
        modifier_brouillon(&mut c.conn, &admin, id, &s).unwrap();
        let e = creer_brouillon(&mut c.conn, &admin, TypeDocument::Facture, &s).unwrap_err();
        assert!(e.to_string().contains("désactivé"));
    }

    #[test]
    fn echeance_proposee() {
        assert_eq!(echeance_par_defaut(date(2026, 5, 3), None), None);
        assert_eq!(
            echeance_par_defaut(date(2026, 5, 3), Some(30)),
            Some(date(2026, 6, 2))
        );
    }

    fn devis_valide(c: &mut Contexte) -> i64 {
        let admin = c.admin.clone();
        let s = saisie(c, date(2026, 5, 3));
        let id = creer_brouillon(&mut c.conn, &admin, TypeDocument::Devis, &s).unwrap();
        valider(&mut c.conn, &admin, id).unwrap();
        id
    }

    fn facture_validee(c: &mut Contexte) -> i64 {
        let admin = c.admin.clone();
        let id = facture(c, date(2026, 5, 3));
        valider(&mut c.conn, &admin, id).unwrap();
        id
    }

    fn doc(c: &Contexte, id: i64) -> Document {
        lire(&c.conn, id).unwrap().unwrap()
    }

    #[test]
    fn conversion_a_la_validation_de_la_facture() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let devis = devis_valide(&mut c);
        let f = convertir_en_facture(&mut c.conn, &admin, devis).unwrap();
        let facture = doc(&c, f);
        assert_eq!(facture.type_document, TypeDocument::Facture);
        assert_eq!(facture.statut, Statut::Brouillon);
        assert_eq!(facture.saisie.lignes, doc(&c, devis).saisie.lignes);
        assert_eq!(
            facture.saisie.taux.retenue, 0,
            "pas de retenue sur une facture"
        );
        let source = facture.source.unwrap();
        assert_eq!((source.id, source.nature), (devis, NatureLien::Conversion));
        // La source ne change pas tant que la facture est en brouillon.
        let d = doc(&c, devis);
        assert_eq!(d.statut, Statut::Valide);
        assert_eq!(d.derives.len(), 1);
        assert!(convertir_en_facture(&mut c.conn, &admin, devis).is_err());
        assert!(annuler(&mut c.conn, &admin, devis).is_err());
        assert!(dupliquer(&mut c.conn, &admin, devis, true).is_err());
        // Validation de la facture : la source passe en Converti.
        let numero = valider(&mut c.conn, &admin, f).unwrap();
        assert_eq!(doc(&c, devis).statut, Statut::Converti);
        let dernier = historique(&c.conn, devis).unwrap().pop().unwrap();
        assert_eq!(dernier.libelle, format!("Converti en facture {numero}"));
        assert!(convertir_en_facture(&mut c.conn, &admin, devis).is_err());
    }

    #[test]
    fn brouillon_de_facture_supprime_source_reconvertible() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let devis = devis_valide(&mut c);
        marquer_envoye(&mut c.conn, &admin, devis).unwrap();
        let f = convertir_en_facture(&mut c.conn, &admin, devis).unwrap();
        supprimer_brouillon(&mut c.conn, &admin, f).unwrap();
        assert_eq!(doc(&c, devis).statut, Statut::Envoye);
        assert!(convertir_en_facture(&mut c.conn, &admin, devis).is_ok());
    }

    #[test]
    fn conversion_refusee() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let s = saisie(&c, date(2026, 5, 3));
        let brouillon = creer_brouillon(&mut c.conn, &admin, TypeDocument::Proforma, &s).unwrap();
        assert!(convertir_en_facture(&mut c.conn, &admin, brouillon).is_err());
        let facture = facture_validee(&mut c);
        assert!(convertir_en_facture(&mut c.conn, &admin, facture).is_err());
        // Source annulée entre-temps (par une voie que l'application interdit) :
        // la validation de la facture est refusée.
        let devis = devis_valide(&mut c);
        let f = convertir_en_facture(&mut c.conn, &admin, devis).unwrap();
        c.conn
            .execute(
                "UPDATE document SET statut = 'annule' WHERE id = ?1",
                [devis],
            )
            .unwrap();
        let e = valider(&mut c.conn, &admin, f).unwrap_err();
        assert!(e.to_string().contains("ne peut plus être converti"), "{e}");
        assert_eq!(doc(&c, f).statut, Statut::Brouillon);
    }

    #[test]
    fn correction_et_copie() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let devis = devis_valide(&mut c);
        let correction = dupliquer(&mut c.conn, &admin, devis, true).unwrap();
        assert_eq!(doc(&c, devis).statut, Statut::Annule);
        let n = doc(&c, correction);
        assert_eq!(
            (n.type_document, n.statut),
            (TypeDocument::Devis, Statut::Brouillon)
        );
        assert_eq!(n.source.unwrap().nature, NatureLien::Correction);
        assert_eq!(doc(&c, devis).derives[0].id, correction);

        let facture = facture_validee(&mut c);
        assert!(dupliquer(&mut c.conn, &admin, facture, true).is_err());
        let copie = dupliquer(&mut c.conn, &admin, facture, false).unwrap();
        assert_eq!(doc(&c, facture).statut, Statut::Valide);
        assert_eq!(doc(&c, copie).source.unwrap().nature, NatureLien::Copie);
        let avoir = creer_avoir(&mut c.conn, &admin, facture).unwrap();
        assert!(dupliquer(&mut c.conn, &admin, avoir, false).is_err());
    }

    #[test]
    fn avoir_plafonne_au_ttc_de_la_facture() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let facture = facture_validee(&mut c);
        assert_eq!(doc(&c, facture).reste_a_crediter, Some(280_250));

        // Avoir partiel : seulement la première ligne (Pelle, 200 000 HT).
        let partiel = creer_avoir(&mut c.conn, &admin, facture).unwrap();
        let mut s = doc(&c, partiel).saisie;
        assert_eq!(s.lignes.len(), 2);
        s.lignes.truncate(1);
        modifier_brouillon(&mut c.conn, &admin, partiel, &s).unwrap();
        let mut autre_client = s.clone();
        autre_client.tiers_id = Some(c.fournisseur);
        assert!(modifier_brouillon(&mut c.conn, &admin, partiel, &autre_client).is_err());
        let mut autres_taux = s.clone();
        autres_taux.taux.tva = 0;
        assert!(modifier_brouillon(&mut c.conn, &admin, partiel, &autres_taux).is_err());
        valider(&mut c.conn, &admin, partiel).unwrap();
        let f = doc(&c, facture);
        assert_eq!(f.statut, Statut::Valide);
        assert_eq!(f.reste_a_crediter, Some(280_250 - 224_200));
        assert_eq!(f.montant_du, Some(56_050));
        assert!(
            historique(&c.conn, facture)
                .unwrap()
                .iter()
                .any(|e| e.libelle.starts_with("Avoir A"))
        );

        // Un second avoir complet dépasse le reste à créditer.
        let total = creer_avoir(&mut c.conn, &admin, facture).unwrap();
        let e = valider(&mut c.conn, &admin, total).unwrap_err();
        assert!(e.to_string().contains("dépasse"), "{e}");
        supprimer_brouillon(&mut c.conn, &admin, total).unwrap();

        // La facture ne s'annule pas tant qu'elle a un avoir validé ; annuler
        // l'avoir lui rend son montant.
        assert!(annuler(&mut c.conn, &admin, facture).is_err());
        annuler(&mut c.conn, &admin, partiel).unwrap();
        assert_eq!(doc(&c, facture).reste_a_crediter, Some(280_250));
        annuler(&mut c.conn, &admin, facture).unwrap();
    }

    #[test]
    fn facture_couverte_par_ses_avoirs() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let facture = facture_validee(&mut c);
        let avoir = creer_avoir(&mut c.conn, &admin, facture).unwrap();
        valider(&mut c.conn, &admin, avoir).unwrap();
        let f = doc(&c, facture);
        assert_eq!(f.statut, Statut::Valide, "statut inchangé");
        assert_eq!(f.montant_du, Some(0));
        assert_eq!(
            etat_paiement_de(&c.conn, &f).unwrap(),
            Some(EtatPaiement::Paye)
        );
        let registre = lister(&c.conn, TypeDocument::Facture, &FiltreDocuments::default()).unwrap();
        assert_eq!(registre[0].etat_paiement, Some(EtatPaiement::Paye));
        assert!(creer_avoir(&mut c.conn, &admin, facture).is_err());
    }

    #[test]
    fn avoir_seulement_depuis_une_facture_validee() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let s = saisie(&c, date(2026, 5, 3));
        assert!(creer_brouillon(&mut c.conn, &admin, TypeDocument::Avoir, &s).is_err());
        let brouillon = facture(&mut c, date(2026, 5, 3));
        assert!(creer_avoir(&mut c.conn, &admin, brouillon).is_err());
        let devis = devis_valide(&mut c);
        assert!(creer_avoir(&mut c.conn, &admin, devis).is_err());
        // Facture annulée après la création de l'avoir brouillon.
        let facture = facture_validee(&mut c);
        let avoir = creer_avoir(&mut c.conn, &admin, facture).unwrap();
        annuler(&mut c.conn, &admin, facture).unwrap();
        assert!(valider(&mut c.conn, &admin, avoir).is_err());
    }

    #[test]
    fn client_desactive_depuis_la_source() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let devis = devis_valide(&mut c);
        let facture = facture_validee(&mut c);
        tiers::definir_actif(&c.conn, &admin, c.client, false).unwrap();
        let f = convertir_en_facture(&mut c.conn, &admin, devis).unwrap();
        valider(&mut c.conn, &admin, f).unwrap();
        let avoir = creer_avoir(&mut c.conn, &admin, facture).unwrap();
        valider(&mut c.conn, &admin, avoir).unwrap();
        assert!(dupliquer(&mut c.conn, &admin, facture, false).is_ok());
    }

    #[test]
    fn droits_des_documents_lies() {
        let mut c = contexte();
        let vente = c.vente.clone();
        let devis = devis_valide(&mut c);
        assert!(matches!(
            convertir_en_facture(&mut c.conn, &vente, devis),
            Err(Erreur::Interdit(Droit::BrouillonFacture))
        ));
        let facture = facture_validee(&mut c);
        assert!(matches!(
            creer_avoir(&mut c.conn, &vente, facture),
            Err(Erreur::Interdit(Droit::BrouillonFacture))
        ));
    }

    /// Bon de commande au fournisseur : 2 pelles et 1 brouette, laissé au statut donné.
    fn bon_commande(c: &mut Contexte, jusqu_a: Statut) -> i64 {
        let admin = c.admin.clone();
        let s = Saisie {
            tiers_id: Some(c.fournisseur),
            ..saisie(c, date(2026, 6, 2))
        };
        let id = creer_brouillon(&mut c.conn, &admin, TypeDocument::BonCommande, &s).unwrap();
        if jusqu_a != Statut::Brouillon {
            valider(&mut c.conn, &admin, id).unwrap();
        }
        if jusqu_a == Statut::Envoye {
            marquer_envoye(&mut c.conn, &admin, id).unwrap();
        }
        id
    }

    fn recues(c: &Contexte, id: i64) -> Vec<Quantite> {
        doc(c, id)
            .saisie
            .lignes
            .iter()
            .map(|l| l.quantite_livree)
            .collect()
    }

    #[test]
    fn reception_partielle_puis_complete() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let bc = bon_commande(&mut c, Statut::Envoye);
        assert_eq!(recues(&c, bc), [0, 0]);
        enregistrer_reception(&mut c.conn, &admin, bc, &[UNE_UNITE, 0]).unwrap();
        assert_eq!(doc(&c, bc).statut, Statut::ReceptionPartielle);
        assert_eq!(recues(&c, bc), [UNE_UNITE, 0]);
        // Une ligne peut être corrigée à la baisse tant que la réception reste partielle.
        enregistrer_reception(&mut c.conn, &admin, bc, &[0, UNE_UNITE / 2]).unwrap();
        assert_eq!(doc(&c, bc).statut, Statut::ReceptionPartielle);
        // Revenir à rien reçu est refusé.
        assert!(enregistrer_reception(&mut c.conn, &admin, bc, &[0, 0]).is_err());
        enregistrer_reception(&mut c.conn, &admin, bc, &[2 * UNE_UNITE, UNE_UNITE]).unwrap();
        let d = doc(&c, bc);
        assert_eq!(d.statut, Statut::ReceptionComplete);
        // Le bon de commande garde son état de paiement et son montant dû.
        assert_eq!(d.montant_du, Some(d.totaux.ttc));
        assert!(etat_paiement_de(&c.conn, &d).unwrap().is_some());
        let au_registre = lister(
            &c.conn,
            TypeDocument::BonCommande,
            &FiltreDocuments::default(),
        )
        .unwrap();
        assert!(au_registre[0].etat_paiement.is_some());
        // Réception complète : définitive.
        assert!(enregistrer_reception(&mut c.conn, &admin, bc, &[UNE_UNITE, UNE_UNITE]).is_err());
        // Historique : libellé fixe et statut à part.
        let evenements = historique(&c.conn, bc).unwrap();
        let dernier = evenements.last().unwrap();
        assert_eq!(dernier.modele, "Réception enregistrée, {}");
        assert_eq!(dernier.valeurs, ["Réception complète"]);
    }

    #[test]
    fn reception_refusee_hors_cas_prevus() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let brouillon = bon_commande(&mut c, Statut::Brouillon);
        assert!(enregistrer_reception(&mut c.conn, &admin, brouillon, &[0, UNE_UNITE]).is_err());
        let valide = bon_commande(&mut c, Statut::Valide);
        assert!(enregistrer_reception(&mut c.conn, &admin, valide, &[0, UNE_UNITE]).is_err());
        let bc = bon_commande(&mut c, Statut::Envoye);
        // Nombre de lignes, quantité négative, dépassement de la quantité commandée.
        assert!(enregistrer_reception(&mut c.conn, &admin, bc, &[UNE_UNITE]).is_err());
        assert!(enregistrer_reception(&mut c.conn, &admin, bc, &[-1, 0]).is_err());
        assert!(enregistrer_reception(&mut c.conn, &admin, bc, &[3 * UNE_UNITE, 0]).is_err());
        assert_eq!(recues(&c, bc), [0, 0]);
        assert_eq!(doc(&c, bc).statut, Statut::Envoye);
        // Une facture ne se réceptionne pas.
        let facture = facture_validee(&mut c);
        assert!(enregistrer_reception(&mut c.conn, &admin, facture, &[0, 0]).is_err());
        // Droit : le rôle Commercial ne valide pas, il ne réceptionne donc pas.
        let vente = c.vente.clone();
        assert!(matches!(
            enregistrer_reception(&mut c.conn, &vente, bc, &[UNE_UNITE, 0]),
            Err(Erreur::Interdit(Droit::ValiderDocument))
        ));
        // Annulation refusée après une réception partielle (section 7.1).
        enregistrer_reception(&mut c.conn, &admin, bc, &[UNE_UNITE, 0]).unwrap();
        assert!(annuler(&mut c.conn, &admin, bc).is_err());
    }

    #[test]
    fn brouillon_et_copie_d_un_bc_sans_quantite_recue() {
        let mut c = contexte();
        let admin = c.admin.clone();
        // Une quantité reçue passée par le brouillon est ignorée.
        let mut s = Saisie {
            tiers_id: Some(c.fournisseur),
            ..saisie(&c, date(2026, 6, 2))
        };
        s.lignes[0].quantite_livree = UNE_UNITE;
        let id = creer_brouillon(&mut c.conn, &admin, TypeDocument::BonCommande, &s).unwrap();
        assert_eq!(recues(&c, id), [0, 0]);
        // La copie d'un bon de commande réceptionné repart de zéro.
        let bc = bon_commande(&mut c, Statut::Envoye);
        enregistrer_reception(&mut c.conn, &admin, bc, &[UNE_UNITE, 0]).unwrap();
        let copie = dupliquer(&mut c.conn, &admin, bc, false).unwrap();
        assert_eq!(recues(&c, copie), [0, 0]);
    }

    /// Quantités livrées d'un bordereau, ligne à ligne.
    fn livrees(c: &Contexte, id: i64) -> Vec<Quantite> {
        recues(c, id)
    }

    /// Enregistre de nouvelles quantités livrées sur un bordereau brouillon.
    fn livrer(c: &mut Contexte, id: i64, quantites: &[Quantite]) -> Resultat<()> {
        let admin = c.admin.clone();
        let mut s = doc(c, id).saisie;
        for (ligne, q) in s.lignes.iter_mut().zip(quantites) {
            ligne.quantite_livree = *q;
        }
        modifier_brouillon(&mut c.conn, &admin, id, &s)
    }

    #[test]
    fn bordereau_depuis_une_facture_et_reste_a_livrer() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let f = facture_validee(&mut c);
        // Facture de 2 pelles et 1 brouette.
        assert_eq!(doc(&c, f).restes_a_livrer, [2 * UNE_UNITE, UNE_UNITE]);
        let b = creer_bordereau(&mut c.conn, &admin, f).unwrap();
        let d = doc(&c, b);
        assert_eq!(
            (d.type_document, d.statut),
            (TypeDocument::Livraison, Statut::Brouillon)
        );
        assert_eq!(d.saisie.tiers_id, Some(c.client));
        assert_eq!(
            livrees(&c, b),
            [2 * UNE_UNITE, UNE_UNITE],
            "reste à livrer proposé"
        );
        assert_eq!(d.restes_a_livrer, [2 * UNE_UNITE, UNE_UNITE]);
        assert_eq!(d.source.unwrap().nature, NatureLien::Livraison);
        let derive = doc(&c, f).derives.pop().unwrap();
        assert_eq!((derive.id, derive.nature), (b, NatureLien::Livraison));
        // Première livraison : une pelle.
        livrer(&mut c, b, &[UNE_UNITE, 0]).unwrap();
        valider(&mut c.conn, &admin, b).unwrap();
        assert_eq!(doc(&c, f).restes_a_livrer, [UNE_UNITE, UNE_UNITE]);
        // Second bordereau : le reste, plafonné.
        let b2 = creer_bordereau(&mut c.conn, &admin, f).unwrap();
        assert_eq!(livrees(&c, b2), [UNE_UNITE, UNE_UNITE]);
        assert_eq!(doc(&c, b2).restes_a_livrer, [UNE_UNITE, UNE_UNITE]);
        let e = livrer(&mut c, b2, &[2 * UNE_UNITE, 0]).unwrap_err();
        assert_eq!(
            e.to_string(),
            "ligne 1 : la quantité livrée dépasse le reste à livrer, la corriger"
        );
        // Le premier bordereau annulé rend son reste à la facture.
        annuler(&mut c.conn, &admin, b).unwrap();
        assert_eq!(doc(&c, f).restes_a_livrer, [2 * UNE_UNITE, UNE_UNITE]);
        livrer(&mut c, b2, &[2 * UNE_UNITE, UNE_UNITE]).unwrap();
        valider(&mut c.conn, &admin, b2).unwrap();
        assert_eq!(doc(&c, f).restes_a_livrer, [0, 0]);
        assert!(
            creer_bordereau(&mut c.conn, &admin, f).is_err(),
            "tout est livré"
        );
    }

    #[test]
    fn reste_a_livrer_reverifie_a_la_validation() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let f = facture_validee(&mut c);
        // Deux brouillons proposent chacun tout le reste.
        let b1 = creer_bordereau(&mut c.conn, &admin, f).unwrap();
        let b2 = creer_bordereau(&mut c.conn, &admin, f).unwrap();
        valider(&mut c.conn, &admin, b1).unwrap();
        let e = valider(&mut c.conn, &admin, b2).unwrap_err();
        assert!(e.to_string().contains("dépasse le reste à livrer"), "{e}");
        assert_eq!(doc(&c, b2).statut, Statut::Brouillon);
    }

    #[test]
    fn lignes_d_un_bordereau_issu_d_une_facture_verrouillees() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let f = facture_validee(&mut c);
        let b = creer_bordereau(&mut c.conn, &admin, f).unwrap();
        let origine = doc(&c, b).saisie;
        let mut s = origine.clone();
        s.lignes[0].designation = "Autre chose".into();
        assert!(modifier_brouillon(&mut c.conn, &admin, b, &s).is_err());
        let mut s = origine.clone();
        s.lignes[1].quantite = 5 * UNE_UNITE;
        assert!(modifier_brouillon(&mut c.conn, &admin, b, &s).is_err());
        let mut s = origine.clone();
        s.lignes.pop();
        assert!(modifier_brouillon(&mut c.conn, &admin, b, &s).is_err());
        let mut s = origine.clone();
        s.tiers_id = None;
        assert!(modifier_brouillon(&mut c.conn, &admin, b, &s).is_err());
        // Prix masqués à l'écran : repris de la facture ; observations libres.
        let mut s = origine.clone();
        s.lignes[0].prix = 0;
        s.lignes[0].observations = "Livré au dépôt".into();
        modifier_brouillon(&mut c.conn, &admin, b, &s).unwrap();
        let d = doc(&c, b);
        assert_eq!(d.saisie.lignes[0].prix, origine.lignes[0].prix);
        assert_eq!(d.saisie.lignes[0].observations, "Livré au dépôt");
        // Pas de copie : elle échapperait au reste à livrer.
        assert!(dupliquer(&mut c.conn, &admin, b, false).is_err());
    }

    #[test]
    fn bordereau_refuse_hors_facture_en_vigueur() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let brouillon = facture(&mut c, date(2026, 5, 3));
        assert!(creer_bordereau(&mut c.conn, &admin, brouillon).is_err());
        let annulee = facture_validee(&mut c);
        annuler(&mut c.conn, &admin, annulee).unwrap();
        assert!(creer_bordereau(&mut c.conn, &admin, annulee).is_err());
        let devis = devis_valide(&mut c);
        assert!(creer_bordereau(&mut c.conn, &admin, devis).is_err());
    }

    #[test]
    fn bordereau_vierge_client_visas_et_confirmation() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let mut s = Saisie {
            tiers_id: Some(c.fournisseur),
            ..saisie(&c, date(2026, 6, 10))
        };
        s.livraison = ChampsLivraison {
            visa_client: "  M. Client ".into(),
            visa_fournisseur: "Magasinier".into(),
            date_livraison: "10/06/2026".into(),
            date_reception: String::new(),
        };
        // Client seulement (décision du 2026-11-10).
        let e = creer_brouillon(&mut c.conn, &admin, TypeDocument::Livraison, &s).unwrap_err();
        assert!(e.to_string().contains("comme client"), "{e}");
        s.tiers_id = Some(c.client);
        s.lignes[0].quantite_livree = 3 * UNE_UNITE;
        assert!(creer_brouillon(&mut c.conn, &admin, TypeDocument::Livraison, &s).is_err());
        s.lignes[0].quantite_livree = 0;
        let b = creer_brouillon(&mut c.conn, &admin, TypeDocument::Livraison, &s).unwrap();
        assert_eq!(doc(&c, b).saisie.livraison.visa_client, "M. Client");
        // Rien à livrer : validation refusée.
        assert!(valider(&mut c.conn, &admin, b).is_err());
        livrer(&mut c, b, &[UNE_UNITE, UNE_UNITE]).unwrap();
        // Confirmation : seulement un bordereau validé.
        assert!(confirmer_livraison(&mut c.conn, &admin, b).is_err());
        valider(&mut c.conn, &admin, b).unwrap();
        confirmer_livraison(&mut c.conn, &admin, b).unwrap();
        assert_eq!(doc(&c, b).statut, Statut::LivraisonPartielle);
        assert!(
            confirmer_livraison(&mut c.conn, &admin, b).is_err(),
            "définitif"
        );
        let dernier = historique(&c.conn, b).unwrap().pop().unwrap();
        assert_eq!(dernier.modele, "Livraison confirmée, {}");
        assert_eq!(dernier.valeurs, ["Livraison partielle"]);
        // Les visas ne s'enregistrent que sur un bordereau.
        let mut s_facture = saisie(&c, date(2026, 6, 10));
        s_facture.livraison.visa_client = "X".into();
        let id = creer_brouillon(&mut c.conn, &admin, TypeDocument::Facture, &s_facture).unwrap();
        assert_eq!(doc(&c, id).saisie.livraison, ChampsLivraison::default());
    }

    #[test]
    fn livraison_complete_selon_les_bordereaux_confirmes() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let f = facture_validee(&mut c);
        // Premier bordereau : 1 pelle et la brouette, validé mais pas encore confirmé.
        let b1 = creer_bordereau(&mut c.conn, &admin, f).unwrap();
        livrer(&mut c, b1, &[UNE_UNITE, UNE_UNITE]).unwrap();
        valider(&mut c.conn, &admin, b1).unwrap();
        // Second : la dernière pelle. Le premier n'est pas confirmé : partielle.
        let b2 = creer_bordereau(&mut c.conn, &admin, f).unwrap();
        assert_eq!(livrees(&c, b2), [UNE_UNITE, 0]);
        valider(&mut c.conn, &admin, b2).unwrap();
        confirmer_livraison(&mut c.conn, &admin, b2).unwrap();
        assert_eq!(doc(&c, b2).statut, Statut::LivraisonPartielle);
        // Le premier confirmé à son tour : avec le second, tout est livré.
        confirmer_livraison(&mut c.conn, &admin, b1).unwrap();
        assert_eq!(doc(&c, b1).statut, Statut::LivraisonComplete);
        // Un bordereau confirmé ne s'annule plus (section 7.1).
        assert!(annuler(&mut c.conn, &admin, b1).is_err());
    }

    #[test]
    fn annulation_d_une_facture_selon_ses_bordereaux() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let f = facture_validee(&mut c);
        let b = creer_bordereau(&mut c.conn, &admin, f).unwrap();
        // Bordereau en brouillon, puis validé : refus (décision du 2026-11-10).
        let e = annuler(&mut c.conn, &admin, f).unwrap_err();
        assert!(e.to_string().contains("bordereaux non confirmés"), "{e}");
        valider(&mut c.conn, &admin, b).unwrap();
        assert!(annuler(&mut c.conn, &admin, f).is_err());
        // Livraison confirmée : la facture s'annule.
        confirmer_livraison(&mut c.conn, &admin, b).unwrap();
        annuler(&mut c.conn, &admin, f).unwrap();
        assert_eq!(doc(&c, f).statut, Statut::Annule);
        // Un bordereau annulé ne bloque pas non plus.
        let f2 = facture_validee(&mut c);
        let b2 = creer_bordereau(&mut c.conn, &admin, f2).unwrap();
        annuler(&mut c.conn, &admin, b2).unwrap();
        annuler(&mut c.conn, &admin, f2).unwrap();
    }

    #[test]
    fn visas_d_un_bordereau_valide_avant_confirmation() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let f = facture_validee(&mut c);
        let b = creer_bordereau(&mut c.conn, &admin, f).unwrap();
        let visas = ChampsLivraison {
            visa_client: " Reçu, M. Client ".into(),
            date_reception: "12/06/2026".into(),
            ..Default::default()
        };
        // En brouillon : par modifier_brouillon, pas ici.
        assert!(modifier_livraison(&mut c.conn, &admin, b, &visas).is_err());
        valider(&mut c.conn, &admin, b).unwrap();
        modifier_livraison(&mut c.conn, &admin, b, &visas).unwrap();
        let d = doc(&c, b);
        assert_eq!(d.saisie.livraison.visa_client, "Reçu, M. Client");
        assert_eq!(d.saisie.livraison.date_reception, "12/06/2026");
        let dernier = historique(&c.conn, b).unwrap().pop().unwrap();
        assert_eq!(
            dernier.modele,
            "Modification des visas et dates de livraison"
        );
        // Après la confirmation : figés.
        confirmer_livraison(&mut c.conn, &admin, b).unwrap();
        assert!(modifier_livraison(&mut c.conn, &admin, b, &ChampsLivraison::default()).is_err());
        // Pas sur une facture.
        assert!(modifier_livraison(&mut c.conn, &admin, f, &visas).is_err());
    }

    #[test]
    fn droits_du_bordereau() {
        let mut c = contexte();
        let admin = c.admin.clone();
        let vente = c.vente.clone();
        let f = facture_validee(&mut c);
        // Le rôle Commercial prépare un bordereau, sans le valider ni le confirmer.
        let b = creer_bordereau(&mut c.conn, &vente, f).unwrap();
        valider(&mut c.conn, &admin, b).unwrap();
        assert!(matches!(
            confirmer_livraison(&mut c.conn, &vente, b),
            Err(Erreur::Interdit(Droit::ValiderDocument))
        ));
    }
}
