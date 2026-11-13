//! Contrôle des droits et persistance de la matrice (cahier des charges, section 10.1).
//!
//! Toute fonction publique du stockage qui modifie des données reçoit le compte
//! qui agit et vérifie son droit ici, d'après la matrice enregistrée (ou celle du
//! cahier des charges si aucune n'a été enregistrée).

use kadenz_domaine::role::{Droit, MatriceDroits, Role};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Map, Value, json};

use crate::audit::{self, Entree};
use crate::comptes::Utilisateur;
use crate::{Erreur, Resultat};

const CLE_MATRICE: &str = "droits.matrice";

pub fn lire_matrice(conn: &Connection) -> Resultat<MatriceDroits> {
    let texte: Option<String> = conn
        .query_row(
            "SELECT valeur FROM parametre_systeme WHERE cle = ?1",
            [CLE_MATRICE],
            |r| r.get(0),
        )
        .optional()?;
    Ok(texte
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .map(|v| matrice_depuis_json(&v))
        .unwrap_or_default())
}

/// Matrice décrite en JSON (rôle vers la liste de ses droits), telle que
/// gardée en base et envoyée par un poste relié.
pub fn matrice_depuis_json(valeur: &Value) -> MatriceDroits {
    let mut matrice = MatriceDroits::vide();
    if let Some(objet) = valeur.as_object() {
        for (code_role, droits) in objet {
            let Some(role) = Role::depuis_code(code_role) else {
                continue;
            };
            for code in droits.as_array().into_iter().flatten() {
                if let Some(droit) = code.as_str().and_then(Droit::depuis_code) {
                    matrice.definir(role, droit, true);
                }
            }
        }
    }
    matrice
}

pub fn matrice_en_json(matrice: &MatriceDroits) -> Value {
    let mut objet = Map::new();
    for role in Role::TOUS {
        let droits: Vec<&str> = matrice
            .autorisations()
            .filter(|(r, _)| *r == role)
            .map(|(_, d)| d.code())
            .collect();
        objet.insert(role.code().to_string(), json!(droits));
    }
    Value::Object(objet)
}

pub fn enregistrer_matrice(
    conn: &Connection,
    acteur: &Utilisateur,
    matrice: &MatriceDroits,
) -> Resultat<()> {
    exiger(conn, acteur, Droit::GererUtilisateurs)?;
    let valeur = matrice_en_json(matrice);
    conn.execute(
        "INSERT INTO parametre_systeme (cle, valeur) VALUES (?1, ?2)
         ON CONFLICT (cle) DO UPDATE SET valeur = excluded.valeur",
        params![CLE_MATRICE, valeur.to_string()],
    )?;
    audit::journaliser(
        conn,
        Entree {
            entite: "droits",
            entite_id: 0,
            action: "modification",
            ancienne_valeur: None,
            nouvelle_valeur: Some(valeur),
            utilisateur_id: Some(acteur.id),
        },
    )
}

/// Vérifie que le compte qui agit possède le droit demandé. Le rôle et l'état du
/// compte sont relus en base : un changement de rôle ou une désactivation prend
/// effet immédiatement, même pour une session déjà ouverte.
pub fn exiger(conn: &Connection, acteur: &Utilisateur, droit: Droit) -> Resultat<()> {
    if possede(conn, acteur, droit)? {
        Ok(())
    } else {
        Err(Erreur::Interdit(droit))
    }
}

/// Vérifie un droit sans erreur, pour adapter l'interface (boutons, menus).
pub fn possede(conn: &Connection, acteur: &Utilisateur, droit: Droit) -> Resultat<bool> {
    Ok(match crate::comptes::lire(conn, acteur.id)? {
        Some(courant) => courant.actif && lire_matrice(conn)?.autorise(courant.role, droit),
        None => false,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::comptes::{self, NouvelUtilisateur};
    use crate::ouvrir_en_memoire;

    /// Crée un compte du rôle donné, pour les tests des autres modules.
    pub(crate) fn compte(conn: &Connection, identifiant: &str, role: Role) -> Utilisateur {
        comptes::creer_sans_controle(
            conn,
            &NouvelUtilisateur {
                nom_complet: format!("Compte {identifiant}"),
                identifiant: identifiant.into(),
                mot_de_passe: "motdepasse1".into(),
                role,
            },
            None,
        )
        .unwrap()
    }

    #[test]
    fn matrice_par_defaut_puis_enregistree() {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        let vente = compte(&conn, "vente", Role::Commercial);
        assert!(exiger(&conn, &vente, Droit::ModifierReferentiel).is_err());

        let mut m = lire_matrice(&conn).unwrap();
        m.definir(Role::Commercial, Droit::ModifierReferentiel, true);
        assert!(matches!(
            enregistrer_matrice(&conn, &vente, &m),
            Err(Erreur::Interdit(Droit::GererUtilisateurs))
        ));
        enregistrer_matrice(&conn, &admin, &m).unwrap();
        assert_eq!(lire_matrice(&conn).unwrap(), m);
        assert!(exiger(&conn, &vente, Droit::ModifierReferentiel).is_ok());
    }

    #[test]
    fn compte_inactif_sans_droit() {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        conn.execute("UPDATE utilisateur SET actif = 0", [])
            .unwrap();
        assert!(exiger(&conn, &admin, Droit::Consulter).is_err());
    }
}
