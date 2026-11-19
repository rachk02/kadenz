//! Double authentification des éditeurs (GET et POST
//! /double-authentification) : mise en place avec la clé à saisir dans
//! l'application, codes de secours montrés une seule fois, état une fois
//! active.

use maud::{Markup, html};

use super::{Cadre, Rubrique, erreur_formulaire, jeton, page, titre};
use crate::{Role, totp};

/// Mise en place : clé en groupes Base32, puis premier code.
pub fn mise_en_place(c: &Cadre, secret: &str, erreur: Option<&str>) -> Markup {
    let contenu = html! {
        (titre("Double authentification", html! {}))
        p.texte {
            "Un compte éditeur doit activer la double authentification avant tout autre usage du portail. À chaque connexion, après le mot de passe, il saisit un code à 6 chiffres donné par une application d'authentification, sur un téléphone par exemple."
        }
        div.carte {
            div.carte-interieur {
                form.formulaire method="post" action="/double-authentification" {
                    (jeton(c))
                    (erreur_formulaire(erreur))
                    div.champ {
                        label for="cle" { "Clé à saisir dans l'application" }
                        input #cle .code-activation type="text" readonly value=(totp::groupes(secret)) spellcheck="false";
                        p.aide {
                            "Dans l'application, ajoutez un compte par saisie manuelle de la clé : type basé sur l'heure, 6 chiffres, toutes les 30 secondes. Gardez cette clé pour vous : qui la connaît peut produire vos codes."
                        }
                    }
                    div.champ {
                        label for="code" { "Code affiché par l'application" }
                        input #code type="text" name="code" required autocomplete="one-time-code"
                            inputmode="numeric" spellcheck="false";
                    }
                    div.actions {
                        button.bouton.bouton-principal type="submit" { "Activer la double authentification" }
                    }
                }
            }
        }
    };
    page(c, "Double authentification", Rubrique::MotDePasse, contenu)
}

/// Codes de secours tout juste créés : seul moment où ils s'affichent.
pub fn codes_secours(c: &Cadre, codes: &[String]) -> Markup {
    let contenu = html! {
        (titre("Double authentification activée", html! { a.bouton href="/" { "Continuer" } }))
        div.carte {
            div.carte-interieur {
                p.aide {
                    "Voici vos " (codes.len()) " codes de secours. Chacun remplace une fois le code de l'application, si le téléphone est perdu. Notez-les maintenant et gardez-les à part : ils ne s'afficheront plus."
                }
                ul.codes-tires {
                    @for k in codes {
                        li { span.code-activation { (k) } }
                    }
                }
            }
        }
    };
    page(c, "Double authentification", Rubrique::MotDePasse, contenu)
}

/// Double authentification déjà active (éditeur), ou pas encore proposée
/// (revendeur).
pub fn double_active(c: &Cadre) -> Markup {
    let contenu = html! {
        (titre("Double authentification", html! {}))
        div.carte {
            div.carte-interieur {
                @if c.compte.role == Role::Editeur {
                    p.texte {
                        "La double authentification est active pour ce compte. Téléphone perdu sans code de secours : elle se retire sur le serveur du portail, par le programme (« retirer-double-authentification »), puis se remet en place à la connexion suivante."
                    }
                } @else {
                    p.texte { "La double authentification est réservée pour l'instant aux comptes éditeur." }
                }
            }
        }
    };
    page(c, "Double authentification", Rubrique::MotDePasse, contenu)
}
