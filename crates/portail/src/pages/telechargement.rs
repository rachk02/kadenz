//! Page « Votre téléchargement va démarrer » (décision 77) : sans compte, au
//! thème piliers. Le téléchargement part tout seul par un rafraîchissement
//! (aucun script, la politique de sécurité n'en admet pas d'autre que les
//! siens) ; le bouton porte le même lien signé.

use maud::{Markup, html};

use super::connexion::cadre_large;
use super::tete_avec;

/// Page qui lance le téléchargement de `edition` (nom affiché) par `lien`
/// (lien relatif signé, `/fichiers/...`).
pub fn telechargement(edition: &str, nom: &str, lien: &str) -> Markup {
    cadre_large(
        tete_avec(
            "Téléchargement",
            html! {
                meta http-equiv="refresh" content=(format!("2;url={lien}"));
            },
        ),
        html! {
            p.surtitre { "Téléchargement" }
            h1 { (edition) }
            p.texte {
                "Votre téléchargement va démarrer dans quelques secondes. "
                "S'il ne démarre pas, cliquez sur le bouton ci-dessous."
            }
            p { a.bouton-sauge href=(lien) download=(nom) { "Télécharger " (nom) } }
            p.texte {
                "Une fois le fichier enregistré, ouvrez-le pour installer Kadenz. "
                "L'essai gratuit de 30 jours est compris."
            }
        },
    )
}
