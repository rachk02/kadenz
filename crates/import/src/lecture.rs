//! Lecture d'un fichier en tableau de textes : première ligne non vide en en-têtes,
//! lignes suivantes en données.

use std::path::Path;

use calamine::{Data, Reader, open_workbook_auto};

/// Contenu d'un fichier, cellules converties en texte.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Table {
    pub entetes: Vec<String>,
    /// Chaque ligne avec son numéro dans le fichier (à partir de 1).
    pub lignes: Vec<(usize, Vec<String>)>,
}

impl Table {
    /// Construit la table depuis des lignes brutes numérotées : la première ligne
    /// non vide donne les en-têtes, les lignes entièrement vides sont sautées.
    fn depuis_lignes(lignes: impl IntoIterator<Item = (usize, Vec<String>)>) -> Table {
        let mut table = Table::default();
        let mut entetes_lus = false;
        for (numero, cellules) in lignes {
            if cellules.iter().all(|c| c.trim().is_empty()) {
                continue;
            }
            if entetes_lus {
                table.lignes.push((numero, cellules));
            } else {
                table.entetes = cellules.iter().map(|c| c.trim().to_string()).collect();
                entetes_lus = true;
            }
        }
        table
    }
}

/// Lit un fichier CSV, Excel ou OpenDocument d'après son extension.
pub fn lire_fichier(chemin: &Path) -> Result<Table, String> {
    let extension = chemin
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match extension.as_str() {
        "csv" | "txt" => {
            let octets = std::fs::read(chemin)
                .map_err(|e| format!("lecture du fichier impossible : {e}"))?;
            lire_csv(&octets)
        }
        "xlsx" | "xlsm" | "xlsb" | "xls" | "ods" => lire_classeur(chemin),
        _ => Err(
            "format non reconnu : choisir un fichier CSV, Excel (xlsx, xls) ou OpenDocument (ods)"
                .into(),
        ),
    }
}

/// Décode le texte : UTF-8 (avec ou sans BOM), sinon Windows-1252, l'encodage
/// d'un CSV enregistré par Excel en français.
pub fn decoder(octets: &[u8]) -> String {
    let sans_bom = octets.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(octets);
    match std::str::from_utf8(sans_bom) {
        Ok(texte) => texte.to_string(),
        Err(_) => encoding_rs::WINDOWS_1252.decode(sans_bom).0.into_owned(),
    }
}

/// Séparateur le plus fréquent dans la première ligne non vide, hors guillemets :
/// point-virgule (Excel en français), virgule ou tabulation.
pub fn detecter_separateur(texte: &str) -> u8 {
    let premiere = texte.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    let mut comptes = [(b';', 0usize), (b',', 0), (b'\t', 0)];
    let mut entre_guillemets = false;
    for c in premiere.bytes() {
        if c == b'"' {
            entre_guillemets = !entre_guillemets;
        } else if !entre_guillemets {
            for (sep, n) in comptes.iter_mut() {
                if c == *sep {
                    *n += 1;
                }
            }
        }
    }
    comptes
        .iter()
        .max_by_key(|(_, n)| *n)
        .filter(|(_, n)| *n > 0)
        .map_or(b';', |(sep, _)| *sep)
}

pub fn lire_csv(octets: &[u8]) -> Result<Table, String> {
    let texte = decoder(octets);
    let mut lecteur = csv::ReaderBuilder::new()
        .delimiter(detecter_separateur(&texte))
        .has_headers(false)
        .flexible(true)
        .from_reader(texte.as_bytes());
    let mut lignes = Vec::new();
    for enregistrement in lecteur.records() {
        let enregistrement = enregistrement.map_err(|e| format!("fichier CSV illisible : {e}"))?;
        // Numéro de ligne déduit de la position en octets du début de
        // l'enregistrement : exact même avec des retours à la ligne entre guillemets.
        let numero = enregistrement.position().map_or(lignes.len() + 1, |p| {
            // La position peut tomber sur la fin de ligne qui précède
            // l'enregistrement (le « \n » d'un « \r\n ») : on la saute.
            let octets = texte.as_bytes();
            let mut debut = usize::try_from(p.byte()).unwrap_or(0).min(octets.len());
            while debut < octets.len() && matches!(octets[debut], b'\r' | b'\n') {
                debut += 1;
            }
            octets[..debut].iter().filter(|&&o| o == b'\n').count() + 1
        });
        lignes.push((numero, enregistrement.iter().map(String::from).collect()));
    }
    Ok(Table::depuis_lignes(lignes))
}

/// Texte d'une cellule de classeur. Un nombre entier stocké en virgule flottante
/// (cas de tous les nombres Excel) s'écrit sans décimale : 485000.0 donne « 485000 ».
pub fn cellule_en_texte(cellule: &Data) -> String {
    match cellule {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Int(i) => i.to_string(),
        Data::Float(f) if f.fract() == 0.0 && f.abs() < 9.0e15 => (*f as i64).to_string(),
        Data::Float(f) => f.to_string(),
        Data::Bool(b) => if *b { "VRAI" } else { "FAUX" }.into(),
        Data::DateTime(d) => d.to_string(),
        Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
        Data::Error(e) => format!("#{e}"),
    }
}

/// Première feuille non vide du classeur.
fn lire_classeur(chemin: &Path) -> Result<Table, String> {
    let mut classeur =
        open_workbook_auto(chemin).map_err(|e| format!("classeur illisible : {e}"))?;
    for (_, plage) in classeur.worksheets() {
        // Numéro de ligne Excel : la plage peut commencer plus bas que la ligne 1.
        let premiere = plage.start().map_or(0, |(ligne, _)| ligne as usize);
        let table = Table::depuis_lignes(plage.rows().enumerate().map(|(rang, ligne)| {
            (
                premiere + rang + 1,
                ligne.iter().map(cellule_en_texte).collect::<Vec<_>>(),
            )
        }));
        if !table.entetes.is_empty() {
            return Ok(table);
        }
    }
    Err("le classeur ne contient aucune feuille remplie".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodages() {
        assert_eq!(decoder("Désignation".as_bytes()), "Désignation");
        let avec_bom = [&[0xEF, 0xBB, 0xBF][..], "Prix".as_bytes()].concat();
        assert_eq!(decoder(&avec_bom), "Prix");
        // « Désignation » en Windows-1252 : é = 0xE9.
        assert_eq!(decoder(b"D\xE9signation"), "Désignation");
    }

    #[test]
    fn separateurs() {
        assert_eq!(detecter_separateur("NOM;ADRESSE;IFU\n"), b';');
        assert_eq!(detecter_separateur("nom,adresse\n"), b',');
        assert_eq!(detecter_separateur("nom\tadresse\n"), b'\t');
        assert_eq!(detecter_separateur("\"a;b\",c,d\n"), b',');
        assert_eq!(detecter_separateur("seule colonne\n"), b';');
    }

    #[test]
    fn csv_avec_lignes_vides() {
        let csv =
            "\n\nRÉFÉRENCE;DÉSIGNATION;PRIX\nA-1;\"Pelle; ronde\";1 500\n;;\nB-2;Brouette;12 000\n";
        let t = lire_csv(csv.as_bytes()).unwrap();
        assert_eq!(t.entetes, ["RÉFÉRENCE", "DÉSIGNATION", "PRIX"]);
        assert_eq!(t.lignes.len(), 2);
        assert_eq!(
            t.lignes[0],
            (4, vec!["A-1".into(), "Pelle; ronde".into(), "1 500".into()])
        );
        assert_eq!(t.lignes[1].0, 6);
    }

    #[test]
    fn numeros_de_ligne_avec_crlf() {
        let t = lire_csv(
            b"NOM;IFU
A;1
B;2
",
        )
        .unwrap();
        let numeros: Vec<usize> = t.lignes.iter().map(|(n, _)| *n).collect();
        assert_eq!(numeros, [2, 3]);
    }

    #[test]
    fn numeros_de_ligne_avec_crlf_et_ligne_vide() {
        let csv = "DÉSIGNATION;RÉFÉRENCE;PRIX;Remarque\r\nA;B;1;x\r\n\r\nC;D;2;\r\n";
        let t = lire_csv(csv.as_bytes()).unwrap();
        let numeros: Vec<usize> = t.lignes.iter().map(|(n, _)| *n).collect();
        assert_eq!(numeros, [2, 4]);
    }

    #[test]
    fn cellules_de_classeur() {
        assert_eq!(cellule_en_texte(&Data::Float(485000.0)), "485000");
        assert_eq!(cellule_en_texte(&Data::Float(12.5)), "12.5");
        assert_eq!(cellule_en_texte(&Data::Int(7)), "7");
        assert_eq!(cellule_en_texte(&Data::Empty), "");
        assert_eq!(cellule_en_texte(&Data::String("Pelle".into())), "Pelle");
    }

    #[test]
    fn extension_inconnue() {
        assert!(lire_fichier(Path::new("produits.pdf")).is_err());
    }
}
