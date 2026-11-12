//! Droits du dossier des données du serveur (spécification de synchronisation,
//! section 8). La clé de la base est protégée par Windows au niveau de la
//! machine : n'importe quel compte qui lit le fichier peut la déchiffrer. La
//! protection repose donc sur les droits du dossier, réservé à SYSTEM et aux
//! Administrateurs, posés à la création et vérifiés à chaque démarrage.
//!
//! Le propriétaire compte aussi : il peut toujours modifier les droits, donc
//! s'en donner. Il doit être SYSTEM ou les Administrateurs.

use std::path::Path;

/// Droits voulus, au format SDDL : propriétaire Administrateurs (O:BA) ;
/// accès complet pour SYSTEM (SY) et les Administrateurs (BA), hérité par les
/// fichiers et sous-dossiers, sans héritage du dossier parent (P).
#[cfg(windows)]
const SDDL: &str = "O:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)";

/// Comptes qui peuvent recevoir un droit sur le dossier ou le posséder.
const PERMIS: [&str; 2] = ["SY", "BA"];

fn large(sid: &str) -> bool {
    !PERMIS.contains(&sid)
}

/// Écarts d'une description de sécurité au format SDDL par rapport aux droits
/// voulus : propriétaire autre que SYSTEM ou les Administrateurs, et comptes
/// autres qu'eux qui ont un droit d'accès. Une liste absente
/// (`NO_ACCESS_CONTROL`) ouvre le dossier à tous (WD).
pub fn ecarts(sddl: &str) -> Vec<String> {
    let mut ecarts = Vec::new();
    if let Some(reste) = sddl.strip_prefix("O:") {
        let fin = [reste.find("G:"), reste.find("D:")]
            .into_iter()
            .flatten()
            .min()
            .unwrap_or(reste.len());
        let proprietaire = &reste[..fin];
        if large(proprietaire) {
            ecarts.push(format!("propriétaire {proprietaire}"));
        }
    }
    if sddl.contains("NO_ACCESS_CONTROL") {
        ecarts.push("WD".to_string());
        return ecarts;
    }
    ecarts.extend(sddl.split('(').skip(1).filter_map(|ace| {
        let champs: Vec<&str> = ace.trim_end_matches(')').split(';').collect();
        // Entrées d'autorisation : simple, objet, conditionnelle.
        let autorise = matches!(champs[0], "A" | "OA" | "XA" | "ZA");
        (champs.len() >= 6 && autorise && large(champs[5])).then(|| champs[5].to_string())
    }));
    ecarts
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
        ACL, DACL_SECURITY_INFORMATION, GetSecurityDescriptorDacl, GetSecurityDescriptorOwner,
        OWNER_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
        PSID,
    };

    fn large(texte: &str) -> Vec<u16> {
        texte.encode_utf16().chain(Some(0)).collect()
    }

    fn chemin_large(chemin: &Path) -> Vec<u16> {
        chemin.as_os_str().encode_wide().chain(Some(0)).collect()
    }

    /// Applique la liste de contrôle du SDDL, et son propriétaire s'il en a un.
    pub fn appliquer(chemin: &Path, sddl: &str) -> Result<(), String> {
        let sddl = large(sddl);
        let chemin = chemin_large(chemin);
        let mut descripteur: PSECURITY_DESCRIPTOR = null_mut();
        // SAFETY : chaînes terminées par un zéro ; descripteur alloué par
        // Windows, libéré par LocalFree ; la liste et le propriétaire pointent
        // dans le descripteur, encore vivant lors de l'appel.
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
            let mut proprietaire: PSID = null_mut();
            let ok = GetSecurityDescriptorDacl(descripteur, &mut present, &mut dacl, &mut defaut)
                != 0
                && GetSecurityDescriptorOwner(descripteur, &mut proprietaire, &mut defaut) != 0;
            let resultat = if !ok || present == 0 {
                Err("liste de contrôle absente".to_string())
            } else {
                let mut quoi = DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION;
                if !proprietaire.is_null() {
                    quoi |= OWNER_SECURITY_INFORMATION;
                }
                let code = SetNamedSecurityInfoW(
                    chemin.as_ptr(),
                    SE_FILE_OBJECT,
                    quoi,
                    proprietaire,
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

    /// Propriétaire et liste de contrôle, au format SDDL.
    pub fn lire(chemin: &Path) -> Result<String, String> {
        let chemin = chemin_large(chemin);
        let quoi = OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION;
        let mut descripteur: PSECURITY_DESCRIPTOR = null_mut();
        // SAFETY : comme pour appliquer ; la chaîne rendue est libérée par LocalFree.
        unsafe {
            let code = GetNamedSecurityInfoW(
                chemin.as_ptr(),
                SE_FILE_OBJECT,
                quoi,
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
                quoi,
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

/// Donne le dossier aux Administrateurs et le réserve à SYSTEM et aux
/// Administrateurs. Demande un compte administrateur (le service tourne
/// sous SYSTEM).
#[cfg(windows)]
pub fn restreindre(dossier: &Path) -> Result<(), String> {
    windows::appliquer(dossier, SDDL)
}

/// Écarts du dossier ; vide si ses droits sont ceux voulus.
#[cfg(windows)]
pub fn verifier(dossier: &Path) -> Result<Vec<String>, String> {
    Ok(ecarts(&windows::lire(dossier)?))
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
        assert!(ecarts("O:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)").is_empty());
        assert!(ecarts("O:SYD:PAI(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)").is_empty());
        assert_eq!(
            ecarts("O:BAD:AI(A;OICIID;FA;;;SY)(A;OICIID;0x1200a9;;;BU)(D;;FA;;;WD)"),
            vec!["BU".to_string()]
        );
        assert_eq!(
            ecarts("O:S-1-5-21-1-2-3-1001D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)"),
            vec!["propriétaire S-1-5-21-1-2-3-1001".to_string()]
        );
        assert_eq!(ecarts("O:BAD:NO_ACCESS_CONTROL"), vec!["WD".to_string()]);
    }

    #[cfg(windows)]
    #[test]
    fn dossier_restreint_puis_verifie() {
        let dossier =
            std::env::temp_dir().join(format!("kadenz-serveur-droits-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(&dossier).unwrap();
        // Droits hérités du dossier temporaire du compte, qui en est le
        // propriétaire : jamais ceux voulus.
        assert!(!verifier(&dossier).unwrap().is_empty());
        if restreindre(&dossier).is_ok() {
            // Essai lancé en administrateur.
            assert!(verifier(&dossier).unwrap().is_empty());
        } else {
            // Sans droits d'administrateur, le dossier ne peut être donné aux
            // Administrateurs : la liste seule est posée, et le propriétaire
            // restant est signalé.
            windows::appliquer(&dossier, "D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)").unwrap();
            let restants = verifier(&dossier).unwrap();
            assert_eq!(restants.len(), 1, "{restants:?}");
            assert!(restants[0].starts_with("propriétaire "), "{restants:?}");
        }
        // Le dossier est rendu au compte pour le supprimer.
        windows::appliquer(
            &dossier,
            "D:(A;OICI;FA;;;OW)(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)",
        )
        .unwrap();
        assert!(verifier(&dossier).unwrap().contains(&"OW".to_string()));
        std::fs::remove_dir_all(&dossier).unwrap();
    }
}
