//! Activation d'un code de licence court auprès du portail, ligne V1
//! (décisions 74 et 75, docs/spec-codes-licence.md, sections 5 et 7 bis), pour
//! `ActivationUi` de licence.slint : assistant et page Licence.
//!
//! [`activer`] est bloquant et ne traduit rien (la langue est propre au fil de
//! l'interface) : l'appelant le lance dans un fil à part, puis traduit le refus
//! avec [`message`] une fois revenu dans la boucle de l'interface.

use std::fmt::Display;

use kadenz_domaine::message::MessageCompose;
use kadenz_licence::Licence;

use crate::traduction::trf;

/// Envoie le code au portail et vérifie la licence reçue comme une clé
/// collée : clés embarquées selon la version (une licence datée seulement
/// avec la clé du portail) et identifiant de ce poste. Une réponse d'un faux
/// portail n'est donc jamais retenue.
pub(crate) fn activer(
    code: &str,
    poste: &str,
    client: &str,
) -> Result<(String, Licence), MessageCompose> {
    let reponse = kadenz_maj::activation::activer(code, poste, client)?;
    let lue = kadenz_licence::verifier(&reponse.licence, poste)?;
    Ok((reponse.licence, lue))
}

/// Refus traduit, en phrase : première lettre en majuscule, point final.
pub(crate) fn message(m: &MessageCompose) -> String {
    let valeurs: Vec<&dyn Display> = m.valeurs.iter().map(|v| v as &dyn Display).collect();
    let texte = trf(m.modele, &valeurs);
    let mut caracteres = texte.chars();
    let mut phrase: String = match caracteres.next() {
        Some(c) => c.to_uppercase().chain(caracteres).collect(),
        None => return String::new(),
    };
    if !phrase.ends_with('.') {
        phrase.push('.');
    }
    phrase
}

#[cfg(test)]
mod tests {
    use kadenz_licence::code_court::MODELES_ACTIVATION;
    use kadenz_maj::activation::MODELES_ERREUR;

    #[test]
    fn refus_d_activation_traduits() {
        let mut modeles: Vec<&str> = MODELES_ACTIVATION.to_vec();
        modeles.extend_from_slice(MODELES_ERREUR);
        let manques = crate::traduction::sans_traduction(&modeles);
        assert!(manques.is_empty(), "{manques:#?}");
    }
}
