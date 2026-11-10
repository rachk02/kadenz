//! Panneau d'import des tiers et des produits depuis un fichier CSV ou Excel.

use std::cell::RefCell;
use std::collections::HashSet;
use std::path::Path;
use std::rc::Rc;

use kadenz_import::analyse::{self, Analyse, LigneAnalysee, Statut, cle_produit, cle_tiers};
use kadenz_import::lecture::{self, Table};
use kadenz_stockage::produits::{self, FicheProduit, FiltreProduits, Sens};
use kadenz_stockage::tiers::{self, FicheTiers, FiltreTiers, TypeTiers};
use kadenz_stockage::{BilanImport, Connexion};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::referentiels::{
    afficher_produits, afficher_tiers, modele_lignes, retour_erreur, retour_ok, texte_erreur,
};
use crate::traduction::{tr, trf};
use crate::{EtatPartage, FenetrePrincipale, ImportApi, Retour, phrase};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Cible {
    Tiers,
    Produits,
}

enum Apercu {
    Tiers(Analyse<FicheTiers>),
    Produits(Analyse<FicheProduit>),
}

struct ImportEnCours {
    cible: Cible,
    table: Option<Table>,
    apercu: Option<Apercu>,
    /// Rang du type (tiers) ou du sens (produits) appliqué aux lignes qui n'en ont pas.
    defaut: i32,
}

type Partage = Rc<RefCell<ImportEnCours>>;

fn cellules<F>(ligne: &LigneAnalysee<F>) -> Vec<String> {
    let (etat, remarque) = match &ligne.statut {
        Statut::Valide => ("À importer", String::new()),
        Statut::Doublon(r) => ("Doublon", r.clone()),
        Statut::Erreur(r) => ("Erreur", r.clone()),
    };
    vec![
        ligne.numero.to_string(),
        tr(etat),
        ligne.libelle.clone(),
        phrase(remarque).to_string(),
    ]
}

fn afficher_apercu<F: Clone>(api: &ImportApi, a: &Analyse<F>) {
    api.set_lignes(modele_lignes(a.lignes.iter().map(cellules).collect()));
    let (valides, doublons, erreurs) = a.comptes();
    api.set_resume(
        trf(
            "Fiches à importer : {} ; doublons ignorés : {} ; lignes en erreur : {}.",
            &[&valides, &doublons, &erreurs],
        )
        .into(),
    );
    let reconnues: Vec<String> = a
        .colonnes_reconnues
        .iter()
        .map(|(entete, champ)| format!("{entete} ({})", tr(champ)))
        .collect();
    let mut colonnes = trf("Colonnes reconnues : {}.", &[&reconnues.join(", ")]);
    if !a.colonnes_ignorees.is_empty() {
        colonnes.push(' ');
        colonnes.push_str(&trf(
            "Colonnes ignorées, non importées : {}.",
            &[&a.colonnes_ignorees.join(", ")],
        ));
    }
    api.set_colonnes(colonnes.into());
    api.set_peut_importer(valides > 0);
}

fn vider_apercu(api: &ImportApi) {
    api.set_lignes(modele_lignes(Vec::new()));
    api.set_resume(SharedString::new());
    api.set_colonnes(SharedString::new());
    api.set_peut_importer(false);
}

/// Analyse la table lue avec les fiches déjà en base, et met l'aperçu à jour.
fn analyser(api: &ImportApi, conn: &Connexion, import: &mut ImportEnCours) {
    let Some(table) = &import.table else {
        vider_apercu(api);
        return;
    };
    let tous_tiers = FiltreTiers {
        inclure_inactifs: true,
        ..Default::default()
    };
    let tous_produits = FiltreProduits {
        inclure_inactifs: true,
        ..Default::default()
    };
    let rang = usize::try_from(import.defaut).unwrap_or(0);
    let resultat = match import.cible {
        Cible::Tiers => tiers::lister(conn, &tous_tiers)
            .map_err(texte_erreur)
            .and_then(|existants| {
                let cles: HashSet<String> =
                    existants.iter().map(|t| cle_tiers(&t.fiche.nom)).collect();
                let defaut = TypeTiers::TOUS[rang.min(TypeTiers::TOUS.len() - 1)];
                analyse::analyser_tiers(table, defaut, &cles).map(Apercu::Tiers)
            }),
        Cible::Produits => produits::lister(conn, &tous_produits)
            .map_err(texte_erreur)
            .and_then(|existants| {
                let cles: HashSet<String> = existants
                    .iter()
                    .map(|p| cle_produit(&p.fiche.reference))
                    .collect();
                let defaut = Sens::TOUS[rang.min(Sens::TOUS.len() - 1)];
                analyse::analyser_produits(table, defaut, &cles).map(Apercu::Produits)
            }),
    };
    match resultat {
        Ok(apercu) => {
            api.set_erreur(SharedString::new());
            match &apercu {
                Apercu::Tiers(a) => afficher_apercu(api, a),
                Apercu::Produits(a) => afficher_apercu(api, a),
            }
            import.apercu = Some(apercu);
        }
        Err(message) => {
            api.set_erreur(phrase(message));
            vider_apercu(api);
            import.apercu = None;
        }
    }
}

/// Ouvre le dialogue de fichier (aucun emprunt tenu pendant qu'il est ouvert),
/// lit le fichier choisi et prépare l'aperçu.
fn choisir_et_analyser(fenetre: &FenetrePrincipale, etat: &EtatPartage, import: &Partage) {
    let choix = rfd::FileDialog::new()
        .set_title(tr("Choisir le fichier à importer"))
        .add_filter(
            tr("CSV, Excel ou OpenDocument"),
            &["csv", "txt", "xlsx", "xlsm", "xls", "ods"],
        )
        .set_parent(&fenetre.window().window_handle())
        .pick_file();
    let Some(chemin) = choix else { return };
    let api = fenetre.global::<ImportApi>();
    api.set_retour(Retour::default());
    api.set_fichier(nom_de_fichier(&chemin).into());
    let mut import = import.borrow_mut();
    match lecture::lire_fichier(&chemin) {
        Ok(table) => {
            import.table = Some(table);
            analyser(&api, &e_conn(etat), &mut import);
        }
        Err(message) => {
            import.table = None;
            import.apercu = None;
            api.set_erreur(phrase(message));
            vider_apercu(&api);
        }
    }
}

fn e_conn(etat: &EtatPartage) -> std::cell::Ref<'_, Connexion> {
    std::cell::Ref::map(etat.borrow(), |e| &e.conn)
}

fn nom_de_fichier(chemin: &Path) -> String {
    chemin
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn message_bilan(bilan: &BilanImport) -> String {
    let mut message = trf("Fiches ajoutées : {}", &[&bilan.ajoutees]);
    if !bilan.ignorees.is_empty() {
        message.push_str(&trf(
            " ; ignorées car déjà présentes au moment de l'enregistrement : {}",
            &[&bilan.ignorees.len()],
        ));
    }
    message
}

pub(crate) fn brancher(fenetre: &FenetrePrincipale, etat: &EtatPartage) {
    let api = fenetre.global::<ImportApi>();
    let import: Partage = Rc::new(RefCell::new(ImportEnCours {
        cible: Cible::Tiers,
        table: None,
        apercu: None,
        defaut: 0,
    }));

    let (f, e, i) = (fenetre.as_weak(), etat.clone(), import.clone());
    api.on_demarrer(move |cible| {
        let Some(f) = f.upgrade() else { return };
        let api = f.global::<ImportApi>();
        let cible = if cible == 1 {
            Cible::Produits
        } else {
            Cible::Tiers
        };
        let (titre, libelle, choix): (String, String, Vec<SharedString>) = match cible {
            Cible::Tiers => (
                tr("Importer des tiers"),
                tr("Type des lignes sans colonne Type :"),
                TypeTiers::TOUS
                    .iter()
                    .map(|t| tr(&t.to_string()).into())
                    .collect(),
            ),
            Cible::Produits => (
                tr("Importer des produits"),
                tr("Sens des lignes sans colonne Sens :"),
                Sens::TOUS
                    .iter()
                    .map(|s| tr(&s.to_string()).into())
                    .collect(),
            ),
        };
        *i.borrow_mut() = ImportEnCours {
            cible,
            table: None,
            apercu: None,
            defaut: 0,
        };
        api.set_titre(titre.into());
        api.set_libelle_defaut(libelle.into());
        api.set_choix_defaut(ModelRc::new(VecModel::from(choix)));
        api.set_defaut(0);
        api.set_fichier(SharedString::new());
        api.set_erreur(SharedString::new());
        api.set_retour(Retour::default());
        vider_apercu(&api);
        api.set_actif(true);
        choisir_et_analyser(&f, &e, &i);
    });

    let (f, e, i) = (fenetre.as_weak(), etat.clone(), import.clone());
    api.on_choisir_fichier(move || {
        if let Some(f) = f.upgrade() {
            choisir_et_analyser(&f, &e, &i);
        }
    });

    let (f, e, i) = (fenetre.as_weak(), etat.clone(), import.clone());
    api.on_changer_defaut(move |rang| {
        let Some(f) = f.upgrade() else { return };
        let mut import = i.borrow_mut();
        import.defaut = rang;
        analyser(&f.global::<ImportApi>(), &e_conn(&e), &mut import);
    });

    let (f, e, i) = (fenetre.as_weak(), etat.clone(), import.clone());
    api.on_importer(move || {
        let Some(f) = f.upgrade() else { return };
        let api = f.global::<ImportApi>();
        let mut etat = e.borrow_mut();
        let Some(acteur) = etat.utilisateur.clone() else {
            api.set_retour(retour_erreur("aucune session ouverte"));
            return;
        };
        let import = i.borrow();
        let resultat = match &import.apercu {
            Some(Apercu::Tiers(a)) => tiers::importer(&mut etat.conn, &acteur, &a.fiches_valides()),
            Some(Apercu::Produits(a)) => {
                produits::importer(&mut etat.conn, &acteur, &a.fiches_valides())
            }
            None => return,
        };
        match resultat {
            Ok(bilan) => {
                api.set_peut_importer(false);
                api.set_retour(retour_ok(message_bilan(&bilan)));
                match import.cible {
                    Cible::Tiers => afficher_tiers(&f, &mut etat),
                    Cible::Produits => afficher_produits(&f, &mut etat),
                }
            }
            Err(erreur) => api.set_retour(retour_erreur(erreur)),
        }
    });

    let (f, i) = (fenetre.as_weak(), import);
    api.on_fermer(move || {
        let mut import = i.borrow_mut();
        import.table = None;
        import.apercu = None;
        if let Some(f) = f.upgrade() {
            f.global::<ImportApi>().set_actif(false);
        }
    });
}
