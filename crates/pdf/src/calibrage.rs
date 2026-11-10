//! Feuille de calibrage des marges (section 12, jalon J8b) : un cadre sur la
//! zone imprimable et une croix à chaque angle, pour ajuster les marges au
//! papier pré-imprimé et à l'imprimante.

use kadenz_domaine::mise_en_page::Marges;
use krilla::Document;
use krilla::metadata::Metadata;
use krilla::page::PageSettings;

use crate::rendu::{BORDURE, Cadre, ENCRE, HAUTEUR, LARGEUR, SECONDAIRE, Toile, base_centree};
use crate::texte::Polices;

/// Demi-longueur des branches d'une croix, en points.
const BRANCHE: f32 = 12.0;

/// Produit la feuille de calibrage pour les marges données.
pub fn generer_calibrage(marges: &Marges, polices: &Polices) -> Result<Vec<u8>, String> {
    let g = Cadre::depuis(marges);
    let mut document = Document::new();
    document.set_metadata(
        Metadata::new()
            .title("Feuille de calibrage".into())
            .language("fr".into()),
    );
    let reglages = PageSettings::from_wh(LARGEUR, HAUTEUR).ok_or("format de page invalide")?;
    let mut page = document.start_page_with(reglages);
    let mut surface = page.surface();
    let mut t = Toile {
        s: Some(&mut surface),
    };

    t.cadre(g.gauche, g.haut, g.utile(), g.bas - g.haut, ENCRE);
    for (x, y) in [
        (g.gauche, g.haut),
        (g.droite, g.haut),
        (g.gauche, g.bas),
        (g.droite, g.bas),
    ] {
        t.trait_horizontal(x - BRANCHE, x + BRANCHE, y, ENCRE, 0.5);
        t.trait_vertical(x, y - BRANCHE, y + BRANCHE, ENCRE, 0.5);
    }
    // Repères au milieu de chaque bord, pour vérifier le centrage.
    let (cx, cy) = ((g.gauche + g.droite) / 2.0, (g.haut + g.bas) / 2.0);
    t.trait_vertical(cx, g.haut, g.haut + BRANCHE, BORDURE, 0.5);
    t.trait_vertical(cx, g.bas - BRANCHE, g.bas, BORDURE, 0.5);
    t.trait_horizontal(g.gauche, g.gauche + BRANCHE, cy, BORDURE, 0.5);
    t.trait_horizontal(g.droite - BRANCHE, g.droite, cy, BORDURE, 0.5);

    let lignes = [
        (
            "Feuille de calibrage".to_owned(),
            &polices.titre,
            16.0,
            ENCRE,
        ),
        (
            format!(
                "Marges : haut {} mm, bas {} mm, gauche {} mm, droite {} mm",
                marges.haut, marges.bas, marges.gauche, marges.droite
            ),
            &polices.texte_moyen,
            10.0,
            ENCRE,
        ),
        (
            "Le cadre marque la zone où s'imprime le document ; les croix en marquent les angles."
                .to_owned(),
            &polices.texte,
            9.0,
            SECONDAIRE,
        ),
        (
            "Ajuster les marges jusqu'à ce que le cadre tombe dans la zone libre du papier."
                .to_owned(),
            &polices.texte,
            9.0,
            SECONDAIRE,
        ),
    ];
    let mut y = cy - 40.0;
    for (texte, police, taille, c) in &lignes {
        let l = police.largeur(texte, *taille);
        t.ecrire(
            police,
            *taille,
            *c,
            cx - l / 2.0,
            base_centree(police, *taille, y, 20.0),
            texte,
        );
        y += 20.0;
    }

    surface.finish();
    page.finish();
    document.finish().map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feuille_produite() {
        let octets = generer_calibrage(&Marges::default(), &Polices::embarquees()).unwrap();
        assert!(octets.starts_with(b"%PDF"));
    }
}
