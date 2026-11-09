//! Écrans Entreprise (fiche et logos) et Paramètres.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::Datelike;
use kadenz_domaine::document::TypeDocument;
use kadenz_domaine::lettres::{Orthographe, en_lettres};
use kadenz_domaine::numerotation::{CHIFFRES_MAX, CHIFFRES_MIN, Numerotation, RemiseAZero};
use kadenz_domaine::saisie;
use kadenz_stockage::entreprise::{self, Entreprise, FormatImage, Logo};
use kadenz_stockage::parametres::{self, Parametres};
use slint::{ComponentHandle, Image, SharedString};

use crate::referentiels::{retour_erreur, retour_ok};
use crate::{
    EntrepriseApi, EtatPartage, FenetrePrincipale, FicheEntrepriseUi, ParametresApi, ParametresUi,
    Retour,
};

// ---------------------------------------------------------------- Entreprise

fn fiche_ui(e: &Entreprise) -> FicheEntrepriseUi {
    FicheEntrepriseUi {
        nom: e.nom.as_str().into(),
        adresse: e.adresse.as_str().into(),
        telephone: e.telephone.as_str().into(),
        email: e.email.as_str().into(),
        site_web: e.site_web.as_str().into(),
        localisation: e.localisation.as_str().into(),
        regime: e.regime_imposition.as_str().into(),
        rccm: e.rccm.as_str().into(),
        ifu: e.ifu.as_str().into(),
        banque1: e.banque1.as_str().into(),
        banque2: e.banque2.as_str().into(),
        responsable_facturation: e.responsable_facturation.as_str().into(),
        responsable_comptabilite: e.responsable_comptabilite.as_str().into(),
    }
}

fn fiche_depuis_ui(u: &FicheEntrepriseUi) -> Entreprise {
    Entreprise {
        nom: u.nom.to_string(),
        adresse: u.adresse.to_string(),
        telephone: u.telephone.to_string(),
        email: u.email.to_string(),
        site_web: u.site_web.to_string(),
        localisation: u.localisation.to_string(),
        regime_imposition: u.regime.to_string(),
        rccm: u.rccm.to_string(),
        ifu: u.ifu.to_string(),
        banque1: u.banque1.to_string(),
        banque2: u.banque2.to_string(),
        responsable_facturation: u.responsable_facturation.to_string(),
        responsable_comptabilite: u.responsable_comptabilite.to_string(),
    }
}

fn logo_de_rang(rang: i32) -> Logo {
    if rang == 1 {
        Logo::Secondaire
    } else {
        Logo::Principal
    }
}

/// Image affichable depuis les octets stockés en base.
///
/// Slint charge les images depuis un fichier : les octets passent par un fichier
/// temporaire au nom unique (le cache d'images de Slint est indexé par chemin),
/// supprimé aussitôt chargé.
fn image_depuis_octets(octets: &[u8]) -> Option<Image> {
    static COMPTEUR: AtomicU64 = AtomicU64::new(0);
    let format = FormatImage::detecter(octets)?;
    let chemin: PathBuf = std::env::temp_dir().join(format!(
        "kadenz-logo-{}-{}.{}",
        std::process::id(),
        COMPTEUR.fetch_add(1, Ordering::Relaxed),
        format.extension()
    ));
    std::fs::write(&chemin, octets).ok()?;
    let image = Image::load_from_path(&chemin).ok();
    let _ = std::fs::remove_file(&chemin);
    image
}

fn afficher_logos(fenetre: &FenetrePrincipale, conn: &kadenz_stockage::Connexion) {
    let api = fenetre.global::<EntrepriseApi>();
    for logo in [Logo::Principal, Logo::Secondaire] {
        let image = entreprise::lire_logo(conn, logo)
            .ok()
            .flatten()
            .and_then(|o| image_depuis_octets(&o));
        let present = image.is_some();
        let image = image.unwrap_or_default();
        match logo {
            Logo::Principal => {
                api.set_logo_principal(image);
                api.set_a_logo_principal(present);
            }
            Logo::Secondaire => {
                api.set_logo_secondaire(image);
                api.set_a_logo_secondaire(present);
            }
        }
    }
}

fn brancher_entreprise(fenetre: &FenetrePrincipale, etat: &EtatPartage) {
    let api = fenetre.global::<EntrepriseApi>();

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_ouvrir(move || {
        let etat = e.borrow();
        if let Some(f) = f.upgrade() {
            afficher_logos(&f, &etat.conn);
        }
        entreprise::lire(&etat.conn)
            .ok()
            .flatten()
            .map(|e| fiche_ui(&e))
            .unwrap_or_default()
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_enregistrer(move |saisie| {
        let etat = e.borrow();
        let Some(acteur) = &etat.utilisateur else {
            return retour_erreur("aucune session ouverte");
        };
        let fiche = fiche_depuis_ui(&saisie);
        match entreprise::enregistrer(&etat.conn, acteur, &fiche) {
            Ok(()) => {
                if let Some(f) = f.upgrade() {
                    f.set_nom_entreprise(fiche.nom.trim().into());
                }
                retour_ok("fiche enregistrée")
            }
            Err(erreur) => retour_erreur(erreur),
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_choisir_logo(move |rang| {
        let Some(f) = f.upgrade() else {
            return Retour::default();
        };
        // Le dialogue est modal et tourne sa propre boucle de messages : aucun
        // emprunt de l'état ne doit être tenu pendant qu'il est ouvert.
        let choix = rfd::FileDialog::new()
            .set_title("Choisir un logo")
            .add_filter("Images PNG ou JPEG", &["png", "jpg", "jpeg"])
            .set_parent(&f.window().window_handle())
            .pick_file();
        let Some(chemin) = choix else {
            return Retour::default();
        };
        let octets = match std::fs::read(&chemin) {
            Ok(octets) => octets,
            Err(erreur) => {
                return retour_erreur(format!("lecture du fichier impossible : {erreur}"));
            }
        };
        let etat = e.borrow();
        let Some(acteur) = &etat.utilisateur else {
            return retour_erreur("aucune session ouverte");
        };
        let resultat =
            entreprise::definir_logo(&etat.conn, acteur, logo_de_rang(rang), Some(&octets));
        afficher_logos(&f, &etat.conn);
        match resultat {
            Ok(()) => retour_ok("logo enregistré"),
            Err(erreur) => retour_erreur(erreur),
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_retirer_logo(move |rang| {
        let etat = e.borrow();
        let Some(acteur) = &etat.utilisateur else {
            return retour_erreur("aucune session ouverte");
        };
        let resultat = entreprise::definir_logo(&etat.conn, acteur, logo_de_rang(rang), None);
        if let Some(f) = f.upgrade() {
            afficher_logos(&f, &etat.conn);
        }
        match resultat {
            Ok(()) => retour_ok("logo retiré"),
            Err(erreur) => retour_erreur(erreur),
        }
    });
}

// ---------------------------------------------------------------- Paramètres

/// Montant d'exemple pour comparer les deux orthographes.
const EXEMPLE_MONTANT: i64 = 1_221_380;

fn parametres_ui(p: &Parametres) -> ParametresUi {
    let jours =
        |j: Option<u32>| -> SharedString { j.map(|j| j.to_string()).unwrap_or_default().into() };
    ParametresUi {
        taux_tva: saisie::liste_pourcentages_en_texte(&p.taux_tva).into(),
        taux_remise: saisie::liste_pourcentages_en_texte(&p.taux_remise).into(),
        taux_retenue: saisie::liste_pourcentages_en_texte(&p.taux_retenue).into(),
        modes_paiement: p.modes_paiement.join(" ; ").into(),
        orthographe: match p.orthographe {
            Orthographe::Rectifiee => 0,
            Orthographe::Traditionnelle => 1,
        },
        validite_devis: jours(p.validite_devis_jours),
        delai_paiement: jours(p.delai_paiement_jours),
        prefixe_devis: p.numerotation.prefixes[0].clone().into(),
        prefixe_proforma: p.numerotation.prefixes[1].clone().into(),
        prefixe_commande: p.numerotation.prefixes[2].clone().into(),
        prefixe_livraison: p.numerotation.prefixes[3].clone().into(),
        prefixe_facture: p.numerotation.prefixes[4].clone().into(),
        prefixe_avoir: p.numerotation.prefixes[5].clone().into(),
        remise_annuelle: p.numerotation.remise == RemiseAZero::Annuelle,
        chiffres: p.numerotation.chiffres.to_string().into(),
    }
}

fn parametres_depuis_ui(u: &ParametresUi) -> Result<Parametres, String> {
    let taux = |libelle: &str, texte: &SharedString| {
        saisie::liste_pourcentages_depuis_texte(texte).ok_or_else(|| {
            format!(
                "taux de {libelle} invalides : saisir des pourcentages entre 0 et 100 séparés par des points-virgules"
            )
        })
    };
    let jours = |libelle: &str, texte: &SharedString| -> Result<Option<u32>, String> {
        let texte = texte.trim();
        if texte.is_empty() {
            return Ok(None);
        }
        match texte.parse::<u32>() {
            Ok(j) if j > 0 => Ok(Some(j)),
            _ => Err(format!(
                "{libelle} invalide : saisir un nombre entier de jours, ou laisser vide pour ne pas en fixer"
            )),
        }
    };
    Ok(Parametres {
        taux_tva: taux("TVA", &u.taux_tva)?,
        taux_remise: taux("remise", &u.taux_remise)?,
        taux_retenue: taux("retenue", &u.taux_retenue)?,
        modes_paiement: saisie::liste_libelles_depuis_texte(&u.modes_paiement),
        orthographe: if u.orthographe == 1 {
            Orthographe::Traditionnelle
        } else {
            Orthographe::Rectifiee
        },
        validite_devis_jours: jours("validité des devis", &u.validite_devis)?,
        delai_paiement_jours: jours("délai de paiement", &u.delai_paiement)?,
        numerotation: numerotation_depuis_ui(
            [
                &u.prefixe_devis,
                &u.prefixe_proforma,
                &u.prefixe_commande,
                &u.prefixe_livraison,
                &u.prefixe_facture,
                &u.prefixe_avoir,
            ],
            u.remise_annuelle,
            &u.chiffres,
        )?,
        // Textes et couleur des documents : pas encore à l'écran, repris des
        // valeurs enregistrées par l'appelant.
        ..Parametres::default()
    })
}

/// Réglage de numérotation saisi, contrôlé par le domaine.
fn numerotation_depuis_ui(
    prefixes: [&SharedString; 6],
    annuelle: bool,
    chiffres: &str,
) -> Result<Numerotation, String> {
    let chiffres = chiffres.trim().parse::<u8>().map_err(|_| {
        format!(
            "nombre de chiffres du rang invalide : saisir un entier entre {CHIFFRES_MIN} et {CHIFFRES_MAX}"
        )
    })?;
    Numerotation {
        prefixes: prefixes.map(|p| p.to_string()),
        remise: if annuelle {
            RemiseAZero::Annuelle
        } else {
            RemiseAZero::Mensuelle
        },
        chiffres,
    }
    .nettoyee()
}

fn brancher_parametres(fenetre: &FenetrePrincipale, etat: &EtatPartage) {
    let api = fenetre.global::<ParametresApi>();
    let exemple = |o| {
        format!(
            "Exemple : {} FCFA s'écrit « {} ».",
            saisie::montant_en_texte(EXEMPLE_MONTANT),
            en_lettres(EXEMPLE_MONTANT, o)
        )
    };
    api.set_exemple_rectifiee(exemple(Orthographe::Rectifiee).into());
    api.set_exemple_traditionnelle(exemple(Orthographe::Traditionnelle).into());

    api.on_exemple_numero(|prefixe, annuelle, chiffres| {
        let mut prefixes = Numerotation::default().prefixes.map(SharedString::from);
        prefixes[4] = prefixe;
        let refs = [
            &prefixes[0],
            &prefixes[1],
            &prefixes[2],
            &prefixes[3],
            &prefixes[4],
            &prefixes[5],
        ];
        match numerotation_depuis_ui(refs, annuelle, &chiffres) {
            Ok(n) => {
                let aujourd_hui = chrono::Local::now().date_naive();
                let numero = n.formater(
                    TypeDocument::Facture,
                    u16::try_from(aujourd_hui.year()).unwrap_or(0),
                    aujourd_hui.month() as u8,
                    1,
                );
                if annuelle {
                    format!("Exemple : première facture de l'année, {numero}").into()
                } else {
                    format!("Exemple : première facture du mois, {numero}").into()
                }
            }
            Err(message) => message.into(),
        }
    });

    let e = etat.clone();
    api.on_ouvrir(move || {
        let etat = e.borrow();
        parametres::lire(&etat.conn)
            .map(|p| parametres_ui(&p))
            .unwrap_or_else(|_| parametres_ui(&Parametres::default()))
    });

    let e = etat.clone();
    api.on_enregistrer(move |saisie| {
        let mut p = match parametres_depuis_ui(&saisie) {
            Ok(p) => p,
            Err(message) => return retour_erreur(message),
        };
        let etat = e.borrow();
        // Textes et couleur des documents gardés tels qu'enregistrés (pas encore à l'écran).
        if let Ok(enregistres) = parametres::lire(&etat.conn) {
            p.textes_documents = enregistres.textes_documents;
            p.couleur_documents = enregistres.couleur_documents;
        }
        let Some(acteur) = &etat.utilisateur else {
            return retour_erreur("aucune session ouverte");
        };
        match parametres::enregistrer(&etat.conn, acteur, &p) {
            Ok(()) => retour_ok("paramètres enregistrés"),
            Err(erreur) => retour_erreur(erreur),
        }
    });
}

pub(crate) fn brancher(fenetre: &FenetrePrincipale, etat: &EtatPartage) {
    brancher_entreprise(fenetre, etat);
    brancher_parametres(fenetre, etat);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parametres_saisis() {
        let mut u = parametres_ui(&Parametres::default());
        assert_eq!(parametres_depuis_ui(&u).unwrap(), Parametres::default());
        u.validite_devis = " 30 ".into();
        u.orthographe = 1;
        let p = parametres_depuis_ui(&u).unwrap();
        assert_eq!(p.validite_devis_jours, Some(30));
        assert_eq!(p.orthographe, Orthographe::Traditionnelle);
        for invalide in ["abc", "0", "-3", "2,5"] {
            u.validite_devis = invalide.into();
            assert!(parametres_depuis_ui(&u).is_err(), "{invalide}");
        }
        u.validite_devis = "".into();
        u.taux_tva = "18 ; 120".into();
        assert!(parametres_depuis_ui(&u).is_err());
    }

    #[test]
    fn numerotation_saisie() {
        let mut p = Parametres::default();
        p.numerotation.remise = RemiseAZero::Annuelle;
        let mut u = parametres_ui(&p);
        assert!(u.remise_annuelle);
        assert_eq!(parametres_depuis_ui(&u).unwrap(), p);
        u.prefixe_facture = " fa ".into();
        u.chiffres = "5".into();
        let n = parametres_depuis_ui(&u).unwrap().numerotation;
        assert_eq!(n.prefixes[4], "FA");
        assert_eq!(n.chiffres, 5);
        for invalide in ["", "0", "10", "x"] {
            u.chiffres = invalide.into();
            assert!(parametres_depuis_ui(&u).is_err(), "{invalide}");
        }
        u.chiffres = "4".into();
        u.prefixe_facture = "D".into();
        assert!(parametres_depuis_ui(&u).is_err(), "préfixe en double");
    }
}
