//! Export Excel de Kadenz (cahier des charges, sections 5, 6 et 8).
//!
//! Le modèle est indépendant des données métier : l'application décrit chaque
//! feuille (titre, informations d'en-tête, tableau, totaux) avec des textes
//! déjà traduits, et [`classeur`] produit le fichier xlsx. Les montants, les
//! quantités et les dates sont écrits comme des nombres et de vraies dates
//! Excel, pour que l'utilisateur puisse les additionner et les trier.

use chrono::NaiveDate;
use rust_xlsxwriter::{Format, FormatAlign, Workbook, Worksheet, XlsxError};

/// Valeur d'une cellule.
#[derive(Debug, Clone, PartialEq)]
pub enum Valeur {
    Vide,
    Texte(String),
    /// Montant en FCFA entiers.
    Montant(i64),
    /// Quantité en millièmes (1000 = 1 unité).
    Quantite(i64),
    /// Pourcentage en centièmes de point (1800 = 18 %).
    Pourcentage(i64),
    /// Nombre entier (jours de retard, nombre de lignes).
    Entier(i64),
    Date(NaiveDate),
}

impl From<&str> for Valeur {
    fn from(t: &str) -> Self {
        Valeur::Texte(t.to_owned())
    }
}

impl From<String> for Valeur {
    fn from(t: String) -> Self {
        Valeur::Texte(t)
    }
}

impl From<Option<NaiveDate>> for Valeur {
    fn from(d: Option<NaiveDate>) -> Self {
        d.map_or(Valeur::Vide, Valeur::Date)
    }
}

/// Colonne du tableau : titre et largeur en caractères.
#[derive(Debug, Clone, PartialEq)]
pub struct Colonne {
    pub titre: String,
    pub largeur: f64,
}

impl Colonne {
    pub fn new(titre: impl Into<String>, largeur: f64) -> Colonne {
        Colonne {
            titre: titre.into(),
            largeur,
        }
    }
}

/// Une feuille : titre, couples libellé et valeur au-dessus du tableau
/// (critères d'un registre, en-tête d'un document), tableau, puis couples
/// libellé et valeur sous le tableau (totaux), alignés sur ses deux dernières
/// colonnes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Feuille {
    /// Nom de l'onglet (31 caractères au plus, sans `[]:*?/\`).
    pub nom: String,
    pub titre: String,
    pub entete: Vec<(String, Valeur)>,
    pub colonnes: Vec<Colonne>,
    pub lignes: Vec<Vec<Valeur>>,
    pub totaux: Vec<(String, Valeur)>,
}

/// Nom d'onglet admis par Excel.
fn nom_onglet(nom: &str) -> String {
    let propre: String = nom
        .chars()
        .map(|c| if "[]:*?/\\".contains(c) { '-' } else { c })
        .collect();
    let propre = propre.trim().trim_matches('\'');
    let court: String = propre.chars().take(31).collect();
    if court.is_empty() {
        "Feuille".to_owned()
    } else {
        court
    }
}

struct Formats {
    titre: Format,
    gras: Format,
    entete_tableau: Format,
    texte: Format,
    montant: Format,
    montant_gras: Format,
    quantite: Format,
    pourcentage: Format,
    pourcentage_entier: Format,
    entier: Format,
    date: Format,
}

impl Formats {
    fn new() -> Formats {
        let montant = Format::new().set_num_format("#,##0");
        Formats {
            titre: Format::new().set_bold().set_font_size(14),
            gras: Format::new().set_bold(),
            entete_tableau: Format::new()
                .set_bold()
                .set_text_wrap()
                .set_align(FormatAlign::VerticalCenter)
                .set_border_bottom(rust_xlsxwriter::FormatBorder::Thin),
            texte: Format::new(),
            montant_gras: montant.clone().set_bold(),
            montant,
            quantite: Format::new().set_num_format("General"),
            pourcentage: Format::new().set_num_format("0.##\" %\""),
            pourcentage_entier: Format::new().set_num_format("0\" %\""),
            entier: Format::new().set_num_format("0"),
            date: Format::new()
                .set_num_format("dd/mm/yyyy")
                .set_align(FormatAlign::Left),
        }
    }
}

fn ecrire(
    f: &mut Worksheet,
    formats: &Formats,
    ligne: u32,
    col: u16,
    v: &Valeur,
    gras: bool,
) -> Result<(), XlsxError> {
    match v {
        Valeur::Vide => {}
        Valeur::Texte(t) => {
            let format = if gras { &formats.gras } else { &formats.texte };
            f.write_string_with_format(ligne, col, t, format)?;
        }
        Valeur::Montant(m) => {
            let format = if gras {
                &formats.montant_gras
            } else {
                &formats.montant
            };
            f.write_number_with_format(ligne, col, *m as f64, format)?;
        }
        Valeur::Quantite(q) => {
            f.write_number_with_format(ligne, col, *q as f64 / 1000.0, &formats.quantite)?;
        }
        Valeur::Pourcentage(p) => {
            // Sans partie décimale, « 0.## » laisserait le séparateur (« 18. % »).
            let format = if p % 100 == 0 {
                &formats.pourcentage_entier
            } else {
                &formats.pourcentage
            };
            f.write_number_with_format(ligne, col, *p as f64 / 100.0, format)?;
        }
        Valeur::Entier(n) => {
            f.write_number_with_format(ligne, col, *n as f64, &formats.entier)?;
        }
        Valeur::Date(d) => {
            f.write_date_with_format(ligne, col, d, &formats.date)?;
        }
    }
    Ok(())
}

fn remplir(f: &mut Worksheet, formats: &Formats, feuille: &Feuille) -> Result<(), XlsxError> {
    f.set_name(nom_onglet(&feuille.nom))?;
    let mut ligne = 0u32;
    f.write_string_with_format(ligne, 0, &feuille.titre, &formats.titre)?;
    ligne += 1;
    for (libelle, valeur) in &feuille.entete {
        f.write_string_with_format(ligne, 0, libelle, &formats.gras)?;
        ecrire(f, formats, ligne, 1, valeur, false)?;
        ligne += 1;
    }
    ligne += 1;

    let largeur_libelle = feuille
        .entete
        .iter()
        .map(|(l, _)| l.chars().count() as f64 + 2.0)
        .fold(0.0, f64::max);
    let nb = feuille.colonnes.len() as u16;
    for (i, c) in feuille.colonnes.iter().enumerate() {
        let largeur = if i == 0 {
            c.largeur.max(largeur_libelle)
        } else {
            c.largeur
        };
        f.set_column_width(i as u16, largeur)?;
        f.write_string_with_format(ligne, i as u16, &c.titre, &formats.entete_tableau)?;
    }
    let premiere = ligne;
    ligne += 1;
    for valeurs in &feuille.lignes {
        for (i, v) in valeurs.iter().enumerate().take(nb as usize) {
            ecrire(f, formats, ligne, i as u16, v, false)?;
        }
        ligne += 1;
    }
    if nb > 0 {
        f.set_freeze_panes(premiere + 1, 0)?;
        f.autofilter(premiere, 0, ligne.saturating_sub(1).max(premiere), nb - 1)?;
    }

    if !feuille.totaux.is_empty() {
        ligne += 1;
        let (col_libelle, col_valeur) = if nb >= 2 { (nb - 2, nb - 1) } else { (0, 1) };
        for (libelle, valeur) in &feuille.totaux {
            f.write_string_with_format(ligne, col_libelle, libelle, &formats.gras)?;
            ecrire(f, formats, ligne, col_valeur, valeur, true)?;
            ligne += 1;
        }
    }
    Ok(())
}

/// Classeur xlsx des feuilles données, dans l'ordre.
pub fn classeur(feuilles: &[Feuille]) -> Result<Vec<u8>, String> {
    let formats = Formats::new();
    let mut c = Workbook::new();
    for feuille in feuilles {
        remplir(c.add_worksheet(), &formats, feuille).map_err(|e| e.to_string())?;
    }
    if feuilles.is_empty() {
        c.add_worksheet();
    }
    c.save_to_buffer().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use calamine::{Data, Reader, Xlsx};
    use std::io::Cursor;

    fn date(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    #[test]
    fn feuille_relue() {
        let feuille = Feuille {
            nom: "Factures: 2026/11".into(),
            titre: "Registre des factures".into(),
            entete: vec![("Du".into(), Valeur::Date(date(2026, 11, 1)))],
            colonnes: vec![
                Colonne::new("Numéro", 14.0),
                Colonne::new("Date", 11.0),
                Colonne::new("Quantité", 10.0),
                Colonne::new("TVA", 8.0),
                Colonne::new("Montant TTC", 14.0),
            ],
            lignes: vec![vec![
                "FA-0001".into(),
                Valeur::Date(date(2026, 11, 5)),
                Valeur::Quantite(2500),
                Valeur::Pourcentage(1800),
                Valeur::Montant(1_180_000),
            ]],
            totaux: vec![("Total".into(), Valeur::Montant(1_180_000))],
        };
        let octets = classeur(&[feuille]).unwrap();
        let mut x: Xlsx<_> = Xlsx::new(Cursor::new(octets)).unwrap();
        assert_eq!(x.sheet_names(), vec!["Factures- 2026-11".to_owned()]);
        let r = x.worksheet_range("Factures- 2026-11").unwrap();
        assert_eq!(
            r.get_value((0, 0)),
            Some(&Data::String("Registre des factures".into()))
        );
        assert_eq!(
            r.get_value((3, 4)),
            Some(&Data::String("Montant TTC".into()))
        );
        assert_eq!(r.get_value((4, 0)), Some(&Data::String("FA-0001".into())));
        match r.get_value((4, 1)) {
            Some(Data::DateTime(d)) => {
                let jours = (date(2026, 11, 5) - date(1899, 12, 30)).num_days();
                assert_eq!(d.as_f64(), jours as f64);
            }
            autre => panic!("date attendue : {autre:?}"),
        }
        assert_eq!(r.get_value((4, 2)), Some(&Data::Float(2.5)));
        assert_eq!(r.get_value((4, 3)), Some(&Data::Float(18.0)));
        assert_eq!(r.get_value((4, 4)), Some(&Data::Float(1_180_000.0)));
        assert_eq!(r.get_value((6, 3)), Some(&Data::String("Total".into())));
        assert_eq!(r.get_value((6, 4)), Some(&Data::Float(1_180_000.0)));
    }

    #[test]
    fn noms_d_onglet() {
        assert_eq!(nom_onglet("a/b"), "a-b");
        assert_eq!(nom_onglet(""), "Feuille");
        assert_eq!(nom_onglet(&"x".repeat(40)).chars().count(), 31);
    }
}
