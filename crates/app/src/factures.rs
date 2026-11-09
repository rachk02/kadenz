//! Écran Factures : registre et fiche (cahier des charges, sections 4.1, 4.2, 6.6, 7).
//!
//! La fiche garde ici l'identifiant du document et les listes affichées (clients,
//! produits, taux, modes) : un rang choisi à l'écran est converti en valeur tout
//! de suite. Les totaux affichés sont calculés par le domaine, jamais par l'écran,
//! et le stockage les recalcule de toute façon à l'enregistrement.

use std::cmp::Ordering;
use std::rc::Rc;

use chrono::NaiveDate;
use kadenz_domaine::calcul::{Pourcentage, Taux, Totaux, calculer_totaux, total_ligne};
use kadenz_domaine::lettres::{Orthographe, en_lettres_majuscule};
use kadenz_domaine::role::Droit;
use kadenz_domaine::saisie;
use kadenz_domaine::texte::{cle_comparaison, contient};
use kadenz_stockage::comptes::Utilisateur;
use kadenz_stockage::documents::{
    self, Document, EtatPaiement, FiltreDocuments, LigneSaisie, ResumeDocument, Saisie, Statut,
    TypeDocument,
};
use kadenz_stockage::produits::{self, FiltreProduits, Produit};
use kadenz_stockage::tiers::{self, FiltreTiers};
use kadenz_stockage::{Connexion, droits, parametres};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use crate::referentiels::{Vue, modele_lignes, retour_erreur, retour_ok};
use crate::traduction::{tr, trf};
use crate::{
    Etat, EtatPartage, EvenementUi, FacturesApi, FenetrePrincipale, FicheFactureUi, LigneFactureUi,
    Retour, TotauxUi,
};

const TYPE: TypeDocument = TypeDocument::Facture;

/// Fiche ouverte : document éventuel et correspondance rang affiché vers valeur.
pub(crate) struct EditionFacture {
    id: Option<i64>,
    /// Tiers du document à l'ouverture : gardé dans la liste même s'il a été désactivé.
    tiers_document: Option<i64>,
    /// Le rang 0 est « aucun client choisi ».
    clients: Vec<Option<i64>>,
    produits: Vec<Produit>,
    remises: Vec<Pourcentage>,
    tvas: Vec<Pourcentage>,
    /// Le rang 0 est « non précisé » (chaîne vide).
    modes: Vec<String>,
    lignes: Rc<VecModel<LigneFactureUi>>,
    orthographe: Orthographe,
    delai_paiement_jours: Option<u32>,
    /// Derniers rangs de remise et de TVA choisis, pour recalculer les totaux
    /// quand une ligne est supprimée (le bouton disparaît avec sa ligne).
    remise_rang: i32,
    tva_rang: i32,
    fiche: FicheFactureUi,
}

pub(crate) type Registre = Vue<ResumeDocument, FiltreDocuments>;

// ---------------------------------------------------------------- Textes

pub(crate) fn date_en_texte(d: NaiveDate) -> String {
    d.format("%d/%m/%Y").to_string()
}

/// Lit une date JJ/MM/AAAA (aussi avec des tirets ou des points). L'année doit
/// avoir quatre chiffres : « 10/05/20 », en cours de frappe, n'est pas une date.
pub(crate) fn date_depuis_texte(texte: &str) -> Option<NaiveDate> {
    let t = texte.trim().replace(['-', '.'], "/");
    let annee = t.rsplit('/').next()?;
    if annee.len() != 4 || !annee.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    NaiveDate::parse_from_str(&t, "%d/%m/%Y").ok()
}

/// « 2026-05-12T09:30:00 » devient « 12/05/2026 09:30 ».
fn horodatage_en_texte(iso: &str) -> String {
    chrono::NaiveDateTime::parse_from_str(iso, "%Y-%m-%dT%H:%M:%S")
        .map(|d| d.format("%d/%m/%Y %H:%M").to_string())
        .unwrap_or_else(|_| iso.to_string())
}

fn pourcentage_libelle(p: Pourcentage) -> SharedString {
    format!("{} %", saisie::pourcentage_en_texte(p)).into()
}

fn modele_textes(textes: impl IntoIterator<Item = String>) -> ModelRc<SharedString> {
    let v: Vec<SharedString> = textes.into_iter().map(SharedString::from).collect();
    ModelRc::new(VecModel::from(v))
}

// ---------------------------------------------------------------- Registre

fn cellules(d: &ResumeDocument) -> Vec<String> {
    vec![
        d.numero.clone().unwrap_or_else(|| tr("(brouillon)")),
        date_en_texte(d.date),
        d.tiers_nom.clone(),
        d.libelle.clone(),
        saisie::montant_en_texte(d.montant_ttc),
        tr(&d.statut.to_string()),
        d.etat_paiement
            .map(|e| tr(&e.to_string()))
            .unwrap_or_default(),
        d.mode_paiement.clone(),
        d.date_echeance.map(date_en_texte).unwrap_or_default(),
    ]
}

fn trier(lignes: &mut [ResumeDocument], colonne: i32, croissant: bool) {
    let comparer = |a: &ResumeDocument, b: &ResumeDocument| -> Ordering {
        match colonne {
            0 => a.numero.cmp(&b.numero),
            1 => a.date.cmp(&b.date),
            2 => cle_comparaison(&a.tiers_nom).cmp(&cle_comparaison(&b.tiers_nom)),
            3 => cle_comparaison(&a.libelle).cmp(&cle_comparaison(&b.libelle)),
            4 => a.montant_ttc.cmp(&b.montant_ttc),
            5 => a.statut.to_string().cmp(&b.statut.to_string()),
            6 => a
                .etat_paiement
                .map(|e| e.to_string())
                .cmp(&b.etat_paiement.map(|e| e.to_string())),
            7 => cle_comparaison(&a.mode_paiement).cmp(&cle_comparaison(&b.mode_paiement)),
            _ => a.date_echeance.cmp(&b.date_echeance),
        }
    };
    lignes.sort_by(|a, b| {
        let o = comparer(a, b);
        if croissant { o } else { o.reverse() }
    });
}

fn afficher_registre(f: &FenetrePrincipale, etat: &mut Etat) {
    let api = f.global::<FacturesApi>();
    match documents::lister(&etat.conn, TYPE, &etat.factures.filtre) {
        Ok(mut lignes) => {
            if let Some((colonne, croissant)) = etat.factures.tri {
                trier(&mut lignes, colonne, croissant);
            }
            api.set_registre(modele_lignes(lignes.iter().map(cellules).collect()));
            api.set_resume(
                match lignes.len() {
                    0 => tr("Aucune facture."),
                    1 => tr("1 facture."),
                    n => trf("{} factures.", &[&n]),
                }
                .into(),
            );
            etat.factures.lignes = lignes;
        }
        Err(e) => api.set_resume(crate::phrase(e)),
    }
    api.set_ligne_courante(-1);
}

// ---------------------------------------------------------------- Fiche

fn possede(etat: &Etat, droit: Droit) -> bool {
    match &etat.utilisateur {
        Some(u) => droits::possede(&etat.conn, u, droit).unwrap_or(false),
        None => false,
    }
}

/// Coordonnées affichées sous le client.
fn detail_tiers(adresse: &str, ifu: &str, rccm: &str) -> String {
    let mut morceaux = Vec::new();
    if !adresse.is_empty() {
        morceaux.push(adresse.to_string());
    }
    if !ifu.is_empty() {
        morceaux.push(format!("IFU {ifu}"));
    }
    if !rccm.is_empty() {
        morceaux.push(format!("RCCM {rccm}"));
    }
    morceaux.join(", ")
}

/// Liste des valeurs proposées, complétée par la valeur du document si elle a
/// été retirée des paramètres depuis.
fn avec_valeur<T: PartialEq + Clone>(mut liste: Vec<T>, valeur: &T) -> (Vec<T>, i32) {
    let rang = match liste.iter().position(|v| v == valeur) {
        Some(r) => r,
        None => {
            liste.push(valeur.clone());
            liste.len() - 1
        }
    };
    (liste, rang as i32)
}

fn ligne_ui(l: &LigneSaisie) -> LigneFactureUi {
    LigneFactureUi {
        produit_id: l.produit_id.unwrap_or(0) as i32,
        reference: l.reference.as_str().into(),
        designation: l.designation.as_str().into(),
        prix: saisie::montant_en_texte(l.prix).into(),
        quantite: saisie::quantite_en_texte(l.quantite).into(),
        total: saisie::montant_en_texte(total_ligne(l.prix, l.quantite)).into(),
    }
}

fn totaux_ui(t: &Totaux, orthographe: Orthographe) -> TotauxUi {
    let m = |v| SharedString::from(saisie::montant_en_texte(v));
    TotauxUi {
        ht: m(t.ht),
        remise: m(t.remise),
        base_tva: m(t.base_tva),
        tva: m(t.tva),
        ttc: m(t.ttc),
        // Le montant en lettres reste en français : il est repris tel quel sur le document.
        lettres: trf(
            "En lettres : {}",
            &[&en_lettres_majuscule(t.ttc, orthographe)],
        )
        .into(),
    }
}

/// Clients proposés : « aucun client choisi », puis les clients actifs filtrés
/// par le texte, plus le client choisi (et celui du document) même s'il ne
/// correspond pas au filtre ou a été désactivé.
fn liste_clients(
    conn: &Connexion,
    texte: &str,
    garder: &[Option<i64>],
) -> (Vec<Option<i64>>, Vec<String>) {
    let tous = tiers::lister(
        conn,
        &FiltreTiers {
            inclure_inactifs: true,
            ..Default::default()
        },
    )
    .unwrap_or_default();
    let mut ids = vec![None];
    let mut noms = vec![tr("Aucun client choisi")];
    for t in tous {
        let garde = garder.contains(&Some(t.id));
        let propose = t.actif && t.fiche.type_tiers.est_client() && contient(&t.fiche.nom, texte);
        if garde || propose {
            ids.push(Some(t.id));
            noms.push(if t.actif {
                t.fiche.nom
            } else {
                trf("{} (désactivé)", &[&t.fiche.nom])
            });
        }
    }
    (ids, noms)
}

fn liste_produits(conn: &Connexion, texte: &str) -> Vec<Produit> {
    produits::lister(conn, &FiltreProduits::default())
        .unwrap_or_default()
        .into_iter()
        .filter(|p| {
            p.fiche.sens.en_vente()
                && (contient(&p.fiche.reference, texte) || contient(&p.fiche.designation, texte))
        })
        .collect()
}

fn afficher_produits(api: &FacturesApi, liste: &[Produit]) {
    api.set_produits(modele_textes(liste.iter().map(|p| {
        format!(
            "{} - {} ({} FCFA)",
            p.fiche.reference,
            p.fiche.designation,
            saisie::montant_en_texte(p.fiche.prix)
        )
    })));
}

/// Prépare la fiche d'un document existant, ou d'une nouvelle facture.
fn ouvrir_fiche(f: &FenetrePrincipale, etat: &mut Etat, id: Option<i64>) -> Result<(), String> {
    let api = f.global::<FacturesApi>();
    let p = parametres::lire(&etat.conn).map_err(|e| e.to_string())?;
    let document = match id {
        Some(id) => Some(
            documents::lire(&etat.conn, id)
                .map_err(|e| e.to_string())?
                .ok_or("cette facture n'existe plus")?,
        ),
        None => None,
    };
    let aujourd_hui = chrono::Local::now().date_naive();
    let saisie_doc = document.as_ref().map_or_else(
        || Saisie {
            date_echeance: documents::echeance_par_defaut(aujourd_hui, p.delai_paiement_jours),
            taux: Taux {
                remise: p.taux_remise.first().copied().unwrap_or(0),
                tva: p.taux_tva.first().copied().unwrap_or(0),
                retenue: 0,
            },
            ..Saisie::vide(aujourd_hui)
        },
        |d| d.saisie.clone(),
    );
    let statut = document.as_ref().map_or(Statut::Brouillon, |d| d.statut);
    let brouillon = statut == Statut::Brouillon;
    let droit_brouillon = possede(etat, Droit::brouillon(TYPE));
    let modifiable = brouillon && droit_brouillon;

    // Clients : liste modifiable, ou seulement le nom figé du document.
    let (clients, noms, client_index, detail) = if modifiable {
        let (ids, noms) = liste_clients(&etat.conn, "", &[saisie_doc.tiers_id]);
        let rang = ids
            .iter()
            .position(|i| *i == saisie_doc.tiers_id)
            .unwrap_or(0) as i32;
        let detail = document
            .as_ref()
            .map(|d| detail_tiers(&d.tiers.adresse, &d.tiers.ifu, &d.tiers.rccm))
            .unwrap_or_default();
        (ids, noms, rang, detail)
    } else {
        let d = document.as_ref();
        let nom = d.map(|d| d.tiers.nom.clone()).unwrap_or_default();
        let detail = d
            .map(|d| detail_tiers(&d.tiers.adresse, &d.tiers.ifu, &d.tiers.rccm))
            .unwrap_or_default();
        (vec![saisie_doc.tiers_id], vec![nom], 0, detail)
    };
    api.set_clients(modele_textes(noms));

    let (remises, remise_index) = avec_valeur(p.taux_remise.clone(), &saisie_doc.taux.remise);
    let (tvas, tva_index) = avec_valeur(p.taux_tva.clone(), &saisie_doc.taux.tva);
    let mut modes_base = vec![String::new()];
    modes_base.extend(p.modes_paiement.iter().cloned());
    let (modes, mode_index) = avec_valeur(modes_base, &saisie_doc.mode_paiement);
    api.set_taux_remise(ModelRc::new(VecModel::from(
        remises
            .iter()
            .map(|&r| pourcentage_libelle(r))
            .collect::<Vec<_>>(),
    )));
    api.set_taux_tva(ModelRc::new(VecModel::from(
        tvas.iter()
            .map(|&t| pourcentage_libelle(t))
            .collect::<Vec<_>>(),
    )));
    api.set_modes_paiement(modele_textes(modes.iter().map(|m| {
        if m.is_empty() {
            tr("Non précisé")
        } else {
            m.clone()
        }
    })));

    let produits = if modifiable {
        liste_produits(&etat.conn, "")
    } else {
        Vec::new()
    };
    afficher_produits(&api, &produits);

    let lignes = Rc::new(VecModel::from(
        saisie_doc.lignes.iter().map(ligne_ui).collect::<Vec<_>>(),
    ));
    api.set_lignes(ModelRc::from(lignes.clone()));
    let totaux = document
        .as_ref()
        .map_or_else(|| calculer_totaux([], saisie_doc.taux, false), |d| d.totaux);
    api.set_totaux(totaux_ui(&totaux, p.orthographe));

    let historique = match id {
        Some(id) => documents::historique(&etat.conn, id).unwrap_or_default(),
        None => Vec::new(),
    };
    api.set_historique(ModelRc::new(VecModel::from(
        historique
            .into_iter()
            .map(|e| EvenementUi {
                date: horodatage_en_texte(&e.date_action).into(),
                utilisateur: e.utilisateur.into(),
                libelle: tr(&e.libelle).into(),
            })
            .collect::<Vec<_>>(),
    )));

    let paiement = match &document {
        Some(d) => documents::etat_paiement_de(&etat.conn, d)
            .ok()
            .flatten()
            .map(|e: EtatPaiement| tr(&e.to_string()))
            .unwrap_or_default(),
        None => String::new(),
    };
    let titre = match &document {
        Some(Document {
            numero: Some(n), ..
        }) => trf("Facture {}", &[n]),
        Some(_) => tr("Facture (brouillon)"),
        None => tr("Nouvelle facture"),
    };
    let valide_ou_envoye = matches!(statut, Statut::Valide | Statut::Envoye);
    let fiche = FicheFactureUi {
        existe: document.is_some(),
        titre: titre.into(),
        statut: tr(&statut.to_string()).into(),
        paiement: paiement.into(),
        brouillon,
        annule: statut == Statut::Annule,
        date: date_en_texte(saisie_doc.date).into(),
        echeance: saisie_doc
            .date_echeance
            .map(date_en_texte)
            .unwrap_or_default()
            .into(),
        libelle: saisie_doc.libelle.as_str().into(),
        observations: saisie_doc.observations.as_str().into(),
        client_index,
        tiers_detail: detail.into(),
        remise_index,
        tva_index,
        mode_index,
        modifiable,
        admin_modifiable: valide_ou_envoye && droit_brouillon,
        peut_valider: modifiable && possede(etat, Droit::ValiderDocument),
        peut_envoyer: statut == Statut::Valide && possede(etat, Droit::ValiderDocument),
        peut_annuler: valide_ou_envoye && possede(etat, Droit::AnnulerDocumentValide),
        peut_supprimer: document.is_some() && modifiable,
    };
    etat.facture = Some(EditionFacture {
        id,
        tiers_document: saisie_doc.tiers_id,
        clients,
        produits,
        remises,
        tvas,
        modes,
        lignes,
        orthographe: p.orthographe,
        delai_paiement_jours: p.delai_paiement_jours,
        remise_rang: remise_index,
        tva_rang: tva_index,
        fiche,
    });
    api.set_version_fiche(api.get_version_fiche().wrapping_add(1));
    api.set_fiche_ouverte(true);
    Ok(())
}

/// Valeur choisie au rang donné, ou `None` hors liste.
fn au_rang<T: Clone>(liste: &[T], rang: i32) -> Option<T> {
    usize::try_from(rang)
        .ok()
        .and_then(|r| liste.get(r).cloned())
}

/// Lignes de l'écran, contrôlées.
fn lignes_saisies(e: &EditionFacture) -> Result<Vec<LigneSaisie>, String> {
    e.lignes
        .iter()
        .enumerate()
        .map(|(rang, l)| {
            let n = rang + 1;
            let prix = saisie::montant_depuis_texte(&l.prix).ok_or_else(|| {
                trf(
                    "ligne {} : prix illisible, saisir un montant entier en FCFA",
                    &[&n],
                )
            })?;
            let quantite = saisie::quantite_depuis_texte(&l.quantite).ok_or_else(|| {
                trf(
                    "ligne {} : quantité illisible, saisir un nombre positif (3 décimales au plus)",
                    &[&n],
                )
            })?;
            Ok(LigneSaisie {
                produit_id: (l.produit_id > 0).then_some(i64::from(l.produit_id)),
                reference: l.reference.to_string(),
                designation: l.designation.to_string(),
                prix,
                quantite,
                observations: String::new(),
            })
        })
        .collect()
}

fn saisie_depuis_ui(e: &EditionFacture, fiche: &FicheFactureUi) -> Result<Saisie, String> {
    let date = date_depuis_texte(&fiche.date)
        .ok_or("date illisible : saisir la date au format JJ/MM/AAAA")?;
    let date_echeance = if fiche.echeance.trim().is_empty() {
        None
    } else {
        Some(date_depuis_texte(&fiche.echeance).ok_or(
            "échéance illisible : saisir la date au format JJ/MM/AAAA, ou la laisser vide",
        )?)
    };
    Ok(Saisie {
        date,
        tiers_id: au_rang(&e.clients, fiche.client_index).flatten(),
        libelle: fiche.libelle.to_string(),
        taux: Taux {
            remise: au_rang(&e.remises, fiche.remise_index).unwrap_or(0),
            tva: au_rang(&e.tvas, fiche.tva_index).unwrap_or(0),
            retenue: 0,
        },
        mode_paiement: au_rang(&e.modes, fiche.mode_index).unwrap_or_default(),
        observations: fiche.observations.to_string(),
        date_echeance,
        lignes: lignes_saisies(e)?,
    })
}

/// Enregistre le brouillon affiché ; renvoie son identifiant.
fn enregistrer_brouillon(
    etat: &mut Etat,
    acteur: &Utilisateur,
    fiche: &FicheFactureUi,
) -> Result<i64, String> {
    let e = etat.facture.as_ref().ok_or("aucune fiche ouverte")?;
    let s = saisie_depuis_ui(e, fiche)?;
    let id = e.id;
    match id {
        Some(id) => documents::modifier_brouillon(&mut etat.conn, acteur, id, &s)
            .map(|()| id)
            .map_err(|e| e.to_string()),
        None => {
            documents::creer_brouillon(&mut etat.conn, acteur, TYPE, &s).map_err(|e| e.to_string())
        }
    }
}

/// Action sur le document ouvert, puis rechargement de la fiche et du registre.
fn agir(
    f: &FenetrePrincipale,
    etat: &EtatPartage,
    action: impl FnOnce(&mut Etat, &Utilisateur, i64) -> Result<String, String>,
) -> Retour {
    let mut etat = etat.borrow_mut();
    let Some(acteur) = etat.utilisateur.clone() else {
        return retour_erreur("aucune session ouverte");
    };
    let Some(id) = etat.facture.as_ref().and_then(|e| e.id) else {
        return retour_erreur("enregistrer d'abord la facture");
    };
    match action(&mut etat, &acteur, id) {
        Ok(message) => {
            if let Err(e) = ouvrir_fiche(f, &mut etat, Some(id)) {
                return retour_erreur(e);
            }
            afficher_registre(f, &mut etat);
            retour_ok(message)
        }
        Err(message) => retour_erreur(message),
    }
}

/// Recalcule le total de chaque ligne lisible et les totaux du document.
fn recalculer(api: &FacturesApi, e: &EditionFacture, remise: i32, tva: i32) {
    let mut valeurs = Vec::new();
    for rang in 0..e.lignes.row_count() {
        let Some(mut l) = e.lignes.row_data(rang) else {
            continue;
        };
        let lue =
            saisie::montant_depuis_texte(&l.prix).zip(saisie::quantite_depuis_texte(&l.quantite));
        let total: SharedString = match lue {
            Some((prix, quantite)) => {
                valeurs.push((prix, quantite));
                saisie::montant_en_texte(total_ligne(prix, quantite)).into()
            }
            None => SharedString::new(),
        };
        if l.total != total {
            l.total = total;
            e.lignes.set_row_data(rang, l);
        }
    }
    let taux = Taux {
        remise: au_rang(&e.remises, remise).unwrap_or(0),
        tva: au_rang(&e.tvas, tva).unwrap_or(0),
        retenue: 0,
    };
    api.set_totaux(totaux_ui(
        &calculer_totaux(valeurs, taux, false),
        e.orthographe,
    ));
}

/// Échéance à afficher après un changement de date (voir `echeance-apres-date`).
fn echeance_apres_date(
    delai: Option<u32>,
    date_precedente: &str,
    date: &str,
    echeance: &str,
) -> Option<String> {
    let ancienne_auto = date_depuis_texte(date_precedente)
        .and_then(|d| documents::echeance_par_defaut(d, delai))
        .map(date_en_texte);
    let automatique =
        echeance.trim().is_empty() || ancienne_auto.as_deref() == Some(echeance.trim());
    if !automatique || delai.is_none() {
        return None;
    }
    // Date en cours de frappe : l'échéance automatique est gardée telle quelle.
    let nouvelle = date_depuis_texte(date)?;
    documents::echeance_par_defaut(nouvelle, delai).map(date_en_texte)
}

pub(crate) fn brancher(fenetre: &FenetrePrincipale, etat: &EtatPartage) {
    let api = fenetre.global::<FacturesApi>();

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_ouvrir(move || {
        let Some(f) = f.upgrade() else { return };
        let mut etat = e.borrow_mut();
        etat.factures = Registre::default();
        etat.facture = None;
        let api = f.global::<FacturesApi>();
        api.set_peut_creer(possede(&etat, Droit::brouillon(TYPE)));
        api.set_fiche_ouverte(false);
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_filtrer(move |texte, statut, paiement, du, au| {
        let Some(f) = f.upgrade() else {
            return SharedString::new();
        };
        let mut etat = e.borrow_mut();
        let mut erreurs = Vec::new();
        let mut periode = |texte: &str, libelle: &'static str| {
            if texte.trim().is_empty() {
                None
            } else {
                let d = date_depuis_texte(texte);
                if d.is_none() {
                    erreurs.push(libelle);
                }
                d
            }
        };
        etat.factures.filtre = FiltreDocuments {
            texte: texte.to_string(),
            statut: match statut {
                1 => Some(Statut::Brouillon),
                2 => Some(Statut::Valide),
                3 => Some(Statut::Envoye),
                4 => Some(Statut::Annule),
                _ => None,
            },
            etat_paiement: match paiement {
                1 => Some(EtatPaiement::Impaye),
                2 => Some(EtatPaiement::Partiel),
                3 => Some(EtatPaiement::Paye),
                _ => None,
            },
            du: periode(&du, "début"),
            au: periode(&au, "fin"),
        };
        afficher_registre(&f, &mut etat);
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

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_trier(move |colonne, croissant| {
        let Some(f) = f.upgrade() else { return };
        let mut etat = e.borrow_mut();
        etat.factures.tri = Some((colonne, croissant));
        afficher_registre(&f, &mut etat);
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_selectionner(move |ligne| {
        let Some(f) = f.upgrade() else { return };
        let mut etat = e.borrow_mut();
        let Some(id) = usize::try_from(ligne)
            .ok()
            .and_then(|r| etat.factures.lignes.get(r))
            .map(|d| d.id)
        else {
            return;
        };
        if let Err(message) = ouvrir_fiche(&f, &mut etat, Some(id)) {
            f.global::<FacturesApi>().set_resume(crate::phrase(message));
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_nouvelle(move || {
        let Some(f) = f.upgrade() else { return };
        let mut etat = e.borrow_mut();
        if let Err(message) = ouvrir_fiche(&f, &mut etat, None) {
            f.global::<FacturesApi>().set_resume(crate::phrase(message));
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_fermer_fiche(move || {
        let Some(f) = f.upgrade() else { return };
        e.borrow_mut().facture = None;
        f.global::<FacturesApi>().set_fiche_ouverte(false);
    });

    let e = etat.clone();
    api.on_fiche_courante(move || {
        e.borrow()
            .facture
            .as_ref()
            .map(|e| e.fiche.clone())
            .unwrap_or_default()
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_filtrer_clients(move |texte, rang_courant| {
        let Some(f) = f.upgrade() else { return -1 };
        let mut guard = e.borrow_mut();
        let etat = &mut *guard;
        let Some(edition) = etat.facture.as_mut() else {
            return -1;
        };
        let courant = au_rang(&edition.clients, rang_courant).flatten();
        let (ids, noms) = liste_clients(&etat.conn, &texte, &[courant, edition.tiers_document]);
        let rang = ids.iter().position(|i| *i == courant).unwrap_or(0) as i32;
        edition.clients = ids;
        f.global::<FacturesApi>().set_clients(modele_textes(noms));
        rang
    });

    let e = etat.clone();
    api.on_client_choisi(move |rang| {
        let etat = e.borrow();
        let Some(id) = etat
            .facture
            .as_ref()
            .and_then(|ed| au_rang(&ed.clients, rang).flatten())
        else {
            return SharedString::new();
        };
        match tiers::lire(&etat.conn, id) {
            Ok(Some(t)) => detail_tiers(&t.fiche.adresse, &t.fiche.ifu, &t.fiche.rccm).into(),
            _ => SharedString::new(),
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_filtrer_produits(move |texte| {
        let Some(f) = f.upgrade() else { return };
        let mut guard = e.borrow_mut();
        let etat = &mut *guard;
        let Some(edition) = etat.facture.as_mut() else {
            return;
        };
        edition.produits = liste_produits(&etat.conn, &texte);
        afficher_produits(&f.global::<FacturesApi>(), &edition.produits);
    });

    let e = etat.clone();
    api.on_ajouter_produit(move |rang| {
        let etat = e.borrow();
        let Some(edition) = etat.facture.as_ref() else {
            return;
        };
        if let Some(p) = au_rang(&edition.produits, rang) {
            edition.lignes.push(ligne_ui(&LigneSaisie {
                produit_id: Some(p.id),
                reference: p.fiche.reference,
                designation: p.fiche.designation,
                prix: p.fiche.prix,
                quantite: kadenz_domaine::calcul::UNE_UNITE,
                observations: String::new(),
            }));
        }
    });

    let e = etat.clone();
    api.on_ajouter_ligne_libre(move || {
        if let Some(edition) = e.borrow().facture.as_ref() {
            edition.lignes.push(LigneFactureUi {
                quantite: "1".into(),
                ..Default::default()
            });
        }
    });

    let e = etat.clone();
    api.on_reference_saisie(move |ligne| {
        let etat = e.borrow();
        let Some(edition) = etat.facture.as_ref() else {
            return;
        };
        let Ok(rang) = usize::try_from(ligne) else {
            return;
        };
        let Some(mut l) = edition.lignes.row_data(rang) else {
            return;
        };
        match produits::par_reference(&etat.conn, l.reference.trim()) {
            Ok(Some(p)) if p.actif && p.fiche.sens.en_vente() => {
                l.produit_id = p.id as i32;
                l.reference = p.fiche.reference.into();
                l.designation = p.fiche.designation.into();
                l.prix = saisie::montant_en_texte(p.fiche.prix).into();
            }
            // Référence inconnue : la ligne devient libre, la saisie est gardée.
            _ => l.produit_id = 0,
        }
        edition.lignes.set_row_data(rang, l);
    });

    let e = etat.clone();
    api.on_deplacer_ligne(move |ligne, sens| {
        let etat = e.borrow();
        let Some(edition) = etat.facture.as_ref() else {
            return;
        };
        let (Ok(de), Ok(vers)) = (usize::try_from(ligne), usize::try_from(ligne + sens)) else {
            return;
        };
        if de < edition.lignes.row_count() && vers < edition.lignes.row_count() {
            let l = edition.lignes.remove(de);
            edition.lignes.insert(vers, l);
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_supprimer_ligne(move |ligne| {
        let Some(f) = f.upgrade() else { return };
        let etat = e.borrow();
        if let (Some(edition), Ok(rang)) = (etat.facture.as_ref(), usize::try_from(ligne))
            && rang < edition.lignes.row_count()
        {
            edition.lignes.remove(rang);
            recalculer(
                &f.global::<FacturesApi>(),
                edition,
                edition.remise_rang,
                edition.tva_rang,
            );
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_recalculer(move |remise, tva| {
        let Some(f) = f.upgrade() else { return };
        if let Some(edition) = e.borrow_mut().facture.as_mut() {
            edition.remise_rang = remise;
            edition.tva_rang = tva;
            recalculer(&f.global::<FacturesApi>(), edition, remise, tva);
        }
    });

    let e = etat.clone();
    api.on_echeance_apres_date(move |precedente, date, echeance| {
        let delai = e
            .borrow()
            .facture
            .as_ref()
            .and_then(|ed| ed.delai_paiement_jours);
        echeance_apres_date(delai, &precedente, &date, &echeance)
            .map(SharedString::from)
            .unwrap_or(echeance)
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_enregistrer(move |fiche| {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        let mut etat = e.borrow_mut();
        let Some(acteur) = etat.utilisateur.clone() else {
            return retour_erreur("aucune session ouverte");
        };
        match enregistrer_brouillon(&mut etat, &acteur, &fiche) {
            Ok(id) => {
                if let Err(message) = ouvrir_fiche(&f, &mut etat, Some(id)) {
                    return retour_erreur(message);
                }
                afficher_registre(&f, &mut etat);
                retour_ok("brouillon enregistré")
            }
            Err(message) => retour_erreur(message),
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_valider(move |fiche| {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        let mut etat = e.borrow_mut();
        let Some(acteur) = etat.utilisateur.clone() else {
            return retour_erreur("aucune session ouverte");
        };
        // La saisie affichée est enregistrée avant la validation.
        let id = match enregistrer_brouillon(&mut etat, &acteur, &fiche) {
            Ok(id) => id,
            Err(message) => return retour_erreur(message),
        };
        let resultat = documents::valider(&mut etat.conn, &acteur, id);
        // La fiche est rechargée dans tous les cas : le brouillon a été enregistré.
        let rechargement = ouvrir_fiche(&f, &mut etat, Some(id));
        afficher_registre(&f, &mut etat);
        match (resultat, rechargement) {
            (Ok(numero), Ok(())) => retour_ok(trf("facture validée sous le numéro {}", &[&numero])),
            (Err(erreur), _) => retour_erreur(trf(
                "brouillon enregistré, mais validation refusée : {}",
                &[&tr(&erreur.to_string())],
            )),
            (Ok(_), Err(message)) => retour_erreur(message),
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_marquer_envoye(move || {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        agir(&f, &e, |etat, acteur, id| {
            documents::marquer_envoye(&mut etat.conn, acteur, id)
                .map(|()| "facture marquée envoyée".to_string())
                .map_err(|e| e.to_string())
        })
    });

    let e = etat.clone();
    api.on_exporter_pdf(move || {
        let etat = e.borrow();
        let Some(id) = etat.facture.as_ref().and_then(|ed| ed.id) else {
            return retour_erreur("enregistrer d'abord la facture");
        };
        let (octets, nom) = match crate::pdf::generer(&etat.conn, id) {
            Ok(resultat) => resultat,
            Err(message) => return retour_erreur(message),
        };
        let Some(chemin) = rfd::FileDialog::new()
            .set_title(crate::traduction::tr("Enregistrer le PDF"))
            .set_file_name(&nom)
            .add_filter("PDF", &["pdf"])
            .save_file()
        else {
            return Retour::default();
        };
        if let Err(erreur) = std::fs::write(&chemin, &octets) {
            return retour_erreur(crate::traduction::trf(
                "écriture du PDF impossible : {}",
                &[&erreur],
            ));
        }
        // Ouvre le PDF dans la visionneuse par défaut de Windows.
        let _ = std::process::Command::new("explorer").arg(&chemin).spawn();
        retour_ok(crate::traduction::trf(
            "PDF enregistré : {}",
            &[&chemin.display()],
        ))
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_annuler_document(move || {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        agir(&f, &e, |etat, acteur, id| {
            documents::annuler(&mut etat.conn, acteur, id)
                .map(|()| "facture annulée".to_string())
                .map_err(|e| e.to_string())
        })
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_enregistrer_administratif(move |mode, observations| {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        agir(&f, &e, |etat, acteur, id| {
            let mode = etat
                .facture
                .as_ref()
                .and_then(|ed| au_rang(&ed.modes, mode))
                .unwrap_or_default();
            documents::modifier_administratif(&mut etat.conn, acteur, id, &mode, &observations)
                .map(|()| "mode de paiement et observations enregistrés".to_string())
                .map_err(|e| e.to_string())
        })
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_supprimer(move || {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        let mut etat = e.borrow_mut();
        let Some(acteur) = etat.utilisateur.clone() else {
            return retour_erreur("aucune session ouverte");
        };
        let Some(id) = etat.facture.as_ref().and_then(|ed| ed.id) else {
            return retour_erreur("ce brouillon n'a pas été enregistré");
        };
        match documents::supprimer_brouillon(&mut etat.conn, &acteur, id) {
            Ok(()) => {
                etat.facture = None;
                f.global::<FacturesApi>().set_fiche_ouverte(false);
                retour_ok("brouillon supprimé")
            }
            Err(erreur) => retour_erreur(erreur),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_saisies() {
        let d = NaiveDate::from_ymd_opt(2026, 5, 12).unwrap();
        assert_eq!(date_depuis_texte("12/05/2026"), Some(d));
        assert_eq!(date_depuis_texte(" 12-05-2026 "), Some(d));
        assert_eq!(date_depuis_texte("12.05.2026"), Some(d));
        assert_eq!(date_depuis_texte("31/02/2026"), None);
        assert_eq!(date_depuis_texte("2026-05-12"), None);
        assert_eq!(date_depuis_texte("12/05/20"), None);
        assert_eq!(date_en_texte(d), "12/05/2026");
        assert_eq!(
            horodatage_en_texte("2026-05-12T09:30:00"),
            "12/05/2026 09:30"
        );
    }

    #[test]
    fn echeance_suit_la_date_tant_qu_elle_est_automatique() {
        let suivre =
            |precedente, date, echeance| echeance_apres_date(Some(30), precedente, date, echeance);
        // Échéance automatique (date + 30 jours) : elle suit la nouvelle date.
        assert_eq!(
            suivre("01/05/2026", "10/05/2026", "31/05/2026").as_deref(),
            Some("09/06/2026")
        );
        // Échéance saisie à la main : gardée.
        assert_eq!(suivre("01/05/2026", "10/05/2026", "15/07/2026"), None);
        // Date incomplète pendant la frappe : rien ne change.
        assert_eq!(suivre("01/05/2026", "10/05/20", "31/05/2026"), None);
        // Sans délai réglé : jamais de calcul.
        assert_eq!(
            echeance_apres_date(None, "01/05/2026", "10/05/2026", ""),
            None
        );
    }

    #[test]
    fn valeur_retiree_des_parametres_gardee() {
        assert_eq!(avec_valeur(vec![0, 1800], &1800), (vec![0, 1800], 1));
        assert_eq!(avec_valeur(vec![0, 1800], &900), (vec![0, 1800, 900], 2));
    }
}
