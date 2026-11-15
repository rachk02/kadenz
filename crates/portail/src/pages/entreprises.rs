//! Liste des entreprises (GET /) : toutes pour l'éditeur, celles qui lui sont
//! rattachées pour un revendeur (filtre fait par le crate).

use maud::{Markup, html};

use super::{Cadre, Rubrique, chemin_entreprise, date, page, titre};
use crate::{Role, VueEntreprise};

pub fn entreprises(c: &Cadre, liste: &[VueEntreprise]) -> Markup {
    let editeur = c.compte.role == Role::Editeur;
    let contenu = html! {
        (titre("Entreprises", html! {
            a.bouton.bouton-principal href="/entreprises/nouvelle" { "Nouvelle entreprise" }
        }))
        p.texte {
            @if editeur {
                "Toutes les entreprises clientes, directes ou rattachées à un revendeur. Ouvrez une entreprise pour enregistrer un paiement et émettre sa licence."
            } @else {
                "Les entreprises qui vous sont rattachées. Ouvrez une entreprise pour enregistrer un paiement et émettre sa licence."
            }
        }
        div.carte {
            @if liste.is_empty() {
                p.vide { "Aucune entreprise pour l'instant." }
            } @else {
                div.tableau-cadre {
                    table {
                        thead {
                            tr {
                                th { "Client" }
                                th { "Identifiant de l'entreprise" }
                                @if editeur { th { "Revendeur" } }
                                th { "Échéance" }
                                th.nombre { "Postes" }
                                th { "Paiements" }
                            }
                        }
                        tbody {
                            @for v in liste {
                                tr {
                                    td {
                                        a href=(chemin_entreprise(&v.entreprise.identifiant)) {
                                            (v.entreprise.client)
                                        }
                                    }
                                    td { span.identifiant { (v.entreprise.identifiant) } }
                                    @if editeur {
                                        td {
                                            @match &v.entreprise.revendeur {
                                                Some((_, nom)) => (nom),
                                                None => "Client direct",
                                            }
                                        }
                                    }
                                    @match &v.derniere {
                                        Some(l) => {
                                            td.court { (date(&l.echeance)) }
                                            td.nombre { (l.postes) }
                                        }
                                        None => {
                                            td.court { span.etat.etat-neutre { "Aucune licence" } }
                                            td.nombre { "-" }
                                        }
                                    }
                                    td.court {
                                        @if v.paiements_libres > 0 {
                                            span.etat.etat-attente {
                                                @if v.paiements_libres == 1 {
                                                    "1 paiement à utiliser"
                                                } @else {
                                                    (v.paiements_libres) " paiements à utiliser"
                                                }
                                            }
                                        } @else {
                                            span.etat.etat-neutre { "Aucun à utiliser" }
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
    page(c, "Entreprises", Rubrique::Entreprises, contenu)
}
