//! Mode clair ou sombre (refonte des écrans du 2026-11-18). Le mode est choisi
//! par compte, sur chaque poste (`comptes::ModeAffichage`, migration 0018) ; en
//! mode « comme Windows », l'écran suit le réglage « Choisir votre mode
//! d'application » de Windows, relu toutes les deux secondes pour suivre un
//! changement fait pendant que Kadenz est ouvert.

use std::time::Duration;

use kadenz_stockage::comptes::ModeAffichage;
use slint::{ComponentHandle, Timer, TimerMode};

use crate::{FenetrePrincipale, Theme};

/// Rang du mode dans `Theme.mode` : 0 comme Windows, 1 clair, 2 sombre.
pub(crate) fn rang(mode: ModeAffichage) -> i32 {
    match mode {
        ModeAffichage::Systeme => 0,
        ModeAffichage::Clair => 1,
        ModeAffichage::Sombre => 2,
    }
}

pub(crate) fn depuis_rang(rang: i32) -> ModeAffichage {
    match rang {
        1 => ModeAffichage::Clair,
        2 => ModeAffichage::Sombre,
        _ => ModeAffichage::Systeme,
    }
}

/// Lit le réglage de Windows tout de suite, puis toutes les deux secondes. Le
/// minuteur doit vivre aussi longtemps que la fenêtre : il est rendu à l'appelant.
pub(crate) fn suivre_windows(fenetre: &FenetrePrincipale) -> Timer {
    fenetre.global::<Theme>().set_windows_sombre(windows_sombre());
    let minuteur = Timer::default();
    let f = fenetre.as_weak();
    minuteur.start(TimerMode::Repeated, Duration::from_secs(2), move || {
        if let Some(f) = f.upgrade() {
            let t = f.global::<Theme>();
            let sombre = windows_sombre();
            if t.get_windows_sombre() != sombre {
                t.set_windows_sombre(sombre);
            }
        }
    });
    minuteur
}

/// Windows est-il en mode sombre pour les applications ? Valeur
/// `AppsUseLightTheme` de `HKCU\...\Themes\Personalize` : 0 pour sombre. Absente
/// ou illisible (Windows ancien, réglage jamais touché) : mode clair.
#[cfg(windows)]
fn windows_sombre() -> bool {
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
    let large = |s: &str| s.encode_utf16().chain([0]).collect::<Vec<u16>>();
    let cle = large(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
    let valeur = large("AppsUseLightTheme");
    let mut donnee: u32 = 1;
    let mut taille = std::mem::size_of::<u32>() as u32;
    // SAFETY : chaînes terminées par un zéro ; tampon d'un DWORD et taille en octets.
    let code = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            cle.as_ptr(),
            valeur.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&mut donnee as *mut u32).cast(),
            &mut taille,
        )
    };
    code == 0 && donnee == 0
}

#[cfg(not(windows))]
fn windows_sombre() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rangs_des_modes() {
        for mode in [ModeAffichage::Systeme, ModeAffichage::Clair, ModeAffichage::Sombre] {
            assert_eq!(depuis_rang(rang(mode)), mode);
        }
        assert_eq!(depuis_rang(7), ModeAffichage::Systeme);
    }
}
