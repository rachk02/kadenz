//! Notes de version affichées dans la fenêtre de mise à jour.
//!
//! Les notes viennent du corps de la release GitHub, écrit en Markdown. La fenêtre
//! affiche du texte brut : ce module retire la syntaxe Markdown (titres, puces,
//! gras, code en ligne) et écarte les sections destinées aux développeurs, pour que
//! l'utilisateur ne lise que ce qui le concerne.
//!
//! Règle d'écriture (voir CHANGELOG.md) : les notes parlent à l'utilisateur, sans
//! terme interne. Ce qui n'intéresse que l'équipe va sous un titre `### Technique`,
//! que la publication (`scripts/release.ps1`) retire et que ce module écarte aussi
//! pour les anciennes releases.

/// Titre de section réservé aux détails techniques, jamais montré à l'utilisateur.
const SECTION_TECHNIQUE: &str = "technique";

fn est_technique(titre: &str) -> bool {
    titre.trim().eq_ignore_ascii_case(SECTION_TECHNIQUE)
}

/// Texte brut affichable : titres sans dièses, puces « • », sans gras ni code en
/// ligne, sans section technique, sans ligne vide en double.
pub fn pour_affichage(markdown: &str) -> String {
    let mut sortie: Vec<String> = Vec::new();
    let mut ignorer = false;
    for ligne in markdown.replace("\r\n", "\n").replace('\r', "\n").lines() {
        let ligne = ligne.trim_end();
        let net = ligne.trim_start();
        if net.starts_with('#') {
            let titre = net.trim_start_matches('#').trim();
            ignorer = est_technique(titre);
            if ignorer || titre.is_empty() {
                continue;
            }
            if !sortie.is_empty() && sortie.last().is_some_and(|l| !l.is_empty()) {
                sortie.push(String::new());
            }
            sortie.push(nettoyer(titre));
            continue;
        }
        if ignorer {
            continue;
        }
        let texte = if let Some(reste) = net.strip_prefix("- ").or_else(|| net.strip_prefix("* ")) {
            format!("\u{2022} {}", nettoyer(reste))
        } else {
            nettoyer(net)
        };
        if texte.is_empty() && sortie.last().is_none_or(|l| l.is_empty()) {
            continue;
        }
        sortie.push(texte);
    }
    while sortie.last().is_some_and(|l| l.is_empty()) {
        sortie.pop();
    }
    sortie.join("\n")
}

/// Retire le gras, l'italique par tirets bas doublés et les accents graves.
fn nettoyer(texte: &str) -> String {
    texte
        .replace("**", "")
        .replace("__", "")
        .replace('`', "")
        .trim()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_titres_et_les_puces_perdent_leur_syntaxe() {
        let md = "### Ajouté\r\n\r\n- Reprise d'un **document** Word.\r\n- Fichier `LISEZMOI.txt`.\r\n\r\n### Corrigé\r\n\r\n- La page ne plante plus.\r\n";
        assert_eq!(
            pour_affichage(md),
            "Ajouté\n\n\u{2022} Reprise d'un document Word.\n\u{2022} Fichier LISEZMOI.txt.\n\nCorrigé\n\n\u{2022} La page ne plante plus."
        );
    }

    #[test]
    fn la_section_technique_est_ecartee() {
        let md = "### Corrigé\n- La page ne plante plus.\n\n### Technique\n- récursion dans le FlexboxLayout.\n\n### Ajouté\n- Un bouton.";
        let texte = pour_affichage(md);
        assert!(texte.contains("La page ne plante plus."));
        assert!(texte.contains("Un bouton."));
        assert!(!texte.contains("récursion") && !texte.contains("Technique"));
    }

    #[test]
    fn un_texte_sans_markdown_reste_tel_quel() {
        assert_eq!(pour_affichage("  Notes  "), "Notes");
        assert_eq!(pour_affichage(""), "");
    }
}
