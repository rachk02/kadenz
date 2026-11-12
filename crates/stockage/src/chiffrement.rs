//! Chiffrement de la base (cahier des charges, section 13 ; décision de
//! l'utilisateur du 2026-11-11).
//!
//! Le moteur est SQLite3 Multiple Ciphers (libsqlite3-sys patché, voir
//! `vendor/libsqlite3-sys`), chiffrement ChaCha20-Poly1305 de toute la base.
//! La clé, 32 octets tirés de l'aléa du système, est conservée dans
//! `kadenz.cle` à côté de la base, protégée par Windows (DPAPI) pour le compte
//! courant : rien à saisir, et le fichier copié sur un autre poste ou un autre
//! compte est illisible. Perdre `kadenz.cle` ou le profil Windows rend la base
//! illisible : seule une sauvegarde protégée par mot de passe permet alors de
//! retrouver les données.

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};

use crate::{Erreur, Resultat};

/// Algorithme de SQLite3 Multiple Ciphers, fixé explicitement pour ne pas
/// dépendre de la valeur par défaut d'une version future.
const CHIFFRE: &str = "chacha20";
/// Début de tout fichier SQLite en clair.
const ENTETE_EN_CLAIR: &[u8; 16] = b"SQLite format 3\0";

/// Fichier de la clé protégée, à côté de la base (`kadenz.cle`).
pub fn chemin_cle(base: &Path) -> PathBuf {
    base.with_extension("cle")
}

/// Vrai si le fichier existe, n'est pas vide et commence par l'en-tête d'une
/// base SQLite en clair.
pub fn en_clair(base: &Path) -> Resultat<bool> {
    let mut entete = [0u8; 16];
    match File::open(base) {
        Ok(mut f) => Ok(f.read(&mut entete)? == 16 && &entete == ENTETE_EN_CLAIR),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// Applique la clé à une connexion qui vient d'être ouverte, puis vérifie
/// qu'elle déchiffre la base (une mauvaise clé échoue ici).
pub fn appliquer_cle(conn: &Connection, cle: &str) -> Resultat<()> {
    conn.pragma_update(None, "cipher", CHIFFRE)?;
    conn.pragma_update(None, "key", cle)?;
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(()))
        .map_err(|_| Erreur::BaseIndechiffrable)
}

/// Clé de la base, lue dans `kadenz.cle`. Absente : créée si `creer`, sinon
/// erreur (base chiffrée dont la clé a disparu).
pub fn cle(fichier: &Path, creer: bool) -> Resultat<String> {
    match fs::read(fichier) {
        Ok(protegee) => {
            let octets = dpapi::deproteger(&protegee).ok_or(Erreur::CleAutreCompte)?;
            String::from_utf8(octets).map_err(|_| Erreur::CleEndommagee)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && creer => {
            let (cle, protegee) = nouvelle_cle()?;
            fs::write(fichier, protegee)?;
            Ok(cle)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(Erreur::CleIntrouvable),
        Err(e) => Err(e.into()),
    }
}

/// Nouvelle clé tirée de l'aléa du système : la clé, et sa forme protégée par
/// Windows à écrire dans `kadenz.cle`.
pub(crate) fn nouvelle_cle() -> Resultat<(String, Vec<u8>)> {
    let mut alea = [0u8; 32];
    getrandom::fill(&mut alea)
        .map_err(|e| Erreur::Validation(format!("aléa du système indisponible : {e}")))?;
    let cle: String = alea.iter().map(|o| format!("{o:02x}")).collect();
    let protegee = dpapi::proteger(cle.as_bytes())
        .ok_or_else(|| Erreur::Validation("protection de la clé par Windows impossible".into()))?;
    Ok((cle, protegee))
}

/// Chiffre une base existante en clair, sans jamais toucher à l'original avant
/// d'avoir vérifié la copie chiffrée :
/// 1. journal WAL reversé dans la base, mode de journal classique ;
/// 2. copie du fichier, chiffrée sur place (`PRAGMA rekey`) ;
/// 3. copie rouverte avec la clé : intégrité, et même nombre de lignes que
///    l'original dans chaque table ;
/// 4. l'original est écarté, la copie prend sa place, puis l'original est
///    supprimé avec ses fichiers -wal et -shm.
pub fn chiffrer_base_en_clair(base: &Path, cle: &str) -> Resultat<()> {
    {
        let conn = Connection::open(base)?;
        conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))?;
        conn.pragma_update(None, "journal_mode", "DELETE")?;
    }
    let copie = avec_suffixe(base, "chiffrement");
    let ecartee = avec_suffixe(base, "en-clair");
    let _ = fs::remove_file(&copie);
    fs::copy(base, &copie)?;

    let verification = (|| -> Resultat<()> {
        {
            let conn = Connection::open(&copie)?;
            conn.pragma_update(None, "cipher", CHIFFRE)?;
            conn.pragma_update(None, "rekey", cle)?;
        }
        if en_clair(&copie)? {
            return Err(Erreur::Validation(
                "le chiffrement de la base n'a pas eu lieu".into(),
            ));
        }
        let chiffree = Connection::open_with_flags(&copie, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        appliquer_cle(&chiffree, cle)?;
        let integrite: String = chiffree.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        if integrite != "ok" {
            return Err(Erreur::Validation(format!(
                "base chiffrée incohérente : {integrite}"
            )));
        }
        let originale = Connection::open_with_flags(base, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        if comptes_par_table(&originale)? != comptes_par_table(&chiffree)? {
            return Err(Erreur::Validation(
                "la base chiffrée ne contient pas les mêmes données que l'originale".into(),
            ));
        }
        Ok(())
    })();
    if let Err(e) = verification {
        let _ = fs::remove_file(&copie);
        return Err(e);
    }

    let _ = fs::remove_file(&ecartee);
    fs::rename(base, &ecartee)?;
    if let Err(e) = fs::rename(&copie, base) {
        let _ = fs::rename(&ecartee, base);
        return Err(e.into());
    }
    let _ = fs::remove_file(&ecartee);
    for suffixe in ["-wal", "-shm"] {
        let _ = fs::remove_file(avec_suffixe_brut(base, suffixe));
    }
    Ok(())
}

/// Nombre de lignes de chaque table, par ordre de nom.
pub(crate) fn comptes_par_table(conn: &Connection) -> Resultat<Vec<(String, i64)>> {
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name")?
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    tables
        .into_iter()
        .map(|t| {
            let n = conn.query_row(&format!("SELECT count(*) FROM \"{t}\""), [], |r| r.get(0))?;
            Ok((t, n))
        })
        .collect()
}

pub(crate) fn avec_suffixe(base: &Path, suffixe: &str) -> PathBuf {
    avec_suffixe_brut(base, &format!(".{suffixe}"))
}

pub(crate) fn avec_suffixe_brut(base: &Path, suffixe: &str) -> PathBuf {
    let mut nom = base.as_os_str().to_owned();
    nom.push(suffixe);
    PathBuf::from(nom)
}

#[cfg(windows)]
pub(crate) mod dpapi {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
    };

    /// Libellé enregistré avec la donnée protégée.
    const DESCRIPTION: &[u16] = &[
        'K' as u16, 'a' as u16, 'd' as u16, 'e' as u16, 'n' as u16, 'z' as u16, 0,
    ];

    fn sortie(blob: CRYPT_INTEGER_BLOB) -> Vec<u8> {
        // SAFETY : pbData pointe vers cbData octets alloués par Windows,
        // libérés juste après la copie.
        unsafe {
            let octets = std::slice::from_raw_parts(blob.pbData, blob.cbData as usize).to_vec();
            LocalFree(blob.pbData.cast());
            octets
        }
    }

    pub fn proteger(donnee: &[u8]) -> Option<Vec<u8>> {
        let entree = CRYPT_INTEGER_BLOB {
            cbData: u32::try_from(donnee.len()).ok()?,
            pbData: donnee.as_ptr().cast_mut(),
        };
        let mut resultat = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        // SAFETY : blobs valides le temps de l'appel ; Windows ne modifie pas l'entrée.
        let ok = unsafe {
            CryptProtectData(
                &entree,
                DESCRIPTION.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut resultat,
            )
        };
        (ok != 0).then(|| sortie(resultat))
    }

    pub fn deproteger(donnee: &[u8]) -> Option<Vec<u8>> {
        let entree = CRYPT_INTEGER_BLOB {
            cbData: u32::try_from(donnee.len()).ok()?,
            pbData: donnee.as_ptr().cast_mut(),
        };
        let mut resultat = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        // SAFETY : comme pour proteger.
        let ok = unsafe {
            CryptUnprotectData(
                &entree,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut resultat,
            )
        };
        (ok != 0).then(|| sortie(resultat))
    }
}

/// Hors de Windows (développement seulement) : clé conservée telle quelle.
#[cfg(not(windows))]
pub(crate) mod dpapi {
    pub fn proteger(donnee: &[u8]) -> Option<Vec<u8>> {
        Some(donnee.to_vec())
    }
    pub fn deproteger(donnee: &[u8]) -> Option<Vec<u8>> {
        Some(donnee.to_vec())
    }
}
