//! Essai du générateur (J4) : facture fictive de plusieurs pages.
//!
//! `cargo run -p kadenz-pdf --example essai_facture -- <dossier>` écrit
//! `essai-facture.pdf` (validée), `essai-brouillon.pdf`, `essai-annulee.pdf`,
//! `essai-devis.pdf`, `essai-avoir.pdf`, `essai-bon-commande.pdf`,
//! `essai-bordereau.pdf`, `essai-relance.pdf` et `essai-registre.pdf`.
//! Deux arguments facultatifs de plus : fichiers du logo principal et du logo
//! secondaire (PNG ou JPEG).
//! Toutes les données sont inventées pour l'essai.

use kadenz_domaine::calcul::{Taux, UNE_UNITE, calculer_totaux, total_ligne};
use kadenz_domaine::lettres::{Orthographe, en_lettres_majuscule};
use kadenz_domaine::mise_en_page::{Gabarit, MiseEnPage, MiseEnPageAnalysee};
use kadenz_pdf::modele::{
    ColonnePdf, DocumentPdf, EntreprisePdf, Filigrane, LettrePdf, LignePdf, ListePdf, LivraisonPdf,
    TextesPdf, TiersPdf, TotalPdf,
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
            localisation: "Secteur fictif".into(),
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
        mise_en_page: MiseEnPageAnalysee::default(),
    };
    let polices = Polices::embarquees();

    // Gabarits et zones réglées (J8b).
    let reglee = MiseEnPage {
        entete: "-> {logo}
-> **{nom}** · *{localisation}*
-> {adresse} · Tél. : {telephone}"
            .into(),
        pied: "-> *Merci de votre confiance.*
-> RCCM : {rccm} · IFU : {ifu} · {banque2}"
            .into(),
        ..MiseEnPage::default()
    };
    for (nom, gabarit, m) in [
        ("essai-compact.pdf", Gabarit::Compact, MiseEnPage::default()),
        (
            "essai-preimprime.pdf",
            Gabarit::PreImprime,
            MiseEnPage::default(),
        ),
        ("essai-zones.pdf", Gabarit::Classique, reglee),
    ] {
        let m = MiseEnPage { gabarit, ..m };
        let doc = DocumentPdf {
            mise_en_page: m.analyser().expect("réglages valides"),
            ..doc.clone()
        };
        let octets = kadenz_pdf::generer(&doc, &polices).expect("PDF produit");
        let chemin = std::path::Path::new(&dossier).join(nom);
        std::fs::write(&chemin, &octets).expect("écriture");
        println!("{}", chemin.display());
    }
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

    // Lettre de relance (J7), texte par défaut validé par l'utilisateur.
    let relance = LettrePdf {
        entreprise: doc.entreprise.clone(),
        couleur: doc.couleur,
        lieu_et_date: "Ville Fictive, le 11/06/2026".into(),
        destinataire: doc.tiers.clone(),
        titre: "LETTRE DE RELANCE".into(),
        objet: "Objet : facture n° F2026-05-0001 impayée".into(),
        avant: "Madame, Monsieur,\n\nSauf erreur ou omission de notre part, la facture \
                n° F2026-05-0001 du 03/05/2026, arrivée à échéance le 01/06/2026, reste \
                impayée à ce jour pour un montant de 60\u{a0}000 FCFA, soit 10 jours de retard."
            .into(),
        apres: "Nous vous remercions de bien vouloir procéder à son règlement dans les \
                meilleurs délais. Si votre paiement a été effectué entre-temps, nous vous \
                prions de ne pas tenir compte de la présente.\n\nNous vous prions \
                d'agréer, Madame, Monsieur, l'expression de nos salutations distinguées."
            .into(),
        recapitulatif: [
            ("Facture", "F2026-05-0001"),
            ("Date", "03/05/2026"),
            ("Échéance", "01/06/2026"),
            ("Montant dû", "100\u{a0}000"),
            ("Réglé", "40\u{a0}000"),
            ("Reste dû", "60\u{a0}000"),
        ]
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .to_vec(),
        signataire: "LA COMPTABILITE".into(),
        nom_signataire: "Nom Fictif".into(),
        mise_en_page: doc.mise_en_page.clone(),
    };
    let octets = kadenz_pdf::generer_lettre(&relance, &polices).expect("PDF produit");
    let chemin = std::path::Path::new(&dossier).join("essai-relance.pdf");
    std::fs::write(&chemin, &octets).expect("écriture");
    println!("{} : {} octets", chemin.display(), octets.len());

    // Registre des règlements (J7) : 70 lignes fictives sur deux pages.
    let colonne = |titre: &str, largeur: f32, a_droite: bool| ColonnePdf {
        titre: titre.into(),
        largeur,
        a_droite,
    };
    let registre = ListePdf {
        entreprise: doc.entreprise.clone(),
        couleur: doc.couleur,
        titre: "REGISTRE DES RÈGLEMENTS".into(),
        date: "11/06/2026".into(),
        criteres: vec![
            "Encaissements et décaissements".into(),
            "Période : du 01/05/2026 au 31/05/2026".into(),
        ],
        colonnes: vec![
            colonne("Date", 58.0, false),
            colonne("Sens", 74.0, false),
            colonne("Document", 80.0, false),
            colonne("Tiers", 0.0, false),
            colonne("Mode", 72.0, false),
            colonne("Référence", 70.0, false),
            colonne("Montant", 66.0, true),
        ],
        lignes: (0..70)
            .map(|k| {
                let achat = k % 4 == 0;
                vec![
                    format!("{:02}/05/2026", 1 + k % 30),
                    if achat {
                        "Décaissement"
                    } else {
                        "Encaissement"
                    }
                    .into(),
                    if achat {
                        format!("C2026-05-{:04}", k + 1)
                    } else {
                        format!("F2026-05-{:04}", k + 1)
                    },
                    if achat {
                        "Fournisseur Beta"
                    } else {
                        "Client Alpha, établissement principal"
                    }
                    .into(),
                    if k % 3 == 0 { "Chèque" } else { "Espèce" }.into(),
                    if k % 3 == 0 {
                        format!("CHQ {k:04}")
                    } else {
                        String::new()
                    },
                    "25\u{a0}000".into(),
                ]
            })
            .collect(),
        totaux: [
            ("Total des encaissements", "1\u{a0}300\u{a0}000 FCFA"),
            ("dont Chèque", "425\u{a0}000 FCFA"),
            ("dont Espèce", "875\u{a0}000 FCFA"),
            ("Total des décaissements", "450\u{a0}000 FCFA"),
            ("dont Chèque", "175\u{a0}000 FCFA"),
            ("dont Espèce", "275\u{a0}000 FCFA"),
        ]
        .map(|(libelle, valeur)| TotalPdf {
            principal: libelle.starts_with("Total"),
            libelle: libelle.into(),
            valeur: valeur.into(),
        })
        .to_vec(),
        mise_en_page: doc.mise_en_page.clone(),
    };
    let octets = kadenz_pdf::generer_liste(&registre, &polices).expect("PDF produit");
    let chemin = std::path::Path::new(&dossier).join("essai-registre.pdf");
    std::fs::write(&chemin, &octets).expect("écriture");
    println!("{} : {} octets", chemin.display(), octets.len());
}
