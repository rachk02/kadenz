//! Échange de licence avec le portail (décision 80, lot R5,
//! `docs/spec-revocation-licences.md`, section 6.2).
//!
//! Dans un fil à part, au lancement puis toutes les 6 heures tant que
//! l'application est ouverte : le poste demande à son serveur l'état de la
//! licence, l'envoie au portail et rend au serveur le bulletin reçu. Rien ne
//! s'affiche tant que tout va bien ; le nouvel état de l'abonnement arrive à
//! l'écran comme d'habitude (`Evenement::Abonnement`). Sans Internet, rien ne
//! se passe et l'échange repart une heure plus tard. Sans licence installée
//! (essai gratuit), le portail n'est pas appelé.

use std::time::Duration;

use kadenz_maj::activation::MODELES_ERREUR as ERREURS_PORTAIL;
use kadenz_maj::etat::{self as portail, PosteEtat};
use kadenz_poste::{ErreurPoste, Poignee};

/// Période de l'échange réussi.
const PERIODE: Duration = Duration::from_secs(6 * 3600);
/// Nouvel essai après un portail injoignable (pas d'Internet).
const NOUVEL_ESSAI: Duration = Duration::from_secs(3600);
/// Attente d'une session (serveur joint, compte connecté).
const ATTENTE_SESSION: Duration = Duration::from_secs(60);

/// Ce qui décide de la prochaine attente.
#[derive(Debug, PartialEq, Eq)]
enum Issue {
    /// Bulletin appliqué, ou rien à échanger (essai gratuit).
    Fait,
    /// Pas encore de session avec un compte connecté.
    SansSession,
    /// Portail injoignable : nouvel essai dans une heure.
    Injoignable,
    /// Refus du portail ou du serveur : nouvel essai à la période suivante,
    /// pour ne pas répéter un refus toutes les heures.
    Refus,
    /// Synchroniseur arrêté : le fil s'arrête.
    Arret,
}

fn attente(issue: &Issue) -> Option<Duration> {
    match issue {
        Issue::Fait | Issue::Refus => Some(PERIODE),
        Issue::SansSession => Some(ATTENTE_SESSION),
        Issue::Injoignable => Some(NOUVEL_ESSAI),
        Issue::Arret => None,
    }
}

/// Lance le fil d'échange pour le synchroniseur de `poignee` ; il s'arrête
/// avec lui.
pub(crate) fn lancer(poignee: Poignee) {
    let _ = std::thread::Builder::new()
        .name("licence-portail".into())
        .spawn(move || {
            while let Some(duree) = attente(&echanger(&poignee)) {
                std::thread::sleep(duree);
            }
        });
}

fn issue_poste(e: &ErreurPoste) -> Issue {
    match e {
        ErreurPoste::Base(m) if m == kadenz_poste::SYNCHRO_ARRETE => Issue::Arret,
        ErreurPoste::HorsConnexion | ErreurPoste::Delai => Issue::SansSession,
        ErreurPoste::Injoignable => Issue::Injoignable,
        _ => Issue::Refus,
    }
}

fn echanger(poignee: &Poignee) -> Issue {
    let etat = match poignee.etat_licence() {
        Ok(etat) => etat,
        Err(e) => return issue_poste(&e),
    };
    if !etat.licence {
        return Issue::Fait;
    }
    let postes: Vec<PosteEtat> = etat
        .postes
        .into_iter()
        .map(|p| PosteEtat {
            empreinte: p.empreinte,
            ordinateur: p.ordinateur,
            nom: p.nom,
            inscrit_le: p.inscrit_le,
            vu_le: p.vu_le.unwrap_or_default(),
        })
        .collect();
    let bulletin = match portail::echanger(&etat.entreprise, &etat.produit, &postes) {
        Ok(b) => b,
        Err(m) if m.modele == ERREURS_PORTAIL[1] => return Issue::Injoignable,
        Err(m) => {
            crate::consigner_erreur(&format!("État de licence refusé par le portail : {m}"));
            return Issue::Refus;
        }
    };
    match poignee.appliquer_bulletin(&bulletin) {
        Ok(_) => Issue::Fait,
        Err(e) => {
            let issue = issue_poste(&e);
            if issue == Issue::Refus {
                crate::consigner_erreur(&format!("Bulletin du portail refusé : {e}"));
            }
            issue
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attentes_selon_l_issue() {
        assert_eq!(attente(&Issue::Fait), Some(PERIODE));
        assert_eq!(attente(&Issue::Refus), Some(PERIODE));
        assert_eq!(attente(&Issue::Injoignable), Some(NOUVEL_ESSAI));
        assert_eq!(attente(&Issue::SansSession), Some(ATTENTE_SESSION));
        assert_eq!(attente(&Issue::Arret), None);
        assert_eq!(
            issue_poste(&ErreurPoste::Base(kadenz_poste::SYNCHRO_ARRETE.into())),
            Issue::Arret
        );
        assert_eq!(issue_poste(&ErreurPoste::HorsConnexion), Issue::SansSession);
        assert_eq!(
            issue_poste(&ErreurPoste::Refus {
                modele: "x".into(),
                valeurs: vec![]
            }),
            Issue::Refus
        );
    }
}
