//! Sauvegardes de la base (cahier des charges, section 10.3 ; décisions de
//! l'utilisateur du 2026-11-12).
//!
//! - Le réglage (dossier, mot de passe) vit hors de la base, dans
//!   `sauvegarde.reglage` à côté d'elle : il reste lisible quand la base ne
//!   l'est plus. Le mot de passe y est protégé par Windows (DPAPI) pour le
//!   compte courant ; illisible ailleurs, il doit alors être saisi.
//! - Dossier par défaut : `Documents\Kadenz\Sauvegardes`, hors du dossier des
//!   données que la désinstallation efface.
//! - Chaque sauvegarde est chiffrée par le mot de passe (voir
//!   [`crate::sauvegarder`]) et nommée `kadenz-<genre>-AAAAMMJJ-HHMMSS.kzs` ;
//!   seuls les fichiers de ce motif sont listés ou supprimés, le dossier peut
//!   contenir autre chose sans risque.
//! - Seules les [`AUTOMATIQUES_GARDEES`] dernières sauvegardes automatiques sont
//!   conservées ; les manuelles et celles d'avant restauration ne sont jamais
//!   supprimées.
//! - La restauration ne supprime jamais rien : l'ancienne base et, si elle est
//!   illisible, l'ancienne clé sont renommées en `.avant-restauration-<horodatage>`.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{Local, NaiveDateTime};
use kadenz_domaine::role::Droit;
use rusqlite::{Connection, OpenFlags};
use serde_json::{Value, json};

use crate::audit::{self, Entree};
use crate::chiffrement::{self, avec_suffixe, avec_suffixe_brut, dpapi};
use crate::comptes::{LONGUEUR_MIN_MOT_DE_PASSE, Utilisateur};
use crate::droits::exiger;
use crate::{Erreur, Resultat, ouvrir_sauvegarde, refus, sauvegarder, version_schema_application};

/// Nombre de sauvegardes automatiques conservées (section 10.3).
pub const AUTOMATIQUES_GARDEES: usize = 10;

const EXTENSION: &str = "kzs";
const FICHIER_REGLAGE: &str = "sauvegarde.reglage";
/// Horodatage des noms de fichiers : triable, sans caractère interdit par Windows.
const FORMAT_DATE: &str = "%Y%m%d-%H%M%S";

/// Réglage des sauvegardes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reglage {
    pub dossier: PathBuf,
    /// `None` : aucun mot de passe choisi, ou mot de passe protégé pour un autre
    /// compte Windows ; la sauvegarde automatique ne se fait pas.
    pub mot_de_passe: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Genre {
    Automatique,
    Manuelle,
    AvantRestauration,
}

impl Genre {
    const TOUS: [Genre; 3] = [
        Genre::Automatique,
        Genre::Manuelle,
        Genre::AvantRestauration,
    ];

    fn prefixe(self) -> &'static str {
        match self {
            Genre::Automatique => "kadenz-auto-",
            Genre::Manuelle => "kadenz-manuelle-",
            Genre::AvantRestauration => "kadenz-avant-restauration-",
        }
    }
}

/// Une sauvegarde trouvée dans le dossier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoSauvegarde {
    pub chemin: PathBuf,
    /// Heure locale de la sauvegarde, lue dans le nom du fichier.
    pub date: NaiveDateTime,
    pub genre: Genre,
    /// Taille du fichier, en octets.
    pub taille: u64,
}

/// `Documents\Kadenz\Sauvegardes`, d'après le dossier Documents réel de
/// Windows (même redirigé, par exemple vers OneDrive).
pub fn dossier_par_defaut() -> PathBuf {
    documents().join("Kadenz").join("Sauvegardes")
}

#[cfg(windows)]
fn documents() -> PathBuf {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::Com::CoTaskMemFree;
    use windows_sys::Win32::UI::Shell::{
        FOLDERID_Documents, KF_FLAG_DEFAULT, SHGetKnownFolderPath,
    };

    let mut chemin: windows_sys::core::PWSTR = std::ptr::null_mut();
    // SAFETY : GUID et pointeur de sortie valides le temps de l'appel ; la
    // chaîne rendue, terminée par un zéro, est copiée puis libérée par
    // CoTaskMemFree, à appeler même en cas d'échec.
    let trouve = unsafe {
        let hr = SHGetKnownFolderPath(
            &FOLDERID_Documents,
            KF_FLAG_DEFAULT as u32,
            std::ptr::null_mut(),
            &mut chemin,
        );
        let trouve = (hr >= 0 && !chemin.is_null()).then(|| {
            let mut longueur = 0;
            while *chemin.add(longueur) != 0 {
                longueur += 1;
            }
            PathBuf::from(OsString::from_wide(std::slice::from_raw_parts(
                chemin, longueur,
            )))
        });
        CoTaskMemFree(chemin.cast());
        trouve
    };
    trouve.unwrap_or_else(|| profil("USERPROFILE"))
}

#[cfg(not(windows))]
fn documents() -> PathBuf {
    profil("HOME")
}

fn profil(variable: &str) -> PathBuf {
    std::env::var_os(variable)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Documents")
}

fn chemin_reglage(chemin_base: &Path) -> PathBuf {
    chemin_base.with_file_name(FICHIER_REGLAGE)
}

/// Contenu brut du fichier de réglage ; absent ou illisible : objet vide.
fn reglage_brut(chemin_base: &Path) -> Resultat<serde_json::Map<String, Value>> {
    match fs::read_to_string(chemin_reglage(chemin_base)) {
        Ok(texte) => Ok(serde_json::from_str::<Value>(&texte)
            .ok()
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Default::default()),
        Err(e) => Err(e.into()),
    }
}

/// Réglage des sauvegardes. Sans fichier de réglage (ou fichier endommagé) :
/// dossier par défaut, aucun mot de passe. Ne lit pas la base : utilisable
/// quand elle ne s'ouvre pas.
pub fn lire_reglage(chemin_base: &Path) -> Resultat<Reglage> {
    let brut = reglage_brut(chemin_base)?;
    let dossier = brut
        .get("dossier")
        .and_then(Value::as_str)
        .filter(|d| !d.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(dossier_par_defaut);
    let mot_de_passe = brut
        .get("mot_de_passe")
        .and_then(Value::as_str)
        .and_then(depuis_hexa)
        .and_then(|protege| dpapi::deproteger(&protege))
        .and_then(|octets| String::from_utf8(octets).ok());
    Ok(Reglage {
        dossier,
        mot_de_passe,
    })
}

/// Enregistre le dossier et, s'il est donné, le nouveau mot de passe des
/// sauvegardes ; `None` garde le mot de passe actuel. Le dossier est créé.
pub fn enregistrer_reglage(
    conn: &Connection,
    acteur: &Utilisateur,
    chemin_base: &Path,
    dossier: &Path,
    mot_de_passe: Option<&str>,
) -> Resultat<()> {
    exiger(conn, acteur, Droit::SauvegarderBase)?;
    if !dossier.is_absolute()
        || dossier
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(Erreur::Validation(
            "le dossier des sauvegardes doit être un chemin complet".into(),
        ));
    }
    if chemin_base
        .parent()
        .is_some_and(|donnees| dans(dossier, donnees))
    {
        return Err(Erreur::Validation(
            "le dossier des sauvegardes ne peut pas être dans le dossier des données de Kadenz, effacé à la désinstallation".into(),
        ));
    }
    let mut brut = reglage_brut(chemin_base)?;
    if let Some(mot_de_passe) = mot_de_passe {
        verifier_longueur(mot_de_passe)?;
        let protege = dpapi::proteger(mot_de_passe.as_bytes()).ok_or_else(|| {
            Erreur::Validation("protection du mot de passe par Windows impossible".into())
        })?;
        brut.insert("mot_de_passe".into(), Value::String(en_hexa(&protege)));
    }
    let dossier_texte = dossier.to_str().ok_or_else(|| {
        Erreur::Validation(
            "le dossier des sauvegardes contient des caractères non pris en charge".into(),
        )
    })?;
    fs::create_dir_all(dossier)?;
    brut.insert("dossier".into(), Value::String(dossier_texte.into()));

    let fichier = chemin_reglage(chemin_base);
    if let Some(parent) = fichier.parent() {
        fs::create_dir_all(parent)?;
    }
    let provisoire = avec_suffixe(&fichier, "nouveau");
    fs::write(&provisoire, Value::Object(brut).to_string())?;
    if let Err(e) = fs::rename(&provisoire, &fichier) {
        let _ = fs::remove_file(&provisoire);
        return Err(e.into());
    }
    audit::journaliser(
        conn,
        Entree {
            entite: "sauvegarde",
            entite_id: 0,
            action: "reglage",
            ancienne_valeur: None,
            nouvelle_valeur: Some(json!({
                "dossier": dossier_texte,
                "mot_de_passe_change": mot_de_passe.is_some(),
            })),
            utilisateur_id: Some(acteur.id),
        },
    )
}

/// Vrai si `chemin` est `dossier` ou se trouve dessous, sans tenir compte de
/// la casse (chemins Windows).
fn dans(chemin: &Path, dossier: &Path) -> bool {
    let minuscules = |p: &Path| PathBuf::from(p.to_string_lossy().to_lowercase());
    minuscules(chemin).starts_with(minuscules(dossier))
}

fn verifier_longueur(mot_de_passe: &str) -> Resultat<()> {
    if mot_de_passe.chars().count() < LONGUEUR_MIN_MOT_DE_PASSE {
        return Err(refus(
            "le mot de passe de la sauvegarde doit compter au moins {} caractères",
            &[&LONGUEUR_MIN_MOT_DE_PASSE],
        ));
    }
    Ok(())
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

/// Nom d'une nouvelle sauvegarde de ce genre, à l'heure actuelle ; deux
/// sauvegardes dans la même seconde prennent `-2`, `-3`… Le rang suit le plus
/// grand déjà présent, jamais un rang libéré par la rotation : l'ordre des
/// noms reste celui des sauvegardes.
fn nom_libre(dossier: &Path, genre: Genre) -> PathBuf {
    let maintenant = Local::now().naive_local();
    let racine = format!("{}{}", genre.prefixe(), maintenant.format(FORMAT_DATE));
    let plus_grand = fs::read_dir(dossier)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().to_str().and_then(analyser_nom))
        .filter(|(g, date, _)| {
            *g == genre
                && date.format(FORMAT_DATE).to_string()
                    == maintenant.format(FORMAT_DATE).to_string()
        })
        .map(|(_, _, rang)| rang)
        .max()
        .unwrap_or(0);
    let mut rang = plus_grand + 1;
    loop {
        let chemin = match rang {
            1 => dossier.join(format!("{racine}.{EXTENSION}")),
            _ => dossier.join(format!("{racine}-{rang}.{EXTENSION}")),
        };
        if !chemin.exists() {
            return chemin;
        }
        rang += 1;
    }
}

fn ecrire(
    conn: &Connection,
    dossier: &Path,
    mot_de_passe: &str,
    genre: Genre,
) -> Resultat<PathBuf> {
    fs::create_dir_all(dossier)?;
    let chemin = nom_libre(dossier, genre);
    if let Err(e) = sauvegarder(conn, &chemin, mot_de_passe) {
        let _ = fs::remove_file(&chemin);
        return Err(e);
    }
    Ok(chemin)
}

fn mot_de_passe_requis(reglage: &Reglage) -> Resultat<&str> {
    reglage
        .mot_de_passe
        .as_deref()
        .ok_or_else(|| Erreur::Validation("aucun mot de passe de sauvegarde n'est choisi".into()))
}

/// Sauvegarde automatique (à la fermeture de l'application), sans contrôle de
/// droit : c'est une action du système. `Ok(None)` si aucun mot de passe n'est
/// choisi. Seules les [`AUTOMATIQUES_GARDEES`] plus récentes sont conservées.
pub fn sauvegarde_auto(conn: &Connection, reglage: &Reglage) -> Resultat<Option<PathBuf>> {
    let Some(mot_de_passe) = reglage.mot_de_passe.as_deref() else {
        return Ok(None);
    };
    let chemin = ecrire(conn, &reglage.dossier, mot_de_passe, Genre::Automatique)?;
    for ancienne in liste_sauvegardes(&reglage.dossier)?
        .into_iter()
        .filter(|s| s.genre == Genre::Automatique)
        .skip(AUTOMATIQUES_GARDEES)
    {
        fs::remove_file(&ancienne.chemin)?;
    }
    Ok(Some(chemin))
}

/// Sauvegarde à la demande, hors de la rotation des automatiques.
pub fn sauvegarde_manuelle(
    conn: &Connection,
    acteur: &Utilisateur,
    reglage: &Reglage,
) -> Resultat<PathBuf> {
    exiger(conn, acteur, Droit::SauvegarderBase)?;
    let mot_de_passe = mot_de_passe_requis(reglage)?;
    let chemin = ecrire(conn, &reglage.dossier, mot_de_passe, Genre::Manuelle)?;
    audit::journaliser(
        conn,
        Entree {
            entite: "sauvegarde",
            entite_id: 0,
            action: "sauvegarde_manuelle",
            ancienne_valeur: None,
            nouvelle_valeur: Some(json!({ "fichier": nom_fichier(&chemin) })),
            utilisateur_id: Some(acteur.id),
        },
    )?;
    Ok(chemin)
}

fn nom_fichier(chemin: &Path) -> String {
    chemin
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Genre, date et rang (doublon dans la même seconde) d'un nom de sauvegarde.
fn analyser_nom(nom: &str) -> Option<(Genre, NaiveDateTime, u32)> {
    let corps = nom.strip_suffix(&format!(".{EXTENSION}"))?;
    Genre::TOUS.into_iter().find_map(|genre| {
        let reste = corps.strip_prefix(genre.prefixe())?;
        let (date, rang) = match reste.get(15..) {
            Some("") => (reste, 1),
            Some(suite) => (reste.get(..15)?, suite.strip_prefix('-')?.parse().ok()?),
            None => return None,
        };
        let date = NaiveDateTime::parse_from_str(date, FORMAT_DATE).ok()?;
        Some((genre, date, rang))
    })
}

/// Sauvegardes du dossier, la plus récente d'abord. Dossier absent : liste vide.
pub fn liste_sauvegardes(dossier: &Path) -> Resultat<Vec<InfoSauvegarde>> {
    let entrees = match fs::read_dir(dossier) {
        Ok(entrees) => entrees,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let mut trouvees = Vec::new();
    for entree in entrees {
        let entree = entree?;
        let Some((genre, date, rang)) = entree.file_name().to_str().and_then(analyser_nom) else {
            continue;
        };
        let meta = entree.metadata()?;
        if !meta.is_file() {
            continue;
        }
        trouvees.push((
            InfoSauvegarde {
                chemin: entree.path(),
                date,
                genre,
                taille: meta.len(),
            },
            rang,
        ));
    }
    trouvees.sort_by(|(a, ra), (b, rb)| (b.date, rb).cmp(&(a.date, ra)));
    Ok(trouvees.into_iter().map(|(info, _)| info).collect())
}

/// Remplace la base par une sauvegarde. La connexion de l'application à la
/// base doit être fermée avant l'appel ; rouvrir ensuite avec [`crate::ouvrir`].
///
/// 1. La sauvegarde s'ouvre avec `mot_de_passe` ; un schéma plus récent que
///    celui de l'application est refusé.
/// 2. La clé du poste est gardée si elle se lit ; sinon une nouvelle est créée.
/// 3. La base restaurée est construite à part, chiffrée par cette clé, puis
///    vérifiée (intégrité, mêmes comptes de lignes que la sauvegarde).
/// 4. Si la base courante s'ouvre, elle est sauvegardée (genre
///    [`Genre::AvantRestauration`], même mot de passe) dans le dossier du réglage.
/// 5. La base courante et ses fichiers `-wal`, `-shm` (et la clé illisible le
///    cas échéant) sont renommés en `.avant-restauration-<horodatage>`, jamais
///    supprimés ; la base restaurée prend leur place. Un échec remet tout en état.
pub fn restaurer(chemin_base: &Path, sauvegarde: &Path, mot_de_passe: &str) -> Resultat<()> {
    let reglage = lire_reglage(chemin_base)?;
    let source = ouvrir_sauvegarde(sauvegarde, mot_de_passe)?;
    let version: u32 = source.pragma_query_value(None, "user_version", |r| r.get(0))?;
    let application = version_schema_application();
    if version > application {
        return Err(Erreur::VersionSchemaInconnue {
            base: version,
            application,
        });
    }
    if let Some(dossier) = chemin_base.parent() {
        fs::create_dir_all(dossier)?;
    }

    let fichier_cle = chiffrement::chemin_cle(chemin_base);
    let (cle, nouvelle_cle) = match chiffrement::cle(&fichier_cle, false) {
        Ok(cle) => (cle, None),
        Err(Erreur::CleIntrouvable | Erreur::CleAutreCompte | Erreur::CleEndommagee) => {
            let (cle, protegee) = chiffrement::nouvelle_cle()?;
            (cle, Some(protegee))
        }
        Err(e) => return Err(e),
    };

    let preparee = avec_suffixe(chemin_base, "restauration");
    let _ = fs::remove_file(&preparee);
    if let Err(e) = preparer(&source, &preparee, &cle, sauvegarde) {
        let _ = fs::remove_file(&preparee);
        return Err(e);
    }
    drop(source);

    // Avec une clé illisible, la base courante ne peut pas s'ouvrir.
    // Une base sans compte (installation neuve, assistant pas encore fait)
    // n'a rien à protéger : pas de sauvegarde préalable, elle est seulement
    // mise de côté comme les autres.
    let courante = match nouvelle_cle {
        None => ouvrir_courante(chemin_base, &cle).filter(a_des_comptes),
        Some(_) => None,
    };
    if let Some(courante) = courante {
        let ecrite = ecrire(
            &courante,
            &reglage.dossier,
            mot_de_passe,
            Genre::AvantRestauration,
        );
        // Fermée avant l'échange : Windows refuse de renommer un fichier ouvert.
        drop(courante);
        if let Err(e) = ecrite {
            let _ = fs::remove_file(&preparee);
            return Err(e);
        }
    }

    let mut a_ecarter = vec![
        chemin_base.to_path_buf(),
        avec_suffixe_brut(chemin_base, "-wal"),
        avec_suffixe_brut(chemin_base, "-shm"),
    ];
    if nouvelle_cle.is_some() {
        a_ecarter.push(fichier_cle.clone());
    }
    // Un seul suffixe pour la base et sa clé, libre pour chacun des fichiers :
    // fs::rename remplacerait sans prévenir un fichier déjà mis de côté.
    let racine = format!(".avant-restauration-{}", Local::now().format(FORMAT_DATE));
    let mut suffixe = racine.clone();
    let mut rang = 2;
    while a_ecarter
        .iter()
        .any(|f| avec_suffixe_brut(f, &suffixe).exists())
    {
        suffixe = format!("{racine}-{rang}");
        rang += 1;
    }
    let mut ecartes: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut cle_ecrite = false;
    let echange = (|| -> Resultat<()> {
        for fichier in &a_ecarter {
            if fichier.exists() {
                let destination = avec_suffixe_brut(fichier, &suffixe);
                fs::rename(fichier, &destination)?;
                ecartes.push((fichier.clone(), destination));
            }
        }
        if let Some(protegee) = &nouvelle_cle {
            fs::write(&fichier_cle, protegee)?;
            cle_ecrite = true;
        }
        fs::rename(&preparee, chemin_base)?;
        Ok(())
    })();
    if let Err(e) = echange {
        // La clé retirée ici est celle que cette restauration vient d'écrire.
        if cle_ecrite {
            let _ = fs::remove_file(&fichier_cle);
        }
        for (origine, destination) in ecartes.iter().rev() {
            let _ = fs::rename(destination, origine);
        }
        let _ = fs::remove_file(&preparee);
        return Err(e);
    }
    Ok(())
}

/// Copie la sauvegarde dans `preparee`, chiffrée par `cle`, la vérifie, puis y
/// consigne la restauration au journal d'audit.
fn preparer(source: &Connection, preparee: &Path, cle: &str, sauvegarde: &Path) -> Resultat<()> {
    let mut cible = Connection::open(preparee)?;
    chiffrement::appliquer_cle(&cible, cle)?;
    rusqlite::backup::Backup::new(source, &mut cible)?.run_to_completion(
        256,
        std::time::Duration::ZERO,
        None,
    )?;
    let integrite: String = cible.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    if integrite != "ok" {
        return Err(refus("sauvegarde endommagée : {}", &[&integrite]));
    }
    if chiffrement::comptes_par_table(source)? != chiffrement::comptes_par_table(&cible)? {
        return Err(Erreur::Validation(
            "la base restaurée ne contient pas les mêmes données que la sauvegarde".into(),
        ));
    }
    audit::journaliser(
        &cible,
        Entree {
            entite: "sauvegarde",
            entite_id: 0,
            action: "restauration",
            ancienne_valeur: None,
            nouvelle_valeur: Some(json!({ "fichier": nom_fichier(sauvegarde) })),
            utilisateur_id: None,
        },
    )
}

/// Base courante ouverte avec la clé, sans migration ; `None` si elle n'existe
/// pas ou ne se déchiffre pas.
fn ouvrir_courante(chemin_base: &Path, cle: &str) -> Option<Connection> {
    if !chemin_base.is_file() {
        return None;
    }
    let conn = Connection::open_with_flags(chemin_base, OpenFlags::SQLITE_OPEN_READ_WRITE).ok()?;
    chiffrement::appliquer_cle(&conn, cle).ok()?;
    Some(conn)
}

/// Vrai si la base compte au moins un utilisateur ; dans le doute (table
/// absente, lecture impossible), vrai : la sauvegarde préalable a lieu.
fn a_des_comptes(conn: &Connection) -> bool {
    conn.query_row("SELECT EXISTS (SELECT 1 FROM utilisateur)", [], |r| {
        r.get::<_, bool>(0)
    })
    .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comptes;
    use crate::droits::tests::compte;
    use kadenz_domaine::role::Role;

    fn dossier_essai(nom: &str) -> PathBuf {
        let dossier =
            std::env::temp_dir().join(format!("kadenz-sauv-{nom}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dossier);
        fs::create_dir_all(&dossier).unwrap();
        dossier
    }

    const MOT_DE_PASSE: &str = "sauvegarde-essai";

    /// Base chiffrée réelle, avec un administrateur et une marque reconnaissable.
    fn base_avec_admin(dossier: &Path) -> (PathBuf, Connection, Utilisateur) {
        let chemin = dossier.join("donnees").join("kadenz.sqlite");
        let conn = crate::ouvrir(&chemin).unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        (chemin, conn, admin)
    }

    fn marquer(conn: &Connection, valeur: &str) {
        conn.execute(
            "INSERT OR REPLACE INTO parametre_systeme (cle, valeur) VALUES ('essai.marque', ?1)",
            [json!(valeur).to_string()],
        )
        .unwrap();
    }

    fn marque(conn: &Connection) -> String {
        let texte: String = conn
            .query_row(
                "SELECT valeur FROM parametre_systeme WHERE cle = 'essai.marque'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        serde_json::from_str(&texte).unwrap()
    }

    fn reglage(dossier: &Path) -> Reglage {
        Reglage {
            dossier: dossier.join("sauvegardes"),
            mot_de_passe: Some(MOT_DE_PASSE.into()),
        }
    }

    #[test]
    fn noms_analyses() {
        let d = NaiveDateTime::parse_from_str("20261112-143005", FORMAT_DATE).unwrap();
        assert_eq!(
            analyser_nom("kadenz-auto-20261112-143005.kzs"),
            Some((Genre::Automatique, d, 1))
        );
        assert_eq!(
            analyser_nom("kadenz-manuelle-20261112-143005-3.kzs"),
            Some((Genre::Manuelle, d, 3))
        );
        assert_eq!(
            analyser_nom("kadenz-avant-restauration-20261112-143005.kzs"),
            Some((Genre::AvantRestauration, d, 1))
        );
        for autre in [
            "kadenz-auto-20261112-143005.txt",
            "kadenz-auto-2026.kzs",
            "kadenz-auto-20261112-143005-x.kzs",
            "rapport.kzs",
            "kadenz.sqlite",
        ] {
            assert_eq!(analyser_nom(autre), None, "{autre}");
        }
    }

    #[test]
    fn hexa_aller_retour() {
        let octets = [0u8, 1, 0xab, 0xff];
        assert_eq!(depuis_hexa(&en_hexa(&octets)).unwrap(), octets);
        assert_eq!(depuis_hexa("abc"), None);
        assert_eq!(depuis_hexa("zz"), None);
    }

    #[test]
    fn reglage_enregistre_et_relu() {
        let dossier = dossier_essai("reglage");
        let (chemin, conn, admin) = base_avec_admin(&dossier);
        let defaut = lire_reglage(&chemin).unwrap();
        assert_eq!(defaut.dossier, dossier_par_defaut());
        assert_eq!(defaut.mot_de_passe, None);

        let cible = dossier.join("ailleurs");
        assert!(enregistrer_reglage(&conn, &admin, &chemin, Path::new("relatif"), None).is_err());
        assert!(enregistrer_reglage(&conn, &admin, &chemin, &cible, Some("court")).is_err());
        assert!(
            enregistrer_reglage(&conn, &admin, &chemin, &dossier.join("a").join(".."), None)
                .is_err()
        );
        // Jamais dans le dossier des données, effacé à la désinstallation.
        let donnees = chemin.parent().unwrap();
        for interdit in [donnees.to_path_buf(), donnees.join("Sauvegardes")] {
            assert!(enregistrer_reglage(&conn, &admin, &chemin, &interdit, None).is_err());
        }
        let majuscules = PathBuf::from(donnees.to_string_lossy().to_uppercase()).join("x");
        assert!(enregistrer_reglage(&conn, &admin, &chemin, &majuscules, None).is_err());
        enregistrer_reglage(&conn, &admin, &chemin, &cible, Some(MOT_DE_PASSE)).unwrap();
        assert!(cible.is_dir(), "dossier créé");
        let lu = lire_reglage(&chemin).unwrap();
        assert_eq!(lu.dossier, cible);
        assert_eq!(lu.mot_de_passe.as_deref(), Some(MOT_DE_PASSE));
        // Le mot de passe n'est pas en clair dans le fichier.
        let texte = fs::read_to_string(chemin_reglage(&chemin)).unwrap();
        assert!(!texte.contains(MOT_DE_PASSE));

        // None garde le mot de passe.
        let autre = dossier.join("autre");
        enregistrer_reglage(&conn, &admin, &chemin, &autre, None).unwrap();
        let lu = lire_reglage(&chemin).unwrap();
        assert_eq!(lu.dossier, autre);
        assert_eq!(lu.mot_de_passe.as_deref(), Some(MOT_DE_PASSE));
        assert_eq!(audit::compter(&conn, "reglage").unwrap(), 2);

        // Fichier endommagé : retour aux valeurs par défaut, sans erreur.
        fs::write(chemin_reglage(&chemin), "pas du json").unwrap();
        assert_eq!(lire_reglage(&chemin).unwrap(), defaut);
    }

    #[test]
    fn reglage_refuse_sans_le_droit() {
        let dossier = dossier_essai("droit");
        let (chemin, conn, _admin) = base_avec_admin(&dossier);
        let vente = compte(&conn, "vente", Role::Commercial);
        let r = enregistrer_reglage(&conn, &vente, &chemin, &dossier, Some(MOT_DE_PASSE));
        assert!(matches!(r, Err(Erreur::Interdit(Droit::SauvegarderBase))));
        let r = sauvegarde_manuelle(&conn, &vente, &reglage(&dossier));
        assert!(matches!(r, Err(Erreur::Interdit(Droit::SauvegarderBase))));
    }

    #[test]
    fn automatiques_limitees_a_dix() {
        let dossier = dossier_essai("rotation");
        let (_, conn, admin) = base_avec_admin(&dossier);
        let reglage = reglage(&dossier);
        assert_eq!(
            sauvegarde_auto(
                &conn,
                &Reglage {
                    mot_de_passe: None,
                    ..reglage.clone()
                }
            )
            .unwrap(),
            None,
            "sans mot de passe, pas de sauvegarde automatique"
        );
        let manuelle = sauvegarde_manuelle(&conn, &admin, &reglage).unwrap();
        // Fichier étranger au motif : jamais touché.
        let etranger = reglage.dossier.join("kadenz-auto-notes.kzs");
        fs::write(&etranger, "à garder").unwrap();
        let mut faites = Vec::new();
        for _ in 0..12 {
            faites.push(sauvegarde_auto(&conn, &reglage).unwrap().unwrap());
        }
        let liste = liste_sauvegardes(&reglage.dossier).unwrap();
        let auto: Vec<_> = liste
            .iter()
            .filter(|s| s.genre == Genre::Automatique)
            .map(|s| s.chemin.clone())
            .collect();
        assert_eq!(auto.len(), AUTOMATIQUES_GARDEES);
        // Les plus récentes, la dernière d'abord.
        let attendues: Vec<_> = faites
            .iter()
            .rev()
            .take(AUTOMATIQUES_GARDEES)
            .cloned()
            .collect();
        assert_eq!(auto, attendues);
        assert!(manuelle.is_file(), "la manuelle reste");
        assert!(etranger.is_file(), "le fichier étranger reste");
        assert!(liste.iter().all(|s| s.taille > 0));
        // Chaque sauvegarde s'ouvre avec le mot de passe.
        ouvrir_sauvegarde(&auto[0], MOT_DE_PASSE).unwrap();
        assert!(ouvrir_sauvegarde(&auto[0], "mauvais-mot-de-passe").is_err());
    }

    #[test]
    fn restauration_avec_cle_lisible() {
        let dossier = dossier_essai("restauration");
        let (chemin, conn, admin) = base_avec_admin(&dossier);
        let reglage = reglage(&dossier);
        enregistrer_reglage(&conn, &admin, &chemin, &reglage.dossier, Some(MOT_DE_PASSE)).unwrap();
        marquer(&conn, "avant");
        let sauvegarde = sauvegarde_manuelle(&conn, &admin, &reglage).unwrap();
        marquer(&conn, "après");
        let cle_avant = fs::read(chiffrement::chemin_cle(&chemin)).unwrap();

        assert!(restaurer(&chemin, &sauvegarde, "mauvais-mot-de-passe").is_err());
        drop(conn);
        restaurer(&chemin, &sauvegarde, MOT_DE_PASSE).unwrap();

        assert_eq!(
            fs::read(chiffrement::chemin_cle(&chemin)).unwrap(),
            cle_avant,
            "clé du poste gardée"
        );
        let conn = crate::ouvrir(&chemin).unwrap();
        assert_eq!(marque(&conn), "avant");
        assert_eq!(audit::compter(&conn, "restauration").unwrap(), 1);
        // La base d'avant est sauvegardée et aussi gardée sur place.
        let avant: Vec<_> = liste_sauvegardes(&reglage.dossier)
            .unwrap()
            .into_iter()
            .filter(|s| s.genre == Genre::AvantRestauration)
            .collect();
        assert_eq!(avant.len(), 1);
        assert_eq!(
            marque(&ouvrir_sauvegarde(&avant[0].chemin, MOT_DE_PASSE).unwrap()),
            "après"
        );
        let ecartees = fs::read_dir(chemin.parent().unwrap())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("kadenz.sqlite.avant-restauration-")
            })
            .count();
        assert_eq!(ecartees, 1, "ancienne base renommée, pas supprimée");
    }

    #[test]
    fn restauration_cle_perdue() {
        let dossier = dossier_essai("cle-perdue");
        let (chemin, conn, admin) = base_avec_admin(&dossier);
        marquer(&conn, "sauvé");
        let sauvegarde = sauvegarde_manuelle(&conn, &admin, &reglage(&dossier)).unwrap();
        drop(conn);
        let fichier_cle = chiffrement::chemin_cle(&chemin);

        // Clé illisible (autre compte) : l'ouverture le dit, la restauration
        // met la clé de côté et en crée une nouvelle.
        fs::write(&fichier_cle, b"pas une cle protegee").unwrap();
        assert!(matches!(
            crate::ouvrir(&chemin),
            Err(Erreur::CleAutreCompte)
        ));
        restaurer(&chemin, &sauvegarde, MOT_DE_PASSE).unwrap();
        let conn = crate::ouvrir(&chemin).unwrap();
        assert_eq!(marque(&conn), "sauvé");
        drop(conn);
        let ecartees: Vec<String> = fs::read_dir(chemin.parent().unwrap())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with("kadenz.cle.avant-restauration-"))
            .collect();
        assert_eq!(ecartees.len(), 1, "ancienne clé gardée : {ecartees:?}");

        // Clé disparue : erreur typée, puis restauration.
        fs::remove_file(&fichier_cle).unwrap();
        assert!(matches!(
            crate::ouvrir(&chemin),
            Err(Erreur::CleIntrouvable)
        ));
        restaurer(&chemin, &sauvegarde, MOT_DE_PASSE).unwrap();
        assert_eq!(marque(&crate::ouvrir(&chemin).unwrap()), "sauvé");
        // Deux restaurations, souvent dans la même seconde : rien n'est écrasé.
        let bases = fs::read_dir(chemin.parent().unwrap())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let nom = e.file_name().to_string_lossy().into_owned();
                nom.starts_with("kadenz.sqlite.avant-restauration-")
                    && !nom.contains("-wal")
                    && !nom.contains("-shm")
            })
            .count();
        assert_eq!(bases, 2, "chaque base écartée est gardée");
    }

    #[test]
    fn sauvegarde_ouverte_sans_fichiers_a_cote() {
        let dossier = dossier_essai("sans-wal");
        let (chemin, conn, admin) = base_avec_admin(&dossier);
        let sauvegarde = sauvegarde_manuelle(&conn, &admin, &reglage(&dossier)).unwrap();
        let a_cote = |suffixe: &str| avec_suffixe_brut(&sauvegarde, suffixe).exists();
        let mode = |c: &Connection| -> String {
            c.pragma_query_value(None, "journal_mode", |r| r.get(0))
                .unwrap()
        };
        assert_eq!(
            mode(&ouvrir_sauvegarde(&sauvegarde, MOT_DE_PASSE).unwrap()),
            "delete"
        );

        // Sauvegarde restée en mode WAL (faite avant ce correctif) : l'ouverture
        // immuable ne crée rien non plus, et la restauration n'y laisse rien.
        {
            let s = Connection::open(&sauvegarde).unwrap();
            chiffrement::appliquer_cle(&s, MOT_DE_PASSE).unwrap();
            s.pragma_update(None, "journal_mode", "WAL").unwrap();
        }
        assert!(!a_cote("-wal") && !a_cote("-shm"));
        let avant = fs::read(&sauvegarde).unwrap();
        let ouverte = ouvrir_sauvegarde(&sauvegarde, MOT_DE_PASSE).unwrap();
        assert_eq!(comptes::lister(&ouverte).unwrap().len(), 1);
        drop(ouverte);
        drop(conn);
        restaurer(&chemin, &sauvegarde, MOT_DE_PASSE).unwrap();
        assert!(!a_cote("-wal"), "aucun -wal à côté de la sauvegarde");
        assert!(!a_cote("-shm"), "aucun -shm à côté de la sauvegarde");
        assert_eq!(fs::read(&sauvegarde).unwrap(), avant, "sauvegarde intacte");
    }

    #[test]
    fn base_neuve_sans_sauvegarde_prealable() {
        let dossier = dossier_essai("neuve-assistant");
        let (_, conn, admin) = base_avec_admin(&dossier);
        marquer(&conn, "à restaurer");
        let reglage = reglage(&dossier);
        let sauvegarde = sauvegarde_manuelle(&conn, &admin, &reglage).unwrap();
        drop(conn);

        // Réinstallation à neuf : base ouverte, aucun compte encore.
        let neuve = dossier.join("neuve").join("kadenz.sqlite");
        drop(crate::ouvrir(&neuve).unwrap());
        fs::write(
            chemin_reglage(&neuve),
            json!({ "dossier": reglage.dossier.to_str().unwrap() }).to_string(),
        )
        .unwrap();
        restaurer(&neuve, &sauvegarde, MOT_DE_PASSE).unwrap();
        assert_eq!(marque(&crate::ouvrir(&neuve).unwrap()), "à restaurer");
        assert!(
            liste_sauvegardes(&reglage.dossier)
                .unwrap()
                .iter()
                .all(|s| s.genre != Genre::AvantRestauration),
            "rien à protéger dans une base sans compte"
        );
    }

    #[test]
    fn base_indechiffrable_typee() {
        let dossier = dossier_essai("indechiffrable");
        let (chemin, conn, _) = base_avec_admin(&dossier);
        drop(conn);
        // Clé valide mais d'une autre base.
        let (_, protegee) = chiffrement::nouvelle_cle().unwrap();
        fs::write(chiffrement::chemin_cle(&chemin), protegee).unwrap();
        assert!(matches!(
            crate::ouvrir(&chemin),
            Err(Erreur::BaseIndechiffrable)
        ));
    }

    #[test]
    fn schema_plus_recent_refuse() {
        let dossier = dossier_essai("schema");
        let (chemin, conn, admin) = base_avec_admin(&dossier);
        let sauvegarde = sauvegarde_manuelle(&conn, &admin, &reglage(&dossier)).unwrap();
        drop(conn);
        {
            let s = Connection::open(&sauvegarde).unwrap();
            chiffrement::appliquer_cle(&s, MOT_DE_PASSE).unwrap();
            s.pragma_update(None, "user_version", version_schema_application() + 1)
                .unwrap();
        }
        let r = restaurer(&chemin, &sauvegarde, MOT_DE_PASSE);
        assert!(matches!(r, Err(Erreur::VersionSchemaInconnue { .. })));
        crate::ouvrir(&chemin).expect("base courante intacte");
    }
}
