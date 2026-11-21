//! Pages du portail des licences (jalon F9b, spec. portail) : HTML simple
//! rendu sur le serveur, sans script ni style en ligne (la politique de
//! sécurité des contenus les refuse ; seul script, servi par le portail :
//! le code de connexion en six cases, `SCRIPT_CODE`), feuille de style
//! unique servie en `/style.css`. Les signatures sont fixées par les routes de `web.rs` ;
//! chaque page reçoit des données déjà filtrées selon le rôle du compte.

use maud::{DOCTYPE, Markup, PreEscaped, html};

use crate::{Compte, Message, Role};
use kadenz_domaine::module::Module;
use kadenz_licence::entreprise::Formule;

mod code;
mod comptes;
mod connexion;
mod double;
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

pub use code::{code, code_cree, codes, codes_crees};
pub use comptes::comptes;
pub use connexion::{code_connexion, connexion};
pub use double::{codes_secours, double_active, mise_en_place};
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
    Codes,
    Comptes,
    Journal,
}

/// Symbole de la marque, en blanc, posé sur le bandeau Nuit.
const SYMBOLE: &str = r##"<svg class="symbole" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" aria-hidden="true"><path fill="#FFFFFF" d="M16 16 L120 16 L120 120 L16 120 Z M136 120 L136 14.44 A105.56 105.56 0 0 1 241.56 120 Z M16 136 L120 240 L16 240 Z M134.44 188 A53.56 53.56 0 1 1 241.56 188 A53.56 53.56 0 1 1 134.44 188 Z"/></svg>"##;

/// Colonnes de la page de connexion (SVG de points, 44 Ko, tiré de
/// docs/marque/propositions-2026-11-19-portail/piliers.js) ; l'empreinte dans
/// le nom permet un cache long. Servi par `web.rs`.
pub const IMAGE_PILIERS: &str = "/piliers-ff8f49030c.svg";

/// Icône du portail : symbole blanc sur fond noir, au thème « piliers »
/// (demande de l'utilisateur du 2026-11-19). Même règle d'empreinte.
pub const FAVICON: &str = "/favicon-portail-6c6118946e.svg";

/// Image d'aperçu des liens partagés (Open Graph, 1200 x 630, 96 Ko) : les
/// colonnes et le titre du portail. Même règle d'empreinte.
pub const IMAGE_PARTAGE: &str = "/partage-9405c98b03.png";

/// Description de la page de connexion, tirée de son pied (balise
/// description et aperçu des liens partagés).
pub const DESCRIPTION: &str = "Portail des licences Kadenz : enregistrement des paiements et émission des licences d'abonnement, pour l'éditeur et ses revendeurs.";

/// Seul script du portail : le code de connexion en six cases (choix de
/// l'utilisateur du 2026-11-19). Il ne fait qu'embellir la page du code ;
/// sans lui, le champ unique reste. Même règle d'empreinte que l'image.
pub const SCRIPT_CODE: &str = "/code-connexion-bd47a7a773.js";

/// Empreinte FNV-1a 32 bits d'un texte, calculée à la compilation.
const fn empreinte(octets: &[u8]) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    let mut i = 0;
    while i < octets.len() {
        h ^= octets[i] as u32;
        h = h.wrapping_mul(0x0100_0193);
        i += 1;
    }
    h
}

/// Empreinte du texte de la feuille de style : chaque changement du CSS
/// change l'adresse de la feuille, que le navigateur recharge aussitôt malgré
/// son cache (la route `/style.css` ignore la requête).
pub const EMPREINTE_STYLE: u32 = empreinte(STYLE.as_bytes());

/// Adresse de la feuille de style, avec son empreinte.
pub fn lien_style() -> String {
    format!("/style.css?v={EMPREINTE_STYLE:08x}")
}

/// En-tête HTML commun : titre, feuille de style, aucune indexation.
fn tete(titre: &str) -> Markup {
    tete_avec(titre, html! {})
}

/// En-tête HTML avec des balises propres à la page. Le portail reste hors
/// des moteurs de recherche (choix de l'utilisateur du 2026-11-19, « rester
/// invisible ») : `noindex` partout, même avec un aperçu de partage.
fn tete_avec(titre: &str, en_plus: Markup) -> Markup {
    html! {
        head {
            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1";
            meta name="robots" content="noindex, nofollow";
            meta name="theme-color" content="#000000";
            title { (titre) " | Portail des licences Kadenz" }
            (en_plus)
            link rel="icon" href=(FAVICON) type="image/svg+xml";
            link rel="stylesheet" href=(lien_style());
        }
    }
}

/// Page de consultation d'un compte connecté : bandeau, menu selon le rôle,
/// message, contenu, pied (thème noir et sauge de la connexion, 2026-11-21).
fn page(c: &Cadre, titre: &str, rubrique: Rubrique, contenu: Markup) -> Markup {
    page_console(c, titre, rubrique, false, contenu)
}

/// Page à formulaire d'un compte connecté : même bandeau et même pied ; à
/// gauche un grand panneau des colonnes de la connexion, à droite le
/// contenu (écran partagé, demande de l'utilisateur du 2026-11-21).
fn page_formulaire(c: &Cadre, titre: &str, rubrique: Rubrique, contenu: Markup) -> Markup {
    page_console(c, titre, rubrique, true, contenu)
}

fn page_console(
    c: &Cadre,
    titre: &str,
    rubrique: Rubrique,
    partage: bool,
    contenu: Markup,
) -> Markup {
    let editeur = c.compte.role == Role::Editeur;
    html! {
        (DOCTYPE)
        html lang="fr" {
            (tete(titre))
            body.console {
                header.bandeau {
                    div.bandeau-interieur {
                        a.marque href="/" {
                            (PreEscaped(SYMBOLE))
                            span.marque-nom { "Portail des licences" }
                        }
                        nav.menu aria-label="Menu" {
                            (lien_menu("/", "Entreprises", rubrique == Rubrique::Entreprises))
                            (lien_menu("/codes", "Codes à vendre", rubrique == Rubrique::Codes))
                            @if editeur {
                                (lien_menu("/comptes", "Revendeurs", rubrique == Rubrique::Comptes))
                            }
                            (lien_menu("/journal", "Journal", rubrique == Rubrique::Journal))
                        }
                        div.compte {
                            a.compte-nom
                                href="/mot-de-passe"
                                title="Changer de mot de passe" {
                                (c.compte.nom)
                                span.compte-role { (libelle_role(c.compte.role)) }
                            }
                            form.deconnexion method="post" action="/deconnexion" {
                                (jeton(c))
                                button.bouton.bouton-principal type="submit" { "Se déconnecter" }
                            }
                        }
                    }
                }
                @if partage {
                    main.contenu.contenu-partage {
                        div.panneau-illustration aria-hidden="true" {
                            img src=(IMAGE_PILIERS) alt="" width="800" height="900" decoding="async";
                        }
                        div.colonne-formulaire {
                            @if let Some(m) = c.message {
                                (message(m.ok, &m.texte))
                            }
                            (contenu)
                        }
                    }
                } @else {
                    main.contenu {
                        @if let Some(m) = c.message {
                            (message(m.ok, &m.texte))
                        }
                        (contenu)
                    }
                }
                footer.pied-console {
                    div.pied-marque {
                        (PreEscaped(SYMBOLE))
                        p {
                            strong { "Portail des licences" }
                            "Enregistrement des paiements et émission des licences d'abonnement Kadenz, pour l'éditeur et ses revendeurs."
                        }
                    }
                    nav.pied-liens aria-label="Rubriques" {
                        a href="/" { "Entreprises" }
                        a href="/codes" { "Codes à vendre" }
                        @if editeur {
                            a href="/comptes" { "Revendeurs" }
                        }
                        a href="/journal" { "Journal" }
                        a href="/mot-de-passe" { "Changer de mot de passe" }
                    }
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

/// Retour d'une action du compte, en tête d'une page du thème piliers (en
/// tête de `page`, pour les autres pages).
fn message_du_cadre(c: &Cadre) -> Markup {
    html! {
        @if let Some(m) = c.message {
            (message(m.ok, &m.texte))
        }
    }
}

/// Bas d'une page du thème piliers pour un compte connecté, qui n'a pas le
/// bandeau : retour à l'accueil (sauf à la mise en place de la double
/// authentification, où l'accueil renvoie ici) et déconnexion.
fn pied_compte(c: &Cadre, avec_retour: bool) -> Markup {
    html! {
        div.pied-compte {
            @if avec_retour {
                a href="/" { "Retour aux entreprises" }
            }
            form.deconnexion method="post" action="/deconnexion" {
                (jeton(c))
                button.lien-code type="submit" { "Se déconnecter" }
            }
        }
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
