//! Tableau de bord (section 9) : indicateurs des ventes et des achats de la
//! période choisie, calculés par le stockage, mis en forme pour l'écran.

use std::cell::RefCell;
use std::rc::Rc;

use chrono::{Datelike, Local, NaiveDate};
use kadenz_domaine::calcul::Montant;
use kadenz_domaine::paiement::EtatPaiement;
use kadenz_domaine::role::Droit;
use kadenz_domaine::saisie;
use kadenz_stockage::documents::TypeDocument;
use kadenz_stockage::droits;
use kadenz_stockage::pilotage::{self, Indicateurs};
use slint::{Color, ComponentHandle, ModelRc, SharedString, VecModel};

use crate::factures::{date_depuis_texte, date_en_texte, page_du_type};
use crate::graphiques::{arcs_anneau, courbe, parts, proportions};
use crate::referentiels::{retour_erreur, texte_erreur};
use crate::traduction::{tr, trf};
use crate::{
    BarreUi, EtatPartage, FactureRecenteUi, FenetrePrincipale, PilotageApi, PointUi, Retour,
    SecteurUi,
};

/// Couleurs des secteurs par mode de paiement, reprises dans l'ordre puis en boucle.
const PALETTE: [u32; 8] = [
    0x2d3a8c, 0xe3a23b, 0x067647, 0x2e90fa, 0x7a5af8, 0x0e9384, 0xc11574, 0x98a2b3,
];

fn couleur(rgb: u32) -> Color {
    Color::from_rgb_u8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

fn couleur_etat(e: EtatPaiement) -> Color {
    match e {
        EtatPaiement::Impaye => couleur(0xb42318),
        EtatPaiement::Partiel => couleur(0xa8650f),
        EtatPaiement::Paye => couleur(0x067647),
    }
}

fn fcfa(m: Montant) -> String {
    trf("{} FCFA", &[&saisie::montant_en_texte(m)])
}

fn mois(d: NaiveDate) -> String {
    format!("{:02}/{}", d.month(), d.year())
}

fn premier_janvier(annee: i32) -> NaiveDate {
    NaiveDate::from_ymd_opt(annee, 1, 1).unwrap_or_default()
}

fn trente_et_un_decembre(annee: i32) -> NaiveDate {
    NaiveDate::from_ymd_opt(annee, 12, 31).unwrap_or_default()
}

/// Années proposées par le filtre : celles des documents, plus l'année en
/// cours, de la plus récente à la plus ancienne.
fn annees_du_filtre(mut annees: Vec<i32>, annee_en_cours: i32) -> Vec<i32> {
    if !annees.contains(&annee_en_cours) {
        annees.push(annee_en_cours);
    }
    annees.sort_unstable_by(|a, b| b.cmp(a));
    annees.dedup();
    annees
}

/// Période affichée. Les dates saisies priment sur l'année choisie (rang 0 :
/// toutes les années) ; une date absente prend la borne de toutes les années.
fn periode(
    annees: &[i32],
    rang: i32,
    du: Option<NaiveDate>,
    au: Option<NaiveDate>,
) -> (NaiveDate, NaiveDate) {
    let plus_ancienne = annees.iter().copied().min().unwrap_or_default();
    let plus_recente = annees.iter().copied().max().unwrap_or_default();
    if du.is_some() || au.is_some() {
        return (
            du.unwrap_or_else(|| premier_janvier(plus_ancienne)),
            au.unwrap_or_else(|| trente_et_un_decembre(plus_recente)),
        );
    }
    match usize::try_from(rang - 1).ok().and_then(|r| annees.get(r)) {
        Some(a) => (premier_janvier(*a), trente_et_un_decembre(*a)),
        None => (
            premier_janvier(plus_ancienne),
            trente_et_un_decembre(plus_recente),
        ),
    }
}

/// Secteurs d'un anneau, dans l'ordre des valeurs ; les valeurs nulles gardent
/// leur ligne de légende (à 0 %), sans arc.
fn secteurs(libelles: Vec<String>, valeurs: &[Montant], couleurs: Vec<Color>) -> Vec<SecteurUi> {
    let arcs = arcs_anneau(valeurs);
    let parts = parts(valeurs);
    libelles
        .into_iter()
        .zip(valeurs)
        .zip(arcs)
        .zip(parts)
        .zip(couleurs)
        .map(|((((libelle, valeur), arc), part), couleur)| SecteurUi {
            libelle: libelle.into(),
            valeur: fcfa(*valeur).into(),
            part: format!("{part} %").into(),
            couleur,
            arc: arc.into(),
        })
        .collect()
}

fn modele<T: Clone + 'static>(v: Vec<T>) -> ModelRc<T> {
    ModelRc::new(VecModel::from(v))
}

fn afficher(api: &PilotageApi, i: &Indicateurs, du: NaiveDate, au: NaiveDate) {
    let v = &i.ventes;
    api.set_periode(
        trf(
            "Période du {} au {}.",
            &[&date_en_texte(du), &date_en_texte(au)],
        )
        .into(),
    );
    api.set_ca_ht(fcfa(v.ca_ht).into());
    api.set_tva(fcfa(v.tva).into());
    api.set_ca_ttc(fcfa(v.ca_ttc).into());
    api.set_paye(fcfa(v.paye).into());
    api.set_impaye(fcfa(v.impaye).into());

    // États : aucun secteur tant qu'il n'y a pas de chiffre d'affaires.
    let valeurs_etat: Vec<Montant> = v.par_etat.iter().map(|(_, m)| *m).collect();
    let par_etat = if valeurs_etat.iter().all(|m| *m <= 0) {
        Vec::new()
    } else {
        secteurs(
            v.par_etat.iter().map(|(e, _)| tr(&e.to_string())).collect(),
            &valeurs_etat,
            v.par_etat.iter().map(|(e, _)| couleur_etat(*e)).collect(),
        )
    };
    api.set_par_etat(modele(par_etat));
    let part_payee = parts(&[v.paye, v.impaye]).first().copied().unwrap_or(0);
    api.set_part_payee(
        if v.ca_ttc > 0 {
            format!("{part_payee} %")
        } else {
            "-".to_string()
        }
        .into(),
    );

    let valeurs_mode: Vec<Montant> = v.par_mode.iter().map(|(_, m)| *m).collect();
    api.set_par_mode(modele(secteurs(
        v.par_mode.iter().map(|(m, _)| m.clone()).collect(),
        &valeurs_mode,
        (0..v.par_mode.len())
            .map(|r| couleur(PALETTE[r % PALETTE.len()]))
            .collect(),
    )));
    api.set_total_modes(saisie::montant_en_texte(valeurs_mode.iter().sum()).into());

    // Tendance : rien à tracer sans facture sur la période.
    let valeurs_tendance: Vec<Montant> = v.tendance.iter().map(|(_, m)| *m).collect();
    if valeurs_tendance.iter().all(|m| *m == 0) {
        api.set_trace(SharedString::new());
        api.set_aire(SharedString::new());
        api.set_points(modele(Vec::<PointUi>::new()));
        api.set_tendance_max(SharedString::new());
    } else {
        let c = courbe(&valeurs_tendance);
        api.set_trace(c.trace.into());
        api.set_aire(c.aire.into());
        api.set_points(modele(
            c.points
                .iter()
                .zip(&v.tendance)
                .map(|((x, y), (d, m))| PointUi {
                    x: *x,
                    y: *y,
                    texte: trf("{} : {}", &[&mois(*d), &fcfa(*m)]).into(),
                })
                .collect(),
        ));
        let max = valeurs_tendance.iter().copied().max().unwrap_or(0);
        api.set_tendance_max(trf("Plus haut : {}", &[&fcfa(max)]).into());
    }
    api.set_tendance_debut(
        v.tendance
            .first()
            .map(|(d, _)| mois(*d))
            .unwrap_or_default()
            .into(),
    );
    api.set_tendance_fin(
        v.tendance
            .last()
            .map(|(d, _)| mois(*d))
            .unwrap_or_default()
            .into(),
    );

    let valeurs_clients: Vec<Montant> = v.top_clients.iter().map(|(_, m)| *m).collect();
    api.set_top_clients(modele(
        v.top_clients
            .iter()
            .zip(proportions(&valeurs_clients))
            .map(|((nom, m), proportion)| BarreUi {
                libelle: nom.as_str().into(),
                valeur: fcfa(*m).into(),
                proportion,
            })
            .collect(),
    ));

    api.set_dernieres(modele(
        v.dernieres_factures
            .iter()
            .map(|f| FactureRecenteUi {
                numero: f.numero.as_str().into(),
                date: date_en_texte(f.date).into(),
                tiers: f.tiers.as_str().into(),
                montant: fcfa(f.montant_du).into(),
                etat: tr(&f.etat.to_string()).into(),
                couleur: couleur_etat(f.etat),
            })
            .collect(),
    ));

    api.set_achats_total(fcfa(i.achats.total).into());
    api.set_achats_paye(fcfa(i.achats.paye).into());
    api.set_achats_reste(fcfa(i.achats.reste).into());
}

pub(crate) fn brancher(fenetre: &FenetrePrincipale, etat: &EtatPartage) {
    let api = fenetre.global::<PilotageApi>();
    // Factures affichées dans « 4 dernières factures », pour les ouvrir.
    let dernieres = Rc::new(RefCell::new(Vec::<i64>::new()));

    let (f, e, l) = (fenetre.as_weak(), etat.clone(), dernieres.clone());
    api.on_filtrer(move |rang, du, au| {
        let Some(f) = f.upgrade() else {
            return SharedString::new();
        };
        let api = f.global::<PilotageApi>();
        let etat = e.borrow();
        let autorise = match &etat.utilisateur {
            Some(u) => droits::possede(&etat.conn, u, Droit::Consulter).unwrap_or(false),
            None => false,
        };
        if !autorise {
            return tr("Votre compte n'a pas le droit de consulter le tableau de bord.").into();
        }
        let annees = match pilotage::annees(&etat.conn) {
            Ok(a) => annees_du_filtre(a, Local::now().year()),
            Err(erreur) => return texte_erreur(erreur).into(),
        };
        let mut liste = vec![tr("Toutes les années")];
        liste.extend(annees.iter().map(|a| a.to_string()));
        api.set_annees(modele(liste.into_iter().map(SharedString::from).collect()));

        let lire = |texte: &str| (!texte.trim().is_empty()).then(|| date_depuis_texte(texte));
        let (du, au) = (lire(&du), lire(&au));
        match (du, au) {
            (Some(None), Some(None)) => {
                return tr("Dates de début et de fin de période incomplètes ou illisibles (JJ/MM/AAAA) : non appliquées.").into();
            }
            (Some(None), _) => {
                return tr("Date de début de période incomplète ou illisible (JJ/MM/AAAA) : non appliquée.").into();
            }
            (_, Some(None)) => {
                return tr("Date de fin de période incomplète ou illisible (JJ/MM/AAAA) : non appliquée.").into();
            }
            _ => {}
        }
        let (debut, fin) = periode(&annees, rang, du.flatten(), au.flatten());
        if debut > fin {
            return tr("La date de début est après la date de fin : période non appliquée.").into();
        }
        match pilotage::indicateurs(&etat.conn, debut, fin) {
            Ok(i) => {
                *l.borrow_mut() = i.ventes.dernieres_factures.iter().map(|f| f.id).collect();
                afficher(&api, &i, debut, fin);
                SharedString::new()
            }
            Err(erreur) => texte_erreur(erreur).into(),
        }
    });

    let (f, e, l) = (fenetre.as_weak(), etat.clone(), dernieres);
    api.on_ouvrir_facture(move |ligne| {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        let id = usize::try_from(ligne)
            .ok()
            .and_then(|r| l.borrow().get(r).copied());
        let Some(id) = id else {
            return retour_erreur(tr("choisir une facture dans la liste"));
        };
        e.borrow_mut().document_a_ouvrir = Some(id);
        f.set_page(page_du_type(TypeDocument::Facture));
        Retour::default()
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    #[test]
    fn annees_avec_l_annee_en_cours() {
        assert_eq!(
            annees_du_filtre(vec![2024, 2025], 2026),
            vec![2026, 2025, 2024]
        );
        assert_eq!(annees_du_filtre(vec![2026, 2024], 2026), vec![2026, 2024]);
        assert_eq!(annees_du_filtre(Vec::new(), 2026), vec![2026]);
    }

    #[test]
    fn periode_selon_le_filtre() {
        let annees = [2026, 2025, 2024];
        // Toutes les années.
        assert_eq!(
            periode(&annees, 0, None, None),
            (d(2024, 1, 1), d(2026, 12, 31))
        );
        // Une année.
        assert_eq!(
            periode(&annees, 2, None, None),
            (d(2025, 1, 1), d(2025, 12, 31))
        );
        // Les dates saisies priment sur l'année choisie.
        assert_eq!(
            periode(&annees, 2, Some(d(2026, 3, 1)), None),
            (d(2026, 3, 1), d(2026, 12, 31))
        );
        assert_eq!(
            periode(&annees, 0, None, Some(d(2025, 6, 30))),
            (d(2024, 1, 1), d(2025, 6, 30))
        );
    }
}
