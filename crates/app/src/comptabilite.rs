//! Export comptable SYSCOHADA (docs/spec-export-syscohada.md, jalon C2) :
//! journal des ventes et des encaissements, clôture d'un mois, TVA collectée,
//! comptes de la rubrique Comptabilité des Paramètres.
//!
//! Les écritures sont calculées sur la copie de ce poste par `kadenz_stockage`.
//! La clôture part au serveur avec l'empreinte du mois ; il la recalcule et
//! refuse si sa copie diffère. Sur un poste seul, tout se fait sur sa base.

use std::cell::RefCell;
use std::rc::Rc;

use chrono::{Datelike, Duration, NaiveDate};
use kadenz_domaine::role::Droit;
use kadenz_domaine::saisie::{montant_en_texte, pourcentage_en_texte};
use kadenz_protocole::{Action, EnCoupure};
use kadenz_stockage::comptabilite::{
    self, ComptesComptables, Journal, JournalComptable, LigneTva, Mois, Periode,
};
use kadenz_stockage::{droits, parametres, replique};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::factures::{date_depuis_texte, date_en_texte};
use crate::referentiels::{modele_lignes, retour_erreur, retour_ok, texte_erreur};
use crate::traduction::{tr, trf};
use crate::{
    ComptabiliteApi, CompteComptableUi, Etat, EtatPartage, FenetrePrincipale, Retour, relie,
};

/// Nombre de mois proposés dans la liste, en plus de « Période libre ».
const MOIS_PROPOSES: usize = 36;

#[derive(Default)]
struct Vue {
    // Rang 0 de la liste : période libre ; ensuite du mois courant vers le passé.
    mois: Vec<Mois>,
    periode: Option<Periode>,
    mois_choisi: Option<Mois>,
    journaux: Vec<Journal>,
    journal: Option<JournalComptable>,
    tva: Vec<LigneTva>,
    periode_tva: Option<Periode>,
    // Comptes en cours de saisie dans les Paramètres.
    comptes: ComptesComptables,
}

type VuePartagee = Rc<RefCell<Vue>>;

fn possede(etat: &Etat, droit: Droit) -> bool {
    etat.utilisateur
        .as_ref()
        .is_some_and(|u| droits::possede(&etat.conn, u, droit).unwrap_or(false))
}

fn peut_exporter(etat: &Etat) -> bool {
    possede(etat, Droit::ExporterComptabilite)
}

/// Mention de la dernière synchronisation, quand le serveur ne répond pas.
fn mention_synchro(etat: &Etat) -> String {
    if relie::blocage_selon(etat, EnCoupure::Validation).is_none() {
        return String::new();
    }
    match replique::derniere_synchro(&etat.conn) {
        Ok(Some(moment)) => trf(
            "Serveur injoignable : données au {}, dernière synchronisation.",
            &[&moment.format("%d/%m/%Y %H:%M")],
        ),
        Ok(None) => tr("Serveur injoignable : aucune synchronisation encore reçue."),
        Err(e) => texte_erreur(e),
    }
}

/// Mois proposés : le mois courant puis les précédents.
fn mois_proposes(aujourdhui: NaiveDate) -> Vec<Mois> {
    let mut mois = Vec::with_capacity(MOIS_PROPOSES);
    let mut m = Mois::de_date(aujourdhui);
    for _ in 0..MOIS_PROPOSES {
        mois.push(m);
        m = Mois::de_date(m.premier_jour() - Duration::days(1));
    }
    mois
}

fn aujourdhui() -> NaiveDate {
    chrono::Local::now().date_naive()
}

fn libelle_mois(m: &Mois) -> String {
    const NOMS: [&str; 12] = [
        "Janvier",
        "Février",
        "Mars",
        "Avril",
        "Mai",
        "Juin",
        "Juillet",
        "Août",
        "Septembre",
        "Octobre",
        "Novembre",
        "Décembre",
    ];
    let rang = m.premier_jour().month0() as usize;
    format!("{} {}", tr(NOMS[rang]), m.premier_jour().year())
}

// ---------------------------------------------------------------- Journal

/// Moment d'une clôture, gardé en ISO local (« 2026-11-16T06:30:00 »), en JJ/MM/AAAA.
fn date_cloture(le: &str) -> String {
    le.get(..10)
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .map(date_en_texte)
        .unwrap_or_else(|| le.to_string())
}

/// Période choisie : un mois de la liste, ou du et au saisis.
fn periode_choisie(
    vue: &Vue,
    rang: i32,
    du: &str,
    au: &str,
) -> Result<(Periode, Option<Mois>), String> {
    if rang > 0
        && let Some(m) = vue.mois.get(rang as usize - 1)
    {
        return Ok((
            Periode {
                du: m.premier_jour(),
                au: m.dernier_jour(),
            },
            Some(*m),
        ));
    }
    let (Some(du), Some(au)) = (date_depuis_texte(du), date_depuis_texte(au)) else {
        return Err(tr("saisir les dates du et au au format JJ/MM/AAAA"));
    };
    if au < du {
        return Err(tr("la date de fin précède la date de début"));
    }
    Ok((Periode { du, au }, None))
}

fn cellules_journal(j: &JournalComptable) -> Vec<Vec<String>> {
    j.ecritures
        .iter()
        .map(|e| {
            vec![
                e.code_journal.clone(),
                date_en_texte(e.date),
                e.piece.clone(),
                e.compte.clone(),
                e.compte_tiers.clone(),
                e.libelle.clone(),
                if e.debit == 0 {
                    String::new()
                } else {
                    montant_en_texte(e.debit)
                },
                if e.credit == 0 {
                    String::new()
                } else {
                    montant_en_texte(e.credit)
                },
            ]
        })
        .collect()
}

/// Alertes avant export : modes de paiement sans compte, mois non clos.
fn alertes(j: &JournalComptable) -> String {
    let mut lignes = Vec::new();
    if !j.modes_sans_compte.is_empty() {
        lignes.push(trf(
            "Modes de paiement sans compte de trésorerie : {}. Renseignez-les dans Paramètres, rubrique Comptabilité, avant d'exporter.",
            &[&j.modes_sans_compte.join(", ")],
        ));
    }
    if !j.mois_non_clos.is_empty() {
        let mois: Vec<String> = j.mois_non_clos.iter().map(|m| m.to_string()).collect();
        lignes.push(trf(
            "Export provisoire : mois non clos {}.",
            &[&mois.join(", ")],
        ));
    }
    lignes.join("\n")
}

/// Mention de clôture du mois choisi, ou vide s'il ne l'est pas.
fn mention_cloture(etat: &Etat, mois: Option<Mois>) -> String {
    let Some(mois) = mois else {
        return String::new();
    };
    match comptabilite::clotures(&etat.conn) {
        Ok(liste) => liste
            .into_iter()
            .find(|c| c.mois == mois)
            .map(|c| trf("Mois clos le {} par {}.", &[&date_cloture(&c.le), &c.par]))
            .unwrap_or_default(),
        Err(e) => texte_erreur(e),
    }
}

fn afficher_journal(f: &FenetrePrincipale, etat: &Etat, vue: &mut Vue) {
    let api = f.global::<ComptabiliteApi>();
    api.set_synchro(mention_synchro(etat).into());
    let Some(periode) = vue.periode else {
        return;
    };
    match comptabilite::journal(&etat.conn, periode, &vue.journaux) {
        Ok(j) => {
            api.set_journal_lignes(modele_lignes(cellules_journal(&j)));
            api.set_journal_resume(
                trf(
                    "{} écriture(s), {} pièce(s).",
                    &[&j.ecritures.len(), &j.pieces],
                )
                .into(),
            );
            api.set_journal_totaux(
                trf(
                    "Total débit : {} FCFA   Total crédit : {} FCFA",
                    &[
                        &montant_en_texte(j.total_debit),
                        &montant_en_texte(j.total_credit),
                    ],
                )
                .into(),
            );
            api.set_alertes(alertes(&j).into());
            vue.journal = Some(j);
        }
        Err(e) => {
            api.set_journal_lignes(modele_lignes(Vec::new()));
            api.set_journal_resume(texte_erreur(e).into());
            api.set_journal_totaux(SharedString::new());
            api.set_alertes(SharedString::new());
            vue.journal = None;
        }
    }
    let cloture = mention_cloture(etat, vue.mois_choisi);
    api.set_clos(!cloture.is_empty());
    api.set_cloture(cloture.into());
}

/// Journal prêt à remettre au comptable, quel que soit le format : droit
/// d'export, écritures présentes et aucun règlement sans compte de trésorerie.
fn journal_exportable<'a>(
    etat: &Etat,
    vue: &'a Vue,
) -> Result<(&'a JournalComptable, &'a Periode), Retour> {
    if !peut_exporter(etat) {
        return Err(retour_erreur(tr(
            "votre compte n'a pas le droit d'exporter la comptabilité",
        )));
    }
    let (Some(j), Some(p)) = (vue.journal.as_ref(), vue.periode.as_ref()) else {
        return Err(retour_erreur(tr("aucune écriture à exporter")));
    };
    if !j.modes_sans_compte.is_empty() {
        return Err(retour_erreur(tr(
            "des règlements n'ont pas de compte de trésorerie : renseignez leur mode de paiement dans Paramètres, rubrique Comptabilité",
        )));
    }
    if j.ecritures.is_empty() {
        return Err(retour_erreur(tr("aucune écriture à exporter")));
    }
    Ok((j, p))
}

/// Écrit le CSV du journal là où l'utilisateur le choisit.
fn exporter_csv(etat: &Etat, vue: &Vue) -> Retour {
    let (j, p) = match journal_exportable(etat, vue) {
        Ok(jp) => jp,
        Err(retour) => return retour,
    };
    let nom = comptabilite::nom_fichier(p.du, p.au, !j.mois_non_clos.is_empty());
    let Some(chemin) = rfd::FileDialog::new()
        .set_title(tr("Enregistrer le journal"))
        .set_file_name(&nom)
        .add_filter("CSV", &["csv"])
        .save_file()
    else {
        return Retour::default();
    };
    match std::fs::write(&chemin, comptabilite::csv(&j.ecritures)) {
        Ok(()) => retour_ok(trf("Journal enregistré : {}", &[&chemin.display()])),
        Err(erreur) => retour_erreur(trf("écriture du fichier impossible : {}", &[&erreur])),
    }
}

fn exporter_excel(etat: &Etat, vue: &Vue) -> Retour {
    let (j, p) = match journal_exportable(etat, vue) {
        Ok(jp) => jp,
        Err(retour) => return retour,
    };
    let nom = comptabilite::nom_fichier(p.du, p.au, !j.mois_non_clos.is_empty());
    let nom = nom.strip_suffix(".csv").unwrap_or(&nom);
    crate::paiements::exporter(crate::excel::journal_comptable(
        &etat.conn,
        j,
        p,
        nom,
        &mention_synchro(etat),
    ))
}

/// Clôture du mois choisi : en ligne sur un poste relié, sur sa base sinon.
fn clore(etat: &Etat, vue: &Vue) -> Retour {
    if !peut_exporter(etat) {
        return retour_erreur(tr("votre compte n'a pas le droit de clore une période"));
    }
    let Some(mois) = vue.mois_choisi else {
        return retour_erreur(tr("choisir un mois dans la liste pour le clore"));
    };
    let Some(acteur) = etat.utilisateur.as_ref() else {
        return retour_erreur(tr("aucun compte connecté"));
    };
    if let Some(raison) = relie::blocage_selon(etat, EnCoupure::Validation) {
        return retour_erreur(raison);
    }
    let empreinte = match comptabilite::empreinte(&etat.conn, mois) {
        Ok(e) => e,
        Err(e) => return retour_erreur(e),
    };
    let action = Action::ClorePeriode {
        mois: mois.code(),
        empreinte: empreinte.clone(),
    };
    let resultat = match relie::ecrire(etat, action) {
        Some(r) => r.map(|_| ()),
        None => comptabilite::clore(&etat.conn, acteur, mois, &empreinte).map_err(texte_erreur),
    };
    match resultat {
        Ok(()) => retour_ok(trf("Mois {} clos.", &[&mois])),
        Err(e) => retour_erreur(e),
    }
}

// ---------------------------------------------------------------- TVA collectée

fn afficher_tva(f: &FenetrePrincipale, etat: &Etat, vue: &mut Vue) {
    let api = f.global::<ComptabiliteApi>();
    let Some(periode) = vue.periode_tva else {
        return;
    };
    match comptabilite::tva_collectee(&etat.conn, periode) {
        Ok(lignes) => {
            api.set_tva_lignes(modele_lignes(
                lignes
                    .iter()
                    .map(|l| {
                        vec![
                            l.mois.to_string(),
                            format!("{} %", pourcentage_en_texte(l.taux)),
                            montant_en_texte(l.base),
                            montant_en_texte(l.tva),
                        ]
                    })
                    .collect(),
            ));
            let base: i64 = lignes.iter().map(|l| l.base).sum();
            let tva: i64 = lignes.iter().map(|l| l.tva).sum();
            api.set_tva_resume(
                trf(
                    "Total de la période : base {} FCFA, TVA collectée {} FCFA. Les avoirs sont déduits dans le mois de leur propre date.",
                    &[&montant_en_texte(base), &montant_en_texte(tva)],
                )
                .into(),
            );
            vue.tva = lignes;
        }
        Err(e) => {
            api.set_tva_lignes(modele_lignes(Vec::new()));
            api.set_tva_resume(texte_erreur(e).into());
            vue.tva = Vec::new();
        }
    }
}

/// Année saisie (vide : l'année en cours) en période du 1er janvier au 31 décembre.
fn periode_annee(texte: &str) -> Option<Periode> {
    let t = texte.trim();
    let annee = if t.is_empty() {
        aujourdhui().year()
    } else if t.len() == 4 && t.chars().all(|c| c.is_ascii_digit()) {
        t.parse().ok()?
    } else {
        return None;
    };
    Some(Periode {
        du: NaiveDate::from_ymd_opt(annee, 1, 1)?,
        au: NaiveDate::from_ymd_opt(annee, 12, 31)?,
    })
}

// ---------------------------------------------------------------- Comptes

fn compte_ui(libelle: &str, compte: &str, defaut: &str) -> CompteComptableUi {
    CompteComptableUi {
        libelle: tr(libelle).into(),
        compte: compte.into(),
        defaut: defaut.into(),
    }
}

/// Comptes de trésorerie : un par mode de paiement des Paramètres, plus ceux
/// déjà enregistrés pour un mode retiré depuis.
fn modes(etat: &Etat, c: &ComptesComptables) -> Vec<(String, String)> {
    let mut liste: Vec<(String, String)> = parametres::lire(&etat.conn)
        .map(|p| p.modes_paiement)
        .unwrap_or_default()
        .into_iter()
        .map(|m| {
            let compte = c
                .tresorerie
                .iter()
                .find(|(mode, _)| *mode == m)
                .map(|(_, compte)| compte.clone())
                .unwrap_or_default();
            (m, compte)
        })
        .collect();
    for (mode, compte) in &c.tresorerie {
        if !liste.iter().any(|(m, _)| m == mode) {
            liste.push((mode.clone(), compte.clone()));
        }
    }
    liste
}

fn afficher_comptes(f: &FenetrePrincipale, etat: &Etat, vue: &mut Vue) {
    let api = f.global::<ComptabiliteApi>();
    let d = ComptesComptables::default();
    let c = &vue.comptes;
    api.set_comptes(ModelRc::new(VecModel::from(vec![
        compte_ui("Compte clients", &c.clients, &d.clients),
        compte_ui(
            "Ventes de marchandises",
            &c.ventes_marchandises,
            &d.ventes_marchandises,
        ),
        compte_ui("Services vendus", &c.ventes_services, &d.ventes_services),
        compte_ui(
            "TVA facturée sur ventes",
            &c.tva_marchandises,
            &d.tva_marchandises,
        ),
        compte_ui(
            "TVA facturée sur prestations de services",
            &c.tva_services,
            &d.tva_services,
        ),
        compte_ui("Compte d'attente", &c.attente, &d.attente),
    ])));
    let tresorerie = modes(etat, c);
    api.set_comptes_modes(ModelRc::new(VecModel::from(
        tresorerie
            .iter()
            .map(|(mode, compte)| {
                let defaut = d
                    .tresorerie
                    .iter()
                    .find(|(m, _)| m == mode)
                    .map(|(_, c)| c.as_str())
                    .unwrap_or("");
                CompteComptableUi {
                    libelle: mode.as_str().into(),
                    compte: compte.as_str().into(),
                    defaut: defaut.into(),
                }
            })
            .collect::<Vec<_>>(),
    )));
    api.set_journaux(ModelRc::new(VecModel::from(vec![
        compte_ui("Journal des ventes", &c.journal_ventes, ""),
        compte_ui("Journal des encaissements", &c.journal_encaissements, ""),
    ])));
    vue.comptes.tresorerie = tresorerie;
}

fn noter_compte(c: &mut ComptesComptables, liste: i32, ligne: i32, compte: &str) {
    let compte = compte.trim().to_string();
    match (liste, ligne) {
        (0, 0) => c.clients = compte,
        (0, 1) => c.ventes_marchandises = compte,
        (0, 2) => c.ventes_services = compte,
        (0, 3) => c.tva_marchandises = compte,
        (0, 4) => c.tva_services = compte,
        (0, 5) => c.attente = compte,
        (1, l) if l >= 0 => {
            if let Some(m) = c.tresorerie.get_mut(l as usize) {
                m.1 = compte;
            }
        }
        (2, 0) => c.journal_ventes = compte,
        (2, 1) => c.journal_encaissements = compte,
        _ => {}
    }
}

fn enregistrer_comptes(etat: &Etat, c: &ComptesComptables) -> Retour {
    if !peut_exporter(etat) {
        return retour_erreur(tr("votre compte n'a pas le droit de modifier les comptes"));
    }
    let Some(acteur) = etat.utilisateur.as_ref() else {
        return retour_erreur(tr("aucun compte connecté"));
    };
    let action = Action::EnregistrerComptes {
        contenu: comptabilite::comptes_en_json(c),
    };
    let resultat = match relie::ecrire(etat, action) {
        Some(r) => r.map(|_| ()),
        None => comptabilite::enregistrer_comptes(&etat.conn, acteur, c).map_err(texte_erreur),
    };
    match resultat {
        Ok(()) => retour_ok("comptes enregistrés"),
        Err(e) => retour_erreur(e),
    }
}

// ---------------------------------------------------------------- Branchement

pub(crate) fn brancher(fenetre: &FenetrePrincipale, etat: &EtatPartage) {
    let vue: VuePartagee = Rc::default();
    let api = fenetre.global::<ComptabiliteApi>();

    let (f, v) = (fenetre.as_weak(), vue.clone());
    api.on_ouvrir(move || {
        let Some(f) = f.upgrade() else { return };
        let mut vue = v.borrow_mut();
        *vue = Vue {
            mois: mois_proposes(aujourdhui()),
            journaux: vec![Journal::Ventes, Journal::Encaissements],
            ..Vue::default()
        };
        let api = f.global::<ComptabiliteApi>();
        let mut libelles = vec![SharedString::from(tr("Période libre"))];
        libelles.extend(vue.mois.iter().map(|m| SharedString::from(libelle_mois(m))));
        api.set_mois(ModelRc::new(VecModel::from(libelles)));
        // Le mois précédent : le dernier mois entièrement passé, celui qu'on exporte.
        api.set_mois_courant(2);
    });

    let (f, e, v) = (fenetre.as_weak(), etat.clone(), vue.clone());
    api.on_actualiser(move || {
        let Some(f) = f.upgrade() else { return };
        let etat = e.borrow();
        let mut vue = v.borrow_mut();
        afficher_journal(&f, &etat, &mut vue);
        afficher_tva(&f, &etat, &mut vue);
    });

    let (f, e, v) = (fenetre.as_weak(), etat.clone(), vue.clone());
    api.on_filtrer_journal(move |rang, du, au, ventes, encaissements| {
        let Some(f) = f.upgrade() else {
            return SharedString::new();
        };
        let etat = e.borrow();
        let mut vue = v.borrow_mut();
        let (periode, mois) = match periode_choisie(&vue, rang, &du, &au) {
            Ok(p) => p,
            Err(message) => {
                // Période libre incomplète : rien n'est affiché tant qu'elle ne l'est pas.
                vue.periode = None;
                vue.journal = None;
                let api = f.global::<ComptabiliteApi>();
                api.set_journal_lignes(modele_lignes(Vec::new()));
                api.set_journal_resume(SharedString::new());
                api.set_journal_totaux(SharedString::new());
                api.set_alertes(SharedString::new());
                api.set_clos(false);
                api.set_cloture(SharedString::new());
                return if du.trim().is_empty() && au.trim().is_empty() {
                    SharedString::new()
                } else {
                    message.into()
                };
            }
        };
        vue.periode = Some(periode);
        vue.mois_choisi = mois;
        vue.journaux = [
            (ventes, Journal::Ventes),
            (encaissements, Journal::Encaissements),
        ]
        .into_iter()
        .filter_map(|(oui, j)| oui.then_some(j))
        .collect();
        afficher_journal(&f, &etat, &mut vue);
        SharedString::new()
    });

    let (e, v) = (etat.clone(), vue.clone());
    api.on_exporter_csv(move || exporter_csv(&e.borrow(), &v.borrow()));

    let (e, v) = (etat.clone(), vue.clone());
    api.on_exporter_excel(move || exporter_excel(&e.borrow(), &v.borrow()));

    let (f, e, v) = (fenetre.as_weak(), etat.clone(), vue.clone());
    api.on_clore(move || {
        let etat = e.borrow();
        let mut vue = v.borrow_mut();
        let retour = clore(&etat, &vue);
        if let Some(f) = f.upgrade() {
            afficher_journal(&f, &etat, &mut vue);
        }
        retour
    });

    let (f, e, v) = (fenetre.as_weak(), etat.clone(), vue.clone());
    api.on_filtrer_tva(move |annee| {
        let Some(f) = f.upgrade() else {
            return SharedString::new();
        };
        let Some(periode) = periode_annee(&annee) else {
            return tr("année invalide : saisir quatre chiffres, par exemple 2026").into();
        };
        let mut vue = v.borrow_mut();
        vue.periode_tva = Some(periode);
        afficher_tva(&f, &e.borrow(), &mut vue);
        SharedString::new()
    });

    let (f, e, v) = (fenetre.as_weak(), etat.clone(), vue.clone());
    api.on_exporter_tva_pdf(move || {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        let etat = e.borrow();
        let vue = v.borrow();
        let Some(p) = vue.periode_tva.as_ref() else {
            return retour_erreur(tr("saisir une année"));
        };
        let pdf = crate::pdf::tva_collectee(&etat.conn, &vue.tva, p);
        crate::apercu::montrer(&f, &etat, pdf, None)
    });

    let (e, v) = (etat.clone(), vue.clone());
    api.on_exporter_tva_excel(move || {
        let etat = e.borrow();
        let vue = v.borrow();
        let Some(p) = vue.periode_tva.as_ref() else {
            return retour_erreur(tr("saisir une année"));
        };
        crate::paiements::exporter(crate::excel::tva_collectee(&etat.conn, &vue.tva, p))
    });

    let (f, e, v) = (fenetre.as_weak(), etat.clone(), vue.clone());
    api.on_ouvrir_comptes(move || {
        let Some(f) = f.upgrade() else { return };
        let etat = e.borrow();
        let mut vue = v.borrow_mut();
        vue.comptes = comptabilite::comptes(&etat.conn).unwrap_or_default();
        afficher_comptes(&f, &etat, &mut vue);
    });

    let v = vue.clone();
    api.on_noter_compte(move |liste, ligne, compte| {
        noter_compte(&mut v.borrow_mut().comptes, liste, ligne, &compte);
    });

    let (e, v) = (etat.clone(), vue.clone());
    api.on_enregistrer_comptes(move || enregistrer_comptes(&e.borrow(), &v.borrow().comptes));

    let (f, e, v) = (fenetre.as_weak(), etat.clone(), vue.clone());
    api.on_comptes_par_defaut(move || {
        let Some(f) = f.upgrade() else { return };
        let etat = e.borrow();
        let mut vue = v.borrow_mut();
        vue.comptes = ComptesComptables::default();
        afficher_comptes(&f, &etat, &mut vue);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mois_proposes_du_plus_recent_au_plus_ancien() {
        let jour = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let mois = mois_proposes(jour);
        assert_eq!(mois.len(), MOIS_PROPOSES);
        assert_eq!(mois[0].code(), "2026-01");
        assert_eq!(mois[1].code(), "2025-12");
        assert_eq!(mois[12].code(), "2025-01");
    }

    #[test]
    fn date_de_cloture_en_texte() {
        assert_eq!(date_cloture("2026-11-16T06:30:00"), "16/11/2026");
        assert_eq!(date_cloture("illisible"), "illisible");
    }

    #[test]
    fn annee_saisie() {
        let p = periode_annee("2025").unwrap();
        assert_eq!(p.du, NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
        assert_eq!(p.au, NaiveDate::from_ymd_opt(2025, 12, 31).unwrap());
        assert!(periode_annee("25").is_none());
        assert!(periode_annee("deux").is_none());
        assert!(periode_annee("").is_some());
    }

    #[test]
    fn textes_de_la_comptabilite_traduits() {
        let manquants = crate::traduction::sans_traduction(comptabilite::MODELES_ERREUR);
        assert!(manquants.is_empty(), "sans traduction : {manquants:?}");
    }

    #[test]
    fn comptes_notes_par_rang() {
        let mut c = ComptesComptables {
            tresorerie: vec![("Espèce".into(), String::new())],
            ..ComptesComptables::default()
        };
        noter_compte(&mut c, 0, 0, " 4111 ");
        noter_compte(&mut c, 1, 0, "5711");
        noter_compte(&mut c, 2, 1, "EN");
        noter_compte(&mut c, 1, 9, "999");
        assert_eq!(c.clients, "4111");
        assert_eq!(c.tresorerie[0].1, "5711");
        assert_eq!(c.journal_encaissements, "EN");
    }
}
