//! Pages du module Paiements (section 8) : registre des règlements, suivi des
//! bons de commande non payés et relances des factures échues.

use std::cell::RefCell;
use std::rc::Rc;

use kadenz_domaine::calcul::Montant;
use kadenz_domaine::saisie;
use kadenz_domaine::texte::contient;
use kadenz_stockage::documents::TypeDocument;
use kadenz_stockage::parametres;
use kadenz_stockage::reglements::{self, DocumentDu, FiltreRegistre, Sens, TotalSens};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::factures::{date_depuis_texte, date_en_texte, page_du_type};
use crate::referentiels::{modele_lignes, retour_erreur, retour_ok, texte_erreur};
use crate::traduction::{tr, trf};
use crate::{EtatPartage, FenetrePrincipale, PaiementsApi, Retour, TotalUi};

/// Listes affichées : la ligne choisie à l'écran désigne un document de ces listes.
#[derive(Default)]
struct Listes {
    filtre: FiltreRegistre,
    reglements: Vec<i64>,
    fournisseurs: Vec<DocumentDu>,
    relances: Vec<DocumentDu>,
}

fn montant(m: Montant) -> String {
    saisie::montant_en_texte(m)
}

fn libelle_sens(s: Sens) -> String {
    match s {
        Sens::Encaissement => tr("Encaissement"),
        Sens::Decaissement => tr("Décaissement"),
    }
}

/// Total d'un sens puis ses sous-totaux par mode ; rien si le sens est vide.
fn totaux_du_sens(libelle: String, t: &TotalSens, lignes: &mut Vec<TotalUi>) {
    if t.par_mode.is_empty() {
        return;
    }
    lignes.push(TotalUi {
        libelle: libelle.into(),
        valeur: trf("{} FCFA", &[&montant(t.total)]).into(),
        fort: true,
    });
    for (mode, m) in &t.par_mode {
        lignes.push(TotalUi {
            libelle: mode.as_str().into(),
            valeur: trf("{} FCFA", &[&montant(*m)]).into(),
            fort: false,
        });
    }
}

/// Ouvre la fiche d'un document dans la page de son type.
fn ouvrir(f: &FenetrePrincipale, etat: &EtatPartage, id: i64, t: TypeDocument) -> Retour {
    etat.borrow_mut().document_a_ouvrir = Some(id);
    f.set_page(page_du_type(t));
    Retour::default()
}

/// Enregistre un PDF à l'endroit choisi par l'utilisateur, puis l'ouvre.
fn enregistrer_pdf(octets: &[u8], nom: &str, dossier: Option<String>) -> Retour {
    let mut boite = rfd::FileDialog::new();
    if let Some(dossier) = dossier.filter(|d| !d.is_empty() && std::path::Path::new(d).is_dir()) {
        boite = boite.set_directory(dossier);
    }
    let Some(chemin) = boite
        .set_title(tr("Enregistrer le PDF"))
        .set_file_name(nom)
        .add_filter("PDF", &["pdf"])
        .save_file()
    else {
        return Retour::default();
    };
    if let Err(erreur) = std::fs::write(&chemin, octets) {
        return retour_erreur(trf("écriture du PDF impossible : {}", &[&erreur]));
    }
    // Ouvre le PDF dans la visionneuse par défaut de Windows.
    let _ = std::process::Command::new("explorer").arg(&chemin).spawn();
    retour_ok(trf("PDF enregistré : {}", &[&chemin.display()]))
}

fn afficher_reglements(f: &FenetrePrincipale, etat: &EtatPartage, listes: &mut Listes) {
    let api = f.global::<PaiementsApi>();
    let etat = etat.borrow();
    let registre = match reglements::registre(&etat.conn, &listes.filtre) {
        Ok(r) => r,
        Err(erreur) => {
            api.set_reglements(modele_lignes(Vec::new()));
            api.set_totaux(ModelRc::new(VecModel::from(Vec::<TotalUi>::new())));
            api.set_resume_reglements(texte_erreur(erreur).into());
            return;
        }
    };
    listes.reglements = registre
        .lignes
        .iter()
        .map(|l| l.reglement.document_id)
        .collect();
    api.set_reglements(modele_lignes(
        registre
            .lignes
            .iter()
            .map(|l| {
                vec![
                    date_en_texte(l.reglement.date),
                    libelle_sens(l.sens),
                    l.numero.clone(),
                    l.tiers.clone(),
                    l.reglement.mode_paiement.clone(),
                    l.reglement.reference.clone(),
                    montant(l.reglement.montant),
                ]
            })
            .collect(),
    ));
    let mut totaux = Vec::new();
    totaux_du_sens(
        tr("Total des encaissements"),
        &registre.encaissements,
        &mut totaux,
    );
    totaux_du_sens(
        tr("Total des décaissements"),
        &registre.decaissements,
        &mut totaux,
    );
    api.set_totaux(ModelRc::new(VecModel::from(totaux)));
    let resume = match registre.lignes.len() {
        0 => tr("Aucun règlement."),
        1 => tr("1 règlement."),
        n => trf("{} règlements.", &[&n]),
    };
    api.set_resume_reglements(resume.into());
}

fn cellules_du(d: &DocumentDu, relance: bool) -> Vec<String> {
    let mut c = vec![d.numero.clone(), date_en_texte(d.date)];
    if relance {
        c.push(d.echeance.map(date_en_texte).unwrap_or_default());
    }
    c.push(d.tiers.clone());
    c.push(montant(d.montant_du));
    c.push(montant(d.regle));
    c.push(montant(d.reste_a_payer));
    if relance {
        c.push(d.jours_de_retard.map(|j| j.to_string()).unwrap_or_default());
    } else {
        c.push(tr(&d.statut.to_string()));
    }
    c
}

/// Résumé d'une liste de documents dus : nombre et reste dû total.
fn resume_du(liste: &[DocumentDu], relance: bool) -> String {
    let reste = montant(liste.iter().map(|d| d.reste_a_payer).sum());
    match (relance, liste.len()) {
        (false, 0) => tr("Aucun bon de commande à payer."),
        (false, 1) => trf("1 bon de commande à payer ; reste dû : {} FCFA.", &[&reste]),
        (false, n) => trf(
            "{} bons de commande à payer ; reste dû : {} FCFA.",
            &[&n, &reste],
        ),
        (true, 0) => tr("Aucune facture échue à relancer."),
        (true, 1) => trf("1 facture échue ; reste dû : {} FCFA.", &[&reste]),
        (true, n) => trf("{} factures échues ; reste dû : {} FCFA.", &[&n, &reste]),
    }
}

fn filtrer_du(liste: Vec<DocumentDu>, texte: &str) -> Vec<DocumentDu> {
    liste
        .into_iter()
        .filter(|d| {
            texte.trim().is_empty() || contient(&d.numero, texte) || contient(&d.tiers, texte)
        })
        .collect()
}

fn au_rang(liste: &[DocumentDu], ligne: i32) -> Option<&DocumentDu> {
    usize::try_from(ligne).ok().and_then(|r| liste.get(r))
}

pub(crate) fn brancher(fenetre: &FenetrePrincipale, etat: &EtatPartage) {
    let api = fenetre.global::<PaiementsApi>();
    let listes = Rc::new(RefCell::new(Listes::default()));

    // Registre des règlements.
    let (f, e, l) = (fenetre.as_weak(), etat.clone(), listes.clone());
    api.on_filtrer_reglements(move |sens, mode, du, au| {
        let Some(f) = f.upgrade() else {
            return SharedString::new();
        };
        let modes = parametres::lire(&e.borrow().conn)
            .map(|p| p.modes_paiement)
            .unwrap_or_default();
        let mut liste_modes = vec![tr("Tous les modes")];
        liste_modes.extend(modes.iter().cloned());
        f.global::<PaiementsApi>()
            .set_modes(ModelRc::new(VecModel::from(
                liste_modes.into_iter().map(SharedString::from).collect::<Vec<_>>(),
            )));
        let mut erreurs = Vec::new();
        let mut periode = |texte: &str, libelle: &'static str| {
            if texte.trim().is_empty() {
                return None;
            }
            let d = date_depuis_texte(texte);
            if d.is_none() {
                erreurs.push(libelle);
            }
            d
        };
        let filtre = FiltreRegistre {
            sens: match sens {
                1 => Some(Sens::Encaissement),
                2 => Some(Sens::Decaissement),
                _ => None,
            },
            du: periode(&du, "début"),
            au: periode(&au, "fin"),
            mode: usize::try_from(mode - 1)
                .ok()
                .and_then(|r| modes.get(r).cloned()),
        };
        let mut listes = l.borrow_mut();
        listes.filtre = filtre;
        afficher_reglements(&f, &e, &mut listes);
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

    let (f, e, l) = (fenetre.as_weak(), etat.clone(), listes.clone());
    api.on_ouvrir_reglement(move |ligne| {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        let Some(id) = usize::try_from(ligne)
            .ok()
            .and_then(|r| l.borrow().reglements.get(r).copied())
        else {
            return Retour::default();
        };
        let t = match kadenz_stockage::documents::lire(&e.borrow().conn, id) {
            Ok(Some(d)) => d.type_document,
            Ok(None) => return retour_erreur(tr("ce document n'existe plus")),
            Err(erreur) => return retour_erreur(erreur),
        };
        ouvrir(&f, &e, id, t)
    });

    let (e, l) = (etat.clone(), listes.clone());
    api.on_imprimer_registre(move || {
        let etat = e.borrow();
        match crate::pdf::registre_reglements(&etat.conn, &l.borrow().filtre) {
            Ok((octets, nom)) => enregistrer_pdf(&octets, &nom, None),
            Err(message) => retour_erreur(tr(&message)),
        }
    });

    // Suivi fournisseurs.
    let (f, e, l) = (fenetre.as_weak(), etat.clone(), listes.clone());
    api.on_filtrer_fournisseurs(move |texte| {
        let Some(f) = f.upgrade() else { return };
        let api = f.global::<PaiementsApi>();
        let liste = match reglements::bons_de_commande_non_payes(&e.borrow().conn) {
            Ok(liste) => filtrer_du(liste, &texte),
            Err(erreur) => {
                api.set_resume_fournisseurs(texte_erreur(erreur).into());
                Vec::new()
            }
        };
        api.set_fournisseurs(modele_lignes(
            liste.iter().map(|d| cellules_du(d, false)).collect(),
        ));
        api.set_resume_fournisseurs(resume_du(&liste, false).into());
        l.borrow_mut().fournisseurs = liste;
    });

    let (f, e, l) = (fenetre.as_weak(), etat.clone(), listes.clone());
    api.on_ouvrir_fournisseur(move |ligne| {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        let Some(id) = au_rang(&l.borrow().fournisseurs, ligne).map(|d| d.id) else {
            return Retour::default();
        };
        ouvrir(&f, &e, id, TypeDocument::BonCommande)
    });

    // Relances.
    let (f, e, l) = (fenetre.as_weak(), etat.clone(), listes.clone());
    api.on_filtrer_relances(move |texte| {
        let Some(f) = f.upgrade() else { return };
        let api = f.global::<PaiementsApi>();
        let aujourd_hui = chrono::Local::now().date_naive();
        let liste = match reglements::factures_echues(&e.borrow().conn, aujourd_hui) {
            Ok(liste) => filtrer_du(liste, &texte),
            Err(erreur) => {
                api.set_resume_relances(texte_erreur(erreur).into());
                Vec::new()
            }
        };
        api.set_relances(modele_lignes(
            liste.iter().map(|d| cellules_du(d, true)).collect(),
        ));
        api.set_resume_relances(resume_du(&liste, true).into());
        l.borrow_mut().relances = liste;
    });

    let (f, e, l) = (fenetre.as_weak(), etat.clone(), listes.clone());
    api.on_ouvrir_relance(move |ligne| {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        let Some(id) = au_rang(&l.borrow().relances, ligne).map(|d| d.id) else {
            return retour_erreur(tr("choisir une facture dans la liste"));
        };
        ouvrir(&f, &e, id, TypeDocument::Facture)
    });

    let (e, l) = (etat.clone(), listes.clone());
    api.on_lettre_relance(move |ligne| {
        let Some(id) = au_rang(&l.borrow().relances, ligne).map(|d| d.id) else {
            return retour_erreur(tr("choisir une facture dans la liste"));
        };
        let etat = e.borrow();
        let dossier = parametres::lire(&etat.conn)
            .ok()
            .map(|p| p.dossier_export(TypeDocument::Facture).to_string());
        match crate::pdf::lettre_relance(&etat.conn, id) {
            Ok((octets, nom)) => enregistrer_pdf(&octets, &nom, dossier),
            Err(message) => retour_erreur(tr(&message)),
        }
    });

    let e = etat.clone();
    api.on_imprimer_relances(move || {
        let etat = e.borrow();
        match crate::pdf::liste_relances(&etat.conn) {
            Ok((octets, nom)) => enregistrer_pdf(&octets, &nom, None),
            Err(message) => retour_erreur(tr(&message)),
        }
    });
}
