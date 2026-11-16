//! Écrans du stock (docs/spec-stock.md, jalon S2) : état du stock à une date,
//! mouvements, ajustement, inventaire en deux temps.
//!
//! Le stock est calculé par le serveur seul ; ce poste lit celui de sa copie,
//! donc à la dernière synchronisation. Un ajustement part en file hors
//! connexion ; un inventaire attend le serveur. Sans le droit « Voir la valeur
//! du stock », aucune valeur n'est affichée ni exportée.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use chrono::NaiveDate;
use kadenz_domaine::calcul::{Montant, Quantite};
use kadenz_domaine::role::Droit;
use kadenz_domaine::saisie::{
    montant_depuis_texte, montant_en_texte, quantite_depuis_texte, quantite_en_texte,
};
use kadenz_domaine::stock::NatureMouvement;
use kadenz_domaine::texte::cle_comparaison;
use kadenz_protocole::{Action, EnCoupure, Entite, Issue, LigneInventaire};
use kadenz_stockage::stock::{self, FiltreMouvements, LigneEtat, Mouvement, Stock};
use kadenz_stockage::{droits, replique, synchro};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use crate::factures::{date_depuis_texte, date_en_texte};
use crate::referentiels::{modele_lignes, retour_erreur, retour_ok, texte_erreur};
use crate::traduction::{tr, trf};
use crate::{
    Etat, EtatPartage, FenetrePrincipale, LigneInventaireUi, Page, PilotageApi, ProduitStockUi,
    Retour, StockApi, relie,
};

/// Ligne de l'inventaire en cours, avec ce qui est gardé sur le poste.
#[derive(Debug, Clone)]
struct LigneCompte {
    produit_id: i64,
    reference: String,
    designation: String,
    theorique: Quantite,
    comptee: Option<Quantite>,
    cout: Option<Montant>,
}

/// Ce que montrent les trois onglets, gardé entre deux actions.
#[derive(Default)]
struct Vue {
    // État du stock.
    etat: Vec<LigneEtat>,
    texte: String,
    negatifs: bool,
    au: Option<NaiveDate>,
    tri: Option<(i32, bool)>,
    selection: Option<i64>,
    // Mouvements : produits suivis proposés (le rang 0 est « tous »).
    produits: Vec<i64>,
    filtre: FiltreMouvements,
    mouvements: Vec<Mouvement>,
    mouvement: Option<usize>,
    // Référence de chaque produit suivi, pour la colonne des mouvements.
    references: HashMap<i64, String>,
    // Inventaire.
    inventaire: Vec<LigneCompte>,
    modele_inventaire: Option<Rc<VecModel<LigneInventaireUi>>>,
}

type VuePartagee = Rc<RefCell<Vue>>;

/// Quantité signée, en millièmes, mise en texte (`quantite_en_texte` perd le
/// signe d'une quantité entre -1 et 0).
pub(crate) fn texte_quantite(q: Quantite) -> String {
    if q < 0 {
        format!("-{}", quantite_en_texte(-q))
    } else {
        quantite_en_texte(q)
    }
}

/// Quantité signée saisie : « 5 », « +5 », « -2,5 ».
fn quantite_signee(texte: &str) -> Option<Quantite> {
    let t = texte.trim();
    match t.strip_prefix('-') {
        Some(reste) => quantite_depuis_texte(reste.trim()).map(|q| -q),
        None => quantite_depuis_texte(t.strip_prefix('+').unwrap_or(t).trim()),
    }
}

/// Quantité comptée à l'inventaire : comme une quantité de ligne, mais zéro
/// (« 0 », « 0,0 ») est permis, pour un produit absent du rayon.
fn quantite_comptee(texte: &str) -> Option<Quantite> {
    quantite_depuis_texte(texte).or_else(|| {
        let t = texte.trim();
        let (entier, decimales) = t.split_once([',', '.']).unwrap_or((t, ""));
        let zeros = |s: &str| s.chars().all(|c| c == '0');
        (!entier.is_empty() && zeros(entier) && zeros(decimales) && decimales.len() <= 3)
            .then_some(0)
    })
}

fn possede(etat: &Etat, droit: Droit) -> bool {
    etat.utilisateur
        .as_ref()
        .is_some_and(|u| droits::possede(&etat.conn, u, droit).unwrap_or(false))
}

pub(crate) fn voit_valeur(etat: &Etat) -> bool {
    possede(etat, Droit::VoirValeurStock)
}

/// Mention de la dernière synchronisation, quand le serveur ne répond pas.
fn mention_synchro(etat: &Etat) -> String {
    if relie::blocage_selon(etat, EnCoupure::Validation).is_none() {
        return String::new();
    }
    match replique::derniere_synchro(&etat.conn) {
        Ok(Some(moment)) => trf(
            "Serveur injoignable : quantités au {}, dernière synchronisation.",
            &[&moment.format("%d/%m/%Y %H:%M")],
        ),
        Ok(None) => tr("Serveur injoignable : aucune synchronisation encore reçue."),
        Err(e) => texte_erreur(e),
    }
}

pub(crate) fn libelle_nature(n: NatureMouvement) -> &'static str {
    match n {
        NatureMouvement::Reception => "Réception",
        NatureMouvement::Livraison => "Livraison",
        NatureMouvement::Ajustement => "Ajustement",
        NatureMouvement::Inventaire => "Inventaire",
    }
}

const NATURES: [NatureMouvement; 4] = [
    NatureMouvement::Reception,
    NatureMouvement::Livraison,
    NatureMouvement::Ajustement,
    NatureMouvement::Inventaire,
];

/// Document d'origine d'un mouvement, en texte : type et numéro.
pub(crate) fn texte_document(m: &Mouvement) -> String {
    match &m.document {
        Some(d) => {
            let numero = d.numero.clone().unwrap_or_else(|| tr("brouillon"));
            format!("{} {}", tr(&d.type_document.to_string()), numero)
        }
        None => String::new(),
    }
}

// ---------------------------------------------------------------- État

fn trier(lignes: &mut [LigneEtat], colonne: i32, croissant: bool, valeur: bool) {
    // Sans le droit de voir la valeur, les colonnes 3 et 4 n'existent pas.
    let colonne = if valeur { colonne } else { colonne.min(2) };
    lignes.sort_by(|a, b| {
        let ordre = match colonne {
            0 => cle_comparaison(&a.reference).cmp(&cle_comparaison(&b.reference)),
            2 => a.stock.quantite.cmp(&b.stock.quantite),
            3 => a.stock.dernier_cout.cmp(&b.stock.dernier_cout),
            4 => a.stock.valeur.cmp(&b.stock.valeur),
            _ => cle_comparaison(&a.designation).cmp(&cle_comparaison(&b.designation)),
        };
        if croissant { ordre } else { ordre.reverse() }
    });
}

fn cellules_etat(l: &LigneEtat, valeur: bool) -> Vec<String> {
    let mut cellules = vec![
        l.reference.clone(),
        l.designation.clone(),
        texte_quantite(l.stock.quantite),
    ];
    if valeur {
        cellules.push(montant_en_texte(l.stock.dernier_cout));
        cellules.push(montant_en_texte(l.stock.valeur));
    }
    cellules
}

fn resume_etat(n: usize) -> String {
    match n {
        0 => tr("Aucun produit suivi en stock"),
        1 => tr("1 produit suivi"),
        n => trf("{} produits suivis", &[&n]),
    }
}

fn afficher_etat(f: &FenetrePrincipale, etat: &Etat, vue: &mut Vue) {
    let api = f.global::<StockApi>();
    let valeur = voit_valeur(etat);
    api.set_synchro(mention_synchro(etat).into());
    let mut lignes = match stock::etat(&etat.conn, vue.au) {
        Ok(l) => l,
        Err(e) => {
            api.set_etat_lignes(modele_lignes(Vec::new()));
            api.set_etat_resume(crate::phrase(e));
            api.set_etat_total("".into());
            return;
        }
    };
    // Produits proposés dans l'onglet des mouvements : tous les suivis.
    let mut noms = vec![SharedString::from(tr("Tous les produits suivis"))];
    vue.produits = vec![0];
    vue.references.clear();
    for l in &lignes {
        vue.produits.push(l.produit_id);
        vue.references.insert(l.produit_id, l.reference.clone());
        noms.push(format!("{} - {}", l.reference, l.designation).into());
    }
    api.set_produits(ModelRc::new(VecModel::from(noms)));

    let cle = cle_comparaison(vue.texte.trim());
    lignes.retain(|l| {
        (cle.is_empty()
            || cle_comparaison(&l.reference).contains(&cle)
            || cle_comparaison(&l.designation).contains(&cle))
            && (!vue.negatifs || l.stock.quantite < 0)
    });
    if let Some((colonne, croissant)) = vue.tri {
        trier(&mut lignes, colonne, croissant, valeur);
    }
    api.set_etat_lignes(modele_lignes(
        lignes.iter().map(|l| cellules_etat(l, valeur)).collect(),
    ));
    api.set_etat_resume(resume_etat(lignes.len()).into());
    let total: Montant = lignes.iter().map(|l| l.stock.valeur).sum();
    api.set_etat_total(if valeur && !lignes.is_empty() {
        trf(
            "Valeur du stock affiché : {} FCFA",
            &[&montant_en_texte(total)],
        )
        .into()
    } else {
        SharedString::new()
    });
    let rang = vue
        .selection
        .and_then(|id| lignes.iter().position(|l| l.produit_id == id));
    if rang.is_none() {
        vue.selection = None;
    }
    api.set_etat_ligne_courante(rang.map_or(-1, |r| r as i32));
    vue.etat = lignes;
}

fn produit_ui(etat: &Etat, vue: &Vue) -> ProduitStockUi {
    let Some(l) = vue
        .selection
        .and_then(|id| vue.etat.iter().find(|l| l.produit_id == id))
    else {
        return ProduitStockUi::default();
    };
    let valeur = voit_valeur(etat);
    let en_attente = stock::ajustements_en_attente(&etat.conn)
        .ok()
        .and_then(|m| m.get(&l.produit_id).copied())
        .unwrap_or(0);
    ProduitStockUi {
        libelle: format!("{} - {}", l.reference, l.designation).into(),
        quantite: texte_quantite(l.stock.quantite).into(),
        cout_moyen: montant_en_texte(l.stock.dernier_cout).into(),
        cout_propose: if valeur && l.stock.dernier_cout > 0 {
            montant_en_texte(l.stock.dernier_cout).into()
        } else {
            SharedString::new()
        },
        negatif: l.stock.quantite < 0,
        en_attente: i32::try_from(en_attente).unwrap_or(i32::MAX),
        existe: true,
    }
}

/// Ajustement saisi : quantité signée non nulle, coût d'une entrée, motif.
fn ajuster(etat: &Etat, produit_id: i64, quantite: &str, cout: &str, motif: &str) -> Retour {
    if !possede(etat, Droit::AjusterStock) {
        return retour_erreur(tr("votre compte n'a pas le droit d'ajuster le stock"));
    }
    let Some(quantite) = quantite_signee(quantite).filter(|q| *q != 0) else {
        return retour_erreur(tr(
            "quantité invalide : saisir une quantité non nulle, positive pour une entrée, négative pour une sortie (par exemple 5 ou -2,5)",
        ));
    };
    let motif = motif.trim();
    if motif.is_empty() {
        return retour_erreur(tr("le motif de l'ajustement est obligatoire"));
    }
    // Le coût ne vaut que pour une entrée ; vide, le serveur prend le coût moyen.
    let cout_unitaire = if quantite > 0 && !cout.trim().is_empty() {
        match montant_depuis_texte(cout) {
            Some(c) if c >= 0 => Some(c),
            _ => {
                return retour_erreur(tr(
                    "coût unitaire invalide : saisir un montant entier en FCFA, ou laisser vide pour le coût moyen",
                ));
            }
        }
    } else {
        None
    };
    let produit = match relie::uid(etat, Entite::Produit, produit_id) {
        Ok(uid) => uid,
        Err(e) => return retour_erreur(e),
    };
    let action = Action::AjusterStock {
        uid: synchro::nouvel_uid(),
        produit,
        quantite,
        cout_unitaire,
        motif: motif.to_string(),
    };
    match relie::ecrire_relie(etat, action) {
        Ok(Issue::EnFile) => {
            retour_ok("ajustement mis en file d'envoi : il sera appliqué au retour du serveur")
        }
        Ok(_) => retour_ok("ajustement enregistré"),
        Err(e) => retour_erreur(e),
    }
}

// ---------------------------------------------------------------- Mouvements

/// Numéro du document d'un mouvement, pour l'écran : la nature dit déjà s'il
/// s'agit d'un bon de commande ou d'un bordereau, et le nom du type coupait le
/// numéro dans une colonne étroite.
fn numero_document(m: &Mouvement) -> String {
    match &m.document {
        Some(d) => d.numero.clone().unwrap_or_else(|| tr("brouillon")),
        None => String::new(),
    }
}

fn cellules_mouvement(m: &Mouvement, valeur: bool, reference: &str) -> Vec<String> {
    let mut cellules = vec![
        date_en_texte(m.date),
        reference.to_string(),
        tr(libelle_nature(m.nature)),
        texte_quantite(m.quantite),
    ];
    if valeur {
        cellules.push(montant_en_texte(m.valeur));
    }
    cellules.push(texte_quantite(m.quantite_apres));
    if valeur {
        cellules.push(montant_en_texte(m.valeur_apres));
    }
    cellules.extend([numero_document(m), m.motif.clone(), m.compte.clone()]);
    cellules
}

fn resume_mouvements(n: usize) -> String {
    match n {
        0 => tr("Aucun mouvement"),
        1 => tr("1 mouvement"),
        n => trf("{} mouvements", &[&n]),
    }
}

fn afficher_mouvements(f: &FenetrePrincipale, etat: &Etat, vue: &mut Vue) {
    let api = f.global::<StockApi>();
    let valeur = voit_valeur(etat);
    match stock::mouvements(&etat.conn, &vue.filtre) {
        Ok(m) => {
            api.set_mouvements_lignes(modele_lignes(
                m.iter()
                    .map(|m| {
                        let reference = vue.references.get(&m.produit_id).map_or("", |r| r);
                        cellules_mouvement(m, valeur, reference)
                    })
                    .collect(),
            ));
            api.set_mouvements_resume(resume_mouvements(m.len()).into());
            vue.mouvements = m;
        }
        Err(e) => {
            api.set_mouvements_lignes(modele_lignes(Vec::new()));
            api.set_mouvements_resume(crate::phrase(e));
            vue.mouvements.clear();
        }
    }
    vue.mouvement = None;
    api.set_mouvement_courant(-1);
    api.set_mouvement_a_document(false);
}

// ---------------------------------------------------------------- Inventaire

fn ligne_inventaire_ui(l: &LigneCompte, valeur: bool) -> LigneInventaireUi {
    LigneInventaireUi {
        reference: l.reference.as_str().into(),
        designation: l.designation.as_str().into(),
        theorique: texte_quantite(l.theorique).into(),
        comptee: l.comptee.map(quantite_en_texte).unwrap_or_default().into(),
        ecart: l
            .comptee
            .map(|c| {
                let ecart = c - l.theorique;
                if ecart > 0 {
                    format!("+{}", texte_quantite(ecart))
                } else {
                    texte_quantite(ecart)
                }
            })
            .unwrap_or_default()
            .into(),
        cout: if valeur {
            l.cout.map(montant_en_texte).unwrap_or_default().into()
        } else {
            SharedString::new()
        },
        demande_cout: l.theorique <= 0,
    }
}

fn resume_inventaire(api: &StockApi, vue: &Vue) {
    let comptes = vue
        .inventaire
        .iter()
        .filter(|l| l.comptee.is_some())
        .count();
    api.set_inventaire_en_cours(comptes > 0);
    api.set_inventaire_resume(
        trf(
            "{} produit(s) compté(s) sur {}",
            &[&comptes, &vue.inventaire.len()],
        )
        .into(),
    );
}

fn afficher_inventaire(f: &FenetrePrincipale, etat: &Etat, vue: &mut Vue) {
    let api = f.global::<StockApi>();
    let valeur = voit_valeur(etat);
    api.set_synchro(mention_synchro(etat).into());
    let comptage: HashMap<i64, (Option<Quantite>, Option<Montant>)> =
        match stock::comptage(&etat.conn) {
            Ok(c) => c
                .into_iter()
                .map(|c| (c.produit_id, (c.quantite_comptee, c.cout_unitaire)))
                .collect(),
            Err(e) => {
                api.set_inventaire_resume(crate::phrase(e));
                HashMap::new()
            }
        };
    let lignes = stock::etat(&etat.conn, None).unwrap_or_default();
    vue.inventaire = lignes
        .into_iter()
        .map(|l| {
            let (comptee, cout) = comptage.get(&l.produit_id).copied().unwrap_or_default();
            LigneCompte {
                produit_id: l.produit_id,
                reference: l.reference,
                designation: l.designation,
                theorique: l.stock.quantite,
                comptee,
                cout,
            }
        })
        .collect();
    let modele = Rc::new(VecModel::from(
        vue.inventaire
            .iter()
            .map(|l| ligne_inventaire_ui(l, valeur))
            .collect::<Vec<_>>(),
    ));
    api.set_inventaire(ModelRc::from(modele.clone()));
    vue.modele_inventaire = Some(modele);
    resume_inventaire(&api, vue);
}

/// Ligne saisie gardée sur le poste, puis rafraîchie à l'écran (écart).
fn noter(f: &FenetrePrincipale, etat: &Etat, vue: &mut Vue, rang: usize) -> Result<(), String> {
    let Some(l) = vue.inventaire.get(rang).cloned() else {
        return Ok(());
    };
    stock::noter_comptage(
        &etat.conn,
        l.produit_id,
        l.comptee,
        l.cout.filter(|_| l.comptee.is_some()),
    )
    .map_err(texte_erreur)?;
    // Les champs de saisie gardent leur texte ; l'écart suit.
    if let Some(modele) = &vue.modele_inventaire
        && rang < modele.row_count()
    {
        modele.set_row_data(rang, ligne_inventaire_ui(&l, voit_valeur(etat)));
    }
    resume_inventaire(&f.global::<StockApi>(), vue);
    Ok(())
}

fn valider_inventaire(etat: &Etat, vue: &Vue, motif: &str) -> Retour {
    if !possede(etat, Droit::AjusterStock) {
        return retour_erreur(tr("votre compte n'a pas le droit de saisir un inventaire"));
    }
    if let Some(raison) = relie::blocage_selon(etat, EnCoupure::Validation) {
        return retour_erreur(raison);
    }
    let motif = motif.trim();
    if motif.is_empty() {
        return retour_erreur(tr("le motif de l'inventaire est obligatoire"));
    }
    let mut lignes = Vec::new();
    for l in vue.inventaire.iter().filter(|l| l.comptee.is_some()) {
        let produit = match relie::uid(etat, Entite::Produit, l.produit_id) {
            Ok(uid) => uid,
            Err(e) => return retour_erreur(e),
        };
        lignes.push(LigneInventaire {
            uid: synchro::nouvel_uid(),
            produit,
            quantite_comptee: l.comptee.unwrap_or_default(),
            cout_unitaire: l.cout,
        });
    }
    if lignes.is_empty() {
        return retour_erreur(tr("aucune quantité comptée : rien à valider"));
    }
    let action = Action::SaisirInventaire {
        uid: synchro::nouvel_uid(),
        motif: motif.to_string(),
        lignes,
    };
    match relie::ecrire_relie(etat, action) {
        Ok(Issue::EnFile) => retour_erreur(tr(
            "inventaire non appliqué : le serveur ne répond pas, la saisie reste gardée sur ce poste",
        )),
        Ok(_) => match stock::effacer_comptage(&etat.conn) {
            Ok(()) => retour_ok("inventaire validé"),
            Err(e) => retour_erreur(e),
        },
        Err(e) => retour_erreur(e),
    }
}

// ---------------------------------------------------------------- Autres écrans

/// Produits suivis que la confirmation du bordereau ouvert rendrait négatifs,
/// d'après la copie du poste (spec 3.6) : texte ajouté à la confirmation, ou vide.
pub(crate) fn avertissement_livraison(etat: &Etat, document_id: i64) -> String {
    let Ok(Some(document)) = kadenz_stockage::documents::lire(&etat.conn, document_id) else {
        return String::new();
    };
    let Ok(stocks) = stock::tous(&etat.conn) else {
        return String::new();
    };
    let mut livre: Vec<(i64, String, Quantite)> = Vec::new();
    for l in &document.saisie.lignes {
        let Some(id) = l.produit_id.filter(|id| stocks.contains_key(id)) else {
            continue;
        };
        match livre.iter_mut().find(|(p, _, _)| *p == id) {
            Some((_, _, q)) => *q += l.quantite_livree,
            None => livre.push((id, l.designation.clone(), l.quantite_livree)),
        }
    }
    let manques: Vec<String> = livre
        .iter()
        .filter_map(|(id, designation, q)| {
            let s: &Stock = stocks.get(id)?;
            (*q > 0 && s.quantite - q < 0).then(|| {
                trf(
                    "{} : stock {}, livré {}",
                    &[
                        designation,
                        &texte_quantite(s.quantite),
                        &texte_quantite(*q),
                    ],
                )
            })
        })
        .collect();
    if manques.is_empty() {
        return String::new();
    }
    format!(
        "\n\n{}\n{}",
        tr(
            "Stock insuffisant d'après la dernière synchronisation ; la livraison reste possible, ces produits passeront en stock négatif :"
        ),
        manques.join("\n")
    )
}

/// Tableau de bord : nombre de produits en stock négatif, vide sans produit suivi.
pub(crate) fn indicateur(f: &FenetrePrincipale, etat: &Etat) {
    let texte = match stock::tous(&etat.conn) {
        Ok(tous) if !tous.is_empty() => stock::nombre_negatifs(&etat.conn)
            .map(|n| n.to_string())
            .unwrap_or_default(),
        _ => String::new(),
    };
    f.global::<PilotageApi>().set_stock_negatifs(texte.into());
}

/// Texte du stock sur la fiche d'un produit suivi, selon le droit de voir la valeur.
pub(crate) fn texte_fiche(etat: &Etat, produit_id: i64) -> (String, bool) {
    match stock::lire(&etat.conn, produit_id) {
        Ok(Some(s)) => {
            let texte = if voit_valeur(etat) {
                trf(
                    "En stock : {}   Coût moyen : {} FCFA   Valeur : {} FCFA",
                    &[
                        &texte_quantite(s.quantite),
                        &montant_en_texte(s.dernier_cout),
                        &montant_en_texte(s.valeur),
                    ],
                )
            } else {
                trf("En stock : {}", &[&texte_quantite(s.quantite)])
            };
            (texte, s.quantite < 0)
        }
        _ => (String::new(), false),
    }
}

// ---------------------------------------------------------------- Branchement

pub(crate) fn brancher(fenetre: &FenetrePrincipale, etat: &EtatPartage) {
    let vue: VuePartagee = Rc::default();
    let api = fenetre.global::<StockApi>();

    let (f, e, v) = (fenetre.as_weak(), etat.clone(), vue.clone());
    api.on_ouvrir(move || {
        let Some(f) = f.upgrade() else { return };
        let etat = e.borrow();
        let mut vue = v.borrow_mut();
        // Les onglets se chargent avant la page (ordre des init de Slint) et ont
        // déjà posé leurs filtres et leurs listes : seules les sélections d'une
        // visite précédente sont oubliées. Une remise à zéro complète laissait à
        // l'écran des lignes que la vue ne connaissait plus (comptage perdu,
        // filtres des mouvements et des produits négatifs ignorés).
        vue.selection = None;
        vue.mouvement = None;
        afficher_etat(&f, &etat, &mut vue);
    });

    let (f, e, v) = (fenetre.as_weak(), etat.clone(), vue.clone());
    api.on_actualiser(move || {
        let Some(f) = f.upgrade() else { return };
        let etat = e.borrow();
        let mut vue = v.borrow_mut();
        afficher_etat(&f, &etat, &mut vue);
        afficher_mouvements(&f, &etat, &mut vue);
        // L'inventaire en cours n'est pas rechargé : les quantités théoriques
        // restent celles qui ont servi au comptage.
    });

    let (f, e, v) = (fenetre.as_weak(), etat.clone(), vue.clone());
    api.on_filtrer_etat(move |texte, negatifs, au| {
        let Some(f) = f.upgrade() else {
            return SharedString::new();
        };
        let mut vue = v.borrow_mut();
        vue.texte = texte.to_string();
        vue.negatifs = negatifs;
        let mut erreur = SharedString::new();
        vue.au = if au.trim().is_empty() {
            None
        } else {
            match date_depuis_texte(&au) {
                Some(d) => Some(d),
                None => {
                    erreur = tr("Date illisible (JJ/MM/AAAA) : état du jour affiché.").into();
                    None
                }
            }
        };
        afficher_etat(&f, &e.borrow(), &mut vue);
        erreur
    });

    let (f, e, v) = (fenetre.as_weak(), etat.clone(), vue.clone());
    api.on_trier_etat(move |colonne, croissant| {
        let Some(f) = f.upgrade() else { return };
        let mut vue = v.borrow_mut();
        vue.tri = Some((colonne, croissant));
        afficher_etat(&f, &e.borrow(), &mut vue);
    });

    let (e, v) = (etat.clone(), vue.clone());
    api.on_selectionner(move |ligne| {
        let mut vue = v.borrow_mut();
        vue.selection = usize::try_from(ligne)
            .ok()
            .and_then(|r| vue.etat.get(r))
            .map(|l| l.produit_id);
        produit_ui(&e.borrow(), &vue)
    });

    let (e, v) = (etat.clone(), vue.clone());
    api.on_exporter_etat_pdf(move || {
        let etat = e.borrow();
        let vue = v.borrow();
        match crate::pdf::etat_stock(&etat.conn, &vue.etat, vue.au, voit_valeur(&etat)) {
            Ok((octets, nom)) => crate::paiements::enregistrer_pdf(&octets, &nom, None),
            Err(message) => retour_erreur(tr(&message)),
        }
    });

    let (e, v) = (etat.clone(), vue.clone());
    api.on_exporter_etat_excel(move || {
        let etat = e.borrow();
        let vue = v.borrow();
        crate::paiements::exporter(crate::excel::etat_stock(
            &etat.conn,
            &vue.etat,
            vue.au,
            voit_valeur(&etat),
        ))
    });

    let (f, e, v) = (fenetre.as_weak(), etat.clone(), vue.clone());
    api.on_ajuster(move |quantite, cout, motif| {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        let etat = e.borrow();
        let mut vue = v.borrow_mut();
        let Some(id) = vue.selection else {
            return retour_erreur(tr("choisir un produit dans la liste"));
        };
        let retour = ajuster(&etat, id, &quantite, &cout, &motif);
        if retour.ok {
            afficher_etat(&f, &etat, &mut vue);
        }
        retour
    });

    // Mouvements.
    let (f, e, v) = (fenetre.as_weak(), etat.clone(), vue.clone());
    api.on_filtrer_mouvements(move |produit, du, au, nature| {
        let Some(f) = f.upgrade() else {
            return SharedString::new();
        };
        let mut vue = v.borrow_mut();
        let lire = |t: &str| (!t.trim().is_empty()).then(|| date_depuis_texte(t));
        let (du, au) = (lire(&du), lire(&au));
        let erreur = if matches!(du, Some(None)) || matches!(au, Some(None)) {
            tr("Date illisible (JJ/MM/AAAA) : non appliquée.")
        } else {
            String::new()
        };
        vue.filtre = FiltreMouvements {
            produit: usize::try_from(produit)
                .ok()
                .filter(|r| *r > 0)
                .and_then(|r| vue.produits.get(r).copied()),
            du: du.flatten(),
            au: au.flatten(),
            nature: usize::try_from(nature - 1)
                .ok()
                .and_then(|r| NATURES.get(r).copied()),
        };
        afficher_mouvements(&f, &e.borrow(), &mut vue);
        erreur.into()
    });

    let v = vue.clone();
    api.on_produit_de_la_selection(move || {
        let vue = v.borrow();
        vue.selection
            .and_then(|id| vue.produits.iter().position(|p| *p == id))
            .map_or(0, |r| r as i32)
    });

    let (f, v) = (fenetre.as_weak(), vue.clone());
    api.on_choisir_mouvement(move |ligne| {
        let mut vue = v.borrow_mut();
        vue.mouvement = usize::try_from(ligne)
            .ok()
            .filter(|r| *r < vue.mouvements.len());
        let a_document = vue
            .mouvement
            .and_then(|r| vue.mouvements.get(r))
            .is_some_and(|m| m.document.is_some());
        if let Some(f) = f.upgrade() {
            f.global::<StockApi>().set_mouvement_a_document(a_document);
        }
    });

    let (f, e, v) = (fenetre.as_weak(), etat.clone(), vue.clone());
    api.on_ouvrir_document(move || {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        let document = {
            let vue = v.borrow();
            vue.mouvement
                .and_then(|r| vue.mouvements.get(r))
                .and_then(|m| m.document.clone())
        };
        let Some(d) = document else {
            return retour_erreur(tr("ce mouvement n'a pas de document d'origine"));
        };
        e.borrow_mut().document_a_ouvrir = Some(d.id);
        f.set_page(crate::factures::page_du_type(d.type_document));
        Retour::default()
    });

    let (e, v) = (etat.clone(), vue.clone());
    api.on_exporter_mouvements(move || {
        let etat = e.borrow();
        let vue = v.borrow();
        crate::paiements::exporter(crate::excel::mouvements_stock(
            &etat.conn,
            &vue.mouvements,
            voit_valeur(&etat),
        ))
    });

    // Inventaire.
    let (f, e, v) = (fenetre.as_weak(), etat.clone(), vue.clone());
    api.on_ouvrir_inventaire(move || {
        let Some(f) = f.upgrade() else { return };
        afficher_inventaire(&f, &e.borrow(), &mut v.borrow_mut());
    });

    let (e, v) = (etat.clone(), vue.clone());
    api.on_feuille_comptage(move |masquer| {
        let etat = e.borrow();
        let vue = v.borrow();
        match crate::pdf::feuille_comptage(&etat.conn, &vue.inventaire_pour_feuille(), masquer) {
            Ok((octets, nom)) => crate::paiements::enregistrer_pdf(&octets, &nom, None),
            Err(message) => retour_erreur(tr(&message)),
        }
    });

    let (f, e, v) = (fenetre.as_weak(), etat.clone(), vue.clone());
    api.on_compter(move |ligne, texte| {
        let Some(f) = f.upgrade() else {
            return SharedString::new();
        };
        let Ok(rang) = usize::try_from(ligne) else {
            return SharedString::new();
        };
        let mut vue = v.borrow_mut();
        let comptee = if texte.trim().is_empty() {
            None
        } else {
            match quantite_comptee(&texte) {
                Some(q) => Some(q),
                _ => {
                    return trf(
                        "Quantité comptée illisible : {} (saisir par exemple 12 ou 2,5).",
                        &[&texte],
                    )
                    .into();
                }
            }
        };
        if let Some(l) = vue.inventaire.get_mut(rang) {
            l.comptee = comptee;
        }
        match noter(&f, &e.borrow(), &mut vue, rang) {
            Ok(()) => SharedString::new(),
            Err(message) => crate::phrase(message),
        }
    });

    let (f, e, v) = (fenetre.as_weak(), etat.clone(), vue.clone());
    api.on_noter_cout(move |ligne, texte| {
        let Some(f) = f.upgrade() else {
            return SharedString::new();
        };
        let Ok(rang) = usize::try_from(ligne) else {
            return SharedString::new();
        };
        let mut vue = v.borrow_mut();
        let cout = if texte.trim().is_empty() {
            None
        } else {
            match montant_depuis_texte(&texte) {
                Some(c) if c >= 0 => Some(c),
                _ => {
                    return trf(
                        "Coût unitaire illisible : {} (montant entier en FCFA).",
                        &[&texte],
                    )
                    .into();
                }
            }
        };
        if let Some(l) = vue.inventaire.get_mut(rang) {
            l.cout = cout;
        }
        match noter(&f, &e.borrow(), &mut vue, rang) {
            Ok(()) => SharedString::new(),
            Err(message) => crate::phrase(message),
        }
    });

    let (f, e, v) = (fenetre.as_weak(), etat.clone(), vue.clone());
    api.on_valider_inventaire(move |motif| {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        let etat = e.borrow();
        let mut vue = v.borrow_mut();
        let retour = valider_inventaire(&etat, &vue, &motif);
        if retour.ok {
            afficher_inventaire(&f, &etat, &mut vue);
            afficher_etat(&f, &etat, &mut vue);
        }
        retour
    });

    let (f, e, v) = (fenetre.as_weak(), etat.clone(), vue.clone());
    api.on_abandonner_inventaire(move || {
        let Some(f) = f.upgrade() else { return };
        let etat = e.borrow();
        if let Err(e) = stock::effacer_comptage(&etat.conn) {
            f.global::<StockApi>()
                .set_inventaire_resume(crate::phrase(e));
            return;
        }
        afficher_inventaire(&f, &etat, &mut v.borrow_mut());
    });

    // Tableau de bord : l'indicateur ouvre l'état sur les produits négatifs.
    let f = fenetre.as_weak();
    fenetre
        .global::<PilotageApi>()
        .on_ouvrir_stock_negatif(move || {
            if let Some(f) = f.upgrade() {
                f.global::<StockApi>().set_demande_negatifs(true);
                f.set_page(Page::Stock);
            }
        });
}

/// Ligne de la feuille de comptage : référence, désignation, quantité théorique.
pub(crate) struct LigneFeuille {
    pub reference: String,
    pub designation: String,
    pub theorique: Quantite,
}

impl Vue {
    fn inventaire_pour_feuille(&self) -> Vec<LigneFeuille> {
        self.inventaire
            .iter()
            .map(|l| LigneFeuille {
                reference: l.reference.clone(),
                designation: l.designation.clone(),
                theorique: l.theorique,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantites_signees() {
        assert_eq!(quantite_signee("5"), Some(5000));
        assert_eq!(quantite_signee("+5"), Some(5000));
        assert_eq!(quantite_signee("-2,5"), Some(-2500));
        assert_eq!(quantite_signee(" - 0,5 "), Some(-500));
        assert_eq!(quantite_signee("abc"), None);
        assert_eq!(texte_quantite(-500), "-0,5");
        assert_eq!(texte_quantite(12_000), "12");
    }

    #[test]
    fn quantites_comptees() {
        assert_eq!(quantite_comptee("0"), Some(0));
        assert_eq!(quantite_comptee(" 0,0 "), Some(0));
        assert_eq!(quantite_comptee("000"), Some(0));
        assert_eq!(quantite_comptee("20"), Some(20_000));
        assert_eq!(quantite_comptee("2,5"), Some(2500));
        assert_eq!(quantite_comptee(""), None);
        assert_eq!(quantite_comptee(",0"), None);
        assert_eq!(quantite_comptee("0,0000"), None);
        assert_eq!(quantite_comptee("-1"), None);
        assert_eq!(quantite_comptee("abc"), None);
    }

    /// Messages d'erreur du stock : six traductions chacun.
    #[test]
    fn textes_du_stock_traduits() {
        let manquants = crate::traduction::sans_traduction(stock::MODELES_ERREUR);
        assert!(manquants.is_empty(), "sans traduction : {manquants:?}");
    }
}
