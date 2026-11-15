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
    // Un texte qui ne fait que répéter l'intitulé laisse place à une phrase
    // utile ; pour un revendeur, l'entreprise d'un autre répond aussi 404.
    let texte = if texte.to_lowercase() == intitule.to_lowercase() {
        match statut {
            403 => "Votre compte n'a pas accès à cette page.",
            404 => "Cette page n'existe pas, ou elle ne relève pas de votre compte.",
            _ => texte,
        }
    } else {
        texte
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
