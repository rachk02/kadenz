//! Codes d'activation (décision 74) : le code tout juste tiré, seul moment où
//! il s'affiche en clair (réponse au POST /paiements/{id}/licence ou au POST
//! /codes), sa fiche sans le code (GET /codes/{id}, avec l'annulation d'un
//! code libre) et les codes à vendre (GET /codes).

use maud::{Markup, html};

use super::{
    Cadre, Rubrique, chemin_entreprise, erreur_formulaire, heure, jeton, libelle_formule,
    liste_modules, page, titre,
};
use crate::{CODES_PAR_TIRAGE, CodeActivation, CodeCree, EtatCode, Gamme};
use kadenz_domaine::module::Module;

fn retour(code: &CodeActivation) -> Markup {
    html! {
        @if let Some(e) = &code.entreprise {
            a.bouton href=(chemin_entreprise(e)) { "Retour à l'entreprise" }
        } @else {
            a.bouton href="/codes" { "Retour aux codes à vendre" }
        }
    }
}

fn grille(code: &CodeActivation) -> Markup {
    html! {
        dl.grille-fiche {
            dt { "Code" }
            dd { span.identifiant.identifiant-court { "…" (code.fin) } }
            dt { "Gamme" }
            dd { (code.gamme.libelle()) }
            dt { "État" }
            dd { (code.etat.libelle()) }
            @if code.gamme == Gamme::V1 {
                dt { "Poste lié" }
                dd {
                    @match &code.poste {
                        Some(p) => span.identifiant { (p) },
                        None => "aucun",
                    }
                }
            }
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
    let rubrique = if code.paiement.is_some() {
        Rubrique::Entreprises
    } else {
        Rubrique::Codes
    };
    let contenu = html! {
        (titre("Code d'activation", retour(code)))
        div.carte {
            div.carte-interieur {
                (grille(code))
                p.aide { "Le code en clair n'est pas gardé par le portail : seule sa fin reste visible." }
                @if code.gamme == Gamme::V1 && code.poste.is_some() {
                    form.formulaire method="post" action={ "/codes/" (code.id) "/liberer" } {
                        (jeton(c))
                        p.aide {
                            "Changement d'ordinateur du client : libérer le poste rend le code activable sur un autre poste, avec la même échéance. L'ancien poste garde sa licence jusqu'à l'échéance."
                        }
                        div.actions {
                            button.bouton type="submit" { "Libérer le poste" }
                        }
                    }
                }
                @if code.etat == EtatCode::Libre {
                    form.formulaire method="post" action={ "/codes/" (code.id) "/annuler" } {
                        (jeton(c))
                        p.aide {
                            "Un code annulé ne peut plus être activé"
                            @if code.paiement.is_some() { " ; son paiement pourra servir à un nouveau code" }
                            "."
                        }
                        div.actions {
                            button.bouton type="submit" { "Annuler ce code" }
                        }
                    }
                }
            }
        }
    };
    page(c, "Code d'activation", rubrique, contenu)
}

/// Codes à vendre : formulaire de tirage et liste.
pub fn codes(c: &Cadre, liste: &[CodeActivation], erreur: Option<&str>) -> Markup {
    let contenu = html! {
        (titre("Codes à vendre", html! {}))
        div.carte {
            div.carte-interieur {
                form.formulaire method="post" action="/codes" {
                    (jeton(c))
                    (erreur_formulaire(erreur))
                    p.aide {
                        "Des codes sans paiement ni entreprise, à remettre ou à vendre : l'entreprise se lie au code quand le client l'active, et la licence est émise ce jour-là."
                    }
                    div.rangee {
                        div.champ {
                            label for="gamme" { "Gamme" }
                            select #gamme name="gamme" required {
                                option value=(Gamme::Entreprise.code()) selected { (Gamme::Entreprise.libelle()) }
                                option value=(Gamme::V1.code()) { (Gamme::V1.libelle()) }
                            }
                            p.aide { "Un code de la ligne V1 vaut un poste." }
                        }
                        div.champ {
                            label for="nombre" { "Nombre de codes" }
                            input #nombre type="number" name="nombre" min="1" max=(CODES_PAR_TIRAGE)
                                step="1" value="1" inputmode="numeric" required;
                        }
                        div.champ {
                            label for="postes" { "Postes par code" }
                            input #postes type="number" name="postes" min="1" max="1000" step="1"
                                inputmode="numeric" required;
                        }
                        div.champ {
                            label for="formule" { "Formule" }
                            select #formule name="formule" required {
                                option value="mensuelle" { "Mensuelle" }
                                option value="annuelle" selected { "Annuelle" }
                            }
                        }
                    }
                    fieldset {
                        legend.legende { "Modules" }
                        div.cases {
                            @for m in Module::TOUS {
                                label.case {
                                    input type="checkbox" name={ "module_" (m.code()) } value="1" checked;
                                    (m.libelle())
                                }
                            }
                        }
                    }
                    div.actions {
                        button.bouton.bouton-principal type="submit" { "Tirer les codes" }
                    }
                }
            }
        }
        h2 { "Codes tirés" }
        div.carte {
            @if liste.is_empty() {
                p.vide { "Aucun code à vendre pour l'instant." }
            } @else {
                div.tableau-cadre {
                    table {
                        thead {
                            tr {
                                th { "Code" }
                                th { "Gamme" }
                                th { "État" }
                                th.nombre { "Postes" }
                                th { "Formule" }
                                th { "Tiré le" }
                                th { "Par" }
                                th { "Entreprise" }
                            }
                        }
                        tbody {
                            @for k in liste {
                                tr {
                                    td.court { a href={ "/codes/" (k.id) } { "…" (k.fin) } }
                                    td.court { (k.gamme.code()) }
                                    td.court { (k.etat.libelle()) }
                                    td.nombre { (k.postes) }
                                    td.court { (libelle_formule(k.formule)) }
                                    td.court { (heure(&k.cree_le)) }
                                    td { (k.cree_par) }
                                    td.court {
                                        @match &k.entreprise {
                                            Some(e) => a href=(chemin_entreprise(e)) { "Voir l'entreprise" },
                                            None => "-",
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    };
    page(c, "Codes à vendre", Rubrique::Codes, contenu)
}

/// Codes à vendre tout juste tirés : seul moment où ils s'affichent en clair.
pub fn codes_crees(c: &Cadre, codes: &[CodeCree]) -> Markup {
    let premier = codes.first().map(|k| &k.fiche);
    let contenu = html! {
        (titre("Codes tirés", html! { a.bouton href="/codes" { "Retour aux codes à vendre" } }))
        div.carte {
            div.carte-interieur {
                @if let Some(f) = premier {
                    p.aide {
                        (codes.len()) " code" @if codes.len() > 1 { "s" } " de " (f.postes) " poste"
                        @if f.postes > 1 { "s" } ", formule " (libelle_formule(f.formule).to_lowercase())
                        ", modules : " (liste_modules(&f.modules)) "."
                    }
                }
                ul.codes-tires {
                    @for k in codes {
                        li { span.code-activation { (k.code) } }
                    }
                }
                p.aide {
                    "Notez ou copiez ces codes maintenant : ils ne s'afficheront plus. Chaque code s'active une fois, pour une entreprise, dans Kadenz (assistant de KadenzAdmin, ou Paramètres, rubrique « Abonnement », sur un poste Admin)."
                }
            }
        }
    };
    page(c, "Codes tirés", Rubrique::Codes, contenu)
}
