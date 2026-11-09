//! Installation initiale faite par l'assistant de premier lancement
//! (cahier des charges, section 4.1).
//!
//! Tout est écrit dans une seule transaction : un arrêt en cours de route ne peut
//! pas laisser une base sans compte administrateur derrière un écran de connexion.
//! L'étape de licence s'ajoutera en tête de l'assistant au jalon J9.

use kadenz_domaine::role::Role;
use rusqlite::Connection;

use crate::audit::{self, Entree};
use crate::comptes::{self, NouvelUtilisateur, Utilisateur};
use crate::entreprise::{self, Entreprise};
use crate::parametres::{self, Parametres};
use crate::{Erreur, Resultat};

pub struct InstallationInitiale {
    pub entreprise: Entreprise,
    /// Le rôle est forcé à Admin.
    pub admin: NouvelUtilisateur,
    pub parametres: Parametres,
}

/// Exécute l'installation et renvoie le compte administrateur créé.
pub fn installer(conn: &mut Connection, init: &InstallationInitiale) -> Resultat<Utilisateur> {
    let tx = conn.transaction()?;
    if comptes::admins_actifs(&tx)? > 0 {
        return Err(Erreur::DejaInstalle);
    }
    let admin = NouvelUtilisateur {
        role: Role::Admin,
        ..init.admin.clone()
    };
    let admin = comptes::creer_sans_controle(&tx, &admin, None)?;
    entreprise::enregistrer_sans_controle(&tx, &init.entreprise, Some(admin.id))?;
    parametres::enregistrer_sans_controle(&tx, &init.parametres, Some(admin.id))?;
    audit::journaliser(
        &tx,
        Entree {
            entite: "installation",
            entite_id: 0,
            action: "installation",
            ancienne_valeur: None,
            nouvelle_valeur: None,
            utilisateur_id: Some(admin.id),
        },
    )?;
    tx.commit()?;
    Ok(admin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comptes::{EtatInstallation, etat_installation};
    use crate::ouvrir_en_memoire;

    fn donnees() -> InstallationInitiale {
        InstallationInitiale {
            entreprise: Entreprise {
                nom: "Entreprise Test".into(),
                ..Default::default()
            },
            admin: NouvelUtilisateur {
                nom_complet: "Administrateur".into(),
                identifiant: "admin".into(),
                mot_de_passe: "motdepasse1".into(),
                role: Role::Consultation,
            },
            parametres: Parametres::default(),
        }
    }

    #[test]
    fn installation_complete() {
        let mut conn = ouvrir_en_memoire().unwrap();
        let admin = installer(&mut conn, &donnees()).unwrap();
        assert_eq!(admin.role, Role::Admin, "le rôle est forcé à Admin");
        assert_eq!(
            etat_installation(&conn).unwrap(),
            EtatInstallation::ConnexionRequise
        );
        assert!(entreprise::lire(&conn).unwrap().is_some());
        assert!(matches!(
            installer(&mut conn, &donnees()),
            Err(Erreur::DejaInstalle)
        ));
    }

    #[test]
    fn echec_sans_trace() {
        let mut conn = ouvrir_en_memoire().unwrap();
        let mut d = donnees();
        d.entreprise.nom = String::new();
        assert!(installer(&mut conn, &d).is_err());
        // Le compte créé avant l'échec a été annulé avec la transaction.
        assert_eq!(
            etat_installation(&conn).unwrap(),
            EtatInstallation::AssistantRequis
        );
        assert!(comptes::lister(&conn).unwrap().is_empty());
    }
}
