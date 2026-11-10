//! Essai du générateur (J4) : facture fictive de plusieurs pages.
//!
//! `cargo run -p kadenz-pdf --example essai_facture -- <dossier>` écrit
//! `essai-facture.pdf` (validée), `essai-brouillon.pdf`, `essai-annulee.pdf`,
//! `essai-devis.pdf` et `essai-avoir.pdf`.
//! Deux arguments facultatifs de plus : fichiers du logo principal et du logo
//! secondaire (PNG ou JPEG).
//! Toutes les données sont inventées pour l'essai.

use kadenz_domaine::calcul::{Taux, UNE_UNITE, calculer_totaux, total_ligne};
use kadenz_domaine::lettres::{Orthographe, en_lettres_majuscule};
use kadenz_pdf::modele::{
    DocumentPdf, EntreprisePdf, Filigrane, LignePdf, LivraisonPdf, TextesPdf, TiersPdf,
};
use kadenz_pdf::texte::Polices;

fn main() {
    let arguments: Vec<String> = std::env::args().collect();
    let dossier = arguments.get(1).cloned().unwrap_or_else(|| ".".into());
    let lire = |rang: usize| {
        arguments
            .get(rang)
            .map(|c| std::fs::read(c).expect("logo lisible"))
    };
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
                quantite_livree: 0,
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
            logo: lire(2),
            logo2: lire(3),
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
        reference: String::new(),
        date: "09/11/2026".into(),
        echeance: "09/12/2026".into(),
        libelle_echeance: String::new(),
        libelle: "Fournitures pour le chantier d'essai".into(),
        mode_paiement: "Virement bancaire".into(),
        observations: String::new(),
        lignes,
        avec_montants: true,
        livraison: LivraisonPdf::default(),
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

    // Devis (J5) : date de fin de validité à la place de l'échéance.
    let devis = DocumentPdf {
        textes: TextesPdf {
            titre: "DEVIS".into(),
            phrase_somme: "Arrêté le présent devis à la somme de".into(),
            ..doc.textes.clone()
        },
        numero: "D2026-11-0001".into(),
        libelle_echeance: "Validité".into(),
        mode_paiement: String::new(),
        ..doc.clone()
    };
    // Avoir partiel (J5) : quelques lignes de la facture, référence de la facture d'origine.
    let lignes_avoir: Vec<LignePdf> = doc.lignes.iter().take(3).cloned().collect();
    let taux_avoir = Taux { remise: 0, ..taux };
    let totaux_avoir = calculer_totaux(
        lignes_avoir.iter().map(|l| (l.prix, l.quantite)),
        taux_avoir,
        false,
    );
    let avoir = DocumentPdf {
        textes: TextesPdf {
            titre: "AVOIR".into(),
            phrase_somme: "Arrêté le présent avoir à la somme de".into(),
            ..doc.textes.clone()
        },
        numero: "A2026-11-0001".into(),
        reference: "Facture d'origine : F2026-11-0001".into(),
        echeance: String::new(),
        libelle: "Retour de marchandises".into(),
        lignes: lignes_avoir,
        taux: taux_avoir,
        montant_en_lettres: en_lettres_majuscule(totaux_avoir.ttc, Orthographe::Rectifiee),
        totaux: totaux_avoir,
        ..doc.clone()
    };
    // Bon de commande (J6) : fournisseur, retenue de 5 % sur la base TVA.
    let lignes_bc: Vec<LignePdf> = doc.lignes.iter().take(5).cloned().collect();
    let taux_bc = Taux {
        retenue: 500,
        ..taux
    };
    let totaux_bc = calculer_totaux(
        lignes_bc.iter().map(|l| (l.prix, l.quantite)),
        taux_bc,
        true,
    );
    let bon_commande = DocumentPdf {
        tiers: TiersPdf {
            nom: "Fournisseur Beta".into(),
            ..doc.tiers.clone()
        },
        textes: TextesPdf {
            titre: "BON DE COMMANDE".into(),
            libelle_tiers: "Fournisseur".into(),
            phrase_somme: "Arrêté le présent bon de commande à la somme de".into(),
            ..doc.textes.clone()
        },
        numero: "C2026-11-0001".into(),
        echeance: String::new(),
        libelle: "Réapprovisionnement du dépôt".into(),
        lignes: lignes_bc,
        taux: taux_bc,
        montant_en_lettres: en_lettres_majuscule(totaux_bc.ttc, Orthographe::Rectifiee),
        totaux: totaux_bc,
        ..doc.clone()
    };
    // Bordereau de livraison (J6) : sans montants, quantités commandée et livrée,
    // zones de visa et dates.
    let bordereau = DocumentPdf {
        textes: TextesPdf {
            titre: "BORDEREAU DE LIVRAISON".into(),
            phrase_somme: String::new(),
            ..doc.textes.clone()
        },
        numero: "L2026-11-0001".into(),
        reference: "Facture d'origine : F2026-11-0001".into(),
        echeance: String::new(),
        mode_paiement: String::new(),
        lignes: doc
            .lignes
            .iter()
            .take(6)
            .enumerate()
            .map(|(k, l)| LignePdf {
                quantite_livree: if k % 3 == 0 {
                    l.quantite / 2
                } else {
                    l.quantite
                },
                ..l.clone()
            })
            .collect(),
        avec_montants: false,
        livraison: LivraisonPdf {
            visa_client: "Reçu en bon état, M. Client".into(),
            visa_fournisseur: "Magasinier".into(),
            date_livraison: "09/11/2026".into(),
            date_reception: "10/11/2026".into(),
        },
        ..doc.clone()
    };
    for (nom, document) in [
        ("essai-devis.pdf", devis),
        ("essai-avoir.pdf", avoir),
        ("essai-bon-commande.pdf", bon_commande),
        ("essai-bordereau.pdf", bordereau),
    ] {
        let octets = kadenz_pdf::generer(&document, &polices).expect("PDF produit");
        let chemin = std::path::Path::new(&dossier).join(nom);
        std::fs::write(&chemin, &octets).expect("écriture");
        println!("{} : {} octets", chemin.display(), octets.len());
    }
}
