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
/// non vide ; un saut de ligne forcé coupe la ligne).
fn xml_vers_markup(xml: &str) -> String {
    let mut lecteur = Reader::from_str(xml);
    let mut lignes: Vec<String> = Vec::new();
    let mut segments: Vec<Segment> = Vec::new();
    let mut centre = false;
    let mut dans_ppr = false;
    let mut dans_texte = false;
    let mut gras = false;

    let finir = |segments: &mut Vec<Segment>, centre: bool, lignes: &mut Vec<String>| {
        if let Some(l) = rendre_ligne(segments, centre) {
            lignes.push(l);
        }
        segments.clear();
    };

    loop {
        match lecteur.read_event() {
            Ok(Event::Start(e)) => match e.local_name().as_ref() {
                "p" => {
                    segments.clear();
                    centre = false;
                }
                "pPr" => dans_ppr = true,
                "r" => gras = false,
                "b" if !dans_ppr => gras = !coupe(&e),
                "jc" if dans_ppr => centre = valeur_val(&e).as_deref() == Some("center"),
                "t" => dans_texte = true,
                _ => {}
            },
            Ok(Event::Empty(e)) => match e.local_name().as_ref() {
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
                "p" => finir(&mut segments, centre, &mut lignes),
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

    #[test]
    fn le_pied_est_lu_et_un_document_sans_zones_donne_des_zones_vides() {
        let pied = r#"<w:ftr xmlns:w="w"><w:p><w:r><w:t>Page</w:t></w:r></w:p></w:ftr>"#;
        let z = lire_zones(&docx(None, Some(pied))).unwrap();
        assert_eq!((z.entete.as_str(), z.pied.as_str()), ("", "Page"));
        assert_eq!(lire_zones(&docx(None, None)).unwrap(), ZonesDocx::default());
    }

    #[test]
    fn un_fichier_qui_n_est_pas_un_docx_est_refuse() {
        assert!(lire_zones(b"pas une archive").is_err());
    }
}
