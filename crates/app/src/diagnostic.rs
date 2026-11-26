//! Diagnostic au démarrage : journal de plantage et choix du rendu.
//!
//! Journal de plantage : un `panic` écrit sa cause, son lieu et la pile dans
//! `journaux\plantage.log` du dossier des données, que l'utilisateur peut envoyer
//! au support. Aucune donnée de l'entreprise n'y est écrite.
//!
//! Rendu : par défaut, le rendu logiciel de Slint (le processeur dessine la
//! fenêtre, sans pilote graphique). Il évite toute la famille de défauts qui
//! tiennent aux pilotes OpenGL des vieux postes (champs invisibles, fenêtre
//! noire) et se voit comme le rendu accéléré. Pour revenir au rendu accéléré
//! (OpenGL), option `--rendu-accelere`, fichier vide nommé `rendu-accelere` dans
//! le dossier des données, ou variable `KADENZ_RENDU=accelere`. Le rendu logiciel
//! reste forçable par `--rendu-logiciel`, par un fichier `rendu-logiciel` ou par
//! `KADENZ_RENDU=logiciel`, et l'emporte sur le rendu accéléré.
//!
//! Garde du démarrage : un fichier `demarrage-en-cours` est posé avant la première
//! fenêtre et retiré quand elle a tenu quelques secondes ou à la fermeture normale.
//! Resté en place au lancement suivant (arrêt brutal du pilote graphique, fermeture
//! sans message), il ramène Kadenz au rendu logiciel de façon durable (fichier
//! `rendu-logiciel` posé, `rendu-accelere` retiré).

use std::io::Write;
use std::path::{Path, PathBuf};

/// Taille au-delà de laquelle le journal de plantage repart de zéro.
const TAILLE_MAX: u64 = 1_000_000;

pub(crate) fn dossier_journaux(dossier_donnees: &Path) -> PathBuf {
    dossier_donnees.join("journaux")
}

/// Fichier posé pendant le démarrage ; voir la garde du démarrage plus haut.
const MARQUE_DEMARRAGE: &str = "demarrage-en-cours";
const MARQUE_RENDU: &str = "rendu-logiciel";
const MARQUE_ACCELERE: &str = "rendu-accelere";

/// Vrai si le démarrage précédent n'a pas abouti ; dans ce cas le rendu logiciel
/// est mémorisé pour les lancements suivants. Pose ensuite la marque du démarrage
/// en cours.
fn garder_le_demarrage(dossier_donnees: &Path) -> bool {
    let marque = dossier_donnees.join(MARQUE_DEMARRAGE);
    let echec = marque.exists();
    if echec {
        let _ = std::fs::write(dossier_donnees.join(MARQUE_RENDU), b"");
        let _ = std::fs::remove_file(dossier_donnees.join(MARQUE_ACCELERE));
        ecrire(
            dossier_donnees,
            "le démarrage précédent n'a pas abouti avant l'affichage de la fenêtre : rendu logiciel activé ; supprimer le fichier rendu-logiciel du dossier des données puis créer rendu-accelere pour essayer le rendu accéléré",
        );
    }
    let _ = std::fs::create_dir_all(dossier_donnees);
    let _ = std::fs::write(&marque, b"");
    echec
}

/// La fenêtre est affichée et tient : le démarrage a abouti.
pub(crate) fn demarrage_reussi(dossier_donnees: &Path) {
    let _ = std::fs::remove_file(dossier_donnees.join(MARQUE_DEMARRAGE));
}

/// Installe le journal de plantage et applique le choix du rendu.
pub(crate) fn installer(dossier_donnees: &Path) {
    installer_journal_plantage(dossier_donnees);
    garder_le_demarrage(dossier_donnees);
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

/// Vrai quand le rendu logiciel s'applique : par défaut, sauf demande du rendu
/// accéléré ; une demande explicite du rendu logiciel l'emporte toujours.
fn rendu_logiciel_demande(dossier_donnees: &Path) -> bool {
    let logiciel_explicite = std::env::args().any(|a| a == "--rendu-logiciel")
        || std::env::var("KADENZ_RENDU").is_ok_and(|v| v.eq_ignore_ascii_case("logiciel"))
        || dossier_donnees.join(MARQUE_RENDU).exists();
    if logiciel_explicite {
        return true;
    }
    !rendu_accelere_demande(dossier_donnees)
}

fn rendu_accelere_demande(dossier_donnees: &Path) -> bool {
    std::env::args().any(|a| a == "--rendu-accelere")
        || std::env::var("KADENZ_RENDU").is_ok_and(|v| v.eq_ignore_ascii_case("accelere"))
        || dossier_donnees.join(MARQUE_ACCELERE).exists()
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
    fn un_demarrage_inacheve_active_le_rendu_logiciel() {
        let dossier = std::env::temp_dir().join(format!("kadenz-garde-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        // Premier lancement : rien d'anormal, la marque est posée.
        assert!(!garder_le_demarrage(&dossier));
        assert!(dossier.join(MARQUE_DEMARRAGE).exists());
        assert!(!dossier.join(MARQUE_RENDU).exists());
        // Le démarrage aboutit : la marque disparaît, le lancement suivant est normal.
        demarrage_reussi(&dossier);
        assert!(!garder_le_demarrage(&dossier));
        // Démarrage inachevé (marque restée) : rendu logiciel mémorisé et journal.
        assert!(garder_le_demarrage(&dossier));
        assert!(dossier.join(MARQUE_RENDU).exists());
        assert!(rendu_logiciel_demande(&dossier));
        let journal =
            std::fs::read_to_string(dossier_journaux(&dossier).join("plantage.log")).unwrap();
        assert!(journal.contains("rendu logiciel activé"));
        let _ = std::fs::remove_dir_all(&dossier);
    }

    #[test]
    fn le_rendu_logiciel_est_le_rendu_par_defaut() {
        let dossier = std::env::temp_dir().join(format!("kadenz-rendu-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(&dossier).unwrap();
        // Rien de posé : rendu logiciel.
        assert!(rendu_logiciel_demande(&dossier));
        // Le rendu accéléré se demande par un fichier.
        std::fs::write(dossier.join(MARQUE_ACCELERE), b"").unwrap();
        assert!(!rendu_logiciel_demande(&dossier));
        // Le fichier rendu-logiciel l'emporte.
        std::fs::write(dossier.join(MARQUE_RENDU), b"").unwrap();
        assert!(rendu_logiciel_demande(&dossier));
        let _ = std::fs::remove_dir_all(&dossier);
    }

    #[test]
    fn un_demarrage_inacheve_retire_le_rendu_accelere() {
        let dossier = std::env::temp_dir().join(format!("kadenz-accel-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(&dossier).unwrap();
        std::fs::write(dossier.join(MARQUE_ACCELERE), b"").unwrap();
        assert!(!garder_le_demarrage(&dossier));
        assert!(garder_le_demarrage(&dossier));
        assert!(!dossier.join(MARQUE_ACCELERE).exists());
        assert!(rendu_logiciel_demande(&dossier));
        let _ = std::fs::remove_dir_all(&dossier);
    }
}
