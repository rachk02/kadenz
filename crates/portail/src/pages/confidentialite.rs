//! Page de confidentialité (lot E3, `docs/spec-telechargements-essai.md`) :
//! ce que Kadenz et le portail envoient et gardent, sans compte. Chaque
//! phrase décrit le code tel qu'il est (essai, activation, téléchargement,
//! frein, journal) : un changement de ces traitements change cette page.

use maud::{Markup, html};

use super::connexion::cadre_texte;
use super::tete;

/// Date de la dernière mise à jour du texte, affichée en bas de page.
const MISE_A_JOUR: &str = "27 novembre 2026";

/// Page publique de confidentialité.
pub fn confidentialite() -> Markup {
    cadre_texte(
        tete("Confidentialité"),
        html! {
            p.surtitre { "Confidentialité" }
            h1 { "Données reçues par Kadenz" }
            p.texte {
                "Cette page dit ce que le logiciel Kadenz et ce portail envoient et gardent, "
                "et pourquoi. Kadenz est édité par Mcarre."
            }

            h2 { "Vos données de gestion restent chez vous" }
            p.texte {
                "Les fiches, factures, stocks et comptes saisis dans Kadenz restent sur votre "
                "ordinateur ou sur le serveur de votre entreprise. Kadenz ne les envoie pas à l'éditeur."
            }
            p.texte {
                "L'accès à distance de Kadenz Entreprise, quand il est réglé, passe par un relais "
                "de l'éditeur. Les échanges entre le poste et le serveur y restent chiffrés de bout "
                "en bout ; le relais ne voit que l'identifiant de l'entreprise et le certificat du poste."
            }

            h2 { "Essai gratuit" }
            p.texte {
                "Installé sans code, Kadenz demande au portail la date de début de son essai de "
                "30 jours. Il envoie :"
            }
            ul.liste-texte {
                li { "l'édition (Kadenz ou Kadenz Entreprise) ;" }
                li {
                    "l'identifiant de l'ordinateur : une empreinte calculée à partir du numéro que "
                    "Windows donne à l'ordinateur. Ce numéro ne quitte jamais l'ordinateur, et "
                    "l'empreinte ne permet pas de le retrouver ;"
                }
                li { "la version du programme." }
            }
            p.texte {
                "Le portail garde l'identifiant, l'édition, la date de début de l'essai et les dates "
                "de la première et de la dernière demande. Il s'en sert pour une seule chose : "
                "retrouver la date d'origine quand Kadenz est réinstallé sur le même ordinateur. "
                "Aucun nom, aucune adresse email et aucune donnée de gestion n'est envoyé."
            }
            p.texte {
                "Sans connexion à Internet, rien n'est envoyé et l'essai fonctionne normalement ; "
                "la demande repart à chaque lancement jusqu'à ce qu'elle aboutisse. La date reçue est "
                "gardée sur l'ordinateur dans le fichier "
                code { "%ProgramData%\\Kadenz\\essai-<édition>.txt" }
                ", décrit dans le fichier LISEZMOI de Kadenz."
            }

            h2 { "Activation d'une licence" }
            p.texte {
                "Quand vous saisissez un code d'activation, Kadenz envoie au portail le code, "
                "l'identifiant de votre entreprise (un numéro tiré au hasard à la création de votre "
                "dossier), sa raison sociale, l'édition et la version du programme. Le portail garde "
                "l'identifiant et la raison sociale avec la licence émise."
            }

            h2 { "Téléchargements et mises à jour" }
            p.texte {
                "Pour un téléchargement, le portail note le nom du fichier et l'adresse IP qui l'a "
                "demandé. Pour chercher une mise à jour, Kadenz demande au portail la liste des "
                "versions publiées ; cette demande n'est pas notée. Si le portail ne répond pas, "
                "Kadenz pose la même question à GitHub, qui voit alors l'adresse IP de l'ordinateur."
            }

            h2 { "Adresse IP et journal" }
            p.texte {
                "Chaque demande au portail arrive avec l'adresse IP de la connexion. Elle sert à "
                "freiner les abus, quand une même adresse fait trop de demandes ; ce frein l'oublie "
                "environ un jour après sa dernière demande. Elle figure aussi au journal du portail "
                "pour un démarrage d'essai, une activation et un téléchargement. Ce journal, protégé "
                "contre toute modification, garde la trace des opérations du portail ; il n'est pas effacé."
            }

            h2 { "Hébergement" }
            p.texte {
                "Le portail tourne sur un serveur de l'éditeur. Les connexions passent par Cloudflare, "
                "qui protège le site et achemine les échanges : Cloudflare voit donc l'adresse IP et "
                "les demandes. Les sauvegardes du portail sont chiffrées avant de quitter le serveur."
            }

            h2 { "Cookies" }
            p.texte {
                "Les pages publiques du portail, comme le téléchargement et cette page, ne posent "
                "aucun cookie et ne contiennent aucun outil de mesure d'audience. Seuls la page de "
                "connexion et les comptes de l'éditeur et des revendeurs utilisent des cookies, "
                "pour la connexion."
            }

            h2 { "Questions" }
            p.texte {
                "Pour toute question sur ces données, adressez-vous au revendeur qui vous a fourni "
                "Kadenz, ou à l'éditeur."
            }
            p.aide { "Page mise à jour le " (MISE_A_JOUR) "." }
        },
    )
}
