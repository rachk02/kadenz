//! Export comptable SYSCOHADA (`docs/spec-export-syscohada.md`) : nature
//! comptable d'un produit et ventilation d'un document entre marchandises et
//! services (section 4.4).

use std::fmt;

use crate::calcul::{Montant, Pourcentage, appliquer_pourcentage};

/// Nature comptable d'un produit, figée sur la ligne à la validation : elle
/// choisit le compte de ventes (701 ou 706) et le compte de TVA (4431 ou
/// 4432). Indépendante du suivi en stock.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NatureComptable {
    #[default]
    Marchandise,
    Service,
}

impl NatureComptable {
    pub const TOUTES: [NatureComptable; 2] =
        [NatureComptable::Marchandise, NatureComptable::Service];

    pub fn code(self) -> &'static str {
        match self {
            NatureComptable::Marchandise => "marchandise",
            NatureComptable::Service => "service",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|n| n.code() == code)
    }
}

impl fmt::Display for NatureComptable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            NatureComptable::Marchandise => "Marchandise",
            NatureComptable::Service => "Service",
        })
    }
}

/// Part d'un document pour une nature : base hors taxes (remise déduite) et
/// TVA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Part {
    pub nature: NatureComptable,
    pub base: Montant,
    pub tva: Montant,
}

/// Ventile la base et la TVA figées d'un document entre ses natures, d'après
/// le total de chaque ligne. La remise du document est répartie par nature au
/// même taux, puis la TVA de chaque nature est calculée sur sa base ; l'écart
/// d'arrondi va à la plus grosse part (la marchandise à égalité), pour que les
/// parts fassent exactement `base_tva` et `tva`. Une nature sans montant ne
/// donne pas de part.
pub fn ventiler(
    base_tva: Montant,
    tva: Montant,
    remise: Pourcentage,
    taux_tva: Pourcentage,
    lignes: &[(NatureComptable, Montant)],
) -> Vec<Part> {
    let mut parts: Vec<Part> = NatureComptable::TOUTES
        .into_iter()
        .filter_map(|nature| {
            let ht: Montant = lignes
                .iter()
                .filter(|(n, _)| *n == nature)
                .map(|(_, t)| *t)
                .sum();
            (ht != 0).then(|| {
                let base = ht - appliquer_pourcentage(ht, remise);
                Part {
                    nature,
                    base,
                    tva: appliquer_pourcentage(base, taux_tva),
                }
            })
        })
        .collect();
    if parts.is_empty() {
        if base_tva == 0 && tva == 0 {
            return parts;
        }
        parts.push(Part {
            nature: NatureComptable::Marchandise,
            base: 0,
            tva: 0,
        });
    }
    // La plus grosse part en valeur absolue ; à égalité, la première.
    let mut grosse = 0;
    for (i, p) in parts.iter().enumerate() {
        if p.base.abs() > parts[grosse].base.abs() {
            grosse = i;
        }
    }
    let somme_base: Montant = parts.iter().map(|p| p.base).sum();
    let somme_tva: Montant = parts.iter().map(|p| p.tva).sum();
    parts[grosse].base += base_tva - somme_base;
    parts[grosse].tva += tva - somme_tva;
    parts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calcul::{Taux, calculer_totaux};

    #[test]
    fn codes() {
        for n in NatureComptable::TOUTES {
            assert_eq!(NatureComptable::depuis_code(n.code()), Some(n));
        }
        assert_eq!(NatureComptable::default(), NatureComptable::Marchandise);
    }

    #[test]
    fn une_seule_nature_reprend_les_totaux() {
        let p = ventiler(
            90_000,
            16_200,
            1_000,
            1_800,
            &[(NatureComptable::Service, 100_000)],
        );
        assert_eq!(
            p,
            [Part {
                nature: NatureComptable::Service,
                base: 90_000,
                tva: 16_200
            }]
        );
    }

    #[test]
    fn deux_natures_sans_ecart() {
        // CIM50 45 x 6 500 = 292 500 et POSE 15 000, TVA 18 %, sans remise.
        let t = calculer_totaux(
            [(6_500, 45_000), (15_000, 1_000)],
            Taux {
                tva: 1_800,
                ..Taux::default()
            },
            false,
        );
        let p = ventiler(
            t.base_tva,
            t.tva,
            0,
            1_800,
            &[
                (NatureComptable::Marchandise, 292_500),
                (NatureComptable::Service, 15_000),
            ],
        );
        assert_eq!(
            p[0],
            Part {
                nature: NatureComptable::Marchandise,
                base: 292_500,
                tva: 52_650
            }
        );
        assert_eq!(
            p[1],
            Part {
                nature: NatureComptable::Service,
                base: 15_000,
                tva: 2_700
            }
        );
    }

    #[test]
    fn ecart_d_arrondi_sur_la_plus_grosse_part() {
        // Lignes 1 005 (marchandise) et 1 005 (service), puis 3 (marchandise) ;
        // remise 15 % et TVA 18 %.
        let lignes = [
            (NatureComptable::Marchandise, 1_005),
            (NatureComptable::Service, 1_005),
            (NatureComptable::Marchandise, 3),
        ];
        let t = calculer_totaux(
            [(1_005, 1_000), (1_005, 1_000), (3, 1_000)],
            Taux {
                remise: 1_500,
                tva: 1_800,
                retenue: 0,
            },
            false,
        );
        // HT 2 013, remise 302, base 1 711, TVA 308.
        assert_eq!(
            (t.ht, t.remise, t.base_tva, t.tva),
            (2_013, 302, 1_711, 308)
        );
        // Par nature avant correction : marchandise 1 008 - 151 = 857, TVA 154 ;
        // service 1 005 - 151 = 854, TVA 154 : 1 711 et 308, sans écart ici.
        let p = ventiler(t.base_tva, t.tva, 1_500, 1_800, &lignes);
        assert_eq!(p.iter().map(|x| x.base).sum::<Montant>(), t.base_tva);
        assert_eq!(p.iter().map(|x| x.tva).sum::<Montant>(), t.tva);
        // Lignes 5 et 5 à 10 % de remise et 18 % : chaque nature donne 5 - 1 = 4
        // (arrondi de 0,5), soit 8, quand le document donne 10 - 1 = 9.
        let p = ventiler(
            9,
            2,
            1_000,
            1_800,
            &[
                (NatureComptable::Marchandise, 5),
                (NatureComptable::Service, 5),
            ],
        );
        // Égalité : l'écart va à la marchandise, la première.
        assert_eq!(
            p[0],
            Part {
                nature: NatureComptable::Marchandise,
                base: 5,
                tva: 1
            }
        );
        assert_eq!(
            p[1],
            Part {
                nature: NatureComptable::Service,
                base: 4,
                tva: 1
            }
        );
    }

    #[test]
    fn document_sans_montant() {
        assert!(ventiler(0, 0, 0, 1_800, &[(NatureComptable::Marchandise, 0)]).is_empty());
    }
}
