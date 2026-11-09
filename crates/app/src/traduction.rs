//! Langue de l'interface (décision du 2026-11-09 : sept langues, choix par compte).
//!
//! Le français est la langue source : les textes sont écrits en français dans les
//! `.slint` (`@tr("...")`) et dans le code Rust ([`tr`]). Les six autres langues
//! sont dans `lang/<code>/LC_MESSAGES/kadenz.po`, embarqués dans l'exécutable :
//! Slint les compile lui-même (build.rs), et [`tr`] relit les mêmes fichiers pour
//! les textes construits côté Rust, afin qu'il n'y ait qu'une seule source.

use std::cell::Cell;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::OnceLock;

/// Fichiers de traduction embarqués, par code de langue.
const FICHIERS: [(&str, &str); 6] = [
    ("en", include_str!("../lang/en/LC_MESSAGES/kadenz.po")),
    ("ar", include_str!("../lang/ar/LC_MESSAGES/kadenz.po")),
    ("ja", include_str!("../lang/ja/LC_MESSAGES/kadenz.po")),
    ("zh", include_str!("../lang/zh/LC_MESSAGES/kadenz.po")),
    ("tr", include_str!("../lang/tr/LC_MESSAGES/kadenz.po")),
    ("es", include_str!("../lang/es/LC_MESSAGES/kadenz.po")),
];

thread_local! {
    /// Code de la langue en cours ; vide pour le français.
    static COURANTE: Cell<&'static str> = const { Cell::new("") };
}

/// Choisit la langue de l'interface, pour Slint comme pour [`tr`], et renvoie le
/// code retenu. Un code inconnu (ou "fr") revient au français (code vide).
pub fn choisir(code: &str) -> &'static str {
    let code = FICHIERS
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(c, _)| *c);
    COURANTE.set(code);
    // Sans traductions embarquées (cas impossible : build.rs les ajoute toujours),
    // l'interface reste simplement en français.
    let _ = slint::select_bundled_translation(code);
    code
}

/// Langue d'avant toute connexion : le français, ou celle de `KADENZ_LANGUE`
/// (fr, en, ar, ja, zh, tr, es) pour le développement.
pub fn choisir_demarrage() -> &'static str {
    choisir(&std::env::var("KADENZ_LANGUE").unwrap_or_default())
}

/// Traduit un texte français dans la langue en cours ; le renvoie tel quel s'il
/// n'a pas de traduction.
pub fn tr(texte: &str) -> String {
    let code = COURANTE.get();
    if code.is_empty() {
        return texte.to_owned();
    }
    tables()
        .get(code)
        .and_then(|table| table.get(texte))
        .cloned()
        .unwrap_or_else(|| texte.to_owned())
}

/// Vrai si la langue en cours se lit de droite à gauche (l'arabe). Même source
/// que `Theme.rtl` côté Slint : la traduction du texte « ltr » vaut « rtl ».
pub fn droite_a_gauche() -> bool {
    tr("ltr") == "rtl"
}

/// Comme [`tr`], puis remplace chaque `{}` par l'argument suivant, comme `@tr` côté Slint.
pub fn trf(texte: &str, arguments: &[&dyn Display]) -> String {
    let modele = tr(texte);
    let mut sortie = String::with_capacity(modele.len());
    let mut arguments = arguments.iter();
    let mut reste = modele.as_str();
    while let Some(position) = reste.find("{}") {
        sortie.push_str(&reste[..position]);
        match arguments.next() {
            Some(argument) => sortie.push_str(&argument.to_string()),
            None => sortie.push_str("{}"),
        }
        reste = &reste[position + 2..];
    }
    sortie.push_str(reste);
    sortie
}

fn tables() -> &'static HashMap<&'static str, HashMap<String, String>> {
    static TABLES: OnceLock<HashMap<&'static str, HashMap<String, String>>> = OnceLock::new();
    TABLES.get_or_init(|| {
        FICHIERS
            .iter()
            .map(|(code, contenu)| (*code, lire_po(contenu)))
            .collect()
    })
}

/// Lit un fichier .po : paires msgid / msgstr, chaînes sur une ou plusieurs lignes.
/// Les entrées sans traduction (msgstr vide) et l'en-tête (msgid vide) sont ignorés.
pub(crate) fn lire_po(contenu: &str) -> HashMap<String, String> {
    #[derive(PartialEq)]
    enum Champ {
        Aucun,
        Id,
        Traduction,
    }
    let mut table = HashMap::new();
    let (mut id, mut traduction, mut champ) = (String::new(), String::new(), Champ::Aucun);
    let mut ranger = |id: &mut String, traduction: &mut String| {
        if !id.is_empty() && !traduction.is_empty() {
            table.insert(std::mem::take(id), std::mem::take(traduction));
        }
        id.clear();
        traduction.clear();
    };
    for ligne in contenu.lines().map(str::trim) {
        if let Some(reste) = ligne.strip_prefix("msgid ") {
            ranger(&mut id, &mut traduction);
            id.push_str(&chaine_po(reste));
            champ = Champ::Id;
        } else if let Some(reste) = ligne.strip_prefix("msgstr ") {
            traduction.push_str(&chaine_po(reste));
            champ = Champ::Traduction;
        } else if ligne.starts_with('"') {
            match champ {
                Champ::Id => id.push_str(&chaine_po(ligne)),
                Champ::Traduction => traduction.push_str(&chaine_po(ligne)),
                Champ::Aucun => {}
            }
        } else if ligne.is_empty() || ligne.starts_with('#') {
            champ = Champ::Aucun;
        }
    }
    ranger(&mut id, &mut traduction);
    table
}

/// Contenu d'une chaîne .po entre guillemets, échappements résolus.
fn chaine_po(brut: &str) -> String {
    let interieur = brut
        .trim()
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or("");
    let mut sortie = String::with_capacity(interieur.len());
    let mut caracteres = interieur.chars();
    while let Some(c) = caracteres.next() {
        if c != '\\' {
            sortie.push(c);
            continue;
        }
        match caracteres.next() {
            Some('n') => sortie.push('\n'),
            Some('t') => sortie.push('\t'),
            Some(autre) => sortie.push(autre),
            None => {}
        }
    }
    sortie
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lit_les_chaines_simples_et_sur_plusieurs_lignes() {
        let table = lire_po(
            "msgid \"\"\nmsgstr \"Content-Type: text/plain; charset=UTF-8\\n\"\n\n\
             msgid \"Devis\"\nmsgstr \"Quotes\"\n\n\
             msgid \"\"\n\"Mot de \\\"passe\\\"\"\nmsgstr \"\"\n\"Pass\"\n\"word\"\n\n\
             msgid \"Sans traduction\"\nmsgstr \"\"\n",
        );
        assert_eq!(table.get("Devis").map(String::as_str), Some("Quotes"));
        assert_eq!(
            table.get("Mot de \"passe\"").map(String::as_str),
            Some("Password")
        );
        assert!(!table.contains_key("Sans traduction"));
        assert!(!table.contains_key(""));
    }

    /// Littéraux qui suivent `debut` dans `source` : `@tr("...")` ou `tr("...")`.
    fn litteraux(source: &str, debut: &str, sortie: &mut Vec<String>) {
        let mut reste = source;
        while let Some(position) = reste.find(debut) {
            reste = &reste[position + debut.len()..];
            let Some(apres) = reste.trim_start().strip_prefix('"') else {
                continue;
            };
            let mut texte = String::new();
            let mut caracteres = apres.chars();
            while let Some(c) = caracteres.next() {
                match c {
                    '"' => break,
                    '\\' => match caracteres.next() {
                        Some('u') => {
                            // \u{00A0} : seul échappement Unicode employé dans les textes.
                            let code: String = caracteres
                                .by_ref()
                                .skip(1)
                                .take_while(|c| *c != '}')
                                .collect();
                            if let Some(c) =
                                u32::from_str_radix(&code, 16).ok().and_then(char::from_u32)
                            {
                                texte.push(c);
                            }
                        }
                        Some(autre) => texte.push(autre),
                        None => {}
                    },
                    _ => texte.push(c),
                }
            }
            sortie.push(texte);
        }
    }

    /// Chaque texte de l'interface (`@tr` des .slint, `tr` et `trf` du code Rust)
    /// a une traduction non vide dans les six langues, avec autant de `{}`.
    #[test]
    fn chaque_texte_a_ses_six_traductions() {
        let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut textes = Vec::new();
        let mut lire = |dossier: &str, extension: &str, debuts: &[&str]| {
            let fichiers = std::fs::read_dir(racine.join(dossier)).expect("dossier des sources");
            for fichier in fichiers.map(|f| f.expect("entrée du dossier").path()) {
                let nom = fichier.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !nom.ends_with(extension) || nom == "traduction.rs" {
                    continue;
                }
                let contenu = std::fs::read_to_string(&fichier).expect("fichier source");
                let contenu = contenu.split("#[cfg(test)]").next().unwrap_or("");
                for debut in debuts {
                    litteraux(contenu, debut, &mut textes);
                }
            }
        };
        lire("ui", ".slint", &["@tr("]);
        lire("src", ".rs", &[" tr(", "(tr(", " trf(", "(trf("]);
        assert!(textes.len() > 300, "{} textes seulement", textes.len());

        let mut manques = Vec::new();
        for (code, contenu) in FICHIERS {
            let table = lire_po(contenu);
            for texte in &textes {
                match table.get(texte) {
                    None => manques.push(format!("[{code}] sans traduction : {texte:?}")),
                    Some(t) if t.matches("{}").count() != texte.matches("{}").count() => {
                        manques.push(format!("[{code}] nombre de {{}} différent : {texte:?}"))
                    }
                    Some(_) => {}
                }
            }
        }
        assert!(manques.is_empty(), "{}", manques.join("\n"));
    }

    #[test]
    fn remplace_les_arguments_dans_l_ordre() {
        choisir("fr");
        assert_eq!(
            trf("Base : {} (schéma {})", &[&"a.sqlite", &3]),
            "Base : a.sqlite (schéma 3)"
        );
        assert_eq!(trf("{} et {}", &[&1]), "1 et {}");
    }
}
