//! Une seule instance de Kadenz par base : un second lancement sur la même base ne
//! s'ouvre pas, il ramène au premier plan la fenêtre déjà ouverte puis s'arrête. Un
//! même compte ne peut donc pas être connecté deux fois sur le poste.
//!
//! Le verrou est un mutex Windows nommé d'après le chemin de la base, tenu jusqu'à la
//! fin du processus (le système le libère à la sortie, même après un arrêt brutal).

use std::path::Path;

/// Nom du verrou : empreinte stable (FNV-1a 64 bits) du chemin absolu de la base, en
/// minuscules (Windows ne distingue pas la casse des chemins).
fn nom_verrou(base: &Path) -> String {
    let absolu = std::path::absolute(base).unwrap_or_else(|_| base.to_path_buf());
    let texte = absolu.to_string_lossy().to_lowercase();
    let mut empreinte: u64 = 0xcbf2_9ce4_8422_2325;
    for octet in texte.bytes() {
        empreinte ^= u64::from(octet);
        empreinte = empreinte.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("Local\\Kadenz-{empreinte:016x}")
}

/// Prend le verrou de la base. Faux si une autre instance l'a déjà : la fenêtre de
/// cette instance est alors ramenée au premier plan.
#[cfg(windows)]
pub(crate) fn prendre(base: &Path) -> bool {
    use windows_sys::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
    use windows_sys::Win32::System::Threading::CreateMutexW;

    let nom: Vec<u16> = nom_verrou(base).encode_utf16().chain([0]).collect();
    // SAFETY : nom est une chaîne UTF-16 terminée par zéro, valide pendant l'appel.
    let verrou = unsafe { CreateMutexW(std::ptr::null(), 0, nom.as_ptr()) };
    // SAFETY : lu juste après CreateMutexW, sur le même fil.
    let deja_pris = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
    if verrou.is_null() {
        // Verrou impossible à créer : Kadenz s'ouvre quand même plutôt que de bloquer.
        return true;
    }
    if deja_pris {
        activer_existante();
        return false;
    }
    // Le handle n'est jamais refermé : le verrou dure autant que le processus.
    true
}

#[cfg(not(windows))]
pub(crate) fn prendre(_base: &Path) -> bool {
    true
}

/// Ramène au premier plan la fenêtre principale d'un autre processus Kadenz (même
/// exécutable), restaurée si elle était réduite.
#[cfg(windows)]
fn activer_existante() {
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GW_OWNER, GetWindow, GetWindowTextLengthW, GetWindowThreadProcessId, IsIconic,
        IsWindowVisible, SW_RESTORE, SetForegroundWindow, ShowWindow,
    };

    struct Recherche {
        exe: Option<String>,
        trouvee: HWND,
    }

    unsafe extern "system" fn examiner(fenetre: HWND, donnees: LPARAM) -> i32 {
        // SAFETY : donnees pointe sur la Recherche de activer_existante, vivante pendant EnumWindows.
        let recherche = unsafe { &mut *(donnees as *mut Recherche) };
        // SAFETY : fenetre est un handle fourni par EnumWindows.
        let principale = unsafe {
            IsWindowVisible(fenetre) != 0
                && GetWindow(fenetre, GW_OWNER).is_null()
                && GetWindowTextLengthW(fenetre) > 0
        };
        if !principale {
            return 1;
        }
        let mut pid = 0u32;
        // SAFETY : pid est une variable locale valide.
        unsafe { GetWindowThreadProcessId(fenetre, &mut pid) };
        if pid == std::process::id() || exe_du_processus(pid) != recherche.exe {
            return 1;
        }
        recherche.trouvee = fenetre;
        0
    }

    let mut recherche = Recherche {
        exe: exe_du_processus(std::process::id()),
        trouvee: std::ptr::null_mut(),
    };
    if recherche.exe.is_none() {
        return;
    }
    // SAFETY : le pointeur sur recherche reste valide pendant tout l'appel.
    unsafe { EnumWindows(Some(examiner), &mut recherche as *mut Recherche as LPARAM) };
    if recherche.trouvee.is_null() {
        return;
    }
    // SAFETY : handle trouvé par EnumWindows juste avant.
    unsafe {
        if IsIconic(recherche.trouvee) != 0 {
            ShowWindow(recherche.trouvee, SW_RESTORE);
        }
        SetForegroundWindow(recherche.trouvee);
    }
}

/// Chemin de l'exécutable d'un processus, en minuscules ; None s'il est inaccessible.
#[cfg(windows)]
fn exe_du_processus(pid: u32) -> Option<String> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    };

    // SAFETY : simple ouverture en lecture limitée, refermée ci-dessous.
    let processus = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if processus.is_null() {
        return None;
    }
    let mut tampon = [0u16; 1024];
    let mut longueur = tampon.len() as u32;
    // SAFETY : tampon et longueur décrivent une zone valide de 1024 caractères.
    let ok = unsafe {
        QueryFullProcessImageNameW(
            processus,
            PROCESS_NAME_WIN32,
            tampon.as_mut_ptr(),
            &mut longueur,
        )
    };
    // SAFETY : handle ouvert ci-dessus.
    unsafe { CloseHandle(processus) };
    (ok != 0).then(|| String::from_utf16_lossy(&tampon[..longueur as usize]).to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verrou_par_base_insensible_a_la_casse() {
        let a = nom_verrou(Path::new("C:\\Donnees\\Kadenz\\kadenz.sqlite"));
        let b = nom_verrou(Path::new("c:\\donnees\\kadenz\\KADENZ.sqlite"));
        let c = nom_verrou(Path::new("C:\\Autre\\kadenz.sqlite"));
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a.starts_with("Local\\Kadenz-"));
    }
}
