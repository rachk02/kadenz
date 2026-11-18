//! Mode clair ou sombre et palette sombre (refonte des écrans du 2026-11-18).
//!
//! - Réglage du poste : fichier `affichage.json` du dossier des données, choisi
//!   au premier lancement ; il sert avant toute connexion et pour les comptes
//!   qui n'ont pas fait leur propre choix. Un fichier plutôt que la base : un
//!   poste qui rejoint une entreprise ouvre ensuite la copie du serveur, le
//!   réglage doit y survivre.
//! - Choix d'un compte (`comptes::changer_mode_affichage`, migrations 0018 et
//!   0019) : gardé dans la base de ce poste ; il devient aussi le réglage du
//!   poste, pour que l'écran de connexion garde le dernier aspect choisi.
//! - Animations : préférence du compte ; le dernier choix fait sur ce poste est
//!   aussi gardé dans `affichage.json`, pour que les motifs des écrans d'avant
//!   connexion (présentation, inscription, connexion) restent immobiles quand
//!   elles sont coupées.
//! - « Comme Windows » : réglage « Choisir votre mode d'application » de
//!   Windows, relu toutes les deux secondes pour suivre un changement fait
//!   pendant que Kadenz est ouvert.

use std::path::{Path, PathBuf};
use std::time::Duration;

use kadenz_stockage::comptes::{ModeAffichage, PaletteSombre, Utilisateur};
use serde_json::json;
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

/// Rang de la palette dans `Theme.palette` : 0 Graphite, 1 Ardoise, 2 Anthracite.
pub(crate) fn rang_palette(palette: PaletteSombre) -> i32 {
    match palette {
        PaletteSombre::Graphite => 0,
        PaletteSombre::Ardoise => 1,
        PaletteSombre::Anthracite => 2,
    }
}

pub(crate) fn palette_depuis_rang(rang: i32) -> PaletteSombre {
    match rang {
        0 => PaletteSombre::Graphite,
        1 => PaletteSombre::Ardoise,
        _ => PaletteSombre::Anthracite,
    }
}

fn fichier_du_poste() -> PathBuf {
    crate::dossier_donnees().join("affichage.json")
}

/// Réglage du poste ; absent ou illisible : comme Windows, palette Anthracite.
pub(crate) fn du_poste() -> (ModeAffichage, PaletteSombre) {
    lire(&fichier_du_poste())
}

fn contenu(chemin: &Path) -> serde_json::Value {
    std::fs::read_to_string(chemin)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .filter(serde_json::Value::is_object)
        .unwrap_or_else(|| json!({}))
}

fn lire(chemin: &Path) -> (ModeAffichage, PaletteSombre) {
    let valeur = contenu(chemin);
    let code = |c: &str| valeur.get(c).and_then(serde_json::Value::as_str).unwrap_or("");
    (
        ModeAffichage::depuis_code(code("mode")).unwrap_or_default(),
        PaletteSombre::depuis_code(code("palette")).unwrap_or_default(),
    )
}

/// Enregistre le réglage du poste. Un échec n'empêche rien : l'écran garde le
/// choix jusqu'à la fermeture ; il est noté dans erreurs.log.
pub(crate) fn enregistrer_poste(mode: ModeAffichage, palette: PaletteSombre) {
    if let Err(e) = ecrire(&fichier_du_poste(), mode, palette) {
        crate::consigner_erreur(&format!("Réglage d'affichage du poste non enregistré : {e}"));
    }
}

fn ecrire(chemin: &Path, mode: ModeAffichage, palette: PaletteSombre) -> std::io::Result<()> {
    fusionner(chemin, &[("mode", json!(mode.code())), ("palette", json!(palette.code()))])
}

/// Écrit des champs du réglage du poste en gardant les autres.
fn fusionner(chemin: &Path, champs: &[(&str, serde_json::Value)]) -> std::io::Result<()> {
    if let Some(dossier) = chemin.parent() {
        std::fs::create_dir_all(dossier)?;
    }
    let mut valeur = contenu(chemin);
    for (nom, v) in champs {
        valeur[*nom] = v.clone();
    }
    std::fs::write(chemin, valeur.to_string())
}

/// Animations avant toute connexion : dernier choix d'un compte sur ce poste ;
/// absent ou illisible : animées.
pub(crate) fn animations_du_poste() -> bool {
    lire_animations(&fichier_du_poste())
}

fn lire_animations(chemin: &Path) -> bool {
    contenu(chemin).get("animations").and_then(serde_json::Value::as_bool).unwrap_or(true)
}

/// Garde le choix d'animations d'un compte pour les écrans d'avant connexion.
pub(crate) fn enregistrer_animations_poste(animations: bool) {
    if let Err(e) = fusionner(&fichier_du_poste(), &[("animations", json!(animations))]) {
        crate::consigner_erreur(&format!("Réglage des animations du poste non enregistré : {e}"));
    }
}

/// Affichage d'un compte : son propre choix s'il en a fait un, sinon le
/// réglage du poste.
pub(crate) fn du_compte(u: &Utilisateur) -> (ModeAffichage, PaletteSombre) {
    if u.affichage_personnel {
        (u.theme, u.palette)
    } else {
        du_poste()
    }
}

pub(crate) fn appliquer(fenetre: &FenetrePrincipale, (mode, palette): (ModeAffichage, PaletteSombre)) {
    let t = fenetre.global::<Theme>();
    t.set_mode(rang(mode));
    t.set_palette(rang_palette(palette));
    barre_de_titre(fenetre, t.get_sombre());
}

/// Barre de titre de Windows sombre en mode sombre (attribut
/// DWMWA_USE_IMMERSIVE_DARK_MODE, Windows 10 20H1 et suivants ; ignoré avant).
/// Sans fenêtre encore créée, rien n'est fait : le minuteur la reprend.
#[cfg(windows)]
fn barre_de_titre(fenetre: &FenetrePrincipale, sombre: bool) {
    use windows_sys::Win32::Graphics::Dwm::{DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute};
    let h = crate::apercu::poignee(fenetre);
    if h.is_null() {
        return;
    }
    let valeur = i32::from(sombre);
    // SAFETY : poignée de la fenêtre de Kadenz ; attribut d'un BOOL (4 octets).
    unsafe {
        DwmSetWindowAttribute(
            h,
            DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
            (&valeur as *const i32).cast(),
            std::mem::size_of::<i32>() as u32,
        );
    }
}

#[cfg(not(windows))]
fn barre_de_titre(_fenetre: &FenetrePrincipale, _sombre: bool) {}

/// Lit le réglage de Windows tout de suite, puis toutes les deux secondes. Le
/// minuteur doit vivre aussi longtemps que la fenêtre : il est rendu à l'appelant.
pub(crate) fn suivre_windows(fenetre: &FenetrePrincipale) -> Timer {
    fenetre.global::<Theme>().set_windows_sombre(windows_sombre());
    let minuteur = Timer::default();
    let f = fenetre.as_weak();
    // Barre de titre : dès que la fenêtre existe, puis à chaque changement de Windows.
    let premiere = fenetre.as_weak();
    Timer::single_shot(Duration::from_millis(200), move || {
        if let Some(f) = premiere.upgrade() {
            barre_de_titre(&f, f.global::<Theme>().get_sombre());
        }
    });
    minuteur.start(TimerMode::Repeated, Duration::from_secs(2), move || {
        if let Some(f) = f.upgrade() {
            let t = f.global::<Theme>();
            let sombre = windows_sombre();
            if t.get_windows_sombre() != sombre {
                t.set_windows_sombre(sombre);
                barre_de_titre(&f, t.get_sombre());
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
    fn rangs_des_modes_et_des_palettes() {
        for mode in [ModeAffichage::Systeme, ModeAffichage::Clair, ModeAffichage::Sombre] {
            assert_eq!(depuis_rang(rang(mode)), mode);
        }
        assert_eq!(depuis_rang(7), ModeAffichage::Systeme);
        for palette in [PaletteSombre::Graphite, PaletteSombre::Ardoise, PaletteSombre::Anthracite] {
            assert_eq!(palette_depuis_rang(rang_palette(palette)), palette);
        }
        assert_eq!(palette_depuis_rang(-1), PaletteSombre::Anthracite);
    }

    #[test]
    fn reglage_du_poste_dans_un_fichier() {
        let dossier = std::env::temp_dir().join(format!("kadenz-affichage-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        let chemin = dossier.join("affichage.json");
        // Absent : comme Windows, Anthracite.
        assert_eq!(lire(&chemin), (ModeAffichage::Systeme, PaletteSombre::Anthracite));
        ecrire(&chemin, ModeAffichage::Sombre, PaletteSombre::Ardoise).unwrap();
        assert_eq!(lire(&chemin), (ModeAffichage::Sombre, PaletteSombre::Ardoise));
        // Les animations s'ajoutent sans effacer le mode ni la palette, et l'inverse.
        assert!(lire_animations(&chemin));
        fusionner(&chemin, &[("animations", json!(false))]).unwrap();
        assert!(!lire_animations(&chemin));
        assert_eq!(lire(&chemin), (ModeAffichage::Sombre, PaletteSombre::Ardoise));
        ecrire(&chemin, ModeAffichage::Clair, PaletteSombre::Graphite).unwrap();
        assert!(!lire_animations(&chemin));
        // Illisible : valeurs par défaut.
        std::fs::write(&chemin, "{pas du json").unwrap();
        assert_eq!(lire(&chemin), (ModeAffichage::Systeme, PaletteSombre::Anthracite));
        assert!(lire_animations(&chemin));
        let _ = std::fs::remove_dir_all(&dossier);
    }
}
