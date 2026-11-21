//! Code d'activation d'un paiement non utilisé (GET et POST
//! /paiements/{id}/licence ; décision 74) : formule et postes repris du
//! paiement, modules proposés et modifiables ; l'échéance est calculée à
//! l'activation.

use maud::{Markup, html};

use super::{
    Cadre, Rubrique, chemin_entreprise, date, erreur_formulaire, jeton, libelle_formule, montant,
    page_formulaire, titre,
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
        (titre("Tirer le code d'activation", html! {
            a.bouton href=(chemin) { "Retour à l'entreprise" }
        }))
        div.carte data-reveal="fade" {
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
        h2 { "Code d'activation" }
        div.carte data-reveal="fade" {
            div.carte-interieur {
                form.formulaire method="post" action={ "/paiements/" (p.id) "/licence" } {
                    (jeton(c))
                    (erreur_formulaire(erreur))
                    p.aide {
                        "Échéance si le code est activé aujourd'hui : " (date(&prop.echeance)) ". Elle est calculée le jour de l'activation : l'échéance de la dernière licence si elle court encore, sinon la veille, plus un mois ou un an selon la formule. Dernier jour couvert, sans délai de grâce."
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
                        "Le code s'affiche une seule fois, à remettre au client. Quand il le saisit dans Kadenz, le portail émet la licence, signée par sa clé, et la renvoie au programme."
                    }
                    div.actions {
                        button.bouton.bouton-principal type="submit" { "Tirer le code" }
                        a.bouton href=(chemin) { "Annuler" }
                    }
                }
            }
        }
    };
    page_formulaire(
        c,
        "Tirer le code d'activation",
        Rubrique::Entreprises,
        contenu,
    )
}
