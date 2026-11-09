//! Comparaison de textes saisis : recherche dans les listes et reconnaissance des
//! en-têtes de colonnes à l'import, sans tenir compte de la casse ni des accents.

/// Forme de comparaison d'un texte : minuscules, sans accents, espaces réduits.
///
/// « Boîte  Postale » et « BOITE POSTALE » donnent la même clé.
pub fn cle_comparaison(texte: &str) -> String {
    let mut cle = String::with_capacity(texte.len());
    let mut espace_en_attente = false;
    for c in texte.chars() {
        if c.is_whitespace() {
            espace_en_attente = !cle.is_empty();
            continue;
        }
        if espace_en_attente {
            cle.push(' ');
            espace_en_attente = false;
        }
        for minuscule in c.to_lowercase() {
            match sans_accent(minuscule) {
                Some(remplacement) => cle.push_str(remplacement),
                None => cle.push(minuscule),
            }
        }
    }
    cle
}

/// Vrai si `texte` contient `recherche`, à la casse et aux accents près.
/// Une recherche vide correspond à tout.
pub fn contient(texte: &str, recherche: &str) -> bool {
    cle_comparaison(texte).contains(&cle_comparaison(recherche))
}

fn sans_accent(c: char) -> Option<&'static str> {
    Some(match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => "a",
        'æ' => "ae",
        'ç' => "c",
        'è' | 'é' | 'ê' | 'ë' => "e",
        'ì' | 'í' | 'î' | 'ï' => "i",
        'ñ' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' => "o",
        'œ' => "oe",
        'ù' | 'ú' | 'û' | 'ü' => "u",
        'ý' | 'ÿ' => "y",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn casse_accents_et_espaces() {
        assert_eq!(cle_comparaison("  Boîte  Postale "), "boite postale");
        assert_eq!(cle_comparaison("DÉSIGNATION"), "designation");
        assert_eq!(cle_comparaison("Référence"), "reference");
        assert_eq!(cle_comparaison("Œuvre"), "oeuvre");
        assert_eq!(cle_comparaison("TELEPHONE\u{00A0}fixe"), "telephone fixe");
    }

    #[test]
    fn recherche() {
        assert!(contient("Société Générale", "generale"));
        assert!(contient("Société Générale", ""));
        assert!(!contient("Société Générale", "banque"));
    }
}
