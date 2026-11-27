//! Reprise de l'en-tête et du pied de page d'un document Word (.docx) dans le
//! markup des zones d'impression de Kadenz (`kadenz_domaine::mise_en_page`).
//!
//! Un .docx est une archive ZIP dont `word/header*.xml` et `word/footer*.xml`
//! portent les zones. Chaque paragraphe devient une ligne : `**gras**` pour les
//! passages en gras, préfixe `->` pour un paragraphe centré. Les images (logo)
//! et les mises en forme autres que gras et centré ne sont pas reprises : le
//! texte arrive tel quel, sans variables, que l'utilisateur remplace ensuite par
//! `{nom}`, `{adresse}`, etc. Rien n'est écrit sur le disque.

use std::io::{Cursor, Read};

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

/// Taille maximale lue pour une partie de l'archive (protège d'une archive piégée).
const TAILLE_MAX_PARTIE: u64 = 1_000_000;

/// En-tête et pied repris d'un document, en markup de zone ; vide si le document
/// n'en a pas.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ZonesDocx {
    pub entete: String,
    pub pied: String,
}

/// Lit l'en-tête et le pied de page d'un .docx. Retient, pour chacun, la première
/// partie (`header1.xml`, puis `header2.xml`...) qui contient du texte.
pub fn lire_zones(octets: &[u8]) -> Result<ZonesDocx, String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(octets))
        .map_err(|_| "ce fichier n'est pas un document Word (.docx) lisible".to_string())?;
    let mut noms: Vec<String> = archive
        .file_names()
        .filter_map(Result::ok)
        .map(|n| n.into_owned())
        .filter(|n| n.starts_with("word/header") || n.starts_with("word/footer"))
        .filter(|n| n.ends_with(".xml"))
        .collect();
    noms.sort();
    let mut zones = ZonesDocx::default();
    for nom in noms {
        let cible = if nom.starts_with("word/header") {
            &mut zones.entete
        } else {
            &mut zones.pied
        };
        if !cible.is_empty() {
            continue;
        }
        let mut xml = String::new();
        let mut partie = archive
            .by_name(&nom)
            .map_err(|_| "document Word illisible".to_string())?
            .take(TAILLE_MAX_PARTIE);
        partie
            .read_to_string(&mut xml)
            .map_err(|_| "document Word illisible (texte non UTF-8)".to_string())?;
        *cible = xml_vers_markup(&xml);
    }
    Ok(zones)
}

/// Segment de texte d'une ligne, avec son gras.
struct Segment {
    texte: String,
    gras: bool,
}

/// Échappe les caractères qui ont un sens dans le markup des zones.
fn echapper(texte: &str) -> String {
    let mut sortie = String::with_capacity(texte.len());
    for c in texte.chars() {
        if matches!(c, '\\' | '*' | '{') {
            sortie.push('\\');
        }
        sortie.push(c);
    }
    sortie
}

/// Assemble les segments d'une ligne : les passages gras voisins sont fusionnés,
/// les blancs de bord ne sont jamais mis dans le gras.
fn rendre_ligne(segments: &[Segment], centre: bool) -> Option<String> {
    let mut fusion: Vec<Segment> = Vec::new();
    for s in segments {
        match fusion.last_mut() {
            Some(dernier) if dernier.gras == s.gras => dernier.texte.push_str(&s.texte),
            _ => fusion.push(Segment {
                texte: s.texte.clone(),
                gras: s.gras,
            }),
        }
    }
    let mut ligne = String::new();
    for s in &fusion {
        let nu = s.texte.trim();
        if nu.is_empty() {
            ligne.push_str(&echapper(&s.texte));
        } else if s.gras {
            let debut = &s.texte[..s.texte.len() - s.texte.trim_start().len()];
            let fin = &s.texte[s.texte.trim_end().len()..];
            ligne.push_str(&format!(
                "{}**{}**{}",
                echapper(debut),
                echapper(nu),
                echapper(fin)
            ));
        } else {
            ligne.push_str(&echapper(&s.texte));
        }
    }
    let ligne = ligne.trim().to_owned();
    if ligne.is_empty() {
        None
    } else if centre {
        Some(format!("-> {ligne}"))
    } else {
        Some(ligne)
    }
}

/// Vrai pour un attribut `w:val` qui coupe une propriété (`false`, `0`, `off`, `none`).
fn coupe(e: &BytesStart<'_>) -> bool {
    e.attributes().flatten().any(|a| {
        a.key.local_name().as_ref() == "val"
            && matches!(a.value.as_ref(), "false" | "0" | "off" | "none")
    })
}

fn valeur_val(e: &BytesStart<'_>) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.local_name().as_ref() == "val")
        .map(|a| a.value.into_owned())
}

/// Convertit le XML d'une partie Word en lignes de markup (une par paragraphe
/// non vide ; un saut de ligne forcé coupe la ligne). Une image du paragraphe
/// (logo) devient une ligne `{logo}`, puis `{logo2}` pour la suivante : l'image
/// du document n'est pas copiée, c'est le logo de l'entreprise qui s'imprime à
/// cette place. Au-delà de deux images, les suivantes sont ignorées.
fn xml_vers_markup(xml: &str) -> String {
    let mut lecteur = Reader::from_str(xml);
    let mut lignes: Vec<String> = Vec::new();
    let mut segments: Vec<Segment> = Vec::new();
    let mut centre = false;
    let mut dans_ppr = false;
    let mut dans_texte = false;
    let mut gras = false;
    let mut image = false;
    let mut logos = 0;
    // Profondeur dans `mc:Fallback` : Word y redit le contenu d'une zone de
    // texte ou d'une image (version de secours pour les anciens lecteurs) ;
    // la lire mettait chaque ligne en double.
    let mut secours = 0usize;

    let finir = |segments: &mut Vec<Segment>, centre: bool, lignes: &mut Vec<String>| {
        if let Some(l) = rendre_ligne(segments, centre) {
            lignes.push(l);
        }
        segments.clear();
    };

    loop {
        let evenement = lecteur.read_event();
        match &evenement {
            Ok(Event::Start(e)) if e.local_name().as_ref() == "Fallback" => {
                secours += 1;
                continue;
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == "Fallback" => {
                secours = secours.saturating_sub(1);
                continue;
            }
            Ok(Event::Eof) | Err(_) => {}
            _ if secours > 0 => continue,
            _ => {}
        }
        match evenement {
            Ok(Event::Start(e)) => match e.local_name().as_ref() {
                "p" => {
                    segments.clear();
                    centre = false;
                    image = false;
                }
                // `pict` double souvent `drawing` (contenu de secours) : un seul
                // repère par paragraphe suffit.
                "drawing" | "pict" => image = true,
                "pPr" => dans_ppr = true,
                "r" => gras = false,
                "b" if !dans_ppr => gras = !coupe(&e),
                "jc" if dans_ppr => centre = valeur_val(&e).as_deref() == Some("center"),
                "t" => dans_texte = true,
                _ => {}
            },
            Ok(Event::Empty(e)) => match e.local_name().as_ref() {
                "drawing" | "pict" => image = true,
                "b" if !dans_ppr => gras = !coupe(&e),
                "jc" if dans_ppr => centre = valeur_val(&e).as_deref() == Some("center"),
                "tab" => segments.push(Segment {
                    texte: " ".into(),
                    gras: false,
                }),
                "br" | "cr" => finir(&mut segments, centre, &mut lignes),
                _ => {}
            },
            Ok(Event::Text(t)) if dans_texte => {
                segments.push(Segment {
                    texte: t.xml10_content().into_owned(),
                    gras,
                });
            }
            Ok(Event::GeneralRef(r)) if dans_texte => {
                let texte = match r.as_ref() {
                    "amp" => "&",
                    "lt" => "<",
                    "gt" => ">",
                    "quot" => "\"",
                    "apos" => "'",
                    _ => "",
                };
                if !texte.is_empty() {
                    segments.push(Segment {
                        texte: texte.into(),
                        gras,
                    });
                }
            }
            Ok(Event::End(e)) => match e.local_name().as_ref() {
                "p" => {
                    if image && logos < 2 {
                        logos += 1;
                        let nom = if logos == 1 { "{logo}" } else { "{logo2}" };
                        lignes.push(if centre {
                            format!("-> {nom}")
                        } else {
                            nom.to_string()
                        });
                    }
                    image = false;
                    finir(&mut segments, centre, &mut lignes);
                }
                "pPr" => dans_ppr = false,
                "t" => dans_texte = false,
                _ => {}
            },
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    lignes.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn docx(entete: Option<&str>, pied: Option<&str>) -> Vec<u8> {
        let mut octets = Vec::new();
        {
            let mut z = zip::ZipWriter::new(Cursor::new(&mut octets));
            let options = zip::write::SimpleFileOptions::default();
            if let Some(x) = entete {
                z.start_file("word/header1.xml", options).unwrap();
                z.write_all(x.as_bytes()).unwrap();
            }
            if let Some(x) = pied {
                z.start_file("word/footer1.xml", options).unwrap();
                z.write_all(x.as_bytes()).unwrap();
            }
            z.start_file("word/document.xml", options).unwrap();
            z.write_all(b"<w:document/>").unwrap();
            z.finish().unwrap();
        }
        octets
    }

    const ENTETE: &str = r#"<w:hdr xmlns:w="w"><w:p><w:pPr><w:jc w:val="center"/><w:rPr><w:b/></w:rPr></w:pPr><w:r><w:rPr><w:b/></w:rPr><w:t>ACME </w:t></w:r><w:r><w:rPr><w:b/></w:rPr><w:t>SARL</w:t></w:r></w:p><w:p><w:r><w:t>Tél. : 70 00 00 00 *urgent* &amp; {x}</w:t></w:r></w:p><w:p/><w:p><w:r><w:t>Ligne</w:t><w:br/><w:t>suivante</w:t></w:r></w:p></w:hdr>"#;

    #[test]
    fn le_gras_le_centrage_et_les_caracteres_speciaux_sont_repris() {
        let z = lire_zones(&docx(Some(ENTETE), None)).unwrap();
        assert_eq!(
            z.entete,
            "-> **ACME SARL**\nTél. : 70 00 00 00 \\*urgent\\* & \\{x}\nLigne\nsuivante"
        );
        assert_eq!(z.pied, "");
    }

    /// Zone de texte flottante : Word l'écrit dans `mc:Choice` et la redit dans
    /// `mc:Fallback` ; elle n'est reprise qu'une fois.
    #[test]
    fn la_version_de_secours_de_word_n_est_pas_reprise() {
        let entete = r#"<w:hdr xmlns:w="w" xmlns:mc="mc"><w:p><w:r><mc:AlternateContent><mc:Choice Requires="wps"><w:drawing><w:txbxContent><w:p><w:r><w:t>MCARRE SARL</w:t></w:r></w:p></w:txbxContent></w:drawing></mc:Choice><mc:Fallback><w:pict><w:txbxContent><w:p><w:r><w:t>MCARRE SARL</w:t></w:r></w:p></w:txbxContent></w:pict></mc:Fallback></mc:AlternateContent></w:r></w:p><w:p><w:r><w:t>Ouagadougou</w:t></w:r></w:p></w:hdr>"#;
        let z = lire_zones(&docx(Some(entete), None)).unwrap();
        assert_eq!(z.entete.matches("MCARRE SARL").count(), 1, "{}", z.entete);
        assert!(z.entete.ends_with("Ouagadougou"), "{}", z.entete);
    }

    #[test]
    fn le_pied_est_lu_et_un_document_sans_zones_donne_des_zones_vides() {
        let pied = r#"<w:ftr xmlns:w="w"><w:p><w:r><w:t>Page</w:t></w:r></w:p></w:ftr>"#;
        let z = lire_zones(&docx(None, Some(pied))).unwrap();
        assert_eq!((z.entete.as_str(), z.pied.as_str()), ("", "Page"));
        assert_eq!(lire_zones(&docx(None, None)).unwrap(), ZonesDocx::default());
    }

    #[test]
    fn les_accents_et_symboles_francais_sont_conserves() {
        let entete = r#"<w:hdr xmlns:w="w"><w:p><w:pPr><w:jc w:val="center"/></w:pPr><w:r><w:rPr><w:b/></w:rPr><w:t>Établissements Ouédraogo &amp; Fils</w:t></w:r></w:p><w:p><w:r><w:t>Œuvre d'été « façon » n° 5 – 12 500 € à Ouagadougou, près d'Élie</w:t></w:r></w:p></w:hdr>"#;
        let z = lire_zones(&docx(Some(entete), None)).unwrap();
        assert_eq!(
            z.entete,
            "-> **Établissements Ouédraogo & Fils**\nŒuvre d'été « façon » n° 5 – 12 500 € à Ouagadougou, près d'Élie"
        );
    }

    #[test]
    fn une_image_devient_le_logo_de_l_entreprise() {
        let entete = r#"<w:hdr xmlns:w="w"><w:p><w:r><w:drawing><a:blip/></w:drawing></w:r></w:p><w:p><w:pPr><w:jc w:val="center"/></w:pPr><w:r><w:pict/></w:r><w:r><w:t>ACME</w:t></w:r></w:p><w:p><w:r><w:drawing/></w:r></w:p><w:p><w:r><w:t>fin</w:t></w:r></w:p></w:hdr>"#;
        let z = lire_zones(&docx(Some(entete), None)).unwrap();
        // Troisième image ignorée ; le logo reste seul sur sa ligne, avant le texte.
        assert_eq!(
            z.entete,
            "{logo}
-> {logo2}
-> ACME
fin"
        );
        // Le markup produit est accepté par les zones d'impression.
        for ligne in z.entete.lines() {
            assert!(!ligne.is_empty());
        }
    }

    #[test]
    fn un_fichier_qui_n_est_pas_un_docx_est_refuse() {
        assert!(lire_zones(b"pas une archive").is_err());
    }
}
