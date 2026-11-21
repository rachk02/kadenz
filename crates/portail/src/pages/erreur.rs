//! Page d'erreur (accès refusé, introuvable, jeton invalide...) : même
//! page, au thème piliers, avec ou sans compte.

use maud::Markup;
use maud::html;

use super::connexion::cadre_large;
use super::{Cadre, message_du_cadre, tete};

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
    cadre_large(
        tete(intitule),
        html! {
            h1 { (intitule) }
            p.texte { (texte) }
            @if let Some(c) = c {
                (message_du_cadre(c))
                a.bouton-sauge href="/" { "Retour aux entreprises" }
            } @else {
                a.bouton-sauge href="/connexion" { "Se connecter" }
            }
        },
    )
}
