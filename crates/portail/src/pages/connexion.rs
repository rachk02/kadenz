//! Connexion : seule page sans compte ni jeton (POST /connexion).

use maud::{DOCTYPE, Markup, PreEscaped, html};

use super::{SYMBOLE, message, tete};

/// Page de connexion ; `jeton` est le jeton anti-falsification de la
/// connexion, le même que dans le cookie posé avec la page.
pub fn connexion(jeton: &str, erreur: Option<&str>) -> Markup {
    html! {
        (DOCTYPE)
        html lang="fr" {
            (tete("Connexion"))
            body {
                div.page-connexion {
                    section.connexion-marque {
                        (PreEscaped(SYMBOLE))
                        div {
                            div.trois-temps aria-hidden="true" { span {} span {} span {} }
                            p.connexion-titre { "Portail des licences" }
                            p.connexion-texte {
                                "Enregistrement des paiements et émission des licences d'abonnement Kadenz, pour l'éditeur et ses revendeurs."
                            }
                        }
                    }
                    main.connexion-formulaire {
                        div.connexion-boite {
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
                                    input #identifiant type="text" name="identifiant"
                                        autocomplete="username" required autofocus;
                                }
                                div.champ {
                                    label for="mot_de_passe" { "Mot de passe" }
                                    input #mot_de_passe type="password" name="mot_de_passe"
                                        autocomplete="current-password" required;
                                }
                                button.bouton.bouton-principal type="submit" { "Se connecter" }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Seconde étape de la connexion (double authentification) ; `etape` est le
/// jeton de l'étape, le même que dans le cookie posé avec la redirection.
pub fn code_connexion(etape: &str, erreur: Option<&str>) -> Markup {
    html! {
        (DOCTYPE)
        html lang="fr" {
            (tete("Code de connexion"))
            body {
                div.page-connexion {
                    section.connexion-marque {
                        (PreEscaped(SYMBOLE))
                        div {
                            div.trois-temps aria-hidden="true" { span {} span {} span {} }
                            p.connexion-titre { "Portail des licences" }
                            p.connexion-texte {
                                "Enregistrement des paiements et émission des licences d'abonnement Kadenz, pour l'éditeur et ses revendeurs."
                            }
                        }
                    }
                    main.connexion-formulaire {
                        div.connexion-boite {
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
                                    input #code type="text" name="code"
                                        autocomplete="one-time-code" spellcheck="false" required autofocus;
                                }
                                button.bouton.bouton-principal type="submit" { "Valider" }
                            }
                            p.texte { a href="/connexion" { "Revenir à la connexion" } }
                        }
                    }
                }
            }
        }
    }
}
