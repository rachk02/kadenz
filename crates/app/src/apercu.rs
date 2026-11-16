//! Aperçu d'un document dans Kadenz (docs/spec-apercu-impression.md, jalon A1).
//!
//! Le PDF généré une fois par `crate::pdf::generer` est rendu page par page par
//! `hayro`. Seules les pages visibles et leurs voisines sont gardées en image ;
//! les autres sont libérées et rendues de nouveau quand elles reviennent à
//! l'écran. « Enregistrer en PDF » écrit les mêmes octets.

use std::cell::RefCell;
use std::rc::Rc;

use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{PixmapSettings, RenderCache, RenderSettings, render};
use slint::{ComponentHandle, Image, Model, ModelRc, Rgba8Pixel, SharedPixelBuffer, VecModel};

use crate::referentiels::{retour_erreur, retour_ok};
use crate::traduction::{tr, trf};
use crate::{ApercuApi, EtatPartage, FacturesApi, FenetrePrincipale, PageApercuUi, Retour};

/// Largeur d'affichage maximale d'une page, en pixels logiques (apercu.slint).
const LARGEUR_AFFICHEE: f32 = 820.0;

/// Largeur de rendu d'une page, en pixels : au moins environ 150 ppp pour une
/// page A4, plus sur un écran agrandi (820 px logiques à 200 % en font 1640),
/// au plus 300 ppp.
const LARGEUR_RENDU_MIN: f32 = 1240.0;
const LARGEUR_RENDU_MAX: f32 = 2480.0;

/// Pages gardées en image de part et d'autre de la plage visible.
const VOISINES: usize = 1;

/// Rapport hauteur sur largeur d'une page A4, si une page n'a pas de dimensions.
const RATIO_A4: f32 = 297.0 / 210.0;

struct Apercu {
    pdf: Pdf,
    // Octets du PDF, pour « Enregistrer en PDF ».
    octets: Vec<u8>,
    nom: String,
    dossier: Option<String>,
    modele: Rc<VecModel<PageApercuUi>>,
    rendues: Vec<bool>,
    // Largeur de rendu en pixels, d'après l'échelle de l'écran à l'ouverture.
    largeur_rendu: f32,
}

thread_local! {
    static APERCU: RefCell<Option<Apercu>> = const { RefCell::new(None) };
}

/// Ouvre l'aperçu d'un PDF, avec le nom de fichier et le dossier proposés à
/// l'enregistrement.
pub(crate) fn ouvrir(
    fenetre: &FenetrePrincipale,
    octets: Vec<u8>,
    nom: String,
    dossier: Option<String>,
) -> Result<(), String> {
    let pdf = Pdf::new(octets.clone()).map_err(|e| format!("{e:?}"))?;
    let ratios: Vec<f32> = pdf
        .pages()
        .iter()
        .map(|page| {
            let (l, h) = page.render_dimensions();
            if l > 0.0 && h > 0.0 { h / l } else { RATIO_A4 }
        })
        .collect();
    if ratios.is_empty() {
        return Err(tr("document sans page"));
    }
    let modele = Rc::new(VecModel::from(
        ratios
            .iter()
            .map(|&ratio| PageApercuUi {
                image: Image::default(),
                ratio,
            })
            .collect::<Vec<_>>(),
    ));
    let api = fenetre.global::<ApercuApi>();
    api.set_titre(nom.trim_end_matches(".pdf").into());
    api.set_resume(trf("{} page(s)", &[&ratios.len()]).into());
    api.set_pages(ModelRc::from(modele.clone()));
    APERCU.with(|a| {
        *a.borrow_mut() = Some(Apercu {
            pdf,
            octets,
            nom,
            dossier,
            modele,
            rendues: vec![false; ratios.len()],
            largeur_rendu: largeur_rendu(fenetre.window().scale_factor()),
        });
    });
    // Premières pages tout de suite : la plage visible ne change pas à l'ouverture.
    afficher_plage(0, 1);
    api.set_ouvert(true);
    Ok(())
}

/// Rend les pages de `premiere` à `derniere` et leurs voisines, libère les autres.
fn afficher_plage(premiere: usize, derniere: usize) {
    APERCU.with(|a| {
        let mut a = a.borrow_mut();
        let Some(apercu) = a.as_mut() else {
            return;
        };
        let nombre = apercu.rendues.len();
        let debut = premiere.saturating_sub(VOISINES);
        let fin = (derniere.max(premiere) + VOISINES).min(nombre - 1);
        for rang in 0..nombre {
            let garder = (debut..=fin).contains(&rang);
            if garder && !apercu.rendues[rang] {
                if let Some(image) = rendre(apercu, rang) {
                    poser_image(&apercu.modele, rang, image);
                    apercu.rendues[rang] = true;
                }
            } else if !garder && apercu.rendues[rang] {
                poser_image(&apercu.modele, rang, Image::default());
                apercu.rendues[rang] = false;
            }
        }
    });
}

fn poser_image(modele: &VecModel<PageApercuUi>, rang: usize, image: Image) {
    if let Some(mut page) = modele.row_data(rang) {
        page.image = image;
        modele.set_row_data(rang, page);
    }
}

fn largeur_rendu(echelle_ecran: f32) -> f32 {
    (LARGEUR_AFFICHEE * echelle_ecran).clamp(LARGEUR_RENDU_MIN, LARGEUR_RENDU_MAX)
}

fn rendre(apercu: &Apercu, rang: usize) -> Option<Image> {
    rendre_page(&apercu.pdf, rang, apercu.largeur_rendu, |pixels, l, h| {
        Image::from_rgba8(SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
            pixels, l, h,
        ))
    })
}

/// Rend la page `rang` sur fond blanc, à `largeur_px` pixels de large, et passe
/// ses pixels RGBA, sa largeur et sa hauteur à `f`.
fn rendre_page<T>(
    pdf: &Pdf,
    rang: usize,
    largeur_px: f32,
    f: impl FnOnce(&[u8], u32, u32) -> T,
) -> Option<T> {
    let pages = pdf.pages();
    let page = pages.get(rang)?;
    let (largeur, _) = page.render_dimensions();
    let echelle = largeur_px / largeur.max(1.0);
    let reglages = PixmapSettings {
        x_scale: echelle,
        y_scale: echelle,
        bg_color: WHITE,
    };
    let cache = RenderCache::new();
    let pixmap = render(
        page,
        &cache,
        &InterpreterSettings::default(),
        &RenderSettings::default(),
        &reglages,
    );
    // Fond blanc opaque : les pixels prémultipliés valent les pixels droits.
    Some(f(
        pixmap.data_as_u8_slice(),
        u32::from(pixmap.width()),
        u32::from(pixmap.height()),
    ))
}

/// Enregistre un PDF par la boîte de dialogue de Windows, dans le dossier
/// proposé s'il existe, puis l'ouvre dans la visionneuse par défaut.
pub(crate) fn enregistrer_pdf(octets: &[u8], nom: &str, dossier: Option<&str>) -> Retour {
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
    let _ = std::process::Command::new("explorer").arg(&chemin).spawn();
    retour_ok(trf("PDF enregistré : {}", &[&chemin.display()]))
}

fn fermer(fenetre: &FenetrePrincipale) {
    let api = fenetre.global::<ApercuApi>();
    api.set_ouvert(false);
    api.set_pages(ModelRc::default());
    APERCU.with(|a| *a.borrow_mut() = None);
    // Le focus partait avec la fenêtre d'aperçu : les raccourcis de la page
    // (Ctrl+P, Ctrl+S) ne répondaient plus (essai A1).
    let factures = fenetre.global::<FacturesApi>();
    factures.set_demande_focus(factures.get_demande_focus() + 1);
}

pub(crate) fn brancher(fenetre: &FenetrePrincipale, _etat: &EtatPartage) {
    let api = fenetre.global::<ApercuApi>();

    api.on_visibles(|premiere, derniere| {
        afficher_plage(premiere.max(0) as usize, derniere.max(0) as usize);
    });

    api.on_enregistrer(|| {
        // Copie avant la boîte de dialogue : elle fait tourner la boucle des
        // événements, qui peut rendre des pages pendant qu'elle est ouverte.
        let copie = APERCU.with(|a| {
            a.borrow().as_ref().map(|apercu| {
                (
                    apercu.octets.clone(),
                    apercu.nom.clone(),
                    apercu.dossier.clone(),
                )
            })
        });
        match copie {
            Some((octets, nom, dossier)) => enregistrer_pdf(&octets, &nom, dossier.as_deref()),
            None => Retour::default(),
        }
    });

    let f = fenetre.as_weak();
    api.on_fermer(move || {
        if let Some(f) = f.upgrade() {
            fermer(&f);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use kadenz_domaine::calcul::{Taux, UNE_UNITE};
    use kadenz_domaine::document::TypeDocument;
    use kadenz_domaine::role::Role;
    use kadenz_stockage::comptes::NouvelUtilisateur;
    use kadenz_stockage::documents::{self, LigneSaisie, Saisie};
    use kadenz_stockage::entreprise::Entreprise;
    use kadenz_stockage::installation::{self, InstallationInitiale};
    use kadenz_stockage::parametres::Parametres;
    use kadenz_stockage::tiers::{self, FicheTiers, TypeTiers};

    /// Essai A1 sans écran : un brouillon de devis et un devis de plusieurs pages
    /// sont produits comme pour l'aperçu, puis chaque page est rendue comme à
    /// l'écran. Avec la variable KADENZ_ESSAI_APERCU (un dossier), les pages y
    /// sont écrites en RGBA brut (nom-page-largeurxhauteur.rgba) pour un contrôle
    /// à l'oeil.
    #[test]
    fn largeur_de_rendu_selon_l_ecran() {
        assert_eq!(largeur_rendu(1.0), LARGEUR_RENDU_MIN);
        assert_eq!(largeur_rendu(2.0), 1640.0);
        assert_eq!(largeur_rendu(4.0), LARGEUR_RENDU_MAX);
    }

    #[test]
    fn pages_de_l_apercu() {
        let mut conn = kadenz_stockage::ouvrir_en_memoire().unwrap();
        let admin = installation::installer(
            &mut conn,
            &InstallationInitiale {
                entreprise: Entreprise {
                    nom: "Entreprise Essai".into(),
                    localisation: "Ville Fictive".into(),
                    responsable_comptabilite: "Nom Fictif".into(),
                    ..Default::default()
                },
                admin: NouvelUtilisateur {
                    nom_complet: "Admin Essai".into(),
                    identifiant: "admin".into(),
                    mot_de_passe: "motdepasse1".into(),
                    role: Role::Admin,
                },
                parametres: Parametres::default(),
            },
        )
        .unwrap();
        let mut fiche = FicheTiers::vide(TypeTiers::Client);
        fiche.nom = "Client Alpha".into();
        let client = tiers::creer(&conn, &admin, &fiche).unwrap().id;
        let ligne = |n: usize| LigneSaisie {
            designation: format!("Article d'essai {n}"),
            prix: 100_000,
            quantite: 2 * UNE_UNITE,
            ..Default::default()
        };
        let saisie = |libelle: &str, lignes: usize| Saisie {
            tiers_id: Some(client),
            libelle: libelle.into(),
            taux: Taux {
                remise: 0,
                tva: 0,
                retenue: 0,
            },
            lignes: (1..=lignes).map(ligne).collect(),
            ..Saisie::vide(chrono::Local::now().date_naive())
        };
        let dossier = std::env::var("KADENZ_ESSAI_APERCU").ok();
        for (libelle, lignes, pages_minimum) in
            [("Essai aperçu A1", 1, 1), ("Essai aperçu A1 long", 80, 2)]
        {
            let id = documents::creer_brouillon(
                &mut conn,
                &admin,
                TypeDocument::Devis,
                &saisie(libelle, lignes),
            )
            .unwrap();
            let (octets, nom) = crate::pdf::generer(&conn, id).unwrap();
            assert!(nom.starts_with("Apercu-"), "nom d'un brouillon : {nom}");
            let pdf = Pdf::new(octets).unwrap();
            let nombre = pdf.pages().len();
            assert!(nombre >= pages_minimum, "{libelle} : {nombre} page(s)");
            for rang in 0..nombre {
                let debut = std::time::Instant::now();
                let (l, h, sombres) = rendre_page(&pdf, rang, LARGEUR_RENDU_MIN, |pixels, l, h| {
                    if let Some(dossier) = &dossier {
                        let fichier = format!(
                            "{}-p{}-{l}x{h}.rgba",
                            nom.trim_end_matches(".pdf"),
                            rang + 1
                        );
                        std::fs::write(std::path::Path::new(dossier).join(fichier), pixels)
                            .unwrap();
                    }
                    (l, h, pixels.chunks(4).filter(|p| p[0] < 128).count())
                })
                .unwrap();
                eprintln!("{nom}, page {} : {:?}", rang + 1, debut.elapsed());
                assert_eq!(l, LARGEUR_RENDU_MIN as u32);
                assert!((1740..=1760).contains(&h), "proportions A4 : {h}");
                assert!(sombres > 1000, "{libelle}, page {} blanche", rang + 1);
            }
        }
    }
}
