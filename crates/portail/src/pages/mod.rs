//! Pages du portail des licences (jalon F9b, spec. portail) : HTML simple
//! rendu sur le serveur, sans script ni style en ligne (la politique de
//! sécurité des contenus les refuse), feuille de style unique servie en
//! `/style.css`. Les signatures sont fixées par les routes de `web.rs` ;
//! chaque page reçoit des données déjà filtrées selon le rôle du compte.

use maud::{DOCTYPE, Markup, PreEscaped, html};

use crate::{Compte, Message, Role};
use kadenz_domaine::module::Module;
use kadenz_licence::entreprise::Formule;

mod code;
mod comptes;
mod connexion;
mod emission;
mod entreprises;
mod erreur;
mod fiche_entreprise;
mod journal;
mod licence;
mod mot_de_passe;
mod nouveau_paiement;
mod nouvelle_entreprise;
mod style;

pub use code::{code, code_cree};
pub use comptes::comptes;
pub use connexion::connexion;
pub use emission::emission;
pub use entreprises::entreprises;
pub use erreur::erreur;
pub use fiche_entreprise::fiche_entreprise;
pub use journal::journal;
pub use licence::licence;
pub use mot_de_passe::mot_de_passe;
pub use nouveau_paiement::nouveau_paiement;
pub use nouvelle_entreprise::nouvelle_entreprise;
pub use style::STYLE;

/// Ce que chaque page d'un compte connecté reçoit : le compte, le jeton
/// anti-falsification de sa session et le message d'une action précédente.
pub struct Cadre<'a> {
    pub compte: &'a Compte,
    pub csrf: &'a str,
    pub message: Option<&'a Message>,
}

/// Rubrique active du menu.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Rubrique {
    Entreprises,
    Comptes,
    Journal,
    MotDePasse,
    Aucune,
}

/// Symbole de la marque, en blanc, posé sur le bandeau Nuit.
const SYMBOLE: &str = r##"<svg class="symbole" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" aria-hidden="true"><path fill="#FFFFFF" d="M16 16 L120 16 L120 120 L16 120 Z M136 120 L136 14.44 A105.56 105.56 0 0 1 241.56 120 Z M16 136 L120 240 L16 240 Z M134.44 188 A53.56 53.56 0 1 1 241.56 188 A53.56 53.56 0 1 1 134.44 188 Z"/></svg>"##;

/// En-tête HTML commun : titre, feuille de style, aucune indexation.
fn tete(titre: &str) -> Markup {
    html! {
        head {
            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1";
            meta name="robots" content="noindex, nofollow";
            title { (titre) " - Portail des licences Kadenz" }
            link rel="icon" href="/favicon.svg" type="image/svg+xml";
            link rel="stylesheet" href="/style.css";
        }
    }
}

/// Page d'un compte connecté : bandeau, menu selon le rôle, message,
/// contenu.
fn page(c: &Cadre, titre: &str, rubrique: Rubrique, contenu: Markup) -> Markup {
    let editeur = c.compte.role == Role::Editeur;
    html! {
        (DOCTYPE)
        html lang="fr" {
            (tete(titre))
            body {
                header.bandeau {
                    div.bandeau-interieur {
                        a.marque href="/" {
                            (PreEscaped(SYMBOLE))
                            span.marque-nom { "Portail des licences" }
                        }
                        nav.menu aria-label="Menu" {
                            (lien_menu("/", "Entreprises", rubrique == Rubrique::Entreprises))
                            @if editeur {
                                (lien_menu("/comptes", "Revendeurs", rubrique == Rubrique::Comptes))
                            }
                            (lien_menu("/journal", "Journal", rubrique == Rubrique::Journal))
                        }
                        div.compte {
                            a.compte-nom
                                href="/mot-de-passe"
                                aria-current=[(rubrique == Rubrique::MotDePasse).then_some("page")]
                                title="Changer de mot de passe" {
                                (c.compte.nom)
                                span.compte-role { (libelle_role(c.compte.role)) }
                            }
                            form.deconnexion method="post" action="/deconnexion" {
                                (jeton(c))
                                button.bouton.bouton-sombre type="submit" { "Se déconnecter" }
                            }
                        }
                    }
                }
                main.contenu {
                    @if let Some(m) = c.message {
                        (message(m.ok, &m.texte))
                    }
                    (contenu)
                }
            }
        }
    }
}

fn lien_menu(href: &str, texte: &str, actif: bool) -> Markup {
    html! {
        a.menu-lien href=(href) aria-current=[actif.then_some("page")] { (texte) }
    }
}

/// Champ caché du jeton anti-falsification, sur chaque formulaire POST.
fn jeton(c: &Cadre) -> Markup {
    html! { input type="hidden" name="csrf" value=(c.csrf); }
}

/// Bandeau de retour d'une action : succès en vert, échec en rouge.
fn message(ok: bool, texte: &str) -> Markup {
    let classe = if ok {
        "message message-ok"
    } else {
        "message message-erreur"
    };
    html! {
        p class=(classe) role=(if ok { "status" } else { "alert" }) { (texte) }
    }
}

/// Erreur d'un formulaire, au-dessus de ses champs.
fn erreur_formulaire(erreur: Option<&str>) -> Markup {
    html! {
        @if let Some(e) = erreur {
            (message(false, e))
        }
    }
}

/// Titre de page avec, à droite, des actions facultatives.
fn titre(texte: &str, actions: Markup) -> Markup {
    html! {
        div.titre-page {
            h1 { (texte) }
            div.titre-actions { (actions) }
        }
    }
}

fn libelle_role(role: Role) -> &'static str {
    match role {
        Role::Editeur => "Éditeur",
        Role::Revendeur => "Revendeur",
    }
}

fn libelle_formule(f: Formule) -> &'static str {
    match f {
        Formule::Mensuelle => "Mensuelle",
        Formule::Annuelle => "Annuelle",
    }
}

/// AAAA-MM-JJ en JJ/MM/AAAA ; toute autre forme est rendue telle quelle.
fn date(texte: &str) -> String {
    let b = texte.as_bytes();
    if b.len() >= 10 && b[4] == b'-' && b[7] == b'-' {
        format!(
            "{}/{}/{}{}",
            &texte[8..10],
            &texte[5..7],
            &texte[0..4],
            &texte[10..]
        )
    } else {
        texte.to_string()
    }
}

/// Heure du journal (UTC, AAAA-MM-JJ HH:MM:SS) en JJ/MM/AAAA HH:MM:SS.
fn heure(texte: &str) -> String {
    date(texte)
}

/// Montant entier avec séparateur des milliers (espace fine insécable) et
/// devise.
fn montant(valeur: i64, devise: &str) -> String {
    let chiffres = valeur.unsigned_abs().to_string();
    let mut groupes = String::new();
    for (i, ch) in chiffres.chars().enumerate() {
        if i > 0 && (chiffres.len() - i).is_multiple_of(3) {
            groupes.push('\u{202F}');
        }
        groupes.push(ch);
    }
    let signe = if valeur < 0 { "-" } else { "" };
    if devise.is_empty() {
        format!("{signe}{groupes}")
    } else {
        format!("{signe}{groupes} {devise}")
    }
}

/// Liste lisible des modules, dans l'ordre de `Module::TOUS`.
fn liste_modules(modules: &[Module]) -> String {
    let noms: Vec<&str> = Module::TOUS
        .into_iter()
        .filter(|m| modules.contains(m))
        .map(|m| m.libelle())
        .collect();
    if noms.len() == Module::TOUS.len() {
        "Tous les modules".to_string()
    } else if noms.is_empty() {
        "Aucun".to_string()
    } else {
        noms.join(", ")
    }
}

/// Chemin d'une entreprise : son identifiant est un UUID, sans caractère à
/// échapper dans un chemin.
fn chemin_entreprise(identifiant: &str) -> String {
    format!("/entreprises/{identifiant}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_et_montants_lisibles() {
        assert_eq!(date("2026-11-15"), "15/11/2026");
        assert_eq!(heure("2026-11-15 16:58:02"), "15/11/2026 16:58:02");
        assert_eq!(date("hier"), "hier");
        assert_eq!(montant(1_250_000, "FCFA"), "1\u{202F}250\u{202F}000 FCFA");
        assert_eq!(montant(950, "FCFA"), "950 FCFA");
        assert_eq!(montant(-12_000, ""), "-12\u{202F}000");
    }

    #[test]
    fn modules_lisibles() {
        assert_eq!(liste_modules(&Module::TOUS), "Tous les modules");
        assert_eq!(
            liste_modules(&[Module::Relances, Module::Devis]),
            "Devis, Relances"
        );
        assert_eq!(liste_modules(&[]), "Aucun");
    }
}
