//! Essai du générateur (J4) : facture fictive de plusieurs pages.
//!
//! `cargo run -p kadenz-pdf --example essai_facture -- <dossier>` écrit
//! `essai-facture.pdf` (validée), `essai-brouillon.pdf` et `essai-annulee.pdf`.
//! Toutes les données sont inventées pour l'essai.

use kadenz_domaine::calcul::{Taux, UNE_UNITE, calculer_totaux, total_ligne};
use kadenz_domaine::lettres::{Orthographe, en_lettres_majuscule};
use kadenz_pdf::modele::{DocumentPdf, EntreprisePdf, Filigrane, LignePdf, TextesPdf, TiersPdf};
use kadenz_pdf::texte::Polices;

fn main() {
    let dossier = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let designations = [
        ("PEL-01", "Pelle", ""),
        (
            "BRO-02",
            "Brouette renforcée à roue gonflable, châssis acier galvanisé",
            "Livraison sur chantier",
        ),
        ("CIM-03", "Ciment, sac de 50 kg", ""),
        ("FER-04", "Fer à béton de 12 mm, barre de 12 m", ""),
        (
            "GRA-05",
            "Gravier concassé, tonne",
            "Quantité estimée sur devis",
        ),
        ("SAB-06", "Sable de rivière lavé, tonne", ""),
    ];
    let lignes: Vec<LignePdf> = (0..46)
        .map(|k| {
            let (reference, designation, observations) = designations[k % designations.len()];
            let prix = 2_500 + (k as i64 % 7) * 1_250;
            let quantite = (1 + k as i64 % 4) * UNE_UNITE + if k % 5 == 0 { 500 } else { 0 };
            LignePdf {
                reference: reference.into(),
                designation: designation.into(),
                observations: observations.into(),
                prix,
                quantite,
                total: total_ligne(prix, quantite),
            }
        })
        .collect();
    let taux = Taux {
        remise: 500,
        tva: 1800,
        retenue: 0,
    };
    let totaux = calculer_totaux(lignes.iter().map(|l| (l.prix, l.quantite)), taux, false);

    let doc = DocumentPdf {
        entreprise: EntreprisePdf {
            nom: "Entreprise Essai".into(),
            adresse: "01 BP 0000, Ville Fictive".into(),
            telephone: "00 00 00 00".into(),
            email: "contact@exemple.invalid".into(),
            site_web: "www.exemple.invalid".into(),
            regime: "Régime fictif".into(),
            rccm: "RCCM-ESSAI-0000".into(),
            ifu: "IFU-ESSAI-0000".into(),
            banque1: "Banque Essai, compte 0000 0000 0000".into(),
            banque2: String::new(),
            logo: None,
            logo2: None,
        },
        tiers: TiersPdf {
            nom: "Client Alpha".into(),
            adresse: "02 BP 1111, Ville Fictive".into(),
            ifu: "IFU-CLIENT-1111".into(),
            rccm: String::new(),
            regime: String::new(),
        },
        textes: TextesPdf {
            titre: "FACTURE".into(),
            libelle_tiers: "Client".into(),
            phrase_somme: "Arrêté la présente facture à la somme de".into(),
            signataire: "LA COMPTABILITE".into(),
            nom_signataire: "Nom Fictif".into(),
        },
        numero: "F2026-11-0001".into(),
        date: "09/11/2026".into(),
        echeance: "09/12/2026".into(),
        libelle: "Fournitures pour le chantier d'essai".into(),
        mode_paiement: "Virement bancaire".into(),
        observations: String::new(),
        lignes,
        avec_montants: true,
        taux,
        montant_en_lettres: en_lettres_majuscule(totaux.ttc, Orthographe::Rectifiee),
        totaux,
        filigrane: Filigrane::Aucun,
        couleur: [0x2d, 0x3a, 0x8c],
    };
    let polices = Polices::embarquees();
    for (nom, filigrane) in [
        ("essai-facture.pdf", Filigrane::Aucun),
        ("essai-brouillon.pdf", Filigrane::Brouillon),
        ("essai-annulee.pdf", Filigrane::Annule),
    ] {
        let doc = DocumentPdf {
            filigrane,
            ..doc.clone()
        };
        let debut = std::time::Instant::now();
        let octets = kadenz_pdf::generer(&doc, &polices).expect("PDF produit");
        let chemin = std::path::Path::new(&dossier).join(nom);
        std::fs::write(&chemin, &octets).expect("écriture");
        println!(
            "{} : {} octets en {:?}",
            chemin.display(),
            octets.len(),
            debut.elapsed()
        );
    }
}
