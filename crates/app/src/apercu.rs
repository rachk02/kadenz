//! Aperçu d'un document dans Kadenz (docs/spec-apercu-impression.md, jalon A1).
//!
//! Le PDF généré une fois par `crate::pdf::generer` est rendu page par page par
//! `hayro`. Seules les pages visibles et leurs voisines sont gardées en image ;
//! les autres sont libérées et rendues de nouveau quand elles reviennent à
//! l'écran. « Enregistrer en PDF » écrit les mêmes octets ; « Imprimer » (jalon
//! A2, `crate::imprimante`) les envoie à l'imprimante.

use std::cell::RefCell;
use std::rc::Rc;

use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{PixmapSettings, RenderCache, RenderSettings, render};
use slint::{ComponentHandle, Image, Model, ModelRc, Rgba8Pixel, SharedPixelBuffer, VecModel};

use crate::referentiels::retour_erreur;
use crate::traduction::{tr, trf};
use kadenz_domaine::mise_en_page::Marges;

use crate::{Alerte, ApercuApi, EtatPartage, FacturesApi, FenetrePrincipale, PageApercuUi, Retour};

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
    // Marges de la mise en page, pour décider à l'impression si la page tient.
    marges: Marges,
    modele: Rc<VecModel<PageApercuUi>>,
    rendues: Vec<bool>,
    // Largeur de rendu en pixels, d'après l'échelle de l'écran à l'ouverture.
    largeur_rendu: f32,
}

thread_local! {
    static APERCU: RefCell<Option<Apercu>> = const { RefCell::new(None) };
}

/// Ouvre l'aperçu d'un PDF, avec le nom de fichier et le dossier proposés à
/// l'enregistrement et les marges de sa mise en page.
pub(crate) fn ouvrir(
    fenetre: &FenetrePrincipale,
    octets: Vec<u8>,
    nom: String,
    dossier: Option<String>,
    marges: Marges,
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
            marges,
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

/// Aperçu d'un état produit par `crate::pdf` (état du stock, TVA collectée,
/// registre des règlements, relances, feuille de comptage), à la place de son
/// enregistrement direct ; l'aperçu l'enregistre ou l'imprime.
pub(crate) fn montrer(
    fenetre: &FenetrePrincipale,
    etat: &crate::Etat,
    resultat: Result<(Vec<u8>, String), String>,
    dossier: Option<String>,
) -> Retour {
    let (octets, nom) = match resultat {
        Ok(pdf) => pdf,
        Err(message) => return retour_erreur(tr(&message)),
    };
    let marges = kadenz_stockage::mise_en_page::lire(&etat.conn)
        .map(|m| m.marges)
        .unwrap_or_default();
    match ouvrir(fenetre, octets, nom, dossier, marges) {
        Ok(()) => Retour::default(),
        Err(message) => retour_erreur(trf("Aperçu impossible : {}", &[&message])),
    }
}

/// « Imprimer » : fenêtre d'impression de Windows, puis envoi dans un fil de
/// travail ; l'avancement et le résultat reviennent par la boucle des événements.
#[cfg(windows)]
fn imprimer(fenetre: &FenetrePrincipale) -> Retour {
    use crate::imprimante::{self, Choix};

    let api = fenetre.global::<ApercuApi>();
    if api.get_impression_en_cours() {
        return retour_erreur(tr("une impression est déjà en cours"));
    }
    // Copie avant la fenêtre d'impression, qui fait tourner la boucle des événements.
    let copie = APERCU.with(|a| {
        a.borrow().as_ref().map(|apercu| {
            (
                apercu.octets.clone(),
                apercu.nom.trim_end_matches(".pdf").to_string(),
                apercu.marges,
                apercu.rendues.len(),
            )
        })
    });
    let Some((octets, titre, marges, nombre)) = copie else {
        return Retour::default();
    };
    let travail = match imprimante::choisir(poignee(fenetre), nombre) {
        Ok(Choix::Imprimer(travail)) => travail,
        Ok(Choix::Rien) => return Retour::default(),
        Err(message) => return retour_erreur(trf("Impression impossible : {}", &[&message])),
    };
    api.set_impression_en_cours(true);
    api.set_avancement(tr("Impression en préparation").into());
    let faible = fenetre.as_weak();
    std::thread::spawn(move || {
        let suivi = faible.clone();
        let resultat = imprimante::envoyer(octets, &titre, &travail, &marges, None, |n, total| {
            let _ = suivi.upgrade_in_event_loop(move |f| {
                f.global::<ApercuApi>()
                    .set_avancement(trf("Impression : page {} sur {}", &[&n, &total]).into());
            });
        });
        let nom = travail.nom_imprimante();
        let _ = faible.upgrade_in_event_loop(move |f| fin_impression(&f, resultat, &nom));
    });
    Retour::default()
}

#[cfg(not(windows))]
fn imprimer(_fenetre: &FenetrePrincipale) -> Retour {
    Retour::default()
}

#[cfg(windows)]
fn fin_impression(
    fenetre: &FenetrePrincipale,
    resultat: Result<bool, crate::imprimante::Echec>,
    imprimante: &str,
) {
    use crate::imprimante::Echec;

    let api = fenetre.global::<ApercuApi>();
    api.set_impression_en_cours(false);
    api.set_avancement("".into());
    let alerte = fenetre.global::<Alerte>();
    match resultat {
        Ok(false) => alerte.invoke_montrer(trf("Envoyé à l'imprimante {}", &[&imprimante]).into()),
        Ok(true) => alerte.invoke_montrer(
            trf(
                "Envoyé à l'imprimante {}, page réduite pour tenir sur le papier",
                &[&imprimante],
            )
            .into(),
        ),
        Err(echec) => {
            let raison = match echec {
                Echec::Lecture => tr("document illisible"),
                Echec::Imprimante(code) => trf(
                    "imprimante introuvable : {} (code {})",
                    &[&imprimante, &code],
                ),
                Echec::Document(code) => {
                    trf("l'imprimante a refusé le document (code {})", &[&code])
                }
                Echec::Page(n) => trf("l'imprimante a refusé la page {}", &[&n]),
            };
            // Durable : l'aperçu a pu être fermé pendant l'envoi.
            alerte.invoke_signaler(trf("Impression impossible : {}", &[&raison]).into());
        }
    }
}

/// Poignée Windows de la fenêtre de Kadenz, propriétaire de la fenêtre d'impression.
#[cfg(windows)]
fn poignee(fenetre: &FenetrePrincipale) -> windows_sys::Win32::Foundation::HWND {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    match fenetre
        .window()
        .window_handle()
        .window_handle()
        .map(|h| h.as_raw())
    {
        Ok(RawWindowHandle::Win32(h)) => h.hwnd.get() as windows_sys::Win32::Foundation::HWND,
        _ => std::ptr::null_mut(),
    }
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
            Some((octets, nom, dossier)) => {
                crate::paiements::enregistrer_pdf(&octets, &nom, dossier)
            }
            None => Retour::default(),
        }
    });

    let f = fenetre.as_weak();
    api.on_imprimer(move || match f.upgrade() {
        Some(f) => imprimer(&f),
        None => Retour::default(),
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

    #[test]
    fn largeur_de_rendu_selon_l_ecran() {
        assert_eq!(largeur_rendu(1.0), LARGEUR_RENDU_MIN);
        assert_eq!(largeur_rendu(2.0), 1640.0);
        assert_eq!(largeur_rendu(4.0), LARGEUR_RENDU_MAX);
    }

    /// Devis d'essai, en brouillon : (libellé, nombre de pages minimal, PDF, nom).
    fn devis_d_essai() -> Vec<(&'static str, usize, Vec<u8>, String)> {
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
        [("Essai aperçu A1", 1, 1), ("Essai aperçu A1 long", 80, 2)]
            .into_iter()
            .map(|(libelle, lignes, pages_minimum)| {
                let id = documents::creer_brouillon(
                    &mut conn,
                    &admin,
                    TypeDocument::Devis,
                    &saisie(libelle, lignes),
                )
                .unwrap();
                let (octets, nom) = crate::pdf::generer(&conn, id).unwrap();
                (libelle, pages_minimum, octets, nom)
            })
            .collect()
    }

    /// Essai A1 sans écran : un brouillon de devis et un devis de plusieurs pages
    /// sont produits comme pour l'aperçu, puis chaque page est rendue comme à
    /// l'écran. Avec la variable KADENZ_ESSAI_APERCU (un dossier), les pages y
    /// sont écrites en RGBA brut (nom-page-largeurxhauteur.rgba) pour un contrôle
    /// à l'oeil. Le temps de rendu seul est affiché (`--nocapture`) à la largeur
    /// d'un écran à 100 % et à 200 % ; `hayro` est optimisé même en développement.
    #[test]
    fn pages_de_l_apercu() {
        let dossier = std::env::var("KADENZ_ESSAI_APERCU").ok();
        for (libelle, pages_minimum, octets, nom) in devis_d_essai() {
            assert!(nom.starts_with("Apercu-"), "nom d'un brouillon : {nom}");
            let pdf = Pdf::new(octets).unwrap();
            let nombre = pdf.pages().len();
            assert!(nombre >= pages_minimum, "{libelle} : {nombre} page(s)");
            for rang in 0..nombre {
                for echelle in [1.0, 2.0] {
                    let debut = std::time::Instant::now();
                    let (l, h) =
                        rendre_page(&pdf, rang, largeur_rendu(echelle), |_, l, h| (l, h)).unwrap();
                    let duree = debut.elapsed();
                    eprintln!("{nom}, page {}, {l} x {h} px : {duree:?}", rang + 1);
                }
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
                    (l, h, sombres(pixels))
                })
                .unwrap();
                assert_eq!(l, LARGEUR_RENDU_MIN as u32);
                assert!((1740..=1760).contains(&h), "proportions A4 : {h}");
                assert!(sombres > 1000, "{libelle}, page {} blanche", rang + 1);
            }
        }
    }

    /// Essai A4 sans écran : les six états qui passent par l'aperçu (état du
    /// stock, feuille de comptage, TVA collectée, registre des règlements, liste
    /// et lettre de relance) sont produits sur une base où une facture à 18 % est
    /// échue depuis dix jours et réglée en partie, puis chaque page est rendue
    /// comme à l'écran. Avec KADENZ_ESSAI_APERCU (un dossier), les pages y sont
    /// écrites en RGBA brut pour un contrôle à l'oeil.
    #[test]
    fn etats_dans_l_apercu() {
        use chrono::Days;
        use kadenz_domaine::stock::Stock;
        use kadenz_stockage::comptabilite::{self, Periode};
        use kadenz_stockage::reglements::{self, FiltreRegistre, SaisieReglement};
        use kadenz_stockage::stock::LigneEtat;

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
        fiche.adresse = "02 BP 1111, Ville Fictive".into();
        let client = tiers::creer(&conn, &admin, &fiche).unwrap().id;
        let jour = chrono::Local::now().date_naive();
        let saisie = Saisie {
            tiers_id: Some(client),
            taux: Taux {
                remise: 0,
                tva: 1800,
                retenue: 0,
            },
            date_echeance: jour.checked_sub_days(Days::new(10)),
            lignes: vec![LigneSaisie {
                designation: "Pelle".into(),
                prix: 100_000,
                quantite: UNE_UNITE,
                ..Default::default()
            }],
            ..Saisie::vide(jour.checked_sub_days(Days::new(40)).unwrap())
        };
        let facture =
            documents::creer_brouillon(&mut conn, &admin, TypeDocument::Facture, &saisie).unwrap();
        documents::valider(&mut conn, &admin, facture).unwrap();
        let reglement = SaisieReglement {
            date: jour,
            montant: 40_000,
            mode_paiement: "Espèce".into(),
            reference: String::new(),
        };
        reglements::ajouter(&mut conn, &admin, facture, &reglement).unwrap();

        let stock = vec![
            LigneEtat {
                produit_id: 1,
                reference: "PEL-01".into(),
                designation: "Pelle".into(),
                stock: Stock {
                    quantite: 12 * UNE_UNITE,
                    valeur: 600_000,
                    dernier_cout: 50_000,
                },
            },
            LigneEtat {
                produit_id: 2,
                reference: "SEA-02".into(),
                designation: "Seau".into(),
                stock: Stock {
                    quantite: 30 * UNE_UNITE,
                    valeur: 90_000,
                    dernier_cout: 3_000,
                },
            },
        ];
        let feuille: Vec<_> = stock
            .iter()
            .map(|l| crate::stock::LigneFeuille {
                reference: l.reference.clone(),
                designation: l.designation.clone(),
                theorique: l.stock.quantite,
            })
            .collect();
        let periode = Periode {
            du: jour.checked_sub_days(Days::new(60)).unwrap(),
            au: jour,
        };
        let tva = comptabilite::tva_collectee(&conn, periode).unwrap();
        assert!(!tva.is_empty(), "TVA collectée vide");

        let etats = [
            (
                "état du stock",
                crate::pdf::etat_stock(&conn, &stock, None, true),
            ),
            (
                "feuille de comptage",
                crate::pdf::feuille_comptage(&conn, &feuille, false),
            ),
            (
                "TVA collectée",
                crate::pdf::tva_collectee(&conn, &tva, &periode),
            ),
            (
                "registre des règlements",
                crate::pdf::registre_reglements(&conn, &FiltreRegistre::default()),
            ),
            ("liste des relances", crate::pdf::liste_relances(&conn)),
            (
                "lettre de relance",
                crate::pdf::lettre_relance(&conn, facture),
            ),
        ];
        let dossier = std::env::var("KADENZ_ESSAI_APERCU").ok();
        for (libelle, resultat) in etats {
            let (octets, nom) = resultat.unwrap_or_else(|e| panic!("{libelle} : {e}"));
            let pdf = Pdf::new(octets).unwrap_or_else(|_| panic!("{libelle} illisible"));
            let nombre = pdf.pages().len();
            assert!(nombre >= 1, "{libelle} sans page");
            for rang in 0..nombre {
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
                    (l, h, sombres(pixels))
                })
                .unwrap();
                assert_eq!(l, LARGEUR_RENDU_MIN as u32);
                // A4 en portrait (1 754) ou en paysage (877).
                assert!(
                    (1740..=1760).contains(&h) || (870..=885).contains(&h),
                    "{libelle}, proportions : {l} x {h}"
                );
                assert!(sombres > 1000, "{libelle}, page {} blanche", rang + 1);
                eprintln!("{libelle} ({nom}), page {} : {l} x {h}", rang + 1);
            }
        }
    }

    /// Nombre de pages annoncé à la fin d'un flux PostScript (`%%Pages: n`),
    /// une fois le flux complet (`%%EOF`, suivi ou non de la fin de travail
    /// Ctrl-D).
    #[cfg(windows)]
    fn pages_postscript(flux: &[u8]) -> Option<usize> {
        let texte = String::from_utf8_lossy(flux);
        let fin = texte.trim_end_matches(|c: char| c.is_whitespace() || c == '\u{4}');
        if !fin.ends_with("%%EOF") {
            return None;
        }
        texte
            .lines()
            .rev()
            .find_map(|l| l.strip_prefix("%%Pages:"))
            .and_then(|n| n.trim().parse().ok())
    }

    fn sombres(pixels: &[u8]) -> usize {
        pixels.chunks(4).filter(|p| p[0] < 128).count()
    }

    /// Essai A2 sans écran, sur demande seulement : avec la variable
    /// KADENZ_ESSAI_IMPRESSION (un dossier), les deux devis d'essai sont envoyés à
    /// l'imprimante virtuelle « Microsoft Print to PDF », ou à celle que nomme
    /// KADENZ_ESSAI_IMPRIMANTE, désignée par son nom exact (jamais l'imprimante
    /// par défaut), qui les écrit dans ce dossier si son pilote suit le fichier
    /// de sortie demandé ; le devis long part en deux copies assemblées faites
    /// par Kadenz. Un PDF imprimé est relu : nombre de pages, pages non
    /// blanches, première page écrite en RGBA brut pour un contrôle à l'oeil.
    /// Un pilote PostScript (« Wondershare PDFelement ») écrit dans le fichier le
    /// flux brut destiné à l'imprimante : on y vérifie le nombre de pages
    /// annoncé (`%%Pages:`).
    #[cfg(windows)]
    #[test]
    fn impression_sur_print_to_pdf() {
        use crate::imprimante::{Echec, Travail, envoyer};

        let Ok(dossier) = std::env::var("KADENZ_ESSAI_IMPRESSION") else {
            return;
        };
        let dossier = std::path::Path::new(&dossier);
        let nom_imprimante = std::env::var("KADENZ_ESSAI_IMPRIMANTE")
            .unwrap_or_else(|_| "Microsoft Print to PDF".to_string());
        for (libelle, _, octets, nom) in devis_d_essai() {
            let nombre = Pdf::new(octets.clone()).unwrap().pages().len();
            let copies = if nombre > 1 { 2 } else { 1 };
            let pages: Vec<usize> = (0..copies).flat_map(|_| 0..nombre).collect();
            let attendues = pages.len();
            let travail = Travail {
                imprimante: nom_imprimante
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect(),
                devmode: None,
                pages,
            };
            let sortie = dossier.join(nom.replace(".pdf", "-imprime.pdf"));
            let _ = std::fs::remove_file(&sortie);
            let mut feuilles = Vec::new();
            let resultat = envoyer(
                octets,
                libelle,
                &travail,
                &Marges::default(),
                // KADENZ_ESSAI_SANS_SORTIE : le pilote choisit lui-même où écrire.
                std::env::var_os("KADENZ_ESSAI_SANS_SORTIE")
                    .is_none()
                    .then_some(sortie.as_path()),
                |n, total| feuilles.push((n, total)),
            );
            let reduite = match resultat {
                Ok(reduite) => reduite,
                Err(Echec::Imprimante(code)) => {
                    eprintln!("{nom_imprimante} ne s'ouvre pas (code {code}) : essai sauté");
                    return;
                }
                Err(echec) => panic!("{libelle} : {echec:?}"),
            };
            assert_eq!(feuilles.last(), Some(&(attendues, attendues)));
            // Le spouleur écrit le fichier après EndDoc : on attend qu'il soit lisible.
            let mut imprime = None;
            let mut postscript = false;
            for _ in 0..60 {
                std::thread::sleep(std::time::Duration::from_millis(500));
                let Ok(lu) = std::fs::read(&sortie) else {
                    continue;
                };
                if lu.starts_with(b"%!PS") {
                    if pages_postscript(&lu) == Some(attendues) {
                        postscript = true;
                        break;
                    }
                    continue;
                }
                if let Ok(pdf) = Pdf::new(lu)
                    && pdf.pages().len() == attendues
                {
                    imprime = Some(pdf);
                    break;
                }
            }
            if postscript {
                eprintln!(
                    "{} : PostScript de {attendues} page(s), réduite : {reduite}",
                    sortie.display()
                );
                continue;
            }
            // Un pilote tiers peut ignorer le fichier demandé et ouvrir sa propre
            // boîte d'enregistrement : l'envoi a réussi, le fichier est ailleurs.
            let Some(imprime) = imprime else {
                assert_ne!(
                    nom_imprimante,
                    "Microsoft Print to PDF",
                    "{libelle} : {} absent",
                    sortie.display()
                );
                eprintln!(
                    "{libelle} : envoyé à {nom_imprimante} ({attendues} feuille(s), réduite : {reduite}), fichier non écrit dans {}",
                    sortie.display()
                );
                continue;
            };
            eprintln!(
                "{} : {attendues} page(s), réduite : {reduite}",
                sortie.display()
            );
            for rang in 0..attendues {
                let noirs = rendre_page(&imprime, rang, LARGEUR_RENDU_MIN, |pixels, l, h| {
                    if rang == 0 {
                        let fichier = sortie.with_file_name(format!(
                            "{}-p1-{l}x{h}.rgba",
                            nom.trim_end_matches(".pdf")
                        ));
                        std::fs::write(fichier, pixels).unwrap();
                    }
                    sombres(pixels)
                })
                .unwrap();
                assert!(noirs > 1000, "{libelle}, feuille {} blanche", rang + 1);
            }
        }
    }
}
