//! Changement du mot de passe du compte connecté (GET et POST /mot-de-passe),
//! au thème piliers (sans le bandeau).

use maud::{Markup, PreEscaped, html};

use super::connexion::{ICONE_MOT_DE_PASSE, cadre_large};
use super::{Cadre, erreur_formulaire, jeton, message_du_cadre, pied_compte, tete};
use crate::LONGUEUR_MIN_MOT_DE_PASSE;

pub fn mot_de_passe(c: &Cadre, erreur: Option<&str>) -> Markup {
    cadre_large(
        tete("Mot de passe"),
        html! {
            h1 { "Mot de passe" }
            p.texte { "Compte " strong { (c.compte.identifiant) } ". Le nouveau mot de passe compte " (LONGUEUR_MIN_MOT_DE_PASSE) " caractères au moins, et ne doit pas être un mot de passe courant. Les autres sessions de ce compte seront fermées." }
            (message_du_cadre(c))
            (erreur_formulaire(erreur))
            form method="post" action="/mot-de-passe" {
                (jeton(c))
                div.champ {
                    label for="ancien" { "Mot de passe actuel" }
                    div.saisie {
                        (PreEscaped(ICONE_MOT_DE_PASSE))
                        input #ancien type="password" name="ancien" required autocomplete="current-password";
                    }
                }
                div.champ {
                    label for="nouveau" { "Nouveau mot de passe" }
                    div.saisie {
                        (PreEscaped(ICONE_MOT_DE_PASSE))
                        input #nouveau type="password" name="nouveau" required minlength=(LONGUEUR_MIN_MOT_DE_PASSE)
                            autocomplete="new-password";
                    }
                }
                div.champ {
                    label for="confirmation" { "Confirmation du nouveau mot de passe" }
                    div.saisie {
                        (PreEscaped(ICONE_MOT_DE_PASSE))
                        input #confirmation type="password" name="confirmation" required minlength=(LONGUEUR_MIN_MOT_DE_PASSE)
                            autocomplete="new-password";
                    }
                }
                button.bouton-sauge type="submit" { "Changer le mot de passe" }
            }
            (pied_compte(c, true))
        },
    )
}
