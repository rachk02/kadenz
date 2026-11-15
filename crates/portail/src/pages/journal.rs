//! Journal des actions (GET /journal), en ajout seul : heure UTC, compte,
//! action, cible et détails. Le crate filtre ce qu'un revendeur peut voir.

use maud::{Markup, html};

use super::{Cadre, Rubrique, heure, page, titre};
use crate::{Action, Role};

pub fn journal(c: &Cadre, actions: &[Action]) -> Markup {
    let contenu = html! {
        (titre("Journal", html! {}))
        p.texte {
            @if c.compte.role == Role::Editeur {
                "Chaque action du portail, de la plus récente à la plus ancienne (500 au plus). Les heures sont en temps universel (UTC)."
            } @else {
                "Vos actions sur le portail, de la plus récente à la plus ancienne (500 au plus). Les heures sont en temps universel (UTC)."
            }
        }
        div.carte {
            @if actions.is_empty() {
                p.vide { "Aucune action enregistrée." }
            } @else {
                div.tableau-cadre {
                    table {
                        thead {
                            tr {
                                th { "Heure (UTC)" }
                                th { "Compte" }
                                th { "Action" }
                                th { "Cible" }
                                th { "Détails" }
                            }
                        }
                        tbody {
                            @for a in actions {
                                tr {
                                    td.court { (heure(&a.heure)) }
                                    td { (a.compte.as_deref().unwrap_or("-")) }
                                    td.court { (libelle_action(&a.action)) }
                                    td.court {
                                        @match a.cible.as_deref() {
                                            Some(cible) => span.identifiant.identifiant-court title=(cible) { (court(cible)) },
                                            None => "-",
                                        }
                                    }
                                    td.details { (a.details) }
                                }
                            }
                        }
                    }
                }
            }
        }
    };
    page(c, "Journal", Rubrique::Journal, contenu)
}

/// Cible abrégée (un identifiant d'entreprise est un UUID) : les huit
/// premiers caractères, le texte complet restant dans l'infobulle.
fn court(cible: &str) -> String {
    if cible.chars().count() > 12 {
        format!("{}...", cible.chars().take(8).collect::<String>())
    } else {
        cible.to_string()
    }
}

/// Libellé d'un code d'action ; un code inconnu s'affiche tel quel.
fn libelle_action(code: &str) -> &str {
    match code {
        "editeur_cree" => "Compte éditeur créé",
        "compte_cree" => "Revendeur créé",
        "compte_active" => "Compte réactivé",
        "compte_desactive" => "Compte désactivé",
        "mot_de_passe_change" => "Mot de passe changé",
        "connexion" => "Connexion",
        "connexion_refusee" => "Connexion refusée",
        "deconnexion" => "Déconnexion",
        "entreprise_creee" => "Entreprise créée",
        "paiement_enregistre" => "Paiement enregistré",
        "licence_emise" => "Licence émise",
        _ => code,
    }
}
