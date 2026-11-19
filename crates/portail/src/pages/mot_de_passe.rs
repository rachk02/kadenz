//! Changement du mot de passe du compte connecté (GET et POST /mot-de-passe).

use maud::{Markup, html};

use super::{Cadre, Rubrique, erreur_formulaire, jeton, page, titre};
use crate::LONGUEUR_MIN_MOT_DE_PASSE;

pub fn mot_de_passe(c: &Cadre, erreur: Option<&str>) -> Markup {
    let contenu = html! {
        (titre("Mot de passe", html! {}))
        p.texte { "Compte " strong { (c.compte.identifiant) } ". Le nouveau mot de passe compte " (LONGUEUR_MIN_MOT_DE_PASSE) " caractères au moins, et ne doit pas être un mot de passe courant. Les autres sessions de ce compte seront fermées." }
        div.carte {
            div.carte-interieur {
                form.formulaire method="post" action="/mot-de-passe" {
                    (jeton(c))
                    (erreur_formulaire(erreur))
                    div.champ {
                        label for="ancien" { "Mot de passe actuel" }
                        input #ancien type="password" name="ancien" required autocomplete="current-password";
                    }
                    div.champ {
                        label for="nouveau" { "Nouveau mot de passe" }
                        input #nouveau type="password" name="nouveau" required minlength=(LONGUEUR_MIN_MOT_DE_PASSE)
                            autocomplete="new-password";
                    }
                    div.champ {
                        label for="confirmation" { "Confirmation du nouveau mot de passe" }
                        input #confirmation type="password" name="confirmation" required minlength=(LONGUEUR_MIN_MOT_DE_PASSE)
                            autocomplete="new-password";
                    }
                    div.actions {
                        button.bouton.bouton-principal type="submit" { "Changer le mot de passe" }
                    }
                }
            }
        }
    };
    page(c, "Mot de passe", Rubrique::MotDePasse, contenu)
}
