//! Géométrie des graphiques du tableau de bord (section 9). Les commandes de
//! chemin sont calculées dans un repère fixe ; l'élément `Path` de Slint les
//! ajuste ensuite à sa taille à l'écran, la largeur du trait restant en pixels.

use std::f64::consts::PI;
use std::fmt::Write;

/// Côté du repère de l'anneau : carré de 100, centre (50, 50), rayon 50.
pub(crate) const COTE_ANNEAU: f64 = 100.0;
/// Repère de la courbe : 300 de large, 100 de haut, origine en haut à gauche.
pub(crate) const LARGEUR_COURBE: f64 = 300.0;
pub(crate) const HAUTEUR_COURBE: f64 = 100.0;

fn point_du_cercle(angle: f64) -> (f64, f64) {
    let r = COTE_ANNEAU / 2.0;
    (r + r * angle.cos(), r + r * angle.sin())
}

/// Un arc de l'anneau par valeur, dans l'ordre, en partant du haut et dans le
/// sens des aiguilles d'une montre. Une valeur nulle ou négative donne une
/// commande vide ; un total nul ne donne que des commandes vides.
pub(crate) fn arcs_anneau(valeurs: &[i64]) -> Vec<String> {
    let total: i64 = valeurs.iter().map(|v| (*v).max(0)).sum();
    if total == 0 {
        return vec![String::new(); valeurs.len()];
    }
    let r = COTE_ANNEAU / 2.0;
    let mut depart = -PI / 2.0;
    valeurs
        .iter()
        .map(|v| {
            let v = (*v).max(0);
            if v == 0 {
                return String::new();
            }
            if v == total {
                // Un arc qui revient à son point de départ ne trace rien :
                // le cercle complet est fait de deux demi-cercles.
                return format!(
                    "M {r} 0 A {r} {r} 0 0 1 {r} {c} A {r} {r} 0 0 1 {r} 0",
                    c = COTE_ANNEAU
                );
            }
            let ouverture = 2.0 * PI * v as f64 / total as f64;
            let (x0, y0) = point_du_cercle(depart);
            let (x1, y1) = point_du_cercle(depart + ouverture);
            depart += ouverture;
            let grand = u8::from(ouverture > PI);
            format!("M {x0:.3} {y0:.3} A {r} {r} 0 {grand} 1 {x1:.3} {y1:.3}")
        })
        .collect()
}

/// Courbe d'une série dans le repère 300 x 100.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Courbe {
    /// Trait reliant les points ; vide s'il y a moins de deux points.
    pub trace: String,
    /// Surface sous le trait, fermée sur l'axe du bas ; vide comme le trait.
    pub aire: String,
    /// Position de chaque point, en fraction de la largeur et de la hauteur.
    pub points: Vec<(f32, f32)>,
}

/// Les points sont répartis à intervalle égal sur la largeur (un point seul au
/// milieu) ; le plus haut touche le bord supérieur, zéro est sur le bord du bas.
pub(crate) fn courbe(valeurs: &[i64]) -> Courbe {
    let n = valeurs.len();
    if n == 0 {
        return Courbe::default();
    }
    let max = valeurs.iter().copied().max().unwrap_or(0).max(0);
    let points: Vec<(f64, f64)> = valeurs
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let x = if n == 1 {
                LARGEUR_COURBE / 2.0
            } else {
                LARGEUR_COURBE * i as f64 / (n - 1) as f64
            };
            let y = if max == 0 {
                HAUTEUR_COURBE
            } else {
                HAUTEUR_COURBE * (1.0 - (*v).max(0) as f64 / max as f64)
            };
            (x, y)
        })
        .collect();
    let mut trace = String::new();
    let mut aire = String::new();
    if n >= 2 {
        for (i, (x, y)) in points.iter().enumerate() {
            let lettre = if i == 0 { 'M' } else { 'L' };
            let _ = write!(trace, "{lettre} {x:.3} {y:.3} ");
        }
        let _ = write!(aire, "M 0 {HAUTEUR_COURBE} ");
        for (x, y) in &points {
            let _ = write!(aire, "L {x:.3} {y:.3} ");
        }
        let _ = write!(aire, "L {LARGEUR_COURBE} {HAUTEUR_COURBE} Z");
    }
    Courbe {
        trace: trace.trim_end().to_string(),
        aire,
        points: points
            .iter()
            .map(|(x, y)| ((x / LARGEUR_COURBE) as f32, (y / HAUTEUR_COURBE) as f32))
            .collect(),
    }
}

/// Longueur de chaque barre, en fraction de la plus grande valeur (0 si toutes
/// sont nulles ou négatives).
pub(crate) fn proportions(valeurs: &[i64]) -> Vec<f32> {
    let max = valeurs.iter().copied().max().unwrap_or(0);
    valeurs
        .iter()
        .map(|v| {
            if max <= 0 {
                0.0
            } else {
                ((*v).max(0) as f64 / max as f64) as f32
            }
        })
        .collect()
}

/// Part de chaque valeur dans le total, en pour cent arrondi (0 si le total est nul).
pub(crate) fn parts(valeurs: &[i64]) -> Vec<u32> {
    let total: i64 = valeurs.iter().map(|v| (*v).max(0)).sum();
    valeurs
        .iter()
        .map(|v| {
            if total == 0 {
                0
            } else {
                ((*v).max(0) as f64 * 100.0 / total as f64).round() as u32
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anneau_vide_sans_total() {
        assert_eq!(arcs_anneau(&[0, 0]), vec![String::new(), String::new()]);
        assert!(arcs_anneau(&[]).is_empty());
    }

    #[test]
    fn anneau_complet_en_deux_demi_cercles() {
        let arcs = arcs_anneau(&[0, 500, 0]);
        assert_eq!(arcs[0], "");
        assert_eq!(arcs[1], "M 50 0 A 50 50 0 0 1 50 100 A 50 50 0 0 1 50 0");
        assert_eq!(arcs[2], "");
    }

    #[test]
    fn anneau_partage_en_quarts() {
        let arcs = arcs_anneau(&[1, 3]);
        // Un quart depuis le haut, jusqu'à droite, petit arc.
        assert_eq!(arcs[0], "M 50.000 0.000 A 50 50 0 0 1 100.000 50.000");
        // Les trois quarts restants, grand arc, retour en haut.
        assert_eq!(arcs[1], "M 100.000 50.000 A 50 50 0 1 1 50.000 0.000");
    }

    #[test]
    fn anneau_ignore_les_negatifs() {
        let arcs = arcs_anneau(&[-5, 10]);
        assert_eq!(arcs[0], "");
        assert!(arcs[1].starts_with("M 50 0 A"));
    }

    #[test]
    fn courbe_vide_ou_point_seul() {
        assert_eq!(courbe(&[]), Courbe::default());
        let c = courbe(&[42]);
        assert_eq!(c.trace, "");
        assert_eq!(c.aire, "");
        assert_eq!(c.points, vec![(0.5, 0.0)]);
    }

    #[test]
    fn courbe_de_trois_points() {
        let c = courbe(&[0, 100, 50]);
        assert_eq!(c.trace, "M 0.000 100.000 L 150.000 0.000 L 300.000 50.000");
        assert_eq!(
            c.aire,
            "M 0 100 L 0.000 100.000 L 150.000 0.000 L 300.000 50.000 L 300 100 Z"
        );
        assert_eq!(c.points, vec![(0.0, 1.0), (0.5, 0.0), (1.0, 0.5)]);
    }

    #[test]
    fn courbe_toute_nulle_sur_l_axe() {
        let c = courbe(&[0, 0]);
        assert_eq!(c.points, vec![(0.0, 1.0), (1.0, 1.0)]);
    }

    #[test]
    fn proportions_et_parts() {
        assert_eq!(proportions(&[50, 100, 0]), vec![0.5, 1.0, 0.0]);
        assert_eq!(proportions(&[0, 0]), vec![0.0, 0.0]);
        assert_eq!(parts(&[1, 2]), vec![33, 67]);
        assert_eq!(parts(&[0, 0]), vec![0, 0]);
    }
}
