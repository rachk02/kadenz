//! Fiche d'une entreprise (GET /entreprises/{identifiant}) : identité,
//! paiements (avec l'émission depuis un paiement non utilisé), licences
//! émises, de la plus récente à la plus ancienne (ordre du crate), puis
//! appareils de la licence, ordinateurs bloqués et révocation (décision 80).

use maud::{Markup, html};

use super::{
    Cadre, Rubrique, chemin_entreprise, date, jeton, libelle_formule, liste_modules, montant, page,
    titre, vide,
};
use crate::{Appareil, FicheEntreprise};

/// État d'un appareil, pour la colonne « État ».
fn etat_appareil(a: &Appareil, bloque: bool) -> String {
    match (&a.retire_le, &a.a_retirer_le) {
        (Some(le), _) if bloque => format!("Retiré le {}, ordinateur bloqué", date(le)),
        (Some(le), _) => format!("Retiré le {}", date(le)),
        (None, Some(le)) => format!(
            "Déconnexion demandée le {}, appliquée au prochain contact",
            date(le)
        ),
        (None, None) => "Actif".into(),
    }
}

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
                    dt { "État de la licence" }
                    dd {
                        @match &f.revocation {
                            Some(r) => {
                                strong { "Révoquée" }
                                " le " (date(&r.revoquee_le)) " par " (r.par) " : " (r.raison)
                            }
                            None => "Active",
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

        h2 { "Appareils" }
        div.carte data-reveal="fade" {
            @if f.appareils.is_empty() {
                (vide("Aucun appareil rapporté. Les postes en version 0.3.5 ou plus récente envoient la liste des postes de l'entreprise à chaque lancement, puis toutes les 6 heures."))
            } @else {
                div.tableau-cadre {
                    table {
                        thead {
                            tr {
                                th { "Poste" }
                                th { "Ordinateur" }
                                th { "Inscrit le" }
                                th { "Dernier contact" }
                                th { "État" }
                                th {}
                            }
                        }
                        tbody {
                            @for a in &f.appareils {
                                @let bloque = !a.ordinateur.is_empty()
                                    && f.bloques.iter().any(|b| b.ordinateur == a.ordinateur);
                                tr {
                                    td { (a.nom) }
                                    td.court {
                                        @if a.ordinateur.is_empty() { "Non donné (poste ancien)" }
                                        @else { span.identifiant { (a.ordinateur) } }
                                    }
                                    td.court { (date(&a.inscrit_le)) }
                                    td.court { (date(&a.contact_le)) }
                                    td.court { (etat_appareil(a, bloque)) }
                                    td.court {
                                        @if a.retire_le.is_none() && a.a_retirer_le.is_none() {
                                            form method="post" action={ "/appareils/" (a.id) "/deconnecter" } {
                                                (jeton(c))
                                                button.bouton.bouton-petit type="submit" { "Déconnecter" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                p.aide {
                    "Déconnecter retire le poste du serveur de l'entreprise au prochain contact et bloque son ordinateur : il ne peut plus s'inscrire tant que vous ne l'avez pas réautorisé. Un poste ancien, sans identifiant d'ordinateur, est retiré mais pas bloqué."
                }
            }
        }

        @if !f.bloques.is_empty() {
            h2 { "Ordinateurs bloqués" }
            div.carte data-reveal="fade" {
                div.tableau-cadre {
                    table {
                        thead {
                            tr {
                                th { "Ordinateur" }
                                th { "Bloqué le" }
                                th { "Par" }
                                th {}
                            }
                        }
                        tbody {
                            @for b in &f.bloques {
                                tr {
                                    td.court { span.identifiant { (b.ordinateur) } }
                                    td.court { (date(&b.bloque_le)) }
                                    td { (b.par) }
                                    td.court {
                                        form method="post" action={ (chemin) "/ordinateurs/reautoriser" } {
                                            (jeton(c))
                                            input type="hidden" name="ordinateur" value=(b.ordinateur);
                                            button.bouton.bouton-petit type="submit" { "Réautoriser" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        h2 { "Révocation" }
        div.carte data-reveal="fade" {
            div.carte-interieur {
                @match &f.revocation {
                    Some(r) => {
                        form.formulaire method="post" action={ (chemin) "/retablir" } {
                            (jeton(c))
                            p.aide {
                                "Licence révoquée le " (date(&r.revoquee_le)) " par " (r.par) " : " (r.raison) ". "
                                "Les postes de l'entreprise sont en lecture seule depuis leur premier contact avec le portail après la révocation. "
                                "Rétablir la licence leur rend la saisie au prochain contact."
                            }
                            div.actions {
                                button.bouton.bouton-principal type="submit" { "Rétablir la licence" }
                            }
                        }
                    }
                    None => {
                        form.formulaire method="post" action={ (chemin) "/revoquer" } {
                            (jeton(c))
                            p.aide {
                                "Révoquer la licence met tous les postes de l'entreprise en lecture seule au prochain contact avec le portail : le client garde ses données, mais ne saisit plus rien. Aucun code ne s'active plus pour cette entreprise tant que la licence n'est pas rétablie."
                            }
                            div.champ {
                                label for="raison" { "Raison" }
                                input #raison type="text" name="raison" maxlength="200" required;
                            }
                            label.case {
                                input type="checkbox" name="confirmation" value="1" required;
                                "Je confirme la révocation de la licence de " (e.client) "."
                            }
                            div.actions {
                                button.bouton type="submit" { "Révoquer la licence" }
                            }
                        }
                    }
                }
            }
        }
    };
    page(c, &e.client, Rubrique::Entreprises, contenu)
}
