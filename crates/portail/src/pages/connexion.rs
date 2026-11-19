//! Connexion : seules pages sans compte (POST /connexion, puis
//! /connexion/code). Thème « piliers » adopté par l'utilisateur le
//! 2026-11-19 : formulaire à gauche, colonnes en points à droite (image
//! servie par le portail, masquée sous 860 px). Seul script : le code en
//! six cases de la seconde étape, qui garde son champ unique sans lui.

use maud::{DOCTYPE, Markup, PreEscaped, html};

use super::{
    DESCRIPTION, IMAGE_PARTAGE, IMAGE_PILIERS, SCRIPT_CODE, SYMBOLE, message, tete, tete_avec,
};

const ICONE_IDENTIFIANT: &str = r#"<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><circle cx="12" cy="8" r="4"/><path d="M4 21c0-4 3.6-6 8-6s8 2 8 6"/></svg>"#;
const ICONE_MOT_DE_PASSE: &str = r#"<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><circle cx="8" cy="15" r="4"/><path d="M11 12l9-9M16 7l3 3M14 9l2 2"/></svg>"#;
const ICONE_CODE: &str = r#"<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><rect x="5" y="11" width="14" height="10" rx="1"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/></svg>"#;

/// Balises de la page de connexion : description, et aperçu des liens
/// partagés quand l'adresse publique du portail est connue (`og:url` et
/// `og:image` exigent une adresse absolue). Pas de balise canonique : la
/// page reste en `noindex`.
fn balises_connexion(adresse_publique: Option<&str>) -> Markup {
    html! {
        meta name="description" content=(DESCRIPTION);
        @if let Some(a) = adresse_publique {
            meta property="og:type" content="website";
            meta property="og:site_name" content="Kadenz";
            meta property="og:locale" content="fr_FR";
            meta property="og:title" content="Portail des licences Kadenz";
            meta property="og:description" content=(DESCRIPTION);
            meta property="og:url" content=(format!("{a}/connexion"));
            meta property="og:image" content=(format!("{a}{IMAGE_PARTAGE}"));
            meta property="og:image:type" content="image/png";
            meta property="og:image:width" content="1200";
            meta property="og:image:height" content="630";
            meta property="og:image:alt" content="Portail des licences Kadenz : colonnes antiques en points sur fond noir.";
            meta name="twitter:card" content="summary_large_image";
            meta name="twitter:title" content="Portail des licences Kadenz";
            meta name="twitter:description" content=(DESCRIPTION);
            meta name="twitter:image" content=(format!("{a}{IMAGE_PARTAGE}"));
        }
    }
}

/// Cadre commun des deux pages : formulaire, pied, colonnes.
fn cadre(tete_page: Markup, contenu: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="fr" {
            (tete_page)
            body.connexion {
                div.cadre-connexion {
                    main.colonne-connexion {
                        div.connexion-boite {
                            div.connexion-symbole { (PreEscaped(SYMBOLE)) }
                            (contenu)
                            div.separateur aria-hidden="true" { "kadenz" }
                            p.connexion-pied {
                                "Enregistrement des paiements et émission des licences d'abonnement Kadenz, pour l'éditeur et ses revendeurs."
                            }
                        }
                    }
                    div.illustration-connexion aria-hidden="true" {
                        img src=(IMAGE_PILIERS) alt="" width="800" height="900" loading="lazy" decoding="async";
                    }
                }
            }
        }
    }
}

/// Page de connexion ; `jeton` est le jeton anti-falsification de la
/// connexion, le même que dans le cookie posé avec la page ;
/// `adresse_publique`, l'adresse du portail sur Internet, sans barre finale
/// (aperçu des liens partagés).
pub fn connexion(jeton: &str, erreur: Option<&str>, adresse_publique: Option<&str>) -> Markup {
    cadre(
        tete_avec("Connexion", balises_connexion(adresse_publique)),
        html! {
            p.surtitre { "Connexion" }
            h1 { "Portail des licences" }
            p.texte { "Saisissez votre identifiant et votre mot de passe." }
            @if let Some(e) = erreur {
                (message(false, e))
            }
            form method="post" action="/connexion" {
                input type="hidden" name="csrf" value=(jeton);
                div.champ {
                    label for="identifiant" { "Identifiant" }
                    div.saisie {
                        (PreEscaped(ICONE_IDENTIFIANT))
                        input #identifiant type="text" name="identifiant"
                            autocomplete="username" required autofocus;
                    }
                }
                div.champ {
                    label for="mot_de_passe" { "Mot de passe" }
                    div.saisie {
                        (PreEscaped(ICONE_MOT_DE_PASSE))
                        input #mot_de_passe type="password" name="mot_de_passe"
                            autocomplete="current-password" required;
                    }
                }
                button.bouton-sauge type="submit" { "Se connecter" }
            }
        },
    )
}

/// Seconde étape de la connexion (double authentification) ; `etape` est le
/// jeton de l'étape, le même que dans le cookie posé avec la redirection.
/// Le script `SCRIPT_CODE` remplace le champ unique par six cases à
/// validation automatique ; sans script, le champ unique et son bouton
/// restent (codes de secours compris).
pub fn code_connexion(etape: &str, erreur: Option<&str>) -> Markup {
    cadre(
        tete("Code de connexion"),
        html! {
            p.surtitre { "Double authentification" }
            h1 { "Code de connexion" }
            p.texte { "Saisissez le code à 6 chiffres affiché par votre application d'authentification, ou l'un de vos codes de secours." }
            @if let Some(e) = erreur {
                (message(false, e))
            }
            form method="post" action="/connexion/code" {
                input type="hidden" name="etape" value=(etape);
                div.champ {
                    label for="code" { "Code" }
                    div.saisie {
                        (PreEscaped(ICONE_CODE))
                        input #code type="text" name="code"
                            autocomplete="one-time-code" spellcheck="false" required autofocus;
                    }
                }
                button.bouton-sauge type="submit" { "Valider" }
            }
            p.connexion-retour { a href="/connexion" { "Revenir à la connexion" } }
            script src=(SCRIPT_CODE) defer {}
        },
    )
}
