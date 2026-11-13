//! Opérations suspendues sur le serveur (phase 2, jalon F6c) : conflits à
//! trancher (décision 17) et opérations en attente que l'Admin accepte ou
//! rejette (décisions 18, 32 et 52). La liste vient du serveur par le
//! synchroniseur, sans bloquer l'écran ; la copie du poste donne la version du
//! serveur (une écriture en conflit y est défaite), l'opération gardée celle du
//! poste. Les fiches de tiers et de produits se comparent champ par champ, dans
//! la forme du protocole ; un brouillon se garde en entier.

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime};
use kadenz_domaine::referentiel::{Sens, TypeTiers};
use kadenz_domaine::saisie::{montant_en_texte, pourcentage_en_texte, quantite_en_texte};
use kadenz_protocole::{self as proto, Action, Entite, Motif, Suspendue};
use kadenz_stockage::Connexion;
use kadenz_stockage::documents::{self, TypeDocument};
use kadenz_stockage::synchro::{self, id_de};
use kadenz_stockage::{comptes, produits, tiers};
use slint::{ComponentHandle, ModelRc, VecModel};

use crate::factures::{date_en_texte, titre_fiche};
use crate::referentiels::{retour_erreur, retour_ok};
use crate::traduction::{tr, trf};
use crate::{
    ChampSuspenduUi, EtatPartage, FenetrePrincipale, ReglagesApi, Retour, Rubrique, SuspendueUi,
    SuspenduesApi,
};

/// Ce que l'écran garde de la liste : relue à chaque ouverture de la rubrique
/// et à chaque changement des nombres.
#[derive(Default)]
pub(crate) struct Suivi {
    liste: Vec<Suspendue>,
    /// Identifiant de l'opération ouverte.
    selection: Option<String>,
    /// Choix champ par champ de l'opération ouverte : vrai, version du poste.
    choix: Vec<bool>,
}

/// Un champ comparé.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Champ {
    libelle: String,
    serveur: String,
    poste: String,
}

impl Champ {
    fn new(libelle: &str, serveur: String, poste: String) -> Self {
        Champ {
            libelle: tr(libelle),
            serveur,
            poste,
        }
    }

    fn different(&self) -> bool {
        self.serveur != self.poste
    }
}

/// Version du serveur d'une fiche en conflit, pour composer un choix champ par
/// champ.
#[derive(Debug, Clone, PartialEq, Eq)]
enum FicheServeur {
    Tiers(proto::Tiers),
    Produit(proto::Produit),
}

/// Ce que l'écran montre d'une opération ouverte.
#[derive(Debug, Default)]
struct Examen {
    champs: Vec<Champ>,
    /// Version du serveur d'une fiche : choix champ par champ possible.
    fiche: Option<FicheServeur>,
    /// La ligne visée n'est plus dans la copie.
    absente: bool,
}

// --- Branchement -----------------------------------------------------------

pub(crate) fn brancher(fenetre: &FenetrePrincipale, etat: &EtatPartage) {
    let api = fenetre.global::<SuspenduesApi>();

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_actualiser(move || {
        if let Some(f) = f.upgrade() {
            demander(&f, &e);
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_choisir(move |id| {
        let Some(f) = f.upgrade() else { return };
        let mut etat = e.borrow_mut();
        let Some(r) = etat.relie.as_mut() else { return };
        let suivi = &mut r.suspendues;
        suivi.selection = if suivi.selection.as_deref() == Some(id.as_str()) {
            None
        } else {
            Some(id.to_string())
        };
        suivi.choix.clear();
        drop(etat);
        afficher(&f, &e);
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_choisir_champ(move |rang, poste| {
        let Some(f) = f.upgrade() else { return };
        let Ok(rang) = usize::try_from(rang) else {
            return;
        };
        if let Some(c) = e
            .borrow_mut()
            .relie
            .as_mut()
            .and_then(|r| r.suspendues.choix.get_mut(rang))
        {
            *c = poste;
        }
        afficher(&f, &e);
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_garder_serveur(move || -> Retour {
        decider(&f, &e, |_, _, _| {
            Ok((
                None,
                tr("Conflit tranché : la version du serveur est gardée."),
            ))
        })
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_garder_poste(move || -> Retour {
        decider(&f, &e, |s, _, _| {
            Ok((
                Some(Box::new(s.operation.action.clone())),
                tr("Conflit tranché : la version du poste est appliquée."),
            ))
        })
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_garder_choix(move || -> Retour {
        decider(&f, &e, |s, conn, choix| {
            let examen = examiner(conn, s);
            let Some(fiche) = examen.fiche.filter(|_| !examen.absente) else {
                return Err(tr("Cette opération ne se tranche pas champ par champ."));
            };
            let resultat = composer(&s.operation.action, &fiche, choix);
            let texte = match &resultat {
                None => tr("Conflit tranché : la version du serveur est gardée."),
                Some(_) => tr("Conflit tranché : votre choix est appliqué."),
            };
            Ok((resultat, texte))
        })
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_decider(move |accepter| -> Retour {
        let Some(fenetre) = f.upgrade() else {
            return retour_erreur(String::new());
        };
        let Some(id) = selection(&e) else {
            return retour_erreur(tr("Cette opération n'est plus dans la liste."));
        };
        let action = Action::DeciderAttente {
            operation: id.clone(),
            accepter,
        };
        let resultat = crate::relie::ecrire_relie(&e.borrow(), action);
        match resultat {
            Ok(_) => {
                retirer(&fenetre, &e, &id);
                retour_ok(if accepter {
                    tr("Opération acceptée : elle est appliquée.")
                } else {
                    tr("Opération rejetée.")
                })
            }
            Err(texte) => retour_erreur(texte),
        }
    });
}

/// Tranche le conflit ouvert : `choisir` donne le résultat et le message, à
/// partir de l'opération, de la copie et des choix champ par champ.
fn decider(
    f: &slint::Weak<FenetrePrincipale>,
    e: &EtatPartage,
    choisir: impl FnOnce(
        &Suspendue,
        &Connexion,
        &[bool],
    ) -> Result<(Option<Box<Action>>, String), String>,
) -> Retour {
    let Some(fenetre) = f.upgrade() else {
        return retour_erreur(String::new());
    };
    let (id, resultat) = {
        let etat = e.borrow();
        let Some(r) = etat.relie.as_ref() else {
            return retour_erreur(tr("Cette opération n'est plus dans la liste."));
        };
        let suivi = &r.suspendues;
        let Some(s) = suivi
            .selection
            .as_deref()
            .and_then(|id| suivi.liste.iter().find(|s| s.operation.id == id))
        else {
            return retour_erreur(tr("Cette opération n'est plus dans la liste."));
        };
        let (resultat, message) = match choisir(s, &etat.conn, &suivi.choix) {
            Ok(choix) => choix,
            Err(texte) => return retour_erreur(texte),
        };
        let action = Action::TrancherConflit {
            operation: s.operation.id.clone(),
            resultat,
        };
        (
            s.operation.id.clone(),
            crate::relie::ecrire_relie(&etat, action).map(|_| message),
        )
    };
    match resultat {
        Ok(message) => {
            retirer(&fenetre, e, &id);
            retour_ok(message)
        }
        Err(texte) => retour_erreur(texte),
    }
}

fn selection(e: &EtatPartage) -> Option<String> {
    let etat = e.borrow();
    let suivi = &etat.relie.as_ref()?.suspendues;
    let id = suivi.selection.clone()?;
    suivi
        .liste
        .iter()
        .any(|s| s.operation.id == id)
        .then_some(id)
}

/// Opération tranchée ou décidée : retirée de la liste tout de suite, puis la
/// liste est relue sur le serveur.
fn retirer(f: &FenetrePrincipale, e: &EtatPartage, id: &str) {
    if let Some(r) = e.borrow_mut().relie.as_mut() {
        r.suspendues.liste.retain(|s| s.operation.id != id);
        r.suspendues.selection = None;
        r.suspendues.choix.clear();
    }
    afficher(f, e);
    demander(f, e);
}

/// Demande la liste au serveur ; elle revient par `recevoir_liste`.
pub(crate) fn demander(f: &FenetrePrincipale, e: &EtatPartage) {
    let api = f.global::<SuspenduesApi>();
    match e.borrow().relie.as_ref() {
        Some(r) => {
            api.set_chargement(true);
            r.synchro.demander_suspendues();
        }
        None => {
            api.set_chargement(false);
            api.set_erreur(
                tr("Serveur injoignable : la liste se lit quand le poste est connecté.").into(),
            );
        }
    }
}

/// Nombres signalés par le synchroniseur : marque de la rubrique, et liste
/// relue si la rubrique est ouverte.
pub(crate) fn recevoir_nombres(
    f: &FenetrePrincipale,
    e: &EtatPartage,
    conflits: u32,
    attentes: u32,
) {
    let api = f.global::<SuspenduesApi>();
    api.set_conflits(i32::try_from(conflits).unwrap_or(i32::MAX));
    api.set_attentes(i32::try_from(attentes).unwrap_or(i32::MAX));
    if f.global::<ReglagesApi>().get_rubrique() == Rubrique::Suspendues {
        demander(f, e);
    }
}

/// Liste reçue du serveur, ou l'erreur qui l'empêche.
pub(crate) fn recevoir_liste(
    f: &FenetrePrincipale,
    e: &EtatPartage,
    liste: Result<Vec<Suspendue>, kadenz_poste::ErreurPoste>,
) {
    let api = f.global::<SuspenduesApi>();
    api.set_chargement(false);
    match liste {
        Ok(liste) => {
            api.set_erreur("".into());
            if let Some(r) = e.borrow_mut().relie.as_mut() {
                let suivi = &mut r.suspendues;
                // L'opération ouverte reste ouverte si elle est encore là.
                if !suivi
                    .selection
                    .as_deref()
                    .is_some_and(|id| liste.iter().any(|s| s.operation.id == id))
                {
                    suivi.selection = None;
                    suivi.choix.clear();
                }
                suivi.liste = liste;
            }
        }
        Err(erreur) => {
            api.set_erreur(
                trf(
                    "Liste illisible : {}.",
                    &[&crate::relie::texte_erreur(&erreur)],
                )
                .into(),
            );
        }
    }
    afficher(f, e);
}

/// Remplit la page : liste, et détail de l'opération ouverte.
fn afficher(f: &FenetrePrincipale, e: &EtatPartage) {
    let api = f.global::<SuspenduesApi>();
    let mut etat = e.borrow_mut();
    let etat = &mut *etat;
    let Some(r) = etat.relie.as_mut() else {
        api.set_lignes(ModelRc::default());
        api.set_champs(ModelRc::default());
        api.set_selection("".into());
        return;
    };
    let conn = &etat.conn;
    let suivi = &mut r.suspendues;
    let lignes: Vec<SuspendueUi> = suivi
        .liste
        .iter()
        .map(|s| SuspendueUi {
            id: s.operation.id.clone().into(),
            conflit: s.motif == Motif::Conflit,
            titre: libelle_action(conn, &s.operation.action).into(),
            detail: auteur_et_heure(conn, s).into(),
        })
        .collect();
    api.set_lignes(ModelRc::new(VecModel::from(lignes)));

    let ouverte = suivi
        .selection
        .as_deref()
        .and_then(|id| suivi.liste.iter().find(|s| s.operation.id == id));
    let Some(s) = ouverte else {
        api.set_selection("".into());
        api.set_champs(ModelRc::default());
        api.set_par_champ(false);
        api.set_absente(false);
        return;
    };
    let examen = if s.motif == Motif::Conflit {
        examiner(conn, s)
    } else {
        contenu(conn, &s.operation.action)
    };
    if suivi.choix.len() != examen.champs.len() {
        suivi.choix = vec![false; examen.champs.len()];
    }
    let champs: Vec<ChampSuspenduUi> = examen
        .champs
        .iter()
        .zip(&suivi.choix)
        .map(|(c, &choix)| ChampSuspenduUi {
            libelle: c.libelle.clone().into(),
            serveur: c.serveur.clone().into(),
            poste: c.poste.clone().into(),
            different: c.different(),
            choix_poste: choix,
        })
        .collect();
    api.set_selection(s.operation.id.clone().into());
    api.set_par_champ(examen.fiche.is_some());
    api.set_absente(examen.absente);
    api.set_champs(ModelRc::new(VecModel::from(champs)));
}

// --- Libellés ---------------------------------------------------------------

/// Auteur (compte de l'opération) et heure : celle du poste, sinon celle de
/// réception par le serveur.
fn auteur_et_heure(conn: &Connexion, s: &Suspendue) -> String {
    let auteur = id_de(conn, Entite::Utilisateur, &s.operation.compte)
        .ok()
        .flatten()
        .and_then(|id| comptes::lire(conn, id).ok().flatten())
        .map(|u| u.nom_complet)
        .unwrap_or_else(|| tr("compte inconnu"));
    match s.operation.heure_poste.as_deref() {
        Some(heure) => trf(
            "Par {}, le {} (heure du poste)",
            &[&auteur, &heure_poste(heure)],
        ),
        None => trf(
            "Par {}, reçue par le serveur le {}",
            &[&auteur, &heure_serveur(&s.recue)],
        ),
    }
}

/// Heure RFC 3339 du poste, en heure locale JJ/MM/AAAA HH:MM.
fn heure_poste(texte: &str) -> String {
    DateTime::parse_from_rfc3339(texte)
        .map(|d| {
            let d = d.with_timezone(&Local);
            format!("{} {}", date_en_texte(d.date_naive()), d.format("%H:%M"))
        })
        .unwrap_or_else(|_| texte.to_string())
}

/// Heure `AAAA-MM-JJTHH:MM:SS` du serveur, en JJ/MM/AAAA HH:MM.
fn heure_serveur(texte: &str) -> String {
    NaiveDateTime::parse_from_str(texte, "%Y-%m-%dT%H:%M:%S")
        .map(|d| format!("{} {}", date_en_texte(d.date()), d.format("%H:%M")))
        .unwrap_or_else(|_| texte.to_string())
}

/// Date `AAAA-MM-JJ` du protocole, en JJ/MM/AAAA.
fn date_texte(texte: &str) -> String {
    NaiveDate::parse_from_str(texte, "%Y-%m-%d")
        .map(date_en_texte)
        .unwrap_or_else(|_| texte.to_string())
}

fn nom_tiers(conn: &Connexion, uid: &str) -> Option<String> {
    let id = id_de(conn, Entite::Tiers, uid).ok().flatten()?;
    tiers::lire(conn, id).ok().flatten().map(|t| t.fiche.nom)
}

fn nom_produit(conn: &Connexion, uid: &str) -> Option<String> {
    let id = id_de(conn, Entite::Produit, uid).ok().flatten()?;
    produits::lire(conn, id)
        .ok()
        .flatten()
        .map(|p| p.fiche.designation)
}

/// Document de la copie : type, numéro et tiers.
fn nom_document(conn: &Connexion, uid: &str) -> String {
    id_de(conn, Entite::Document, uid)
        .ok()
        .flatten()
        .and_then(|id| documents::lire(conn, id).ok().flatten())
        .map(|d| {
            let titre = titre_fiche(d.type_document, d.numero.as_deref(), true);
            if d.tiers.nom.is_empty() {
                titre
            } else {
                format!("{titre} - {}", d.tiers.nom)
            }
        })
        .unwrap_or_else(|| tr("document absent de ce poste"))
}

/// Ce que fait une opération, en une ligne.
fn libelle_action(conn: &Connexion, action: &Action) -> String {
    use Action::*;
    let tiers = |uid: &str| nom_tiers(conn, uid).unwrap_or_else(|| tr("fiche absente de ce poste"));
    let produit =
        |uid: &str| nom_produit(conn, uid).unwrap_or_else(|| tr("fiche absente de ce poste"));
    let document = |uid: &str| nom_document(conn, uid);
    match action {
        CreerTiers { fiche, .. } => trf("Création de la fiche « {} »", &[&fiche.nom]),
        ModifierTiers { fiche, .. } => trf("Modification de la fiche « {} »", &[&fiche.nom]),
        RetirerTiers { uid } => trf("Retrait de la fiche « {} »", &[&tiers(uid)]),
        DefinirActifTiers { uid, actif: true } => {
            trf("Réactivation de la fiche « {} »", &[&tiers(uid)])
        }
        DefinirActifTiers { uid, actif: false } => {
            trf("Désactivation de la fiche « {} »", &[&tiers(uid)])
        }
        ImporterTiers { fiches, .. } => trf("Import de {} fiches de tiers", &[&fiches.len()]),
        CreerProduit { fiche, .. } => trf("Création de la fiche « {} »", &[&fiche.designation]),
        ModifierProduit { fiche, .. } => {
            trf("Modification de la fiche « {} »", &[&fiche.designation])
        }
        RetirerProduit { uid } => trf("Retrait de la fiche « {} »", &[&produit(uid)]),
        DefinirActifProduit { uid, actif: true } => {
            trf("Réactivation de la fiche « {} »", &[&produit(uid)])
        }
        DefinirActifProduit { uid, actif: false } => {
            trf("Désactivation de la fiche « {} »", &[&produit(uid)])
        }
        ImporterProduits { fiches, .. } => trf("Import de {} produits", &[&fiches.len()]),
        CreerBrouillon {
            type_document,
            saisie,
            ..
        } => {
            let titre = TypeDocument::depuis_code(type_document)
                .map(|t| titre_fiche(t, None, false))
                .unwrap_or_else(|| tr("Nouveau document"));
            match saisie.tiers.as_deref().and_then(|uid| nom_tiers(conn, uid)) {
                Some(nom) => format!("{titre} - {nom}"),
                None => titre,
            }
        }
        ModifierBrouillon { uid, .. }
        | ModifierLivraison { uid, .. }
        | ModifierAdministratif { uid, .. } => trf("Modification : {}", &[&document(uid)]),
        Valider { uid } => trf("Validation : {}", &[&document(uid)]),
        Annuler { uid } => trf("Annulation : {}", &[&document(uid)]),
        SupprimerBrouillon { uid } => trf("Suppression : {}", &[&document(uid)]),
        Dupliquer { uid, .. } => trf("Copie : {}", &[&document(uid)]),
        CreerAvoir { uid, .. } => trf("Avoir sur : {}", &[&document(uid)]),
        ConvertirEnFacture { uid, .. } => trf("Conversion en facture : {}", &[&document(uid)]),
        CreerBordereau { uid, .. } => trf("Bordereau depuis : {}", &[&document(uid)]),
        ConfirmerLivraison { uid } => trf("Livraison confirmée : {}", &[&document(uid)]),
        EnregistrerReception { uid, .. } => trf("Réception enregistrée : {}", &[&document(uid)]),
        MarquerEnvoye { uid } => trf("Envoi noté : {}", &[&document(uid)]),
        AjouterReglement {
            document: doc,
            montant,
            ..
        } => trf(
            "Règlement de {} : {}",
            &[&montant_en_texte(*montant), &document(doc)],
        ),
        SupprimerReglement { .. } => tr("Suppression d'un règlement"),
        ChangerLangue { .. } => tr("Changement de langue"),
        ChangerAffichage { .. } => tr("Changement de navigation et d'affichage"),
        _ => tr("Opération sur les comptes ou les réglages"),
    }
}

// --- Comparaison ------------------------------------------------------------

fn type_tiers_texte(code: &str) -> String {
    TypeTiers::depuis_code(code)
        .map(|t| tr(&t.to_string()))
        .unwrap_or_else(|| code.to_string())
}

fn sens_texte(code: &str) -> String {
    Sens::depuis_code(code)
        .map(|s| tr(&s.to_string()))
        .unwrap_or_else(|| code.to_string())
}

const LIBELLES_TIERS: [&str; 8] = [
    "Nom",
    "Adresse",
    "Téléphone",
    "Localisation",
    "IFU",
    "RCCM",
    "Régime d'imposition",
    "Type",
];

fn valeurs_tiers(t: &proto::Tiers) -> [String; 8] {
    [
        t.nom.clone(),
        t.adresse.clone(),
        t.telephone.clone(),
        t.localisation.clone(),
        t.ifu.clone(),
        t.rccm.clone(),
        t.regime_imposition.clone(),
        type_tiers_texte(&t.type_tiers),
    ]
}

const LIBELLES_PRODUIT: [&str; 4] = ["Désignation", "Référence", "Prix (FCFA)", "Sens"];

fn valeurs_produit(p: &proto::Produit) -> [String; 4] {
    [
        p.designation.clone(),
        p.reference.clone(),
        montant_en_texte(p.prix),
        sens_texte(&p.sens),
    ]
}

fn champs<const N: usize>(
    libelles: &[&str; N],
    serveur: Option<[String; N]>,
    poste: [String; N],
) -> Vec<Champ> {
    let serveur = serveur.unwrap_or_else(|| std::array::from_fn(|_| String::new()));
    libelles
        .iter()
        .zip(serveur)
        .zip(poste)
        .map(|((l, s), p)| Champ::new(l, s, p))
        .collect()
}

/// Champs d'un brouillon, dans la forme du protocole ; les visas et dates de
/// livraison seulement s'ils sont remplis d'un côté.
fn champs_saisie(
    conn: &Connexion,
    serveur: Option<&proto::Saisie>,
    poste: &proto::Saisie,
) -> Vec<Champ> {
    let vide = proto::Saisie::default();
    let s = serveur.unwrap_or(&vide);
    let nom = |t: &Option<String>| {
        t.as_deref()
            .map(|uid| nom_tiers(conn, uid).unwrap_or_else(|| uid.to_string()))
            .unwrap_or_default()
    };
    let echeance = |d: &Option<String>| d.as_deref().map(date_texte).unwrap_or_default();
    let lignes = |x: &proto::Saisie| {
        x.lignes
            .iter()
            .map(|l| format!("{} x {}", quantite_en_texte(l.quantite), l.designation))
            .collect::<Vec<_>>()
            .join(" ; ")
    };
    let date = |x: &proto::Saisie| {
        if x.date.is_empty() {
            String::new()
        } else {
            date_texte(&x.date)
        }
    };
    let mut c = vec![
        Champ::new("Date", date(s), date(poste)),
        Champ::new("Tiers", nom(&s.tiers), nom(&poste.tiers)),
        Champ::new("Libellé", s.libelle.clone(), poste.libelle.clone()),
        Champ::new(
            "Remise",
            pourcentage_en_texte(s.remise),
            pourcentage_en_texte(poste.remise),
        ),
        Champ::new(
            "TVA",
            pourcentage_en_texte(s.tva),
            pourcentage_en_texte(poste.tva),
        ),
        Champ::new(
            "Retenue",
            pourcentage_en_texte(s.retenue),
            pourcentage_en_texte(poste.retenue),
        ),
        Champ::new(
            "Mode de paiement",
            s.mode_paiement.clone(),
            poste.mode_paiement.clone(),
        ),
        Champ::new(
            "Observations",
            s.observations.clone(),
            poste.observations.clone(),
        ),
        Champ::new(
            "Échéance",
            echeance(&s.date_echeance),
            echeance(&poste.date_echeance),
        ),
        Champ::new("Lignes", lignes(s), lignes(poste)),
    ];
    c.extend(champs_livraison(
        [
            s.visa_client.clone(),
            s.visa_fournisseur.clone(),
            s.date_livraison.clone(),
            s.date_reception.clone(),
        ],
        [
            poste.visa_client.clone(),
            poste.visa_fournisseur.clone(),
            poste.date_livraison.clone(),
            poste.date_reception.clone(),
        ],
    ));
    if serveur.is_none() {
        for champ in &mut c {
            champ.serveur.clear();
        }
    }
    c
}

/// Visas et dates d'un bordereau, ceux qui sont remplis d'un côté.
fn champs_livraison(serveur: [String; 4], poste: [String; 4]) -> Vec<Champ> {
    const LIBELLES: [&str; 4] = [
        "Visa du client",
        "Visa du fournisseur",
        "Date de livraison",
        "Date de réception",
    ];
    LIBELLES
        .iter()
        .zip(serveur)
        .zip(poste)
        .filter(|((_, s), p)| !s.is_empty() || !p.is_empty())
        .map(|((l, s), p)| Champ::new(l, s, p))
        .collect()
}

/// Version du serveur d'une ligne, lue dans la copie.
fn tiers_serveur(conn: &Connexion, uid: &str) -> Option<proto::Tiers> {
    let id = id_de(conn, Entite::Tiers, uid).ok().flatten()?;
    let t = tiers::lire(conn, id).ok().flatten()?;
    Some(synchro::tiers_pour_serveur(&t.fiche))
}

fn produit_serveur(conn: &Connexion, uid: &str) -> Option<proto::Produit> {
    let id = id_de(conn, Entite::Produit, uid).ok().flatten()?;
    let p = produits::lire(conn, id).ok().flatten()?;
    Some(synchro::produit_pour_serveur(&p.fiche))
}

fn saisie_serveur(conn: &Connexion, uid: &str) -> Option<proto::Saisie> {
    let id = id_de(conn, Entite::Document, uid).ok().flatten()?;
    let d = documents::lire(conn, id).ok().flatten()?;
    synchro::saisie_pour_serveur(conn, &d.saisie).ok()
}

/// Conflit : les deux versions, côte à côte.
fn examiner(conn: &Connexion, s: &Suspendue) -> Examen {
    use Action::*;
    match &s.operation.action {
        ModifierTiers { uid, fiche } => {
            let serveur = tiers_serveur(conn, uid);
            Examen {
                champs: champs(
                    &LIBELLES_TIERS,
                    serveur.as_ref().map(valeurs_tiers),
                    valeurs_tiers(fiche),
                ),
                absente: serveur.is_none(),
                fiche: serveur.map(FicheServeur::Tiers),
            }
        }
        ModifierProduit { uid, fiche } => {
            let serveur = produit_serveur(conn, uid);
            Examen {
                champs: champs(
                    &LIBELLES_PRODUIT,
                    serveur.as_ref().map(valeurs_produit),
                    valeurs_produit(fiche),
                ),
                absente: serveur.is_none(),
                fiche: serveur.map(FicheServeur::Produit),
            }
        }
        ModifierBrouillon { uid, saisie } => {
            let serveur = saisie_serveur(conn, uid);
            Examen {
                champs: champs_saisie(conn, serveur.as_ref(), saisie),
                absente: serveur.is_none(),
                fiche: None,
            }
        }
        ModifierLivraison {
            uid,
            visa_client,
            visa_fournisseur,
            date_livraison,
            date_reception,
        } => {
            let serveur = saisie_serveur(conn, uid);
            let s = serveur.clone().unwrap_or_default();
            Examen {
                champs: champs_livraison(
                    [
                        s.visa_client,
                        s.visa_fournisseur,
                        s.date_livraison,
                        s.date_reception,
                    ],
                    [
                        visa_client.clone(),
                        visa_fournisseur.clone(),
                        date_livraison.clone(),
                        date_reception.clone(),
                    ],
                ),
                absente: serveur.is_none(),
                fiche: None,
            }
        }
        ModifierAdministratif {
            uid,
            mode_paiement,
            observations,
        } => {
            let serveur = saisie_serveur(conn, uid);
            let s = serveur.clone().unwrap_or_default();
            Examen {
                champs: vec![
                    Champ::new("Mode de paiement", s.mode_paiement, mode_paiement.clone()),
                    Champ::new("Observations", s.observations, observations.clone()),
                ],
                absente: serveur.is_none(),
                fiche: None,
            }
        }
        // Autre conflit (pas attendu) : la cible, sans détail.
        _ => Examen {
            absente: s
                .cible
                .as_ref()
                .is_some_and(|(e, uid)| id_de(conn, *e, uid).ok().flatten().is_none()),
            ..Examen::default()
        },
    }
}

/// Opération en attente : son contenu, dans la colonne du poste.
fn contenu(conn: &Connexion, action: &Action) -> Examen {
    use Action::*;
    let champs = match action {
        CreerTiers { fiche, .. } | ModifierTiers { fiche, .. } => {
            champs(&LIBELLES_TIERS, None, valeurs_tiers(fiche))
        }
        CreerProduit { fiche, .. } | ModifierProduit { fiche, .. } => {
            champs(&LIBELLES_PRODUIT, None, valeurs_produit(fiche))
        }
        CreerBrouillon { saisie, .. } | ModifierBrouillon { saisie, .. } => {
            champs_saisie(conn, None, saisie)
                .into_iter()
                .filter(|c| !c.poste.is_empty())
                .collect()
        }
        AjouterReglement {
            date,
            montant,
            mode_paiement,
            reference,
            ..
        } => vec![
            Champ::new("Date", String::new(), date_texte(date)),
            Champ::new("Montant", String::new(), montant_en_texte(*montant)),
            Champ::new("Mode de paiement", String::new(), mode_paiement.clone()),
            Champ::new("Référence", String::new(), reference.clone()),
        ],
        _ => Vec::new(),
    };
    Examen {
        champs,
        ..Examen::default()
    }
}

/// Résultat d'un choix champ par champ : `None` si tout vient du serveur,
/// l'action d'origine si tout vient du poste, sinon une action du même genre
/// sur la même ligne, composée champ par champ.
fn composer(action: &Action, serveur: &FicheServeur, choix: &[bool]) -> Option<Box<Action>> {
    let poste = |i: usize| choix.get(i).copied().unwrap_or(false);
    let pris = |i: usize, s: &String, p: &String| if poste(i) { p.clone() } else { s.clone() };
    let composee = match (action, serveur) {
        (Action::ModifierTiers { uid, fiche: p }, FicheServeur::Tiers(s)) => {
            let fiche = proto::Tiers {
                nom: pris(0, &s.nom, &p.nom),
                adresse: pris(1, &s.adresse, &p.adresse),
                telephone: pris(2, &s.telephone, &p.telephone),
                localisation: pris(3, &s.localisation, &p.localisation),
                ifu: pris(4, &s.ifu, &p.ifu),
                rccm: pris(5, &s.rccm, &p.rccm),
                regime_imposition: pris(6, &s.regime_imposition, &p.regime_imposition),
                type_tiers: pris(7, &s.type_tiers, &p.type_tiers),
            };
            if &fiche == s {
                return None;
            }
            if &fiche == p {
                return Some(Box::new(action.clone()));
            }
            Action::ModifierTiers {
                uid: uid.clone(),
                fiche,
            }
        }
        (Action::ModifierProduit { uid, fiche: p }, FicheServeur::Produit(s)) => {
            let fiche = proto::Produit {
                designation: pris(0, &s.designation, &p.designation),
                reference: pris(1, &s.reference, &p.reference),
                prix: if poste(2) { p.prix } else { s.prix },
                sens: pris(3, &s.sens, &p.sens),
            };
            if &fiche == s {
                return None;
            }
            if &fiche == p {
                return Some(Box::new(action.clone()));
            }
            Action::ModifierProduit {
                uid: uid.clone(),
                fiche,
            }
        }
        // Genre inattendu : la version du serveur.
        _ => return None,
    };
    Some(Box::new(composee))
}

#[cfg(test)]
mod essais {
    use super::*;

    fn tiers(nom: &str, telephone: &str) -> proto::Tiers {
        proto::Tiers {
            nom: nom.into(),
            telephone: telephone.into(),
            type_tiers: "client".into(),
            ..proto::Tiers::default()
        }
    }

    #[test]
    fn choix_champ_par_champ() {
        let serveur = tiers("Kaboré SARL", "70 00 00 00");
        let poste = tiers("Kaboré et fils", "76 11 11 11");
        let action = Action::ModifierTiers {
            uid: "t1".into(),
            fiche: poste.clone(),
        };
        let s = FicheServeur::Tiers(serveur.clone());
        // Rien du poste : la version du serveur.
        assert_eq!(composer(&action, &s, &[false; 8]), None);
        assert_eq!(composer(&action, &s, &[]), None);
        // Tout ce qui diffère vient du poste : l'action d'origine.
        let mut tout = [false; 8];
        tout[0] = true;
        tout[2] = true;
        assert_eq!(composer(&action, &s, &tout), Some(Box::new(action.clone())));
        assert_eq!(
            composer(&action, &s, &[true; 8]),
            Some(Box::new(action.clone()))
        );
        // Mélange : même genre, même ligne, champs composés.
        let mut melange = [false; 8];
        melange[2] = true;
        assert_eq!(
            composer(&action, &s, &melange),
            Some(Box::new(Action::ModifierTiers {
                uid: "t1".into(),
                fiche: tiers("Kaboré SARL", "76 11 11 11"),
            }))
        );
    }

    #[test]
    fn choix_sur_un_produit() {
        let serveur = proto::Produit {
            designation: "Ciment".into(),
            reference: "C1".into(),
            prix: 5000,
            sens: "vente".into(),
        };
        let poste = proto::Produit {
            prix: 5500,
            reference: "C2".into(),
            ..serveur.clone()
        };
        let action = Action::ModifierProduit {
            uid: "p1".into(),
            fiche: poste,
        };
        let s = FicheServeur::Produit(serveur.clone());
        let mut prix = [false; 4];
        prix[2] = true;
        assert_eq!(
            composer(&action, &s, &prix),
            Some(Box::new(Action::ModifierProduit {
                uid: "p1".into(),
                fiche: proto::Produit {
                    prix: 5500,
                    ..serveur
                },
            }))
        );
        // Fiche d'un autre genre : la version du serveur.
        assert_eq!(
            composer(
                &action,
                &FicheServeur::Tiers(proto::Tiers::default()),
                &[true; 8]
            ),
            None
        );
    }

    #[test]
    fn comparaison_des_champs() {
        let c = champs(
            &LIBELLES_PRODUIT,
            Some(["Ciment".into(), "C1".into(), "5 000".into(), "Vente".into()]),
            ["Ciment".into(), "C2".into(), "5 000".into(), "Vente".into()],
        );
        assert_eq!(c.len(), 4);
        let differents: Vec<bool> = c.iter().map(Champ::different).collect();
        assert_eq!(differents, [false, true, false, false]);
        // Livraison : seuls les champs remplis d'un côté.
        let l = champs_livraison(
            [
                String::new(),
                String::new(),
                "2026-11-10".into(),
                String::new(),
            ],
            [
                String::new(),
                "Visa".into(),
                "2026-11-10".into(),
                String::new(),
            ],
        );
        assert_eq!(l.len(), 2);
    }

    #[test]
    fn libelles_des_champs_traduits() {
        let mut textes: Vec<&str> = LIBELLES_TIERS.into_iter().chain(LIBELLES_PRODUIT).collect();
        textes.extend([
            "Date",
            "Tiers",
            "Libellé",
            "Remise",
            "TVA",
            "Retenue",
            "Mode de paiement",
            "Observations",
            "Échéance",
            "Lignes",
            "Montant",
            "Visa du client",
            "Visa du fournisseur",
            "Date de livraison",
            "Date de réception",
        ]);
        let types: Vec<String> = [
            TypeTiers::Client,
            TypeTiers::Fournisseur,
            TypeTiers::LesDeux,
        ]
        .iter()
        .map(ToString::to_string)
        .chain(
            [Sens::Vente, Sens::Achat, Sens::LesDeux]
                .iter()
                .map(ToString::to_string),
        )
        .collect();
        textes.extend(types.iter().map(String::as_str));
        let manquants = crate::traduction::sans_traduction(&textes);
        assert!(manquants.is_empty(), "{manquants:#?}");
    }

    #[test]
    fn heures_lisibles() {
        assert_eq!(heure_serveur("2026-11-13T08:05:00"), "13/11/2026 08:05");
        assert_eq!(heure_serveur("illisible"), "illisible");
        assert_eq!(date_texte("2026-11-13"), "13/11/2026");
        assert!(heure_poste("2026-11-13T08:05:00+00:00").starts_with("13/11/2026"));
    }
}
