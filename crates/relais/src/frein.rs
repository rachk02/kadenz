//! Frein des nouvelles connexions par adresse (lot R1, relecture de sécurité
//! d'OpenCode, docs/securite/echanges-opencode.md, réponse 6) : sans lui, quelques
//! centaines de connexions par seconde depuis une seule adresse prennent les
//! 4096 places du relais.
//!
//! Deux limites par adresse : connexions ouvertes en même temps, et rythme des
//! nouvelles connexions (seau de jetons). Une adresse IPv6 compte pour son
//! préfixe /64, qu'un seul client obtient d'ordinaire en entier. La table est
//! bornée : une entrée sans connexion ouverte et au seau plein ne sert à rien
//! et part à la purge ; si la table reste pleine, l'adresse nouvelle est
//! refusée, le plafond global du relais tenant le reste.

use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Instant;

/// Connexions ouvertes en même temps depuis une adresse. Pire cas d'une
/// entreprise entière derrière un seul accès Internet (serveur et postes
/// distants sur la même adresse publique), borné par les limites du relais
/// lui-même (relais.rs) :
/// - 1 connexion de contrôle du serveur ;
/// - 64 sessions reliées (RELIEES_PAR_ENTREPRISE), chacune 2 connexions :
///   celle du poste et celle que le serveur ouvre à la demande, soit 128 ;
/// - 16 sessions en attente (ATTENTES_PAR_ENTREPRISE) : la connexion du poste
///   et, au plus, celle du serveur en route, soit 32.
///
/// Soit 161, plus quelques connexions refusées encore en cours de fermeture.
/// Un poste ouvre une connexion par session (`Client::connecter_par_relais`),
/// et une entreprise de 20 postes en ouvre donc 41 dans le cas courant
/// (20 sessions, 2 connexions chacune, plus le contrôle). 256 laisse de la
/// marge au-dessus du pire cas sans laisser une adresse prendre plus d'un
/// seizième des 4096 places.
pub(crate) const OUVERTES_PAR_ADRESSE: usize = 256;
/// Rafale permise, puis nouvelles connexions par seconde. Après un
/// redémarrage du relais, une entreprise à 64 sessions se reconnecte d'un
/// coup avec 129 connexions : la rafale de 160 les admet sans attente.
pub(crate) const RAFALE: f64 = 160.0;
pub(crate) const PAR_SECONDE: f64 = 16.0;
/// Adresses suivies au plus.
pub(crate) const ADRESSES_MAX: usize = 16384;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Cle([u8; 16]);

impl Cle {
    pub(crate) fn de(adresse: IpAddr) -> Cle {
        match adresse {
            IpAddr::V4(v4) => {
                let mut octets = [0u8; 16];
                octets[..4].copy_from_slice(&v4.octets());
                Cle(octets)
            }
            IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
                Some(v4) => Cle::de(IpAddr::V4(v4)),
                None => {
                    // Préfixe /64, marqué pour ne jamais rencontrer une IPv4.
                    let mut octets = [0u8; 16];
                    octets[..8].copy_from_slice(&v6.octets()[..8]);
                    octets[15] = 6;
                    Cle(octets)
                }
            },
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Refus {
    TropOuvertes,
    TropRapide,
    TablePleine,
}

impl Refus {
    pub(crate) fn motif(&self) -> &'static str {
        match self {
            Refus::TropOuvertes => "trop_de_connexions_adresse",
            Refus::TropRapide => "rythme_adresse",
            Refus::TablePleine => "table_adresses_pleine",
        }
    }
}

struct Entree {
    ouvertes: usize,
    jetons: f64,
    vu: Instant,
}

pub(crate) struct Frein {
    entrees: HashMap<Cle, Entree>,
    ouvertes_max: usize,
    rafale: f64,
    par_seconde: f64,
    adresses_max: usize,
}

impl Frein {
    pub(crate) fn nouveau() -> Frein {
        Frein::regle(OUVERTES_PAR_ADRESSE, RAFALE, PAR_SECONDE, ADRESSES_MAX)
    }

    pub(crate) fn regle(
        ouvertes_max: usize,
        rafale: f64,
        par_seconde: f64,
        adresses_max: usize,
    ) -> Frein {
        Frein {
            entrees: HashMap::new(),
            ouvertes_max,
            rafale,
            par_seconde,
            adresses_max,
        }
    }

    /// Nombre d'adresses suivies (essais).
    #[cfg(test)]
    pub(crate) fn suivies(&self) -> usize {
        self.entrees.len()
    }

    /// Nouvelle connexion de `cle` à l'instant `maintenant` : admise (la
    /// place est prise, à rendre par [`Frein::fermer`]) ou refusée.
    pub(crate) fn admettre(&mut self, cle: Cle, maintenant: Instant) -> Result<(), Refus> {
        if !self.entrees.contains_key(&cle) && self.entrees.len() >= self.adresses_max {
            self.purger(maintenant);
            if self.entrees.len() >= self.adresses_max {
                return Err(Refus::TablePleine);
            }
        }
        let (rafale, par_seconde, ouvertes_max) =
            (self.rafale, self.par_seconde, self.ouvertes_max);
        let entree = self.entrees.entry(cle).or_insert(Entree {
            ouvertes: 0,
            jetons: rafale,
            vu: maintenant,
        });
        let ecoule = maintenant
            .saturating_duration_since(entree.vu)
            .as_secs_f64();
        entree.jetons = (entree.jetons + ecoule * par_seconde).min(rafale);
        entree.vu = maintenant;
        if entree.ouvertes >= ouvertes_max {
            return Err(Refus::TropOuvertes);
        }
        if entree.jetons < 1.0 {
            return Err(Refus::TropRapide);
        }
        entree.jetons -= 1.0;
        entree.ouvertes += 1;
        Ok(())
    }

    /// Connexion admise terminée.
    pub(crate) fn fermer(&mut self, cle: Cle) {
        if let Some(entree) = self.entrees.get_mut(&cle) {
            entree.ouvertes = entree.ouvertes.saturating_sub(1);
        }
    }

    /// Retire les entrées sans connexion ouverte dont le seau est redevenu
    /// plein : les garder ne change rien à la décision suivante.
    pub(crate) fn purger(&mut self, maintenant: Instant) {
        let (rafale, par_seconde) = (self.rafale, self.par_seconde);
        self.entrees.retain(|_, e| {
            let ecoule = maintenant.saturating_duration_since(e.vu).as_secs_f64();
            e.ouvertes > 0 || e.jetons + ecoule * par_seconde < rafale
        });
    }
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, Ipv6Addr};
    use std::time::Duration;

    use super::*;

    fn v4(d: u8) -> Cle {
        Cle::de(IpAddr::V4(Ipv4Addr::new(203, 0, 113, d)))
    }

    #[test]
    fn rafale_puis_rythme() {
        let mut f = Frein::regle(1000, 4.0, 2.0, 100);
        let t = Instant::now();
        for _ in 0..4 {
            f.admettre(v4(1), t).unwrap();
        }
        assert_eq!(f.admettre(v4(1), t), Err(Refus::TropRapide));
        // Une autre adresse n'est pas touchée.
        f.admettre(v4(2), t).unwrap();
        // Une demi-seconde rend un jeton.
        f.admettre(v4(1), t + Duration::from_millis(500)).unwrap();
        assert_eq!(
            f.admettre(v4(1), t + Duration::from_millis(500)),
            Err(Refus::TropRapide)
        );
    }

    #[test]
    fn ouvertes_par_adresse() {
        let mut f = Frein::regle(2, 100.0, 100.0, 100);
        let t = Instant::now();
        f.admettre(v4(1), t).unwrap();
        f.admettre(v4(1), t).unwrap();
        assert_eq!(f.admettre(v4(1), t), Err(Refus::TropOuvertes));
        f.fermer(v4(1));
        f.admettre(v4(1), t).unwrap();
    }

    #[test]
    fn table_bornee_et_purgee() {
        let mut f = Frein::regle(10, 2.0, 1.0, 2);
        let t = Instant::now();
        f.admettre(v4(1), t).unwrap();
        f.admettre(v4(2), t).unwrap();
        // Deux adresses avec une connexion ouverte : la troisième est refusée.
        assert_eq!(f.admettre(v4(3), t), Err(Refus::TablePleine));
        // Connexions fermées, seaux revenus pleins : la purge fait de la place.
        f.fermer(v4(1));
        f.fermer(v4(2));
        let plus_tard = t + Duration::from_secs(5);
        f.admettre(v4(3), plus_tard).unwrap();
        assert!(f.suivies() <= 2);
    }

    #[test]
    fn ipv6_par_prefixe() {
        let a = Cle::de(IpAddr::V6("2001:db8:1:2::10".parse::<Ipv6Addr>().unwrap()));
        let b = Cle::de(IpAddr::V6(
            "2001:db8:1:2:ffff::1".parse::<Ipv6Addr>().unwrap(),
        ));
        let c = Cle::de(IpAddr::V6("2001:db8:1:3::10".parse::<Ipv6Addr>().unwrap()));
        assert_eq!(a, b);
        assert_ne!(a, c);
        // Une IPv4 vue en IPv6 compte comme l'IPv4.
        let mappee = Cle::de(IpAddr::V6(Ipv4Addr::new(203, 0, 113, 1).to_ipv6_mapped()));
        assert_eq!(mappee, v4(1));
    }
}
