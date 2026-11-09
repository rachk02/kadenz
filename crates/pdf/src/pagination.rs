//! Répartition des lignes d'un document sur les pages (section 12, règles
//! multi-pages). Calcul pur, sans tracé : il travaille sur des hauteurs mesurées.
//!
//! - Une ligne n'est jamais coupée entre deux pages.
//! - « À reporter » en bas de chaque page sauf la dernière, « Report » en haut de
//!   la suivante (documents à montants seulement).
//! - Le bloc final (totaux, montant en lettres, signataire) n'apparaît que sur la
//!   dernière page ; s'il ne tient pas sous les dernières lignes, il passe en entier
//!   sur une page supplémentaire.

use std::ops::Range;

/// Hauteurs disponibles, en points.
#[derive(Debug, Clone, Copy)]
pub struct Gabarit {
    /// Hauteur offerte aux lignes du tableau en page 1 (sous l'en-tête du tableau).
    pub premiere: f32,
    /// Même chose sur les pages suivantes.
    pub suivantes: f32,
    /// Hauteur d'une ligne « Report » ou « À reporter ».
    pub ligne_report: f32,
    /// Hauteur du bloc final.
    pub bloc_final: f32,
    /// Lignes de report (documents à montants).
    pub avec_report: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PageTableau {
    pub lignes: Range<usize>,
    pub report_en_haut: bool,
    pub a_reporter_en_bas: bool,
    pub bloc_final: bool,
}

pub fn paginer(hauteurs: &[f32], g: &Gabarit) -> Vec<PageTableau> {
    let n = hauteurs.len();
    let report = if g.avec_report { g.ligne_report } else { 0.0 };
    let mut pages: Vec<PageTableau> = Vec::new();
    let mut i = 0;
    loop {
        let premiere = pages.is_empty();
        let report_en_haut = g.avec_report && !premiere;
        let dispo = if premiere { g.premiere } else { g.suivantes }
            - if report_en_haut { report } else { 0.0 };
        let reste: f32 = hauteurs[i..].iter().sum();
        // Dernière page : toutes les lignes restantes et le bloc final tiennent.
        // Sans ligne restante, le bloc final est posé même s'il déborde (rien de
        // mieux à faire, et la boucle doit finir).
        if reste + g.bloc_final <= dispo || i == n {
            pages.push(PageTableau {
                lignes: i..n,
                report_en_haut,
                a_reporter_en_bas: false,
                bloc_final: true,
            });
            return pages;
        }
        // Sinon, autant de lignes que possible, place gardée pour « À reporter ».
        let debut = i;
        let mut prise = 0.0;
        while i < n && prise + hauteurs[i] + report <= dispo {
            prise += hauteurs[i];
            i += 1;
        }
        if i == debut {
            // Ligne plus haute qu'une page : posée seule, pour avancer malgré tout.
            i += 1;
        }
        pages.push(PageTableau {
            lignes: debut..i,
            report_en_haut,
            a_reporter_en_bas: g.avec_report,
            bloc_final: false,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gabarit() -> Gabarit {
        Gabarit {
            premiere: 100.0,
            suivantes: 150.0,
            ligne_report: 10.0,
            bloc_final: 40.0,
            avec_report: true,
        }
    }

    fn page(lignes: Range<usize>, haut: bool, bas: bool, fin: bool) -> PageTableau {
        PageTableau {
            lignes,
            report_en_haut: haut,
            a_reporter_en_bas: bas,
            bloc_final: fin,
        }
    }

    #[test]
    fn une_page() {
        assert_eq!(
            paginer(&[20.0; 3], &gabarit()),
            [page(0..3, false, false, true)]
        );
        assert_eq!(paginer(&[], &gabarit()), [page(0..0, false, false, true)]);
    }

    #[test]
    fn plusieurs_pages_avec_report() {
        // Page 1 : 100 dont 10 réservés pour « À reporter » : 4 lignes de 20.
        // Page 2 : 150 - 10 (report) - 10 (à reporter) = 130 : 6 lignes.
        // Page 3 : 140 disponibles, 2 lignes (40) + bloc (40) tiennent.
        let pages = paginer(&[20.0; 12], &gabarit());
        assert_eq!(
            pages,
            [
                page(0..4, false, true, false),
                page(4..10, true, true, false),
                page(10..12, true, false, true),
            ]
        );
    }

    #[test]
    fn bloc_final_seul_sur_une_page_supplementaire() {
        // 4 lignes (80) tiennent en page 1, mais pas avec le bloc final (40).
        let pages = paginer(&[20.0; 4], &gabarit());
        assert_eq!(
            pages,
            [
                page(0..4, false, true, false),
                page(4..4, true, false, true),
            ]
        );
    }

    #[test]
    fn ligne_trop_haute_posee_seule() {
        let pages = paginer(&[500.0, 20.0], &gabarit());
        assert_eq!(pages[0], page(0..1, false, true, false));
        assert_eq!(pages[1], page(1..2, true, false, true));
    }

    #[test]
    fn sans_montants_pas_de_report() {
        let g = Gabarit {
            avec_report: false,
            ..gabarit()
        };
        let pages = paginer(&[20.0; 8], &g);
        assert_eq!(
            pages,
            [
                page(0..5, false, false, false),
                page(5..8, false, false, true)
            ]
        );
    }

    #[test]
    fn aucune_ligne_perdue_ni_dupliquee() {
        for n in 0..60 {
            let hauteurs: Vec<f32> = (0..n).map(|k| 12.0 + (k % 5) as f32 * 7.0).collect();
            let pages = paginer(&hauteurs, &gabarit());
            let mut attendu = 0;
            for p in &pages {
                assert_eq!(p.lignes.start, attendu);
                attendu = p.lignes.end;
            }
            assert_eq!(attendu, n);
            assert_eq!(pages.iter().filter(|p| p.bloc_final).count(), 1);
            assert!(pages.last().unwrap().bloc_final);
        }
    }
}
