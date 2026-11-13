//! Serveur de l'entreprise (phase 2, jalon F4 ; écrans, section 2) : rubrique
//! « Serveur » des Paramètres, inscription de ce poste par code et test de la
//! connexion. Les appels au réseau tournent dans un fil à part ; leur résultat
//! revient au fil de l'interface, qui seul touche à la base.

use std::cell::RefCell;
use std::time::Duration;

use kadenz_client::{Client, ErreurClient, Identite, Inscrit};
use kadenz_protocole::inscription::{self, CodeComplet, PORT_PAR_DEFAUT};
use kadenz_stockage::poste_relie::{self, PosteRelie};
use slint::{ComponentHandle, ModelRc, VecModel};

use crate::referentiels::{retour_erreur, retour_ok, texte_erreur};
use crate::traduction::{tr, trf};
use crate::{EtatPartage, FenetrePrincipale, ServeurApi, ServeurTrouveUi};

/// Délai d'un appel au serveur (connexion, poignée de main, réponse).
const DELAI: Duration = Duration::from_secs(10);
/// Durée d'écoute des réponses à la recherche sur le réseau local.
const DELAI_RECHERCHE: Duration = Duration::from_millis(1500);

/// Inscription en attente de la confirmation de l'empreinte (code court).
struct EnAttente {
    adresse: String,
    empreinte: String,
    secret: String,
}

thread_local! {
    /// État de l'application, pour les résultats qui reviennent du réseau.
    static ETAT: RefCell<Option<EtatPartage>> = const { RefCell::new(None) };
    static EN_ATTENTE: RefCell<Option<EnAttente>> = const { RefCell::new(None) };
}

/// Empreinte en groupes de quatre caractères, comme dans la fenêtre du serveur.
pub(crate) fn empreinte_lisible(empreinte: &str) -> String {
    empreinte
        .to_uppercase()
        .as_bytes()
        .chunks(4)
        .map(|g| String::from_utf8_lossy(g).into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Texte traduit d'une erreur du client.
pub(crate) fn texte_erreur_client(e: &ErreurClient) -> String {
    match e {
        ErreurClient::Injoignable(detail) => trf("serveur injoignable : {}", &[detail]),
        ErreurClient::CertificatDifferent => tr(
            "Le serveur ne présente pas le certificat attendu : la connexion est bloquée. Vérifiez qu'il s'agit bien du serveur de l'entreprise ; s'il a été réinstallé, demandez un nouveau code à l'Admin.",
        ),
        ErreurClient::VersionDifferente { serveur } => trf(
            "le serveur utilise une autre version du protocole ({}) : mettez à jour le serveur ou ce poste",
            &[serveur],
        ),
        ErreurClient::PosteNonInscrit => tr("ce poste n'est pas inscrit sur le serveur"),
        ErreurClient::CompteRefuse { attente_secondes } => trf(
            "identifiant ou mot de passe incorrect (nouvel essai dans {} s)",
            &[attente_secondes],
        ),
        ErreurClient::Refus { modele, valeurs } => {
            let valeurs: Vec<&dyn std::fmt::Display> = valeurs
                .iter()
                .map(|v| v as &dyn std::fmt::Display)
                .collect();
            trf(modele, &valeurs)
        }
        ErreurClient::LectureSeule { .. } => tr(kadenz_stockage::abonnement::MODELE_LECTURE_SEULE),
        ErreurClient::Rejet(motif) => trf("requête refusée par le serveur : {}", &[motif]),
        ErreurClient::Protocole(detail) => trf("réponse du serveur inattendue : {}", &[detail]),
    }
}

/// Adresse saisie : le port par défaut s'ajoute s'il manque.
fn adresse_complete(saisie: &str) -> String {
    let adresse = saisie.trim();
    if adresse
        .rsplit_once(':')
        .is_some_and(|(_, port)| port.parse::<u16>().is_ok())
    {
        adresse.to_string()
    } else {
        format!("{adresse}:{PORT_PAR_DEFAUT}")
    }
}

/// Vrai si ce poste est inscrit sur un serveur.
pub(crate) fn inscrit(conn: &kadenz_stockage::Connexion) -> bool {
    matches!(poste_relie::lire(conn), Ok(Some(_)))
}

/// Recopie le serveur de ce poste dans la rubrique.
pub(crate) fn afficher(f: &FenetrePrincipale, conn: &kadenz_stockage::Connexion) {
    let api = f.global::<ServeurApi>();
    match poste_relie::lire(conn) {
        Ok(Some(p)) => {
            api.set_relie(true);
            api.set_adresse(p.adresse.into());
            api.set_entreprise(p.entreprise.into());
            api.set_nom_poste(p.nom.into());
            api.set_empreinte(empreinte_lisible(&p.empreinte_serveur).into());
        }
        Ok(None) => api.set_relie(false),
        Err(e) => {
            api.set_relie(false);
            api.set_retour(retour_erreur(texte_erreur(e)));
        }
    }
}

fn commencer(f: &FenetrePrincipale) {
    let api = f.global::<ServeurApi>();
    api.set_occupe(true);
    api.set_retour(retour_ok(String::new()));
}

fn finir(f: &FenetrePrincipale, retour: crate::Retour) {
    let api = f.global::<ServeurApi>();
    api.set_occupe(false);
    api.set_retour(retour);
}

/// Inscrit ce poste dans un fil à part, en essayant les adresses dans l'ordre :
/// la suivante seulement si la précédente est injoignable.
fn lancer_inscription(
    f: &FenetrePrincipale,
    adresses: Vec<String>,
    empreinte: String,
    secret: String,
) {
    commencer(f);
    let fa = f.as_weak();
    std::thread::spawn(move || {
        let machine = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "poste".into());
        let resultat = Identite::nouvelle(&machine)
            .map_err(ErreurClient::Protocole)
            .and_then(|identite| {
                let mut derniere = ErreurClient::Injoignable(tr("aucune adresse"));
                for adresse in &adresses {
                    match Client::inscrire(adresse, &empreinte, &identite, &secret, DELAI) {
                        Ok(inscrit) => return Ok((adresse.clone(), identite, inscrit)),
                        Err(ErreurClient::Injoignable(e)) => {
                            derniere = ErreurClient::Injoignable(e);
                        }
                        Err(e) => return Err(e),
                    }
                }
                Err(derniere)
            });
        let _ = fa.upgrade_in_event_loop(move |f| match resultat {
            Ok((adresse, identite, inscrit)) => {
                inscription_reussie(&f, adresse, empreinte, identite, inscrit)
            }
            Err(e) => finir(&f, retour_erreur(texte_erreur_client(&e))),
        });
    });
}

fn inscription_reussie(
    f: &FenetrePrincipale,
    adresse: String,
    empreinte: String,
    identite: Identite,
    inscrit: Inscrit,
) {
    let Some(etat) = ETAT.with_borrow(|e| e.clone()) else {
        return;
    };
    let etat = etat.borrow();
    let poste = PosteRelie {
        adresse,
        empreinte_serveur: empreinte,
        entreprise: inscrit.entreprise,
        nom: inscrit.nom.clone(),
        certificat: identite.certificat,
        cle: identite.cle,
    };
    let retour = match poste_relie::garder(&etat.conn, &poste) {
        Ok(()) => retour_ok(trf("Poste inscrit sous le nom « {} ».", &[&inscrit.nom])),
        Err(e) => retour_erreur(texte_erreur(e)),
    };
    EN_ATTENTE.set(None);
    f.global::<ServeurApi>()
        .set_empreinte_a_confirmer("".into());
    afficher(f, &etat.conn);
    // Rubriques : un poste inscrit n'a plus de « Sauvegardes » (décision 47).
    crate::appliquer_droits(f, &etat);
    finir(f, retour);
}

pub(crate) fn brancher(fenetre: &FenetrePrincipale, etat: &EtatPartage) {
    ETAT.set(Some(etat.clone()));
    let api = fenetre.global::<ServeurApi>();

    let fa = fenetre.as_weak();
    api.on_rechercher(move || {
        let Some(f) = fa.upgrade() else { return };
        commencer(&f);
        let fa = f.as_weak();
        std::thread::spawn(move || {
            let trouves = kadenz_client::decouvrir(DELAI_RECHERCHE);
            let _ = fa.upgrade_in_event_loop(move |f| {
                let api = f.global::<ServeurApi>();
                api.set_trouves(ModelRc::new(VecModel::from(
                    trouves
                        .into_iter()
                        .map(|(source, s)| ServeurTrouveUi {
                            machine: s.machine.into(),
                            adresse: format!("{}:{}", source.ip(), s.port).into(),
                        })
                        .collect::<Vec<_>>(),
                )));
                api.set_recherche_faite(true);
                finir(&f, retour_ok(String::new()));
            });
        });
    });

    let fa = fenetre.as_weak();
    api.on_inscrire(move |code, adresse| {
        let Some(f) = fa.upgrade() else { return };
        // Code complet : adresses et empreinte viennent du code, le poste
        // vérifie seul qu'il parle au bon serveur.
        if let Some(complet) = CodeComplet::decoder(&code) {
            lancer_inscription(&f, complet.adresses, complet.empreinte, complet.secret);
            return;
        }
        if code.trim().is_empty() {
            finir(
                &f,
                retour_erreur(tr("Collez le code complet, ou tapez le code court.")),
            );
            return;
        }
        let Some(secret) = inscription::normaliser_secret(&code) else {
            finir(
                &f,
                retour_erreur(tr(
                    "Code d'inscription illisible : collez le code complet, ou tapez les 12 caractères du code court.",
                )),
            );
            return;
        };
        if adresse.trim().is_empty() {
            finir(
                &f,
                retour_erreur(tr(
                    "Avec le code court, indiquez l'adresse du serveur ou choisissez-le dans la recherche.",
                )),
            );
            return;
        }
        // Code court : l'empreinte présentée est montrée à l'Admin, qui la
        // compare à celle de la fenêtre du serveur avant de confirmer.
        let adresse = adresse_complete(&adresse);
        commencer(&f);
        let fa = f.as_weak();
        std::thread::spawn(move || {
            let resultat = kadenz_client::empreinte_presentee(&adresse, DELAI);
            let _ = fa.upgrade_in_event_loop(move |f| match resultat {
                Ok(empreinte) => {
                    f.global::<ServeurApi>()
                        .set_empreinte_a_confirmer(empreinte_lisible(&empreinte).into());
                    EN_ATTENTE.set(Some(EnAttente {
                        adresse,
                        empreinte,
                        secret,
                    }));
                    finir(&f, retour_ok(String::new()));
                }
                Err(e) => finir(&f, retour_erreur(texte_erreur_client(&e))),
            });
        });
    });

    let fa = fenetre.as_weak();
    api.on_confirmer(move || {
        let Some(f) = fa.upgrade() else { return };
        if let Some(attente) = EN_ATTENTE.take() {
            lancer_inscription(&f, vec![attente.adresse], attente.empreinte, attente.secret);
        }
    });

    let fa = fenetre.as_weak();
    api.on_abandonner(move || {
        EN_ATTENTE.set(None);
        if let Some(f) = fa.upgrade() {
            f.global::<ServeurApi>()
                .set_empreinte_a_confirmer("".into());
            finir(&f, retour_ok(String::new()));
        }
    });

    let (fa, e) = (fenetre.as_weak(), etat.clone());
    api.on_tester(move || {
        let Some(f) = fa.upgrade() else { return };
        let poste = match poste_relie::lire(&e.borrow().conn) {
            Ok(Some(p)) => p,
            Ok(None) => return,
            Err(err) => {
                finir(&f, retour_erreur(texte_erreur(err)));
                return;
            }
        };
        commencer(&f);
        let fa = f.as_weak();
        std::thread::spawn(move || {
            let identite = Identite {
                certificat: poste.certificat,
                cle: poste.cle,
            };
            let resultat =
                Client::connecter(&poste.adresse, &poste.empreinte_serveur, &identite, DELAI);
            let _ = fa.upgrade_in_event_loop(move |f| match resultat {
                Ok(_) => finir(&f, retour_ok(tr("Connexion établie avec le serveur."))),
                Err(e) => finir(&f, retour_erreur(texte_erreur_client(&e))),
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_par_defaut_ajoute() {
        assert_eq!(adresse_complete(" serveur "), "serveur:47380");
        assert_eq!(adresse_complete("192.168.1.10:5000"), "192.168.1.10:5000");
        assert_eq!(adresse_complete("serveur:abc"), "serveur:abc:47380");
    }

    #[test]
    fn empreinte_en_groupes() {
        assert_eq!(empreinte_lisible("ab12cd34ef"), "AB12 CD34 EF");
    }
}
