//! Champ des codes de licence courts (`ChampCode` de composants.slint,
//! docs/spec-codes-licence.md, section 6) : mise en forme à chaque frappe et
//! message à montrer. Partagé par l'application et par KadenzAdmin, qui
//! l'inclut par `#[path]` comme traduction.rs ; chacun branche
//! `CodeCourtUi.formater` sur [`formater`].

use kadenz_licence::code_court::{self, LONGUEUR, MODELES_ERREUR, Signal};

use crate::CodeSaisi;
use crate::traduction::tr;

/// Texte affiché avant la modification (déjà mis en forme) et texte brut après :
/// texte remis en forme, curseur, message et validité du code.
pub(crate) fn formater(precedent: &str, nouveau: &str) -> CodeSaisi {
    let mise = code_court::mettre_en_forme(precedent, nouveau);
    let controle = code_court::verifier(&mise.texte);
    let complet = code_court::normaliser(&mise.texte).chars().count() == LONGUEUR;
    let message = match (mise.signal, &controle) {
        (Signal::Ambigu, _) => phrase(&tr(MODELES_ERREUR[2])),
        (Signal::Autre, _) => phrase(&tr(
            "caractère retiré : un code n'a que des lettres de A à Z et des chiffres de 2 à 9",
        )),
        // Rien pendant la saisie : le message ne vient qu'une fois les 25 caractères tapés.
        (Signal::Aucun, Err(e)) if complet => phrase(&tr(&e.to_string())),
        _ => String::new(),
    };
    CodeSaisi {
        texte: mise.texte.into(),
        curseur: i32::try_from(mise.curseur).unwrap_or(0),
        message: message.into(),
        valide: controle.is_ok(),
    }
}

/// Première lettre en majuscule et point final, comme les autres messages.
fn phrase(texte: &str) -> String {
    let mut caracteres = texte.chars();
    let mut s: String = match caracteres.next() {
        Some(c) => c.to_uppercase().chain(caracteres).collect(),
        None => return String::new(),
    };
    if !s.ends_with(['.', '。', '!', '?', '؟']) {
        s.push('.');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Code complet et juste, tiré par le module des codes.
    fn code_juste() -> String {
        code_court::tirer().expect("tirage d'un code")
    }

    #[test]
    fn textes_du_champ_traduits() {
        let manques = crate::traduction::sans_traduction(MODELES_ERREUR);
        assert!(manques.is_empty(), "{}", manques.join("\n"));
    }

    #[test]
    fn frappe_et_collage_mis_en_forme() {
        crate::traduction::choisir("fr");
        let code = code_juste();
        // Collage en minuscules, sans tirets, avec espaces et retour à la ligne.
        let colle = format!(" {}\n", code.replace('-', " ").to_lowercase());
        let r = formater("", &colle);
        assert_eq!(r.texte.as_str(), code);
        assert!(r.valide);
        assert_eq!(r.message.as_str(), "");
        assert_eq!(r.curseur as usize, code.len());
        // Frappe d'un caractère au début d'un champ vide : pas de message.
        let r = formater("", "a");
        assert_eq!((r.texte.as_str(), r.curseur, r.valide), ("A", 1, false));
        assert_eq!(r.message.as_str(), "");
    }

    #[test]
    fn messages_du_champ() {
        crate::traduction::choisir("fr");
        // O, 0, I ou 1 : retiré et signalé.
        let r = formater("", "AB0");
        assert_eq!(r.texte.as_str(), "AB");
        assert!(r.message.contains('O'), "{}", r.message);
        // Autre caractère : retiré et signalé.
        let r = formater("", "AB#");
        assert_eq!(r.texte.as_str(), "AB");
        assert!(r.message.starts_with("Caractère retiré"), "{}", r.message);
        // Code complet au contrôle faux : message ; code juste : aucun.
        let code = code_juste();
        let dernier = code.chars().last().unwrap();
        let faux_signe = code_court::ALPHABET
            .chars()
            .find(|c| *c != dernier)
            .unwrap();
        let faux = format!("{}{faux_signe}", &code[..code.len() - 1]);
        let r = formater("", &faux);
        assert!(!r.valide);
        assert!(r.message.starts_with("Code erroné"), "{}", r.message);
    }
}
