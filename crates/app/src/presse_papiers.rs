//! Image du presse-papiers pour l'en-tête ou le pied de page des documents.
//!
//! Quand on copie du texte dans Word, Word dépose aussi une image vectorielle
//! de la sélection (métafichier amélioré, EMF), telle qu'il l'affiche : zones
//! de texte flottantes, images, polices et positions comprises. Elle est
//! rendue ici en PNG à la résolution d'impression, sur fond blanc, marges
//! blanches retirées. À défaut d'EMF, une image PNG copiée est reprise telle
//! quelle.

/// Image du presse-papiers en PNG, au plus `taille_max` octets.
#[cfg(windows)]
pub(crate) fn image_png(taille_max: usize) -> Result<Vec<u8>, String> {
    windows::image_png(taille_max)
}

#[cfg(not(windows))]
pub(crate) fn image_png(_taille_max: usize) -> Result<Vec<u8>, String> {
    Err("le presse-papiers n'est lu que sous Windows".into())
}

/// Pixels BGRA (ligne par ligne, de haut en bas) d'une image sur fond blanc.
struct Pixels {
    largeur: usize,
    hauteur: usize,
    bgra: Vec<u8>,
}

/// Seuil au-dessus duquel un pixel compte comme blanc (anticrénelage compris).
const BLANC: u8 = 250;

impl Pixels {
    fn blanc(&self, x: usize, y: usize) -> bool {
        let i = (y * self.largeur + x) * 4;
        self.bgra[i] >= BLANC && self.bgra[i + 1] >= BLANC && self.bgra[i + 2] >= BLANC
    }

    /// Rectangle utile `(x0, y0, x1, y1)`, bornes exclues à droite et en bas ;
    /// `None` si l'image est toute blanche.
    fn cadre_utile(&self) -> Option<(usize, usize, usize, usize)> {
        let ligne_blanche = |y: usize| (0..self.largeur).all(|x| self.blanc(x, y));
        let colonne_blanche = |x: usize, y0: usize, y1: usize| (y0..y1).all(|y| self.blanc(x, y));
        let y0 = (0..self.hauteur).find(|&y| !ligne_blanche(y))?;
        let y1 = (0..self.hauteur).rev().find(|&y| !ligne_blanche(y))? + 1;
        let x0 = (0..self.largeur).find(|&x| !colonne_blanche(x, y0, y1))?;
        let x1 = (0..self.largeur)
            .rev()
            .find(|&x| !colonne_blanche(x, y0, y1))?
            + 1;
        Some((x0, y0, x1, y1))
    }

    /// PNG en couleurs (RGB) du rectangle utile, avec une petite marge blanche.
    fn en_png(&self) -> Result<Vec<u8>, String> {
        let (x0, y0, x1, y1) = self.cadre_utile().ok_or("l'image copiée est vide")?;
        let marge = 4;
        let (l, h) = (x1 - x0 + 2 * marge, y1 - y0 + 2 * marge);
        let mut rgb = vec![255u8; l * h * 3];
        for y in y0..y1 {
            for x in x0..x1 {
                let s = (y * self.largeur + x) * 4;
                let d = ((y - y0 + marge) * l + (x - x0 + marge)) * 3;
                rgb[d] = self.bgra[s + 2];
                rgb[d + 1] = self.bgra[s + 1];
                rgb[d + 2] = self.bgra[s];
            }
        }
        let mut sortie = Vec::new();
        {
            let mut e = png::Encoder::new(&mut sortie, l as u32, h as u32);
            e.set_color(png::ColorType::Rgb);
            e.set_depth(png::BitDepth::Eight);
            e.set_compression(png::Compression::High);
            let mut w = e.write_header().map_err(|e| e.to_string())?;
            w.write_image_data(&rgb).map_err(|e| e.to_string())?;
        }
        Ok(sortie)
    }
}

#[cfg(windows)]
mod windows {
    use super::Pixels;
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CopyEnhMetaFileW, CreateCompatibleDC,
        CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteEnhMetaFile, DeleteObject, ENHMETAHEADER,
        GdiFlush, GetEnhMetaFileHeader, HENHMETAFILE, PlayEnhMetaFile, SelectObject,
    };
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, OpenClipboard, RegisterClipboardFormatW,
    };
    use windows_sys::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};

    /// Format standard du métafichier amélioré.
    const CF_ENHMETAFILE: u32 = 14;
    /// Résolutions essayées, de la meilleure à la plus légère, jusqu'à tenir
    /// sous la taille maximale.
    const RESOLUTIONS: [f64; 4] = [300.0, 220.0, 160.0, 110.0];
    /// Largeur maximale du rendu, en pixels (A4 paysage à 300 points par pouce).
    const LARGEUR_MAX: f64 = 3600.0;

    /// Ce que contient le presse-papiers, copié avant de le refermer.
    enum Contenu {
        Emf(HENHMETAFILE),
        Png(Vec<u8>),
    }

    pub(super) fn image_png(taille_max: usize) -> Result<Vec<u8>, String> {
        match lire()? {
            Contenu::Emf(emf) => {
                let resultat = rendre_emf(emf, taille_max);
                // SAFETY : copie possédée, créée par `lire`.
                unsafe { DeleteEnhMetaFile(emf) };
                resultat
            }
            Contenu::Png(octets) if octets.len() <= taille_max => Ok(octets),
            Contenu::Png(_) => Err(
                "l'image copiée est trop lourde : copiez plutôt le texte de l'en-tête dans Word"
                    .into(),
            ),
        }
    }

    fn lire() -> Result<Contenu, String> {
        // SAFETY : presse-papiers ouvert puis refermé dans ce bloc ; les poignées
        // qu'il prête ne servent qu'avant la fermeture (l'EMF est copié).
        unsafe {
            if OpenClipboard(null_mut()) == 0 {
                return Err("le presse-papiers est occupé : réessayez".into());
            }
            let resultat = (|| {
                let h = GetClipboardData(CF_ENHMETAFILE);
                if !h.is_null() {
                    let copie = CopyEnhMetaFileW(h as HENHMETAFILE, std::ptr::null());
                    if !copie.is_null() {
                        return Ok(Contenu::Emf(copie));
                    }
                }
                let nom: Vec<u16> = "PNG\0".encode_utf16().collect();
                let format = RegisterClipboardFormatW(nom.as_ptr());
                if format != 0 {
                    let h = GetClipboardData(format);
                    if !h.is_null() {
                        let taille = GlobalSize(h);
                        let p = GlobalLock(h) as *const u8;
                        if !p.is_null() && taille > 0 {
                            let octets = std::slice::from_raw_parts(p, taille).to_vec();
                            GlobalUnlock(h);
                            return Ok(Contenu::Png(octets));
                        }
                    }
                }
                Err("le presse-papiers ne contient pas de copie de Word : sélectionnez l'en-tête dans Word, copiez-le, puis recommencez".to_string())
            })();
            CloseClipboard();
            resultat
        }
    }

    fn rendre_emf(emf: HENHMETAFILE, taille_max: usize) -> Result<Vec<u8>, String> {
        // SAFETY : en-tête lu dans une structure de la bonne taille.
        let entete = unsafe {
            let mut e: ENHMETAHEADER = std::mem::zeroed();
            let lu = GetEnhMetaFileHeader(emf, size_of::<ENHMETAHEADER>() as u32, &mut e);
            if lu == 0 {
                return Err("l'image copiée est illisible".into());
            }
            e
        };
        // Cadre en centièmes de millimètre.
        let cadre = entete.rclFrame;
        let (l_mm, h_mm) = (
            (cadre.right - cadre.left) as f64 / 100.0,
            (cadre.bottom - cadre.top) as f64 / 100.0,
        );
        if l_mm <= 0.0 || h_mm <= 0.0 {
            return Err("l'image copiée est vide".into());
        }
        let mut derniere = String::new();
        for dpi in RESOLUTIONS {
            let echelle = (dpi / 25.4).min(LARGEUR_MAX / l_mm);
            let (l, h) = (
                (l_mm * echelle).ceil() as usize,
                (h_mm * echelle).ceil() as usize,
            );
            let png = rendre(emf, l.max(1), h.max(1))?.en_png()?;
            if png.len() <= taille_max {
                return Ok(png);
            }
            derniere = format!("{} Ko", png.len() / 1024);
        }
        Err(format!(
            "l'image copiée reste trop lourde ({derniere}) : copiez une zone plus petite"
        ))
    }

    /// Joue le métafichier dans une image blanche de `l` sur `h` pixels.
    fn rendre(emf: HENHMETAFILE, l: usize, h: usize) -> Result<Pixels, String> {
        // SAFETY : contexte, image et sélection créés et libérés dans ce bloc ;
        // les pixels sont recopiés avant la libération de l'image.
        unsafe {
            let dc = CreateCompatibleDC(null_mut());
            if dc.is_null() {
                return Err("rendu de l'image impossible".into());
            }
            let mut info: BITMAPINFO = std::mem::zeroed();
            info.bmiHeader = BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: l as i32,
                // Négatif : lignes de haut en bas.
                biHeight: -(h as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..std::mem::zeroed()
            };
            let mut bits: *mut core::ffi::c_void = null_mut();
            let image = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
            if image.is_null() || bits.is_null() {
                DeleteDC(dc);
                return Err("rendu de l'image impossible : mémoire insuffisante".into());
            }
            let ancienne = SelectObject(dc, image);
            let taille = l * h * 4;
            std::ptr::write_bytes(bits as *mut u8, 0xFF, taille);
            let cadre = RECT {
                left: 0,
                top: 0,
                right: l as i32,
                bottom: h as i32,
            };
            PlayEnhMetaFile(dc, emf, &cadre);
            GdiFlush();
            let bgra = std::slice::from_raw_parts(bits as *const u8, taille).to_vec();
            SelectObject(dc, ancienne);
            DeleteObject(image);
            DeleteDC(dc);
            Ok(Pixels {
                largeur: l,
                hauteur: h,
                bgra,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(largeur: usize, hauteur: usize, noirs: &[(usize, usize)]) -> Pixels {
        let mut bgra = vec![255u8; largeur * hauteur * 4];
        for &(x, y) in noirs {
            let i = (y * largeur + x) * 4;
            bgra[i..i + 3].copy_from_slice(&[0, 0, 0]);
        }
        Pixels {
            largeur,
            hauteur,
            bgra,
        }
    }

    /// Essai manuel : copier un en-tête dans Word, puis lancer ce test avec
    /// `KADENZ_ESSAI_PNG` (chemin du PNG à écrire).
    #[test]
    #[ignore]
    fn rendu_du_presse_papiers() {
        let chemin = std::env::var("KADENZ_ESSAI_PNG").unwrap();
        let png = image_png(768 * 1024).unwrap();
        std::fs::write(chemin, &png).unwrap();
    }

    #[test]
    fn les_marges_blanches_sont_retirees() {
        let p = image(20, 10, &[(3, 2), (15, 7)]);
        assert_eq!(p.cadre_utile(), Some((3, 2, 16, 8)));
        assert_eq!(image(5, 5, &[]).cadre_utile(), None);
        let png = p.en_png().unwrap();
        assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
        // 13 sur 6 pixels utiles, plus 4 pixels de marge de chaque côté.
        let decodeur = png::Decoder::new(std::io::Cursor::new(png));
        let lecteur = decodeur.read_info().unwrap();
        assert_eq!((lecteur.info().width, lecteur.info().height), (21, 14));
    }
}
