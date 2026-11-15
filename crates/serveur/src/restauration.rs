//! Restauration de la base du serveur par la fenêtre d'administration (phase
//! 2, jalon F7b ; décision 35 ; spec. synchronisation, section 10).
//!
//! Déroulé de la fenêtre : [`preparer_restauration`] (service démarré ou non),
//! confirmation, [`arreter_service`], [`restaurer`], [`demarrer_service`].
//! Chaque fonction ne prend que des chemins et des valeurs : elles s'appellent
//! depuis un fil à part.
//!
//! - La sauvegarde choisie est d'abord copiée dans un fichier de travail du
//!   dossier des données : la sauvegarde faite à l'arrêt du service fait
//!   tourner les sauvegardes d'arrêt, et un mot de passe faux n'arrête rien.
//! - La base courante est sauvegardée (genre « avant restauration »), puis
//!   mise de côté, jamais supprimée ; l'identité TLS portée par la sauvegarde
//!   remplace celle du dossier dans le même échange.
//! - De la base courante de la même entreprise, la base restaurée garde la
//!   licence et la date retenue les plus récentes, les postes inscrits (un
//!   poste retiré depuis la sauvegarde reste retiré, un poste inscrit depuis
//!   reste inscrit), les codes d'inscription et le port.
//! - La base restaurée reçoit une nouvelle génération : chaque poste qui la
//!   voit refait sa copie.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use kadenz_stockage::chiffrement::{self, Portee};
use kadenz_stockage::sauvegarde::{self, Genre, Restauration as Variantes};
use kadenz_stockage::{Erreur, abonnement, distance, synchro};
use rusqlite::{Connection, OptionalExtension};

use crate::admin::{self, EtatService};
use crate::sauvegardes::{self, TABLE_IDENTITE};
use crate::{NOM_BASE, NOM_CERTIFICAT, NOM_CLE_TLS};

/// Fichier de travail : copie de la sauvegarde choisie.
const TRAVAIL: &str = "restauration-en-cours.kzs";

/// Sauvegarde choisie, vérifiée et copiée, prête à restaurer. Son fichier de
/// travail est effacé quand elle est abandonnée : elle doit vivre jusqu'à la
/// fin de [`restaurer`].
#[derive(Debug)]
pub struct Restauration {
    travail: PathBuf,
    mot_de_passe: String,
    /// Nom de l'entreprise de la sauvegarde.
    pub entreprise: String,
    /// Nom de l'entreprise de la base courante du serveur, s'il y en a une.
    pub entreprise_serveur: Option<String>,
    /// La sauvegarde est celle de l'entreprise du serveur (même identifiant).
    /// Sinon, la fenêtre avertit : l'entreprise du serveur sera remplacée.
    pub meme_entreprise: bool,
    /// La sauvegarde porte le certificat et la clé TLS du serveur.
    pub identite: bool,
}

impl Drop for Restauration {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.travail);
    }
}

/// Ce que la restauration a fait.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BilanRestauration {
    /// Sauvegarde de la base courante faite juste avant.
    pub avant: Option<PathBuf>,
    /// Base courante mise de côté.
    pub mise_de_cote: Option<PathBuf>,
    /// Le certificat du dossier a été remplacé par celui de la sauvegarde :
    /// les postes inscrits du temps de la sauvegarde (cas d'une nouvelle
    /// machine) se reconnectent sans rien faire ; seul un poste inscrit
    /// depuis sur ce serveur doit être réinscrit. L'empreinte et le code
    /// complet sont à relire.
    pub identite_remplacee: bool,
    /// L'entreprise du serveur a été remplacée par celle de la sauvegarde.
    pub entreprise_changee: bool,
}

/// Copie la sauvegarde dans le fichier de travail et la vérifie (mot de passe,
/// schéma, intégrité). Le service peut tourner.
pub fn preparer_restauration(
    donnees: &Path,
    sauvegarde: &Path,
    mot_de_passe: &str,
) -> Result<Restauration, String> {
    if !sauvegarde.is_file() {
        return Err(format!("sauvegarde introuvable : {}", sauvegarde.display()));
    }
    let travail = donnees.join(TRAVAIL);
    let _ = fs::remove_file(&travail);
    fs::copy(sauvegarde, &travail)
        .map_err(|e| format!("copie de la sauvegarde impossible : {e}"))?;
    // Effacé par Drop sur toute erreur qui suit.
    let mut preparee = Restauration {
        travail,
        mot_de_passe: mot_de_passe.to_string(),
        entreprise: String::new(),
        entreprise_serveur: None,
        meme_entreprise: false,
        identite: false,
    };
    let lue = lire_sauvegarde(&preparee.travail, mot_de_passe)?;
    preparee.entreprise = lue.nom;
    preparee.identite = lue.identite.is_some();
    let base = donnees.join(NOM_BASE);
    if base.is_file() {
        let conn = admin::ouvrir_base(donnees)?;
        preparee.entreprise_serveur = Some(nom_entreprise(&conn));
        preparee.meme_entreprise = synchro::identifiant_entreprise(&conn).ok().as_deref()
            == Some(lue.identifiant.as_str());
    }
    Ok(preparee)
}

struct Lue {
    identifiant: String,
    nom: String,
    identite: Option<(Vec<u8>, Vec<u8>)>,
}

fn lire_sauvegarde(chemin: &Path, mot_de_passe: &str) -> Result<Lue, String> {
    let conn = kadenz_stockage::ouvrir_sauvegarde(chemin, mot_de_passe).map_err(texte)?;
    let version: u32 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let application = kadenz_stockage::version_schema_application();
    if version > application {
        return Err(texte(Erreur::VersionSchemaInconnue {
            base: version,
            application,
        }));
    }
    let integrite: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if integrite != "ok" {
        return Err(format!("sauvegarde endommagée : {integrite}"));
    }
    Ok(Lue {
        identifiant: synchro::identifiant_entreprise(&conn).unwrap_or_default(),
        nom: nom_entreprise(&conn),
        identite: identite_de(&conn),
    })
}

fn nom_entreprise(conn: &Connection) -> String {
    conn.query_row("SELECT nom FROM entreprise WHERE id = 1", [], |r| r.get(0))
        .optional()
        .ok()
        .flatten()
        .unwrap_or_default()
}

/// Certificat et clé TLS en clair portés par une sauvegarde du serveur.
fn identite_de(conn: &Connection) -> Option<(Vec<u8>, Vec<u8>)> {
    conn.query_row(
        &format!("SELECT certificat, cle FROM {TABLE_IDENTITE} WHERE id = 1"),
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .ok()
}

fn texte(e: Erreur) -> String {
    e.to_string()
}

/// Restaure la sauvegarde préparée, service arrêté. Toutes les connexions de
/// la fenêtre à la base doivent être fermées (Windows refuse de renommer un
/// fichier ouvert). En cas d'erreur, la base courante et l'identité TLS sont
/// en place, inchangées, et le service peut redémarrer dessus ; sauf si le
/// message commence par [`sauvegarde::RESTAURATION_INTERROMPUE`] : il nomme
/// alors les fichiers à remettre en place, et le service ne doit pas
/// redémarrer.
pub fn restaurer(donnees: &Path, preparee: &Restauration) -> Result<BilanRestauration, String> {
    restaurer_si(donnees, preparee, admin::etat_service()?)
}

/// [`restaurer`] avec l'état du service donné (essais).
#[doc(hidden)]
pub fn restaurer_si(
    donnees: &Path,
    preparee: &Restauration,
    service: EtatService,
) -> Result<BilanRestauration, String> {
    if !matches!(service, EtatService::Arrete | EtatService::NonInstalle) {
        return Err("arrêtez le service du serveur avant la restauration".into());
    }
    if !preparee.travail.is_file() {
        return Err("la sauvegarde préparée a disparu : choisissez-la de nouveau".into());
    }
    let lue = lire_sauvegarde(&preparee.travail, &preparee.mot_de_passe)?;
    let mut fichiers = Vec::new();
    let mut identite_remplacee = false;
    if let Some((certificat, cle)) = &lue.identite {
        identite_remplacee =
            fs::read(donnees.join(NOM_CERTIFICAT)).ok().as_ref() != Some(certificat);
        let protegee = chiffrement::proteger(cle, Portee::Machine)
            .ok_or("protection de la clé TLS par Windows impossible")?;
        fichiers.push((donnees.join(NOM_CERTIFICAT), certificat.clone()));
        fichiers.push((donnees.join(NOM_CLE_TLS), protegee));
    }

    // La sauvegarde « avant restauration » prend le mot de passe réglé, sinon
    // celui de la sauvegarde restaurée.
    let reglage = sauvegardes::lire_reglage(donnees);
    let mot_de_passe_avant = reglage
        .mot_de_passe
        .clone()
        .unwrap_or_else(|| preparee.mot_de_passe.clone());
    let avant = |courante: &Connection| {
        sauvegardes::sauvegarder_avec(
            courante,
            donnees,
            &reglage.dossier,
            &mot_de_passe_avant,
            Genre::AvantRestauration,
        )
        .map(Some)
        .map_err(Erreur::Validation)
    };
    let identifiant = lue.identifiant.clone();
    let entreprise_changee = std::cell::Cell::new(false);
    let ajuster = |courante: Option<&Connection>,
                   cible: &Connection|
     -> kadenz_stockage::Resultat<()> {
        cible.execute_batch(&format!("DROP TABLE IF EXISTS {TABLE_IDENTITE}"))?;
        synchro::nouvelle_generation(cible)?;
        let Some(courante) = courante else {
            return Ok(());
        };
        if synchro::identifiant_entreprise(courante).ok().as_deref() != Some(identifiant.as_str()) {
            entreprise_changee.set(true);
            return Ok(());
        }
        abonnement::reporter(courante, cible)?;
        reporter_postes(courante, cible)?;
        Ok(())
    };
    let faite = sauvegarde::restaurer_avec(
        &donnees.join(NOM_BASE),
        &preparee.travail,
        &preparee.mot_de_passe,
        &Variantes {
            portee: Portee::Machine,
            avant: &avant,
            ajuster: &ajuster,
            fichiers: &fichiers,
        },
    )
    .map_err(texte)?;
    Ok(BilanRestauration {
        avant: faite.avant,
        mise_de_cote: faite.mise_de_cote,
        identite_remplacee,
        entreprise_changee: entreprise_changee.get(),
    })
}

/// Postes inscrits (avec leur case « accès à distance »), codes
/// d'inscription, port et réglage de l'accès à distance : l'état courant
/// prime.
fn reporter_postes(courante: &Connection, cible: &Connection) -> kadenz_stockage::Resultat<()> {
    type Poste = (String, String, String, Option<String>, bool);
    let postes: Vec<Poste> = courante
        .prepare("SELECT empreinte, nom, inscrit_le, retire_le, distance FROM poste_inscrit")?
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?
        .collect::<Result<_, _>>()?;
    for (empreinte, nom, inscrit_le, retire_le, distance) in postes {
        cible.execute(
            "INSERT INTO poste_inscrit (empreinte, nom, inscrit_le, retire_le, distance)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (empreinte) DO UPDATE SET nom = excluded.nom,
                 inscrit_le = excluded.inscrit_le, retire_le = excluded.retire_le,
                 distance = excluded.distance",
            rusqlite::params![empreinte, nom, inscrit_le, retire_le, distance],
        )?;
    }
    distance::activer(cible, distance::actif(courante)?, None)?;
    distance::regler_relais(cible, distance::relais(courante)?.as_ref())?;
    type Code = (
        i64,
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
    );
    let codes: Vec<Code> = courante
        .prepare(
            "SELECT id, secret_hache, nom_poste, cree_le, expire_le, servi_le, annule_le, poste
             FROM code_inscription",
        )?
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
            ))
        })?
        .collect::<Result<_, _>>()?;
    cible.execute("DELETE FROM code_inscription", [])?;
    for c in codes {
        cible.execute(
            "INSERT INTO code_inscription
                 (id, secret_hache, nom_poste, cree_le, expire_le, servi_le, annule_le, poste)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![c.0, c.1, c.2, c.3, c.4, c.5, c.6, c.7],
        )?;
    }
    let port = admin::port(courante);
    admin::noter_port(cible, port).map_err(Erreur::Validation)?;
    Ok(())
}

/// Arrête le service du serveur et attend son arrêt, au plus `delai` (prévoir
/// large : l'arrêt fait une sauvegarde). Déjà arrêté ou non installé : rien.
/// Un démarrage en cours est attendu d'abord : le service n'accepte l'arrêt
/// qu'une fois démarré.
#[cfg(windows)]
pub fn arreter_service(delai: Duration) -> Result<(), String> {
    use windows_service::service::{ServiceAccess, ServiceState};
    if admin::etat_service()? == EtatService::NonInstalle {
        return Ok(());
    }
    let fin = std::time::Instant::now() + delai;
    let service = ouvrir_service(ServiceAccess::STOP | ServiceAccess::QUERY_STATUS)?;
    match attendre_hors(&service, ServiceState::StartPending, fin, delai, "arrêt")? {
        ServiceState::Stopped => return Ok(()),
        ServiceState::StopPending => {}
        _ => service
            .stop()
            .map(|_| ())
            .map_err(|e| format!("arrêt du service impossible : {e}"))?,
    }
    match attendre_hors(&service, ServiceState::StopPending, fin, delai, "arrêt")? {
        ServiceState::Stopped => Ok(()),
        _ => Err("arrêt du service impossible : il tourne encore".into()),
    }
}

/// Démarre le service du serveur et attend qu'il tourne, au plus `delai`.
/// Un arrêt en cours (par exemple un arrêt qui a dépassé son délai) est
/// attendu d'abord. Un échec du démarrage se lit ensuite dans
/// [`admin::derniere_erreur`].
#[cfg(windows)]
pub fn demarrer_service(delai: Duration) -> Result<(), String> {
    use windows_service::service::{ServiceAccess, ServiceState};
    if admin::etat_service()? == EtatService::NonInstalle {
        return Err("le service du serveur n'est pas installé".into());
    }
    let fin = std::time::Instant::now() + delai;
    let service = ouvrir_service(ServiceAccess::START | ServiceAccess::QUERY_STATUS)?;
    match attendre_hors(&service, ServiceState::StopPending, fin, delai, "démarrage")? {
        ServiceState::Running => return Ok(()),
        ServiceState::Stopped => service
            .start::<&str>(&[])
            .map_err(|e| format!("démarrage du service impossible : {e}"))?,
        _ => {}
    }
    match attendre_hors(
        &service,
        ServiceState::StartPending,
        fin,
        delai,
        "démarrage",
    )? {
        ServiceState::Running => Ok(()),
        ServiceState::Stopped => Err("démarrage du service impossible : il s'est arrêté".into()),
        _ => Err("démarrage du service impossible : état inattendu".into()),
    }
}

#[cfg(windows)]
fn ouvrir_service(
    acces: windows_service::service::ServiceAccess,
) -> Result<windows_service::service::Service, String> {
    use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};
    let gestionnaire = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .map_err(|e| format!("gestionnaire de services inaccessible : {e}"))?;
    gestionnaire
        .open_service(crate::NOM_SERVICE, acces)
        .map_err(|e| format!("service inaccessible : {e}"))
}

/// Attend que le service quitte l'état transitoire donné, jusqu'à `fin` ;
/// rend l'état atteint.
#[cfg(windows)]
fn attendre_hors(
    service: &windows_service::service::Service,
    transitoire: windows_service::service::ServiceState,
    fin: std::time::Instant,
    delai: Duration,
    quoi: &str,
) -> Result<windows_service::service::ServiceState, String> {
    loop {
        let etat = service
            .query_status()
            .map_err(|e| format!("état du service illisible : {e}"))?
            .current_state;
        if etat != transitoire {
            return Ok(etat);
        }
        if std::time::Instant::now() >= fin {
            return Err(format!(
                "{quoi} du service non terminé après {} s",
                delai.as_secs()
            ));
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

#[cfg(not(windows))]
pub fn arreter_service(_delai: Duration) -> Result<(), String> {
    Ok(())
}

#[cfg(not(windows))]
pub fn demarrer_service(_delai: Duration) -> Result<(), String> {
    Err("le service n'existe que sous Windows".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Config, Serveur};
    use chrono::NaiveTime;
    use kadenz_domaine::referentiel::{FicheTiers, TypeTiers};
    use kadenz_domaine::role::Role;
    use kadenz_stockage::comptes::NouvelUtilisateur;
    use kadenz_stockage::entreprise::Entreprise;
    use kadenz_stockage::installation::{self, InstallationInitiale};
    use kadenz_stockage::parametres::Parametres;
    use kadenz_stockage::{postes, tiers};
    use std::net::SocketAddr;

    const MDP: &str = "motdepasse-sauvegardes";

    fn config(dossier: &Path) -> Config {
        let mut config = Config::nouvelle(dossier.to_path_buf());
        config.adresse = SocketAddr::from(([127, 0, 0, 1], 0));
        config.controle_droits = false;
        config.decouverte = false;
        config
    }

    fn nombre_tiers(serveur: &Serveur, nom: &'static str) -> i64 {
        serveur
            .base(move |c| {
                c.query_row("SELECT count(*) FROM tiers WHERE nom = ?1", [nom], |r| {
                    r.get(0)
                })
                .unwrap()
            })
            .unwrap()
    }

    #[test]
    fn restauration_de_la_base_du_serveur() {
        let racine = std::env::temp_dir().join(format!(
            "kadenz-serveur-restauration-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&racine);
        let donnees = racine.join("donnees");
        let dossier_sauvegardes = racine.join("sauvegardes");
        let serveur = Serveur::demarrer(config(&donnees)).unwrap();
        let empreinte = serveur.empreinte().to_string();
        let admin = serveur
            .base(|c| {
                let admin = installation::installer(
                    c,
                    &InstallationInitiale {
                        entreprise: Entreprise {
                            nom: "Entreprise Essai".into(),
                            ..Default::default()
                        },
                        admin: NouvelUtilisateur {
                            nom_complet: "Administrateur".into(),
                            identifiant: "admin".into(),
                            mot_de_passe: "motdepasse1".into(),
                            role: Role::Admin,
                        },
                        parametres: Parametres::default(),
                    },
                )
                .unwrap();
                postes::inscrire(c, "poste-un", "Poste un").unwrap();
                admin
            })
            .unwrap();
        sauvegardes::enregistrer_reglage(
            &donnees,
            &dossier_sauvegardes,
            NaiveTime::from_hms_opt(7, 30, 0).unwrap(),
            Some(MDP),
        )
        .unwrap();
        let d = donnees.clone();
        let sauvegarde = serveur
            .base(move |c| sauvegardes::sauvegarder(c, &d, Genre::Manuelle))
            .unwrap()
            .unwrap();
        let generation = serveur.base(|c| synchro::generation(c).unwrap()).unwrap();

        // Après la sauvegarde : tiers créé, poste inscrit, poste retiré.
        let a = admin.clone();
        serveur
            .base(move |c| {
                let mut f = FicheTiers::vide(TypeTiers::Client);
                f.nom = "Client perdu".into();
                tiers::creer(c, &a, &f).unwrap();
                postes::inscrire(c, "poste-deux", "Poste deux").unwrap();
                postes::retirer(c, "poste-un").unwrap();
            })
            .unwrap();
        assert_eq!(nombre_tiers(&serveur, "Client perdu"), 1);
        serveur.arreter();

        assert!(preparer_restauration(&donnees, &sauvegarde, "pas-le-bon-mot").is_err());
        let preparee = preparer_restauration(&donnees, &sauvegarde, MDP).unwrap();
        assert_eq!(preparee.entreprise, "Entreprise Essai");
        assert_eq!(
            preparee.entreprise_serveur.as_deref(),
            Some("Entreprise Essai")
        );
        assert!(preparee.meme_entreprise);
        assert!(preparee.identite);
        // Service qui tourne : refus, rien ne bouge.
        assert!(restaurer_si(&donnees, &preparee, EtatService::Demarre).is_err());
        let bilan = restaurer_si(&donnees, &preparee, EtatService::NonInstalle).unwrap();
        assert!(!bilan.identite_remplacee);
        assert!(!bilan.entreprise_changee);
        assert!(bilan.avant.as_ref().is_some_and(|p| p.is_file()));
        assert!(bilan.mise_de_cote.as_ref().is_some_and(|p| p.is_file()));
        let travail = preparee.travail.clone();
        drop(preparee);
        assert!(!travail.exists());

        // Le serveur redémarre sur la base restaurée, même certificat.
        let serveur = Serveur::demarrer(config(&donnees)).unwrap();
        assert_eq!(serveur.empreinte(), empreinte);
        assert_eq!(nombre_tiers(&serveur, "Client perdu"), 0);
        let (un, deux, nouvelle, identite) = serveur
            .base(|c| {
                (
                    postes::est_inscrit(c, "poste-un").unwrap(),
                    postes::est_inscrit(c, "poste-deux").unwrap(),
                    synchro::generation(c).unwrap(),
                    c.query_row(
                        "SELECT count(*) FROM sqlite_master WHERE name = ?1",
                        [TABLE_IDENTITE],
                        |r| r.get::<_, i64>(0),
                    )
                    .unwrap(),
                )
            })
            .unwrap();
        assert!(!un, "poste retiré après la sauvegarde : reste retiré");
        assert!(deux, "poste inscrit après la sauvegarde : reste inscrit");
        assert_ne!(nouvelle, generation);
        assert_eq!(identite, 0);
        serveur.arreter();
        let _ = fs::remove_dir_all(&racine);
    }
}
