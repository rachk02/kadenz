//! Mise en page et tracé d'un document commercial au format A4 portrait.
//!
//! Chaque bloc est écrit une seule fois et sert deux fois : à blanc pour mesurer
//! sa hauteur (avant la pagination), puis pour de bon sur la page. Mesure et
//! tracé ne peuvent donc pas diverger.

use kadenz_domaine::calcul::{Montant, Pourcentage};
use kadenz_domaine::saisie::{montant_en_texte, pourcentage_en_texte, quantite_en_texte};
use krilla::Document;
use krilla::color::rgb;
use krilla::geom::{PathBuilder, Point, Rect, Size, Transform};
use krilla::image::Image;
use krilla::metadata::Metadata;
use krilla::num::NormalizedF32;
use krilla::page::PageSettings;
use krilla::paint::{Fill, Stroke};
use krilla::surface::Surface;

use crate::modele::{DocumentPdf, Filigrane, LignePdf};
use crate::pagination::{Gabarit, paginer};
use crate::texte::{Police, Polices};

// Format A4 en points (1/72 de pouce).
pub(crate) const LARGEUR: f32 = 595.28;
pub(crate) const HAUTEUR: f32 = 841.89;
pub(crate) const BANDE: f32 = 12.0;
pub(crate) const GAUCHE: f32 = 42.0;
pub(crate) const DROITE: f32 = LARGEUR - 36.0;
pub(crate) const HAUT: f32 = 34.0;
pub(crate) const BAS: f32 = HAUTEUR - 26.0;
pub(crate) const UTILE: f32 = DROITE - GAUCHE;

const H_ENTETE_TABLEAU: f32 = 20.0;
const H_REPORT: f32 = 18.0;
pub(crate) const MARGE_CELLULE: f32 = 4.0;
const ESPACE_AVANT_FINAL: f32 = 14.0;
const LARGEUR_TOTAUX: f32 = 230.0;

// Couleurs de la charte (Theme de l'application).
pub(crate) const ENCRE: [u8; 3] = [0x1f, 0x29, 0x33];
pub(crate) const SECONDAIRE: [u8; 3] = [0x52, 0x60, 0x6d];
pub(crate) const BORDURE: [u8; 3] = [0xd9, 0xdc, 0xe3];
pub(crate) const FOND_CLAIR: [u8; 3] = [0xf4, 0xf5, 0xf9];
pub(crate) const BLANC: [u8; 3] = [0xff, 0xff, 0xff];
const ROUGE: [u8; 3] = [0xb4, 0x23, 0x18];

fn couleur(c: [u8; 3]) -> rgb::Color {
    rgb::Color::new(c[0], c[1], c[2])
}

fn remplissage(c: [u8; 3], opacite: f32) -> Fill {
    Fill {
        paint: couleur(c).into(),
        opacity: NormalizedF32::new(opacite).unwrap_or(NormalizedF32::ONE),
        rule: Default::default(),
    }
}

pub(crate) fn montant(m: Montant) -> String {
    montant_en_texte(m)
}

fn taux(p: Pourcentage) -> String {
    format!("{}\u{00A0}%", pourcentage_en_texte(p))
}

/// Phrase du montant en lettres, sur le modèle de l'outil Excel d'origine :
/// « {phrase} {lettres} francs ({chiffres}) FCFA. » ; sans phrase, les lettres
/// seules suivies des chiffres.
fn phrase_somme(phrase: &str, lettres: &str, ttc: Montant) -> String {
    let fin = format!("{lettres} francs ({}) FCFA.", montant(ttc));
    if phrase.trim().is_empty() {
        fin
    } else {
        format!("{} {fin}", phrase.trim())
    }
}

/// Surface de tracé, ou rien quand on ne fait que mesurer.
pub(crate) struct Toile<'a, 'b> {
    pub(crate) s: Option<&'a mut Surface<'b>>,
}

impl Toile<'_, '_> {
    pub(crate) fn a_blanc() -> Toile<'static, 'static> {
        Toile { s: None }
    }

    pub(crate) fn ecrire(
        &mut self,
        police: &Police,
        taille: f32,
        c: [u8; 3],
        x: f32,
        base: f32,
        texte: &str,
    ) {
        let Some(s) = self.s.as_deref_mut() else {
            return;
        };
        if texte.is_empty() {
            return;
        }
        s.set_stroke(None);
        s.set_fill(Some(remplissage(c, 1.0)));
        let (glyphes, _) = police.faconner(texte);
        s.draw_glyphs(
            Point::from_xy(x, base),
            &glyphes,
            police.font.clone(),
            texte,
            taille,
            false,
        );
    }

    pub(crate) fn rectangle(&mut self, x: f32, y: f32, l: f32, h: f32, c: [u8; 3]) {
        let Some(s) = self.s.as_deref_mut() else {
            return;
        };
        let Some(rect) = Rect::from_xywh(x, y, l, h) else {
            return;
        };
        let mut pb = PathBuilder::new();
        pb.push_rect(rect);
        if let Some(chemin) = pb.finish() {
            s.set_stroke(None);
            s.set_fill(Some(remplissage(c, 1.0)));
            s.draw_path(&chemin);
        }
    }

    pub(crate) fn cadre(&mut self, x: f32, y: f32, l: f32, h: f32, c: [u8; 3]) {
        let Some(s) = self.s.as_deref_mut() else {
            return;
        };
        let Some(rect) = Rect::from_xywh(x, y, l, h) else {
            return;
        };
        let mut pb = PathBuilder::new();
        pb.push_rect(rect);
        if let Some(chemin) = pb.finish() {
            s.set_fill(None);
            s.set_stroke(Some(Stroke {
                paint: couleur(c).into(),
                width: 0.75,
                ..Default::default()
            }));
            s.draw_path(&chemin);
            s.set_stroke(None);
        }
    }

    pub(crate) fn trait_horizontal(
        &mut self,
        x1: f32,
        x2: f32,
        y: f32,
        c: [u8; 3],
        epaisseur: f32,
    ) {
        let Some(s) = self.s.as_deref_mut() else {
            return;
        };
        let mut pb = PathBuilder::new();
        pb.move_to(x1, y);
        pb.line_to(x2, y);
        if let Some(chemin) = pb.finish() {
            s.set_fill(None);
            s.set_stroke(Some(Stroke {
                paint: couleur(c).into(),
                width: epaisseur,
                ..Default::default()
            }));
            s.draw_path(&chemin);
            s.set_stroke(None);
        }
    }

    /// Image ajustée dans une boîte, proportions gardées, calée à gauche ou à droite.
    fn image(&mut self, image: &Image, x: f32, y: f32, l: f32, h: f32, a_droite: bool) -> f32 {
        let (lp, hp) = image.size();
        if lp == 0 || hp == 0 {
            return 0.0;
        }
        let echelle = (l / lp as f32).min(h / hp as f32);
        let (lr, hr) = (lp as f32 * echelle, hp as f32 * echelle);
        if let (Some(s), Some(taille)) = (self.s.as_deref_mut(), Size::from_wh(lr, hr)) {
            let x = if a_droite { x + l - lr } else { x };
            s.push_transform(&Transform::from_translate(x, y));
            s.draw_image(image.clone(), taille);
            s.pop();
        }
        hr
    }
}

/// Ligne de base d'un texte centré verticalement dans une bande `[haut, haut + h]`.
pub(crate) fn base_centree(police: &Police, taille: f32, haut: f32, h: f32) -> f32 {
    haut + h / 2.0 + police.hauteur_capitales * taille / 2.0
}

fn lire_image(octets: &Option<Vec<u8>>) -> Option<Image> {
    let o = octets.as_ref()?;
    if o.starts_with(&[0x89, b'P', b'N', b'G']) {
        Image::from_png(o.clone().into(), true).ok()
    } else if o.starts_with(&[0xFF, 0xD8]) {
        Image::from_jpeg(o.clone().into(), true).ok()
    } else {
        None
    }
}

struct Colonne {
    titre: &'static str,
    largeur: f32,
    a_droite: bool,
}

pub(crate) struct Moteur<'d> {
    doc: &'d DocumentPdf,
    p: &'d Polices,
    logo: Option<Image>,
    logo2: Option<Image>,
    colonnes: Vec<Colonne>,
}

impl<'d> Moteur<'d> {
    pub(crate) fn new(doc: &'d DocumentPdf, p: &'d Polices) -> Self {
        let colonnes = if doc.avec_montants {
            vec![
                Colonne {
                    titre: "Référence",
                    largeur: 64.0,
                    a_droite: false,
                },
                Colonne {
                    titre: "Désignation",
                    largeur: 0.0,
                    a_droite: false,
                },
                Colonne {
                    titre: "Prix unitaire",
                    largeur: 76.0,
                    a_droite: true,
                },
                Colonne {
                    titre: "Quantité",
                    largeur: 54.0,
                    a_droite: true,
                },
                Colonne {
                    titre: "Total",
                    largeur: 84.0,
                    a_droite: true,
                },
            ]
        } else {
            vec![
                Colonne {
                    titre: "Référence",
                    largeur: 80.0,
                    a_droite: false,
                },
                Colonne {
                    titre: "Désignation",
                    largeur: 0.0,
                    a_droite: false,
                },
                Colonne {
                    titre: "Qté commandée",
                    largeur: 76.0,
                    a_droite: true,
                },
                Colonne {
                    titre: "Qté livrée",
                    largeur: 70.0,
                    a_droite: true,
                },
            ]
        };
        let fixe: f32 = colonnes.iter().map(|c| c.largeur).sum();
        let colonnes = colonnes
            .into_iter()
            .map(|c| Colonne {
                largeur: if c.largeur == 0.0 {
                    UTILE - fixe
                } else {
                    c.largeur
                },
                ..c
            })
            .collect();
        Moteur {
            doc,
            p,
            logo: lire_image(&doc.entreprise.logo),
            logo2: lire_image(&doc.entreprise.logo2),
            colonnes,
        }
    }

    fn x_colonne(&self, rang: usize) -> f32 {
        GAUCHE + self.colonnes[..rang].iter().map(|c| c.largeur).sum::<f32>()
    }

    /// Texte sur plusieurs lignes ; renvoie la hauteur occupée.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn paragraphe(
        &self,
        t: &mut Toile,
        police: &Police,
        taille: f32,
        interligne: f32,
        c: [u8; 3],
        x: f32,
        y: f32,
        largeur: f32,
        texte: &str,
    ) -> f32 {
        let lignes = police.couper(texte, taille, largeur);
        for (k, ligne) in lignes.iter().enumerate() {
            let haut = y + k as f32 * interligne;
            t.ecrire(
                police,
                taille,
                c,
                x,
                base_centree(police, taille, haut, interligne),
                ligne,
            );
        }
        lignes.len() as f32 * interligne
    }

    // ------------------------------------------------------------ en-tête

    /// En-tête répété sur chaque page : logos, entreprise, type, numéro, date.
    pub(crate) fn en_tete(&self, t: &mut Toile, y: f32) -> f32 {
        let e = &self.doc.entreprise;
        let p = self.p;
        let largeur_droite = 190.0;
        let x_droite = DROITE - largeur_droite;

        // Colonne gauche : logo principal, puis coordonnées.
        let mut x_texte = GAUCHE;
        let mut h_logo = 0.0;
        if let Some(logo) = &self.logo {
            h_logo = t.image(logo, GAUCHE, y, 110.0, 54.0, false);
            x_texte = GAUCHE + 122.0;
        }
        let largeur_texte = x_droite - 12.0 - x_texte;
        let mut yg = y;
        yg += self.paragraphe(
            t,
            &p.texte_gras,
            11.5,
            14.0,
            ENCRE,
            x_texte,
            yg,
            largeur_texte,
            &e.nom,
        );
        let mut coordonnees = vec![e.adresse.clone()];
        if !e.telephone.is_empty() {
            coordonnees.push(format!("Tél. : {}", e.telephone));
        }
        coordonnees.push(e.email.clone());
        coordonnees.push(e.site_web.clone());
        for ligne in coordonnees.iter().filter(|l| !l.trim().is_empty()) {
            yg += self.paragraphe(
                t,
                &p.texte,
                8.0,
                10.5,
                SECONDAIRE,
                x_texte,
                yg,
                largeur_texte,
                ligne,
            );
        }
        let h_gauche = (yg - y).max(h_logo);

        // Colonne droite : logo secondaire, titre, numéro, date.
        let mut yd = y;
        if let Some(logo2) = &self.logo2 {
            yd += t.image(logo2, x_droite, yd, largeur_droite, 36.0, true) + 6.0;
        }
        let titre = &self.doc.textes.titre;
        if !titre.is_empty() {
            let taille_titre = 17.0;
            let l = p.titre.largeur(titre, taille_titre);
            t.ecrire(
                &p.titre,
                taille_titre,
                self.doc.couleur,
                DROITE - l,
                base_centree(&p.titre, taille_titre, yd, 22.0),
                titre,
            );
            yd += 22.0;
        }
        if !self.doc.numero.is_empty() {
            let texte = format!("N° {}", self.doc.numero);
            let l = p.texte_gras.largeur(&texte, 10.0);
            t.ecrire(
                &p.texte_gras,
                10.0,
                ENCRE,
                DROITE - l,
                base_centree(&p.texte_gras, 10.0, yd, 14.0),
                &texte,
            );
            yd += 14.0;
        }
        if !self.doc.date.is_empty() {
            let date = format!("Date : {}", self.doc.date);
            let l = p.texte.largeur(&date, 9.0);
            t.ecrire(
                &p.texte,
                9.0,
                ENCRE,
                DROITE - l,
                base_centree(&p.texte, 9.0, yd, 13.0),
                &date,
            );
            yd += 13.0;
        }
        if !self.doc.reference.is_empty() {
            let reference = &self.doc.reference;
            let l = p.texte.largeur(reference, 9.0);
            t.ecrire(
                &p.texte,
                9.0,
                SECONDAIRE,
                DROITE - l,
                base_centree(&p.texte, 9.0, yd, 13.0),
                reference,
            );
            yd += 13.0;
        }

        let bas = y + h_gauche.max(yd - y) + 10.0;
        t.trait_horizontal(GAUCHE, DROITE, bas, BORDURE, 0.75);
        bas + 12.0 - y
    }

    // ------------------------------------------------------------ bloc tiers (page 1)

    fn bloc_tiers(&self, t: &mut Toile, y: f32) -> f32 {
        let d = self.doc;
        let p = self.p;
        let demi = UTILE / 2.0;

        // Gauche : libellé, échéance, mode de paiement.
        let mut yg = y;
        let mut infos = Vec::new();
        if !d.libelle.trim().is_empty() {
            infos.push(("Libellé", d.libelle.as_str()));
        }
        if !d.echeance.is_empty() {
            let libelle = if d.libelle_echeance.is_empty() {
                "Échéance"
            } else {
                d.libelle_echeance.as_str()
            };
            infos.push((libelle, d.echeance.as_str()));
        }
        if !d.mode_paiement.is_empty() {
            infos.push(("Mode de paiement", d.mode_paiement.as_str()));
        }
        for (libelle, valeur) in infos {
            yg += self.paragraphe(
                t,
                &p.texte_gras,
                7.5,
                10.0,
                SECONDAIRE,
                GAUCHE,
                yg,
                demi - 16.0,
                libelle,
            );
            yg += self.paragraphe(
                t,
                &p.texte,
                9.0,
                12.0,
                ENCRE,
                GAUCHE,
                yg,
                demi - 16.0,
                valeur,
            );
            yg += 5.0;
        }

        // Droite : cadre du tiers.
        let x = GAUCHE + demi + 8.0;
        let largeur = demi - 8.0;
        let marge = 8.0;
        let interieur = largeur - 2.0 * marge;
        let mut yd = y + marge;
        let tiers = &d.tiers;
        yd += self.paragraphe(
            t,
            &p.texte_gras,
            7.5,
            10.0,
            SECONDAIRE,
            x + marge,
            yd,
            interieur,
            &d.textes.libelle_tiers,
        );
        yd += self.paragraphe(
            t,
            &p.texte_gras,
            10.5,
            13.5,
            ENCRE,
            x + marge,
            yd,
            interieur,
            &tiers.nom,
        );
        let mut lignes = vec![tiers.adresse.clone()];
        for (libelle, valeur) in [
            ("IFU", &tiers.ifu),
            ("RCCM", &tiers.rccm),
            ("Régime", &tiers.regime),
        ] {
            if !valeur.trim().is_empty() {
                lignes.push(format!("{libelle} : {valeur}"));
            }
        }
        for ligne in lignes.iter().filter(|l| !l.trim().is_empty()) {
            yd += self.paragraphe(
                t,
                &p.texte,
                8.5,
                11.0,
                ENCRE,
                x + marge,
                yd,
                interieur,
                ligne,
            );
        }
        yd += marge;
        t.cadre(x, y, largeur, yd - y, BORDURE);

        (yg - y).max(yd - y) + 14.0
    }

    // ------------------------------------------------------------ tableau

    fn entete_tableau(&self, t: &mut Toile, y: f32) -> f32 {
        let p = self.p;
        t.rectangle(GAUCHE, y, UTILE, H_ENTETE_TABLEAU, self.doc.couleur);
        for (rang, c) in self.colonnes.iter().enumerate() {
            let x = self.x_colonne(rang);
            let base = base_centree(&p.texte_gras, 8.0, y, H_ENTETE_TABLEAU);
            let l = p.texte_gras.largeur(c.titre, 8.0);
            let xt = if c.a_droite {
                x + c.largeur - MARGE_CELLULE - l
            } else {
                x + MARGE_CELLULE
            };
            t.ecrire(&p.texte_gras, 8.0, BLANC, xt, base, c.titre);
        }
        H_ENTETE_TABLEAU
    }

    /// Une ligne du tableau ; renvoie sa hauteur.
    fn ligne(&self, t: &mut Toile, y: f32, l: &LignePdf) -> f32 {
        let p = self.p;
        let (taille, interligne) = (8.5, 11.0);
        let haut = y + MARGE_CELLULE;
        let interieur = |rang: usize| self.colonnes[rang].largeur - 2.0 * MARGE_CELLULE;

        let h_ref = self.paragraphe(
            t,
            &p.texte,
            taille,
            interligne,
            ENCRE,
            GAUCHE + MARGE_CELLULE,
            haut,
            interieur(0),
            &l.reference,
        );
        let x_des = self.x_colonne(1) + MARGE_CELLULE;
        let mut h_des = self.paragraphe(
            t,
            &p.texte,
            taille,
            interligne,
            ENCRE,
            x_des,
            haut,
            interieur(1),
            &l.designation,
        );
        if !l.observations.trim().is_empty() {
            h_des += self.paragraphe(
                t,
                &p.texte,
                7.5,
                9.5,
                SECONDAIRE,
                x_des,
                haut + h_des,
                interieur(1),
                &l.observations,
            );
        }
        let valeurs: Vec<String> = if self.doc.avec_montants {
            vec![
                montant(l.prix),
                quantite_en_texte(l.quantite),
                montant(l.total),
            ]
        } else {
            vec![
                quantite_en_texte(l.quantite),
                quantite_en_texte(l.quantite_livree),
            ]
        };
        for (k, v) in valeurs.iter().enumerate() {
            let rang = k + 2;
            let x_fin = self.x_colonne(rang) + self.colonnes[rang].largeur - MARGE_CELLULE;
            let lv = p.texte.largeur(v, taille);
            t.ecrire(
                &p.texte,
                taille,
                ENCRE,
                x_fin - lv,
                base_centree(&p.texte, taille, haut, interligne),
                v,
            );
        }
        let h = h_ref.max(h_des).max(interligne) + 2.0 * MARGE_CELLULE;
        t.trait_horizontal(GAUCHE, DROITE, y + h, BORDURE, 0.5);
        h
    }

    fn ligne_report(&self, t: &mut Toile, y: f32, libelle: &str, valeur: Montant) -> f32 {
        let p = self.p;
        t.rectangle(GAUCHE, y, UTILE, H_REPORT, FOND_CLAIR);
        let base = base_centree(&p.texte_gras, 8.5, y, H_REPORT);
        t.ecrire(
            &p.texte_gras,
            8.5,
            ENCRE,
            self.x_colonne(1) + MARGE_CELLULE,
            base,
            libelle,
        );
        let dernier = self.colonnes.len() - 1;
        let texte = montant(valeur);
        let l = p.texte_gras.largeur(&texte, 8.5);
        let x_fin = self.x_colonne(dernier) + self.colonnes[dernier].largeur - MARGE_CELLULE;
        t.ecrire(&p.texte_gras, 8.5, ENCRE, x_fin - l, base, &texte);
        H_REPORT
    }

    // ------------------------------------------------------------ bloc final (dernière page)

    fn bloc_final(&self, t: &mut Toile, y: f32) -> f32 {
        let d = self.doc;
        let p = self.p;
        let mut bas = y;

        if d.avec_montants {
            // Totaux, à droite.
            let x = DROITE - LARGEUR_TOTAUX;
            let h = 17.0;
            let mut rangs: Vec<(String, Montant)> = vec![("Montant HT".into(), d.totaux.ht)];
            if d.totaux.remise != 0 {
                rangs.push((format!("Remise ({})", taux(d.taux.remise)), d.totaux.remise));
                rangs.push(("Base TVA".into(), d.totaux.base_tva));
            }
            rangs.push((format!("TVA ({})", taux(d.taux.tva)), d.totaux.tva));
            if d.totaux.retenue != 0 {
                rangs.push((
                    format!("Retenue ({})", taux(d.taux.retenue)),
                    d.totaux.retenue,
                ));
            }
            let mut yt = y;
            for (libelle, valeur) in &rangs {
                let base = base_centree(&p.texte, 9.0, yt, h);
                t.ecrire(&p.texte, 9.0, ENCRE, x + 6.0, base, libelle);
                let v = format!("{} FCFA", montant(*valeur));
                let l = p.texte.largeur(&v, 9.0);
                t.ecrire(&p.texte, 9.0, ENCRE, DROITE - 6.0 - l, base, &v);
                yt += h;
                t.trait_horizontal(x, DROITE, yt, BORDURE, 0.5);
            }
            let h_ttc = 20.0;
            t.rectangle(x, yt, LARGEUR_TOTAUX, h_ttc, d.couleur);
            let base = base_centree(&p.texte_gras, 10.0, yt, h_ttc);
            t.ecrire(&p.texte_gras, 10.0, BLANC, x + 6.0, base, "Montant TTC");
            let v = format!("{} FCFA", montant(d.totaux.ttc));
            let l = p.texte_gras.largeur(&v, 10.0);
            t.ecrire(&p.texte_gras, 10.0, BLANC, DROITE - 6.0 - l, base, &v);
            yt += h_ttc;
            bas = bas.max(yt);
        }

        // Observations, à gauche des totaux.
        if !d.observations.trim().is_empty() {
            let largeur = if d.avec_montants {
                UTILE - LARGEUR_TOTAUX - 16.0
            } else {
                UTILE
            };
            let mut yo = y;
            yo += self.paragraphe(
                t,
                &p.texte_gras,
                7.5,
                10.0,
                SECONDAIRE,
                GAUCHE,
                yo,
                largeur,
                "Observations",
            );
            yo += self.paragraphe(
                t,
                &p.texte,
                8.5,
                11.0,
                ENCRE,
                GAUCHE,
                yo,
                largeur,
                &d.observations,
            );
            bas = bas.max(yo);
        }

        // Montant en lettres, dans la phrase de l'outil Excel d'origine :
        // « Arrêté le présent ... à la somme de <lettres> francs (<chiffres>) FCFA. »
        if d.avec_montants && !d.montant_en_lettres.is_empty() {
            bas += 12.0;
            let phrase = phrase_somme(&d.textes.phrase_somme, &d.montant_en_lettres, d.totaux.ttc);
            bas += self.paragraphe(
                t,
                &p.texte_gras,
                9.5,
                12.5,
                ENCRE,
                GAUCHE,
                bas,
                UTILE,
                &phrase,
            );
        }

        // Bordereau : zones de visa du client et du fournisseur, avec les dates
        // de réception et de livraison (section 6.5), à la place du signataire.
        if !d.avec_montants {
            bas += 16.0;
            let ecart = 16.0;
            let largeur = (UTILE - ecart) / 2.0;
            let hauteur = 96.0;
            let l = &d.livraison;
            let zones = [
                (
                    GAUCHE,
                    "Visa du client",
                    "Date de réception",
                    &l.date_reception,
                    &l.visa_client,
                ),
                (
                    GAUCHE + largeur + ecart,
                    "Visa du fournisseur",
                    "Date de livraison",
                    &l.date_livraison,
                    &l.visa_fournisseur,
                ),
            ];
            for (x, titre, libelle_date, date, visa) in zones {
                t.cadre(x, bas, largeur, hauteur, BORDURE);
                let interieur = largeur - 12.0;
                let mut yz = bas + 6.0;
                yz += self.paragraphe(
                    t,
                    &p.texte_gras,
                    8.5,
                    11.0,
                    ENCRE,
                    x + 6.0,
                    yz,
                    interieur,
                    titre,
                );
                yz += self.paragraphe(
                    t,
                    &p.texte,
                    8.0,
                    11.0,
                    SECONDAIRE,
                    x + 6.0,
                    yz,
                    interieur,
                    &format!("{libelle_date} : {date}"),
                );
                if !visa.is_empty() {
                    self.paragraphe(t, &p.texte, 8.5, 11.0, ENCRE, x + 6.0, yz, interieur, visa);
                }
            }
            bas += hauteur;
            return bas - y;
        }

        // Signataire, à droite.
        if !d.textes.signataire.is_empty() {
            bas += 16.0;
            let largeur = 200.0;
            let x = DROITE - largeur;
            let centrer = |police: &Police, taille: f32, texte: &str| {
                x + (largeur - police.largeur(texte, taille)) / 2.0
            };
            let s = &d.textes.signataire;
            t.ecrire(
                &p.texte_gras,
                9.0,
                ENCRE,
                centrer(&p.texte_gras, 9.0, s),
                base_centree(&p.texte_gras, 9.0, bas, 12.0),
                s,
            );
            bas += 12.0 + 44.0;
            let nom = &d.textes.nom_signataire;
            if !nom.is_empty() {
                t.ecrire(
                    &p.texte,
                    9.0,
                    ENCRE,
                    centrer(&p.texte, 9.0, nom),
                    base_centree(&p.texte, 9.0, bas, 12.0),
                    nom,
                );
                bas += 12.0;
            }
        }
        bas - y
    }

    // ------------------------------------------------------------ pied de page

    fn lignes_pied(&self) -> Vec<String> {
        let e = &self.doc.entreprise;
        let joindre = |morceaux: Vec<String>| {
            morceaux
                .into_iter()
                .filter(|m| !m.trim().is_empty())
                .collect::<Vec<_>>()
                .join("  ·  ")
        };
        let avec = |libelle: &str, v: &str| {
            if v.trim().is_empty() {
                String::new()
            } else {
                format!("{libelle} : {v}")
            }
        };
        let largeur = UTILE - 50.0;
        [
            joindre(vec![
                e.adresse.clone(),
                avec("Tél.", &e.telephone),
                e.email.clone(),
                e.site_web.clone(),
            ]),
            joindre(vec![
                avec("RCCM", &e.rccm),
                avec("IFU", &e.ifu),
                avec("Régime", &e.regime),
            ]),
            joindre(vec![e.banque1.clone(), e.banque2.clone()]),
        ]
        .into_iter()
        .filter(|l| !l.is_empty())
        .flat_map(|l| self.p.texte.couper(&l, 7.0, largeur))
        .collect()
    }

    pub(crate) fn hauteur_pied(&self) -> f32 {
        8.0 + self.lignes_pied().len().max(1) as f32 * 9.0
    }

    pub(crate) fn pied(&self, t: &mut Toile, page: usize, total: usize) {
        let p = self.p;
        let haut = BAS - self.hauteur_pied();
        t.trait_horizontal(GAUCHE, DROITE, haut, BORDURE, 0.75);
        let mut y = haut + 6.0;
        let centre = GAUCHE + (UTILE - 50.0) / 2.0;
        for ligne in self.lignes_pied() {
            let l = p.texte.largeur(&ligne, 7.0);
            t.ecrire(
                &p.texte,
                7.0,
                SECONDAIRE,
                centre - l / 2.0,
                base_centree(&p.texte, 7.0, y, 9.0),
                &ligne,
            );
            y += 9.0;
        }
        let numero = format!("Page {page}/{total}");
        let l = p.texte_moyen.largeur(&numero, 7.5);
        t.ecrire(
            &p.texte_moyen,
            7.5,
            SECONDAIRE,
            DROITE - l,
            base_centree(&p.texte_moyen, 7.5, BAS - 9.0, 9.0),
            &numero,
        );
    }

    fn filigrane(&self, t: &mut Toile) {
        let (texte, c) = match self.doc.filigrane {
            Filigrane::Aucun => return,
            Filigrane::Brouillon => ("BROUILLON", SECONDAIRE),
            Filigrane::Annule => ("ANNULÉ", ROUGE),
        };
        let Some(s) = t.s.as_deref_mut() else {
            return;
        };
        let police = &self.p.titre;
        let taille = 96.0;
        let (cx, cy) = (LARGEUR / 2.0, HAUTEUR / 2.0);
        let l = police.largeur(texte, taille);
        s.push_transform(&Transform::from_rotate_at(-40.0, cx, cy));
        s.set_stroke(None);
        s.set_fill(Some(remplissage(c, 0.12)));
        let (glyphes, _) = police.faconner(texte);
        s.draw_glyphs(
            Point::from_xy(cx - l / 2.0, cy + police.hauteur_capitales * taille / 2.0),
            &glyphes,
            police.font.clone(),
            texte,
            taille,
            false,
        );
        s.pop();
    }
}

/// Produit le PDF d'un document.
pub fn generer(doc: &DocumentPdf, polices: &Polices) -> Result<Vec<u8>, String> {
    let m = Moteur::new(doc, polices);

    // Mesures à blanc.
    let mut blanc = Toile::a_blanc();
    let h_entete = m.en_tete(&mut blanc, HAUT);
    let h_tiers = m.bloc_tiers(&mut blanc, 0.0);
    let h_final = ESPACE_AVANT_FINAL + m.bloc_final(&mut blanc, 0.0);
    let hauteurs: Vec<f32> = doc
        .lignes
        .iter()
        .map(|l| m.ligne(&mut blanc, 0.0, l))
        .collect();

    let haut_corps = HAUT + h_entete;
    let bas_corps = BAS - m.hauteur_pied() - 10.0;
    let gabarit = Gabarit {
        premiere: bas_corps - haut_corps - h_tiers - H_ENTETE_TABLEAU,
        suivantes: bas_corps - haut_corps - H_ENTETE_TABLEAU,
        ligne_report: H_REPORT,
        bloc_final: h_final,
        avec_report: doc.avec_montants,
    };
    let pages = paginer(&hauteurs, &gabarit);

    let mut document = Document::new();
    let mut titre = doc.textes.titre.clone();
    if !doc.numero.is_empty() {
        titre = format!("{titre} {}", doc.numero);
    }
    document.set_metadata(Metadata::new().title(titre).language("fr".into()));

    let nombre = pages.len();
    let mut cumul: Montant = 0;
    for (k, page_tableau) in pages.iter().enumerate() {
        let reglages = PageSettings::from_wh(LARGEUR, HAUTEUR).ok_or("format de page invalide")?;
        let mut page = document.start_page_with(reglages);
        let mut surface = page.surface();
        let mut t = Toile {
            s: Some(&mut surface),
        };

        t.rectangle(0.0, 0.0, BANDE, HAUTEUR, doc.couleur);
        let mut y = HAUT + m.en_tete(&mut t, HAUT);
        if k == 0 {
            y += m.bloc_tiers(&mut t, y);
        }
        y += m.entete_tableau(&mut t, y);
        if page_tableau.report_en_haut {
            y += m.ligne_report(&mut t, y, "Report", cumul);
        }
        for ligne in &doc.lignes[page_tableau.lignes.clone()] {
            y += m.ligne(&mut t, y, ligne);
            cumul += ligne.total;
        }
        if page_tableau.a_reporter_en_bas {
            y += m.ligne_report(&mut t, y, "À reporter", cumul);
        }
        if page_tableau.bloc_final {
            m.bloc_final(&mut t, y + ESPACE_AVANT_FINAL);
        }
        m.pied(&mut t, k + 1, nombre);
        m.filigrane(&mut t);

        surface.finish();
        page.finish();
    }
    document.finish().map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modele::{LignePdf, TextesPdf};

    fn document(n: usize) -> DocumentPdf {
        DocumentPdf {
            textes: TextesPdf {
                titre: "FACTURE".into(),
                libelle_tiers: "Client".into(),
                signataire: "SIGNATAIRE".into(),
                ..Default::default()
            },
            numero: "F2026-0001".into(),
            date: "09/11/2026".into(),
            lignes: (0..n)
                .map(|k| LignePdf {
                    reference: format!("R{k}"),
                    designation: "Article fictif".into(),
                    prix: 1_000,
                    quantite: 1_000,
                    total: 1_000,
                    ..Default::default()
                })
                .collect(),
            avec_montants: true,
            montant_en_lettres: "Mille".into(),
            couleur: [0x2d, 0x3a, 0x8c],
            ..Default::default()
        }
    }

    #[test]
    fn pdf_produit_avec_ou_sans_lignes() {
        let p = Polices::embarquees();
        for n in [0, 3, 80] {
            for filigrane in [Filigrane::Aucun, Filigrane::Brouillon, Filigrane::Annule] {
                let doc = DocumentPdf {
                    filigrane,
                    ..document(n)
                };
                let octets = generer(&doc, &p).unwrap();
                assert!(octets.starts_with(b"%PDF-"), "{n} lignes");
            }
        }
        let sans_montants = DocumentPdf {
            avec_montants: false,
            ..document(30)
        };
        assert!(generer(&sans_montants, &p).unwrap().starts_with(b"%PDF-"));
        let avoir = DocumentPdf {
            reference: "Facture d'origine : F2026-0001".into(),
            ..document(60)
        };
        assert!(generer(&avoir, &p).unwrap().starts_with(b"%PDF-"));
    }

    #[test]
    fn phrase_du_montant() {
        assert_eq!(
            phrase_somme(
                "Arrêté la présente facture à la somme de",
                "Mille deux cents",
                1_200
            ),
            "Arrêté la présente facture à la somme de Mille deux cents francs (1\u{a0}200) FCFA."
        );
        assert_eq!(phrase_somme(" ", "Cent", 100), "Cent francs (100) FCFA.");
    }

    #[test]
    fn logo_illisible_ignore() {
        let mut doc = document(2);
        doc.entreprise.logo = Some(b"pas une image".to_vec());
        assert!(generer(&doc, &Polices::embarquees()).is_ok());
    }
}
