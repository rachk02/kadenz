//! Sessions du portail (spécification du portail, section 6) : jeton
//! aléatoire de 32 octets gardé ici, en mémoire ; fin après 8 heures sans
//! activité, à la déconnexion, ou quand le compte est désactivé. Un arrêt du
//! portail ferme toutes les sessions.

use std::collections::HashMap;
use std::time::{Duration, Instant};

const INACTIVITE: Duration = Duration::from_secs(8 * 3600);

struct Session {
    compte: i64,
    csrf: String,
    derniere: Instant,
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
                derniere: Instant::now(),
            },
        );
        jeton
    }

    /// Compte et jeton anti-falsification d'une session encore valide, qui
    /// est prolongée.
    pub(crate) fn toucher(&mut self, jeton: &str) -> Option<(i64, String)> {
        let session = self.ouvertes.get_mut(jeton)?;
        if session.derniere.elapsed() > INACTIVITE {
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
        self.ouvertes
            .retain(|_, s| s.derniere.elapsed() <= INACTIVITE);
    }
}
