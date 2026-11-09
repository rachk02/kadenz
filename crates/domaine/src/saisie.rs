//! Conversion entre les saisies de l'utilisateur et les valeurs du domaine.
//!
//! La virgule étant le séparateur décimal en français, les listes saisies sur
//! une seule ligne sont séparées par des points-virgules : « 0 ; 18 ».

use crate::calcul::{CENT_POUR_CENT, Montant, Pourcentage};

/// Séparateurs de milliers possibles : espace, espace insécable (U+00A0) et espace
/// fine insécable (U+202F), celle qu'écrit Excel en français.
fn est_separateur_milliers(c: char) -> bool {
    matches!(c, ' ' | '\u{00A0}' | '\u{202F}')
}

/// Lit un montant en FCFA : « 485000 », « 485 000 », « 485 000,00 CFA », « 1 250 000 FCFA ».
/// Des centimes nuls (« ,00 ») sont acceptés ; des centimes non nuls sont refusés,
/// le FCFA n'ayant pas de subdivision. Les montants négatifs sont refusés.
pub fn montant_depuis_texte(texte: &str) -> Option<Montant> {
    let mut t = texte.trim();
    for devise in ["FCFA", "CFA", "F"] {
        if let Some(reste) = t.strip_suffix(devise) {
            t = reste.trim_end_matches(est_separateur_milliers);
            break;
        }
    }
    let (entier, decimales) = match t.split_once([',', '.']) {
        Some((e, d)) => (e, d),
        None => (t, ""),
    };
    if !decimales.chars().all(|c| c == '0') {
        return None;
    }
    let chiffres: String = entier
        .chars()
        .filter(|c| !est_separateur_milliers(*c))
        .collect();
    if chiffres.is_empty() || !chiffres.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    chiffres.parse().ok()
}

/// Écrit un montant avec des espaces insécables entre les milliers : « 1 250 000 ».
pub fn montant_en_texte(m: Montant) -> String {
    let chiffres = m.unsigned_abs().to_string();
    let mut s = String::new();
    for (i, c) in chiffres.chars().enumerate() {
        if i > 0 && (chiffres.len() - i).is_multiple_of(3) {
            s.push('\u{00A0}');
        }
        s.push(c);
    }
    if m < 0 { format!("-{s}") } else { s }
}

/// Lit un pourcentage saisi : « 18 », « 18 % », « 2,5 », « 2.5 ». Au plus deux
/// décimales, entre 0 et 100 %. Renvoie des centièmes de point (18 % = 1800).
pub fn pourcentage_depuis_texte(texte: &str) -> Option<Pourcentage> {
    let nettoye = texte.trim().trim_end_matches('%').trim().replace(',', ".");
    if nettoye.is_empty() {
        return None;
    }
    let (entier, decimales) = match nettoye.split_once('.') {
        Some((e, d)) => (e, d),
        None => (nettoye.as_str(), ""),
    };
    if entier.is_empty()
        || decimales.len() > 2
        || !entier.chars().all(|c| c.is_ascii_digit())
        || !decimales.chars().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    let entier: i64 = entier.parse().ok()?;
    let decimales: i64 = format!("{decimales:0<2}").parse().ok()?;
    let valeur = entier.checked_mul(100)?.checked_add(decimales)?;
    (0..=CENT_POUR_CENT).contains(&valeur).then_some(valeur)
}

/// Écrit un pourcentage sans le symbole, avec une virgule décimale : 250 donne « 2,5 ».
pub fn pourcentage_en_texte(p: Pourcentage) -> String {
    let (entier, centiemes) = (p / 100, p % 100);
    match centiemes {
        0 => entier.to_string(),
        c if c % 10 == 0 => format!("{entier},{}", c / 10),
        c => format!("{entier},{c:02}"),
    }
}

/// Lit une liste de pourcentages séparés par des points-virgules, sans doublon,
/// triée. `None` si une valeur est invalide ou si la liste est vide.
pub fn liste_pourcentages_depuis_texte(texte: &str) -> Option<Vec<Pourcentage>> {
    let mut valeurs = texte
        .split(';')
        .filter(|morceau| !morceau.trim().is_empty())
        .map(pourcentage_depuis_texte)
        .collect::<Option<Vec<_>>>()?;
    valeurs.sort_unstable();
    valeurs.dedup();
    (!valeurs.is_empty()).then_some(valeurs)
}

pub fn liste_pourcentages_en_texte(valeurs: &[Pourcentage]) -> String {
    valeurs
        .iter()
        .map(|&p| pourcentage_en_texte(p))
        .collect::<Vec<_>>()
        .join(" ; ")
}

/// Lit une liste de libellés séparés par des points-virgules, sans vide ni doublon,
/// dans l'ordre de saisie.
pub fn liste_libelles_depuis_texte(texte: &str) -> Vec<String> {
    let mut libelles: Vec<String> = Vec::new();
    for morceau in texte.split(';') {
        let libelle = morceau.trim();
        if !libelle.is_empty() && !libelles.iter().any(|l| l == libelle) {
            libelles.push(libelle.to_string());
        }
    }
    libelles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn montants() {
        assert_eq!(montant_depuis_texte("485000"), Some(485_000));
        assert_eq!(montant_depuis_texte("485 000"), Some(485_000));
        assert_eq!(montant_depuis_texte("485\u{202F}000,00 CFA"), Some(485_000));
        assert_eq!(
            montant_depuis_texte("1\u{00A0}250\u{00A0}000,00\u{00A0}CFA"),
            Some(1_250_000)
        );
        assert_eq!(montant_depuis_texte("4 500 000 FCFA"), Some(4_500_000));
        assert_eq!(montant_depuis_texte("0"), Some(0));
        for invalide in ["", "abc", "12,50", "-5", "1 2a", "CFA", "12.5"] {
            assert_eq!(montant_depuis_texte(invalide), None, "« {invalide} »");
        }
        assert_eq!(montant_en_texte(1_250_000), "1\u{00A0}250\u{00A0}000");
        assert_eq!(montant_en_texte(485), "485");
        assert_eq!(
            montant_depuis_texte(&montant_en_texte(65_000)),
            Some(65_000)
        );
    }

    #[test]
    fn pourcentages_valides() {
        assert_eq!(pourcentage_depuis_texte("18"), Some(1800));
        assert_eq!(pourcentage_depuis_texte(" 18 % "), Some(1800));
        assert_eq!(pourcentage_depuis_texte("2,5"), Some(250));
        assert_eq!(pourcentage_depuis_texte("2.25"), Some(225));
        assert_eq!(pourcentage_depuis_texte("0"), Some(0));
        assert_eq!(pourcentage_depuis_texte("100"), Some(10_000));
    }

    #[test]
    fn pourcentages_invalides() {
        for texte in ["", "abc", "101", "2,555", "-5", ",5", "1 8"] {
            assert_eq!(pourcentage_depuis_texte(texte), None, "« {texte} »");
        }
    }

    #[test]
    fn aller_retour_texte() {
        for p in [0, 250, 225, 1800, 10_000] {
            assert_eq!(pourcentage_depuis_texte(&pourcentage_en_texte(p)), Some(p));
        }
        assert_eq!(pourcentage_en_texte(250), "2,5");
        assert_eq!(pourcentage_en_texte(205), "2,05");
    }

    #[test]
    fn listes() {
        assert_eq!(
            liste_pourcentages_depuis_texte("18 ; 0 ; 18"),
            Some(vec![0, 1800])
        );
        assert_eq!(liste_pourcentages_depuis_texte("0 ; x"), None);
        assert_eq!(liste_pourcentages_depuis_texte(" ; "), None);
        assert_eq!(liste_pourcentages_en_texte(&[0, 500, 1000]), "0 ; 5 ; 10");
        assert_eq!(
            liste_libelles_depuis_texte("Espèce ; Chèque ;; Espèce "),
            vec!["Espèce".to_string(), "Chèque".to_string()]
        );
    }
}
