//! Codes d'activation (décision 74) : le code tout juste tiré, seul moment où
//! il s'affiche en clair (réponse au POST /paiements/{id}/licence ou au POST
//! /codes), sa fiche sans le code (GET /codes/{id}, avec l'annulation d'un
//! code libre) et les codes à vendre (GET /codes).

use maud::{Markup, PreEscaped, html};

use super::{
    Cadre, Rubrique, SCRIPT_CODES_TIRES, bloc_formulaire, chemin_entreprise, erreur_formulaire,
    heure, jeton, libelle_formule, liste_modules, page, page_formulaire, titre, vide,
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
        div.carte data-reveal="fade" {
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
    page_formulaire(c, "Code d'activation", Rubrique::Entreprises, contenu)
}

pub fn code(c: &Cadre, code: &CodeActivation) -> Markup {
    let rubrique = if code.paiement.is_some() {
        Rubrique::Entreprises
    } else {
        Rubrique::Codes
    };
    let contenu = html! {
        (titre("Code d'activation", retour(code)))
        div.carte data-reveal="fade" {
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
        (bloc_formulaire("amphore", html! {
        div.carte data-reveal="fade" {
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
                                option value="monoposte" { "Kadenz, édition un poste" }
                            }
                            p.aide { "Un code de la ligne V1 ou de l'édition un poste vaut un poste. Un code de l'édition un poste n'active pas Kadenz Entreprise, et inversement." }
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
        }))
        h2 { "Codes tirés" }
        div.carte data-reveal="fade" {
            @if liste.is_empty() {
                (vide("Aucun code à vendre pour l'instant."))
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
    let resume = premier.map(|f| {
        format!(
            "{} code{} de {} poste{}, formule {}, modules : {}.",
            codes.len(),
            if codes.len() > 1 { "s" } else { "" },
            f.postes,
            if f.postes > 1 { "s" } else { "" },
            libelle_formule(f.formule).to_lowercase(),
            liste_modules(&f.modules)
        )
    });
    let maintenant = chrono::Local::now();
    let fichier = liste_a_telecharger(
        resume.as_deref(),
        &maintenant.format("%Y-%m-%d %H:%M").to_string(),
        codes,
    );
    let nom_fichier = format!("codes-kadenz-{}.txt", maintenant.format("%Y-%m-%d-%H%M"));
    let contenu = html! {
        (titre("Codes tirés", html! {
            a.bouton href=(fichier) download=(nom_fichier) { "Télécharger la liste" }
            a.bouton href="/codes" { "Retour aux codes à vendre" }
        }))
        div.carte data-reveal="fade" {
            div.carte-interieur {
                @if let Some(r) = &resume {
                    p.aide { (r) }
                }
                ul.codes-tires {
                    @for k in codes {
                        li {
                            span.code-activation { (k.code) }
                            button.bouton-copier type="button" hidden data-code=(k.code)
                                aria-label=(format!("Copier le code {}", k.code)) title="Copier" {
                                (PreEscaped(ICONE_COPIER))
                            }
                        }
                    }
                }
                p.avis-code #avis-copie aria-live="polite" {}
                p.aide {
                    "Notez ou copiez ces codes maintenant : ils ne s'afficheront plus. Chaque code s'active une fois, pour une entreprise, dans Kadenz (assistant de KadenzAdmin, ou Paramètres, rubrique « Abonnement », sur un poste Admin)."
                }
            }
        }
    };
    page_formulaire(
        c,
        "Codes tirés",
        Rubrique::Codes,
        html! {
            (contenu)
            script src=(SCRIPT_CODES_TIRES) defer {}
        },
    )
}

/// Icône « copier » (deux feuilles) et coche affichée après la copie.
const ICONE_COPIER: &str = r#"<svg class="icone-copier" viewBox="0 0 24 24" aria-hidden="true"><rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2"/></svg><svg class="icone-fait" viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12.5l4.5 4.5L19 7.5"/></svg>"#;

/// Liste des codes tirés en fichier texte (lignes Windows), en lien `data:`.
/// Rien n'est gardé ni renvoyé par le serveur : les codes en clair ne
/// s'affichent qu'une fois.
fn liste_a_telecharger(resume: Option<&str>, date: &str, codes: &[CodeCree]) -> String {
    const FIN: &str = "\r\n";
    let mut texte = format!("Codes d'activation Kadenz tirés le {date}{FIN}");
    if let Some(r) = resume {
        texte.push_str(r);
        texte.push_str(FIN);
    }
    texte.push_str(FIN);
    for k in codes {
        texte.push_str(&k.code);
        texte.push_str(FIN);
    }
    let mut lien = String::from("data:text/plain;charset=utf-8,");
    for o in texte.bytes() {
        if o.is_ascii_alphanumeric() || b"-_.~".contains(&o) {
            lien.push(o as char);
        } else {
            lien.push_str(&format!("%{o:02X}"));
        }
    }
    lien
}
