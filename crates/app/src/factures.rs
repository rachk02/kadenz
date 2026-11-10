//! Écran commun des documents de vente : Devis, Proforma, Facture et Avoir, chacun
//! avec son registre et sa fiche (cahier des charges, sections 4.1, 4.2, 6, 7).
//! Le type affiché est `Etat::type_document`, choisi à l'ouverture de la page.
//!
//! La fiche garde ici l'identifiant du document et les listes affichées (clients,
//! produits, taux, modes) : un rang choisi à l'écran est converti en valeur tout
//! de suite. Les totaux affichés sont calculés par le domaine, jamais par l'écran,
//! et le stockage les recalcule de toute façon à l'enregistrement.

use std::cmp::Ordering;
use std::rc::Rc;

use chrono::NaiveDate;
use kadenz_domaine::calcul::{Pourcentage, Quantite, Taux, Totaux, calculer_totaux, total_ligne};
use kadenz_domaine::lettres::{Orthographe, en_lettres_majuscule};
use kadenz_domaine::role::Droit;
use kadenz_domaine::saisie;
use kadenz_domaine::texte::{cle_comparaison, contient};
use kadenz_stockage::comptes::Utilisateur;
use kadenz_stockage::documents::{
    self, EtatPaiement, FiltreDocuments, Lien, LigneSaisie, NatureLien, ResumeDocument, Saisie,
    Statut, TypeDocument,
};
use kadenz_stockage::produits::{self, FiltreProduits, Produit};
use kadenz_stockage::tiers::{self, FicheTiers, FiltreTiers, TypeTiers};
use kadenz_stockage::{Connexion, droits, parametres};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use crate::referentiels::{Vue, modele_lignes, retour_erreur, retour_ok, texte_erreur};
use crate::traduction::{tr, trf};
use crate::{
    Etat, EtatPartage, EvenementUi, FacturesApi, FenetrePrincipale, FicheFactureUi, LienUi,
    LigneFactureUi, Page, Retour, TextesDocumentUi, TotauxUi,
};

/// Fiche ouverte : document éventuel et correspondance rang affiché vers valeur.
pub(crate) struct EditionFacture {
    id: Option<i64>,
    type_document: TypeDocument,
    /// Tiers du document à l'ouverture : gardé dans la liste même s'il a été désactivé.
    tiers_document: Option<i64>,
    /// Le rang 0 est « aucun client choisi ».
    clients: Vec<Option<i64>>,
    produits: Vec<Produit>,
    remises: Vec<Pourcentage>,
    tvas: Vec<Pourcentage>,
    retenues: Vec<Pourcentage>,
    /// Le rang 0 est « non précisé » (chaîne vide).
    modes: Vec<String>,
    lignes: Rc<VecModel<LigneFactureUi>>,
    orthographe: Orthographe,
    /// Délai de l'échéance proposée : validité des devis pour un devis ou une
    /// proforma, délai de paiement pour une facture, aucun pour un avoir.
    delai_echeance_jours: Option<u32>,
    /// Derniers rangs de remise, de TVA et de retenue choisis, pour recalculer les
    /// totaux quand une ligne est supprimée (le bouton disparaît avec sa ligne).
    remise_rang: i32,
    tva_rang: i32,
    retenue_rang: i32,
    fiche: FicheFactureUi,
}

pub(crate) type Registre = Vue<ResumeDocument, FiltreDocuments>;

// ---------------------------------------------------------------- Textes

pub(crate) fn date_en_texte(d: NaiveDate) -> String {
    d.format("%d/%m/%Y").to_string()
}

/// Lit une date JJ/MM/AAAA (aussi avec des tirets ou des points). L'année doit
/// avoir quatre chiffres : « 10/05/20 », en cours de frappe, n'est pas une date.
pub(crate) fn date_depuis_texte(texte: &str) -> Option<NaiveDate> {
    let t = texte.trim().replace(['-', '.'], "/");
    let annee = t.rsplit('/').next()?;
    if annee.len() != 4 || !annee.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    NaiveDate::parse_from_str(&t, "%d/%m/%Y").ok()
}

/// « 2026-05-12T09:30:00 » devient « 12/05/2026 09:30 ».
fn horodatage_en_texte(iso: &str) -> String {
    chrono::NaiveDateTime::parse_from_str(iso, "%Y-%m-%dT%H:%M:%S")
        .map(|d| d.format("%d/%m/%Y %H:%M").to_string())
        .unwrap_or_else(|_| iso.to_string())
}

fn pourcentage_libelle(p: Pourcentage) -> SharedString {
    format!("{} %", saisie::pourcentage_en_texte(p)).into()
}

fn modele_textes(textes: impl IntoIterator<Item = String>) -> ModelRc<SharedString> {
    let v: Vec<SharedString> = textes.into_iter().map(SharedString::from).collect();
    ModelRc::new(VecModel::from(v))
}

// ---------------------------------------------------------------- Textes par type
//
// Chaque texte est un littéral dans `tr(...)` : le test de couverture des
// traductions les retrouve ainsi tous. Le genre suit le type (un devis, une proforma).

/// Document d'achat : son tiers est un fournisseur, ses produits ceux du sens Achat.
fn est_achat(t: TypeDocument) -> bool {
    t == TypeDocument::BonCommande
}

fn textes_type(t: TypeDocument) -> TextesDocumentUi {
    let (titre, nouveau, aide, sans_numero, envoye, annuler, validation, annulation) = match t {
        TypeDocument::Devis => (
            tr("Devis"),
            tr("Nouveau devis"),
            tr("Cliquer sur un devis pour ouvrir sa fiche."),
            tr("Aucun numéro tant que le devis n'est pas validé."),
            tr("Marquer envoyé"),
            tr("Annuler le devis"),
            tr(
                "Valider ce devis ? Il reçoit son numéro définitif ; ses lignes et ses montants ne pourront plus être modifiés.",
            ),
            tr("Annuler ce devis ? Il reste dans le registre avec son numéro, en lecture seule."),
        ),
        TypeDocument::Proforma => (
            tr("Proforma"),
            tr("Nouvelle proforma"),
            tr("Cliquer sur une proforma pour ouvrir sa fiche."),
            tr("Aucun numéro tant que la proforma n'est pas validée."),
            tr("Marquer envoyée"),
            tr("Annuler la proforma"),
            tr(
                "Valider cette proforma ? Elle reçoit son numéro définitif ; ses lignes et ses montants ne pourront plus être modifiés.",
            ),
            tr(
                "Annuler cette proforma ? Elle reste dans le registre avec son numéro, en lecture seule.",
            ),
        ),
        TypeDocument::BonCommande => (
            tr("Bons de commande"),
            tr("Nouveau bon de commande"),
            tr("Cliquer sur un bon de commande pour ouvrir sa fiche."),
            tr("Aucun numéro tant que le bon de commande n'est pas validé."),
            tr("Marquer envoyé"),
            tr("Annuler le bon de commande"),
            tr(
                "Valider ce bon de commande ? Il reçoit son numéro définitif ; ses lignes et ses montants ne pourront plus être modifiés.",
            ),
            tr(
                "Annuler ce bon de commande ? Il reste dans le registre avec son numéro, en lecture seule.",
            ),
        ),
        TypeDocument::Avoir => (
            tr("Avoirs"),
            String::new(),
            tr("Cliquer sur un avoir pour ouvrir sa fiche."),
            tr("Aucun numéro tant que l'avoir n'est pas validé."),
            tr("Marquer envoyé"),
            tr("Annuler l'avoir"),
            tr(
                "Valider cet avoir ? Il reçoit son numéro définitif ; ses lignes et ses montants ne pourront plus être modifiés.",
            ),
            tr("Annuler cet avoir ? Il reste dans le registre avec son numéro, en lecture seule."),
        ),
        _ => (
            tr("Factures"),
            tr("Nouvelle facture"),
            tr("Cliquer sur une facture pour ouvrir sa fiche."),
            tr("Aucun numéro tant que la facture n'est pas validée."),
            tr("Marquer envoyée"),
            tr("Annuler la facture"),
            tr(
                "Valider cette facture ? Elle reçoit son numéro définitif ; ses lignes et ses montants ne pourront plus être modifiés.",
            ),
            tr(
                "Annuler cette facture ? Elle reste dans le registre avec son numéro, en lecture seule.",
            ),
        ),
    };
    let echeance = if t.convertible_en_facture() {
        tr("Validité (facultative)")
    } else {
        tr("Échéance (facultative)")
    };
    let (tiers, recherche, filtrer, nouveau_tiers, creer, note) = if est_achat(t) {
        (
            tr("Fournisseur"),
            tr("Rechercher : numéro, fournisseur, libellé"),
            tr("Filtrer les fournisseurs"),
            tr("Nouveau fournisseur"),
            tr("Créer et choisir ce fournisseur"),
            tr(
                "Le fournisseur est enregistré tout de suite dans les tiers ; sa fiche se complète ensuite depuis l'écran Tiers.",
            ),
        )
    } else {
        (
            tr("Client"),
            tr("Rechercher : numéro, client, libellé"),
            tr("Filtrer les clients"),
            tr("Nouveau client"),
            tr("Créer et choisir ce client"),
            tr(
                "Le client est enregistré tout de suite dans les tiers ; sa fiche se complète ensuite depuis l'écran Tiers.",
            ),
        )
    };
    TextesDocumentUi {
        titre: titre.into(),
        nouveau: nouveau.into(),
        aide_registre: aide.into(),
        sans_numero: sans_numero.into(),
        libelle_echeance: echeance.into(),
        marquer_envoye: envoye.into(),
        annuler: annuler.into(),
        confirmer_validation: validation.into(),
        confirmer_annulation: annulation.into(),
        tiers: tiers.into(),
        recherche: recherche.into(),
        filtrer_tiers: filtrer.into(),
        nouveau_tiers: nouveau_tiers.into(),
        creer_tiers: creer.into(),
        note_tiers: note.into(),
    }
}

/// Bilan du registre : « Aucun devis. », « 1 devis. », « 3 devis. »
fn resume_registre(t: TypeDocument, n: usize) -> String {
    match (t, n) {
        (TypeDocument::Devis, 0) => tr("Aucun devis."),
        (TypeDocument::Devis, 1) => tr("1 devis."),
        (TypeDocument::Devis, n) => trf("{} devis.", &[&n]),
        (TypeDocument::Proforma, 0) => tr("Aucune proforma."),
        (TypeDocument::Proforma, 1) => tr("1 proforma."),
        (TypeDocument::Proforma, n) => trf("{} proformas.", &[&n]),
        (TypeDocument::BonCommande, 0) => tr("Aucun bon de commande."),
        (TypeDocument::BonCommande, 1) => tr("1 bon de commande."),
        (TypeDocument::BonCommande, n) => trf("{} bons de commande.", &[&n]),
        (TypeDocument::Avoir, 0) => tr("Aucun avoir."),
        (TypeDocument::Avoir, 1) => tr("1 avoir."),
        (TypeDocument::Avoir, n) => trf("{} avoirs.", &[&n]),
        (_, 0) => tr("Aucune facture."),
        (_, 1) => tr("1 facture."),
        (_, n) => trf("{} factures.", &[&n]),
    }
}

/// Titre de la fiche : numéro, brouillon ou nouveau document.
fn titre_fiche(t: TypeDocument, numero: Option<&str>, existe: bool) -> String {
    match (t, numero, existe) {
        (TypeDocument::Devis, Some(n), _) => trf("Devis {}", &[&n]),
        (TypeDocument::Devis, None, true) => tr("Devis (brouillon)"),
        (TypeDocument::Devis, None, false) => tr("Nouveau devis"),
        (TypeDocument::Proforma, Some(n), _) => trf("Proforma {}", &[&n]),
        (TypeDocument::Proforma, None, true) => tr("Proforma (brouillon)"),
        (TypeDocument::Proforma, None, false) => tr("Nouvelle proforma"),
        (TypeDocument::BonCommande, Some(n), _) => trf("Bon de commande {}", &[&n]),
        (TypeDocument::BonCommande, None, true) => tr("Bon de commande (brouillon)"),
        (TypeDocument::BonCommande, None, false) => tr("Nouveau bon de commande"),
        (TypeDocument::Avoir, Some(n), _) => trf("Avoir {}", &[&n]),
        (TypeDocument::Avoir, None, _) => tr("Avoir (brouillon)"),
        (_, Some(n), _) => trf("Facture {}", &[&n]),
        (_, None, true) => tr("Facture (brouillon)"),
        (_, None, false) => tr("Nouvelle facture"),
    }
}

/// Explication affichée sous les actions d'une fiche non modifiable.
fn note_fiche(t: TypeDocument, statut: Statut, modifiable: bool, admin_modifiable: bool) -> String {
    use TypeDocument::{Avoir, BonCommande, Devis, Proforma};
    match (t, statut) {
        (Devis, Statut::Annule) => tr("Devis annulé : lecture seule, numéro conservé."),
        (Proforma, Statut::Annule) => tr("Proforma annulée : lecture seule, numéro conservé."),
        (Avoir, Statut::Annule) => tr("Avoir annulé : lecture seule, numéro conservé."),
        (BonCommande, Statut::Annule) => {
            tr("Bon de commande annulé : lecture seule, numéro conservé.")
        }
        (_, Statut::Annule) => tr("Facture annulée : lecture seule, numéro conservé."),
        _ if modifiable || admin_modifiable => String::new(),
        (Devis, Statut::Converti) => tr("Devis converti en facture : lecture seule."),
        (Proforma, Statut::Converti) => tr("Proforma convertie en facture : lecture seule."),
        (BonCommande, Statut::ReceptionComplete) => {
            tr("Bon de commande entièrement reçu : lecture seule.")
        }
        (Devis, Statut::Brouillon) => {
            tr("Lecture seule : votre rôle ne permet pas de modifier un devis.")
        }
        (Proforma, Statut::Brouillon) => {
            tr("Lecture seule : votre rôle ne permet pas de modifier une proforma.")
        }
        (Avoir, Statut::Brouillon) => {
            tr("Lecture seule : votre rôle ne permet pas de modifier un avoir.")
        }
        (BonCommande, Statut::Brouillon) => {
            tr("Lecture seule : votre rôle ne permet pas de modifier un bon de commande.")
        }
        (_, Statut::Brouillon) => {
            tr("Lecture seule : votre rôle ne permet pas de modifier une facture.")
        }
        (Devis, _) => tr("Devis validé : lignes et montants figés."),
        (Proforma, _) => tr("Proforma validée : lignes et montants figés."),
        (Avoir, _) => tr("Avoir validé : lignes et montants figés."),
        (BonCommande, _) => tr("Bon de commande validé : lignes et montants figés."),
        _ => tr("Facture validée : lignes et montants figés."),
    }
}

/// Message après une validation réussie.
fn message_validation(t: TypeDocument, numero: &str) -> String {
    match t {
        TypeDocument::Devis => trf("devis validé sous le numéro {}", &[&numero]),
        TypeDocument::Proforma => trf("proforma validée sous le numéro {}", &[&numero]),
        TypeDocument::BonCommande => trf("bon de commande validé sous le numéro {}", &[&numero]),
        TypeDocument::Avoir => trf("avoir validé sous le numéro {}", &[&numero]),
        _ => trf("facture validée sous le numéro {}", &[&numero]),
    }
}

fn message_envoi(t: TypeDocument) -> String {
    match t {
        TypeDocument::Devis => tr("devis marqué envoyé"),
        TypeDocument::Proforma => tr("proforma marquée envoyée"),
        TypeDocument::BonCommande => tr("bon de commande marqué envoyé"),
        TypeDocument::Avoir => tr("avoir marqué envoyé"),
        _ => tr("facture marquée envoyée"),
    }
}

fn message_annulation(t: TypeDocument) -> String {
    match t {
        TypeDocument::Devis => tr("devis annulé"),
        TypeDocument::Proforma => tr("proforma annulée"),
        TypeDocument::BonCommande => tr("bon de commande annulé"),
        TypeDocument::Avoir => tr("avoir annulé"),
        _ => tr("facture annulée"),
    }
}

/// Statuts proposés par le filtre du registre ; `None` est « tous les statuts ».
fn statuts_filtre(t: TypeDocument) -> Vec<Option<Statut>> {
    let mut statuts = vec![
        None,
        Some(Statut::Brouillon),
        Some(Statut::Valide),
        Some(Statut::Envoye),
    ];
    if t.convertible_en_facture() {
        statuts.push(Some(Statut::Converti));
    }
    if t == TypeDocument::BonCommande {
        statuts.push(Some(Statut::ReceptionPartielle));
        statuts.push(Some(Statut::ReceptionComplete));
    }
    statuts.push(Some(Statut::Annule));
    statuts
}

/// Page du menu qui affiche un type de document.
fn page_du_type(t: TypeDocument) -> Page {
    match t {
        TypeDocument::Devis => Page::Devis,
        TypeDocument::Proforma => Page::Proforma,
        TypeDocument::BonCommande => Page::BonsCommande,
        TypeDocument::Livraison => Page::Livraisons,
        TypeDocument::Avoir => Page::Avoirs,
        TypeDocument::Facture => Page::Factures,
    }
}

/// Texte d'un document lié : sa nature, puis le document (numéro et statut).
fn texte_lien(l: &Lien, origine: bool) -> String {
    // La facture d'origine d'un avoir est nommée par son seul numéro : son libellé dit déjà
    // qu'il s'agit d'une facture.
    let titre = match (l.nature, origine, &l.numero) {
        (NatureLien::Avoir, true, Some(n)) => n.clone(),
        _ => titre_fiche(l.type_document, l.numero.as_deref(), true),
    };
    let document = if l.numero.is_some() {
        trf("{} ({})", &[&titre, &tr(&l.statut.to_string())])
    } else {
        titre
    };
    match (l.nature, origine) {
        (NatureLien::Conversion, true) => trf("Origine (conversion) : {}", &[&document]),
        (NatureLien::Copie, true) => trf("Origine (copie) : {}", &[&document]),
        (NatureLien::Correction, true) => trf("Origine (correction) : {}", &[&document]),
        (NatureLien::Avoir, true) => trf("Facture d'origine : {}", &[&document]),
        (NatureLien::Conversion, false) => trf("Conversion : {}", &[&document]),
        (NatureLien::Copie, false) => trf("Copie : {}", &[&document]),
        (NatureLien::Correction, false) => trf("Correction : {}", &[&document]),
        // « Avoir A-... » se suffit à lui-même.
        (NatureLien::Avoir, false) => document,
    }
}

/// Type d'après son rang dans `TypeDocument::TOUS` ; la facture par défaut.
fn type_au_rang(rang: i32) -> TypeDocument {
    usize::try_from(rang)
        .ok()
        .and_then(|r| TypeDocument::TOUS.get(r).copied())
        .unwrap_or(TypeDocument::Facture)
}

// ---------------------------------------------------------------- Registre

fn cellules(d: &ResumeDocument) -> Vec<String> {
    vec![
        d.numero.clone().unwrap_or_else(|| tr("(brouillon)")),
        date_en_texte(d.date),
        d.tiers_nom.clone(),
        d.libelle.clone(),
        saisie::montant_en_texte(d.montant_ttc),
        tr(&d.statut.to_string()),
        d.etat_paiement
            .map(|e| tr(&e.to_string()))
            .unwrap_or_default(),
        d.mode_paiement.clone(),
        d.date_echeance.map(date_en_texte).unwrap_or_default(),
    ]
}

fn trier(lignes: &mut [ResumeDocument], colonne: i32, croissant: bool) {
    let comparer = |a: &ResumeDocument, b: &ResumeDocument| -> Ordering {
        match colonne {
            0 => a.numero.cmp(&b.numero),
            1 => a.date.cmp(&b.date),
            2 => cle_comparaison(&a.tiers_nom).cmp(&cle_comparaison(&b.tiers_nom)),
            3 => cle_comparaison(&a.libelle).cmp(&cle_comparaison(&b.libelle)),
            4 => a.montant_ttc.cmp(&b.montant_ttc),
            5 => a.statut.to_string().cmp(&b.statut.to_string()),
            6 => a
                .etat_paiement
                .map(|e| e.to_string())
                .cmp(&b.etat_paiement.map(|e| e.to_string())),
            7 => cle_comparaison(&a.mode_paiement).cmp(&cle_comparaison(&b.mode_paiement)),
            _ => a.date_echeance.cmp(&b.date_echeance),
        }
    };
    lignes.sort_by(|a, b| {
        let o = comparer(a, b);
        if croissant { o } else { o.reverse() }
    });
}

fn afficher_registre(f: &FenetrePrincipale, etat: &mut Etat) {
    let api = f.global::<FacturesApi>();
    let t = etat.type_document;
    match documents::lister(&etat.conn, t, &etat.factures.filtre) {
        Ok(mut lignes) => {
            if let Some((colonne, croissant)) = etat.factures.tri {
                trier(&mut lignes, colonne, croissant);
            }
            api.set_registre(modele_lignes(lignes.iter().map(cellules).collect()));
            api.set_resume(resume_registre(t, lignes.len()).into());
            etat.factures.lignes = lignes;
        }
        Err(e) => api.set_resume(crate::phrase(e)),
    }
    api.set_ligne_courante(-1);
}

// ---------------------------------------------------------------- Fiche

fn possede(etat: &Etat, droit: Droit) -> bool {
    match &etat.utilisateur {
        Some(u) => droits::possede(&etat.conn, u, droit).unwrap_or(false),
        None => false,
    }
}

/// Coordonnées affichées sous le client.
fn detail_tiers(adresse: &str, ifu: &str, rccm: &str) -> String {
    let mut morceaux = Vec::new();
    if !adresse.is_empty() {
        morceaux.push(adresse.to_string());
    }
    if !ifu.is_empty() {
        morceaux.push(format!("IFU {ifu}"));
    }
    if !rccm.is_empty() {
        morceaux.push(format!("RCCM {rccm}"));
    }
    morceaux.join(", ")
}

/// Liste des valeurs proposées, complétée par la valeur du document si elle a
/// été retirée des paramètres depuis.
fn avec_valeur<T: PartialEq + Clone>(mut liste: Vec<T>, valeur: &T) -> (Vec<T>, i32) {
    let rang = match liste.iter().position(|v| v == valeur) {
        Some(r) => r,
        None => {
            liste.push(valeur.clone());
            liste.len() - 1
        }
    };
    (liste, rang as i32)
}

fn ligne_ui(l: &LigneSaisie) -> LigneFactureUi {
    LigneFactureUi {
        produit_id: l.produit_id.unwrap_or(0) as i32,
        reference: l.reference.as_str().into(),
        designation: l.designation.as_str().into(),
        prix: saisie::montant_en_texte(l.prix).into(),
        quantite: saisie::quantite_en_texte(l.quantite).into(),
        total: saisie::montant_en_texte(total_ligne(l.prix, l.quantite)).into(),
        recue: saisie::quantite_en_texte(l.quantite_livree).into(),
    }
}

fn totaux_ui(t: &Totaux, orthographe: Orthographe) -> TotauxUi {
    let m = |v| SharedString::from(saisie::montant_en_texte(v));
    TotauxUi {
        ht: m(t.ht),
        remise: m(t.remise),
        base_tva: m(t.base_tva),
        tva: m(t.tva),
        retenue: m(t.retenue),
        ttc: m(t.ttc),
        // Le montant en lettres reste en français : il est repris tel quel sur le document.
        lettres: trf(
            "En lettres : {}",
            &[&en_lettres_majuscule(t.ttc, orthographe)],
        )
        .into(),
    }
}

/// Tiers proposés : « aucun client choisi » (ou fournisseur pour un achat), puis
/// les tiers actifs du bon type filtrés par le texte, plus le tiers choisi (et
/// celui du document) même s'il ne correspond pas au filtre ou a été désactivé.
fn liste_clients(
    conn: &Connexion,
    achat: bool,
    texte: &str,
    garder: &[Option<i64>],
) -> (Vec<Option<i64>>, Vec<String>) {
    let tous = tiers::lister(
        conn,
        &FiltreTiers {
            inclure_inactifs: true,
            ..Default::default()
        },
    )
    .unwrap_or_default();
    let mut ids = vec![None];
    let mut noms = vec![if achat {
        tr("Aucun fournisseur choisi")
    } else {
        tr("Aucun client choisi")
    }];
    for t in tous {
        let garde = garder.contains(&Some(t.id));
        let du_bon_type = if achat {
            t.fiche.type_tiers.est_fournisseur()
        } else {
            t.fiche.type_tiers.est_client()
        };
        let propose = t.actif && du_bon_type && contient(&t.fiche.nom, texte);
        if garde || propose {
            ids.push(Some(t.id));
            noms.push(if t.actif {
                t.fiche.nom
            } else {
                trf("{} (désactivé)", &[&t.fiche.nom])
            });
        }
    }
    (ids, noms)
}

/// Le produit se propose-t-il pour ce document ? Sens Achat pour un achat, Vente sinon.
fn dans_le_sens(p: &Produit, achat: bool) -> bool {
    if achat {
        p.fiche.sens.en_achat()
    } else {
        p.fiche.sens.en_vente()
    }
}

fn liste_produits(conn: &Connexion, achat: bool, texte: &str) -> Vec<Produit> {
    produits::lister(conn, &FiltreProduits::default())
        .unwrap_or_default()
        .into_iter()
        .filter(|p| {
            dans_le_sens(p, achat)
                && (contient(&p.fiche.reference, texte) || contient(&p.fiche.designation, texte))
        })
        .collect()
}

fn afficher_produits(api: &FacturesApi, liste: &[Produit]) {
    api.set_produits(modele_textes(liste.iter().map(|p| {
        format!(
            "{} - {} ({} FCFA)",
            p.fiche.reference,
            p.fiche.designation,
            saisie::montant_en_texte(p.fiche.prix)
        )
    })));
}

/// Prépare la fiche d'un document existant, ou d'un nouveau document du type affiché.
fn ouvrir_fiche(f: &FenetrePrincipale, etat: &mut Etat, id: Option<i64>) -> Result<(), String> {
    let api = f.global::<FacturesApi>();
    let p = parametres::lire(&etat.conn).map_err(texte_erreur)?;
    let document = match id {
        Some(id) => Some(
            documents::lire(&etat.conn, id)
                .map_err(texte_erreur)?
                .ok_or("ce document n'existe plus")?,
        ),
        None => None,
    };
    let t = document
        .as_ref()
        .map_or(etat.type_document, |d| d.type_document);
    let delai_echeance = match t {
        TypeDocument::Devis | TypeDocument::Proforma => p.validite_devis_jours,
        TypeDocument::Facture => p.delai_paiement_jours,
        _ => None,
    };
    let aujourd_hui = chrono::Local::now().date_naive();
    let saisie_doc = document.as_ref().map_or_else(
        || Saisie {
            date_echeance: documents::echeance_par_defaut(aujourd_hui, delai_echeance),
            taux: Taux {
                remise: p.taux_remise.first().copied().unwrap_or(0),
                tva: p.taux_tva.first().copied().unwrap_or(0),
                retenue: if t.retenue_applicable() {
                    p.taux_retenue.first().copied().unwrap_or(0)
                } else {
                    0
                },
            },
            ..Saisie::vide(aujourd_hui)
        },
        |d| d.saisie.clone(),
    );
    let statut = document.as_ref().map_or(Statut::Brouillon, |d| d.statut);
    let brouillon = statut == Statut::Brouillon;
    let droit_brouillon = possede(etat, Droit::brouillon(t));
    let modifiable = brouillon && droit_brouillon;
    // Le client d'un avoir est celui de la facture d'origine : jamais choisi à l'écran.
    let client_modifiable = modifiable && t != TypeDocument::Avoir;

    // Clients : liste modifiable, ou seulement le nom figé du document.
    let (clients, noms, client_index, detail) = if client_modifiable {
        let (ids, noms) = liste_clients(&etat.conn, est_achat(t), "", &[saisie_doc.tiers_id]);
        let rang = ids
            .iter()
            .position(|i| *i == saisie_doc.tiers_id)
            .unwrap_or(0) as i32;
        let detail = document
            .as_ref()
            .map(|d| detail_tiers(&d.tiers.adresse, &d.tiers.ifu, &d.tiers.rccm))
            .unwrap_or_default();
        (ids, noms, rang, detail)
    } else {
        let d = document.as_ref();
        let nom = d.map(|d| d.tiers.nom.clone()).unwrap_or_default();
        let detail = d
            .map(|d| detail_tiers(&d.tiers.adresse, &d.tiers.ifu, &d.tiers.rccm))
            .unwrap_or_default();
        (vec![saisie_doc.tiers_id], vec![nom], 0, detail)
    };
    api.set_clients(modele_textes(noms));

    let (remises, remise_index) = avec_valeur(p.taux_remise.clone(), &saisie_doc.taux.remise);
    let (tvas, tva_index) = avec_valeur(p.taux_tva.clone(), &saisie_doc.taux.tva);
    let (retenues, retenue_index) = avec_valeur(p.taux_retenue.clone(), &saisie_doc.taux.retenue);
    let mut modes_base = vec![String::new()];
    modes_base.extend(p.modes_paiement.iter().cloned());
    let (modes, mode_index) = avec_valeur(modes_base, &saisie_doc.mode_paiement);
    api.set_taux_remise(ModelRc::new(VecModel::from(
        remises
            .iter()
            .map(|&r| pourcentage_libelle(r))
            .collect::<Vec<_>>(),
    )));
    api.set_taux_tva(ModelRc::new(VecModel::from(
        tvas.iter()
            .map(|&t| pourcentage_libelle(t))
            .collect::<Vec<_>>(),
    )));
    api.set_taux_retenue(ModelRc::new(VecModel::from(
        retenues
            .iter()
            .map(|&r| pourcentage_libelle(r))
            .collect::<Vec<_>>(),
    )));
    api.set_modes_paiement(modele_textes(modes.iter().map(|m| {
        if m.is_empty() {
            tr("Non précisé")
        } else {
            m.clone()
        }
    })));

    let produits = if modifiable {
        liste_produits(&etat.conn, est_achat(t), "")
    } else {
        Vec::new()
    };
    afficher_produits(&api, &produits);

    let nouvelles: Vec<LigneFactureUi> = saisie_doc.lignes.iter().map(ligne_ui).collect();
    // Fiche déjà ouverte : son modèle est mis à jour rangée par rangée plutôt que
    // remplacé. Les champs des lignes restent alors en place, avec le focus : la
    // saisie et les raccourcis continuent après un enregistrement.
    let lignes = match etat.facture.as_ref() {
        Some(ed) => {
            let modele = ed.lignes.clone();
            remplacer_lignes(&modele, nouvelles);
            modele
        }
        None => {
            let modele = Rc::new(VecModel::from(nouvelles));
            api.set_lignes(ModelRc::from(modele.clone()));
            modele
        }
    };
    let totaux = document.as_ref().map_or_else(
        || calculer_totaux([], saisie_doc.taux, t.retenue_applicable()),
        |d| d.totaux,
    );
    api.set_totaux(totaux_ui(&totaux, p.orthographe));

    let historique = match id {
        Some(id) => documents::historique(&etat.conn, id).unwrap_or_default(),
        None => Vec::new(),
    };
    api.set_historique(ModelRc::new(VecModel::from(
        historique
            .into_iter()
            .map(|e| EvenementUi {
                date: horodatage_en_texte(&e.date_action).into(),
                utilisateur: e.utilisateur.into(),
                // Libellé fixe traduit, puis ses valeurs : un numéro passe tel quel
                // (aucun msgid), un statut est traduit.
                libelle: {
                    let valeurs: Vec<String> = e.valeurs.iter().map(|v| tr(v)).collect();
                    let arguments: Vec<&dyn std::fmt::Display> = valeurs
                        .iter()
                        .map(|v| v as &dyn std::fmt::Display)
                        .collect();
                    trf(&e.modele, &arguments)
                }
                .into(),
            })
            .collect::<Vec<_>>(),
    )));

    let paiement = match &document {
        Some(d) => documents::etat_paiement_de(&etat.conn, d)
            .ok()
            .flatten()
            .map(|e: EtatPaiement| tr(&e.to_string()))
            .unwrap_or_default(),
        None => String::new(),
    };
    let titre = titre_fiche(
        t,
        document.as_ref().and_then(|d| d.numero.as_deref()),
        document.is_some(),
    );
    let valide_ou_envoye = matches!(statut, Statut::Valide | Statut::Envoye);
    // Mode de paiement et observations : modifiables tant que le document est en
    // vigueur (converti, en réception ou livré compris), comme le permet le stockage.
    let admin_modifiable = statut.en_vigueur() && droit_brouillon;
    // Réception d'un bon de commande (section 7.7) : quantités reçues affichées dès
    // l'envoi, saisies avec le droit « Valider un document » tant qu'elle n'est pas complète.
    let avec_reception = t == TypeDocument::BonCommande
        && matches!(
            statut,
            Statut::Envoye | Statut::ReceptionPartielle | Statut::ReceptionComplete
        );
    let reception_modifiable = avec_reception
        && statut != Statut::ReceptionComplete
        && possede(etat, Droit::ValiderDocument);

    // Documents qui peuvent naître de celui-ci (sections 2.11 et 7.2).
    let en_preparation = document.as_ref().is_some_and(|d| {
        d.derives.iter().any(|l| {
            l.nature == NatureLien::Conversion
                && l.type_document == TypeDocument::Facture
                && l.statut == Statut::Brouillon
        })
    });
    let convertible = t.convertible_en_facture() && valide_ou_envoye && !en_preparation;
    let peut_convertir = convertible && possede(etat, Droit::brouillon(TypeDocument::Facture));
    let peut_corriger =
        convertible && droit_brouillon && possede(etat, Droit::AnnulerDocumentValide);
    let peut_dupliquer = document.is_some() && t != TypeDocument::Avoir && droit_brouillon;
    let peut_creer_avoir = t == TypeDocument::Facture
        && valide_ou_envoye
        && possede(etat, Droit::brouillon(TypeDocument::Avoir));
    let avoir_possible = document
        .as_ref()
        .and_then(|d| d.reste_a_crediter)
        .is_some_and(|r| r > 0);
    // Le montant dû n'est affiché que s'il diffère du TTC, c'est-à-dire après un avoir.
    let montant_du = document
        .as_ref()
        .and_then(|d| d.montant_du.filter(|m| *m != d.totaux.ttc))
        .map(saisie::montant_en_texte)
        .unwrap_or_default();
    let liens: Vec<LienUi> = document
        .iter()
        .flat_map(|d| {
            d.source
                .iter()
                .map(|l| (l, true))
                .chain(d.derives.iter().map(|l| (l, false)))
        })
        .map(|(l, origine)| LienUi {
            id: l.id as i32,
            texte: texte_lien(l, origine).into(),
        })
        .collect();
    api.set_liens(ModelRc::new(VecModel::from(liens)));
    let fiche = FicheFactureUi {
        existe: document.is_some(),
        titre: titre.into(),
        statut: tr(&statut.to_string()).into(),
        paiement: paiement.into(),
        brouillon,
        annule: statut == Statut::Annule,
        date: date_en_texte(saisie_doc.date).into(),
        echeance: saisie_doc
            .date_echeance
            .map(date_en_texte)
            .unwrap_or_default()
            .into(),
        libelle: saisie_doc.libelle.as_str().into(),
        observations: saisie_doc.observations.as_str().into(),
        client_index,
        tiers_detail: detail.into(),
        remise_index,
        tva_index,
        retenue_index,
        mode_index,
        modifiable,
        client_modifiable,
        // L'avoir garde la remise et la TVA de sa facture, comme son client.
        taux_modifiables: client_modifiable,
        admin_modifiable,
        peut_valider: modifiable && possede(etat, Droit::ValiderDocument),
        peut_envoyer: statut == Statut::Valide && possede(etat, Droit::ValiderDocument),
        peut_annuler: valide_ou_envoye && possede(etat, Droit::AnnulerDocumentValide),
        peut_supprimer: document.is_some() && modifiable,
        avec_reception,
        reception_modifiable,
        peut_convertir,
        peut_dupliquer,
        peut_corriger,
        peut_creer_avoir,
        avoir_possible,
        montant_du: montant_du.into(),
        en_preparation,
        note: note_fiche(t, statut, modifiable, admin_modifiable).into(),
    };
    etat.facture = Some(EditionFacture {
        id,
        type_document: t,
        tiers_document: saisie_doc.tiers_id,
        clients,
        produits,
        remises,
        tvas,
        retenues,
        modes,
        lignes,
        orthographe: p.orthographe,
        delai_echeance_jours: delai_echeance,
        remise_rang: remise_index,
        tva_rang: tva_index,
        retenue_rang: retenue_index,
        fiche,
    });
    api.set_version_fiche(api.get_version_fiche().wrapping_add(1));
    api.set_fiche_ouverte(true);
    Ok(())
}

/// Remplace le contenu d'un modèle de lignes en ne touchant que les rangées qui changent.
fn remplacer_lignes(modele: &VecModel<LigneFactureUi>, nouvelles: Vec<LigneFactureUi>) {
    while modele.row_count() > nouvelles.len() {
        modele.remove(modele.row_count() - 1);
    }
    for (rang, ligne) in nouvelles.into_iter().enumerate() {
        if rang < modele.row_count() {
            if modele.row_data(rang).as_ref() != Some(&ligne) {
                modele.set_row_data(rang, ligne);
            }
        } else {
            modele.push(ligne);
        }
    }
}

/// Valeur choisie au rang donné, ou `None` hors liste.
fn au_rang<T: Clone>(liste: &[T], rang: i32) -> Option<T> {
    usize::try_from(rang)
        .ok()
        .and_then(|r| liste.get(r).cloned())
}

/// Lignes de l'écran, contrôlées.
fn lignes_saisies(e: &EditionFacture) -> Result<Vec<LigneSaisie>, String> {
    e.lignes
        .iter()
        .enumerate()
        .map(|(rang, l)| {
            let n = rang + 1;
            let prix = saisie::montant_depuis_texte(&l.prix).ok_or_else(|| {
                trf(
                    "ligne {} : prix illisible, saisir un montant entier en FCFA",
                    &[&n],
                )
            })?;
            let quantite = saisie::quantite_depuis_texte(&l.quantite).ok_or_else(|| {
                trf(
                    "ligne {} : quantité illisible, saisir un nombre positif (3 décimales au plus)",
                    &[&n],
                )
            })?;
            Ok(LigneSaisie {
                produit_id: (l.produit_id > 0).then_some(i64::from(l.produit_id)),
                reference: l.reference.to_string(),
                designation: l.designation.to_string(),
                prix,
                quantite,
                observations: String::new(),
                ..Default::default()
            })
        })
        .collect()
}

fn saisie_depuis_ui(e: &EditionFacture, fiche: &FicheFactureUi) -> Result<Saisie, String> {
    let date = date_depuis_texte(&fiche.date)
        .ok_or("date illisible : saisir la date au format JJ/MM/AAAA")?;
    let date_echeance = if fiche.echeance.trim().is_empty() {
        None
    } else {
        Some(date_depuis_texte(&fiche.echeance).ok_or(
            "échéance illisible : saisir la date au format JJ/MM/AAAA, ou la laisser vide",
        )?)
    };
    Ok(Saisie {
        date,
        tiers_id: au_rang(&e.clients, fiche.client_index).flatten(),
        libelle: fiche.libelle.to_string(),
        taux: Taux {
            remise: au_rang(&e.remises, fiche.remise_index).unwrap_or(0),
            tva: au_rang(&e.tvas, fiche.tva_index).unwrap_or(0),
            retenue: au_rang(&e.retenues, fiche.retenue_index).unwrap_or(0),
        },
        mode_paiement: au_rang(&e.modes, fiche.mode_index).unwrap_or_default(),
        observations: fiche.observations.to_string(),
        date_echeance,
        lignes: lignes_saisies(e)?,
    })
}

/// Enregistre le brouillon affiché ; renvoie son identifiant.
fn enregistrer_brouillon(
    etat: &mut Etat,
    acteur: &Utilisateur,
    fiche: &FicheFactureUi,
) -> Result<i64, String> {
    let e = etat.facture.as_ref().ok_or("aucune fiche ouverte")?;
    let s = saisie_depuis_ui(e, fiche)?;
    let id = e.id;
    match id {
        Some(id) => documents::modifier_brouillon(&mut etat.conn, acteur, id, &s)
            .map(|()| id)
            .map_err(texte_erreur),
        None => documents::creer_brouillon(&mut etat.conn, acteur, etat.type_document, &s)
            .map_err(texte_erreur),
    }
}

/// Action sur le document ouvert, puis rechargement de la fiche et du registre.
fn agir(
    f: &FenetrePrincipale,
    etat: &EtatPartage,
    action: impl FnOnce(&mut Etat, &Utilisateur, i64) -> Result<String, String>,
) -> Retour {
    let mut etat = etat.borrow_mut();
    let Some(acteur) = etat.utilisateur.clone() else {
        return retour_erreur("aucune session ouverte");
    };
    let Some(id) = etat.facture.as_ref().and_then(|e| e.id) else {
        return retour_erreur("enregistrer d'abord le document");
    };
    match action(&mut etat, &acteur, id) {
        Ok(message) => {
            if let Err(e) = ouvrir_fiche(f, &mut etat, Some(id)) {
                return retour_erreur(e);
            }
            afficher_registre(f, &mut etat);
            retour_ok(message)
        }
        Err(message) => retour_erreur(message),
    }
}

/// Quantités reçues saisies sur les lignes, contrôlées ; une case vide vaut zéro.
fn quantites_recues(e: Option<&EditionFacture>) -> Result<Vec<Quantite>, String> {
    let Some(e) = e else {
        return Err(tr("aucune fiche ouverte"));
    };
    e.lignes
        .iter()
        .enumerate()
        .map(|(rang, l)| {
            if l.recue.trim().is_empty() {
                return Ok(0);
            }
            saisie::quantite_depuis_texte(&l.recue).ok_or_else(|| {
                trf(
                    "ligne {} : quantité reçue illisible, saisir un nombre positif (3 décimales au plus)",
                    &[&(rang + 1)],
                )
            })
        })
        .collect()
}

/// Ouvre la fiche d'un document : dans la page affichée s'il est de son type,
/// sinon dans la page de son type, qui l'ouvre dès son affichage (`on_ouvrir`).
fn ouvrir_document(f: &FenetrePrincipale, etat: &EtatPartage, id: i64, retour: Retour) -> Retour {
    let mut guard = etat.borrow_mut();
    let t = match documents::lire(&guard.conn, id) {
        Ok(Some(d)) => d.type_document,
        Ok(None) => return retour_erreur("ce document n'existe plus"),
        Err(erreur) => return retour_erreur(erreur),
    };
    if t == guard.type_document {
        if let Err(message) = ouvrir_fiche(f, &mut guard, Some(id)) {
            return retour_erreur(message);
        }
        afficher_registre(f, &mut guard);
        return retour;
    }
    guard.document_a_ouvrir = Some(id);
    // L'état est libéré avant le changement de page, qui rappellera `on_ouvrir`.
    drop(guard);
    f.global::<FacturesApi>()
        .set_message_ouverture(retour.clone());
    f.set_page(page_du_type(t));
    retour
}

/// Crée un document depuis celui de la fiche, puis ouvre sa fiche.
fn creer_puis_ouvrir(
    f: &FenetrePrincipale,
    etat: &EtatPartage,
    action: impl FnOnce(&mut Etat, &Utilisateur, i64) -> Result<i64, String>,
    message: String,
) -> Retour {
    let nouveau = {
        let mut etat = etat.borrow_mut();
        let Some(acteur) = etat.utilisateur.clone() else {
            return retour_erreur("aucune session ouverte");
        };
        let Some(id) = etat.facture.as_ref().and_then(|e| e.id) else {
            return retour_erreur("enregistrer d'abord le document");
        };
        match action(&mut etat, &acteur, id) {
            Ok(nouveau) => nouveau,
            Err(message) => return retour_erreur(message),
        }
    };
    ouvrir_document(f, etat, nouveau, retour_ok(message))
}

/// Recalcule le total de chaque ligne lisible et les totaux du document.
fn recalculer(api: &FacturesApi, e: &EditionFacture, remise: i32, tva: i32, retenue: i32) {
    let mut valeurs = Vec::new();
    for rang in 0..e.lignes.row_count() {
        let Some(mut l) = e.lignes.row_data(rang) else {
            continue;
        };
        let lue =
            saisie::montant_depuis_texte(&l.prix).zip(saisie::quantite_depuis_texte(&l.quantite));
        let total: SharedString = match lue {
            Some((prix, quantite)) => {
                valeurs.push((prix, quantite));
                saisie::montant_en_texte(total_ligne(prix, quantite)).into()
            }
            None => SharedString::new(),
        };
        if l.total != total {
            l.total = total;
            e.lignes.set_row_data(rang, l);
        }
    }
    let taux = Taux {
        remise: au_rang(&e.remises, remise).unwrap_or(0),
        tva: au_rang(&e.tvas, tva).unwrap_or(0),
        retenue: au_rang(&e.retenues, retenue).unwrap_or(0),
    };
    api.set_totaux(totaux_ui(
        &calculer_totaux(valeurs, taux, e.type_document.retenue_applicable()),
        e.orthographe,
    ));
}

/// Échéance à afficher après un changement de date (voir `echeance-apres-date`).
fn echeance_apres_date(
    delai: Option<u32>,
    date_precedente: &str,
    date: &str,
    echeance: &str,
) -> Option<String> {
    let ancienne_auto = date_depuis_texte(date_precedente)
        .and_then(|d| documents::echeance_par_defaut(d, delai))
        .map(date_en_texte);
    let automatique =
        echeance.trim().is_empty() || ancienne_auto.as_deref() == Some(echeance.trim());
    if !automatique || delai.is_none() {
        return None;
    }
    // Date en cours de frappe : l'échéance automatique est gardée telle quelle.
    let nouvelle = date_depuis_texte(date)?;
    documents::echeance_par_defaut(nouvelle, delai).map(date_en_texte)
}

pub(crate) fn brancher(fenetre: &FenetrePrincipale, etat: &EtatPartage) {
    let api = fenetre.global::<FacturesApi>();

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_ouvrir(move |rang| {
        let Some(f) = f.upgrade() else { return };
        let mut etat = e.borrow_mut();
        let t = type_au_rang(rang);
        etat.type_document = t;
        etat.factures = Registre::default();
        etat.facture = None;
        let api = f.global::<FacturesApi>();
        api.set_textes(textes_type(t));
        api.set_statuts(modele_textes(statuts_filtre(t).into_iter().map(
            |s| match s {
                Some(s) => tr(&s.to_string()),
                None => tr("Tous les statuts"),
            },
        )));
        api.set_avec_paiement(t.recoit_des_reglements());
        api.set_avec_retenue(t.retenue_applicable());
        api.set_avec_pdf(true);
        // Un avoir naît toujours d'une facture validée, jamais d'une page vierge.
        api.set_peut_creer(t != TypeDocument::Avoir && possede(&etat, Droit::brouillon(t)));
        api.set_fiche_ouverte(false);
        // Document créé ou lien suivi depuis une autre page.
        if let Some(id) = etat.document_a_ouvrir.take()
            && let Err(message) = ouvrir_fiche(&f, &mut etat, Some(id))
        {
            api.set_message_ouverture(retour_erreur(message));
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_filtrer(move |texte, statut, paiement, du, au| {
        let Some(f) = f.upgrade() else {
            return SharedString::new();
        };
        let mut etat = e.borrow_mut();
        let mut erreurs = Vec::new();
        let mut periode = |texte: &str, libelle: &'static str| {
            if texte.trim().is_empty() {
                None
            } else {
                let d = date_depuis_texte(texte);
                if d.is_none() {
                    erreurs.push(libelle);
                }
                d
            }
        };
        let statut = au_rang(&statuts_filtre(etat.type_document), statut).flatten();
        etat.factures.filtre = FiltreDocuments {
            texte: texte.to_string(),
            statut,
            etat_paiement: match paiement {
                1 => Some(EtatPaiement::Impaye),
                2 => Some(EtatPaiement::Partiel),
                3 => Some(EtatPaiement::Paye),
                _ => None,
            },
            du: periode(&du, "début"),
            au: periode(&au, "fin"),
        };
        afficher_registre(&f, &mut etat);
        match erreurs.as_slice() {
            [] => SharedString::new(),
            ["début"] => tr(
                "Date de début de période incomplète ou illisible (JJ/MM/AAAA) : non appliquée.",
            )
            .into(),
            [_] => tr(
                "Date de fin de période incomplète ou illisible (JJ/MM/AAAA) : non appliquée.",
            )
            .into(),
            _ => tr(
                "Dates de début et de fin de période incomplètes ou illisibles (JJ/MM/AAAA) : non appliquées.",
            )
            .into(),
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_trier(move |colonne, croissant| {
        let Some(f) = f.upgrade() else { return };
        let mut etat = e.borrow_mut();
        etat.factures.tri = Some((colonne, croissant));
        afficher_registre(&f, &mut etat);
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_selectionner(move |ligne| {
        let Some(f) = f.upgrade() else { return };
        let mut etat = e.borrow_mut();
        let Some(id) = usize::try_from(ligne)
            .ok()
            .and_then(|r| etat.factures.lignes.get(r))
            .map(|d| d.id)
        else {
            return;
        };
        if let Err(message) = ouvrir_fiche(&f, &mut etat, Some(id)) {
            f.global::<FacturesApi>().set_resume(crate::phrase(message));
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_nouvelle(move || {
        let Some(f) = f.upgrade() else { return };
        let mut etat = e.borrow_mut();
        if let Err(message) = ouvrir_fiche(&f, &mut etat, None) {
            f.global::<FacturesApi>().set_resume(crate::phrase(message));
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_fermer_fiche(move || {
        let Some(f) = f.upgrade() else { return };
        e.borrow_mut().facture = None;
        f.global::<FacturesApi>().set_fiche_ouverte(false);
    });

    let e = etat.clone();
    api.on_fiche_courante(move || {
        e.borrow()
            .facture
            .as_ref()
            .map(|e| e.fiche.clone())
            .unwrap_or_default()
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_filtrer_clients(move |texte, rang_courant| {
        let Some(f) = f.upgrade() else { return -1 };
        let mut guard = e.borrow_mut();
        let etat = &mut *guard;
        let Some(edition) = etat.facture.as_mut() else {
            return -1;
        };
        let courant = au_rang(&edition.clients, rang_courant).flatten();
        let (ids, noms) = liste_clients(
            &etat.conn,
            est_achat(edition.type_document),
            &texte,
            &[courant, edition.tiers_document],
        );
        let rang = ids.iter().position(|i| *i == courant).unwrap_or(0) as i32;
        edition.clients = ids;
        f.global::<FacturesApi>().set_clients(modele_textes(noms));
        rang
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_creer_client(move |nom, telephone, adresse, ifu, rccm| {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        let mut guard = e.borrow_mut();
        let etat = &mut *guard;
        let Some(acteur) = etat.utilisateur.clone() else {
            return retour_erreur("aucune session ouverte");
        };
        let achat = etat
            .facture
            .as_ref()
            .is_some_and(|ed| est_achat(ed.type_document));
        let fiche = FicheTiers {
            nom: nom.to_string(),
            telephone: telephone.to_string(),
            adresse: adresse.to_string(),
            ifu: ifu.to_string(),
            rccm: rccm.to_string(),
            ..FicheTiers::vide(if achat {
                TypeTiers::Fournisseur
            } else {
                TypeTiers::Client
            })
        };
        // Le stockage contrôle le droit d'ajouter un tiers et le nom obligatoire.
        let cree = match tiers::creer(&etat.conn, &acteur, &fiche) {
            Ok(t) => t,
            Err(erreur) => return retour_erreur(erreur),
        };
        let Some(edition) = etat.facture.as_mut() else {
            return retour_erreur("aucune fiche ouverte");
        };
        // Liste sans filtre, qui garde le client créé et celui du document.
        let (ids, noms) = liste_clients(
            &etat.conn,
            achat,
            "",
            &[Some(cree.id), edition.tiers_document],
        );
        let rang = ids.iter().position(|i| *i == Some(cree.id)).unwrap_or(0) as i32;
        edition.clients = ids;
        let api = f.global::<FacturesApi>();
        api.set_clients(modele_textes(noms));
        api.set_rang_nouveau_client(rang);
        retour_ok(if achat {
            trf(
                "fournisseur « {} » créé et choisi pour ce document",
                &[&cree.fiche.nom],
            )
        } else {
            trf(
                "client « {} » créé et choisi pour ce document",
                &[&cree.fiche.nom],
            )
        })
    });

    let e = etat.clone();
    api.on_client_choisi(move |rang| {
        let etat = e.borrow();
        let Some(id) = etat
            .facture
            .as_ref()
            .and_then(|ed| au_rang(&ed.clients, rang).flatten())
        else {
            return SharedString::new();
        };
        match tiers::lire(&etat.conn, id) {
            Ok(Some(t)) => detail_tiers(&t.fiche.adresse, &t.fiche.ifu, &t.fiche.rccm).into(),
            _ => SharedString::new(),
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_filtrer_produits(move |texte| {
        let Some(f) = f.upgrade() else { return };
        let mut guard = e.borrow_mut();
        let etat = &mut *guard;
        let Some(edition) = etat.facture.as_mut() else {
            return;
        };
        edition.produits = liste_produits(&etat.conn, est_achat(edition.type_document), &texte);
        afficher_produits(&f.global::<FacturesApi>(), &edition.produits);
    });

    let e = etat.clone();
    api.on_ajouter_produit(move |rang| {
        let etat = e.borrow();
        let Some(edition) = etat.facture.as_ref() else {
            return;
        };
        if let Some(p) = au_rang(&edition.produits, rang) {
            edition.lignes.push(ligne_ui(&LigneSaisie {
                produit_id: Some(p.id),
                reference: p.fiche.reference,
                designation: p.fiche.designation,
                prix: p.fiche.prix,
                quantite: kadenz_domaine::calcul::UNE_UNITE,
                observations: String::new(),
                ..Default::default()
            }));
        }
    });

    let e = etat.clone();
    api.on_ajouter_ligne_libre(move || {
        if let Some(edition) = e.borrow().facture.as_ref() {
            edition.lignes.push(LigneFactureUi {
                quantite: "1".into(),
                ..Default::default()
            });
        }
    });

    let e = etat.clone();
    api.on_reference_saisie(move |ligne| {
        let etat = e.borrow();
        let Some(edition) = etat.facture.as_ref() else {
            return;
        };
        let Ok(rang) = usize::try_from(ligne) else {
            return;
        };
        let Some(mut l) = edition.lignes.row_data(rang) else {
            return;
        };
        match produits::par_reference(&etat.conn, l.reference.trim()) {
            Ok(Some(p)) if p.actif && dans_le_sens(&p, est_achat(edition.type_document)) => {
                l.produit_id = p.id as i32;
                l.reference = p.fiche.reference.into();
                l.designation = p.fiche.designation.into();
                l.prix = saisie::montant_en_texte(p.fiche.prix).into();
            }
            // Référence inconnue : la ligne devient libre, la saisie est gardée.
            _ => l.produit_id = 0,
        }
        edition.lignes.set_row_data(rang, l);
    });

    let e = etat.clone();
    api.on_deplacer_ligne(move |ligne, sens| {
        let etat = e.borrow();
        let Some(edition) = etat.facture.as_ref() else {
            return;
        };
        let (Ok(de), Ok(vers)) = (usize::try_from(ligne), usize::try_from(ligne + sens)) else {
            return;
        };
        if de < edition.lignes.row_count() && vers < edition.lignes.row_count() {
            let l = edition.lignes.remove(de);
            edition.lignes.insert(vers, l);
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_supprimer_ligne(move |ligne| {
        let Some(f) = f.upgrade() else { return };
        let etat = e.borrow();
        if let (Some(edition), Ok(rang)) = (etat.facture.as_ref(), usize::try_from(ligne))
            && rang < edition.lignes.row_count()
        {
            edition.lignes.remove(rang);
            recalculer(
                &f.global::<FacturesApi>(),
                edition,
                edition.remise_rang,
                edition.tva_rang,
                edition.retenue_rang,
            );
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_recalculer(move |remise, tva, retenue| {
        let Some(f) = f.upgrade() else { return };
        if let Some(edition) = e.borrow_mut().facture.as_mut() {
            edition.remise_rang = remise;
            edition.tva_rang = tva;
            edition.retenue_rang = retenue;
            recalculer(&f.global::<FacturesApi>(), edition, remise, tva, retenue);
        }
    });

    let e = etat.clone();
    api.on_echeance_apres_date(move |precedente, date, echeance| {
        let delai = e
            .borrow()
            .facture
            .as_ref()
            .and_then(|ed| ed.delai_echeance_jours);
        echeance_apres_date(delai, &precedente, &date, &echeance)
            .map(SharedString::from)
            .unwrap_or(echeance)
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_enregistrer(move |fiche| {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        let mut etat = e.borrow_mut();
        let Some(acteur) = etat.utilisateur.clone() else {
            return retour_erreur("aucune session ouverte");
        };
        match enregistrer_brouillon(&mut etat, &acteur, &fiche) {
            Ok(id) => {
                if let Err(message) = ouvrir_fiche(&f, &mut etat, Some(id)) {
                    return retour_erreur(message);
                }
                afficher_registre(&f, &mut etat);
                retour_ok("brouillon enregistré")
            }
            Err(message) => retour_erreur(message),
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_valider(move |fiche| {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        let mut etat = e.borrow_mut();
        let Some(acteur) = etat.utilisateur.clone() else {
            return retour_erreur("aucune session ouverte");
        };
        // La saisie affichée est enregistrée avant la validation.
        let id = match enregistrer_brouillon(&mut etat, &acteur, &fiche) {
            Ok(id) => id,
            Err(message) => return retour_erreur(message),
        };
        let t = etat.type_document;
        let resultat = documents::valider(&mut etat.conn, &acteur, id);
        // La fiche est rechargée dans tous les cas : le brouillon a été enregistré.
        let rechargement = ouvrir_fiche(&f, &mut etat, Some(id));
        afficher_registre(&f, &mut etat);
        match (resultat, rechargement) {
            (Ok(numero), Ok(())) => retour_ok(message_validation(t, &numero)),
            (Err(erreur), _) => retour_erreur(trf(
                "brouillon enregistré, mais validation refusée : {}",
                &[&tr(&erreur.to_string())],
            )),
            (Ok(_), Err(message)) => retour_erreur(message),
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_marquer_envoye(move || {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        agir(&f, &e, |etat, acteur, id| {
            documents::marquer_envoye(&mut etat.conn, acteur, id)
                .map(|()| message_envoi(etat.type_document))
                .map_err(texte_erreur)
        })
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_enregistrer_reception(move || {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        agir(&f, &e, |etat, acteur, id| {
            let recues = quantites_recues(etat.facture.as_ref())?;
            documents::enregistrer_reception(&mut etat.conn, acteur, id, &recues)
                .map(|()| tr("réception enregistrée"))
                .map_err(texte_erreur)
        })
    });

    let e = etat.clone();
    api.on_exporter_pdf(move || {
        let etat = e.borrow();
        let Some(id) = etat.facture.as_ref().and_then(|ed| ed.id) else {
            return retour_erreur("enregistrer d'abord le document");
        };
        let (octets, nom) = match crate::pdf::generer(&etat.conn, id) {
            Ok(resultat) => resultat,
            Err(message) => return retour_erreur(message),
        };
        // Dossier d'export du type de document s'il est réglé et existe encore.
        let dossier = parametres::lire(&etat.conn)
            .ok()
            .map(|p| p.dossier_export(etat.type_document).to_string())
            .filter(|d| !d.is_empty() && std::path::Path::new(d).is_dir());
        let mut boite = rfd::FileDialog::new();
        if let Some(dossier) = dossier {
            boite = boite.set_directory(dossier);
        }
        let Some(chemin) = boite
            .set_title(crate::traduction::tr("Enregistrer le PDF"))
            .set_file_name(&nom)
            .add_filter("PDF", &["pdf"])
            .save_file()
        else {
            return Retour::default();
        };
        if let Err(erreur) = std::fs::write(&chemin, &octets) {
            return retour_erreur(crate::traduction::trf(
                "écriture du PDF impossible : {}",
                &[&erreur],
            ));
        }
        // Ouvre le PDF dans la visionneuse par défaut de Windows.
        let _ = std::process::Command::new("explorer").arg(&chemin).spawn();
        retour_ok(crate::traduction::trf(
            "PDF enregistré : {}",
            &[&chemin.display()],
        ))
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_annuler_document(move || {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        agir(&f, &e, |etat, acteur, id| {
            documents::annuler(&mut etat.conn, acteur, id)
                .map(|()| message_annulation(etat.type_document))
                .map_err(texte_erreur)
        })
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_enregistrer_administratif(move |mode, observations| {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        agir(&f, &e, |etat, acteur, id| {
            let mode = etat
                .facture
                .as_ref()
                .and_then(|ed| au_rang(&ed.modes, mode))
                .unwrap_or_default();
            documents::modifier_administratif(&mut etat.conn, acteur, id, &mode, &observations)
                .map(|()| "mode de paiement et observations enregistrés".to_string())
                .map_err(texte_erreur)
        })
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_convertir(move || {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        creer_puis_ouvrir(
            &f,
            &e,
            |etat, acteur, id| {
                documents::convertir_en_facture(&mut etat.conn, acteur, id)
                    .map_err(texte_erreur)
            },
            tr("facture créée en brouillon ; sa validation marquera le document d'origine comme converti"),
        )
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_dupliquer(move |annuler_original| {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        let message = if annuler_original {
            tr("original annulé ; sa copie est créée en brouillon pour être corrigée")
        } else {
            tr("copie créée en brouillon")
        };
        creer_puis_ouvrir(
            &f,
            &e,
            |etat, acteur, id| {
                documents::dupliquer(&mut etat.conn, acteur, id, annuler_original)
                    .map_err(texte_erreur)
            },
            message,
        )
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_creer_avoir(move || {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        creer_puis_ouvrir(
            &f,
            &e,
            |etat, acteur, id| {
                documents::creer_avoir(&mut etat.conn, acteur, id).map_err(texte_erreur)
            },
            tr("avoir créé en brouillon avec les lignes de la facture : retirer des lignes ou baisser les quantités pour un avoir partiel"),
        )
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_ouvrir_lien(move |id| {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        ouvrir_document(&f, &e, i64::from(id), Retour::default())
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_supprimer(move || {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        let mut etat = e.borrow_mut();
        let Some(acteur) = etat.utilisateur.clone() else {
            return retour_erreur("aucune session ouverte");
        };
        let Some(id) = etat.facture.as_ref().and_then(|ed| ed.id) else {
            return retour_erreur("ce brouillon n'a pas été enregistré");
        };
        match documents::supprimer_brouillon(&mut etat.conn, &acteur, id) {
            Ok(()) => {
                etat.facture = None;
                f.global::<FacturesApi>().set_fiche_ouverte(false);
                retour_ok("brouillon supprimé")
            }
            Err(erreur) => retour_erreur(erreur),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_saisies() {
        let d = NaiveDate::from_ymd_opt(2026, 5, 12).unwrap();
        assert_eq!(date_depuis_texte("12/05/2026"), Some(d));
        assert_eq!(date_depuis_texte(" 12-05-2026 "), Some(d));
        assert_eq!(date_depuis_texte("12.05.2026"), Some(d));
        assert_eq!(date_depuis_texte("31/02/2026"), None);
        assert_eq!(date_depuis_texte("2026-05-12"), None);
        assert_eq!(date_depuis_texte("12/05/20"), None);
        assert_eq!(date_en_texte(d), "12/05/2026");
        assert_eq!(
            horodatage_en_texte("2026-05-12T09:30:00"),
            "12/05/2026 09:30"
        );
    }

    #[test]
    fn echeance_suit_la_date_tant_qu_elle_est_automatique() {
        let suivre =
            |precedente, date, echeance| echeance_apres_date(Some(30), precedente, date, echeance);
        // Échéance automatique (date + 30 jours) : elle suit la nouvelle date.
        assert_eq!(
            suivre("01/05/2026", "10/05/2026", "31/05/2026").as_deref(),
            Some("09/06/2026")
        );
        // Échéance saisie à la main : gardée.
        assert_eq!(suivre("01/05/2026", "10/05/2026", "15/07/2026"), None);
        // Date incomplète pendant la frappe : rien ne change.
        assert_eq!(suivre("01/05/2026", "10/05/20", "31/05/2026"), None);
        // Sans délai réglé : jamais de calcul.
        assert_eq!(
            echeance_apres_date(None, "01/05/2026", "10/05/2026", ""),
            None
        );
    }

    #[test]
    fn valeur_retiree_des_parametres_gardee() {
        assert_eq!(avec_valeur(vec![0, 1800], &1800), (vec![0, 1800], 1));
        assert_eq!(avec_valeur(vec![0, 1800], &900), (vec![0, 1800, 900], 2));
    }
}
