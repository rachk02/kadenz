//! Sauvegardes du serveur (phase 2, jalon F7 ; décisions 35 et 56 ; spec.
//! synchronisation, section 8).
//!
//! - Le réglage (dossier, heure, mot de passe) vit dans `sauvegardes.reglage`,
//!   dans le dossier des données du serveur, réservé à SYSTEM et aux
//!   Administrateurs ; le mot de passe y est protégé par Windows au niveau de
//!   la machine, pour que le service le lise sans session ouverte. Il se règle
//!   dans la fenêtre d'administration (décision 56).
//! - Le service sauvegarde chaque jour à l'heure réglée, et à son arrêt ; une
//!   sauvegarde du jour manquée (machine éteinte à l'heure dite) est faite dès
//!   que possible. Les [`AUTOMATIQUES_GARDEES`] sauvegardes quotidiennes et
//!   les [`ARRETS_GARDES`] sauvegardes d'arrêt les plus récentes sont gardées,
//!   comptées à part : des arrêts rapprochés ne chassent pas les quotidiennes.
//! - Chaque sauvegarde porte l'identité TLS du serveur (certificat et clé) :
//!   restaurée sur une nouvelle machine, elle garde l'empreinte que les postes
//!   ont épinglée. Elle est chiffrée par le mot de passe, comme toute la base.
//! - Sans mot de passe, rien n'est sauvegardé ; tant que le dossier est sur le
//!   même disque que la base, un avertissement permanent conseille une copie
//!   hors de la machine.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{Local, NaiveDateTime, NaiveTime};
use kadenz_protocole::sauvegarde::AvertissementSauvegarde;
use kadenz_stockage::chiffrement;
use kadenz_stockage::comptes::LONGUEUR_MIN_MOT_DE_PASSE;
use kadenz_stockage::sauvegarde::{self, Genre, InfoSauvegarde};
use rusqlite::Connection;
use serde_json::{Value, json};

pub use kadenz_stockage::sauvegarde::{ARRETS_GARDES, AUTOMATIQUES_GARDEES};

use crate::{NOM_CERTIFICAT, NOM_CLE_TLS, portee_du_dossier};

/// Fichier du réglage, dans le dossier des données du serveur.
pub const FICHIER_REGLAGE: &str = "sauvegardes.reglage";

/// Heure de la sauvegarde quotidienne tant qu'aucune n'est réglée : en
/// journée, quand la machine d'un bureau est allumée.
pub const HEURE_PAR_DEFAUT: (u32, u32) = (12, 0);

/// Sous-dossier du dossier des sauvegardes où une sauvegarde s'écrit avant
/// d'être complète.
const SOUS_DOSSIER_EN_COURS: &str = "en-cours";

/// Table ajoutée à chaque sauvegarde (jamais à la base vivante) : identité
/// TLS du serveur.
pub const TABLE_IDENTITE: &str = "serveur_identite";

/// Réglage des sauvegardes du serveur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reglage {
    pub dossier: PathBuf,
    /// Heure locale de la sauvegarde quotidienne.
    pub heure: NaiveTime,
    /// `None` : aucun mot de passe choisi (ou illisible sur cette machine) ;
    /// aucune sauvegarde n'est faite.
    pub mot_de_passe: Option<String>,
}

/// `%ProgramData%\Kadenz\Sauvegardes`, à côté du dossier des données : sur
/// le même disque, d'où l'avertissement tant qu'il n'est pas changé.
pub fn dossier_par_defaut() -> PathBuf {
    crate::dossier_par_defaut()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData\Kadenz"))
        .join("Sauvegardes")
}

fn heure_par_defaut() -> NaiveTime {
    NaiveTime::from_hms_opt(HEURE_PAR_DEFAUT.0, HEURE_PAR_DEFAUT.1, 0).unwrap_or_default()
}

fn brut(donnees: &Path) -> serde_json::Map<String, Value> {
    fs::read_to_string(donnees.join(FICHIER_REGLAGE))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

/// Réglage lu dans le dossier des données `donnees` ; absent ou endommagé :
/// dossier et heure par défaut, aucun mot de passe.
pub fn lire_reglage(donnees: &Path) -> Reglage {
    let brut = brut(donnees);
    Reglage {
        dossier: brut
            .get("dossier")
            .and_then(Value::as_str)
            .filter(|d| !d.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(dossier_par_defaut),
        heure: brut
            .get("heure")
            .and_then(Value::as_str)
            .and_then(|h| NaiveTime::parse_from_str(h, "%H:%M").ok())
            .unwrap_or_else(heure_par_defaut),
        mot_de_passe: brut
            .get("mot_de_passe")
            .and_then(Value::as_str)
            .and_then(depuis_hexa)
            .and_then(|p| chiffrement::deproteger(&p))
            .and_then(|o| String::from_utf8(o).ok()),
    }
}

/// Enregistre le dossier, l'heure et, s'il est donné, le nouveau mot de
/// passe ; `None` garde le mot de passe actuel. Le dossier est créé.
pub fn enregistrer_reglage(
    donnees: &Path,
    dossier: &Path,
    heure: NaiveTime,
    mot_de_passe: Option<&str>,
) -> Result<(), String> {
    if !dossier.is_absolute()
        || dossier
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("le dossier des sauvegardes doit être un chemin complet".into());
    }
    if dans(dossier, donnees) {
        return Err(
            "le dossier des sauvegardes ne peut pas être dans le dossier des données du serveur"
                .into(),
        );
    }
    let dossier_texte = dossier
        .to_str()
        .ok_or("le dossier des sauvegardes contient des caractères non pris en charge")?;
    let mut brut = brut(donnees);
    if let Some(mot_de_passe) = mot_de_passe {
        if mot_de_passe.chars().count() < LONGUEUR_MIN_MOT_DE_PASSE {
            return Err(format!(
                "le mot de passe des sauvegardes doit compter au moins {LONGUEUR_MIN_MOT_DE_PASSE} caractères"
            ));
        }
        let protege = chiffrement::proteger(mot_de_passe.as_bytes(), portee_du_dossier(donnees))
            .ok_or("protection du mot de passe par Windows impossible")?;
        brut.insert("mot_de_passe".into(), Value::String(en_hexa(&protege)));
    }
    fs::create_dir_all(dossier)
        .map_err(|e| format!("dossier des sauvegardes impossible à créer : {e}"))?;
    brut.insert("dossier".into(), json!(dossier_texte));
    brut.insert("heure".into(), json!(heure.format("%H:%M").to_string()));
    let fichier = donnees.join(FICHIER_REGLAGE);
    let provisoire = donnees.join(format!("{FICHIER_REGLAGE}.nouveau"));
    fs::write(&provisoire, Value::Object(brut).to_string())
        .map_err(|e| format!("réglage des sauvegardes non écrit : {e}"))?;
    fs::rename(&provisoire, &fichier).map_err(|e| {
        let _ = fs::remove_file(&provisoire);
        format!("réglage des sauvegardes non écrit : {e}")
    })
}

/// Avertissement permanent : aucun mot de passe, ou dossier sur le même
/// disque que la base.
pub fn avertissement(donnees: &Path) -> Option<AvertissementSauvegarde> {
    let reglage = lire_reglage(donnees);
    if reglage.mot_de_passe.is_none() {
        Some(AvertissementSauvegarde::SansMotDePasse)
    } else if meme_disque(&reglage.dossier, donnees) {
        Some(AvertissementSauvegarde::MemeDisque)
    } else {
        None
    }
}

/// Vrai si les deux chemins sont sur le même disque (même lettre de lecteur
/// ou même partage réseau ; sans tenir compte de la casse).
#[cfg(windows)]
pub fn meme_disque(a: &Path, b: &Path) -> bool {
    use std::path::Component;
    let racine = |p: &Path| match p.components().next() {
        Some(Component::Prefix(prefixe)) => {
            Some(prefixe.as_os_str().to_string_lossy().to_lowercase())
        }
        _ => None,
    };
    racine(a) == racine(b)
}

/// Vrai si les deux chemins sont sur le même système de fichiers (le plus
/// proche dossier existant de chacun).
#[cfg(not(windows))]
pub fn meme_disque(a: &Path, b: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    let peripherique = |p: &Path| {
        p.ancestors()
            .find_map(|d| fs::metadata(d).ok())
            .map(|m| m.dev())
    };
    peripherique(a) == peripherique(b)
}

/// Vrai si `chemin` est `dossier` ou se trouve dessous, sans tenir compte de
/// la casse (chemins Windows).
fn dans(chemin: &Path, dossier: &Path) -> bool {
    let minuscules = |p: &Path| PathBuf::from(p.to_string_lossy().to_lowercase());
    minuscules(chemin).starts_with(minuscules(dossier))
}

fn en_hexa(octets: &[u8]) -> String {
    octets.iter().map(|o| format!("{o:02x}")).collect()
}

fn depuis_hexa(texte: &str) -> Option<Vec<u8>> {
    if !texte.len().is_multiple_of(2) {
        return None;
    }
    (0..texte.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(texte.get(i..i + 2)?, 16).ok())
        .collect()
}

/// Sauvegardes du dossier réglé, la plus récente d'abord.
pub fn liste(donnees: &Path) -> Result<Vec<InfoSauvegarde>, String> {
    sauvegarde::liste_sauvegardes(&lire_reglage(donnees).dossier).map_err(|e| e.to_string())
}

/// Sauvegarde de la base `conn` dans le dossier réglé, avec l'identité TLS
/// du serveur (lue dans `donnees`). Genre automatique : les plus anciennes
/// au-delà de [`AUTOMATIQUES_GARDEES`] sont supprimées. Sans mot de passe :
/// erreur.
pub fn sauvegarder(conn: &Connection, donnees: &Path, genre: Genre) -> Result<PathBuf, String> {
    let reglage = lire_reglage(donnees);
    let mot_de_passe = reglage
        .mot_de_passe
        .as_deref()
        .ok_or("aucun mot de passe de sauvegarde n'est choisi")?;
    sauvegarder_avec(conn, donnees, &reglage.dossier, mot_de_passe, genre)
}

/// Comme [`sauvegarder`], avec le dossier et le mot de passe donnés.
pub(crate) fn sauvegarder_avec(
    conn: &Connection,
    donnees: &Path,
    dossier: &Path,
    mot_de_passe: &str,
    genre: Genre,
) -> Result<PathBuf, String> {
    let (certificat, cle) = identite(donnees)?;
    // Écrite d'abord dans un sous-dossier, puis renommée une fois complète :
    // une sauvegarde coupée (arrêt de Windows) n'entre jamais dans la liste
    // ni dans la rotation. Les restes d'une sauvegarde coupée sont effacés.
    let en_cours = dossier.join(SOUS_DOSSIER_EN_COURS);
    if let Ok(restes) = fs::read_dir(&en_cours) {
        for reste in restes.flatten() {
            let _ = fs::remove_file(reste.path());
        }
    }
    let provisoire = sauvegarde::ecrire(conn, &en_cours, mot_de_passe, genre)
        .map_err(|e| format!("sauvegarde impossible : {e}"))?;
    let complete = ajouter_identite(&provisoire, mot_de_passe, &certificat, &cle).and_then(|()| {
        let nom = provisoire
            .file_name()
            .ok_or_else(|| "nom de sauvegarde manquant".to_string())?
            .to_os_string();
        let chemin = dossier.join(nom);
        if chemin.exists() {
            return Err(format!("{} existe déjà", chemin.display()));
        }
        fs::rename(&provisoire, &chemin).map_err(|e| e.to_string())?;
        Ok(chemin)
    });
    let chemin = match complete {
        Ok(chemin) => chemin,
        Err(e) => {
            let _ = fs::remove_file(&provisoire);
            return Err(format!("sauvegarde impossible : {e}"));
        }
    };
    let gardees = match genre {
        Genre::Automatique => Some(AUTOMATIQUES_GARDEES),
        Genre::Arret => Some(ARRETS_GARDES),
        Genre::Manuelle | Genre::AvantRestauration => None,
    };
    if let Some(nombre) = gardees {
        sauvegarde::garder(dossier, genre, nombre)
            .map_err(|e| format!("rotation des sauvegardes impossible : {e}"))?;
    }
    Ok(chemin)
}

/// Certificat et clé privée TLS du serveur, en clair.
fn identite(donnees: &Path) -> Result<(Vec<u8>, Vec<u8>), String> {
    let certificat = fs::read(donnees.join(NOM_CERTIFICAT))
        .map_err(|e| format!("certificat du serveur illisible : {e}"))?;
    let protegee = fs::read(donnees.join(NOM_CLE_TLS))
        .map_err(|e| format!("clé TLS du serveur illisible : {e}"))?;
    let cle = chiffrement::deproteger(&protegee)
        .ok_or("clé TLS du serveur illisible sur cette machine")?;
    Ok((certificat, cle))
}

fn ajouter_identite(
    sauvegarde: &Path,
    mot_de_passe: &str,
    certificat: &[u8],
    cle: &[u8],
) -> Result<(), String> {
    let conn = Connection::open(sauvegarde).map_err(|e| e.to_string())?;
    chiffrement::appliquer_cle(&conn, mot_de_passe).map_err(|e| e.to_string())?;
    conn.execute_batch(&format!(
        "CREATE TABLE {TABLE_IDENTITE} (
             id INTEGER PRIMARY KEY CHECK (id = 1),
             certificat BLOB NOT NULL,
             cle BLOB NOT NULL
         ) STRICT"
    ))
    .map_err(|e| e.to_string())?;
    conn.execute(
        &format!("INSERT INTO {TABLE_IDENTITE} (id, certificat, cle) VALUES (1, ?1, ?2)"),
        rusqlite::params![certificat, cle],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Date de la plus récente sauvegarde automatique du dossier.
fn derniere_automatique(dossier: &Path) -> Option<NaiveDateTime> {
    sauvegarde::liste_sauvegardes(dossier)
        .ok()?
        .into_iter()
        .find(|s| s.genre == Genre::Automatique)
        .map(|s| s.date)
}

/// La sauvegarde quotidienne est-elle due ? Oui si aucune sauvegarde
/// automatique n'a été faite depuis la dernière heure prévue (aujourd'hui si
/// elle est passée, sinon hier).
pub(crate) fn due(
    maintenant: NaiveDateTime,
    heure: NaiveTime,
    derniere: Option<NaiveDateTime>,
) -> bool {
    let mut prevue = maintenant.date().and_time(heure);
    if maintenant < prevue {
        prevue -= chrono::Duration::days(1);
    }
    derniere.is_none_or(|d| d < prevue)
}

/// Pause entre deux examens de la planification.
pub(crate) const EXAMEN: Duration = Duration::from_secs(60);
/// Attente après un échec avant le prochain essai.
pub(crate) const APRES_ECHEC: Duration = Duration::from_secs(3600);

/// Sauvegarde quotidienne à faire maintenant, d'après le réglage.
pub(crate) fn quotidienne_due(donnees: &Path) -> bool {
    let reglage = lire_reglage(donnees);
    reglage.mot_de_passe.is_some()
        && due(
            Local::now().naive_local(),
            reglage.heure,
            derniere_automatique(&reglage.dossier),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn moment(texte: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(texte, "%Y-%m-%d %H:%M").unwrap()
    }

    #[test]
    fn sauvegarde_quotidienne_due() {
        let midi = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
        // Jamais sauvegardé : due tout de suite.
        assert!(due(moment("2026-11-15 08:00"), midi, None));
        // Avant l'heure, la sauvegarde d'hier après midi suffit.
        assert!(!due(
            moment("2026-11-15 08:00"),
            midi,
            Some(moment("2026-11-14 12:00"))
        ));
        // Après l'heure : due tant qu'aucune n'est faite depuis midi.
        assert!(due(
            moment("2026-11-15 12:01"),
            midi,
            Some(moment("2026-11-14 18:30"))
        ));
        assert!(!due(
            moment("2026-11-15 15:00"),
            midi,
            Some(moment("2026-11-15 12:00"))
        ));
        // Machine éteinte hier à l'heure dite : rattrapée au démarrage.
        assert!(due(
            moment("2026-11-15 08:00"),
            midi,
            Some(moment("2026-11-13 18:00"))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn disques() {
        assert!(meme_disque(
            Path::new(r"C:\ProgramData\Kadenz\Sauvegardes"),
            Path::new(r"c:\ProgramData\Kadenz\Serveur")
        ));
        assert!(!meme_disque(
            Path::new(r"D:\Sauvegardes"),
            Path::new(r"C:\ProgramData\Kadenz\Serveur")
        ));
        assert!(!meme_disque(
            Path::new(r"\\nas\partage\Kadenz"),
            Path::new(r"C:\ProgramData\Kadenz\Serveur")
        ));
    }

    #[test]
    fn reglage_puis_avertissements() {
        let donnees =
            std::env::temp_dir().join(format!("kadenz-serveur-reglage-{}", std::process::id()));
        let _ = fs::remove_dir_all(&donnees);
        fs::create_dir_all(&donnees).unwrap();
        // Sans réglage : défauts, aucun mot de passe.
        let lu = lire_reglage(&donnees);
        assert_eq!(lu.dossier, dossier_par_defaut());
        assert_eq!(lu.heure, heure_par_defaut());
        assert_eq!(lu.mot_de_passe, None);
        assert_eq!(
            avertissement(&donnees),
            Some(AvertissementSauvegarde::SansMotDePasse)
        );
        let a_cote =
            donnees.with_file_name(format!("kadenz-serveur-sauvegardes-{}", std::process::id()));
        let sept_heures = NaiveTime::from_hms_opt(7, 30, 0).unwrap();
        assert!(enregistrer_reglage(&donnees, &a_cote, sept_heures, Some("court")).is_err());
        assert!(enregistrer_reglage(&donnees, &donnees.join("dedans"), sept_heures, None).is_err());
        assert!(enregistrer_reglage(&donnees, Path::new("relatif"), sept_heures, None).is_err());
        enregistrer_reglage(
            &donnees,
            &a_cote,
            sept_heures,
            Some("motdepasse-sauvegardes"),
        )
        .unwrap();
        let lu = lire_reglage(&donnees);
        assert_eq!(lu.dossier, a_cote);
        assert_eq!(lu.heure, sept_heures);
        assert_eq!(lu.mot_de_passe.as_deref(), Some("motdepasse-sauvegardes"));
        // Même disque (dossier temporaire) : avertissement permanent.
        assert_eq!(
            avertissement(&donnees),
            Some(AvertissementSauvegarde::MemeDisque)
        );
        // Changer l'heure sans redonner le mot de passe le garde.
        enregistrer_reglage(&donnees, &a_cote, heure_par_defaut(), None).unwrap();
        assert_eq!(
            lire_reglage(&donnees).mot_de_passe.as_deref(),
            Some("motdepasse-sauvegardes")
        );
        let _ = fs::remove_dir_all(&donnees);
        let _ = fs::remove_dir_all(&a_cote);
    }
}
