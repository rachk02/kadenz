//! Fiche de l'entreprise utilisatrice, une seule par installation
//! (cahier des charges, sections 5.1 et 12).
//!
//! Les deux logos sont stockés dans la base, pas comme chemins de fichiers : ils
//! suivent ainsi les sauvegardes, les restaurations et les changements de poste.

use kadenz_domaine::role::Droit;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use crate::audit::{self, Entree};
use crate::comptes::Utilisateur;
use crate::droits::exiger;
use crate::{Erreur, Resultat};

/// Taille maximale d'un logo : largement assez pour un en-tête de document A4.
pub const TAILLE_MAX_LOGO: usize = 2 * 1024 * 1024;

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
    pub responsable_facturation: String,
    pub responsable_comptabilite: String,
}

/// Logo principal ou secondaire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Logo {
    Principal,
    Secondaire,
}

impl Logo {
    fn colonne(self) -> &'static str {
        match self {
            Logo::Principal => "logo",
            Logo::Secondaire => "logo2",
        }
    }
}

/// Format d'image reconnu d'après les premiers octets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatImage {
    Png,
    Jpeg,
}

impl FormatImage {
    pub fn detecter(octets: &[u8]) -> Option<Self> {
        if octets.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
            Some(FormatImage::Png)
        } else if octets.starts_with(&[0xFF, 0xD8, 0xFF]) {
            Some(FormatImage::Jpeg)
        } else {
            None
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            FormatImage::Png => "png",
            FormatImage::Jpeg => "jpg",
        }
    }
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
            responsable_facturation: t(&self.responsable_facturation),
            responsable_comptabilite: t(&self.responsable_comptabilite),
        }
    }
}

pub fn lire(conn: &Connection) -> Resultat<Option<Entreprise>> {
    Ok(conn
        .query_row(
            "SELECT nom, adresse, telephone, email, site_web, localisation, regime_imposition,
                    rccm, ifu, banque1, banque2, responsable_facturation, responsable_comptabilite
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
                    responsable_facturation: r.get(11)?,
                    responsable_comptabilite: r.get(12)?,
                })
            },
        )
        .optional()?)
}

/// Met à jour la fiche. Réservé au droit « Gérer la fiche entreprise et les paramètres ».
pub fn enregistrer(
    conn: &Connection,
    acteur: &Utilisateur,
    entreprise: &Entreprise,
) -> Resultat<()> {
    exiger(conn, acteur, Droit::GererParametres)?;
    enregistrer_sans_controle(conn, entreprise, Some(acteur.id))
}

/// Écriture sans contrôle de droit : réservée à l'installation initiale.
pub(crate) fn enregistrer_sans_controle(
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
                                 regime_imposition, rccm, ifu, banque1, banque2,
                                 responsable_facturation, responsable_comptabilite)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
         ON CONFLICT (id) DO UPDATE SET
             nom = excluded.nom, adresse = excluded.adresse, telephone = excluded.telephone,
             email = excluded.email, site_web = excluded.site_web,
             localisation = excluded.localisation, regime_imposition = excluded.regime_imposition,
             rccm = excluded.rccm, ifu = excluded.ifu, banque1 = excluded.banque1,
             banque2 = excluded.banque2,
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

pub fn lire_logo(conn: &Connection, logo: Logo) -> Resultat<Option<Vec<u8>>> {
    Ok(conn
        .query_row(
            &format!("SELECT {} FROM entreprise WHERE id = 1", logo.colonne()),
            [],
            |r| r.get::<_, Option<Vec<u8>>>(0),
        )
        .optional()?
        .flatten())
}

/// Remplace un logo (`Some`) ou le retire (`None`). PNG ou JPEG, 2 Mo au plus.
pub fn definir_logo(
    conn: &Connection,
    acteur: &Utilisateur,
    logo: Logo,
    octets: Option<&[u8]>,
) -> Resultat<()> {
    exiger(conn, acteur, Droit::GererParametres)?;
    if let Some(o) = octets {
        if FormatImage::detecter(o).is_none() {
            return Err(Erreur::Validation(
                "le logo doit être une image PNG ou JPEG".into(),
            ));
        }
        if o.len() > TAILLE_MAX_LOGO {
            return Err(Erreur::Validation(format!(
                "le logo dépasse la taille maximale de {} Mo",
                TAILLE_MAX_LOGO / (1024 * 1024)
            )));
        }
    }
    let modifies = conn.execute(
        &format!("UPDATE entreprise SET {} = ?1 WHERE id = 1", logo.colonne()),
        params![octets],
    )?;
    if modifies == 0 {
        return Err(Erreur::Validation(
            "la fiche entreprise n'existe pas encore".into(),
        ));
    }
    audit::journaliser(
        conn,
        Entree {
            entite: "entreprise",
            entite_id: 1,
            action: if octets.is_some() {
                "logo_defini"
            } else {
                "logo_retire"
            },
            ancienne_valeur: None,
            nouvelle_valeur: Some(
                json!({ "logo": logo.colonne(), "octets": octets.map(<[u8]>::len) }),
            ),
            utilisateur_id: Some(acteur.id),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::droits::tests::compte;
    use crate::ouvrir_en_memoire;
    use kadenz_domaine::role::Role;

    const PNG_MINIMAL: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0];

    fn base_installee() -> (Connection, Utilisateur) {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        let e = Entreprise {
            nom: "  Entreprise Test  ".into(),
            ..Default::default()
        };
        enregistrer(&conn, &admin, &e).unwrap();
        (conn, admin)
    }

    #[test]
    fn creation_puis_modification() {
        let (conn, admin) = base_installee();
        let mut e = lire(&conn).unwrap().unwrap();
        assert_eq!(e.nom, "Entreprise Test");
        e.telephone = "+226 00 00 00 00".into();
        enregistrer(&conn, &admin, &e).unwrap();
        assert_eq!(lire(&conn).unwrap().unwrap().telephone, "+226 00 00 00 00");
        assert_eq!(audit::compter(&conn, "modification").unwrap(), 1);
    }

    #[test]
    fn nom_obligatoire_et_droit_exige() {
        let (conn, _) = base_installee();
        let compta = compte(&conn, "compta", Role::Comptable);
        let e = lire(&conn).unwrap().unwrap();
        assert!(matches!(
            enregistrer(&conn, &compta, &e),
            Err(Erreur::Interdit(Droit::GererParametres))
        ));
        assert!(matches!(
            enregistrer_sans_controle(&conn, &Entreprise::default(), None),
            Err(Erreur::Validation(_))
        ));
    }

    #[test]
    fn logos() {
        let (conn, admin) = base_installee();
        assert_eq!(lire_logo(&conn, Logo::Principal).unwrap(), None);
        definir_logo(&conn, &admin, Logo::Principal, Some(PNG_MINIMAL)).unwrap();
        assert_eq!(
            lire_logo(&conn, Logo::Principal).unwrap().as_deref(),
            Some(PNG_MINIMAL)
        );
        assert_eq!(lire_logo(&conn, Logo::Secondaire).unwrap(), None);

        assert!(matches!(
            definir_logo(&conn, &admin, Logo::Secondaire, Some(b"GIF89a")),
            Err(Erreur::Validation(_))
        ));
        let trop_gros = [PNG_MINIMAL, &vec![0u8; TAILLE_MAX_LOGO]].concat();
        assert!(matches!(
            definir_logo(&conn, &admin, Logo::Secondaire, Some(&trop_gros)),
            Err(Erreur::Validation(_))
        ));

        definir_logo(&conn, &admin, Logo::Principal, None).unwrap();
        assert_eq!(lire_logo(&conn, Logo::Principal).unwrap(), None);
    }

    #[test]
    fn format_image() {
        assert_eq!(FormatImage::detecter(PNG_MINIMAL), Some(FormatImage::Png));
        assert_eq!(
            FormatImage::detecter(&[0xFF, 0xD8, 0xFF, 0xE0]),
            Some(FormatImage::Jpeg)
        );
        assert_eq!(FormatImage::detecter(b"BM"), None);
    }
}
