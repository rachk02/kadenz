//! Fiche de l'entreprise utilisatrice, une seule par installation
//! (cahier des charges, section 5.1).

use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use crate::audit::{self, Entree};
use crate::{Erreur, Resultat};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Entreprise {
    pub nom: String,
    pub adresse: String,
    pub telephone: String,
    pub email: String,
    pub site_web: String,
    pub localisation: String,
    pub regime_imposition: String,
    pub rccm: String,
    pub ifu: String,
    pub banque1: String,
    pub banque2: String,
    pub logo_chemin: String,
    pub logo2_chemin: String,
    pub responsable_facturation: String,
    pub responsable_comptabilite: String,
}

impl Entreprise {
    fn en_json(&self) -> Value {
        json!({
            "nom": self.nom,
            "adresse": self.adresse,
            "telephone": self.telephone,
            "email": self.email,
            "site_web": self.site_web,
            "localisation": self.localisation,
            "regime_imposition": self.regime_imposition,
            "rccm": self.rccm,
            "ifu": self.ifu,
            "banque1": self.banque1,
            "banque2": self.banque2,
            "logo_chemin": self.logo_chemin,
            "logo2_chemin": self.logo2_chemin,
            "responsable_facturation": self.responsable_facturation,
            "responsable_comptabilite": self.responsable_comptabilite,
        })
    }

    /// Supprime les espaces superflus de chaque champ.
    fn nettoyee(&self) -> Entreprise {
        let t = |s: &String| s.trim().to_string();
        Entreprise {
            nom: t(&self.nom),
            adresse: t(&self.adresse),
            telephone: t(&self.telephone),
            email: t(&self.email),
            site_web: t(&self.site_web),
            localisation: t(&self.localisation),
            regime_imposition: t(&self.regime_imposition),
            rccm: t(&self.rccm),
            ifu: t(&self.ifu),
            banque1: t(&self.banque1),
            banque2: t(&self.banque2),
            logo_chemin: t(&self.logo_chemin),
            logo2_chemin: t(&self.logo2_chemin),
            responsable_facturation: t(&self.responsable_facturation),
            responsable_comptabilite: t(&self.responsable_comptabilite),
        }
    }
}

pub fn lire(conn: &Connection) -> Resultat<Option<Entreprise>> {
    Ok(conn
        .query_row(
            "SELECT nom, adresse, telephone, email, site_web, localisation, regime_imposition,
                    rccm, ifu, banque1, banque2, logo_chemin, logo2_chemin,
                    responsable_facturation, responsable_comptabilite
             FROM entreprise WHERE id = 1",
            [],
            |r| {
                Ok(Entreprise {
                    nom: r.get(0)?,
                    adresse: r.get(1)?,
                    telephone: r.get(2)?,
                    email: r.get(3)?,
                    site_web: r.get(4)?,
                    localisation: r.get(5)?,
                    regime_imposition: r.get(6)?,
                    rccm: r.get(7)?,
                    ifu: r.get(8)?,
                    banque1: r.get(9)?,
                    banque2: r.get(10)?,
                    logo_chemin: r.get(11)?,
                    logo2_chemin: r.get(12)?,
                    responsable_facturation: r.get(13)?,
                    responsable_comptabilite: r.get(14)?,
                })
            },
        )
        .optional()?)
}

/// Crée ou met à jour la fiche. Seul le nom est obligatoire.
pub fn enregistrer(
    conn: &Connection,
    entreprise: &Entreprise,
    auteur: Option<i64>,
) -> Resultat<()> {
    let e = entreprise.nettoyee();
    if e.nom.is_empty() {
        return Err(Erreur::Validation(
            "le nom de l'entreprise est obligatoire".into(),
        ));
    }
    let ancienne = lire(conn)?;
    conn.execute(
        "INSERT INTO entreprise (id, nom, adresse, telephone, email, site_web, localisation,
                                 regime_imposition, rccm, ifu, banque1, banque2, logo_chemin,
                                 logo2_chemin, responsable_facturation, responsable_comptabilite)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
         ON CONFLICT (id) DO UPDATE SET
             nom = excluded.nom, adresse = excluded.adresse, telephone = excluded.telephone,
             email = excluded.email, site_web = excluded.site_web,
             localisation = excluded.localisation, regime_imposition = excluded.regime_imposition,
             rccm = excluded.rccm, ifu = excluded.ifu, banque1 = excluded.banque1,
             banque2 = excluded.banque2, logo_chemin = excluded.logo_chemin,
             logo2_chemin = excluded.logo2_chemin,
             responsable_facturation = excluded.responsable_facturation,
             responsable_comptabilite = excluded.responsable_comptabilite",
        params![
            e.nom,
            e.adresse,
            e.telephone,
            e.email,
            e.site_web,
            e.localisation,
            e.regime_imposition,
            e.rccm,
            e.ifu,
            e.banque1,
            e.banque2,
            e.logo_chemin,
            e.logo2_chemin,
            e.responsable_facturation,
            e.responsable_comptabilite
        ],
    )?;
    audit::journaliser(
        conn,
        Entree {
            entite: "entreprise",
            entite_id: 1,
            action: if ancienne.is_some() {
                "modification"
            } else {
                "creation"
            },
            ancienne_valeur: ancienne.map(|a| a.en_json()),
            nouvelle_valeur: Some(e.en_json()),
            utilisateur_id: auteur,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ouvrir_en_memoire;

    #[test]
    fn creation_puis_modification() {
        let conn = ouvrir_en_memoire().unwrap();
        assert_eq!(lire(&conn).unwrap(), None);

        let mut e = Entreprise {
            nom: "  Entreprise Test  ".into(),
            ifu: "123".into(),
            ..Default::default()
        };
        enregistrer(&conn, &e, None).unwrap();
        assert_eq!(lire(&conn).unwrap().unwrap().nom, "Entreprise Test");

        e.telephone = "+226 00 00 00 00".into();
        enregistrer(&conn, &e, None).unwrap();
        let lue = lire(&conn).unwrap().unwrap();
        assert_eq!(lue.telephone, "+226 00 00 00 00");
        assert_eq!(audit::compter(&conn, "creation").unwrap(), 1);
        assert_eq!(audit::compter(&conn, "modification").unwrap(), 1);
    }

    #[test]
    fn nom_obligatoire() {
        let conn = ouvrir_en_memoire().unwrap();
        let e = Entreprise::default();
        assert!(matches!(
            enregistrer(&conn, &e, None),
            Err(Erreur::Validation(_))
        ));
    }
}
