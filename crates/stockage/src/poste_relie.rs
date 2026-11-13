//! Serveur auquel ce poste est relié (phase 2, jalon F4 ; écrans, section 2) :
//! adresse, empreinte du certificat du serveur (épinglage), entreprise, nom du
//! poste et identité TLS du poste. La clé privée du poste est protégée par
//! Windows pour le compte de la personne, comme `kadenz.cle`.
//!
//! Jalon F5 : un poste relié travaille sur une copie du serveur,
//! `kadenz-poste.sqlite`, à côté de sa base locale (cahier de la phase 2,
//! décisions 45 à 47). La base locale reste intacte et sert de nouveau si le
//! poste est délié.

use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension};
use serde_json::{Value, json};

use crate::chiffrement::{self, Portee};
use crate::{Erreur, Resultat};

const CLE: &str = "poste_relie";

/// Nom de la copie du serveur, dans le dossier de la base locale.
pub const NOM_COPIE: &str = "kadenz-poste.sqlite";

#[derive(Clone, PartialEq, Eq)]
pub struct PosteRelie {
    /// Adresse `machine:port` du serveur.
    pub adresse: String,
    /// Empreinte SHA-256 du certificat du serveur, en hexadécimal.
    pub empreinte_serveur: String,
    pub entreprise: String,
    /// Nom donné au poste par l'Admin à la création du code.
    pub nom: String,
    /// Certificat du poste (DER).
    pub certificat: Vec<u8>,
    /// Clé privée du poste (PKCS 8), en clair en mémoire seulement.
    pub cle: Vec<u8>,
}

impl std::fmt::Debug for PosteRelie {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // La clé privée n'apparaît jamais dans une trace.
        f.debug_struct("PosteRelie")
            .field("adresse", &self.adresse)
            .field("empreinte_serveur", &self.empreinte_serveur)
            .field("entreprise", &self.entreprise)
            .field("nom", &self.nom)
            .finish_non_exhaustive()
    }
}

fn hex(octets: &[u8]) -> String {
    octets.iter().map(|o| format!("{o:02x}")).collect()
}

fn octets(texte: &str) -> Option<Vec<u8>> {
    if !texte.len().is_multiple_of(2) {
        return None;
    }
    (0..texte.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(texte.get(i..i + 2)?, 16).ok())
        .collect()
}

/// Garde le serveur de ce poste (après l'inscription, ou un changement
/// d'adresse).
pub fn garder(conn: &Connection, poste: &PosteRelie) -> Resultat<()> {
    let protegee = chiffrement::proteger(&poste.cle, Portee::Compte)
        .ok_or_else(|| Erreur::Validation("clé du poste non protégée".into()))?;
    let valeur = json!({
        "adresse": poste.adresse,
        "empreinte_serveur": poste.empreinte_serveur,
        "entreprise": poste.entreprise,
        "nom": poste.nom,
        "certificat": hex(&poste.certificat),
        "cle": hex(&protegee),
    });
    conn.execute(
        "INSERT INTO parametre_systeme (cle, valeur) VALUES (?1, ?2)
         ON CONFLICT (cle) DO UPDATE SET valeur = excluded.valeur",
        rusqlite::params![CLE, valeur.to_string()],
    )?;
    Ok(())
}

/// Serveur de ce poste ; `None` si le poste n'est relié à aucun serveur.
/// Une clé protégée pour un autre compte Windows est une erreur.
pub fn lire(conn: &Connection) -> Resultat<Option<PosteRelie>> {
    let valeur: Option<String> = rusqlite::OptionalExtension::optional(conn.query_row(
        "SELECT valeur FROM parametre_systeme WHERE cle = ?1",
        [CLE],
        |r| r.get(0),
    ))?;
    let Some(valeur) = valeur else {
        return Ok(None);
    };
    let illisible = || Erreur::Validation("serveur du poste illisible".into());
    let v: Value = serde_json::from_str(&valeur).map_err(|_| illisible())?;
    let texte = |champ: &str| v[champ].as_str().map(str::to_owned).ok_or_else(illisible);
    let protegee = octets(&texte("cle")?).ok_or_else(illisible)?;
    let cle = chiffrement::deproteger(&protegee).ok_or(Erreur::CleAutreCompte)?;
    Ok(Some(PosteRelie {
        adresse: texte("adresse")?,
        empreinte_serveur: texte("empreinte_serveur")?,
        entreprise: texte("entreprise")?,
        nom: texte("nom")?,
        certificat: octets(&texte("certificat")?).ok_or_else(illisible)?,
        cle,
    }))
}

/// Chemin de la copie du serveur dans le dossier donné. En développement, la
/// variable d'environnement `KADENZ_COPIE` le remplace (comme `KADENZ_BASE`
/// pour la base locale).
pub fn chemin_copie(dossier: &Path) -> PathBuf {
    #[cfg(debug_assertions)]
    if let Some(chemin) = std::env::var_os("KADENZ_COPIE") {
        return chemin.into();
    }
    dossier.join(NOM_COPIE)
}

/// Copie du serveur présente dans le dossier donné, s'il y en a une.
pub fn copie(dossier: &Path) -> Option<PathBuf> {
    let chemin = chemin_copie(dossier);
    chemin.is_file().then_some(chemin)
}

/// Ouvre la copie du serveur : la fenêtre et la synchronisation l'ouvrent en
/// même temps, d'où l'attente de 5 secondes sur un verrou. Les modules actifs
/// sont ceux de l'abonnement, appliqués par l'application après l'ouverture.
pub fn ouvrir_copie(chemin: &Path) -> Resultat<Connection> {
    let conn = crate::ouvrir(chemin)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    Ok(conn)
}

/// Identifiant global d'un compte de la copie, par son identifiant de
/// connexion : pour un compte connecté en local sans serveur (décision 51).
pub fn uid_du_compte(conn: &Connection, identifiant: &str) -> Resultat<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT uid FROM utilisateur WHERE identifiant = ?1",
            [identifiant],
            |r| r.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten())
}

/// Note un brouillon dont la validation a été demandée hors connexion (écrans,
/// section 4).
pub fn noter_a_valider(conn: &Connection, uid: &str) -> Resultat<()> {
    conn.execute(
        "INSERT OR IGNORE INTO synchro_a_valider (uid) VALUES (?1)",
        [uid],
    )?;
    Ok(())
}

/// Oublie un brouillon de la liste « à valider au retour ».
pub fn oublier_a_valider(conn: &Connection, uid: &str) -> Resultat<()> {
    conn.execute("DELETE FROM synchro_a_valider WHERE uid = ?1", [uid])?;
    Ok(())
}

/// Brouillons à valider au retour du serveur, encore présents et encore en
/// brouillon : `uid` et identifiant dans la copie.
pub fn a_valider(conn: &Connection) -> Resultat<Vec<(String, i64)>> {
    Ok(conn
        .prepare(
            "SELECT a.uid, d.id FROM synchro_a_valider a JOIN document d ON d.uid = a.uid
             WHERE d.statut = 'brouillon' ORDER BY d.id",
        )?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?)
}

/// Retire de la liste « à valider au retour » les documents disparus ou qui
/// ne sont plus des brouillons ; vrai si la liste a changé.
pub fn purger_a_valider(conn: &Connection) -> Resultat<bool> {
    let n = conn.execute(
        "DELETE FROM synchro_a_valider WHERE uid NOT IN
         (SELECT uid FROM document WHERE uid IS NOT NULL AND statut = 'brouillon')",
        [],
    )?;
    Ok(n > 0)
}

/// Modules actifs sur la copie d'après le dernier abonnement reçu du serveur :
/// ceux de la licence d'abonnement, ou le socle seul sans licence. À appeler
/// après `ouvrir_copie` et à chaque abonnement reçu.
pub fn appliquer_modules(conn: &Connection) -> Resultat<()> {
    let modules: Vec<kadenz_domaine::module::Module> = crate::abonnement::dernier_recu(conn)?
        .and_then(|(a, _)| a.licence)
        .map(|l| {
            l.modules
                .iter()
                .filter_map(|m| kadenz_domaine::module::Module::depuis_code(m))
                .collect()
        })
        .unwrap_or_default();
    crate::licence::remplir(conn, &modules)
}

/// Crée la copie du serveur, vide, dans le dossier donné : elle porte le lien
/// au serveur et l'identifiant de l'entreprise du serveur ; la première
/// synchronisation la remplit. Une copie déjà présente est refusée. En cas
/// d'échec, rien ne reste sur le disque.
pub fn creer_copie(dossier: &Path, poste: &PosteRelie) -> Resultat<PathBuf> {
    let chemin = chemin_copie(dossier);
    if chemin.exists() {
        return Err(Erreur::Validation(
            "la copie du serveur existe déjà sur ce poste".into(),
        ));
    }
    let creer = || -> Resultat<()> {
        let conn = ouvrir_copie(&chemin)?;
        garder(&conn, poste)?;
        conn.execute(
            "UPDATE parametre_systeme SET valeur = json_quote(?1)
             WHERE cle = 'identifiant_entreprise'",
            [&poste.entreprise],
        )?;
        Ok(())
    };
    if let Err(e) = creer() {
        let mut fichiers = vec![chemin.clone(), crate::chiffrement::chemin_cle(&chemin)];
        for suffixe in ["-wal", "-shm"] {
            let mut nom = chemin.clone().into_os_string();
            nom.push(suffixe);
            fichiers.push(nom.into());
        }
        for f in fichiers {
            let _ = std::fs::remove_file(f);
        }
        return Err(e);
    }
    Ok(chemin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copie_creee_une_seule_fois() {
        let dossier =
            std::env::temp_dir().join(format!("kadenz-essai-copie-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(&dossier).unwrap();
        assert!(copie(&dossier).is_none());
        let poste = PosteRelie {
            adresse: "serveur:47380".into(),
            empreinte_serveur: "ab".repeat(32),
            entreprise: "0190e000-0000-7000-8000-000000000001".into(),
            nom: "Comptabilité".into(),
            certificat: vec![1, 2, 3],
            cle: vec![9, 8, 7],
        };
        let chemin = creer_copie(&dossier, &poste).unwrap();
        assert_eq!(copie(&dossier), Some(chemin.clone()));
        assert!(creer_copie(&dossier, &poste).is_err());
        let conn = ouvrir_copie(&chemin).unwrap();
        assert_eq!(lire(&conn).unwrap().unwrap(), poste);
        assert_eq!(
            crate::synchro::identifiant_entreprise(&conn).unwrap(),
            poste.entreprise
        );
        drop(conn);
        let _ = std::fs::remove_dir_all(&dossier);
    }

    #[test]
    fn garde_et_relit() {
        let conn = crate::ouvrir_en_memoire().unwrap();
        assert!(lire(&conn).unwrap().is_none());
        let poste = PosteRelie {
            adresse: "serveur:47380".into(),
            empreinte_serveur: "ab".repeat(32),
            entreprise: "e".into(),
            nom: "Comptabilité".into(),
            certificat: vec![1, 2, 3],
            cle: vec![9, 8, 7],
        };
        garder(&conn, &poste).unwrap();
        assert_eq!(lire(&conn).unwrap().unwrap(), poste);
        // La clé n'est pas en clair dans la base.
        let brut: String = conn
            .query_row(
                "SELECT valeur FROM parametre_systeme WHERE cle = ?1",
                [CLE],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!brut.contains("\"cle\":\"090807\""));
        assert!(!format!("{poste:?}").contains("cle"));
    }
}
