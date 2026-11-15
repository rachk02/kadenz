//! Connexions freinées (spécification du portail, section 6) : après 5 échecs
//! par identifiant ou par adresse, attente de 5 secondes doublée à chaque
//! échec jusqu'à 15 minutes, comme les sessions du serveur.

use std::collections::HashMap;
use std::time::{Duration, Instant};

const ECHECS_PERMIS: u32 = 5;
const ATTENTE_MIN: Duration = Duration::from_secs(5);
const ATTENTE_MAX: Duration = Duration::from_secs(15 * 60);

#[derive(Default)]
struct Suivi {
    echecs: u32,
    jusqua: Option<Instant>,
}

#[derive(Default)]
pub(crate) struct Frein {
    suivis: HashMap<String, Suivi>,
}

impl Frein {
    /// Secondes d'attente restantes pour l'une des clés, s'il y en a.
    pub(crate) fn attente(&self, cles: &[String]) -> Option<u64> {
        let maintenant = Instant::now();
        cles.iter()
            .filter_map(|c| self.suivis.get(c)?.jusqua)
            .filter(|j| *j > maintenant)
            .map(|j| ((j - maintenant).as_millis() as u64).div_ceil(1000).max(1))
            .max()
    }

    pub(crate) fn echec(&mut self, cles: &[String]) {
        let maintenant = Instant::now();
        for cle in cles {
            let suivi = self.suivis.entry(cle.clone()).or_default();
            suivi.echecs += 1;
            if suivi.echecs >= ECHECS_PERMIS {
                let rang = (suivi.echecs - ECHECS_PERMIS).min(16);
                let attente = (ATTENTE_MIN * 2u32.pow(rang)).min(ATTENTE_MAX);
                suivi.jusqua = Some(maintenant + attente);
            }
        }
    }

    pub(crate) fn reussite(&mut self, cles: &[String]) {
        for cle in cles {
            self.suivis.remove(cle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cinq_echecs_puis_attente_doublee() {
        let mut f = Frein::default();
        let cles = vec!["id:a".to_string()];
        for _ in 0..4 {
            f.echec(&cles);
            assert_eq!(f.attente(&cles), None);
        }
        f.echec(&cles);
        assert_eq!(f.attente(&cles), Some(5));
        f.echec(&cles);
        assert_eq!(f.attente(&cles), Some(10));
        for _ in 0..20 {
            f.echec(&cles);
        }
        assert_eq!(f.attente(&cles), Some(15 * 60));
        f.reussite(&cles);
        assert_eq!(f.attente(&cles), None);
    }
}
