//! Droits du dossier des données du serveur (spécification de synchronisation,
//! section 8). La clé de la base est protégée par Windows au niveau de la
//! machine : n'importe quel compte qui lit le fichier peut la déchiffrer. La
//! protection repose donc sur les droits du dossier, réservé à SYSTEM et aux
//! Administrateurs, posés à la création et vérifiés à chaque démarrage.
//!
//! Le propriétaire compte aussi : il peut toujours modifier les droits, donc
//! s'en donner. Il doit être SYSTEM ou les Administrateurs.
//!
//! Serveur intégré du monoposte (portée `Compte`, spécification du monoposte,
//! section 2.3) : le dossier, dans le profil, appartient au compte Windows
//! courant et n'est ouvert qu'à lui et à SYSTEM ; ses clés sont protégées au
//! niveau du compte.

use std::path::Path;

use kadenz_stockage::chiffrement::Portee;

/// Droits voulus pour le service, au format SDDL : propriétaire
/// Administrateurs (O:BA) ; accès complet pour SYSTEM (SY) et les
/// Administrateurs (BA), hérité par les fichiers et sous-dossiers, sans
/// héritage du dossier parent (P).
pub const SDDL_SERVICE: &str = "O:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)";

/// Comptes qui peuvent recevoir un droit sur le dossier du service ou le
/// posséder.
const PERMIS_SERVICE: [&str; 2] = ["SY", "BA"];

/// Droits voulus pour le serveur intégré : propriétaire le compte `sid`,
/// accès complet pour lui et SYSTEM, sans héritage du dossier parent.
pub fn sddl_compte(sid: &str) -> String {
    format!("O:{sid}D:P(A;OICI;FA;;;{sid})(A;OICI;FA;;;SY)")
}

/// Écarts d'une description de sécurité au format SDDL par rapport aux droits
/// du service : propriétaire autre que SYSTEM ou les Administrateurs, et
/// comptes autres qu'eux qui ont un droit d'accès. Une liste absente
/// (`NO_ACCESS_CONTROL`) ouvre le dossier à tous (WD).
pub fn ecarts(sddl: &str) -> Vec<String> {
    ecarts_selon(sddl, &PERMIS_SERVICE)
}

/// Écarts par rapport aux droits du serveur intégré du compte `sid`.
pub fn ecarts_compte(sddl: &str, sid: &str) -> Vec<String> {
    ecarts_selon(sddl, &[sid, "SY"])
}

fn ecarts_selon(sddl: &str, permis: &[&str]) -> Vec<String> {
    let large = |sid: &str| !permis.contains(&sid);
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

    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_SUCCESS, HANDLE, LocalFree};
    use windows_sys::Win32::Security::Authorization::{
        ConvertSecurityDescriptorToStringSecurityDescriptorW, ConvertSidToStringSidW,
        ConvertStringSecurityDescriptorToSecurityDescriptorW, GetNamedSecurityInfoW,
        SDDL_REVISION_1, SE_FILE_OBJECT, SetNamedSecurityInfoW,
    };
    use windows_sys::Win32::Security::{
        ACL, DACL_SECURITY_INFORMATION, GetSecurityDescriptorDacl, GetSecurityDescriptorOwner,
        GetTokenInformation, OWNER_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION,
        PSECURITY_DESCRIPTOR, PSID, TOKEN_QUERY, TOKEN_USER, TokenUser,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

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

    /// Identifiant (SID) du compte Windows du processus, au format texte.
    pub fn sid_courant() -> Result<String, String> {
        // SAFETY : jeton du processus ouvert puis fermé ; le tampon, aligné
        // sur 8 octets, reçoit un TOKEN_USER dont le SID pointe dans le
        // tampon, encore vivant lors de la conversion ; la chaîne rendue est
        // libérée par LocalFree.
        unsafe {
            let mut jeton: HANDLE = null_mut();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut jeton) == 0 {
                return Err("jeton du processus illisible".into());
            }
            let mut tampon = vec![0u64; 64];
            let mut taille = 0u32;
            let ok = GetTokenInformation(
                jeton,
                TokenUser,
                tampon.as_mut_ptr().cast(),
                (tampon.len() * 8) as u32,
                &mut taille,
            );
            CloseHandle(jeton);
            if ok == 0 {
                return Err("compte du processus illisible".into());
            }
            let utilisateur = &*(tampon.as_ptr() as *const TOKEN_USER);
            let mut texte = null_mut();
            if ConvertSidToStringSidW(utilisateur.User.Sid, &mut texte) == 0 {
                return Err("identifiant du compte illisible".into());
            }
            let mut n = 0;
            while *texte.add(n) != 0 {
                n += 1;
            }
            let sid = String::from_utf16_lossy(std::slice::from_raw_parts(texte, n));
            LocalFree(texte.cast());
            Ok(sid)
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

/// Pose les droits voulus selon la portée. `Machine` (service) : dossier
/// donné aux Administrateurs et réservé à SYSTEM et aux Administrateurs ;
/// demande un compte administrateur (le service tourne sous SYSTEM).
/// `Compte` (serveur intégré) : dossier au compte courant, ouvert à lui et à
/// SYSTEM ; aucun droit d'administrateur demandé.
#[cfg(windows)]
pub fn restreindre(dossier: &Path, portee: Portee) -> Result<(), String> {
    match portee {
        Portee::Machine => windows::appliquer(dossier, SDDL_SERVICE),
        Portee::Compte => windows::appliquer(dossier, &sddl_compte(&windows::sid_courant()?)),
    }
}

/// Écarts du dossier selon la portée ; vide si ses droits sont ceux voulus.
#[cfg(windows)]
pub fn verifier(dossier: &Path, portee: Portee) -> Result<Vec<String>, String> {
    let sddl = windows::lire(dossier)?;
    Ok(match portee {
        Portee::Machine => ecarts(&sddl),
        Portee::Compte => ecarts_compte(&sddl, &windows::sid_courant()?),
    })
}

#[cfg(not(windows))]
pub fn restreindre(_dossier: &Path, _portee: Portee) -> Result<(), String> {
    Ok(())
}

#[cfg(not(windows))]
pub fn verifier(_dossier: &Path, _portee: Portee) -> Result<Vec<String>, String> {
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

    /// Le multiposte ne change pas : mêmes droits voulus qu'avant le
    /// monoposte, et le compte du serveur intégré n'y est pas permis.
    #[test]
    fn droits_du_service_inchanges() {
        assert_eq!(SDDL_SERVICE, "O:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)");
        let sid = "S-1-5-21-1-2-3-1001";
        assert!(ecarts_compte(&sddl_compte(sid), sid).is_empty());
        assert_eq!(
            ecarts(&sddl_compte(sid)),
            vec![format!("propriétaire {sid}"), sid.to_string()]
        );
        // Droits du service vus par le serveur intégré : propriétaire et
        // Administrateurs en trop.
        assert_eq!(
            ecarts_compte(SDDL_SERVICE, sid),
            vec!["propriétaire BA".to_string(), "BA".to_string()]
        );
    }

    #[cfg(windows)]
    #[test]
    fn dossier_du_compte_restreint_puis_verifie() {
        let dossier = std::env::temp_dir().join(format!(
            "kadenz-serveur-droits-compte-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(&dossier).unwrap();
        assert!(!verifier(&dossier, Portee::Compte).unwrap().is_empty());
        // Aucun droit d'administrateur demandé.
        restreindre(&dossier, Portee::Compte).unwrap();
        assert!(verifier(&dossier, Portee::Compte).unwrap().is_empty());
        // Le compte garde l'accès : le dossier s'écrit et se supprime.
        std::fs::write(dossier.join("essai"), b"x").unwrap();
        std::fs::remove_dir_all(&dossier).unwrap();
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
        assert!(!verifier(&dossier, Portee::Machine).unwrap().is_empty());
        if restreindre(&dossier, Portee::Machine).is_ok() {
            // Essai lancé en administrateur.
            assert!(verifier(&dossier, Portee::Machine).unwrap().is_empty());
        } else {
            // Sans droits d'administrateur, le dossier ne peut être donné aux
            // Administrateurs : la liste seule est posée, et le propriétaire
            // restant est signalé.
            windows::appliquer(&dossier, "D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)").unwrap();
            let restants = verifier(&dossier, Portee::Machine).unwrap();
            assert_eq!(restants.len(), 1, "{restants:?}");
            assert!(restants[0].starts_with("propriétaire "), "{restants:?}");
        }
        // Le dossier est rendu au compte pour le supprimer.
        windows::appliquer(
            &dossier,
            "D:(A;OICI;FA;;;OW)(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)",
        )
        .unwrap();
        assert!(
            verifier(&dossier, Portee::Machine)
                .unwrap()
                .contains(&"OW".to_string())
        );
        std::fs::remove_dir_all(&dossier).unwrap();
    }
}
