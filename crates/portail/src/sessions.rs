//! Sessions du portail (spécification du portail, sections 6 et 9) : jeton
//! aléatoire de 32 octets gardé ici, en mémoire ; fin après 8 heures sans
//! activité ou 12 heures après l'ouverture quoi qu'il arrive, à la
//! déconnexion, au changement de mot de passe, ou quand le compte est
//! désactivé. Un arrêt du portail ferme toutes les sessions.

use std::collections::HashMap;
use std::time::{Duration, Instant};

const INACTIVITE: Duration = Duration::from_secs(8 * 3600);
const DUREE_MAX: Duration = Duration::from_secs(12 * 3600);

struct Session {
    compte: i64,
    csrf: String,
    ouverte: Instant,
    derniere: Instant,
}

impl Session {
    fn valable(&self) -> bool {
        self.derniere.elapsed() <= INACTIVITE && self.ouverte.elapsed() <= DUREE_MAX
    }
}

#[derive(Default)]
pub(crate) struct Sessions {
    ouvertes: HashMap<String, Session>,
}

/// 32 octets aléatoires en hexadécimal.
pub(crate) fn alea() -> String {
    let mut octets = [0u8; 32];
    getrandom::fill(&mut octets).expect("générateur aléatoire du système");
    octets.iter().map(|o| format!("{o:02x}")).collect()
}

impl Sessions {
    pub(crate) fn ouvrir(&mut self, compte: i64) -> String {
        self.purger();
        let jeton = alea();
        self.ouvertes.insert(
            jeton.clone(),
            Session {
                compte,
                csrf: alea(),
                ouverte: Instant::now(),
                derniere: Instant::now(),
            },
        );
        jeton
    }

    /// Compte et jeton anti-falsification d'une session encore valide, qui
    /// est prolongée.
    pub(crate) fn toucher(&mut self, jeton: &str) -> Option<(i64, String)> {
        let session = self.ouvertes.get_mut(jeton)?;
        if !session.valable() {
            self.ouvertes.remove(jeton);
            return None;
        }
        session.derniere = Instant::now();
        Some((session.compte, session.csrf.clone()))
    }

    /// Ferme une session ; renvoie son compte et son jeton anti-falsification.
    pub(crate) fn fermer(&mut self, jeton: &str) -> Option<(i64, String)> {
        self.ouvertes.remove(jeton).map(|s| (s.compte, s.csrf))
    }

    pub(crate) fn fermer_compte(&mut self, compte: i64) {
        self.ouvertes.retain(|_, s| s.compte != compte);
    }

    fn purger(&mut self) {
        self.ouvertes.retain(|_, s| s.valable());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duree_absolue_et_fermeture_par_compte() {
        let mut sessions = Sessions::default();
        let jeton = sessions.ouvrir(7);
        let autre = sessions.ouvrir(7);
        let tiers = sessions.ouvrir(8);
        assert!(sessions.toucher(&jeton).is_some());
        // Ouverte depuis plus de 12 heures, même active : finie.
        let vieille = Instant::now().checked_sub(DUREE_MAX + Duration::from_secs(1));
        if let Some(vieille) = vieille {
            sessions.ouvertes.get_mut(&jeton).unwrap().ouverte = vieille;
            assert!(sessions.toucher(&jeton).is_none());
        }
        sessions.fermer_compte(7);
        assert!(sessions.toucher(&autre).is_none());
        assert!(sessions.toucher(&tiers).is_some());
    }
}
