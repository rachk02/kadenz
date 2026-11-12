//! Droits du dossier des données du serveur (spécification de synchronisation,
//! section 8). La clé de la base est protégée par Windows au niveau de la
//! machine : n'importe quel compte qui lit le fichier peut la déchiffrer. La
//! protection repose donc sur les droits du dossier, réservé à SYSTEM et aux
//! Administrateurs, posés à la création et vérifiés à chaque démarrage.

use std::path::Path;

/// Droits voulus, au format SDDL : accès complet pour SYSTEM (SY) et les
/// Administrateurs (BA), hérité par les fichiers et sous-dossiers, sans
/// héritage du dossier parent (P).
#[cfg(windows)]
const SDDL: &str = "D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)";

/// Comptes qui peuvent recevoir un droit sur le dossier.
const PERMIS: [&str; 2] = ["SY", "BA"];

fn large(sid: &str) -> bool {
    !PERMIS.contains(&sid)
}

/// Comptes, autres que SYSTEM et les Administrateurs, qui ont un droit
/// d'accès d'après une liste de contrôle au format SDDL.
/// Une liste absente (`NO_ACCESS_CONTROL`) ouvre le dossier à tous (WD).
pub fn comptes_en_trop(sddl: &str) -> Vec<String> {
    if sddl.contains("NO_ACCESS_CONTROL") {
        return vec!["WD".to_string()];
    }
    sddl.split('(')
        .skip(1)
        .filter_map(|ace| {
            let champs: Vec<&str> = ace.trim_end_matches(')').split(';').collect();
            // Entrées d'autorisation : simple, objet, conditionnelle.
            let autorise = matches!(champs[0], "A" | "OA" | "XA" | "ZA");
            (champs.len() >= 6 && autorise && large(champs[5])).then(|| champs[5].to_string())
        })
        .collect()
}

#[cfg(windows)]
mod windows {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use std::ptr::null_mut;

    use windows_sys::Win32::Foundation::{ERROR_SUCCESS, LocalFree};
    use windows_sys::Win32::Security::Authorization::{
        ConvertSecurityDescriptorToStringSecurityDescriptorW,
        ConvertStringSecurityDescriptorToSecurityDescriptorW, GetNamedSecurityInfoW,
        SDDL_REVISION_1, SE_FILE_OBJECT, SetNamedSecurityInfoW,
    };
    use windows_sys::Win32::Security::{
        ACL, DACL_SECURITY_INFORMATION, GetSecurityDescriptorDacl,
        PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
    };

    fn large(texte: &str) -> Vec<u16> {
        texte.encode_utf16().chain(Some(0)).collect()
    }

    fn chemin_large(chemin: &Path) -> Vec<u16> {
        chemin.as_os_str().encode_wide().chain(Some(0)).collect()
    }

    pub fn appliquer(chemin: &Path, sddl: &str) -> Result<(), String> {
        let sddl = large(sddl);
        let chemin = chemin_large(chemin);
        let mut descripteur: PSECURITY_DESCRIPTOR = null_mut();
        // SAFETY : chaîne terminée par un zéro ; descripteur alloué par Windows,
        // libéré par LocalFree.
        unsafe {
            if ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut descripteur,
                null_mut(),
            ) == 0
            {
                return Err("droits illisibles".into());
            }
            let mut present = 0;
            let mut defaut = 0;
            let mut dacl: *mut ACL = null_mut();
            let ok = GetSecurityDescriptorDacl(descripteur, &mut present, &mut dacl, &mut defaut);
            let resultat = if ok == 0 || present == 0 {
                Err("liste de contrôle absente".to_string())
            } else {
                let code = SetNamedSecurityInfoW(
                    chemin.as_ptr(),
                    SE_FILE_OBJECT,
                    DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                    null_mut(),
                    null_mut(),
                    dacl,
                    null_mut(),
                );
                if code == ERROR_SUCCESS {
                    Ok(())
                } else {
                    Err(format!("droits non appliqués (erreur Windows {code})"))
                }
            };
            LocalFree(descripteur.cast());
            resultat
        }
    }

    pub fn lire(chemin: &Path) -> Result<String, String> {
        let chemin = chemin_large(chemin);
        let mut descripteur: PSECURITY_DESCRIPTOR = null_mut();
        // SAFETY : comme pour appliquer ; la chaîne rendue est libérée par LocalFree.
        unsafe {
            let code = GetNamedSecurityInfoW(
                chemin.as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                &mut descripteur,
            );
            if code != ERROR_SUCCESS {
                return Err(format!("droits illisibles (erreur Windows {code})"));
            }
            let mut texte = null_mut();
            let mut longueur = 0u32;
            let ok = ConvertSecurityDescriptorToStringSecurityDescriptorW(
                descripteur,
                SDDL_REVISION_1,
                DACL_SECURITY_INFORMATION,
                &mut texte,
                &mut longueur,
            );
            LocalFree(descripteur.cast());
            if ok == 0 {
                return Err("droits illisibles".into());
            }
            let mut n = 0;
            while *texte.add(n) != 0 {
                n += 1;
            }
            let sddl = String::from_utf16_lossy(std::slice::from_raw_parts(texte, n));
            LocalFree(texte.cast());
            Ok(sddl)
        }
    }
}

/// Réserve le dossier à SYSTEM et aux Administrateurs.
#[cfg(windows)]
pub fn restreindre(dossier: &Path) -> Result<(), String> {
    windows::appliquer(dossier, SDDL)
}

/// Comptes en trop sur le dossier ; vide si les droits sont ceux voulus.
#[cfg(windows)]
pub fn verifier(dossier: &Path) -> Result<Vec<String>, String> {
    Ok(comptes_en_trop(&windows::lire(dossier)?))
}

#[cfg(not(windows))]
pub fn restreindre(_dossier: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(not(windows))]
pub fn verifier(_dossier: &Path) -> Result<Vec<String>, String> {
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyse_des_droits() {
        assert!(comptes_en_trop("D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)").is_empty());
        assert_eq!(
            comptes_en_trop("D:AI(A;OICIID;FA;;;SY)(A;OICIID;0x1200a9;;;BU)(D;;FA;;;WD)"),
            vec!["BU".to_string()]
        );
        assert_eq!(
            comptes_en_trop("D:NO_ACCESS_CONTROL"),
            vec!["WD".to_string()]
        );
    }

    #[cfg(windows)]
    #[test]
    fn dossier_restreint_puis_verifie() {
        let dossier =
            std::env::temp_dir().join(format!("kadenz-serveur-droits-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(&dossier).unwrap();
        // Droits hérités du dossier temporaire du compte : trop larges ou non,
        // ils ne sont pas ceux voulus tant que le dossier n'est pas restreint.
        let avant = windows::lire(&dossier).unwrap();
        assert!(!avant.starts_with("D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)"));
        restreindre(&dossier).unwrap();
        assert!(verifier(&dossier).unwrap().is_empty());
        // Le propriétaire garde le droit de changer les droits : on rend le
        // dossier au compte pour le supprimer.
        windows::appliquer(
            &dossier,
            "D:(A;OICI;FA;;;OW)(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)",
        )
        .unwrap();
        assert_eq!(verifier(&dossier).unwrap(), vec!["OW".to_string()]);
        std::fs::remove_dir_all(&dossier).unwrap();
    }
}
