//! Édition de l'application (docs/spec-monoposte.md, section 6) : un seul
//! programme, et l'installateur de l'édition monoposte pose à côté de
//! l'exécutable un fichier marque. Sans marque, c'est l'édition multiposte.
//!
//! Le dossier d'installation (Program Files) n'est modifiable que par un
//! administrateur : un compte ordinaire ne peut pas changer d'édition en posant
//! ou en retirant la marque.

use std::path::Path;

/// Nom du fichier marque posé par l'installateur monoposte.
pub const MARQUE: &str = "edition-monoposte";

/// Le dossier contient-il la marque de l'édition monoposte ?
pub fn monoposte_dans(dossier: &Path) -> bool {
    dossier.join(MARQUE).is_file()
}

/// L'application en cours est-elle de l'édition monoposte ? Lue à côté de
/// l'exécutable ; faux si son emplacement est inconnu.
pub fn monoposte() -> bool {
    std::env::current_exe()
        .ok()
        .as_deref()
        .and_then(Path::parent)
        .is_some_and(monoposte_dans)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_marque_fait_l_edition() {
        let dossier = std::env::temp_dir().join(format!("kadenz-édition-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(&dossier).unwrap();
        assert!(!monoposte_dans(&dossier));
        std::fs::write(dossier.join(MARQUE), b"").unwrap();
        assert!(monoposte_dans(&dossier));
        // Un dossier de ce nom ne vaut pas marque.
        std::fs::remove_file(dossier.join(MARQUE)).unwrap();
        std::fs::create_dir(dossier.join(MARQUE)).unwrap();
        assert!(!monoposte_dans(&dossier));
        let _ = std::fs::remove_dir_all(&dossier);
    }
}
