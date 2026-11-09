//! Assemblage du PDF d'un document à partir du stockage (section 12).
//!
//! Le document validé s'imprime avec l'instantané figé à sa validation ; un
//! brouillon s'imprime en aperçu, filigrané « BROUILLON », avec les valeurs du
//! moment ; un document annulé porte le filigrane « ANNULÉ ».

use kadenz_domaine::document::{Statut, TypeDocument};
use kadenz_domaine::lettres::en_lettres_majuscule;
use kadenz_pdf::modele::{DocumentPdf, EntreprisePdf, Filigrane, LignePdf, TextesPdf, TiersPdf};
use kadenz_pdf::texte::Polices;
use kadenz_stockage::Connexion as Connection;
use kadenz_stockage::documents::{self, Document};
use kadenz_stockage::impression;

fn date(d: chrono::NaiveDate) -> String {
    d.format("%d/%m/%Y").to_string()
}

/// Données à imprimer pour un document.
pub(crate) fn document_pdf(conn: &Connection, d: &Document) -> Result<DocumentPdf, String> {
    let i = impression::pour_document(conn, d.id).map_err(|e| e.to_string())?;
    let e = &i.entreprise;
    let filigrane = match d.statut {
        Statut::Brouillon => Filigrane::Brouillon,
        Statut::Annule => Filigrane::Annule,
        _ => Filigrane::Aucun,
    };
    let libelle_tiers = if d.type_document == TypeDocument::BonCommande {
        "Fournisseur"
    } else {
        "Client"
    };
    Ok(DocumentPdf {
        entreprise: EntreprisePdf {
            nom: e.nom.clone(),
            adresse: e.adresse.clone(),
            telephone: e.telephone.clone(),
            email: e.email.clone(),
            site_web: e.site_web.clone(),
            regime: e.regime_imposition.clone(),
            rccm: e.rccm.clone(),
            ifu: e.ifu.clone(),
            banque1: e.banque1.clone(),
            banque2: e.banque2.clone(),
            logo: i.logo.clone(),
            logo2: i.logo2.clone(),
        },
        tiers: TiersPdf {
            nom: d.tiers.nom.clone(),
            adresse: d.tiers.adresse.clone(),
            ifu: d.tiers.ifu.clone(),
            rccm: d.tiers.rccm.clone(),
            regime: d.tiers.regime.clone(),
        },
        textes: TextesPdf {
            titre: i.textes.titre.clone(),
            libelle_tiers: libelle_tiers.into(),
            phrase_somme: i.textes.phrase_somme.clone(),
            signataire: i.textes.signataire.clone(),
            nom_signataire: if i.textes.signataire.is_empty() {
                String::new()
            } else {
                e.responsable_comptabilite.clone()
            },
        },
        numero: d.numero.clone().unwrap_or_default(),
        date: date(d.saisie.date),
        echeance: d.saisie.date_echeance.map(date).unwrap_or_default(),
        libelle: d.saisie.libelle.clone(),
        mode_paiement: d.saisie.mode_paiement.clone(),
        observations: d.saisie.observations.clone(),
        lignes: d
            .saisie
            .lignes
            .iter()
            .zip(&d.totaux_lignes)
            .map(|(l, total)| LignePdf {
                reference: l.reference.clone(),
                designation: l.designation.clone(),
                observations: l.observations.clone(),
                prix: l.prix,
                quantite: l.quantite,
                total: *total,
            })
            .collect(),
        avec_montants: d.type_document.a_des_montants(),
        taux: d.saisie.taux,
        totaux: d.totaux,
        montant_en_lettres: en_lettres_majuscule(d.totaux.ttc, i.orthographe),
        filigrane,
        couleur: i.couleur,
    })
}

/// Nom de fichier proposé : le numéro du document, ou « Apercu-<type>-<id> » pour
/// un brouillon, qui n'a pas encore de numéro.
fn nom_de_fichier(d: &Document) -> String {
    match &d.numero {
        Some(numero) => format!("{numero}.pdf"),
        None => format!("Apercu-{}-{}.pdf", d.type_document.code(), d.id),
    }
}

/// Produit le PDF d'un document lu en base, avec le nom de fichier proposé.
pub(crate) fn generer(conn: &Connection, id: i64) -> Result<(Vec<u8>, String), String> {
    let d = documents::lire(conn, id)
        .map_err(|e| e.to_string())?
        .ok_or("document introuvable")?;
    let doc = document_pdf(conn, &d)?;
    let octets = kadenz_pdf::generer(&doc, &Polices::embarquees())?;
    Ok((octets, nom_de_fichier(&d)))
}
