//! Coque animée des pages du compte (demande de l'utilisateur du 2026-11-21 :
//! « utilise complètement le template Entreprises ») : couverture de
//! transition, en-tête en pilules avec méga-menu et menu mobile, pied au
//! C Tournant, scripts du gabarit (`gabarit.rs`, généré). Le balisage reprend
//! celui de la maquette, attributs de composant compris, pour que ses feuilles
//! s'y appliquent ; listes du menu tirées des vraies données du compte.

use maud::{Markup, PreEscaped, html};

use super::{Cadre, Rubrique, chemin_entreprise, gabarit, jeton};
use crate::{Action, Compte, Role, VueEntreprise};

/// Éléments montrés par section du menu ; le pied donne le nombre total.
const MONTRES: usize = 6;

/// Listes du méga-menu et du menu mobile, lues avec la session.
#[derive(Debug, Default, Clone)]
pub struct Menus {
    /// Entreprises visibles par le compte : nom du client, adresse de la fiche.
    pub entreprises: Vec<(String, String)>,
    /// Revendeurs (éditeur seulement) : nom.
    pub revendeurs: Vec<String>,
    /// Dernières actions du journal : libellé, suivi de l'entreprise visée.
    pub journal: Vec<String>,
}

/// Listes du menu à partir des données du compte (aucun exemple : une liste
/// vide laisse le lien simple, sans panneau).
pub fn menus(entreprises: &[VueEntreprise], revendeurs: &[Compte], journal: &[Action]) -> Menus {
    Menus {
        entreprises: entreprises
            .iter()
            .map(|v| {
                (
                    v.entreprise.client.clone(),
                    chemin_entreprise(&v.entreprise.identifiant),
                )
            })
            .collect(),
        revendeurs: revendeurs.iter().map(|r| r.nom.clone()).collect(),
        journal: journal
            .iter()
            .take(5)
            .map(|a| {
                let libelle = super::journal::libelle_action(&a.action);
                let visee = a
                    .cible
                    .as_deref()
                    .and_then(|c| entreprises.iter().find(|v| v.entreprise.identifiant == c));
                match visee {
                    Some(v) => format!("{libelle}, {}", v.entreprise.client),
                    None => libelle.to_string(),
                }
            })
            .collect(),
    }
}

const FORMES_P: [&str; 7] = [
    "fronton",
    "colonne",
    "arche",
    "amphore",
    "escalier",
    "chapiteau",
    "ruine",
];
const PIECES: [&str; 4] = ["carre", "quart", "tri", "disque"];

/// Vignette d'un élément de menu : A2 et Pierres en alternance (maquette).
fn vignette(n: usize, cle: &str) -> Markup {
    let decalage = format!("{}", (n as f64) * 0.6);
    html! {
        @if n % 2 == 1 {
            canvas data-motif="P" data-cadre="vignette"
                data-forme=(FORMES_P[(n / 2 + cle.len()) % FORMES_P.len()])
                data-decalage=(decalage) aria-hidden="true" {}
        } @else {
            canvas data-motif="A2" data-cadre="vignette" data-piece=(PIECES[n % 4])
                data-decalage=(decalage) aria-hidden="true" {}
        }
    }
}

/// Une section du menu : clé, lien, libellé, éléments (titre, lien), pied.
struct Section<'a> {
    cle: &'static str,
    lien: &'static str,
    texte: &'static str,
    libelle: &'static str,
    elements: Vec<(&'a str, &'a str)>,
    pied: String,
}

fn pluriel(n: usize, un: &str, plusieurs: &str) -> String {
    if n == 1 {
        un.to_string()
    } else {
        format!("Les {n} {plusieurs}")
    }
}

fn sections<'a>(c: &'a Cadre) -> Vec<Section<'a>> {
    let m = c.menus;
    let mut s = vec![Section {
        cle: "entreprises",
        lien: "/",
        texte: "Entreprises",
        libelle: "Toutes les entreprises clientes",
        elements: m
            .entreprises
            .iter()
            .take(MONTRES)
            .map(|(nom, lien)| (nom.as_str(), lien.as_str()))
            .collect(),
        pied: pluriel(m.entreprises.len(), "L'entreprise", "entreprises"),
    }];
    if c.compte.role == Role::Editeur {
        s.push(Section {
            cle: "revendeurs",
            lien: "/comptes",
            texte: "Revendeurs",
            libelle: "Revendeurs",
            elements: m
                .revendeurs
                .iter()
                .take(MONTRES)
                .map(|nom| (nom.as_str(), "/comptes"))
                .collect(),
            pied: pluriel(m.revendeurs.len(), "Le revendeur", "revendeurs"),
        });
    }
    s.push(Section {
        cle: "journal",
        lien: "/journal",
        texte: "Journal",
        libelle: "Chaque action du portail",
        elements: m.journal.iter().map(|t| (t.as_str(), "/journal")).collect(),
        pied: "Tout le journal".to_string(),
    });
    s
}

fn actif(rubrique: Rubrique, cle: &str) -> bool {
    matches!(
        (rubrique, cle),
        (Rubrique::Entreprises, "entreprises")
            | (Rubrique::Codes, "codes")
            | (Rubrique::Comptes, "revendeurs")
            | (Rubrique::Journal, "journal")
    )
}

/// Balises de tête de la coque : feuille du gabarit, couverture posée avant
/// la première peinture, scripts différés dans l'ordre (GSAP avant la coque).
pub(super) fn tete_coque() -> Markup {
    html! {
        link rel="stylesheet" href=(gabarit::CSS);
        script src=(gabarit::JS_COUVERTURE) {}
        script src=(gabarit::JS_MOTIFS) defer {}
        script src=(gabarit::JS_GSAP) defer {}
        script src=(gabarit::JS_SCROLLTRIGGER) defer {}
        script src=(gabarit::JS_SPLITTEXT) defer {}
        script src=(gabarit::JS_OBSERVER) defer {}
        script src=(gabarit::JS_CUSTOMEASE) defer {}
        script src=(gabarit::JS_LENIS) defer {}
        script src=(gabarit::JS_COQUE) defer {}
    }
}

/// Scripts de fin de page, synchrones comme dans la maquette : identifiant du
/// contenu, couleur de la barre avant la première peinture, transitions.
pub(super) fn scripts_fin() -> Markup {
    html! {
        script src=(gabarit::JS_ID) {}
        script src=(gabarit::JS_COULEUR) {}
        script src=(gabarit::JS_TRANSITIONS) {}
    }
}

pub(super) fn couverture() -> Markup {
    PreEscaped(gabarit::COUVERTURE.to_string())
}

/// En-tête en pilules, menu mobile et méga-menu.
pub(super) fn entete(c: &Cadre, rubrique: Rubrique) -> Markup {
    let secs = sections(c);
    let codes_actif = rubrique == Rubrique::Codes;
    html! {
        header.navigation.inverted data-astro-cid-wpvy4v7s {
            div.navigation-bar data-nav-bar data-astro-cid-wpvy4v7s {
                div.navigation-content data-indicator-group data-nav-indicator data-astro-cid-wpvy4v7s {
                    span.pill-indicator data-indicator aria-hidden="true" data-astro-cid-wpvy4v7s {}
                    a.nav-pill.nav-logo href="/" aria-label="Portail des licences Kadenz, accueil"
                        data-indicator-item data-astro-cid-wpvy4v7s {
                        (PreEscaped(gabarit::SVG_LOGO))
                    }
                    nav.nav-pill.nav-menu aria-label="Menu" data-astro-cid-wpvy4v7s {
                        @for (k, s) in secs.iter().enumerate() {
                            @if k == 1 {
                                a.nav-link href="/codes" data-indicator-item
                                    aria-current=[codes_actif.then_some("page")]
                                    data-astro-cid-wpvy4v7s { "Codes à vendre" }
                            }
                            a.nav-link href=(s.lien) data-indicator-item
                                aria-current=[actif(rubrique, s.cle).then_some("page")]
                                data-mega-cle=[(!s.elements.is_empty()).then_some(s.cle)]
                                data-astro-cid-wpvy4v7s { (s.texte) }
                        }
                    }
                    a.nav-pill.nav-login href="/mot-de-passe" title="Changer de mot de passe"
                        data-indicator-item data-astro-cid-wpvy4v7s {
                        span.nav-avatar data-astro-cid-wpvy4v7s {}
                        span.nav-guest aria-hidden="true" data-astro-cid-wpvy4v7s {
                            (PreEscaped(gabarit::SVG_INVITE))
                        }
                        span.nav-link-label data-astro-cid-wpvy4v7s {
                            span.nav-when-out data-astro-cid-wpvy4v7s { (c.compte.nom) }
                        }
                    }
                    form.deconnexion method="post" action="/deconnexion" data-astro-cid-wpvy4v7s {
                        (jeton(c))
                        button.button-animated.nav-cta type="submit" data-text="Se déconnecter"
                            data-astro-cid-wpvy4v7s {
                            span.button-animated-inner data-astro-cid-wpvy4v7s {
                                span data-astro-cid-wpvy4v7s { "Se déconnecter" }
                            }
                        }
                    }
                    button.nav-pill.nav-toggle type="button" aria-expanded="false"
                        aria-controls="mobile-menu" data-menu-toggle data-astro-cid-wpvy4v7s {
                        span.nav-link data-astro-cid-wpvy4v7s {
                            span.nav-toggle-label data-menu-label data-astro-cid-wpvy4v7s { "Menu" }
                        }
                        (PreEscaped(gabarit::SVG_ICONES_MENU))
                    }
                }
            }
            div.mobile-menu #mobile-menu data-menu hidden data-astro-cid-wpvy4v7s {
                div.mobile-menu__overlay data-menu-toggle data-astro-cid-wpvy4v7s {}
                div.mobile-menu__panel data-menu-panel data-astro-cid-wpvy4v7s {
                    nav.mm-step.is-on data-mm-step="root" aria-label="Menu mobile" data-astro-cid-wpvy4v7s {
                        ul.mm-links data-astro-cid-wpvy4v7s {
                            @for (k, s) in secs.iter().enumerate() {
                                @if k == 1 {
                                    li data-astro-cid-wpvy4v7s {
                                        a.mm-link href="/codes" data-menu-link data-astro-cid-wpvy4v7s {
                                            span data-astro-cid-wpvy4v7s { "Codes à vendre" }
                                        }
                                    }
                                }
                                li data-astro-cid-wpvy4v7s {
                                    @if s.elements.is_empty() {
                                        a.mm-link href=(s.lien) data-menu-link data-astro-cid-wpvy4v7s {
                                            span data-astro-cid-wpvy4v7s { (s.texte) }
                                        }
                                    } @else {
                                        button.mm-link type="button" data-mm-open=(s.cle) data-astro-cid-wpvy4v7s {
                                            span data-astro-cid-wpvy4v7s { (s.texte) }
                                            (PreEscaped(gabarit::SVG_CARET))
                                        }
                                    }
                                }
                            }
                        }
                        div.mm-auth data-astro-cid-wpvy4v7s {
                            form method="post" action="/deconnexion" data-astro-cid-wpvy4v7s {
                                (jeton(c))
                                button.mm-button type="submit" data-astro-cid-wpvy4v7s {
                                    span data-astro-cid-wpvy4v7s { "Se déconnecter" }
                                }
                            }
                        }
                    }
                    @for s in secs.iter().filter(|s| !s.elements.is_empty()) {
                        div.mm-step data-mm-step=(s.cle) hidden data-astro-cid-wpvy4v7s {
                            button.mm-back type="button" data-mm-back aria-label={ "Retour, " (s.libelle) }
                                data-astro-cid-wpvy4v7s {
                                (PreEscaped(gabarit::SVG_RETOUR))
                            }
                            ul.mm-items data-astro-cid-wpvy4v7s {
                                @for (n, (titre, lien)) in s.elements.iter().enumerate() {
                                    li data-astro-cid-wpvy4v7s {
                                        a.mm-item href=(lien) data-menu-link data-astro-cid-wpvy4v7s {
                                            span.mm-thumb data-astro-cid-wpvy4v7s { (vignette(n, s.cle)) }
                                            span.mm-item-title data-astro-cid-wpvy4v7s { (titre) }
                                        }
                                    }
                                }
                            }
                            div.mm-foot data-astro-cid-wpvy4v7s {
                                a.mm-all href=(s.lien) data-menu-link data-astro-cid-wpvy4v7s {
                                    span data-astro-cid-wpvy4v7s { (s.pied) }
                                    (PreEscaped(gabarit::SVG_CARET.replace(" class=\"mm-caret\"", "")))
                                }
                            }
                        }
                    }
                }
            }
            div.mega data-mega aria-hidden="true" {
                div.mega-panel data-mega-panel {
                    @for s in secs.iter().filter(|s| !s.elements.is_empty()) {
                        section.mega-section data-mega-section=(s.cle) {
                            div.mega-list {
                                p.mega-label { (s.libelle) }
                                ul {
                                    @for (n, (titre, lien)) in s.elements.iter().enumerate() {
                                        li {
                                            a.mega-item href=(lien) data-mega-index=(n) tabindex="-1" {
                                                span.mega-title { (titre) }
                                            }
                                        }
                                    }
                                }
                                div.mega-foot {
                                    a.mega-all href=(s.lien) tabindex="-1" {
                                        (s.pied) " " span aria-hidden="true" { "→" }
                                    }
                                }
                            }
                            div.mega-window {
                                div.mega-strip data-mega-strip {
                                    @for n in 0..s.elements.len() {
                                        span.mega-canvas { (vignette(n, s.cle)) }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Grand pied : C Tournant en fond, symbole au style des colonnes, liens des
/// rubriques, compte.
pub(super) fn pied(c: &Cadre) -> Markup {
    let editeur = c.compte.role == Role::Editeur;
    html! {
        footer.footer.inverted data-dot-area data-astro-cid-jo6i4kqk {
            canvas.pied-fond data-motif="C" data-ambre-etroit="5,0.43" data-cadre="plein"
                data-decalage="1.3" aria-hidden="true" data-astro-cid-jo6i4kqk {}
            div.container.footer-layout data-astro-cid-jo6i4kqk {
                (PreEscaped(gabarit::PIED_LOGO))
                div.footer-upper data-astro-cid-jo6i4kqk {
                    div.footer-top data-astro-cid-jo6i4kqk {
                        div.footer-newsletter data-astro-cid-jo6i4kqk {
                            p.newsletter-consent.text-small.subtle data-astro-cid-jo6i4kqk {
                                "Enregistrement des paiements et émission des licences d'abonnement Kadenz, pour l'éditeur et ses revendeurs."
                            }
                        }
                        nav.footer-nav aria-label="Entreprises" data-astro-cid-jo6i4kqk {
                            (lien_pied("/", "Toutes les entreprises"))
                            (lien_pied("/entreprises/nouvelle", "Nouvelle entreprise"))
                        }
                        nav.footer-nav aria-label="Licences" data-astro-cid-jo6i4kqk {
                            (lien_pied("/codes", "Codes à vendre"))
                            (lien_pied("/journal", "Journal"))
                        }
                        nav.footer-nav aria-label="Portail" data-astro-cid-jo6i4kqk {
                            @if editeur {
                                (lien_pied("/comptes", "Revendeurs"))
                            }
                            (lien_pied("/mot-de-passe", "Changer de mot de passe"))
                        }
                    }
                }
                div.footer-bottom data-astro-cid-jo6i4kqk {
                    div.footer-block data-astro-cid-jo6i4kqk {
                        div.footer-year data-astro-cid-jo6i4kqk {
                            span.footer-year-roll data-astro-cid-jo6i4kqk {
                                span.footer-year-line data-astro-cid-jo6i4kqk { "Kadenz" }
                                span.footer-year-line.footer-year-build aria-hidden="true" data-astro-cid-jo6i4kqk {
                                    "Portail des licences"
                                }
                            }
                        }
                        div.muted data-astro-cid-jo6i4kqk { "Portail des licences" }
                    }
                    div.footer-block data-astro-cid-jo6i4kqk {
                        div data-astro-cid-jo6i4kqk { "Compte" }
                        a.muted.footer-link-small href="/mot-de-passe" data-astro-cid-jo6i4kqk { (c.compte.nom) }
                    }
                    div.footer-block data-astro-cid-jo6i4kqk {
                        div data-astro-cid-jo6i4kqk { "Polices" }
                        div.muted data-astro-cid-jo6i4kqk {
                            span.footer-link-small data-astro-cid-jo6i4kqk { "Inter" }
                            span.subtle data-astro-cid-jo6i4kqk { "," }
                            " "
                            span.footer-link-small data-astro-cid-jo6i4kqk { "Geist Mono" }
                        }
                    }
                }
            }
        }
    }
}

fn lien_pied(href: &str, texte: &str) -> Markup {
    html! {
        div.row.footer-link-row data-astro-cid-jo6i4kqk {
            a.footer-link.text-large href=(href) data-astro-cid-jo6i4kqk { (texte) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attributs_de_composant_du_gabarit() {
        // Le balisage écrit à la main porte ces attributs : un gabarit régénéré
        // avec d'autres doit être repris ici.
        assert_eq!(gabarit::CID_NAV, "data-astro-cid-wpvy4v7s");
        assert_eq!(gabarit::CID_PIED, "data-astro-cid-jo6i4kqk");
    }

    #[test]
    fn pluriels_du_menu() {
        assert_eq!(pluriel(1, "L'entreprise", "entreprises"), "L'entreprise");
        assert_eq!(
            pluriel(3, "L'entreprise", "entreprises"),
            "Les 3 entreprises"
        );
    }
}
