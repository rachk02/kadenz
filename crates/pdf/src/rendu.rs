//! Mise en page et tracé d'un document commercial au format A4 portrait.
//!
//! Chaque bloc est écrit une seule fois et sert deux fois : à blanc pour mesurer
//! sa hauteur (avant la pagination), puis pour de bon sur la page. Mesure et
//! tracé ne peuvent donc pas diverger.

use kadenz_domaine::calcul::{Montant, Pourcentage};
use kadenz_domaine::mise_en_page::{
    Gabarit as GabaritImpression, LigneResolue, LigneTotal, Marges, PassageResolu, StylePied,
    Variable,
};
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

use crate::modele::{DocumentPdf, EntreprisePdf, Filigrane, LignePdf};
use crate::pagination::{Gabarit, paginer};
use crate::texte::{Police, Polices};

// Format A4 en points (1/72 de pouce).
pub(crate) const LARGEUR: f32 = 595.28;
pub(crate) const HAUTEUR: f32 = 841.89;

/// Zone imprimable de la page, d'après les marges réglées (en points).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Cadre {
    pub(crate) gauche: f32,
    pub(crate) droite: f32,
    pub(crate) haut: f32,
    pub(crate) bas: f32,
}

impl Cadre {
    pub(crate) fn depuis(m: &Marges) -> Cadre {
        let pt = |mm: u8| f32::from(mm) * 72.0 / 25.4;
        Cadre {
            gauche: pt(m.gauche),
            droite: LARGEUR - pt(m.droite),
            haut: pt(m.haut),
            bas: HAUTEUR - pt(m.bas),
        }
    }

    pub(crate) fn utile(&self) -> f32 {
        self.droite - self.gauche
    }
}

/// Inclinaison de l'italique, obtenue en penchant la police droite (aucune
/// police italique n'est embarquée).
const PENTE_ITALIQUE: f32 = 0.2;

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

    /// Texte, penché autour de sa ligne de base s'il est en italique.
    #[allow(clippy::too_many_arguments)]
    fn ecrire_style(
        &mut self,
        police: &Police,
        taille: f32,
        c: [u8; 3],
        x: f32,
        base: f32,
        texte: &str,
        italique: bool,
    ) {
        if !italique {
            return self.ecrire(police, taille, c, x, base, texte);
        }
        if let Some(s) = self.s.as_deref_mut() {
            s.push_transform(&Transform::from_row(
                1.0,
                0.0,
                -PENTE_ITALIQUE,
                1.0,
                PENTE_ITALIQUE * base,
                0.0,
            ));
        }
        self.ecrire(police, taille, c, x, base, texte);
        if let Some(s) = self.s.as_deref_mut() {
            s.pop();
        }
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

    pub(crate) fn trait_vertical(&mut self, x: f32, y1: f32, y2: f32, c: [u8; 3], epaisseur: f32) {
        let Some(s) = self.s.as_deref_mut() else {
            return;
        };
        let mut pb = PathBuilder::new();
        pb.move_to(x, y1);
        pb.line_to(x, y2);
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

    /// Image ajustée dans une boîte, proportions gardées ; `alignement` vaut 0 à
    /// gauche, 0,5 au centre, 1 à droite. Renvoie la hauteur tracée.
    fn image(&mut self, image: &Image, x: f32, y: f32, l: f32, h: f32, alignement: f32) -> f32 {
        let (lp, hp) = image.size();
        if lp == 0 || hp == 0 {
            return 0.0;
        }
        let echelle = (l / lp as f32).min(h / hp as f32);
        let (lr, hr) = (lp as f32 * echelle, hp as f32 * echelle);
        if let (Some(s), Some(taille)) = (self.s.as_deref_mut(), Size::from_wh(lr, hr)) {
            let x = x + (l - lr) * alignement;
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

/// Style d'une zone de contenu libre : la première ligne de texte de l'en-tête
/// prend la taille du nom de l'entreprise.
#[derive(Clone, Copy)]
struct StyleZone {
    taille: f32,
    interligne: f32,
    couleur: [u8; 3],
    premiere: Option<(f32, f32, [u8; 3])>,
    h_logo: f32,
    /// Tout le texte en gras (pied de page accentué).
    gras: bool,
}

/// Valeur d'une variable d'impression, lue dans les données de l'entreprise.
fn valeur(e: &EntreprisePdf, v: Variable) -> String {
    match v {
        Variable::Nom => e.nom.clone(),
        Variable::Adresse => e.adresse.clone(),
        Variable::Telephone => e.telephone.clone(),
        Variable::Email => e.email.clone(),
        Variable::Site => e.site_web.clone(),
        Variable::Localisation => e.localisation.clone(),
        Variable::Regime => e.regime.clone(),
        Variable::Rccm => e.rccm.clone(),
        Variable::Ifu => e.ifu.clone(),
        Variable::Banque1 => e.banque1.clone(),
        Variable::Banque2 => e.banque2.clone(),
        Variable::Logo | Variable::Logo2 => String::new(),
    }
}

pub(crate) struct Moteur<'d> {
    doc: &'d DocumentPdf,
    p: &'d Polices,
    pub(crate) g: Cadre,
    gabarit: GabaritImpression,
    style_pied: StylePied,
    entete: Vec<LigneResolue>,
    pied: Vec<LigneResolue>,
    logo: Option<Image>,
    logo2: Option<Image>,
    colonnes: Vec<Colonne>,
}

impl<'d> Moteur<'d> {
    pub(crate) fn new(doc: &'d DocumentPdf, p: &'d Polices) -> Self {
        let mep = &doc.mise_en_page;
        let g = Cadre::depuis(&mep.marges);
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
                    g.utile() - fixe
                } else {
                    c.largeur
                },
                ..c
            })
            .collect();
        let zone = |z: &kadenz_domaine::mise_en_page::Zone| {
            if mep.gabarit.imprime_les_zones() {
                z.resoudre(|v| valeur(&doc.entreprise, v))
            } else {
                Vec::new()
            }
        };
        Moteur {
            doc,
            p,
            g,
            gabarit: mep.gabarit,
            style_pied: mep.style_pied,
            entete: zone(&mep.entete),
            pied: zone(&mep.pied),
            logo: lire_image(&doc.entreprise.logo),
            logo2: lire_image(&doc.entreprise.logo2),
            colonnes,
        }
    }

    fn x_colonne(&self, rang: usize) -> f32 {
        self.g.gauche + self.colonnes[..rang].iter().map(|c| c.largeur).sum::<f32>()
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

    fn image_de(&self, v: Variable) -> Option<&Image> {
        match v {
            Variable::Logo => self.logo.as_ref(),
            Variable::Logo2 => self.logo2.as_ref(),
            _ => None,
        }
    }

    fn police(&self, gras: bool) -> &Police {
        if gras {
            &self.p.texte_gras
        } else {
            &self.p.texte
        }
    }

    /// Coupe une ligne à styles mêlés en rangées tenant dans `largeur` ; chaque
    /// morceau est un mot suivi de ses espaces, avec sa chasse.
    fn couper_riche(
        &self,
        passages: &[PassageResolu],
        taille: f32,
        largeur: f32,
        tout_gras: bool,
    ) -> Vec<Vec<(String, bool, bool, f32)>> {
        let mut morceaux: Vec<(String, bool, bool)> = Vec::new();
        for p in passages {
            let mut courant = String::new();
            let mut apres_espace = false;
            for c in p.texte.chars() {
                if !c.is_whitespace() && apres_espace && !courant.trim().is_empty() {
                    morceaux.push((std::mem::take(&mut courant), p.gras, p.italique));
                }
                apres_espace = c.is_whitespace();
                courant.push(c);
            }
            if !courant.is_empty() {
                morceaux.push((courant, p.gras, p.italique));
            }
        }
        let mut rangees: Vec<Vec<(String, bool, bool, f32)>> = vec![Vec::new()];
        let mut occupe = 0.0;
        for (texte, gras, italique) in morceaux {
            let police = self.police(gras || tout_gras);
            let utile = police.largeur(texte.trim_end(), taille);
            let rangee = rangees.last_mut().expect("au moins une rangée");
            if !rangee.is_empty() && occupe + utile > largeur {
                rangees.push(Vec::new());
                occupe = 0.0;
            }
            let texte = if occupe == 0.0 {
                texte.trim_start().to_owned()
            } else {
                texte
            };
            let l = police.largeur(&texte, taille);
            occupe += l;
            rangees
                .last_mut()
                .expect("au moins une rangée")
                .push((texte, gras, italique, l));
        }
        rangees
    }

    /// Zone de contenu libre (en-tête ou pied de page), mesurée à blanc ou
    /// tracée ; renvoie sa hauteur.
    fn zone(
        &self,
        t: &mut Toile,
        lignes: &[LigneResolue],
        style: StyleZone,
        x: f32,
        y: f32,
        largeur: f32,
    ) -> f32 {
        let mut h = 0.0;
        let mut premiere = true;
        for ligne in lignes {
            match ligne {
                LigneResolue::Logo { logo, centree } => {
                    if let Some(image) = self.image_de(*logo) {
                        let alignement = if *centree { 0.5 } else { 0.0 };
                        h += t.image(image, x, y + h, largeur, style.h_logo, alignement) + 4.0;
                    }
                }
                LigneResolue::Texte { centree, passages } => {
                    let (taille, interligne, c) = match (premiere, style.premiere) {
                        (true, Some(p)) => p,
                        _ => (style.taille, style.interligne, style.couleur),
                    };
                    premiere = false;
                    if passages.is_empty() {
                        h += interligne;
                        continue;
                    }
                    for rangee in self.couper_riche(passages, taille, largeur, style.gras) {
                        let mut l_rangee: f32 = rangee.iter().map(|m| m.3).sum();
                        if let Some((texte, gras, _, l)) = rangee.last() {
                            let police = self.police(*gras || style.gras);
                            l_rangee += police.largeur(texte.trim_end(), taille) - l;
                        }
                        let mut xx = if *centree {
                            x + (largeur - l_rangee) / 2.0
                        } else {
                            x
                        };
                        for (texte, gras, italique, l) in &rangee {
                            let police = self.police(*gras || style.gras);
                            let base = base_centree(police, taille, y + h, interligne);
                            t.ecrire_style(police, taille, c, xx, base, texte, *italique);
                            xx += l;
                        }
                        h += interligne;
                    }
                }
            }
        }
        h
    }

    fn compact(&self) -> bool {
        self.gabarit == GabaritImpression::Compact
    }

    fn style_entete(&self) -> StyleZone {
        if self.compact() {
            StyleZone {
                taille: 7.5,
                interligne: 9.5,
                couleur: SECONDAIRE,
                premiere: Some((10.0, 12.5, ENCRE)),
                h_logo: 42.0,
                gras: false,
            }
        } else {
            StyleZone {
                taille: 8.0,
                interligne: 10.5,
                couleur: SECONDAIRE,
                premiere: Some((11.5, 14.0, ENCRE)),
                h_logo: 54.0,
                gras: false,
            }
        }
    }

    fn accentue(&self) -> bool {
        self.style_pied == StylePied::Accentue
    }

    fn style_pied(&self) -> StyleZone {
        StyleZone {
            taille: 7.0,
            interligne: 9.0,
            couleur: if self.accentue() { ENCRE } else { SECONDAIRE },
            premiere: None,
            h_logo: 30.0,
            gras: self.accentue(),
        }
    }

    /// En-tête répété sur chaque page : zone d'en-tête à gauche, type, numéro et
    /// date à droite. Un logo non centré en première ligne de la zone se place à
    /// gauche du texte ; un logo non centré en dernière ligne se place à droite,
    /// au-dessus du titre.
    pub(crate) fn en_tete(&self, t: &mut Toile, y: f32) -> f32 {
        let p = self.p;
        let largeur_droite = 190.0;
        let x_droite = self.g.droite - largeur_droite;

        let mut lignes: &[LigneResolue] = &self.entete;
        let mut logo_gauche = None;
        let mut logo_droite = None;
        if let Some((
            LigneResolue::Logo {
                logo,
                centree: false,
            },
            reste,
        )) = lignes.split_first()
        {
            logo_gauche = self.image_de(*logo);
            lignes = reste;
        }
        if let Some((
            LigneResolue::Logo {
                logo,
                centree: false,
            },
            reste,
        )) = lignes.split_last()
        {
            logo_droite = self.image_de(*logo);
            lignes = reste;
        }
        let style = self.style_entete();

        // Colonne gauche : logo en tête de zone, puis le reste de la zone.
        let mut x_texte = self.g.gauche;
        let mut h_logo = 0.0;
        if let Some(logo) = logo_gauche {
            h_logo = t.image(logo, self.g.gauche, y, 110.0, style.h_logo, 0.0);
            x_texte = self.g.gauche + 122.0;
        }
        let largeur_texte = x_droite - 12.0 - x_texte;
        let h_gauche = self
            .zone(t, lignes, style, x_texte, y, largeur_texte)
            .max(h_logo);

        // Colonne droite : logo secondaire, titre, numéro, date.
        let mut yd = y;
        if let Some(logo) = logo_droite {
            let h = if self.compact() { 28.0 } else { 36.0 };
            yd += t.image(logo, x_droite, yd, largeur_droite, h, 1.0) + 6.0;
        }
        let titre = &self.doc.textes.titre;
        if !titre.is_empty() {
            let taille_titre = 17.0;
            let l = p.titre.largeur(titre, taille_titre);
            t.ecrire(
                &p.titre,
                taille_titre,
                self.doc.couleur,
                self.g.droite - l,
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
                self.g.droite - l,
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
                self.g.droite - l,
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
                self.g.droite - l,
                base_centree(&p.texte, 9.0, yd, 13.0),
                reference,
            );
            yd += 13.0;
        }

        let (avant, apres) = if self.compact() {
            (7.0, 8.0)
        } else {
            (10.0, 12.0)
        };
        let bas = y + h_gauche.max(yd - y) + avant;
        t.trait_horizontal(self.g.gauche, self.g.droite, bas, BORDURE, 0.75);
        bas + apres - y
    }

    // ------------------------------------------------------------ bloc tiers (page 1)

    fn bloc_tiers(&self, t: &mut Toile, y: f32) -> f32 {
        let d = self.doc;
        let p = self.p;
        let demi = self.g.utile() / 2.0;

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
                self.g.gauche,
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
                self.g.gauche,
                yg,
                demi - 16.0,
                valeur,
            );
            yg += 5.0;
        }

        // Droite : cadre du tiers.
        let x = self.g.gauche + demi + 8.0;
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
        t.rectangle(
            self.g.gauche,
            y,
            self.g.utile(),
            H_ENTETE_TABLEAU,
            self.doc.couleur,
        );
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
            self.g.gauche + MARGE_CELLULE,
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
        t.trait_horizontal(self.g.gauche, self.g.droite, y + h, BORDURE, 0.5);
        h
    }

    fn ligne_report(&self, t: &mut Toile, y: f32, libelle: &str, valeur: Montant) -> f32 {
        let p = self.p;
        t.rectangle(self.g.gauche, y, self.g.utile(), H_REPORT, FOND_CLAIR);
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

    /// Lignes de totaux à imprimer, avec libellé et valeur, selon le bas réglé.
    fn lignes_totaux(&self) -> Vec<(LigneTotal, String, Montant)> {
        let d = self.doc;
        let b = &d.bas;
        let t = &d.totaux;
        b.totaux
            .iter()
            .filter_map(|&l| {
                let (valeur, libelle) = match l {
                    LigneTotal::Ht => (t.ht, l.libelle().to_owned()),
                    LigneTotal::Remise => (
                        t.remise,
                        format!("{} ({})", l.libelle(), taux(d.taux.remise)),
                    ),
                    LigneTotal::BaseTva => (t.base_tva, l.libelle().to_owned()),
                    LigneTotal::Tva => (t.tva, format!("{} ({})", l.libelle(), taux(d.taux.tva))),
                    LigneTotal::Retenue => (
                        t.retenue,
                        format!("{} ({})", l.libelle(), taux(d.taux.retenue)),
                    ),
                    LigneTotal::Ttc => (t.ttc, l.libelle().to_owned()),
                };
                let masquee = b.masquer_zero
                    && match l {
                        LigneTotal::Ttc => false,
                        LigneTotal::BaseTva => t.remise == 0,
                        _ => valeur == 0,
                    };
                (!masquee).then_some((l, libelle, valeur))
            })
            .collect()
    }

    /// Case de visa ou de date : titre centré, contenu dessous.
    #[allow(clippy::too_many_arguments)]
    fn case_bas(
        &self,
        t: &mut Toile,
        x: f32,
        y: f32,
        largeur: f32,
        h: f32,
        titre: &str,
        texte: &str,
    ) {
        let p = self.p;
        t.cadre(x, y, largeur, h, BORDURE);
        let l = p.texte_gras.largeur(titre, 8.5);
        t.ecrire(
            &p.texte_gras,
            8.5,
            ENCRE,
            x + (largeur - l) / 2.0,
            base_centree(&p.texte_gras, 8.5, y + 3.0, 12.0),
            titre,
        );
        if !texte.trim().is_empty() {
            self.paragraphe(
                t,
                &p.texte,
                8.5,
                11.0,
                ENCRE,
                x + 6.0,
                y + 16.0,
                largeur - 12.0,
                texte,
            );
        }
    }

    // ------------------------------------------------------------ bloc final

    /// Bas du document, selon les éléments réglés pour son type : totaux,
    /// observations, montant en lettres, visas et dates, signataire.
    fn bloc_final(&self, t: &mut Toile, y: f32) -> f32 {
        let d = self.doc;
        let b = &d.bas;
        let p = self.p;
        let mut bas = y;

        // Totaux, à droite ; la ligne du TTC est mise en valeur.
        let lignes = self.lignes_totaux();
        let avec_totaux = !lignes.is_empty();
        if avec_totaux {
            let x = self.g.droite - LARGEUR_TOTAUX;
            let mut yt = y;
            for (l, libelle, valeur) in &lignes {
                let v = format!("{} FCFA", montant(*valeur));
                if *l == LigneTotal::Ttc {
                    let h = 20.0;
                    t.rectangle(x, yt, LARGEUR_TOTAUX, h, d.couleur);
                    let base = base_centree(&p.texte_gras, 10.0, yt, h);
                    t.ecrire(&p.texte_gras, 10.0, BLANC, x + 6.0, base, libelle);
                    let lv = p.texte_gras.largeur(&v, 10.0);
                    let xv = self.g.droite - 6.0 - lv;
                    t.ecrire(&p.texte_gras, 10.0, BLANC, xv, base, &v);
                    yt += h;
                } else {
                    let h = 17.0;
                    let base = base_centree(&p.texte, 9.0, yt, h);
                    t.ecrire(&p.texte, 9.0, ENCRE, x + 6.0, base, libelle);
                    let lv = p.texte.largeur(&v, 9.0);
                    t.ecrire(&p.texte, 9.0, ENCRE, self.g.droite - 6.0 - lv, base, &v);
                    yt += h;
                    t.trait_horizontal(x, self.g.droite, yt, BORDURE, 0.5);
                }
            }
            bas = bas.max(yt);
        }

        // Observations, à gauche des totaux.
        if !d.observations.trim().is_empty() {
            let largeur = if avec_totaux {
                self.g.utile() - LARGEUR_TOTAUX - 16.0
            } else {
                self.g.utile()
            };
            let mut yo = y;
            yo += self.paragraphe(
                t,
                &p.texte_gras,
                7.5,
                10.0,
                SECONDAIRE,
                self.g.gauche,
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
                self.g.gauche,
                yo,
                largeur,
                &d.observations,
            );
            bas = bas.max(yo);
        }

        // Montant en lettres, dans la phrase de l'outil Excel d'origine :
        // « Arrêté le présent ... à la somme de <lettres> francs (<chiffres>) FCFA. »
        if b.montant_en_lettres && !d.montant_en_lettres.is_empty() {
            bas += 12.0;
            let phrase = phrase_somme(&d.textes.phrase_somme, &d.montant_en_lettres, d.totaux.ttc);
            bas += self.paragraphe(
                t,
                &p.texte_gras,
                9.5,
                12.5,
                ENCRE,
                self.g.gauche,
                bas,
                self.g.utile(),
                &phrase,
            );
        }

        // Visas et dates, en cases sur deux colonnes (modèle Excel du
        // bordereau) : le client à gauche, le fournisseur à droite.
        let ecart = 40.0;
        let largeur_col = (self.g.utile() - ecart) / 2.0;
        let x_droite = self.g.gauche + largeur_col + ecart;
        let avec_cases =
            b.visa_client || b.visa_fournisseur || b.date_reception || b.date_livraison;
        if avec_cases {
            bas += 16.0;
            let l = &d.livraison;
            let (h_visa, h_date, espace) = (56.0, 32.0, 8.0);
            let mut h_cols: f32 = 0.0;
            for (x, visa, texte_visa, date, texte_date, titre_visa, titre_date) in [
                (
                    self.g.gauche,
                    b.visa_client,
                    &l.visa_client,
                    b.date_reception,
                    &l.date_reception,
                    "Visa du client",
                    "Date de réception",
                ),
                (
                    x_droite,
                    b.visa_fournisseur,
                    &l.visa_fournisseur,
                    b.date_livraison,
                    &l.date_livraison,
                    "Visa du fournisseur",
                    "Date de livraison",
                ),
            ] {
                let mut yc = bas;
                if visa {
                    self.case_bas(t, x, yc, largeur_col, h_visa, titre_visa, texte_visa);
                    yc += h_visa + espace;
                }
                if date {
                    self.case_bas(t, x, yc, largeur_col, h_date, titre_date, texte_date);
                    yc += h_date + espace;
                }
                h_cols = h_cols.max(yc - bas);
            }
            bas += h_cols - espace;
        }

        // Signataire : sous la colonne de droite s'il y a des cases, sinon à droite.
        if b.signataire && !d.textes.signataire.is_empty() {
            bas += if avec_cases { 10.0 } else { 16.0 };
            let (x, largeur) = if avec_cases {
                (x_droite, largeur_col)
            } else {
                (self.g.droite - 200.0, 200.0)
            };
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

    fn largeur_pied(&self) -> f32 {
        self.g.utile() - 50.0
    }

    pub(crate) fn hauteur_pied(&self) -> f32 {
        let h = self.zone(
            &mut Toile::a_blanc(),
            &self.pied,
            self.style_pied(),
            self.g.gauche,
            0.0,
            self.largeur_pied(),
        );
        8.0 + h.max(9.0)
    }

    pub(crate) fn pied(&self, t: &mut Toile, page: usize, total: usize) {
        let p = self.p;
        let haut = self.g.bas - self.hauteur_pied();
        if self.gabarit.imprime_les_zones() {
            if self.accentue() {
                t.trait_horizontal(self.g.gauche, self.g.droite, haut, self.doc.couleur, 2.0);
            } else {
                t.trait_horizontal(self.g.gauche, self.g.droite, haut, BORDURE, 0.75);
            }
        }
        self.zone(
            t,
            &self.pied,
            self.style_pied(),
            self.g.gauche,
            haut + 6.0,
            self.largeur_pied(),
        );
        let numero = format!("Page {page}/{total}");
        let l = p.texte_moyen.largeur(&numero, 7.5);
        t.ecrire(
            &p.texte_moyen,
            7.5,
            SECONDAIRE,
            self.g.droite - l,
            base_centree(&p.texte_moyen, 7.5, self.g.bas - 9.0, 9.0),
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
    let h_entete = m.en_tete(&mut blanc, m.g.haut);
    let h_tiers = m.bloc_tiers(&mut blanc, 0.0);
    let h_final = ESPACE_AVANT_FINAL + m.bloc_final(&mut blanc, 0.0);
    let hauteurs: Vec<f32> = doc
        .lignes
        .iter()
        .map(|l| m.ligne(&mut blanc, 0.0, l))
        .collect();

    let haut_corps = m.g.haut + h_entete;
    let bas_corps = m.g.bas - m.hauteur_pied() - 10.0;
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

        let mut y = m.g.haut + m.en_tete(&mut t, m.g.haut);
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
