//! Lettre de relance et listes imprimées (registre des règlements, relances),
//! avec l'en-tête, la bande de couleur et le pied de page des documents.

use krilla::Document;
use krilla::metadata::Metadata;
use krilla::page::PageSettings;

use crate::modele::{ColonnePdf, DocumentPdf, LettrePdf, ListePdf, TotalPdf};
use crate::rendu::{
    BANDE, BAS, BLANC, BORDURE, DROITE, ENCRE, FOND_CLAIR, GAUCHE, HAUT, HAUTEUR, LARGEUR,
    MARGE_CELLULE, Moteur, SECONDAIRE, Toile, UTILE, base_centree,
};
use crate::texte::{Police, Polices};

const H_ENTETE_TABLEAU: f32 = 20.0;
const H_TOTAL: f32 = 17.0;
const LARGEUR_TOTAUX: f32 = 260.0;
const TAILLE_CELLULE: f32 = 8.5;
const INTERLIGNE_CELLULE: f32 = 11.0;

/// Document réduit à ce que l'en-tête et le pied de page impriment.
fn cadre(
    entreprise: &crate::modele::EntreprisePdf,
    titre: &str,
    date: &str,
    couleur: [u8; 3],
) -> DocumentPdf {
    let mut d = DocumentPdf {
        entreprise: entreprise.clone(),
        date: date.to_owned(),
        couleur,
        ..Default::default()
    };
    d.textes.titre = titre.to_owned();
    d
}

fn ecrire_a_droite(
    t: &mut Toile,
    police: &Police,
    taille: f32,
    c: [u8; 3],
    y: f32,
    h: f32,
    texte: &str,
) {
    let l = police.largeur(texte, taille);
    t.ecrire(
        police,
        taille,
        c,
        DROITE - l,
        base_centree(police, taille, y, h),
        texte,
    );
}

/// Bloc du signataire, à droite, comme sur les documents ; renvoie la hauteur.
fn signataire(t: &mut Toile, p: &Polices, y: f32, libelle: &str, nom: &str) -> f32 {
    if libelle.is_empty() {
        return 0.0;
    }
    let largeur = 200.0;
    let x = DROITE - largeur;
    let centrer = |police: &Police, taille: f32, texte: &str| {
        x + (largeur - police.largeur(texte, taille)) / 2.0
    };
    t.ecrire(
        &p.texte_gras,
        9.0,
        ENCRE,
        centrer(&p.texte_gras, 9.0, libelle),
        base_centree(&p.texte_gras, 9.0, y, 12.0),
        libelle,
    );
    let mut h = 12.0 + 44.0;
    if !nom.is_empty() {
        t.ecrire(
            &p.texte,
            9.0,
            ENCRE,
            centrer(&p.texte, 9.0, nom),
            base_centree(&p.texte, 9.0, y + h, 12.0),
            nom,
        );
        h += 12.0;
    }
    h
}

/// Corps de la lettre sous l'en-tête ; renvoie la hauteur occupée.
fn corps_lettre(m: &Moteur, p: &Polices, l: &LettrePdf, t: &mut Toile, y0: f32) -> f32 {
    let mut y = y0;
    if !l.lieu_et_date.is_empty() {
        ecrire_a_droite(t, &p.texte, 9.5, ENCRE, y, 14.0, &l.lieu_et_date);
        y += 14.0;
    }
    y += 12.0;

    // Destinataire, dans la moitié droite.
    let x = GAUCHE + UTILE / 2.0 + 8.0;
    let largeur = UTILE / 2.0 - 8.0;
    y += m.paragraphe(
        t,
        &p.texte_gras,
        10.5,
        13.5,
        ENCRE,
        x,
        y,
        largeur,
        &l.destinataire.nom,
    );
    for ligne in [&l.destinataire.adresse]
        .into_iter()
        .filter(|v| !v.trim().is_empty())
    {
        y += m.paragraphe(t, &p.texte, 9.0, 12.0, ENCRE, x, y, largeur, ligne);
    }
    y += 22.0;

    if !l.titre.is_empty() {
        y += m.paragraphe(
            t, &p.titre, 14.0, 18.0, l.couleur, GAUCHE, y, UTILE, &l.titre,
        );
        y += 2.0;
    }
    if !l.objet.is_empty() {
        y += m.paragraphe(
            t,
            &p.texte_gras,
            10.0,
            13.5,
            ENCRE,
            GAUCHE,
            y,
            UTILE,
            &l.objet,
        );
    }
    y += 14.0;
    if !l.avant.is_empty() {
        y += m.paragraphe(t, &p.texte, 9.5, 13.5, ENCRE, GAUCHE, y, UTILE, &l.avant);
        y += 12.0;
    }

    // Tableau récapitulatif, colonnes de même largeur, valeurs centrées.
    if !l.recapitulatif.is_empty() {
        let n = l.recapitulatif.len() as f32;
        let largeur = UTILE / n;
        t.rectangle(GAUCHE, y, UTILE, H_ENTETE_TABLEAU, l.couleur);
        let h_valeur = 20.0;
        for (k, (titre, valeur)) in l.recapitulatif.iter().enumerate() {
            let x = GAUCHE + k as f32 * largeur;
            let centrer = |police: &Police, taille: f32, texte: &str| {
                x + (largeur - police.largeur(texte, taille)) / 2.0
            };
            t.ecrire(
                &p.texte_gras,
                8.0,
                BLANC,
                centrer(&p.texte_gras, 8.0, titre),
                base_centree(&p.texte_gras, 8.0, y, H_ENTETE_TABLEAU),
                titre,
            );
            t.ecrire(
                &p.texte,
                9.0,
                ENCRE,
                centrer(&p.texte, 9.0, valeur),
                base_centree(&p.texte, 9.0, y + H_ENTETE_TABLEAU, h_valeur),
                valeur,
            );
        }
        y += H_ENTETE_TABLEAU + h_valeur;
        t.trait_horizontal(GAUCHE, DROITE, y, BORDURE, 0.75);
        y += 14.0;
    }

    if !l.apres.is_empty() {
        y += m.paragraphe(t, &p.texte, 9.5, 13.5, ENCRE, GAUCHE, y, UTILE, &l.apres);
    }
    y += 22.0;
    y += signataire(t, p, y, &l.signataire, &l.nom_signataire);
    y - y0
}

/// Produit le PDF d'une lettre de relance, sur une page, ou deux si les textes
/// réglés ne tiennent pas sur la première (le signataire suit alors le texte).
pub fn generer_lettre(l: &LettrePdf, polices: &Polices) -> Result<Vec<u8>, String> {
    let doc = cadre(&l.entreprise, "", "", l.couleur);
    let m = Moteur::new(&doc, polices);
    let mut blanc = Toile::a_blanc();
    let h_entete = m.en_tete(&mut blanc, HAUT);
    let h_corps = corps_lettre(&m, polices, l, &mut blanc, 0.0);
    let disponible = BAS - m.hauteur_pied() - 10.0 - (HAUT + h_entete);
    if h_corps > disponible {
        return Err("le texte de la lettre de relance ne tient pas sur une page : le raccourcir dans les paramètres".into());
    }

    let mut document = Document::new();
    document.set_metadata(Metadata::new().title(l.titre.clone()).language("fr".into()));
    let reglages = PageSettings::from_wh(LARGEUR, HAUTEUR).ok_or("format de page invalide")?;
    let mut page = document.start_page_with(reglages);
    let mut surface = page.surface();
    let mut t = Toile {
        s: Some(&mut surface),
    };
    t.rectangle(0.0, 0.0, BANDE, HAUTEUR, l.couleur);
    let y = HAUT + m.en_tete(&mut t, HAUT);
    corps_lettre(&m, polices, l, &mut t, y);
    m.pied(&mut t, 1, 1);
    surface.finish();
    page.finish();
    document.finish().map_err(|e| format!("{e:?}"))
}

/// Largeurs effectives : les colonnes à 0 se partagent la place restante.
fn largeurs(colonnes: &[ColonnePdf]) -> Vec<f32> {
    let fixe: f32 = colonnes.iter().map(|c| c.largeur).sum();
    let libres = colonnes.iter().filter(|c| c.largeur == 0.0).count().max(1) as f32;
    colonnes
        .iter()
        .map(|c| {
            if c.largeur == 0.0 {
                ((UTILE - fixe) / libres).max(20.0)
            } else {
                c.largeur
            }
        })
        .collect()
}

struct Tableau<'a> {
    liste: &'a ListePdf,
    largeurs: Vec<f32>,
}

impl Tableau<'_> {
    fn x(&self, rang: usize) -> f32 {
        GAUCHE + self.largeurs[..rang].iter().sum::<f32>()
    }

    fn entete(&self, p: &Polices, t: &mut Toile, y: f32) -> f32 {
        t.rectangle(GAUCHE, y, UTILE, H_ENTETE_TABLEAU, self.liste.couleur);
        for (rang, c) in self.liste.colonnes.iter().enumerate() {
            let l = p.texte_gras.largeur(&c.titre, 8.0);
            let x = if c.a_droite {
                self.x(rang) + self.largeurs[rang] - MARGE_CELLULE - l
            } else {
                self.x(rang) + MARGE_CELLULE
            };
            t.ecrire(
                &p.texte_gras,
                8.0,
                BLANC,
                x,
                base_centree(&p.texte_gras, 8.0, y, H_ENTETE_TABLEAU),
                &c.titre,
            );
        }
        H_ENTETE_TABLEAU
    }

    /// Une ligne, valeurs coupées dans leur colonne ; renvoie sa hauteur.
    fn ligne(&self, p: &Polices, t: &mut Toile, y: f32, valeurs: &[String]) -> f32 {
        let mut h_max: f32 = INTERLIGNE_CELLULE;
        for (rang, c) in self.liste.colonnes.iter().enumerate() {
            let v = valeurs.get(rang).map(String::as_str).unwrap_or("");
            let interieur = self.largeurs[rang] - 2.0 * MARGE_CELLULE;
            let lignes = p.texte.couper(v, TAILLE_CELLULE, interieur);
            for (k, morceau) in lignes.iter().enumerate() {
                let haut = y + MARGE_CELLULE + k as f32 * INTERLIGNE_CELLULE;
                let x = if c.a_droite {
                    self.x(rang) + self.largeurs[rang]
                        - MARGE_CELLULE
                        - p.texte.largeur(morceau, TAILLE_CELLULE)
                } else {
                    self.x(rang) + MARGE_CELLULE
                };
                t.ecrire(
                    &p.texte,
                    TAILLE_CELLULE,
                    ENCRE,
                    x,
                    base_centree(&p.texte, TAILLE_CELLULE, haut, INTERLIGNE_CELLULE),
                    morceau,
                );
            }
            h_max = h_max.max(lignes.len() as f32 * INTERLIGNE_CELLULE);
        }
        let h = h_max + 2.0 * MARGE_CELLULE;
        t.trait_horizontal(GAUCHE, DROITE, y + h, BORDURE, 0.5);
        h
    }

    fn criteres(&self, m: &Moteur, p: &Polices, t: &mut Toile, y: f32) -> f32 {
        let mut h = 0.0;
        for c in &self.liste.criteres {
            h += m.paragraphe(t, &p.texte, 9.0, 12.5, SECONDAIRE, GAUCHE, y + h, UTILE, c);
        }
        if h > 0.0 { h + 10.0 } else { 0.0 }
    }

    fn totaux(&self, p: &Polices, t: &mut Toile, y: f32) -> f32 {
        if self.liste.totaux.is_empty() {
            return 0.0;
        }
        let x = DROITE - LARGEUR_TOTAUX;
        let mut yt = y + 10.0;
        for TotalPdf {
            libelle,
            valeur,
            principal,
        } in &self.liste.totaux
        {
            let police = if *principal { &p.texte_gras } else { &p.texte };
            if *principal {
                t.rectangle(x, yt, LARGEUR_TOTAUX, H_TOTAL, FOND_CLAIR);
            }
            let base = base_centree(police, 9.0, yt, H_TOTAL);
            t.ecrire(police, 9.0, ENCRE, x + 6.0, base, libelle);
            let l = police.largeur(valeur, 9.0);
            t.ecrire(police, 9.0, ENCRE, DROITE - 6.0 - l, base, valeur);
            yt += H_TOTAL;
            t.trait_horizontal(x, DROITE, yt, BORDURE, 0.5);
        }
        yt - y
    }
}

/// Produit le PDF d'une liste : tableau paginé, en-tête répété sur chaque page,
/// critères en page 1, totaux à la fin (jamais séparés entre deux pages).
pub fn generer_liste(liste: &ListePdf, polices: &Polices) -> Result<Vec<u8>, String> {
    if liste.colonnes.is_empty() {
        return Err("liste sans colonne".into());
    }
    let doc = cadre(&liste.entreprise, &liste.titre, &liste.date, liste.couleur);
    let m = Moteur::new(&doc, polices);
    let tableau = Tableau {
        liste,
        largeurs: largeurs(&liste.colonnes),
    };

    // Mesures à blanc, puis découpage en pages.
    let mut blanc = Toile::a_blanc();
    let haut_corps = HAUT + m.en_tete(&mut blanc, HAUT);
    let bas_corps = BAS - m.hauteur_pied() - 10.0;
    let h_criteres = tableau.criteres(&m, polices, &mut blanc, 0.0);
    let h_totaux = tableau.totaux(polices, &mut blanc, 0.0);
    let hauteurs: Vec<f32> = liste
        .lignes
        .iter()
        .map(|l| tableau.ligne(polices, &mut blanc, 0.0, l))
        .collect();
    let mut pages: Vec<std::ops::Range<usize>> = Vec::new();
    let mut debut = 0;
    let mut y = haut_corps + h_criteres + H_ENTETE_TABLEAU;
    for (k, h) in hauteurs.iter().enumerate() {
        if y + h > bas_corps && k > debut {
            pages.push(debut..k);
            debut = k;
            y = haut_corps + H_ENTETE_TABLEAU;
        }
        y += h;
    }
    pages.push(debut..liste.lignes.len());
    let totaux_a_part = y + h_totaux > bas_corps;
    let nombre = pages.len() + usize::from(totaux_a_part);

    let mut document = Document::new();
    document.set_metadata(
        Metadata::new()
            .title(liste.titre.clone())
            .language("fr".into()),
    );
    for k in 0..nombre {
        let reglages = PageSettings::from_wh(LARGEUR, HAUTEUR).ok_or("format de page invalide")?;
        let mut page = document.start_page_with(reglages);
        let mut surface = page.surface();
        let mut t = Toile {
            s: Some(&mut surface),
        };
        t.rectangle(0.0, 0.0, BANDE, HAUTEUR, liste.couleur);
        let mut y = HAUT + m.en_tete(&mut t, HAUT);
        if k == 0 {
            y += tableau.criteres(&m, polices, &mut t, y);
        }
        if let Some(rangs) = pages.get(k) {
            y += tableau.entete(polices, &mut t, y);
            for ligne in &liste.lignes[rangs.clone()] {
                y += tableau.ligne(polices, &mut t, y, ligne);
            }
        }
        if k + 1 == nombre {
            tableau.totaux(polices, &mut t, y);
        }
        m.pied(&mut t, k + 1, nombre);
        surface.finish();
        page.finish();
    }
    document.finish().map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modele::TiersPdf;

    fn lettre() -> LettrePdf {
        LettrePdf {
            couleur: [0x2d, 0x3a, 0x8c],
            lieu_et_date: "Le 11/06/2026".into(),
            destinataire: TiersPdf {
                nom: "Client fictif".into(),
                adresse: "01 BP 0000".into(),
                ..Default::default()
            },
            titre: "LETTRE DE RELANCE".into(),
            objet: "Objet : facture n° F2026-0001 impayée".into(),
            avant: "Madame, Monsieur,\n\nTexte fictif.".into(),
            apres: "Formule fictive.".into(),
            recapitulatif: vec![("Facture".into(), "F2026-0001".into())],
            signataire: "LA COMPTABILITE".into(),
            ..Default::default()
        }
    }

    #[test]
    fn lettre_produite_ou_refusee_si_trop_longue() {
        let p = Polices::embarquees();
        assert!(generer_lettre(&lettre(), &p).unwrap().starts_with(b"%PDF-"));
        let longue = LettrePdf {
            avant: "Paragraphe fictif assez long pour remplir une ligne entière. ".repeat(400),
            ..lettre()
        };
        assert!(generer_lettre(&longue, &p).is_err());
    }

    #[test]
    fn liste_paginee_avec_totaux() {
        let p = Polices::embarquees();
        let colonnes = vec![
            ColonnePdf {
                titre: "Date".into(),
                largeur: 60.0,
                a_droite: false,
            },
            ColonnePdf {
                titre: "Tiers".into(),
                largeur: 0.0,
                a_droite: false,
            },
            ColonnePdf {
                titre: "Montant".into(),
                largeur: 80.0,
                a_droite: true,
            },
        ];
        for n in [0, 5, 150] {
            let liste = ListePdf {
                titre: "REGISTRE".into(),
                date: "11/06/2026".into(),
                criteres: vec!["Période : du 01/05/2026 au 31/05/2026".into()],
                colonnes: colonnes.clone(),
                lignes: (0..n)
                    .map(|k| vec!["03/05/2026".into(), format!("Tiers {k}"), "1 000".into()])
                    .collect(),
                totaux: vec![TotalPdf {
                    libelle: "Total".into(),
                    valeur: "1 000 FCFA".into(),
                    principal: true,
                }],
                ..Default::default()
            };
            assert!(
                generer_liste(&liste, &p).unwrap().starts_with(b"%PDF-"),
                "{n} lignes"
            );
        }
        assert!(generer_liste(&ListePdf::default(), &p).is_err());
    }
}
