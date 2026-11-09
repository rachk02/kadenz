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

use chrono::NaiveDate;
use kadenz_domaine::calcul::{
    CENT_POUR_CENT, Montant, Quantite, Taux, Totaux, calculer_totaux, total_ligne,
};
use kadenz_domaine::document::verifier_transition;
pub use kadenz_domaine::document::{Statut, TypeDocument};
pub use kadenz_domaine::paiement::EtatPaiement;
use kadenz_domaine::paiement::etat_paiement;
use kadenz_domaine::role::Droit;
use kadenz_domaine::texte;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use crate::audit::{self, Entree, EntreeLue};
use crate::comptes::Utilisateur;
use crate::droits::exiger;
use crate::{Erreur, Resultat, maintenant};

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
    pub libelle: String,
}

fn invalide(message: impl Into<String>) -> Erreur {
    Erreur::Validation(message.into())
}

fn date_texte(d: NaiveDate) -> String {
    d.format(FORMAT_DATE).to_string()
}

fn conversion(colonne: usize, message: String) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(colonne, rusqlite::types::Type::Text, message.into())
}

fn lire_date(r: &rusqlite::Row<'_>, colonne: usize) -> rusqlite::Result<NaiveDate> {
    let texte: String = r.get(colonne)?;
    NaiveDate::parse_from_str(&texte, FORMAT_DATE)
        .map_err(|_| conversion(colonne, format!("date illisible : {texte}")))
}

fn lire_date_facultative(
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

fn lire_type(r: &rusqlite::Row<'_>, colonne: usize) -> rusqlite::Result<TypeDocument> {
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
    if !type_document.retenue_applicable() {
        s.taux.retenue = 0;
    }
    for (nom, taux) in [
        ("remise", s.taux.remise),
        ("TVA", s.taux.tva),
        ("retenue", s.taux.retenue),
    ] {
        if !(0..=CENT_POUR_CENT).contains(&taux) {
            return Err(invalide(format!(
                "le taux de {nom} doit être compris entre 0 et 100 %"
            )));
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
            return Err(invalide(format!(
                "ligne {n} : la désignation est obligatoire"
            )));
        }
        if ligne.prix < 0 {
            return Err(invalide(format!(
                "ligne {n} : le prix ne peut pas être négatif"
            )));
        }
        if ligne.quantite <= 0 {
            return Err(invalide(format!(
                "ligne {n} : la quantité doit être supérieure à zéro"
            )));
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

/// Le tiers convient-il au type de document ? Client pour les ventes, fournisseur
/// pour le bon de commande, l'un ou l'autre pour le bordereau.
fn tiers_compatible(type_document: TypeDocument, t: &crate::tiers::Tiers) -> bool {
    match type_document {
        TypeDocument::BonCommande => t.fiche.type_tiers.est_fournisseur(),
        TypeDocument::Livraison => true,
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
        return Err(invalide(format!(
            "le tiers « {} » est désactivé",
            t.fiche.nom
        )));
    }
    if !tiers_compatible(type_document, &t) {
        let attendu = if type_document == TypeDocument::BonCommande {
            "fournisseur"
        } else {
            "client"
        };
        return Err(invalide(format!(
            "le tiers « {} » n'est pas enregistré comme {attendu}",
            t.fiche.nom
        )));
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
             mode_paiement = ?18, observations = ?19, date_echeance = ?20
         WHERE id = ?21",
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
            id
        ],
    )?;
    conn.execute("DELETE FROM ligne_document WHERE document_id = ?1", [id])?;
    let mut insertion = conn.prepare(
        "INSERT INTO ligne_document (document_id, produit_id, reference_snapshot,
             designation_snapshot, prix_snapshot, quantite, total, observations, ordre)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )?;
    for (ordre, l) in s.lignes.iter().enumerate() {
        insertion.execute(params![
            id,
            l.produit_id,
            l.reference,
            l.designation,
            l.prix,
            l.quantite,
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
    montant_tva, montant_retenue, montant_ttc, date_creation, date_validation, date_envoi";

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
    })
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
                observations, total
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
                    observations: r.get(5)?,
                },
                r.get::<_, Montant>(6)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    (d.saisie.lignes, d.totaux_lignes) = lignes.into_iter().unzip();
    Ok(Some(d))
}

fn lire_existant(conn: &Connection, id: i64) -> Resultat<Document> {
    lire(conn, id)?.ok_or_else(|| invalide("ce document n'existe plus"))
}

fn exiger_brouillon(d: &Document) -> Resultat<()> {
    if d.statut == Statut::Brouillon {
        Ok(())
    } else {
        Err(invalide(format!(
            "ce document est « {} » : ses lignes et montants ne se modifient plus",
            d.statut
        )))
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
                COALESCE((SELECT SUM(r.montant) FROM reglement r WHERE r.document_id = d.id), 0)
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
            let etat = (recoit && matches!(statut, Statut::Valide | Statut::Envoye))
                .then(|| etat_paiement(ttc, regle));
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

/// Crée un brouillon et renvoie son identifiant.
pub fn creer_brouillon(
    conn: &mut Connection,
    acteur: &Utilisateur,
    type_document: TypeDocument,
    saisie: &Saisie,
) -> Resultat<i64> {
    let tx = conn.transaction()?;
    exiger(&tx, acteur, Droit::brouillon(type_document))?;
    let s = nettoyer(type_document, saisie)?;
    let tiers = snapshot_tiers(&tx, type_document, s.tiers_id, None)?;
    tx.execute(
        "INSERT INTO document (type, date, date_creation, utilisateur_creation_id)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            type_document.code(),
            date_texte(s.date),
            maintenant(),
            acteur.id
        ],
    )?;
    let id = tx.last_insert_rowid();
    ecrire_contenu(&tx, id, type_document, &s, &tiers)?;
    journaliser_statut(
        &tx,
        id,
        "creation",
        None,
        json!({ "type": type_document.code(), "statut": Statut::Brouillon.code() }),
        acteur,
    )?;
    tx.commit()?;
    Ok(id)
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
    let s = nettoyer(d.type_document, saisie)?;
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
        return Err(invalide(format!(
            "seul un brouillon peut être supprimé ; ce document est « {} »",
            d.statut
        )));
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
    verifier_transition(d.type_document, d.statut, Statut::Valide)
        .map_err(|e| invalide(e.to_string()))?;
    let s = nettoyer(d.type_document, &d.saisie)?;
    if s.tiers_id.is_none() {
        return Err(invalide("choisir le tiers avant de valider"));
    }
    if s.lignes.is_empty() {
        return Err(invalide("le document doit compter au moins une ligne"));
    }
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
    tx.commit()?;
    Ok(numero)
}

/// Passage en Envoyé, avec la date d'envoi.
pub fn marquer_envoye(conn: &mut Connection, acteur: &Utilisateur, id: i64) -> Resultat<()> {
    let tx = conn.transaction()?;
    let d = lire_existant(&tx, id)?;
    exiger(&tx, acteur, Droit::ValiderDocument)?;
    verifier_transition(d.type_document, d.statut, Statut::Envoye)
        .map_err(|e| invalide(e.to_string()))?;
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
    verifier_transition(d.type_document, d.statut, Statut::Annule)
        .map_err(|e| invalide(e.to_string()))?;
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
        return Err(invalide(format!(
            "les champs administratifs se modifient sur un document validé ; celui-ci est « {} »",
            d.statut
        )));
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

fn libelle_statut(valeur: &Option<Value>) -> Option<String> {
    let code = valeur.as_ref()?.get("statut")?.as_str()?;
    Statut::depuis_code(code).map(|s| s.to_string())
}

fn libelle_evenement(e: &EntreeLue) -> String {
    let nouveau = libelle_statut(&e.nouvelle_valeur).unwrap_or_default();
    let numero = e
        .nouvelle_valeur
        .as_ref()
        .and_then(|v| v.get("numero"))
        .and_then(Value::as_str);
    match e.action.as_str() {
        "creation" => "Création du brouillon".to_string(),
        "validation" => match numero {
            Some(n) => format!("Validation, numéro {n}"),
            None => "Validation".to_string(),
        },
        "modification_administrative" => {
            "Modification du mode de paiement ou des observations".to_string()
        }
        _ => match libelle_statut(&e.ancienne_valeur) {
            Some(ancien) => format!("{ancien} vers {nouveau}"),
            None => e.action.clone(),
        },
    }
}

/// Historique de la fiche, tiré du journal d'audit.
pub fn historique(conn: &Connection, id: i64) -> Resultat<Vec<Evenement>> {
    Ok(audit::lister_pour(conn, "document", id)?
        .iter()
        .map(|e| Evenement {
            date_action: e.date_action.clone(),
            utilisateur: e.utilisateur.clone(),
            libelle: libelle_evenement(e),
        })
        .collect())
}

/// État de paiement d'un document : seulement pour un type qui reçoit des
/// règlements, une fois Validé ou Envoyé (section 8).
pub fn etat_paiement_de(conn: &Connection, d: &Document) -> Resultat<Option<EtatPaiement>> {
    if !d.type_document.recoit_des_reglements()
        || !matches!(d.statut, Statut::Valide | Statut::Envoye)
    {
        return Ok(None);
    }
    let regle: Montant = conn.query_row(
        "SELECT COALESCE(SUM(montant), 0) FROM reglement WHERE document_id = ?1",
        [d.id],
        |r| r.get(0),
    )?;
    Ok(Some(etat_paiement(d.totaux.ttc, regle)))
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

        let figee = pour_document(&c.conn, validee).unwrap();
        assert_eq!(figee.entreprise.nom, "Avant");
        assert_eq!(figee.textes.titre, "FACTURE");
        let apercu = pour_document(&c.conn, brouillon).unwrap();
        assert_eq!(apercu.entreprise.nom, "Après");
        assert_eq!(apercu.textes.titre, "FACTURE DEFINITIVE");
        let instantanes: i64 = c
            .conn
            .query_row("SELECT COUNT(*) FROM instantane_entreprise", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(instantanes, 1, "instantané identique réutilisé");
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
}
