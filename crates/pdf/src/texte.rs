//! Polices, façonnage et mesure du texte.
//!
//! Le même façonnage (rustybuzz, version utilisée par krilla) sert à mesurer puis à
//! tracer : une ligne mesurée est tracée avec exactement les glyphes mesurés, ce qui
//! garantit des retours à la ligne et une pagination justes.

use krilla::text::{Font, GlyphId, KrillaGlyph};

/// Une police embarquée, prête à mesurer et à tracer.
pub struct Police {
    pub(crate) font: Font,
    face: rustybuzz::Face<'static>,
    unites_par_em: f32,
    /// Hauteur de la ligne de base au sommet des capitales, en em.
    pub(crate) hauteur_capitales: f32,
}

impl Police {
    pub fn depuis_octets(donnees: &'static [u8]) -> Option<Police> {
        let font = Font::new(donnees.into(), 0)?;
        let face = rustybuzz::Face::from_slice(donnees, 0)?;
        let unites_par_em = f32::from(face.units_per_em() as u16);
        let hauteur_capitales = face
            .capital_height()
            .map(|h| f32::from(h) / unites_par_em)
            .unwrap_or(0.7);
        Some(Police {
            font,
            face,
            unites_par_em,
            hauteur_capitales,
        })
    }

    /// Glyphes d'une ligne (gauche à droite) et sa chasse en em.
    pub(crate) fn faconner(&self, texte: &str) -> (Vec<KrillaGlyph>, f32) {
        let mut tampon = rustybuzz::UnicodeBuffer::new();
        tampon.push_str(texte);
        tampon.set_direction(rustybuzz::Direction::LeftToRight);
        tampon.guess_segment_properties();
        let sortie = rustybuzz::shape(&self.face, &[], tampon);
        let infos = sortie.glyph_infos();
        let positions = sortie.glyph_positions();
        let upem = self.unites_par_em;
        let mut glyphes = Vec::with_capacity(infos.len());
        let mut chasse = 0.0;
        for (i, (info, pos)) in infos.iter().zip(positions).enumerate() {
            let debut = info.cluster as usize;
            let fin = infos[i + 1..]
                .iter()
                .find(|suivant| suivant.cluster != info.cluster)
                .map_or(texte.len(), |suivant| suivant.cluster as usize);
            let avance = pos.x_advance as f32 / upem;
            chasse += avance;
            glyphes.push(KrillaGlyph::new(
                GlyphId::new(info.glyph_id),
                avance,
                pos.x_offset as f32 / upem,
                pos.y_offset as f32 / upem,
                pos.y_advance as f32 / upem,
                debut..fin,
                None,
            ));
        }
        (glyphes, chasse)
    }

    /// Largeur d'un texte sur une ligne, en points.
    pub fn largeur(&self, texte: &str, taille: f32) -> f32 {
        self.faconner(texte).1 * taille
    }

    /// La police contient-elle un glyphe pour ce caractère ?
    pub fn couvre(&self, c: char) -> bool {
        self.face.glyph_index(c).is_some()
    }

    /// Coupe un texte en lignes d'au plus `largeur_max` points. Les retours à la
    /// ligne du texte sont conservés ; un mot plus long que la ligne est coupé.
    pub fn couper(&self, texte: &str, taille: f32, largeur_max: f32) -> Vec<String> {
        let mut lignes = Vec::new();
        for paragraphe in texte.lines() {
            let mut courante = String::new();
            for mot in paragraphe.split(' ').filter(|m| !m.is_empty()) {
                let essai = if courante.is_empty() {
                    mot.to_string()
                } else {
                    format!("{courante} {mot}")
                };
                if self.largeur(&essai, taille) <= largeur_max {
                    courante = essai;
                    continue;
                }
                if !courante.is_empty() {
                    lignes.push(std::mem::take(&mut courante));
                }
                // Mot trop long à lui seul : coupé caractère par caractère.
                for c in mot.chars() {
                    let mut essai = courante.clone();
                    essai.push(c);
                    if !courante.is_empty() && self.largeur(&essai, taille) > largeur_max {
                        lignes.push(std::mem::take(&mut courante));
                        courante.push(c);
                    } else {
                        courante = essai;
                    }
                }
            }
            lignes.push(courante);
        }
        if lignes.is_empty() {
            lignes.push(String::new());
        }
        lignes
    }
}

/// Les polices de la marque (SIL Open Font License 1.1), embarquées.
pub struct Polices {
    pub texte: Police,
    pub texte_moyen: Police,
    pub texte_gras: Police,
    pub titre: Police,
}

impl Polices {
    pub fn embarquees() -> Polices {
        let charger = |octets: &'static [u8]| {
            Police::depuis_octets(octets).expect("police embarquée lisible")
        };
        Polices {
            texte: charger(include_bytes!("../../app/ui/polices/Inter-Regular.ttf")),
            texte_moyen: charger(include_bytes!("../../app/ui/polices/Inter-Medium.ttf")),
            texte_gras: charger(include_bytes!("../../app/ui/polices/Inter-SemiBold.ttf")),
            titre: charger(include_bytes!("../../app/ui/polices/Manrope-ExtraBold.ttf")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesure_et_coupure() {
        let p = Polices::embarquees();
        let t = &p.texte;
        assert!(t.couvre('\u{00A0}'), "espace insécable des montants");
        assert!(t.couvre('é') && t.couvre('À') && t.couvre('°'));
        let court = t.largeur("Pelle", 10.0);
        let long = t.largeur("Pelle Pelle", 10.0);
        assert!(court > 0.0 && long > 2.0 * court);
        assert_eq!(t.largeur("Pelle", 20.0), 2.0 * court);

        let lignes = t.couper("Brouette renforcée à roue gonflable", 10.0, 80.0);
        assert!(lignes.len() > 1, "{lignes:?}");
        assert!(lignes.iter().all(|l| t.largeur(l, 10.0) <= 80.0));
        assert_eq!(lignes.join(" "), "Brouette renforcée à roue gonflable");

        let coupe = t.couper("ABCDEFGHIJKLMNOPQRSTUVWXYZ", 10.0, 40.0);
        assert!(coupe.len() > 1);
        assert_eq!(coupe.concat(), "ABCDEFGHIJKLMNOPQRSTUVWXYZ");

        assert_eq!(t.couper("un\ndeux", 10.0, 500.0), ["un", "deux"]);
        assert_eq!(t.couper("", 10.0, 500.0), [""]);
    }
}
