//! Comptes (GET /comptes, éditeur seulement) : liste, désactivation et
//! réactivation (POST /comptes/{id}/actif), création d'un revendeur
//! (POST /comptes). Un compte éditeur ne se crée qu'en ligne de commande.

use maud::{Markup, html};

use super::{Cadre, Rubrique, erreur_formulaire, jeton, libelle_role, page, titre};
use crate::{Compte, Role};

pub fn comptes(c: &Cadre, comptes: &[Compte], erreur: Option<&str>) -> Markup {
    let contenu = html! {
        (titre("Revendeurs", html! {}))
        p.texte {
            "Un revendeur ne voit que les entreprises qui lui sont rattachées. Son nom est repris comme émetteur dans les licences qu'il émet. Un compte désactivé ne peut plus se connecter et ses sessions sont fermées."
        }
        div.carte {
            @if comptes.is_empty() {
                p.vide { "Aucun compte." }
            } @else {
                div.tableau-cadre {
                    table {
                        thead {
                            tr {
                                th { "Nom" }
                                th { "Identifiant" }
                                th { "Rôle" }
                                th { "État" }
                                th {}
                            }
                        }
                        tbody {
                            @for compte in comptes {
                                tr {
                                    td { (compte.nom) }
                                    td { span.identifiant { (compte.identifiant) } }
                                    td.court { (libelle_role(compte.role)) }
                                    td.court {
                                        @if compte.actif {
                                            span.etat.etat-ok { "Actif" }
                                        } @else {
                                            span.etat.etat-neutre { "Désactivé" }
                                        }
                                    }
                                    td.court {
                                        @if compte.role == Role::Revendeur && compte.id != c.compte.id {
                                            form.en-ligne method="post" action={ "/comptes/" (compte.id) "/actif" } {
                                                (jeton(c))
                                                @if compte.actif {
                                                    input type="hidden" name="actif" value="0";
                                                    button.bouton.bouton-petit type="submit" { "Désactiver" }
                                                } @else {
                                                    input type="hidden" name="actif" value="1";
                                                    button.bouton.bouton-petit type="submit" { "Réactiver" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        h2 { "Nouveau revendeur" }
        div.carte {
            div.carte-interieur {
                form.formulaire method="post" action="/comptes" {
                    (jeton(c))
                    (erreur_formulaire(erreur))
                    div.rangee {
                        div.champ {
                            label for="identifiant" { "Identifiant de connexion" }
                            input #identifiant type="text" name="identifiant" required
                                autocomplete="off" spellcheck="false";
                        }
                        div.champ {
                            label for="nom" { "Nom du revendeur" }
                            input #nom type="text" name="nom" required autocomplete="organization";
                        }
                    }
                    div.champ {
                        label for="mot_de_passe" { "Mot de passe provisoire" }
                        input #mot_de_passe type="password" name="mot_de_passe" required
                            minlength="10" autocomplete="new-password";
                        p.aide { "10 caractères au moins. Transmettez-le au revendeur par un canal sûr ; il pourra le changer après sa connexion." }
                    }
                    div.actions {
                        button.bouton.bouton-principal type="submit" { "Créer le revendeur" }
                    }
                }
            }
        }
    };
    page(c, "Revendeurs", Rubrique::Comptes, contenu)
}
