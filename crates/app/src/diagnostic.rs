//! Diagnostic au démarrage : journal de plantage et choix du rendu.
//!
//! Journal de plantage : un `panic` écrit sa cause, son lieu et la pile dans
//! `journaux\plantage.log` du dossier des données, que l'utilisateur peut envoyer
//! au support. Aucune donnée de l'entreprise n'y est écrite.
//!
//! Rendu : par défaut, celui de Slint (OpenGL, avec repli automatique si la
//! fenêtre ne peut pas être créée). Sur un poste dont le pilote graphique affiche
//! mal l'interface (champs invisibles par exemple), le rendu logiciel se force par
//! l'option `--rendu-logiciel`, par un fichier vide nommé `rendu-logiciel` dans le
//! dossier des données, ou par la variable `KADENZ_RENDU=logiciel`.

use std::io::Write;
use std::path::{Path, PathBuf};

/// Taille au-delà de laquelle le journal de plantage repart de zéro.
const TAILLE_MAX: u64 = 1_000_000;

pub(crate) fn dossier_journaux(dossier_donnees: &Path) -> PathBuf {
    dossier_donnees.join("journaux")
}

/// Installe le journal de plantage et applique le choix du rendu.
pub(crate) fn installer(dossier_donnees: &Path) {
    installer_journal_plantage(dossier_donnees);
    if rendu_logiciel_demande(dossier_donnees) {
        let choix = slint::BackendSelector::new()
            .backend_name("winit".into())
            .renderer_name("software".into())
            .select();
        if let Err(e) = choix {
            ecrire(
                dossier_donnees,
                &format!("rendu logiciel indisponible, rendu par défaut gardé : {e}"),
            );
        }
    }
}

fn rendu_logiciel_demande(dossier_donnees: &Path) -> bool {
    std::env::args().any(|a| a == "--rendu-logiciel")
        || std::env::var("KADENZ_RENDU").is_ok_and(|v| v.eq_ignore_ascii_case("logiciel"))
        || dossier_donnees.join("rendu-logiciel").exists()
}

fn installer_journal_plantage(dossier_donnees: &Path) {
    let dossier = dossier_donnees.to_path_buf();
    let precedent = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let lieu = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "lieu inconnu".into());
        let cause = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| (*s).to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "cause inconnue".into());
        let pile = std::backtrace::Backtrace::force_capture();
        ecrire(&dossier, &format!("plantage à {lieu} : {cause}\n{pile}"));
        precedent(info);
    }));
}

/// Ajoute une entrée datée au journal de plantage ; les erreurs d'écriture sont
/// ignorées (il ne faut jamais planter en écrivant le journal).
fn ecrire(dossier_donnees: &Path, texte: &str) {
    let dossier = dossier_journaux(dossier_donnees);
    if std::fs::create_dir_all(&dossier).is_err() {
        return;
    }
    let fichier = dossier.join("plantage.log");
    let trop_gros = std::fs::metadata(&fichier).is_ok_and(|m| m.len() > TAILLE_MAX);
    let ouverture = std::fs::OpenOptions::new()
        .create(true)
        .append(!trop_gros)
        .write(true)
        .truncate(trop_gros)
        .open(&fichier);
    if let Ok(mut f) = ouverture {
        let _ = writeln!(
            f,
            "[{}] Kadenz {} - {texte}\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            kadenz_maj::version_installee()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_journal_ecrit_une_entree_datee_et_repart_de_zero_quand_il_est_trop_gros() {
        let dossier = std::env::temp_dir().join(format!("kadenz-diag-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        ecrire(&dossier, "premier");
        let fichier = dossier_journaux(&dossier).join("plantage.log");
        let contenu = std::fs::read_to_string(&fichier).unwrap();
        assert!(contenu.contains("premier") && contenu.starts_with("[20"));
        std::fs::write(&fichier, vec![b'x'; (TAILLE_MAX + 1) as usize]).unwrap();
        ecrire(&dossier, "second");
        let contenu = std::fs::read_to_string(&fichier).unwrap();
        assert!(contenu.contains("second") && !contenu.contains("xxxx"));
        let _ = std::fs::remove_dir_all(&dossier);
    }

    #[test]
    fn le_fichier_marqueur_demande_le_rendu_logiciel() {
        let dossier = std::env::temp_dir().join(format!("kadenz-rendu-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        assert!(!dossier.join("rendu-logiciel").exists());
        std::fs::write(dossier.join("rendu-logiciel"), b"").unwrap();
        assert!(rendu_logiciel_demande(&dossier));
        let _ = std::fs::remove_dir_all(&dossier);
    }
}
