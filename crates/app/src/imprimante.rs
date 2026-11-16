//! Impression d'un document (docs/spec-apercu-impression.md, jalon A2).
//!
//! La fenêtre d'impression de Windows (`PrintDlgExW`) s'ouvre sur le fil de
//! l'interface et rend un `Travail` qui ne possède que des données Rust ;
//! `envoyer` l'imprime ensuite dans un fil de travail : chaque page est rendue
//! par `hayro` sur fond blanc, en bandes, et posée sur la feuille par
//! `StretchDIBits`. Les réglages choisis (imprimante et propriétés) sont gardés
//! jusqu'à la fermeture de Kadenz, jamais sur le disque.

use std::cell::RefCell;
use std::mem::{offset_of, size_of};
use std::path::Path;
use std::ptr::{null, null_mut};

use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_interpret::util::TransformExt;
use hayro::hayro_syntax::Pdf;
use hayro::kurbo::Affine;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::vello_cpu::{Pixmap, RasterizerSettings, RenderContext, Resources, TargetInit};
use hayro::{RenderCache, RenderSettings, render_into};
use kadenz_domaine::mise_en_page::Marges;
use windows_sys::Win32::Foundation::{GetLastError, GlobalFree, HGLOBAL, HWND, S_FALSE, S_OK};
use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateDCW, DEVMODEW, DEVMODEW_0_0, DIB_RGB_COLORS,
    DM_COLLATE, DM_COPIES, DMCOLLATE_TRUE, DeleteDC, GDI_ERROR, GetDeviceCaps, HALFTONE, HDC,
    HORZRES, LOGPIXELSX, LOGPIXELSY, PHYSICALOFFSETX, PHYSICALOFFSETY, SRCCOPY, SetBrushOrgEx,
    SetStretchBltMode, StretchDIBits, VERTRES,
};
use windows_sys::Win32::Storage::Xps::{
    AbortDoc, DC_COPIES, DOCINFOW, DeviceCapabilitiesW, EndDoc, EndPage, StartDocW, StartPage,
};
use windows_sys::Win32::System::Com::{
    COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize,
};
use windows_sys::Win32::System::Memory::{
    GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock,
};
use windows_sys::Win32::UI::Controls::Dialogs::{
    PD_NOCURRENTPAGE, PD_NOSELECTION, PD_PAGENUMS, PD_RESULT_APPLY, PD_RESULT_PRINT,
    PD_USEDEVMODECOPIESANDCOLLATE, PRINTDLGEXW, PRINTPAGERANGE, PrintDlgExW, START_PAGE_GENERAL,
};

use crate::traduction::trf;

/// Résolution maximale du rendu envoyé à l'imprimante, en points par pouce.
const PPP_MAX: f64 = 300.0;

/// Bandes horizontales par page : certains pilotes refusent une très grande
/// image d'un coup.
const BANDES: u32 = 8;

/// Plages de pages que la fenêtre d'impression peut rendre.
const PLAGES_MAX: usize = 32;

/// Réglages de la dernière fenêtre d'impression : blocs `DEVMODE` et `DEVNAMES`.
struct Reglages {
    devmode: Vec<u8>,
    devnames: Vec<u8>,
}

thread_local! {
    static REGLAGES: RefCell<Option<Reglages>> = const { RefCell::new(None) };
}

/// Travail prêt à envoyer : imprimante, propriétés et pages dans l'ordre
/// d'envoi, copies comprises quand le pilote ne les fait pas lui-même.
pub(crate) struct Travail {
    /// Nom de l'imprimante, terminé par un zéro.
    pub(crate) imprimante: Vec<u16>,
    pub(crate) devmode: Option<Vec<u8>>,
    /// Rangs des pages (à partir de 0).
    pub(crate) pages: Vec<usize>,
}

impl Travail {
    pub(crate) fn nom_imprimante(&self) -> String {
        let fin = self
            .imprimante
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(self.imprimante.len());
        String::from_utf16_lossy(&self.imprimante[..fin])
    }
}

pub(crate) enum Choix {
    Imprimer(Travail),
    /// Annulée, ou « Appliquer » sans imprimer.
    Rien,
}

/// Échec de l'envoi, traduit sur le fil de l'interface.
#[derive(Debug)]
pub(crate) enum Echec {
    Lecture,
    /// Imprimante qui ne s'ouvre pas, avec le code d'erreur de Windows.
    Imprimante(u32),
    /// Document refusé à l'ouverture (`StartDocW`) ou à la fin (`EndDoc`), avec
    /// le code d'erreur de Windows (21 : imprimante non activée, comme celle de
    /// Sage hors de Sage).
    Document(u32),
    Page(usize),
}

/// Ouvre la fenêtre d'impression de Windows pour un document de `nombre_pages`
/// pages, avec `proprietaire` comme fenêtre propriétaire.
pub(crate) fn choisir(proprietaire: HWND, nombre_pages: usize) -> Result<Choix, String> {
    // La fenêtre d'impression exige une fenêtre propriétaire valide.
    if proprietaire.is_null() {
        return Err(trf(
            "fenêtre d'impression indisponible (code {})",
            &[&"HWND"],
        ));
    }
    let nombre = u32::try_from(nombre_pages.max(1)).unwrap_or(u32::MAX);
    let mut plages = [PRINTPAGERANGE {
        nFromPage: 1,
        nToPage: nombre,
    }; PLAGES_MAX];
    // SAFETY : structure C remplie à zéro puis renseignée ; les pointeurs
    // (plages, blocs mémoire) restent valides pendant l'appel.
    let mut pd: PRINTDLGEXW = unsafe { std::mem::zeroed() };
    pd.lStructSize = size_of::<PRINTDLGEXW>() as u32;
    pd.hwndOwner = proprietaire;
    REGLAGES.with(|r| {
        if let Some(r) = r.borrow().as_ref() {
            pd.hDevMode = bloc_global(&r.devmode);
            pd.hDevNames = bloc_global(&r.devnames);
        }
    });
    pd.Flags = PD_NOSELECTION | PD_NOCURRENTPAGE | PD_USEDEVMODECOPIESANDCOLLATE;
    pd.nPageRanges = 0;
    pd.nMaxPageRanges = PLAGES_MAX as u32;
    pd.lpPageRanges = plages.as_mut_ptr();
    pd.nMinPage = 1;
    pd.nMaxPage = nombre;
    pd.nCopies = 1;
    pd.nStartPage = START_PAGE_GENERAL;

    // La fenêtre d'impression demande un fil COM à appartement unique ; s'il
    // l'est déjà (ou dans un autre mode), on le laisse tel quel.
    // SAFETY : appels COM et dialogue Windows sur le fil de l'interface.
    let hr = unsafe {
        let com = CoInitializeEx(
            null(),
            (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32,
        );
        let hr = PrintDlgExW(&mut pd);
        if com == S_OK || com == S_FALSE {
            CoUninitialize();
        }
        hr
    };

    // Les blocs rendus (nouveaux ou repris) sont copiés puis libérés.
    let devmode = lire_bloc(pd.hDevMode);
    let devnames = lire_bloc(pd.hDevNames);
    if hr != S_OK {
        return Err(trf(
            "fenêtre d'impression indisponible (code {})",
            &[&format!("{:#010x}", hr as u32)],
        ));
    }
    let action = pd.dwResultAction;
    if action != PD_RESULT_PRINT && action != PD_RESULT_APPLY {
        return Ok(Choix::Rien);
    }
    let (Some(mut devmode), Some(devnames)) = (devmode, devnames) else {
        return Err(trf(
            "fenêtre d'impression indisponible (code {})",
            &[&"DEVNAMES"],
        ));
    };
    // Seules l'imprimante et ses propriétés sont gardées : la fenêtre suivante
    // repropose une copie, pour qu'un Entrée trop rapide n'imprime pas de
    // nouveau plusieurs exemplaires (relecture de kadenz-a8).
    let mut garde = devmode.clone();
    ecrire_i16(&mut garde, decalage_copies(), 1);
    REGLAGES.with(|r| {
        *r.borrow_mut() = Some(Reglages {
            devmode: garde,
            devnames: devnames.clone(),
        })
    });
    if action == PD_RESULT_APPLY {
        return Ok(Choix::Rien);
    }

    let imprimante = chaine_devnames(&devnames, 1);
    let port = chaine_devnames(&devnames, 2);
    let mut sequence: Vec<usize> = Vec::new();
    if pd.Flags & PD_PAGENUMS != 0 {
        for plage in plages.iter().take(pd.nPageRanges as usize) {
            let debut = plage.nFromPage.clamp(1, nombre);
            let fin = plage.nToPage.clamp(debut, nombre);
            sequence.extend((debut..=fin).map(|p| p as usize - 1));
        }
    }
    if sequence.is_empty() {
        sequence = (0..nombre as usize).collect();
    }

    // Copies : avec PD_USEDEVMODECOPIESANDCOLLATE, la fenêtre grise le nombre
    // de copies quand le pilote ne sait pas les faire ; c'est donc le pilote qui
    // les fait. Garde-fou si DeviceCapabilitiesW échoue ou annonce moins de
    // copies que demandé : Kadenz envoie chaque copie et en demande une seule
    // au pilote.
    let champs = lire_u32(&devmode, offset_of!(DEVMODEW, dmFields)).unwrap_or(0);
    let copies = if champs & DM_COPIES != 0 {
        lire_i16(&devmode, decalage_copies()).unwrap_or(1).max(1) as usize
    } else {
        1
    };
    let assemblees = champs & DM_COLLATE != 0
        && lire_i16(&devmode, offset_of!(DEVMODEW, dmCollate)) == Some(DMCOLLATE_TRUE);
    // SAFETY : chaînes terminées par un zéro, DEVMODE complet rendu par le dialogue.
    let pilote = unsafe {
        DeviceCapabilitiesW(
            imprimante.as_ptr(),
            port.as_ptr(),
            DC_COPIES,
            null_mut(),
            devmode.as_ptr().cast(),
        )
    };
    let pages = if copies > 1 && (pilote as i64) < copies as i64 {
        ecrire_i16(&mut devmode, decalage_copies(), 1);
        repeter(&sequence, copies, assemblees)
    } else {
        sequence
    };
    Ok(Choix::Imprimer(Travail {
        imprimante,
        devmode: Some(devmode),
        pages,
    }))
}

/// Pages dans l'ordre d'envoi pour `copies` copies faites par Kadenz.
fn repeter(sequence: &[usize], copies: usize, assemblees: bool) -> Vec<usize> {
    if assemblees {
        (0..copies).flat_map(|_| sequence.iter().copied()).collect()
    } else {
        sequence
            .iter()
            .flat_map(|&p| std::iter::repeat_n(p, copies))
            .collect()
    }
}

fn decalage_copies() -> usize {
    offset_of!(DEVMODEW, Anonymous1) + offset_of!(DEVMODEW_0_0, dmCopies)
}

fn lire_u32(octets: &[u8], decalage: usize) -> Option<u32> {
    octets
        .get(decalage..decalage + 4)
        .map(|o| u32::from_le_bytes([o[0], o[1], o[2], o[3]]))
}

fn lire_i16(octets: &[u8], decalage: usize) -> Option<i16> {
    octets
        .get(decalage..decalage + 2)
        .map(|o| i16::from_le_bytes([o[0], o[1]]))
}

fn ecrire_i16(octets: &mut [u8], decalage: usize, valeur: i16) {
    if let Some(o) = octets.get_mut(decalage..decalage + 2) {
        o.copy_from_slice(&valeur.to_le_bytes());
    }
}

/// Chaîne `rang` d'un bloc DEVNAMES (0 pilote, 1 imprimante, 2 port), terminée
/// par un zéro.
fn chaine_devnames(octets: &[u8], rang: usize) -> Vec<u16> {
    let decalage = octets
        .get(rang * 2..rang * 2 + 2)
        .map(|o| u16::from_le_bytes([o[0], o[1]]) as usize * 2)
        .unwrap_or(usize::MAX);
    let mut chaine: Vec<u16> = octets
        .get(decalage..)
        .unwrap_or(&[])
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&c| u16::from_le_bytes(c))
        .take_while(|&c| c != 0)
        .collect();
    chaine.push(0);
    chaine
}

/// Bloc mémoire mobile rempli de `octets`, pour la fenêtre d'impression.
fn bloc_global(octets: &[u8]) -> HGLOBAL {
    // SAFETY : bloc alloué à la taille copiée, verrouillé pendant la copie.
    unsafe {
        let bloc = GlobalAlloc(GMEM_MOVEABLE, octets.len());
        if bloc.is_null() {
            return null_mut();
        }
        let p = GlobalLock(bloc);
        if p.is_null() {
            GlobalFree(bloc);
            return null_mut();
        }
        std::ptr::copy_nonoverlapping(octets.as_ptr(), p.cast::<u8>(), octets.len());
        GlobalUnlock(bloc);
        bloc
    }
}

/// Copie puis libère un bloc rendu par la fenêtre d'impression.
fn lire_bloc(bloc: HGLOBAL) -> Option<Vec<u8>> {
    if bloc.is_null() {
        return None;
    }
    // SAFETY : bloc valide rendu par PrintDlgExW, lu à sa taille puis libéré.
    unsafe {
        let taille = GlobalSize(bloc);
        let p = GlobalLock(bloc);
        let copie = (!p.is_null() && taille > 0)
            .then(|| std::slice::from_raw_parts(p.cast::<u8>(), taille).to_vec());
        if !p.is_null() {
            GlobalUnlock(bloc);
        }
        GlobalFree(bloc);
        copie
    }
}

fn large(texte: &str) -> Vec<u16> {
    texte.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Imprime les pages du travail ; `sortie` (un fichier) remplace le port de
/// l'imprimante, pour les essais sur « Microsoft Print to PDF ».
/// `avancement` reçoit le rang de la feuille envoyée et leur nombre.
/// Rend vrai si au moins une page a été réduite pour tenir sur la feuille.
pub(crate) fn envoyer(
    octets: Vec<u8>,
    titre: &str,
    travail: &Travail,
    marges: &Marges,
    sortie: Option<&Path>,
    mut avancement: impl FnMut(usize, usize),
) -> Result<bool, Echec> {
    let pdf = Pdf::new(octets).map_err(|_| Echec::Lecture)?;
    let devmode = travail
        .devmode
        .as_ref()
        .map_or(null(), |d| d.as_ptr().cast::<DEVMODEW>());
    // SAFETY : nom terminé par un zéro, DEVMODE rendu par le dialogue ou nul.
    let hdc = unsafe { CreateDCW(null(), travail.imprimante.as_ptr(), null(), devmode) };
    if hdc.is_null() {
        // SAFETY : lecture du code d'erreur du fil courant.
        return Err(Echec::Imprimante(unsafe { GetLastError() }));
    }
    let resultat = envoyer_au_contexte(hdc, &pdf, titre, travail, marges, sortie, &mut avancement);
    // SAFETY : contexte créé ci-dessus, libéré une fois.
    unsafe { DeleteDC(hdc) };
    resultat
}

fn envoyer_au_contexte(
    hdc: HDC,
    pdf: &Pdf,
    titre: &str,
    travail: &Travail,
    marges: &Marges,
    sortie: Option<&Path>,
    avancement: &mut impl FnMut(usize, usize),
) -> Result<bool, Echec> {
    let titre = large(titre);
    let sortie = sortie.map(|s| large(&s.to_string_lossy()));
    let info = DOCINFOW {
        cbSize: size_of::<DOCINFOW>() as i32,
        lpszDocName: titre.as_ptr(),
        lpszOutput: sortie.as_ref().map_or(null(), |s| s.as_ptr()),
        lpszDatatype: null(),
        fwType: 0,
    };
    // SAFETY : chaînes vivantes pendant l'appel.
    if unsafe { StartDocW(hdc, &info) } <= 0 {
        // SAFETY : lecture du code d'erreur du fil courant.
        return Err(Echec::Document(unsafe { GetLastError() }));
    }
    let pages = pdf.pages();
    let total = travail.pages.len();
    let mut reduite = false;
    for (n, &rang) in travail.pages.iter().enumerate() {
        avancement(n + 1, total);
        let Some(page) = pages.get(rang) else {
            continue;
        };
        // SAFETY : contexte d'impression ouvert par StartDocW.
        let ok = unsafe { StartPage(hdc) } > 0
            && poser_page(hdc, page, marges).map(|r| reduite |= r).is_ok()
            && unsafe { EndPage(hdc) } > 0;
        if !ok {
            // SAFETY : document ouvert, abandonné une fois.
            unsafe { AbortDoc(hdc) };
            return Err(Echec::Page(rang + 1));
        }
    }
    // SAFETY : document ouvert par StartDocW.
    if unsafe { EndDoc(hdc) } <= 0 {
        // SAFETY : lecture du code d'erreur du fil courant.
        return Err(Echec::Document(unsafe { GetLastError() }));
    }
    Ok(reduite)
}

/// Feuille de l'imprimante, en unités du périphérique.
#[derive(Clone, Copy, Debug)]
struct Feuille {
    ppp_x: f64,
    ppp_y: f64,
    decalage_x: f64,
    decalage_y: f64,
    zone_largeur: f64,
    zone_hauteur: f64,
}

/// Rectangle de la page sur la zone imprimable, en unités du périphérique.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Place {
    x: f64,
    y: f64,
    largeur: f64,
    hauteur: f64,
    reduite: bool,
}

/// Place une page de `page_l` sur `page_h` points : à 100 %, calée sur
/// l'origine physique de la feuille, si la partie de la page à l'intérieur des
/// marges de Kadenz tombe dans la zone imprimable ; sinon réduite pour tenir
/// entière dans la zone imprimable, centrée en largeur, jamais agrandie.
fn placer(f: &Feuille, page_l: f64, page_h: f64, m: &Marges) -> Place {
    // Demi-millimètre de tolérance (arrondis du pilote).
    let mm_x = |mm: f64| mm / 25.4 * f.ppp_x;
    let mm_y = |mm: f64| mm / 25.4 * f.ppp_y;
    let largeur = page_l / 72.0 * f.ppp_x;
    let hauteur = page_h / 72.0 * f.ppp_y;
    let (tol_x, tol_y) = (mm_x(0.5), mm_y(0.5));
    let contenu_gauche = mm_x(f64::from(m.gauche));
    let contenu_droite = largeur - mm_x(f64::from(m.droite));
    let contenu_haut = mm_y(f64::from(m.haut));
    let contenu_bas = hauteur - mm_y(f64::from(m.bas));
    let tient = contenu_gauche + tol_x >= f.decalage_x
        && contenu_droite <= f.decalage_x + f.zone_largeur + tol_x
        && contenu_haut + tol_y >= f.decalage_y
        && contenu_bas <= f.decalage_y + f.zone_hauteur + tol_y;
    if tient {
        return Place {
            x: -f.decalage_x,
            y: -f.decalage_y,
            largeur,
            hauteur,
            reduite: false,
        };
    }
    let echelle = (f.zone_largeur / largeur)
        .min(f.zone_hauteur / hauteur)
        .min(1.0);
    let (l, h) = (largeur * echelle, hauteur * echelle);
    Place {
        x: (f.zone_largeur - l) / 2.0,
        y: 0.0,
        largeur: l,
        hauteur: h,
        reduite: echelle < 1.0,
    }
}

/// Rend une page et la pose sur la feuille courante, en bandes. Rend vrai si la
/// page a été réduite.
fn poser_page(
    hdc: HDC,
    page: &hayro::hayro_syntax::page::Page,
    marges: &Marges,
) -> Result<bool, ()> {
    // SAFETY : contexte d'impression valide.
    let cap = |i: u32| f64::from(unsafe { GetDeviceCaps(hdc, i as i32) });
    let feuille = Feuille {
        ppp_x: cap(LOGPIXELSX).max(1.0),
        ppp_y: cap(LOGPIXELSY).max(1.0),
        decalage_x: cap(PHYSICALOFFSETX),
        decalage_y: cap(PHYSICALOFFSETY),
        zone_largeur: cap(HORZRES),
        zone_hauteur: cap(VERTRES),
    };
    let (page_l, page_h) = page.render_dimensions();
    let (page_l, page_h) = (f64::from(page_l.max(1.0)), f64::from(page_h.max(1.0)));
    let place = placer(&feuille, page_l, page_h, marges);

    // Image à la résolution du pilote, plafonnée à 300 ppp.
    let image_l = (place.largeur * feuille.ppp_x.min(PPP_MAX) / feuille.ppp_x)
        .round()
        .clamp(1.0, f64::from(u16::MAX)) as u32;
    let image_h = (place.hauteur * feuille.ppp_y.min(PPP_MAX) / feuille.ppp_y)
        .round()
        .clamp(1.0, f64::from(u16::MAX)) as u32;
    let base = Affine::scale_non_uniform(f64::from(image_l) / page_l, f64::from(image_h) / page_h)
        * page.initial_transform(true).to_kurbo();
    let (dest_x, dest_l) = (place.x.round() as i32, place.largeur.round() as i32);
    let dest_y = |ligne: u32| {
        (place.y + f64::from(ligne) * place.hauteur / f64::from(image_h)).round() as i32
    };

    // SAFETY : réglages du contexte ; SetBrushOrgEx est exigé après HALFTONE.
    unsafe {
        SetStretchBltMode(hdc, HALFTONE);
        SetBrushOrgEx(hdc, 0, 0, null_mut());
    }
    let cache = RenderCache::new();
    for k in 0..BANDES {
        let (y0, y1) = (image_h * k / BANDES, image_h * (k + 1) / BANDES);
        if y1 <= y0 {
            continue;
        }
        let bande_h = y1 - y0;
        let mut ctx = RenderContext::new(image_l as u16, bande_h as u16);
        render_into(
            page,
            &cache,
            &InterpreterSettings::default(),
            &RenderSettings::default(),
            &mut ctx,
            Affine::translate((0.0, -f64::from(y0))) * base,
        );
        ctx.flush();
        let mut pixmap = Pixmap::new(image_l as u16, bande_h as u16);
        ctx.render_with(
            &mut pixmap,
            &mut Resources::default(),
            RasterizerSettings {
                target_init: TargetInit::Clear(WHITE),
                ..Default::default()
            },
        );
        let bgr = en_bgr(
            pixmap.data_as_u8_slice(),
            image_l as usize,
            bande_h as usize,
        );
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: image_l as i32,
                // Négatif : image de haut en bas.
                biHeight: -(bande_h as i32),
                biPlanes: 1,
                biBitCount: 24,
                biCompression: BI_RGB,
                ..Default::default()
            },
            ..Default::default()
        };
        let (haut, bas) = (dest_y(y0), dest_y(y1));
        // SAFETY : pixels de la taille annoncée par l'en-tête, contexte valide.
        let lignes = unsafe {
            StretchDIBits(
                hdc,
                dest_x,
                haut,
                dest_l,
                bas - haut,
                0,
                0,
                image_l as i32,
                bande_h as i32,
                bgr.as_ptr().cast(),
                &info,
                DIB_RGB_COLORS,
                SRCCOPY,
            )
        };
        if lignes == 0 || lignes == GDI_ERROR {
            return Err(());
        }
    }
    Ok(place.reduite)
}

/// Pixels RGBA opaques en lignes BGR 24 bits alignées sur 4 octets (DIB).
fn en_bgr(rgba: &[u8], largeur: usize, hauteur: usize) -> Vec<u8> {
    let pas = (3 * largeur + 3) & !3;
    let mut bgr = vec![0u8; pas * hauteur];
    for (ligne, sortie) in rgba
        .chunks_exact(largeur * 4)
        .zip(bgr.chunks_exact_mut(pas))
    {
        for (p, o) in ligne
            .as_chunks::<4>()
            .0
            .iter()
            .zip(sortie.as_chunks_mut::<3>().0)
        {
            o[0] = p[2];
            o[1] = p[1];
            o[2] = p[0];
        }
    }
    bgr
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feuille A4 à 600 ppp avec des marges matérielles de `marge` mm.
    fn a4(marge: f64) -> Feuille {
        let px = |mm: f64| mm / 25.4 * 600.0;
        Feuille {
            ppp_x: 600.0,
            ppp_y: 600.0,
            decalage_x: px(marge),
            decalage_y: px(marge),
            zone_largeur: px(210.0 - 2.0 * marge),
            zone_hauteur: px(297.0 - 2.0 * marge),
        }
    }

    const A4_PT: (f64, f64) = (595.0, 842.0);

    #[test]
    fn page_a_100_pour_cent_si_les_marges_tiennent() {
        let place = placer(&a4(4.0), A4_PT.0, A4_PT.1, &Marges::default());
        assert!(!place.reduite);
        assert_eq!(place.x, -a4(4.0).decalage_x);
        assert!((place.largeur - 595.0 / 72.0 * 600.0).abs() < 1e-6);
    }

    #[test]
    fn page_reduite_si_une_marge_du_pilote_depasse() {
        // Marges matérielles de 15 mm, plus grandes que les 9 mm du bas de Kadenz.
        let place = placer(&a4(15.0), A4_PT.0, A4_PT.1, &Marges::default());
        assert!(place.reduite);
        assert!(place.largeur <= a4(15.0).zone_largeur + 1e-6);
        assert!(place.hauteur <= a4(15.0).zone_hauteur + 1e-6);
    }

    #[test]
    fn page_reduite_sur_letter_et_en_paysage() {
        let px = |mm: f64| mm / 25.4 * 600.0;
        let mut letter = a4(4.0);
        letter.zone_largeur = px(215.9 - 8.0);
        letter.zone_hauteur = px(279.4 - 8.0);
        assert!(placer(&letter, A4_PT.0, A4_PT.1, &Marges::default()).reduite);
        let mut paysage = a4(4.0);
        std::mem::swap(&mut paysage.zone_largeur, &mut paysage.zone_hauteur);
        let place = placer(&paysage, A4_PT.0, A4_PT.1, &Marges::default());
        assert!(place.reduite);
        assert!(place.hauteur <= paysage.zone_hauteur + 1e-6);
    }

    #[test]
    fn copies_assemblees_ou_non() {
        assert_eq!(repeter(&[0, 1], 2, true), vec![0, 1, 0, 1]);
        assert_eq!(repeter(&[0, 1], 2, false), vec![0, 0, 1, 1]);
    }

    #[test]
    fn lignes_dib_alignees_sur_quatre_octets() {
        let rgba = [10, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255];
        let bgr = en_bgr(&rgba, 3, 1);
        assert_eq!(bgr.len(), 12);
        assert_eq!(&bgr[..9], &[30, 20, 10, 60, 50, 40, 90, 80, 70]);
    }

    #[test]
    fn noms_du_bloc_devnames() {
        // En-tête de 4 mots puis « w », « Imp », « P: » en UTF-16.
        let mots: Vec<u16> = vec![
            4,
            6,
            10,
            0,
            b'w' as u16,
            0,
            b'I' as u16,
            b'm' as u16,
            b'p' as u16,
            0,
            b'P' as u16,
            b':' as u16,
            0,
        ];
        let octets: Vec<u8> = mots.iter().flat_map(|m| m.to_le_bytes()).collect();
        assert_eq!(
            String::from_utf16_lossy(&chaine_devnames(&octets, 1)),
            "Imp\0"
        );
        assert_eq!(
            String::from_utf16_lossy(&chaine_devnames(&octets, 2)),
            "P:\0"
        );
    }
}
