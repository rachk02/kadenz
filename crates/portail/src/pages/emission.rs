//! Émission d'une licence à partir d'un paiement non utilisé (GET et POST
//! /paiements/{id}/licence), spec. portail section 5 : formule et postes
//! repris du paiement, modules et échéance proposés par le crate et
//! modifiables.

use maud::{Markup, html};

use super::{
    Cadre, Rubrique, chemin_entreprise, date, erreur_formulaire, jeton, libelle_formule, montant,
    page, titre,
};
use crate::{Entreprise, Paiement, Proposition};
use kadenz_domaine::module::Module;

pub fn emission(
    c: &Cadre,
    e: &Entreprise,
    p: &Paiement,
    prop: &Proposition,
    erreur: Option<&str>,
) -> Markup {
    let chemin = chemin_entreprise(&e.identifiant);
    let contenu = html! {
        (titre("Émettre la licence", html! {
            a.bouton href=(chemin) { "Retour à l'entreprise" }
        }))
        div.carte {
            div.carte-interieur {
                dl.grille-fiche {
                    dt { "Client" }
                    dd { (e.client) }
                    dt { "Identifiant de l'entreprise" }
                    dd { span.identifiant { (e.identifiant) } }
                    dt { "Paiement" }
                    dd {
                        (montant(p.montant, &p.devise)) ", " (p.mode.libelle()) ", reçu le " (date(&p.recu_le))
                        @if let Some(r) = &p.reference { " (" (r) ")" }
                    }
                    dt { "Postes" }
                    dd { (p.postes) }
                    dt { "Formule" }
                    dd { (libelle_formule(p.formule)) }
                }
            }
        }
        h2 { "Licence" }
        div.carte {
            div.carte-interieur {
                form.formulaire method="post" action={ "/paiements/" (p.id) "/licence" } {
                    (jeton(c))
                    (erreur_formulaire(erreur))
                    div.champ {
                        label for="echeance" { "Échéance" }
                        input #echeance type="date" name="echeance" value=(prop.echeance) required;
                        p.aide {
                            "Dernier jour couvert, sans délai de grâce : le lendemain, Kadenz passe en lecture seule. Proposée d'après la dernière licence et la formule, modifiable."
                        }
                    }
                    fieldset {
                        legend.legende { "Modules" }
                        div.cases {
                            @for m in Module::TOUS {
                                label.case {
                                    input type="checkbox" name={ "module_" (m.code()) } value="1"
                                        checked[prop.modules.contains(&m)];
                                    (m.libelle())
                                }
                            }
                        }
                        p.aide { "Repris de la dernière licence de l'entreprise ; tous les modules pour une première licence." }
                    }
                    p.aide {
                        "La licence est signée par la clé du portail et vérifiée avant d'être enregistrée. Son texte s'affiche ensuite, à coller dans la rubrique « Abonnement » d'un poste Admin."
                    }
                    div.actions {
                        button.bouton.bouton-principal type="submit" { "Émettre la licence" }
                        a.bouton href=(chemin) { "Annuler" }
                    }
                }
            }
        }
    };
    page(c, "Émettre la licence", Rubrique::Entreprises, contenu)
}
