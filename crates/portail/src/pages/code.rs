//! Codes d'activation (décision 74) : le code tout juste tiré, seul moment où
//! il s'affiche en clair (réponse au POST /paiements/{id}/licence), puis sa
//! fiche sans le code (GET /codes/{id}).

use maud::{Markup, html};

use super::{Cadre, Rubrique, chemin_entreprise, heure, libelle_formule, liste_modules, page, titre};
use crate::{CodeActivation, CodeCree};

fn retour(code: &CodeActivation) -> Markup {
    html! {
        @if let Some(e) = &code.entreprise {
            a.bouton href=(chemin_entreprise(e)) { "Retour à l'entreprise" }
        } @else {
            a.bouton href="/" { "Retour aux entreprises" }
        }
    }
}

fn grille(code: &CodeActivation) -> Markup {
    html! {
        dl.grille-fiche {
            dt { "Code" }
            dd { span.identifiant.identifiant-court { "…" (code.fin) } }
            dt { "État" }
            dd { (code.etat.libelle()) }
            @if let Some(e) = &code.entreprise {
                dt { "Identifiant de l'entreprise" }
                dd { span.identifiant { (e) } }
            }
            dt { "Postes" }
            dd { (code.postes) }
            dt { "Formule" }
            dd { (libelle_formule(code.formule)) }
            dt { "Modules" }
            dd { (liste_modules(&code.modules)) }
            dt { "Tiré le" }
            dd { (heure(&code.cree_le)) " par " (code.cree_par) }
            @if let Some(a) = &code.active_le {
                dt { "Activé le" }
                dd { (heure(a)) }
            }
            @if let Some(l) = code.licence {
                dt { "Licence" }
                dd { a href={ "/licences/" (l) } { "Voir la licence émise" } }
            }
        }
    }
}

pub fn code_cree(c: &Cadre, cree: &CodeCree) -> Markup {
    let contenu = html! {
        (titre("Code d'activation", retour(&cree.fiche)))
        div.carte {
            div.carte-interieur {
                div.champ {
                    label for="code" { "Code à remettre au client" }
                    input #code .code-activation type="text" readonly value=(cree.code) spellcheck="false";
                    p.aide {
                        "Notez ou copiez ce code maintenant : il ne s'affichera plus. Le client le saisit dans Kadenz (assistant de KadenzAdmin, ou Paramètres, rubrique « Abonnement », sur un poste Admin) ; la licence est émise à ce moment-là, avec une échéance calculée le jour de l'activation."
                    }
                }
                (grille(&cree.fiche))
            }
        }
    };
    page(c, "Code d'activation", Rubrique::Entreprises, contenu)
}

pub fn code(c: &Cadre, code: &CodeActivation) -> Markup {
    let contenu = html! {
        (titre("Code d'activation", retour(code)))
        div.carte {
            div.carte-interieur {
                (grille(code))
                p.aide { "Le code en clair n'est pas gardé par le portail : seule sa fin reste visible." }
            }
        }
    };
    page(c, "Code d'activation", Rubrique::Entreprises, contenu)
}

