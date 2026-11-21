//! Fiche d'une entreprise (GET /entreprises/{identifiant}) : identité,
//! paiements (avec l'émission depuis un paiement non utilisé) et licences
//! émises, de la plus récente à la plus ancienne (ordre du crate).

use maud::{Markup, html};

use super::{
    Cadre, Rubrique, chemin_entreprise, date, libelle_formule, liste_modules, montant, page, titre,
    vide,
};
use crate::FicheEntreprise;

pub fn fiche_entreprise(c: &Cadre, f: &FicheEntreprise) -> Markup {
    let e = &f.entreprise;
    let chemin = chemin_entreprise(&e.identifiant);
    let contenu = html! {
        (titre(&e.client, html! {
            a.bouton href="/" { "Retour aux entreprises" }
            a.bouton.bouton-principal href={ (chemin) "/paiement" } { "Enregistrer un paiement" }
        }))
        div.carte data-reveal="fade" {
            div.carte-interieur {
                dl.grille-fiche {
                    dt { "Identifiant de l'entreprise" }
                    dd { span.identifiant { (e.identifiant) } }
                    dt { "Revendeur" }
                    dd {
                        @match &e.revendeur {
                            Some((_, nom)) => (nom),
                            None => "Aucun : client direct de l'éditeur",
                        }
                    }
                    dt { "Dernière licence" }
                    dd {
                        @match f.licences.first() {
                            Some(l) => {
                                "Jusqu'au " (date(&l.echeance)) ", " (l.postes)
                                @if l.postes > 1 { " postes" } @else { " poste" }
                            }
                            None => "Aucune licence émise",
                        }
                    }
                }
            }
        }

        h2 { "Paiements" }
        div.carte data-reveal="fade" {
            @if f.paiements.is_empty() {
                (vide("Aucun paiement enregistré. Enregistrez le paiement reçu, puis tirez le code d'activation."))
            } @else {
                div.tableau-cadre {
                    table {
                        thead {
                            tr {
                                th { "Reçu le" }
                                th { "Mode" }
                                th { "Référence" }
                                th.nombre { "Montant" }
                                th.nombre { "Postes" }
                                th { "Formule" }
                                th { "Saisi par" }
                                th { "Licence" }
                            }
                        }
                        tbody {
                            @for p in &f.paiements {
                                tr {
                                    td.court { (date(&p.recu_le)) }
                                    td.court { (p.mode.libelle()) }
                                    td.court { (p.reference.as_deref().unwrap_or("-")) }
                                    td.nombre { (montant(p.montant, &p.devise)) }
                                    td.nombre { (p.postes) }
                                    td.court { (libelle_formule(p.formule)) }
                                    td { (p.saisi_par) }
                                    td.court {
                                        @match (p.licence, p.code) {
                                            (Some(id), _) => a href={ "/licences/" (id) } { "Voir la licence" },
                                            (None, Some(id)) => a href={ "/codes/" (id) } { "Voir le code" },
                                            (None, None) => a.bouton.bouton-principal.bouton-petit
                                                href={ "/paiements/" (p.id) "/licence" } { "Tirer le code" },
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        h2 { "Licences émises" }
        div.carte data-reveal="fade" {
            @if f.licences.is_empty() {
                (vide("Aucune licence émise pour cette entreprise."))
            } @else {
                div.tableau-cadre {
                    table {
                        thead {
                            tr {
                                th { "Émise le" }
                                th { "Début" }
                                th { "Échéance" }
                                th.nombre { "Postes" }
                                th { "Formule" }
                                th { "Modules" }
                                th { "Émetteur" }
                                th {}
                            }
                        }
                        tbody {
                            @for l in &f.licences {
                                tr {
                                    td.court { (date(&l.emise_le)) }
                                    td.court { (date(&l.debut)) }
                                    td.court { (date(&l.echeance)) }
                                    td.nombre { (l.postes) }
                                    td.court { (libelle_formule(l.formule)) }
                                    td { (liste_modules(&l.modules)) }
                                    td { (l.emetteur) }
                                    td.court { a href={ "/licences/" (l.id) } { "Texte de la licence" } }
                                }
                            }
                        }
                    }
                }
            }
        }
    };
    page(c, &e.client, Rubrique::Entreprises, contenu)
}
