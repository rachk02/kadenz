// En version optimisée, pas de console Windows derrière la fenêtre.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

slint::include_modules!();

/// Emplacement de la base : dossier de données de l'utilisateur Windows
/// (cahier des charges, section 13).
fn chemin_base() -> PathBuf {
    let racine = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    racine.join("Kadenz").join("kadenz.sqlite")
}

fn main() -> Result<(), slint::PlatformError> {
    let chemin = chemin_base();
    let etat_base = match kadenz_stockage::ouvrir(&chemin) {
        Ok(_conn) => format!(
            "Base : {} (schéma {})",
            chemin.display(),
            kadenz_stockage::version_schema_application()
        ),
        Err(e) => format!("Base indisponible : {e}"),
    };

    let fenetre = FenetrePrincipale::new()?;
    fenetre.set_etat_base(etat_base.into());
    fenetre.run()
}
