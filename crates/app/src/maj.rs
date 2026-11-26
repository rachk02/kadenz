//! Mise à jour (jalon J10) : vérification sur un fil secondaire après la connexion
//! d'un compte qui gère les paramètres, invite, téléchargement avec son avancement,
//! lancement de l'installateur puis fermeture de Kadenz.
//! Jalon F10 : un poste relié ne prend jamais une version plus récente que son
//! serveur ; serveur plus récent, la recherche se relance pour ce poste.

use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicBool, Ordering};

use kadenz_domaine::message::MessageCompose;
use kadenz_maj::{MiseAJour, Produit, Recherche, Version};
use kadenz_poste::EtatConnexion;
use slint::ComponentHandle;

use crate::traduction::{tr, trf};
use crate::{EtatPartage, FenetrePrincipale, MiseAJourApi, ReglagesApi};

/// Une seule vérification par lancement, même après plusieurs connexions.
static VERIFIEE: AtomicBool = AtomicBool::new(false);
/// Une seule recherche par lancement quand le serveur est plus récent (F10).
static POUR_LE_SERVEUR: AtomicBool = AtomicBool::new(false);

thread_local! {
    /// Mise à jour proposée, gardée sur le fil de l'interface.
    static PROPOSEE: RefCell<Option<MiseAJour>> = const { RefCell::new(None) };
}

/// Mégaoctets entiers.
fn mo(octets: u64) -> String {
    (octets / 1_048_576).to_string()
}

fn avancement(recus: u64, total: Option<u64>) -> String {
    match total.filter(|t| *t > 0) {
        Some(t) => trf(
            "Téléchargement : {} Mo sur {} Mo ({} %)",
            &[&mo(recus), &mo(t), &(recus * 100 / t)],
        ),
        None => trf("Téléchargement : {} Mo", &[&mo(recus)]),
    }
}

/// Après la connexion : vérifie, sans bloquer l'interface, si une version plus
/// récente existe. Hors ligne ou à jour, rien ne s'affiche.
/// `serveur` : version logicielle du serveur, lue par l'appelant dans l'état
/// qu'il tient déjà (l'état partagé est alors emprunté en écriture).
pub(crate) fn verifier_apres_connexion(
    f: &FenetrePrincipale,
    autorise: bool,
    serveur: Option<String>,
) {
    if !autorise || VERIFIEE.swap(true, Ordering::SeqCst) {
        return;
    }
    verifier_en_fond(f, produit_poste(serveur));
}

/// Recherche silencieuse sur un fil secondaire ; une version à installer
/// ouvre l'invite.
fn verifier_en_fond(f: &FenetrePrincipale, produit: Produit) {
    let fenetre = f.as_weak();
    std::thread::spawn(move || {
        if let Some(m) = kadenz_maj::verifier_pour(produit) {
            let _ = fenetre.upgrade_in_event_loop(move |f| proposer(&f, m));
        }
    });
}

/// Ce poste, plafonné à la version de son serveur quand il la connaît (F10) :
/// un poste ne prend jamais une version plus récente que son serveur.
fn produit_poste(serveur: Option<String>) -> Produit {
    // Édition monoposte : le serveur est dans la fenêtre, pas de plafond.
    if crate::monoposte::edition_monoposte() {
        return Produit::Monoposte;
    }
    Produit::Poste {
        serveur: serveur.as_deref().and_then(Version::lire),
    }
}

/// État de la connexion changé (F10) : version du serveur dans la rubrique
/// Mises à jour ; serveur plus récent que ce poste, une recherche plafonnée à
/// sa version, une fois par lancement, pour un compte qui gère les paramètres.
pub(crate) fn connexion_changee(f: &FenetrePrincipale, etat: &EtatPartage) {
    let (a_mettre_a_jour, autorise, serveur) = {
        let e = etat.borrow();
        let a_mettre_a_jour = e
            .relie
            .as_ref()
            .is_some_and(|r| matches!(r.connexion, EtatConnexion::VersionDifferente { .. }));
        (
            a_mettre_a_jour,
            crate::gere_parametres(&e),
            crate::relie::logiciel_de(&e),
        )
    };
    let api = f.global::<ReglagesApi>();
    let version = serveur.clone().unwrap_or_default();
    // Serveur mis à jour entre-temps : le résultat de la recherche précédente
    // ne vaut plus.
    if api.get_version_serveur() != version.as_str() && !api.get_recherche_en_cours() {
        api.set_etat_maj("".into());
        api.set_erreur_maj(false);
    }
    api.set_version_serveur(version.into());
    if a_mettre_a_jour && autorise && !POUR_LE_SERVEUR.swap(true, Ordering::SeqCst) {
        verifier_en_fond(f, produit_poste(serveur));
    }
}

fn proposer(f: &FenetrePrincipale, m: MiseAJour) {
    let api = f.global::<MiseAJourApi>();
    api.set_version(m.version.as_str().into());
    api.set_notes(kadenz_maj::notes::pour_affichage(&m.notes).into());
    api.set_etat("".into());
    api.set_erreur(false);
    api.set_en_cours(false);
    api.set_visible(true);
    PROPOSEE.with(|p| *p.borrow_mut() = Some(m));
}

fn echec(f: &FenetrePrincipale, message: MessageCompose) {
    let api = f.global::<MiseAJourApi>();
    let texte = crate::referentiels::texte_erreur(kadenz_stockage::Erreur::from(message));
    api.set_etat(crate::phrase(texte));
    api.set_erreur(true);
    api.set_en_cours(false);
}

/// Paramètres, rubrique Mises à jour : recherche à la demande, sur un fil secondaire.
/// Une version plus récente ouvre l'invite habituelle.
fn brancher_recherche(fenetre: &FenetrePrincipale) {
    let f = fenetre.as_weak();
    fenetre.global::<ReglagesApi>().on_rechercher_maj(move || {
        let Some(fenetre) = f.upgrade() else {
            return;
        };
        let api = fenetre.global::<ReglagesApi>();
        api.set_recherche_en_cours(true);
        api.set_erreur_maj(false);
        api.set_etat_maj(tr("Recherche en cours").into());
        let serveur = crate::relie::logiciel_serveur();
        api.set_version_serveur(serveur.clone().unwrap_or_default().into());
        let produit = produit_poste(serveur);
        let f = f.clone();
        std::thread::spawn(move || {
            let recherche = kadenz_maj::rechercher_pour(produit);
            let _ = f.upgrade_in_event_loop(move |f| {
                let api = f.global::<ReglagesApi>();
                api.set_recherche_en_cours(false);
                match recherche {
                    Recherche::Disponible(m) => {
                        api.set_erreur_maj(false);
                        api.set_etat_maj(
                            trf("La version {} est disponible.", &[&m.version]).into(),
                        );
                        proposer(&f, m);
                    }
                    Recherche::AJour => {
                        api.set_erreur_maj(false);
                        api.set_etat_maj(tr("Kadenz est à jour.").into());
                    }
                    // Serveur plus ancien que la version publiée (F10) : il
                    // passe d'abord, ce poste ensuite.
                    Recherche::AttendServeur { version, serveur } => {
                        api.set_erreur_maj(false);
                        api.set_etat_maj(
                            trf(
                                "La version {} de Kadenz est disponible, mais le serveur est en version {} : mettez d'abord à jour le serveur, depuis sa fenêtre d'administration. Ce poste se mettra à jour ensuite.",
                                &[&version, &serveur],
                            )
                            .into(),
                        );
                    }
                    Recherche::Refusee => {
                        api.set_erreur_maj(true);
                        api.set_etat_maj(
                            tr("Une version plus récente a été trouvée, mais sa signature n'est pas valide : elle n'est pas proposée.")
                                .into(),
                        );
                    }
                    Recherche::AutreGeneration(version) => {
                        api.set_erreur_maj(false);
                        api.set_etat_maj(
                            trf(
                                "Kadenz {} est une nouvelle génération : elle s'installe séparément.",
                                &[&version],
                            )
                            .into(),
                        );
                    }
                    Recherche::HorsLigne => {
                        api.set_erreur_maj(true);
                        api.set_etat_maj(
                            tr("Recherche impossible : le serveur des mises à jour ne répond pas. Vérifiez la connexion à Internet.")
                                .into(),
                        );
                    }
                }
            });
        });
    });
}

pub(crate) fn brancher(fenetre: &FenetrePrincipale) {
    brancher_recherche(fenetre);
    let api = fenetre.global::<MiseAJourApi>();

    let f = fenetre.as_weak();
    api.on_plus_tard(move || {
        if let Some(f) = f.upgrade() {
            f.global::<MiseAJourApi>().set_visible(false);
        }
    });

    let f = fenetre.as_weak();
    api.on_installer(move || {
        let Some(fenetre) = f.upgrade() else {
            return;
        };
        let Some(m) = PROPOSEE.with(|p| p.borrow().clone()) else {
            return;
        };
        let api = fenetre.global::<MiseAJourApi>();
        api.set_en_cours(true);
        api.set_erreur(false);
        api.set_etat(tr("Téléchargement en cours").into());
        let f = f.clone();
        std::thread::spawn(move || {
            // L'écran n'est rafraîchi qu'à chaque pour cent (ou chaque mégaoctet).
            let dernier = Cell::new(u64::MAX);
            let suivi = f.clone();
            let resultat = kadenz_maj::telecharger(&m, |recus, total| {
                let palier = match total.filter(|t| *t > 0) {
                    Some(t) => recus * 100 / t,
                    None => recus / 1_048_576,
                };
                if palier != dernier.replace(palier) {
                    let texte = avancement(recus, total);
                    let _ = suivi.upgrade_in_event_loop(move |f| {
                        f.global::<MiseAJourApi>().set_etat(texte.into());
                    });
                }
            });
            let _ = f.upgrade_in_event_loop(move |f| match resultat {
                // Essai (développement seulement) : rien n'est lancé.
                Ok(chemin) if kadenz_maj::lancement_simule() => {
                    let api = f.global::<MiseAJourApi>();
                    api.set_en_cours(false);
                    api.set_etat(
                        trf(
                            "Installateur téléchargé et vérifié (essai : lancement désactivé) : {}",
                            &[&chemin.display()],
                        )
                        .into(),
                    );
                }
                Ok(chemin) => match kadenz_maj::lancer_mise_a_jour(
                    &chemin,
                    crate::traduction::code_courant(),
                    &std::env::current_exe().unwrap_or_default(),
                ) {
                    // L'installateur a démarré : Kadenz se ferme pour le laisser remplacer
                    // l'exécutable ; la base est refermée proprement à la sortie.
                    Ok(()) => {
                        let _ = slint::quit_event_loop();
                    }
                    Err(message) => echec(&f, message),
                },
                Err(message) => echec(&f, message),
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avancement_lisible() {
        assert_eq!(mo(3 * 1_048_576 + 5), "3");
        assert!(avancement(5, None).contains('0'));
    }

    /// Messages d'erreur de la mise à jour : six traductions chacun.
    #[test]
    fn textes_du_domaine_traduits() {
        let manquants = crate::traduction::sans_traduction(kadenz_maj::MODELES_ERREUR);
        assert!(manquants.is_empty(), "sans traduction : {manquants:?}");
    }
}
