//! Assemblage du PDF d'un document à partir du stockage (section 12).
//!
//! Le document validé s'imprime avec l'instantané figé à sa validation ; un
//! brouillon s'imprime en aperçu, filigrané « BROUILLON », avec les valeurs du
//! moment ; un document annulé porte le filigrane « ANNULÉ ».

use kadenz_domaine::calcul::Montant;
use kadenz_domaine::document::{Statut, TypeDocument};
use kadenz_domaine::lettres::en_lettres_majuscule;
use kadenz_domaine::saisie::montant_en_texte;
use kadenz_pdf::modele::{
    ColonnePdf, DocumentPdf, EntreprisePdf, Filigrane, LettrePdf, LignePdf, ListePdf, LivraisonPdf,
    TextesPdf, TiersPdf, TotalPdf,
};
use kadenz_pdf::texte::Polices;
use kadenz_stockage::Connexion as Connection;
use kadenz_stockage::documents::{self, Document, NatureLien};
use kadenz_stockage::entreprise::{self, Entreprise, Logo};
use kadenz_stockage::impression;
use kadenz_stockage::parametres;
use kadenz_stockage::reglements::{self, FiltreRegistre, Sens, TotalSens};
use kadenz_stockage::relances;

use crate::referentiels::texte_erreur;

fn date(d: chrono::NaiveDate) -> String {
    d.format("%d/%m/%Y").to_string()
}

/// Référence imprimée sous la date : la facture d'origine d'un avoir ou d'un
/// bordereau issu d'une facture.
fn reference(d: &Document) -> String {
    let Some(l) = &d.source else {
        return String::new();
    };
    match (&l.numero, l.nature) {
        (Some(numero), NatureLien::Avoir | NatureLien::Livraison) => {
            format!("Facture d'origine : {numero}")
        }
        _ => String::new(),
    }
}

fn entreprise_pdf(e: &Entreprise, logo: Option<Vec<u8>>, logo2: Option<Vec<u8>>) -> EntreprisePdf {
    EntreprisePdf {
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
        logo,
        logo2,
    }
}

/// Données à imprimer pour un document.
pub(crate) fn document_pdf(conn: &Connection, d: &Document) -> Result<DocumentPdf, String> {
    let i = impression::pour_document(conn, d.id).map_err(texte_erreur)?;
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
        entreprise: entreprise_pdf(&i.entreprise, i.logo.clone(), i.logo2.clone()),
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
                i.entreprise.responsable_comptabilite.clone()
            },
        },
        numero: d.numero.clone().unwrap_or_default(),
        reference: reference(d),
        date: date(d.saisie.date),
        echeance: d.saisie.date_echeance.map(date).unwrap_or_default(),
        libelle_echeance: match d.type_document {
            TypeDocument::Devis | TypeDocument::Proforma => "Validité".into(),
            _ => String::new(),
        },
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
                quantite_livree: l.quantite_livree,
                total: *total,
            })
            .collect(),
        avec_montants: d.type_document.a_des_montants(),
        livraison: LivraisonPdf {
            visa_client: d.saisie.livraison.visa_client.clone(),
            visa_fournisseur: d.saisie.livraison.visa_fournisseur.clone(),
            date_livraison: d.saisie.livraison.date_livraison.clone(),
            date_reception: d.saisie.livraison.date_reception.clone(),
        },
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
        .map_err(texte_erreur)?
        .ok_or("document introuvable")?;
    let doc = document_pdf(conn, &d)?;
    let octets = kadenz_pdf::generer(&doc, &Polices::embarquees())?;
    Ok((octets, nom_de_fichier(&d)))
}

fn aujourd_hui() -> chrono::NaiveDate {
    chrono::Local::now().date_naive()
}

fn fcfa(m: Montant) -> String {
    format!("{} FCFA", montant_en_texte(m))
}

/// Fiche entreprise et couleur du moment, pour une lettre ou une liste imprimée.
fn entete_du_moment(conn: &Connection) -> Result<(EntreprisePdf, [u8; 3]), String> {
    let e = entreprise::lire(conn)
        .map_err(texte_erreur)?
        .unwrap_or_default();
    let logo = entreprise::lire_logo(conn, Logo::Principal).map_err(texte_erreur)?;
    let logo2 = entreprise::lire_logo(conn, Logo::Secondaire).map_err(texte_erreur)?;
    let couleur = parametres::lire(conn)
        .map_err(texte_erreur)?
        .couleur_documents;
    Ok((entreprise_pdf(&e, logo, logo2), couleur))
}

fn colonne(titre: &str, largeur: f32, a_droite: bool) -> ColonnePdf {
    ColonnePdf {
        titre: titre.into(),
        largeur,
        a_droite,
    }
}

/// Lettre de relance d'une facture échue (section 8), datée du jour ; le texte
/// est celui des paramètres, en français comme tous les PDF (section 13).
// Branché par l'écran des relances (kadenz-bf) : retirer alors l'attribut.
#[allow(dead_code)]
pub(crate) fn lettre_relance(
    conn: &Connection,
    facture_id: i64,
) -> Result<(Vec<u8>, String), String> {
    let l = relances::lettre(conn, facture_id, aujourd_hui()).map_err(texte_erreur)?;
    let lieu = l.entreprise.localisation.trim();
    let lieu_et_date = if lieu.is_empty() {
        format!("Le {}", date(l.date))
    } else {
        format!("{lieu}, le {}", date(l.date))
    };
    let f = &l.facture;
    let lettre = LettrePdf {
        entreprise: entreprise_pdf(&l.entreprise, l.logo.clone(), l.logo2.clone()),
        couleur: l.couleur,
        lieu_et_date,
        destinataire: TiersPdf {
            nom: l.client.nom.clone(),
            adresse: l.client.adresse.clone(),
            ifu: l.client.ifu.clone(),
            rccm: l.client.rccm.clone(),
            regime: l.client.regime.clone(),
        },
        titre: l.titre.clone(),
        objet: l.objet.clone(),
        avant: l.avant.clone(),
        apres: l.apres.clone(),
        recapitulatif: vec![
            ("Facture".into(), f.numero.clone()),
            ("Date".into(), date(f.date)),
            ("Échéance".into(), date(f.echeance)),
            ("Montant dû".into(), montant_en_texte(f.montant_du)),
            ("Réglé".into(), montant_en_texte(f.regle)),
            ("Reste dû".into(), montant_en_texte(f.reste_a_payer)),
        ],
        signataire: l.signataire.clone(),
        nom_signataire: l.nom_signataire.clone(),
    };
    let octets = kadenz_pdf::generer_lettre(&lettre, &Polices::embarquees())?;
    Ok((octets, format!("Relance-{}.pdf", f.numero)))
}

fn libelle_sens(s: Sens) -> &'static str {
    match s {
        Sens::Encaissement => "Encaissement",
        Sens::Decaissement => "Décaissement",
    }
}

fn total(libelle: String, valeur: Montant, principal: bool) -> TotalPdf {
    TotalPdf {
        libelle,
        valeur: fcfa(valeur),
        principal,
    }
}

fn totaux_du_sens(libelle: &str, t: &TotalSens, totaux: &mut Vec<TotalPdf>) {
    totaux.push(total(libelle.into(), t.total, true));
    for (mode, montant) in &t.par_mode {
        totaux.push(total(format!("dont {mode}"), *montant, false));
    }
}

fn criteres_du_registre(filtre: &FiltreRegistre) -> Vec<String> {
    let mut criteres = vec![
        match filtre.sens {
            Some(Sens::Encaissement) => "Encaissements (factures)",
            Some(Sens::Decaissement) => "Décaissements (bons de commande)",
            None => "Encaissements et décaissements",
        }
        .to_string(),
    ];
    match (filtre.du, filtre.au) {
        (Some(du), Some(au)) => criteres.push(format!("Période : du {} au {}", date(du), date(au))),
        (Some(du), None) => criteres.push(format!("Période : depuis le {}", date(du))),
        (None, Some(au)) => criteres.push(format!("Période : jusqu'au {}", date(au))),
        (None, None) => {}
    }
    if let Some(mode) = filtre.mode.as_deref().filter(|m| !m.trim().is_empty()) {
        criteres.push(format!("Mode de paiement : {}", mode.trim()));
    }
    criteres
}

/// Registre des règlements filtré, imprimé, avec le total et les sous-totaux
/// par mode de chaque sens (section 8).
// Branché par l'écran des règlements (kadenz-bf) : retirer alors l'attribut.
#[allow(dead_code)]
pub(crate) fn registre_reglements(
    conn: &Connection,
    filtre: &FiltreRegistre,
) -> Result<(Vec<u8>, String), String> {
    let r = reglements::registre(conn, filtre).map_err(texte_erreur)?;
    let (entreprise, couleur) = entete_du_moment(conn)?;
    let mut totaux = Vec::new();
    if filtre.sens != Some(Sens::Decaissement) {
        totaux_du_sens("Total des encaissements", &r.encaissements, &mut totaux);
    }
    if filtre.sens != Some(Sens::Encaissement) {
        totaux_du_sens("Total des décaissements", &r.decaissements, &mut totaux);
    }
    let liste = ListePdf {
        entreprise,
        couleur,
        titre: "REGISTRE DES RÈGLEMENTS".into(),
        date: date(aujourd_hui()),
        criteres: criteres_du_registre(filtre),
        colonnes: vec![
            colonne("Date", 58.0, false),
            colonne("Sens", 74.0, false),
            colonne("Document", 80.0, false),
            colonne("Tiers", 0.0, false),
            colonne("Mode", 72.0, false),
            colonne("Référence", 70.0, false),
            colonne("Montant", 66.0, true),
        ],
        lignes: r
            .lignes
            .iter()
            .map(|l| {
                vec![
                    date(l.reglement.date),
                    libelle_sens(l.sens).into(),
                    l.numero.clone(),
                    l.tiers.clone(),
                    l.reglement.mode_paiement.clone(),
                    l.reglement.reference.clone(),
                    montant_en_texte(l.reglement.montant),
                ]
            })
            .collect(),
        totaux,
    };
    let octets = kadenz_pdf::generer_liste(&liste, &Polices::embarquees())?;
    Ok((
        octets,
        format!(
            "Registre-reglements-{}.pdf",
            aujourd_hui().format("%Y-%m-%d")
        ),
    ))
}

/// Liste des factures échues non payées au jour, imprimée (section 8).
// Branché par l'écran des relances (kadenz-bf) : retirer alors l'attribut.
#[allow(dead_code)]
pub(crate) fn liste_relances(conn: &Connection) -> Result<(Vec<u8>, String), String> {
    let jour = aujourd_hui();
    let echues = reglements::factures_echues(conn, jour).map_err(texte_erreur)?;
    let (entreprise, couleur) = entete_du_moment(conn)?;
    let reste: Montant = echues.iter().map(|d| d.reste_a_payer).sum();
    let liste = ListePdf {
        entreprise,
        couleur,
        titre: "FACTURES ÉCHUES".into(),
        date: date(jour),
        criteres: vec![format!("Factures échues non payées au {}", date(jour))],
        colonnes: vec![
            colonne("Facture", 82.0, false),
            colonne("Client", 0.0, false),
            colonne("Échéance", 60.0, false),
            colonne("Retard (jours)", 64.0, true),
            colonne("Montant dû", 74.0, true),
            colonne("Reste dû", 74.0, true),
        ],
        lignes: echues
            .iter()
            .map(|d| {
                vec![
                    d.numero.clone(),
                    d.tiers.clone(),
                    d.echeance.map(date).unwrap_or_default(),
                    d.jours_de_retard.unwrap_or_default().to_string(),
                    montant_en_texte(d.montant_du),
                    montant_en_texte(d.reste_a_payer),
                ]
            })
            .collect(),
        totaux: vec![total(
            format!("Reste dû ({} factures)", echues.len()),
            reste,
            true,
        )],
    };
    let octets = kadenz_pdf::generer_liste(&liste, &Polices::embarquees())?;
    Ok((octets, format!("Relances-{}.pdf", jour.format("%Y-%m-%d"))))
}
