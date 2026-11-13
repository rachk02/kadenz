//! Écrans Tiers et Produits : liste filtrée et triée, fiche sélectionnée.
//!
//! La liste affichée est gardée ici avec l'identifiant de la fiche sélectionnée :
//! un clic sur une ligne est converti en identifiant tout de suite, et chaque
//! enregistrement porte sur cet identifiant, jamais sur un rang de ligne.

use std::cmp::Ordering;

use kadenz_domaine::saisie;
use kadenz_domaine::texte::cle_comparaison;
use kadenz_protocole::{Action, Bilan, Entite, Issue};
use kadenz_stockage::produits::{self, FicheProduit, FiltreProduits, Produit, Sens};
use kadenz_stockage::tiers::{self, FicheTiers, FiltreTiers, Tiers, TypeTiers};
use kadenz_stockage::{Erreur, Retrait, synchro};
use slint::{ComponentHandle, ModelRc, SharedString, StandardListViewItem, VecModel};

use crate::traduction::{tr, trf};

use crate::{
    EtatPartage, FenetrePrincipale, FicheProduitUi, FicheTiersUi, ProduitsApi, Retour, TiersApi,
    phrase, relie,
};

/// Liste affichée d'un référentiel et fiche sélectionnée.
pub(crate) struct Vue<T, F> {
    pub lignes: Vec<T>,
    pub selection: Option<i64>,
    pub filtre: F,
    pub tri: Option<(i32, bool)>,
}

impl<T, F: Default> Default for Vue<T, F> {
    fn default() -> Self {
        Vue {
            lignes: Vec::new(),
            selection: None,
            filtre: F::default(),
            tri: None,
        }
    }
}

pub(crate) fn retour_ok(message: impl std::fmt::Display + 'static) -> Retour {
    Retour {
        ok: true,
        message: phrase(message),
    }
}

pub(crate) fn retour_erreur(message: impl std::fmt::Display + 'static) -> Retour {
    // Un refus composé du stockage est traduit par son modèle et ses valeurs.
    let texte = match (&message as &dyn std::any::Any).downcast_ref::<Erreur>() {
        Some(erreur) => texte_erreur_ref(erreur),
        None => message.to_string(),
    };
    Retour {
        ok: false,
        message: phrase(texte),
    }
}

/// Texte d'une erreur du stockage, traduit : un refus composé l'est par son modèle,
/// chaque valeur passant par `tr` (un statut ou un type se traduit, un numéro, un
/// nom ou un montant reste tel quel) ; les autres erreurs par leur texte entier.
pub(crate) fn texte_erreur(erreur: Erreur) -> String {
    texte_erreur_ref(&erreur)
}

pub(crate) fn texte_erreur_ref(erreur: &Erreur) -> String {
    match erreur {
        // La requête et le message de SQLite ne s'affichent pas : un message court,
        // le détail dans erreurs.log.
        Erreur::Sqlite(_) => {
            crate::consigner_erreur(&erreur.to_string());
            tr("erreur de base de données ; le détail est enregistré dans le fichier erreurs.log")
        }
        Erreur::ValidationComposee { modele, valeurs } => {
            let valeurs: Vec<String> = valeurs.iter().map(|v| tr(v)).collect();
            let arguments: Vec<&dyn std::fmt::Display> = valeurs
                .iter()
                .map(|v| v as &dyn std::fmt::Display)
                .collect();
            trf(modele, &arguments)
        }
        // Display composé par format! : recomposé ici pour être traduit.
        Erreur::VersionSchemaInconnue { base, application } => trf(
            "la base est au schéma {}, cette version de l'application ne connaît que le schéma {}",
            &[base, application],
        ),
        // Le libellé du droit est traduit à part, le modèle aussi.
        Erreur::Interdit(droit) => trf(
            kadenz_stockage::synchro::MODELE_INTERDIT,
            &[&tr(&droit.to_string())],
        ),
        autre => tr(&autre.to_string()),
    }
}

/// Lignes d'un tableau. De droite à gauche, les colonnes sont déclarées dans
/// l'ordre inverse côté Slint : les cellules de chaque ligne le sont aussi.
pub(crate) fn modele_lignes(lignes: Vec<Vec<String>>) -> ModelRc<ModelRc<StandardListViewItem>> {
    let inverser = crate::traduction::droite_a_gauche();
    let lignes: Vec<ModelRc<StandardListViewItem>> = lignes
        .into_iter()
        .map(|mut cellules| {
            if inverser {
                cellules.reverse();
            }
            let cellules: Vec<StandardListViewItem> = cellules
                .into_iter()
                .map(|c| StandardListViewItem::from(SharedString::from(c)))
                .collect();
            ModelRc::new(VecModel::from(cellules))
        })
        .collect();
    ModelRc::new(VecModel::from(lignes))
}

fn resume(nombre: usize) -> SharedString {
    match nombre {
        0 => tr("Aucune fiche").into(),
        1 => tr("1 fiche").into(),
        n => trf("{} fiches", &[&n]).into(),
    }
}

fn etat_fiche(actif: bool) -> String {
    if actif {
        String::new()
    } else {
        tr("Désactivée")
    }
}

fn rang<T: Copy + PartialEq>(tous: &[T], valeur: T) -> i32 {
    tous.iter().position(|v| *v == valeur).unwrap_or(0) as i32
}

fn depuis_rang<T: Copy>(tous: &[T], rang: i32) -> T {
    tous[usize::try_from(rang).unwrap_or(0).min(tous.len() - 1)]
}

/// Rang de ligne de la fiche sélectionnée, ou -1 si elle n'est pas affichée.
fn ligne_de(selection: Option<i64>, ids: impl Iterator<Item = i64>) -> i32 {
    let Some(id) = selection else { return -1 };
    ids.enumerate()
        .find(|(_, i)| *i == id)
        .map_or(-1, |(rang, _)| rang as i32)
}

fn dans_l_ordre(ordre: Ordering, croissant: bool) -> Ordering {
    if croissant { ordre } else { ordre.reverse() }
}

// ---------------------------------------------------------------- Tiers

fn cellules_tiers(t: &Tiers) -> Vec<String> {
    let f = &t.fiche;
    vec![
        f.nom.clone(),
        tr(type_court(f.type_tiers)),
        f.telephone.clone(),
        f.localisation.clone(),
        f.ifu.clone(),
        etat_fiche(t.actif),
    ]
}

/// Libellé du type dans la liste, plus court que celui de la fiche.
fn type_court(t: TypeTiers) -> &'static str {
    match t {
        TypeTiers::Client => "Client",
        TypeTiers::Fournisseur => "Fournisseur",
        TypeTiers::LesDeux => "Les deux",
    }
}

fn trier_tiers(lignes: &mut [Tiers], colonne: i32, croissant: bool) {
    let cle = |t: &Tiers| -> String {
        let f = &t.fiche;
        match colonne {
            1 => type_court(f.type_tiers).into(),
            2 => f.telephone.clone(),
            3 => cle_comparaison(&f.localisation),
            4 => f.ifu.clone(),
            5 => etat_fiche(t.actif),
            _ => cle_comparaison(&f.nom),
        }
    };
    lignes.sort_by_cached_key(cle);
    if !croissant {
        lignes.reverse();
    }
}

fn fiche_tiers_ui(t: Option<&Tiers>, type_par_defaut: TypeTiers) -> FicheTiersUi {
    match t {
        Some(t) => {
            let f = &t.fiche;
            FicheTiersUi {
                nom: f.nom.as_str().into(),
                adresse: f.adresse.as_str().into(),
                telephone: f.telephone.as_str().into(),
                localisation: f.localisation.as_str().into(),
                ifu: f.ifu.as_str().into(),
                rccm: f.rccm.as_str().into(),
                regime: f.regime_imposition.as_str().into(),
                type_tiers: rang(&TypeTiers::TOUS, f.type_tiers),
                actif: t.actif,
                existe: true,
            }
        }
        None => FicheTiersUi {
            type_tiers: rang(&TypeTiers::TOUS, type_par_defaut),
            actif: true,
            ..Default::default()
        },
    }
}

fn fiche_tiers_depuis_ui(u: &FicheTiersUi) -> FicheTiers {
    FicheTiers {
        nom: u.nom.to_string(),
        adresse: u.adresse.to_string(),
        telephone: u.telephone.to_string(),
        localisation: u.localisation.to_string(),
        ifu: u.ifu.to_string(),
        rccm: u.rccm.to_string(),
        regime_imposition: u.regime.to_string(),
        type_tiers: depuis_rang(&TypeTiers::TOUS, u.type_tiers),
    }
}

pub(crate) fn afficher_tiers(fenetre: &FenetrePrincipale, etat: &mut crate::Etat) {
    let api = fenetre.global::<TiersApi>();
    match tiers::lister(&etat.conn, &etat.tiers.filtre) {
        Ok(mut lignes) => {
            if let Some((colonne, croissant)) = etat.tiers.tri {
                trier_tiers(&mut lignes, colonne, croissant);
            }
            api.set_lignes(modele_lignes(lignes.iter().map(cellules_tiers).collect()));
            api.set_resume(resume(lignes.len()));
            api.set_ligne_courante(ligne_de(etat.tiers.selection, lignes.iter().map(|t| t.id)));
            etat.tiers.lignes = lignes;
        }
        Err(e) => api.set_resume(phrase(e)),
    }
}

/// Type proposé pour une nouvelle fiche : celui du filtre en cours, sinon Client.
fn type_par_defaut(filtre: &FiltreTiers) -> TypeTiers {
    filtre.type_tiers.unwrap_or(TypeTiers::Client)
}

fn brancher_tiers(fenetre: &FenetrePrincipale, etat: &EtatPartage) {
    let api = fenetre.global::<TiersApi>();

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_ouvrir(move || {
        let mut etat = e.borrow_mut();
        etat.tiers = Vue::default();
        if let Some(f) = f.upgrade() {
            afficher_tiers(&f, &mut etat);
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_filtrer(move |texte, type_tiers, inactifs| {
        let mut etat = e.borrow_mut();
        etat.tiers.filtre = FiltreTiers {
            texte: texte.to_string(),
            type_tiers: match type_tiers {
                1 => Some(TypeTiers::Client),
                2 => Some(TypeTiers::Fournisseur),
                3 => Some(TypeTiers::LesDeux),
                _ => None,
            },
            inclure_inactifs: inactifs,
        };
        if let Some(f) = f.upgrade() {
            afficher_tiers(&f, &mut etat);
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_trier(move |colonne, croissant| {
        let mut etat = e.borrow_mut();
        etat.tiers.tri = Some((colonne, croissant));
        if let Some(f) = f.upgrade() {
            afficher_tiers(&f, &mut etat);
        }
    });

    let e = etat.clone();
    api.on_selectionner(move |ligne| {
        let mut etat = e.borrow_mut();
        let choisi = usize::try_from(ligne)
            .ok()
            .and_then(|i| etat.tiers.lignes.get(i))
            .cloned();
        etat.tiers.selection = choisi.as_ref().map(|t| t.id);
        fiche_tiers_ui(choisi.as_ref(), type_par_defaut(&etat.tiers.filtre))
    });

    // Export Excel des fiches affichées (jalon J10).
    let e = etat.clone();
    api.on_exporter(move || {
        let etat = e.borrow();
        crate::paiements::exporter(crate::excel::tiers(&etat.conn, &etat.tiers.lignes))
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_nouvelle(move || {
        let mut etat = e.borrow_mut();
        etat.tiers.selection = None;
        if let Some(f) = f.upgrade() {
            f.global::<TiersApi>().set_ligne_courante(-1);
        }
        fiche_tiers_ui(None, type_par_defaut(&etat.tiers.filtre))
    });

    let e = etat.clone();
    api.on_fiche_courante(move || {
        let mut etat = e.borrow_mut();
        let fiche = match etat.tiers.selection {
            Some(id) => tiers::lire(&etat.conn, id).ok().flatten(),
            None => None,
        };
        if fiche.is_none() {
            etat.tiers.selection = None;
        }
        fiche_tiers_ui(fiche.as_ref(), type_par_defaut(&etat.tiers.filtre))
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_enregistrer(move |saisie| {
        let mut etat = e.borrow_mut();
        let Some(acteur) = etat.utilisateur.clone() else {
            return retour_erreur("aucune session ouverte");
        };
        let fiche = fiche_tiers_depuis_ui(&saisie);
        let pour_serveur = synchro::tiers_pour_serveur(&fiche);
        let resultat = match etat.tiers.selection {
            Some(id) => {
                match relie::ecrire_sur(&etat, Entite::Tiers, id, |uid| Action::ModifierTiers {
                    uid,
                    fiche: pour_serveur,
                }) {
                    Some(r) => r.map(|_| "fiche enregistrée"),
                    None => tiers::modifier(&etat.conn, &acteur, id, &fiche)
                        .map(|()| "fiche enregistrée")
                        .map_err(texte_erreur),
                }
            }
            None => {
                let action = Action::CreerTiers {
                    uid: synchro::nouvel_uid(),
                    fiche: pour_serveur,
                };
                let cree = match relie::ecrire(&etat, action.clone()) {
                    Some(r) => r.map(|_| relie::id_cree(&etat, &action)),
                    None => tiers::creer(&etat.conn, &acteur, &fiche)
                        .map(|t| Some(t.id))
                        .map_err(texte_erreur),
                };
                cree.map(|id| {
                    etat.tiers.selection = id;
                    "tiers ajouté"
                })
            }
        };
        if let Some(f) = f.upgrade() {
            afficher_tiers(&f, &mut etat);
        }
        match resultat {
            Ok(message) => retour_ok(message),
            Err(erreur) => retour_erreur(erreur),
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_retirer(move || {
        let mut etat = e.borrow_mut();
        let (Some(acteur), Some(id)) = (etat.utilisateur.clone(), etat.tiers.selection) else {
            return retour_erreur("aucune fiche sélectionnée");
        };
        let resultat =
            match relie::ecrire_sur(&etat, Entite::Tiers, id, |uid| Action::RetirerTiers { uid }) {
                Some(r) => r.map(retrait_du_serveur),
                None => tiers::retirer(&etat.conn, &acteur, id).map_err(texte_erreur),
            };
        if let Ok(Retrait::Supprimee) = resultat {
            etat.tiers.selection = None;
        }
        if let Some(f) = f.upgrade() {
            afficher_tiers(&f, &mut etat);
        }
        retour_retrait(resultat)
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_reactiver(move || {
        let mut etat = e.borrow_mut();
        let (Some(acteur), Some(id)) = (etat.utilisateur.clone(), etat.tiers.selection) else {
            return retour_erreur("aucune fiche sélectionnée");
        };
        let resultat = match relie::ecrire_sur(&etat, Entite::Tiers, id, |uid| {
            Action::DefinirActifTiers { uid, actif: true }
        }) {
            Some(r) => r.map(|_| ()),
            None => tiers::definir_actif(&etat.conn, &acteur, id, true).map_err(texte_erreur),
        };
        if let Some(f) = f.upgrade() {
            afficher_tiers(&f, &mut etat);
        }
        match resultat {
            Ok(()) => retour_ok("fiche réactivée"),
            Err(erreur) => retour_erreur(erreur),
        }
    });
}

/// Retrait confirmé par le serveur : supprimée, ou seulement désactivée.
fn retrait_du_serveur(issue: Issue) -> Retrait {
    match issue {
        Issue::Acceptee {
            bilan: Some(Bilan::Retrait { supprimee: true }),
            ..
        } => Retrait::Supprimee,
        _ => Retrait::Desactivee,
    }
}

fn retour_retrait(resultat: Result<Retrait, String>) -> Retour {
    match resultat {
        Ok(Retrait::Supprimee) => retour_ok("fiche supprimée"),
        Ok(Retrait::Desactivee) => retour_ok(
            "fiche désactivée : elle est citée dans au moins un document, l'historique reste intact",
        ),
        Err(erreur) => retour_erreur(erreur),
    }
}

// ---------------------------------------------------------------- Produits

fn cellules_produit(p: &Produit) -> Vec<String> {
    let f = &p.fiche;
    vec![
        f.reference.clone(),
        f.designation.clone(),
        saisie::montant_en_texte(f.prix),
        tr(&f.sens.to_string()),
        etat_fiche(p.actif),
    ]
}

fn trier_produits(lignes: &mut [Produit], colonne: i32, croissant: bool) {
    lignes.sort_by(|a, b| {
        let (fa, fb) = (&a.fiche, &b.fiche);
        let ordre = match colonne {
            0 => cle_comparaison(&fa.reference).cmp(&cle_comparaison(&fb.reference)),
            2 => fa.prix.cmp(&fb.prix),
            3 => fa.sens.to_string().cmp(&fb.sens.to_string()),
            4 => a.actif.cmp(&b.actif),
            _ => cle_comparaison(&fa.designation).cmp(&cle_comparaison(&fb.designation)),
        };
        dans_l_ordre(ordre, croissant)
    });
}

fn fiche_produit_ui(p: Option<&Produit>, sens_par_defaut: Sens) -> FicheProduitUi {
    match p {
        Some(p) => {
            let f = &p.fiche;
            FicheProduitUi {
                designation: f.designation.as_str().into(),
                reference: f.reference.as_str().into(),
                prix: saisie::montant_en_texte(f.prix).into(),
                sens: rang(&Sens::TOUS, f.sens),
                actif: p.actif,
                existe: true,
            }
        }
        None => FicheProduitUi {
            sens: rang(&Sens::TOUS, sens_par_defaut),
            actif: true,
            ..Default::default()
        },
    }
}

fn fiche_produit_depuis_ui(u: &FicheProduitUi) -> Result<FicheProduit, String> {
    let prix = saisie::montant_depuis_texte(&u.prix).ok_or_else(|| {
        "prix invalide : saisir un montant entier en FCFA, par exemple 125\u{00A0}000".to_string()
    })?;
    Ok(FicheProduit {
        designation: u.designation.to_string(),
        reference: u.reference.to_string(),
        prix,
        sens: depuis_rang(&Sens::TOUS, u.sens),
    })
}

pub(crate) fn afficher_produits(fenetre: &FenetrePrincipale, etat: &mut crate::Etat) {
    let api = fenetre.global::<ProduitsApi>();
    match produits::lister(&etat.conn, &etat.produits.filtre) {
        Ok(mut lignes) => {
            if let Some((colonne, croissant)) = etat.produits.tri {
                trier_produits(&mut lignes, colonne, croissant);
            }
            api.set_lignes(modele_lignes(lignes.iter().map(cellules_produit).collect()));
            api.set_resume(resume(lignes.len()));
            api.set_ligne_courante(ligne_de(
                etat.produits.selection,
                lignes.iter().map(|p| p.id),
            ));
            etat.produits.lignes = lignes;
        }
        Err(e) => api.set_resume(phrase(e)),
    }
}

fn sens_par_defaut(filtre: &FiltreProduits) -> Sens {
    filtre.sens.unwrap_or(Sens::Vente)
}

fn brancher_produits(fenetre: &FenetrePrincipale, etat: &EtatPartage) {
    let api = fenetre.global::<ProduitsApi>();

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_ouvrir(move || {
        let mut etat = e.borrow_mut();
        etat.produits = Vue::default();
        if let Some(f) = f.upgrade() {
            afficher_produits(&f, &mut etat);
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_filtrer(move |texte, sens, inactifs| {
        let mut etat = e.borrow_mut();
        etat.produits.filtre = FiltreProduits {
            texte: texte.to_string(),
            sens: match sens {
                1 => Some(Sens::Vente),
                2 => Some(Sens::Achat),
                3 => Some(Sens::LesDeux),
                _ => None,
            },
            inclure_inactifs: inactifs,
        };
        if let Some(f) = f.upgrade() {
            afficher_produits(&f, &mut etat);
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_trier(move |colonne, croissant| {
        let mut etat = e.borrow_mut();
        etat.produits.tri = Some((colonne, croissant));
        if let Some(f) = f.upgrade() {
            afficher_produits(&f, &mut etat);
        }
    });

    let e = etat.clone();
    api.on_selectionner(move |ligne| {
        let mut etat = e.borrow_mut();
        let choisi = usize::try_from(ligne)
            .ok()
            .and_then(|i| etat.produits.lignes.get(i))
            .cloned();
        etat.produits.selection = choisi.as_ref().map(|p| p.id);
        fiche_produit_ui(choisi.as_ref(), sens_par_defaut(&etat.produits.filtre))
    });

    // Export Excel des fiches affichées (jalon J10).
    let e = etat.clone();
    api.on_exporter(move || {
        let etat = e.borrow();
        crate::paiements::exporter(crate::excel::produits(&etat.conn, &etat.produits.lignes))
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_nouvelle(move || {
        let mut etat = e.borrow_mut();
        etat.produits.selection = None;
        if let Some(f) = f.upgrade() {
            f.global::<ProduitsApi>().set_ligne_courante(-1);
        }
        fiche_produit_ui(None, sens_par_defaut(&etat.produits.filtre))
    });

    let e = etat.clone();
    api.on_fiche_courante(move || {
        let mut etat = e.borrow_mut();
        let fiche = match etat.produits.selection {
            Some(id) => produits::lire(&etat.conn, id).ok().flatten(),
            None => None,
        };
        if fiche.is_none() {
            etat.produits.selection = None;
        }
        fiche_produit_ui(fiche.as_ref(), sens_par_defaut(&etat.produits.filtre))
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_enregistrer(move |saisie| {
        let mut etat = e.borrow_mut();
        let Some(acteur) = etat.utilisateur.clone() else {
            return retour_erreur("aucune session ouverte");
        };
        let fiche = match fiche_produit_depuis_ui(&saisie) {
            Ok(fiche) => fiche,
            Err(message) => return retour_erreur(message),
        };
        let pour_serveur = synchro::produit_pour_serveur(&fiche);
        let resultat = match etat.produits.selection {
            Some(id) => {
                match relie::ecrire_sur(&etat, Entite::Produit, id, |uid| Action::ModifierProduit {
                    uid,
                    fiche: pour_serveur,
                }) {
                    Some(r) => r.map(|_| "fiche enregistrée"),
                    None => produits::modifier(&etat.conn, &acteur, id, &fiche)
                        .map(|()| "fiche enregistrée")
                        .map_err(texte_erreur),
                }
            }
            None => {
                let action = Action::CreerProduit {
                    uid: synchro::nouvel_uid(),
                    fiche: pour_serveur,
                };
                let cree = match relie::ecrire(&etat, action.clone()) {
                    Some(r) => r.map(|_| relie::id_cree(&etat, &action)),
                    None => produits::creer(&etat.conn, &acteur, &fiche)
                        .map(|p| Some(p.id))
                        .map_err(texte_erreur),
                };
                cree.map(|id| {
                    etat.produits.selection = id;
                    "produit ajouté"
                })
            }
        };
        if let Some(f) = f.upgrade() {
            afficher_produits(&f, &mut etat);
        }
        match resultat {
            Ok(message) => retour_ok(message),
            Err(erreur) => retour_erreur(erreur),
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_retirer(move || {
        let mut etat = e.borrow_mut();
        let (Some(acteur), Some(id)) = (etat.utilisateur.clone(), etat.produits.selection) else {
            return retour_erreur("aucune fiche sélectionnée");
        };
        let resultat = match relie::ecrire_sur(&etat, Entite::Produit, id, |uid| {
            Action::RetirerProduit { uid }
        }) {
            Some(r) => r.map(retrait_du_serveur),
            None => produits::retirer(&etat.conn, &acteur, id).map_err(texte_erreur),
        };
        if let Ok(Retrait::Supprimee) = resultat {
            etat.produits.selection = None;
        }
        if let Some(f) = f.upgrade() {
            afficher_produits(&f, &mut etat);
        }
        retour_retrait(resultat)
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_reactiver(move || {
        let mut etat = e.borrow_mut();
        let (Some(acteur), Some(id)) = (etat.utilisateur.clone(), etat.produits.selection) else {
            return retour_erreur("aucune fiche sélectionnée");
        };
        let resultat = match relie::ecrire_sur(&etat, Entite::Produit, id, |uid| {
            Action::DefinirActifProduit { uid, actif: true }
        }) {
            Some(r) => r.map(|_| ()),
            None => produits::definir_actif(&etat.conn, &acteur, id, true).map_err(texte_erreur),
        };
        if let Some(f) = f.upgrade() {
            afficher_produits(&f, &mut etat);
        }
        match resultat {
            Ok(()) => retour_ok("fiche réactivée"),
            Err(erreur) => retour_erreur(erreur),
        }
    });
}

pub(crate) fn brancher(fenetre: &FenetrePrincipale, etat: &EtatPartage) {
    brancher_tiers(fenetre, etat);
    brancher_produits(fenetre, etat);
}

#[cfg(test)]
mod tests {
    /// Messages d'erreur rendus par le serveur aux postes : six traductions chacun.
    #[test]
    fn textes_de_la_synchronisation_traduits() {
        let manquants =
            crate::traduction::sans_traduction(&kadenz_stockage::synchro::MODELES_ERREUR);
        assert!(manquants.is_empty(), "sans traduction : {manquants:?}");
        // Décisions sur les opérations suspendues (F6).
        let manquants =
            crate::traduction::sans_traduction(&kadenz_stockage::synchro::MODELES_DECISION);
        assert!(manquants.is_empty(), "sans traduction : {manquants:?}");
    }
}
