//! Page d'erreur (accès refusé, introuvable, jeton invalide...) : dans le
//! cadre du compte s'il est connecté, seule sinon.

use maud::{DOCTYPE, Markup, html};

use super::{Cadre, Rubrique, page, tete, titre};

pub fn erreur(c: Option<&Cadre>, statut: u16, texte: &str) -> Markup {
    let intitule = match statut {
        403 => "Accès refusé",
        404 => "Page introuvable",
        _ => "Erreur",
    };
    let corps = html! {
        (titre(intitule, html! {}))
        p.texte { (texte) }
        div.actions {
            @if c.is_some() {
                a.bouton.bouton-principal href="/" { "Retour aux entreprises" }
            } @else {
                a.bouton.bouton-principal href="/connexion" { "Se connecter" }
            }
        }
    };
    match c {
        Some(c) => page(c, intitule, Rubrique::Aucune, corps),
        None => html! {
            (DOCTYPE)
            html lang="fr" {
                (tete(intitule))
                body {
                    main.page-simple { (corps) }
                }
            }
        },
    }
}
