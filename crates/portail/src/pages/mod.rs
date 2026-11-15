//! Pages du portail (F9b, kadenz-8e). Squelette posé par F9a pour que le
//! crate compile : signatures fixées, contenu minimal à remplacer.
//!
//! Règles : aucune ressource externe ni script, aucun attribut `style`
//! (politique de sécurité du contenu), `noindex` sur chaque page ; chaque
//! formulaire POST porte un champ caché `csrf` (sauf la connexion).

use maud::{DOCTYPE, Markup, html};

use crate::{
    Action, Compte, Entreprise, FicheEntreprise, LicenceEmise, Message, Paiement, Proposition,
    VueEntreprise,
};

/// Feuille de style servie en `/style.css`.
pub const STYLE: &str = "body { font-family: sans-serif; }\n";

/// Ce que chaque page d'un compte connecté reçoit.
pub struct Cadre<'a> {
    pub compte: &'a Compte,
    pub csrf: &'a str,
    pub message: Option<&'a Message>,
}

fn page(titre: &str, corps: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="fr" {
            head {
                meta charset="utf-8";
                meta name="robots" content="noindex, nofollow";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (titre) " - Portail Kadenz" }
                link rel="stylesheet" href="/style.css";
            }
            body { (corps) }
        }
    }
}

fn csrf(c: &Cadre) -> Markup {
    html! { input type="hidden" name="csrf" value=(c.csrf); }
}

fn message(c: &Cadre) -> Markup {
    html! {
        @if let Some(m) = c.message { p { (m.texte) } }
    }
}

fn erreur_saisie(erreur: Option<&str>) -> Markup {
    html! { @if let Some(e) = erreur { p { (e) } } }
}

pub fn connexion(erreur: Option<&str>) -> Markup {
    page(
        "Connexion",
        html! {
            h1 { "Portail Kadenz" }
            (erreur_saisie(erreur))
            form method="post" action="/connexion" {
                input name="identifiant" autocomplete="username";
                input name="mot_de_passe" type="password" autocomplete="current-password";
                button { "Se connecter" }
            }
        },
    )
}

pub fn entreprises(c: &Cadre, liste: &[VueEntreprise]) -> Markup {
    page(
        "Entreprises",
        html! {
            h1 { "Entreprises" }
            (message(c))
            a href="/entreprises/nouvelle" { "Nouvelle entreprise" }
            ul { @for v in liste {
                li { a href={ "/entreprises/" (v.entreprise.identifiant) } { (v.entreprise.client) } }
            } }
        },
    )
}

pub fn nouvelle_entreprise(c: &Cadre, revendeurs: &[Compte], erreur: Option<&str>) -> Markup {
    page(
        "Nouvelle entreprise",
        html! {
            h1 { "Nouvelle entreprise" }
            (erreur_saisie(erreur))
            form method="post" action="/entreprises/nouvelle" {
                (csrf(c))
                input name="identifiant";
                input name="client";
                @if !revendeurs.is_empty() {
                    select name="revendeur" {
                        option value="" { "Aucun" }
                        @for r in revendeurs { option value=(r.id) { (r.nom) } }
                    }
                }
                button { "Enregistrer" }
            }
        },
    )
}

pub fn fiche_entreprise(c: &Cadre, f: &FicheEntreprise) -> Markup {
    page(
        &f.entreprise.client,
        html! {
            h1 { (f.entreprise.client) }
            (message(c))
            a href={ "/entreprises/" (f.entreprise.identifiant) "/paiement" } { "Enregistrer un paiement" }
            ul { @for p in &f.paiements {
                li { (p.recu_le) " " (p.montant) " " (p.devise)
                    @if p.licence.is_none() { " " a href={ "/paiements/" (p.id) "/licence" } { "Émettre la licence" } }
                }
            } }
            ul { @for l in &f.licences {
                li { a href={ "/licences/" (l.id) } { (l.echeance) } }
            } }
        },
    )
}

pub fn nouveau_paiement(c: &Cadre, e: &Entreprise, erreur: Option<&str>) -> Markup {
    page(
        "Paiement",
        html! {
            h1 { "Paiement de " (e.client) }
            (erreur_saisie(erreur))
            form method="post" action={ "/entreprises/" (e.identifiant) "/paiement" } {
                (csrf(c))
                input name="recu_le" type="date";
                select name="mode" { @for m in crate::Mode::TOUS { option value=(m.code()) { (m.libelle()) } } }
                input name="montant";
                input name="devise";
                input name="reference";
                input name="postes";
                select name="formule" { option value="mensuelle" { "Mensuelle" } option value="annuelle" { "Annuelle" } }
                button { "Enregistrer" }
            }
        },
    )
}

pub fn emission(
    c: &Cadre,
    e: &Entreprise,
    p: &Paiement,
    prop: &Proposition,
    erreur: Option<&str>,
) -> Markup {
    page(
        "Émission",
        html! {
            h1 { "Licence de " (e.client) }
            (erreur_saisie(erreur))
            form method="post" action={ "/paiements/" (p.id) "/licence" } {
                (csrf(c))
                input name="echeance" type="date" value=(prop.echeance);
                @for m in kadenz_domaine::module::Module::TOUS {
                    label { input type="checkbox" name={ "module_" (m.code()) } value="1" checked[prop.modules.contains(&m)]; (m.libelle()) }
                }
                button { "Émettre" }
            }
        },
    )
}

pub fn licence(c: &Cadre, l: &LicenceEmise) -> Markup {
    page(
        "Licence",
        html! {
            h1 { "Licence" }
            (message(c))
            textarea readonly { (l.texte) }
        },
    )
}

pub fn comptes(c: &Cadre, comptes: &[Compte], erreur: Option<&str>) -> Markup {
    page(
        "Comptes",
        html! {
            h1 { "Comptes" }
            (message(c))
            (erreur_saisie(erreur))
            ul { @for k in comptes {
                li { (k.nom)
                    @if k.role == crate::Role::Revendeur {
                        form method="post" action={ "/comptes/" (k.id) "/actif" } {
                            (csrf(c))
                            input type="hidden" name="actif" value=(if k.actif { "0" } else { "1" });
                            button { (if k.actif { "Désactiver" } else { "Réactiver" }) }
                        }
                    }
                }
            } }
            form method="post" action="/comptes" {
                (csrf(c))
                input name="identifiant";
                input name="nom";
                input name="mot_de_passe" type="password" autocomplete="new-password";
                button { "Créer le revendeur" }
            }
        },
    )
}

pub fn mot_de_passe(c: &Cadre, erreur: Option<&str>) -> Markup {
    page(
        "Mot de passe",
        html! {
            h1 { "Mot de passe" }
            (erreur_saisie(erreur))
            form method="post" action="/mot-de-passe" {
                (csrf(c))
                input name="ancien" type="password" autocomplete="current-password";
                input name="nouveau" type="password" autocomplete="new-password";
                input name="confirmation" type="password" autocomplete="new-password";
                button { "Changer" }
            }
        },
    )
}

pub fn journal(c: &Cadre, actions: &[Action]) -> Markup {
    page(
        "Journal",
        html! {
            h1 { "Journal" }
            (message(c))
            ul { @for a in actions { li { (a.heure) " " (a.action) " " (a.details) } } }
        },
    )
}

pub fn erreur(c: Option<&Cadre>, statut: u16, texte: &str) -> Markup {
    page(
        "Erreur",
        html! {
            h1 { (statut) }
            p { (texte) }
            @if c.is_some() { a href="/" { "Retour" } }
        },
    )
}
