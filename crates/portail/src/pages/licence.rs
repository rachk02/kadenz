//! Licence émise (GET /licences/{id}) : son texte, à copier dans la rubrique
//! « Abonnement » d'un poste Admin, et ce qu'elle porte.

use maud::{Markup, html};

use super::{
    Cadre, Rubrique, chemin_entreprise, date, libelle_formule, liste_modules, page_formulaire,
    titre,
};
use crate::LicenceEmise;

pub fn licence(c: &Cadre, l: &LicenceEmise) -> Markup {
    let chemin = chemin_entreprise(&l.entreprise);
    let contenu = html! {
        (titre("Licence émise", html! {
            a.bouton href=(chemin) { "Retour à l'entreprise" }
        }))
        div.carte {
            div.carte-interieur {
                div.champ {
                    label for="texte" { "Texte de la licence" }
                    textarea #texte .licence-texte readonly rows="6" spellcheck="false" { (l.texte) }
                    p.aide {
                        "Sélectionnez tout le texte et copiez-le, puis collez-le dans Kadenz : Paramètres, rubrique « Abonnement », sur un poste Admin relié au serveur de l'entreprise."
                    }
                }
                dl.grille-fiche {
                    dt { "Identifiant de l'entreprise" }
                    dd { span.identifiant { (l.entreprise) } }
                    dt { "Début" }
                    dd { (date(&l.debut)) }
                    dt { "Échéance" }
                    dd { (date(&l.echeance)) }
                    dt { "Postes" }
                    dd { (l.postes) }
                    dt { "Formule" }
                    dd { (libelle_formule(l.formule)) }
                    dt { "Modules" }
                    dd { (liste_modules(&l.modules)) }
                    dt { "Émetteur" }
                    dd { (l.emetteur) }
                    dt { "Émise le" }
                    dd { (date(&l.emise_le)) " par " (l.par) }
                }
            }
        }
    };
    page_formulaire(c, "Licence émise", Rubrique::Entreprises, contenu)
}
