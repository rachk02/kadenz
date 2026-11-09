//! Comptes utilisateurs, connexion et garde du dernier administrateur
//! (cahier des charges, sections 7.5, 10.1 et 10.2).

use argon2::Argon2;
use argon2::password_hash::phc::PasswordHash;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use kadenz_domaine::langue::Langue;
use kadenz_domaine::role::{Droit, Role};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::json;

use crate::audit::{self, Entree};
use crate::droits::exiger;
use crate::{Erreur, Resultat, maintenant};

pub const LONGUEUR_MIN_MOT_DE_PASSE: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Utilisateur {
    pub id: i64,
    pub nom_complet: String,
    pub identifiant: String,
    pub role: Role,
    pub actif: bool,
    /// Langue de l'interface choisie par le compte.
    pub langue: Langue,
}

#[derive(Debug, Clone)]
pub struct NouvelUtilisateur {
    pub nom_complet: String,
    pub identifiant: String,
    pub mot_de_passe: String,
    pub role: Role,
}

/// Écran à afficher au démarrage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtatInstallation {
    /// Aucun administrateur actif : l'assistant de premier lancement s'impose.
    AssistantRequis,
    ConnexionRequise,
}

pub fn etat_installation(conn: &Connection) -> Resultat<EtatInstallation> {
    Ok(if admins_actifs(conn)? == 0 {
        EtatInstallation::AssistantRequis
    } else {
        EtatInstallation::ConnexionRequise
    })
}

pub(crate) fn admins_actifs(conn: &Connection) -> Resultat<i64> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM utilisateur WHERE role = 'admin' AND actif = 1",
        [],
        |r| r.get(0),
    )?)
}

fn hacher(mot_de_passe: &str) -> Resultat<String> {
    Argon2::default()
        .hash_password(mot_de_passe.as_bytes())
        .map(|h| h.to_string())
        .map_err(|e| Erreur::Hachage(e.to_string()))
}

fn mot_de_passe_correct(mot_de_passe: &str, hache: &str) -> bool {
    PasswordHash::new(hache)
        .map(|h| {
            Argon2::default()
                .verify_password(mot_de_passe.as_bytes(), &h)
                .is_ok()
        })
        .unwrap_or(false)
}

fn valider_mot_de_passe(mot_de_passe: &str) -> Resultat<()> {
    if mot_de_passe.chars().count() < LONGUEUR_MIN_MOT_DE_PASSE {
        return Err(Erreur::Validation(format!(
            "le mot de passe doit contenir au moins {LONGUEUR_MIN_MOT_DE_PASSE} caractères"
        )));
    }
    Ok(())
}

fn ligne_vers_utilisateur(r: &rusqlite::Row<'_>) -> rusqlite::Result<Utilisateur> {
    let code_role: String = r.get(3)?;
    let role = Role::depuis_code(&code_role).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            3,
            rusqlite::types::Type::Text,
            format!("rôle inconnu : {code_role}").into(),
        )
    })?;
    Ok(Utilisateur {
        id: r.get(0)?,
        nom_complet: r.get(1)?,
        identifiant: r.get(2)?,
        role,
        actif: r.get(4)?,
        // Code inconnu (impossible avec la contrainte) : français par défaut.
        langue: Langue::depuis_code(&r.get::<_, String>(5)?).unwrap_or_default(),
    })
}

const COLONNES: &str = "id, nom_complet, identifiant, role, actif, langue";

pub fn lire(conn: &Connection, id: i64) -> Resultat<Option<Utilisateur>> {
    Ok(conn
        .query_row(
            &format!("SELECT {COLONNES} FROM utilisateur WHERE id = ?1"),
            [id],
            ligne_vers_utilisateur,
        )
        .optional()?)
}

pub fn lister(conn: &Connection) -> Resultat<Vec<Utilisateur>> {
    let mut requete = conn.prepare(&format!(
        "SELECT {COLONNES} FROM utilisateur ORDER BY nom_complet"
    ))?;
    let utilisateurs = requete
        .query_map([], ligne_vers_utilisateur)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(utilisateurs)
}

/// Création d'un compte par un administrateur.
pub fn creer(
    conn: &Connection,
    acteur: &Utilisateur,
    nouveau: &NouvelUtilisateur,
) -> Resultat<Utilisateur> {
    exiger(conn, acteur, Droit::GererUtilisateurs)?;
    creer_sans_controle(conn, nouveau, Some(acteur.id))
}

/// Création sans contrôle de droit : réservée à l'installation initiale.
pub(crate) fn creer_sans_controle(
    conn: &Connection,
    nouveau: &NouvelUtilisateur,
    auteur: Option<i64>,
) -> Resultat<Utilisateur> {
    let nom_complet = nouveau.nom_complet.trim();
    let identifiant = nouveau.identifiant.trim();
    if nom_complet.is_empty() {
        return Err(Erreur::Validation("le nom complet est obligatoire".into()));
    }
    if identifiant.is_empty() || identifiant.contains(char::is_whitespace) {
        return Err(Erreur::Validation(
            "l'identifiant est obligatoire et ne contient pas d'espace".into(),
        ));
    }
    valider_mot_de_passe(&nouveau.mot_de_passe)?;

    let deja_pris: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM utilisateur WHERE identifiant = ?1)",
        [identifiant],
        |r| r.get(0),
    )?;
    if deja_pris {
        return Err(Erreur::Validation(format!(
            "l'identifiant « {identifiant} » est déjà utilisé"
        )));
    }

    conn.execute(
        "INSERT INTO utilisateur (nom_complet, identifiant, mot_de_passe_hache, role, actif, date_creation)
         VALUES (?1, ?2, ?3, ?4, 1, ?5)",
        params![
            nom_complet,
            identifiant,
            hacher(&nouveau.mot_de_passe)?,
            nouveau.role.code(),
            maintenant()
        ],
    )?;
    let id = conn.last_insert_rowid();
    audit::journaliser(
        conn,
        Entree {
            entite: "utilisateur",
            entite_id: id,
            action: "creation",
            ancienne_valeur: None,
            nouvelle_valeur: Some(json!({
                "nom_complet": nom_complet,
                "identifiant": identifiant,
                "role": nouveau.role.code(),
            })),
            utilisateur_id: auteur,
        },
    )?;
    Ok(Utilisateur {
        id,
        nom_complet: nom_complet.to_string(),
        identifiant: identifiant.to_string(),
        role: nouveau.role,
        actif: true,
        langue: Langue::default(),
    })
}

/// Ouvre une session. Le message d'échec ne dit pas si c'est l'identifiant,
/// le mot de passe ou l'état du compte qui pose problème.
pub fn connecter(
    conn: &Connection,
    identifiant: &str,
    mot_de_passe: &str,
) -> Resultat<Utilisateur> {
    let trouve: Option<(Utilisateur, String)> = conn
        .query_row(
            &format!(
                "SELECT {COLONNES}, mot_de_passe_hache FROM utilisateur WHERE identifiant = ?1"
            ),
            [identifiant.trim()],
            |r| Ok((ligne_vers_utilisateur(r)?, r.get(6)?)),
        )
        .optional()?;
    match trouve {
        Some((utilisateur, hache))
            if utilisateur.actif && mot_de_passe_correct(mot_de_passe, &hache) =>
        {
            audit::journaliser(
                conn,
                Entree {
                    entite: "utilisateur",
                    entite_id: utilisateur.id,
                    action: "connexion",
                    ancienne_valeur: None,
                    nouvelle_valeur: None,
                    utilisateur_id: Some(utilisateur.id),
                },
            )?;
            Ok(utilisateur)
        }
        _ => Err(Erreur::ConnexionRefusee),
    }
}

/// Changement de son propre mot de passe : l'ancien est exigé.
pub fn changer_mot_de_passe(
    conn: &Connection,
    id: i64,
    ancien: &str,
    nouveau: &str,
) -> Resultat<()> {
    let hache: Option<String> = conn
        .query_row(
            "SELECT mot_de_passe_hache FROM utilisateur WHERE id = ?1 AND actif = 1",
            [id],
            |r| r.get(0),
        )
        .optional()?;
    match hache {
        Some(h) if mot_de_passe_correct(ancien, &h) => {}
        _ => {
            return Err(Erreur::Validation(
                "le mot de passe actuel est incorrect".into(),
            ));
        }
    }
    enregistrer_mot_de_passe(conn, id, nouveau, "changement_mot_de_passe", id)
}

/// Réinitialisation par un administrateur.
pub fn reinitialiser_mot_de_passe(
    conn: &Connection,
    acteur: &Utilisateur,
    id: i64,
    nouveau: &str,
) -> Resultat<()> {
    exiger(conn, acteur, Droit::GererUtilisateurs)?;
    enregistrer_mot_de_passe(
        conn,
        id,
        nouveau,
        "reinitialisation_mot_de_passe",
        acteur.id,
    )
}

fn enregistrer_mot_de_passe(
    conn: &Connection,
    id: i64,
    nouveau: &str,
    action: &str,
    auteur: i64,
) -> Resultat<()> {
    valider_mot_de_passe(nouveau)?;
    let modifies = conn.execute(
        "UPDATE utilisateur SET mot_de_passe_hache = ?1 WHERE id = ?2",
        params![hacher(nouveau)?, id],
    )?;
    if modifies == 0 {
        return Err(Erreur::Validation("compte introuvable".into()));
    }
    audit::journaliser(
        conn,
        Entree {
            entite: "utilisateur",
            entite_id: id,
            action,
            ancienne_valeur: None,
            nouvelle_valeur: None,
            utilisateur_id: Some(auteur),
        },
    )
}

/// Refuse toute opération qui retirerait le dernier administrateur actif.
fn garder_un_admin(conn: &Connection, cible: &Utilisateur) -> Resultat<()> {
    if cible.role == Role::Admin && cible.actif && admins_actifs(conn)? <= 1 {
        return Err(Erreur::DernierAdmin);
    }
    Ok(())
}

fn lire_existant(conn: &Connection, id: i64) -> Resultat<Utilisateur> {
    lire(conn, id)?.ok_or_else(|| Erreur::Validation("compte introuvable".into()))
}

pub fn definir_actif(
    conn: &Connection,
    acteur: &Utilisateur,
    id: i64,
    actif: bool,
) -> Resultat<()> {
    exiger(conn, acteur, Droit::GererUtilisateurs)?;
    let auteur = acteur.id;
    let cible = lire_existant(conn, id)?;
    if cible.actif == actif {
        return Ok(());
    }
    if !actif {
        garder_un_admin(conn, &cible)?;
    }
    conn.execute(
        "UPDATE utilisateur SET actif = ?1 WHERE id = ?2",
        params![actif, id],
    )?;
    audit::journaliser(
        conn,
        Entree {
            entite: "utilisateur",
            entite_id: id,
            action: if actif { "activation" } else { "desactivation" },
            ancienne_valeur: None,
            nouvelle_valeur: None,
            utilisateur_id: Some(auteur),
        },
    )
}

pub fn changer_role(conn: &Connection, acteur: &Utilisateur, id: i64, role: Role) -> Resultat<()> {
    exiger(conn, acteur, Droit::GererUtilisateurs)?;
    let auteur = acteur.id;
    let cible = lire_existant(conn, id)?;
    if cible.role == role {
        return Ok(());
    }
    if role != Role::Admin {
        garder_un_admin(conn, &cible)?;
    }
    conn.execute(
        "UPDATE utilisateur SET role = ?1 WHERE id = ?2",
        params![role.code(), id],
    )?;
    audit::journaliser(
        conn,
        Entree {
            entite: "utilisateur",
            entite_id: id,
            action: "changement_role",
            ancienne_valeur: Some(json!({ "role": cible.role.code() })),
            nouvelle_valeur: Some(json!({ "role": role.code() })),
            utilisateur_id: Some(auteur),
        },
    )
}

/// Choix de la langue de l'interface pour son propre compte. Aucun droit requis :
/// chacun règle sa langue ; le compte est relu en base (il doit être actif).
pub fn changer_langue(conn: &Connection, acteur: &Utilisateur, langue: Langue) -> Resultat<()> {
    let ancienne: Option<String> = conn
        .query_row(
            "SELECT langue FROM utilisateur WHERE id = ?1 AND actif = 1",
            [acteur.id],
            |r| r.get(0),
        )
        .optional()?;
    let Some(ancienne) = ancienne else {
        return Err(Erreur::Validation("compte inconnu ou désactivé".into()));
    };
    if ancienne == langue.code() {
        return Ok(());
    }
    conn.execute(
        "UPDATE utilisateur SET langue = ?1 WHERE id = ?2",
        params![langue.code(), acteur.id],
    )?;
    audit::journaliser(
        conn,
        Entree {
            entite: "utilisateur",
            entite_id: acteur.id,
            action: "changement_langue",
            ancienne_valeur: Some(json!({ "langue": ancienne })),
            nouvelle_valeur: Some(json!({ "langue": langue.code() })),
            utilisateur_id: Some(acteur.id),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ouvrir_en_memoire;

    #[test]
    fn langue_du_compte() {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = crate::droits::tests::compte(&conn, "admin", Role::Admin);
        assert_eq!(admin.langue, Langue::Francais);
        changer_langue(&conn, &admin, Langue::Japonais).unwrap();
        assert_eq!(
            lire(&conn, admin.id).unwrap().unwrap().langue,
            Langue::Japonais
        );
        assert_eq!(
            connecter(&conn, "admin", "motdepasse1")
                .map(|u| u.langue)
                .ok(),
            Some(Langue::Japonais)
        );
        let refus = conn.execute(
            "UPDATE utilisateur SET langue = 'de' WHERE id = ?1",
            [admin.id],
        );
        assert!(refus.is_err(), "code de langue hors liste refusé");
    }

    fn nouveau(identifiant: &str, role: Role) -> NouvelUtilisateur {
        NouvelUtilisateur {
            nom_complet: format!("Compte {identifiant}"),
            identifiant: identifiant.into(),
            mot_de_passe: "motdepasse1".into(),
            role,
        }
    }

    fn installer_admin(conn: &Connection) -> Utilisateur {
        creer_sans_controle(conn, &nouveau("admin", Role::Admin), None).unwrap()
    }

    #[test]
    fn assistant_tant_qu_aucun_admin_actif() {
        let conn = ouvrir_en_memoire().unwrap();
        assert_eq!(
            etat_installation(&conn).unwrap(),
            EtatInstallation::AssistantRequis
        );
        creer_sans_controle(&conn, &nouveau("compta", Role::Comptable), None).unwrap();
        assert_eq!(
            etat_installation(&conn).unwrap(),
            EtatInstallation::AssistantRequis,
            "un compte non administrateur ne suffit pas"
        );
        installer_admin(&conn);
        assert_eq!(
            etat_installation(&conn).unwrap(),
            EtatInstallation::ConnexionRequise
        );
    }

    #[test]
    fn connexion() {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = installer_admin(&conn);
        let ok = connecter(&conn, " admin ", "motdepasse1").unwrap();
        assert_eq!(ok, admin);
        assert!(matches!(
            connecter(&conn, "admin", "mauvais"),
            Err(Erreur::ConnexionRefusee)
        ));
        assert!(matches!(
            connecter(&conn, "inconnu", "motdepasse1"),
            Err(Erreur::ConnexionRefusee)
        ));
        assert_eq!(audit::compter(&conn, "connexion").unwrap(), 1);
    }

    #[test]
    fn identifiant_insensible_a_la_casse() {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = creer_sans_controle(&conn, &nouveau("Admin", Role::Admin), None).unwrap();
        assert!(connecter(&conn, "admin", "motdepasse1").is_ok());
        assert!(matches!(
            creer(&conn, &admin, &nouveau("ADMIN", Role::Commercial)),
            Err(Erreur::Validation(_))
        ));
    }

    #[test]
    fn seul_l_admin_gere_les_comptes() {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = installer_admin(&conn);
        let compta = creer(&conn, &admin, &nouveau("compta", Role::Comptable)).unwrap();
        assert!(matches!(
            creer(&conn, &compta, &nouveau("autre", Role::Commercial)),
            Err(Erreur::Interdit(Droit::GererUtilisateurs))
        ));
        assert!(matches!(
            definir_actif(&conn, &compta, admin.id, false),
            Err(Erreur::Interdit(_))
        ));
        assert!(matches!(
            reinitialiser_mot_de_passe(&conn, &compta, admin.id, "piratage1"),
            Err(Erreur::Interdit(_))
        ));
    }

    #[test]
    fn compte_desactive_refuse() {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = installer_admin(&conn);
        let commercial = creer(&conn, &admin, &nouveau("vente", Role::Commercial)).unwrap();
        definir_actif(&conn, &admin, commercial.id, false).unwrap();
        assert!(matches!(
            connecter(&conn, "vente", "motdepasse1"),
            Err(Erreur::ConnexionRefusee)
        ));
    }

    #[test]
    fn validations() {
        let conn = ouvrir_en_memoire().unwrap();
        let mut court = nouveau("admin", Role::Admin);
        court.mot_de_passe = "court".into();
        assert!(matches!(
            creer_sans_controle(&conn, &court, None),
            Err(Erreur::Validation(_))
        ));
        let mut espace = nouveau("un admin", Role::Admin);
        espace.identifiant = "un admin".into();
        assert!(matches!(
            creer_sans_controle(&conn, &espace, None),
            Err(Erreur::Validation(_))
        ));
        let mut sans_nom = nouveau("admin", Role::Admin);
        sans_nom.nom_complet = "  ".into();
        assert!(matches!(
            creer_sans_controle(&conn, &sans_nom, None),
            Err(Erreur::Validation(_))
        ));
    }

    #[test]
    fn garde_du_dernier_admin() {
        let conn = ouvrir_en_memoire().unwrap();
        let a1 = creer_sans_controle(&conn, &nouveau("a1", Role::Admin), None).unwrap();
        assert!(matches!(
            definir_actif(&conn, &a1, a1.id, false),
            Err(Erreur::DernierAdmin)
        ));
        assert!(matches!(
            changer_role(&conn, &a1, a1.id, Role::Comptable),
            Err(Erreur::DernierAdmin)
        ));

        let a2 = creer(&conn, &a1, &nouveau("a2", Role::Admin)).unwrap();
        changer_role(&conn, &a2, a1.id, Role::Comptable).unwrap();
        assert!(matches!(
            definir_actif(&conn, &a2, a2.id, false),
            Err(Erreur::DernierAdmin)
        ));
        // a1 n'est plus administrateur : ses droits sont relus en base.
        assert!(matches!(
            creer(&conn, &a1, &nouveau("a3", Role::Admin)),
            Err(Erreur::Interdit(_))
        ));
        assert_eq!(audit::compter(&conn, "changement_role").unwrap(), 1);
    }

    #[test]
    fn mots_de_passe() {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = installer_admin(&conn);
        assert!(matches!(
            changer_mot_de_passe(&conn, admin.id, "mauvais", "nouveaumdp1"),
            Err(Erreur::Validation(_))
        ));
        changer_mot_de_passe(&conn, admin.id, "motdepasse1", "nouveaumdp1").unwrap();
        assert!(connecter(&conn, "admin", "motdepasse1").is_err());
        assert!(connecter(&conn, "admin", "nouveaumdp1").is_ok());

        let vente = creer(&conn, &admin, &nouveau("vente", Role::Commercial)).unwrap();
        reinitialiser_mot_de_passe(&conn, &admin, vente.id, "provisoire1").unwrap();
        assert!(connecter(&conn, "vente", "provisoire1").is_ok());
    }

    #[test]
    fn mot_de_passe_jamais_en_clair() {
        let conn = ouvrir_en_memoire().unwrap();
        installer_admin(&conn);
        let hache: String = conn
            .query_row("SELECT mot_de_passe_hache FROM utilisateur", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert!(hache.starts_with("$argon2id$"));
        assert!(!hache.contains("motdepasse1"));
    }
}
