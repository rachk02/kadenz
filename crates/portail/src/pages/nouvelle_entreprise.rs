//! Nouvelle entreprise (GET et POST /entreprises/nouvelle) : identifiant
//! d'entreprise lu dans Kadenz, nom du client et, pour l'éditeur, revendeur
//! rattaché. Un revendeur crée des entreprises rattachées à lui (liste des
//! revendeurs vide).

use maud::{Markup, html};

use super::{Cadre, Rubrique, erreur_formulaire, jeton, page_formulaire, titre};
use crate::Compte;

pub fn nouvelle_entreprise(c: &Cadre, revendeurs: &[Compte], erreur: Option<&str>) -> Markup {
    let contenu = html! {
        (titre("Nouvelle entreprise", html! {
            a.bouton href="/" { "Retour aux entreprises" }
        }))
        div.carte data-reveal="fade" {
            div.carte-interieur {
                form.formulaire method="post" action="/entreprises/nouvelle" {
                    (jeton(c))
                    (erreur_formulaire(erreur))
                    div.champ {
                        label for="identifiant" { "Identifiant de l'entreprise" }
                        input #identifiant type="text" name="identifiant" required
                            autocomplete="off" spellcheck="false";
                        p.aide {
                            "Tel qu'il apparaît dans Kadenz sous « Identifiant de l'entreprise » : rubrique « Serveur » des Paramètres d'un poste relié, ou assistant d'installation du serveur."
                        }
                    }
                    div.champ {
                        label for="client" { "Client" }
                        input #client type="text" name="client" required autocomplete="organization";
                        p.aide { "Nom du client, repris dans ses licences." }
                    }
                    @if !revendeurs.is_empty() {
                        div.champ {
                            label for="revendeur" { "Revendeur" }
                            select #revendeur name="revendeur" {
                                option value="" selected { "Aucun : client direct de l'éditeur" }
                                @for r in revendeurs {
                                    option value=(r.id) { (r.nom) " (" (r.identifiant) ")" }
                                }
                            }
                            p.aide { "Le revendeur rattaché voit l'entreprise et peut émettre ses licences." }
                        }
                    }
                    div.actions {
                        button.bouton.bouton-principal type="submit" { "Créer l'entreprise" }
                        a.bouton href="/" { "Annuler" }
                    }
                }
            }
        }
    };
    page_formulaire(c, "Nouvelle entreprise", Rubrique::Entreprises, contenu)
}
