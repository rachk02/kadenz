//! Lettre de relance d'une facture échue (cahier des charges, section 8).
//!
//! Le texte par défaut a été validé par l'utilisateur le 2026-11-10 (jalon J7) ;
//! titre, objet, texte avant et après le tableau récapitulatif et signataire se
//! règlent dans les paramètres. Dans l'objet et les deux textes, `{numero}`,
//! `{date}`, `{echeance}`, `{reste}` et `{retard}` reçoivent les valeurs de la
//! facture.
//!
//! La lettre s'imprime à la date du jour, avec la fiche entreprise du moment :
//! c'est un nouveau courrier, pas la réimpression d'un document validé.

use chrono::NaiveDate;
use kadenz_domaine::calcul::Montant;
use kadenz_domaine::role::Droit;
use kadenz_domaine::saisie::montant_en_texte;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use crate::audit::{self, Entree};
use crate::comptes::Utilisateur;
use crate::documents::{self, SnapshotTiers};
use crate::droits::exiger;
use crate::entreprise::{self, Entreprise, Logo};
use crate::parametres;
use crate::reglements;
use crate::{Erreur, Resultat};

const CLE_TEXTES_RELANCE: &str = "relance.textes";

/// Textes réglables de la lettre de relance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextesRelance {
    pub titre: String,
    pub objet: String,
    /// Texte avant le tableau récapitulatif ; paragraphes séparés par une ligne vide.
    pub avant: String,
    /// Texte après le tableau récapitulatif ; paragraphes séparés par une ligne vide.
    pub apres: String,
    /// Libellé du signataire ; vide pour aucun.
    pub signataire: String,
}

impl Default for TextesRelance {
    /// Texte validé par l'utilisateur le 2026-11-10.
    fn default() -> Self {
        TextesRelance {
            titre: "LETTRE DE RELANCE".into(),
            objet: "Objet : facture n° {numero} impayée".into(),
            avant: "Madame, Monsieur,\n\nSauf erreur ou omission de notre part, la facture \
                    n° {numero} du {date}, arrivée à échéance le {echeance}, reste impayée \
                    à ce jour pour un montant de {reste} FCFA, soit {retard} jours de retard."
                .into(),
            apres: "Nous vous remercions de bien vouloir procéder à son règlement dans les \
                    meilleurs délais. Si votre paiement a été effectué entre-temps, nous vous \
                    prions de ne pas tenir compte de la présente.\n\nNous vous prions \
                    d'agréer, Madame, Monsieur, l'expression de nos salutations distinguées."
                .into(),
            signataire: "LA COMPTABILITE".into(),
        }
    }
}

fn en_json(t: &TextesRelance) -> Value {
    json!({
        "titre": t.titre.trim(),
        "objet": t.objet.trim(),
        "avant": t.avant.trim(),
        "apres": t.apres.trim(),
        "signataire": t.signataire.trim(),
    })
}

/// Textes enregistrés ; absents ou illisibles, les textes par défaut.
pub fn lire_textes(conn: &Connection) -> Resultat<TextesRelance> {
    let texte: Option<String> = conn
        .query_row(
            "SELECT valeur FROM parametre_systeme WHERE cle = ?1",
            [CLE_TEXTES_RELANCE],
            |r| r.get(0),
        )
        .optional()?;
    let Some(v) = texte.and_then(|t| serde_json::from_str::<Value>(&t).ok()) else {
        return Ok(TextesRelance::default());
    };
    let champ = |nom: &str| v.get(nom).and_then(Value::as_str).map(String::from);
    Ok(
        match (
            champ("titre"),
            champ("objet"),
            champ("avant"),
            champ("apres"),
            champ("signataire"),
        ) {
            (Some(titre), Some(objet), Some(avant), Some(apres), Some(signataire)) => {
                TextesRelance {
                    titre,
                    objet,
                    avant,
                    apres,
                    signataire,
                }
            }
            _ => TextesRelance::default(),
        },
    )
}

/// Réservé au droit « Gérer la fiche entreprise et les paramètres ».
pub fn enregistrer_textes(
    conn: &Connection,
    acteur: &Utilisateur,
    textes: &TextesRelance,
) -> Resultat<()> {
    exiger(conn, acteur, Droit::GererParametres)?;
    if textes.titre.trim().is_empty() {
        return Err(Erreur::Validation(
            "le titre de la lettre de relance est obligatoire".into(),
        ));
    }
    if textes.avant.trim().is_empty() && textes.apres.trim().is_empty() {
        return Err(Erreur::Validation(
            "le texte de la lettre de relance est obligatoire".into(),
        ));
    }
    let valeur = en_json(textes);
    conn.execute(
        "INSERT INTO parametre_systeme (cle, valeur) VALUES (?1, ?2)
         ON CONFLICT (cle) DO UPDATE SET valeur = excluded.valeur",
        params![CLE_TEXTES_RELANCE, valeur.to_string()],
    )?;
    audit::journaliser(
        conn,
        Entree {
            entite: "parametres",
            entite_id: 0,
            action: "modification_relance",
            ancienne_valeur: None,
            nouvelle_valeur: Some(valeur),
            utilisateur_id: Some(acteur.id),
        },
    )
}

/// Tableau récapitulatif de la facture relancée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RappelFacture {
    pub numero: String,
    pub date: NaiveDate,
    pub echeance: NaiveDate,
    pub montant_du: Montant,
    pub regle: Montant,
    pub reste_a_payer: Montant,
    pub jours_de_retard: i64,
}

/// Tout ce qui s'imprime sur la lettre, textes déjà remplis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LettreRelance {
    pub entreprise: Entreprise,
    pub logo: Option<Vec<u8>>,
    pub logo2: Option<Vec<u8>>,
    pub couleur: [u8; 3],
    pub date: NaiveDate,
    pub client: SnapshotTiers,
    pub titre: String,
    pub objet: String,
    pub avant: String,
    pub apres: String,
    pub signataire: String,
    /// Responsable comptabilité de la fiche entreprise, sous le signataire.
    pub nom_signataire: String,
    pub facture: RappelFacture,
}

fn date_lettre(d: NaiveDate) -> String {
    d.format("%d/%m/%Y").to_string()
}

/// Remplace les champs `{numero}`, `{date}`, `{echeance}`, `{reste}`, `{retard}`.
fn remplir(modele: &str, f: &RappelFacture) -> String {
    modele
        .trim()
        .replace("{numero}", &f.numero)
        .replace("{date}", &date_lettre(f.date))
        .replace("{echeance}", &date_lettre(f.echeance))
        .replace("{reste}", &montant_en_texte(f.reste_a_payer))
        .replace("{retard}", &f.jours_de_retard.to_string())
}

/// Lettre de relance d'une facture échue au jour donné ; refusée pour une
/// facture qui n'est pas échue ou ne reste pas à payer.
pub fn lettre(
    conn: &Connection,
    facture_id: i64,
    aujourd_hui: NaiveDate,
) -> Resultat<LettreRelance> {
    let du = reglements::factures_echues(conn, aujourd_hui)?
        .into_iter()
        .find(|d| d.id == facture_id)
        .ok_or_else(|| {
            Erreur::Validation(
                "cette facture n'est pas échue ou ne reste pas à payer : pas de relance".into(),
            )
        })?;
    let d = documents::lire(conn, facture_id)?
        .ok_or_else(|| Erreur::Validation("ce document n'existe plus".into()))?;
    let facture = RappelFacture {
        numero: du.numero,
        date: du.date,
        echeance: du.echeance.expect("facture échue : échéance présente"),
        montant_du: du.montant_du,
        regle: du.regle,
        reste_a_payer: du.reste_a_payer,
        jours_de_retard: du.jours_de_retard.unwrap_or_default(),
    };
    let textes = lire_textes(conn)?;
    let entreprise = entreprise::lire(conn)?.unwrap_or_default();
    let signataire = textes.signataire.trim().to_owned();
    Ok(LettreRelance {
        nom_signataire: if signataire.is_empty() {
            String::new()
        } else {
            entreprise.responsable_comptabilite.clone()
        },
        logo: entreprise::lire_logo(conn, Logo::Principal)?,
        logo2: entreprise::lire_logo(conn, Logo::Secondaire)?,
        couleur: parametres::lire(conn)?.couleur_documents,
        date: aujourd_hui,
        client: d.tiers,
        titre: textes.titre.trim().to_owned(),
        objet: remplir(&textes.objet, &facture),
        avant: remplir(&textes.avant, &facture),
        apres: remplir(&textes.apres, &facture),
        signataire,
        entreprise,
        facture,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::documents::{LigneSaisie, Saisie, TypeDocument};
    use crate::droits::tests::compte;
    use crate::ouvrir_en_memoire;
    use crate::reglements::{SaisieReglement, ajouter};
    use crate::tiers::{self, FicheTiers, TypeTiers};
    use kadenz_domaine::calcul::{Taux, UNE_UNITE};
    use kadenz_domaine::role::Role;

    fn date(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    #[test]
    fn lettre_d_une_facture_echue() {
        let mut conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        let mut fiche = FicheTiers::vide(TypeTiers::Client);
        fiche.nom = "Client Alpha".into();
        fiche.adresse = "01 BP 1".into();
        let client = tiers::creer(&conn, &admin, &fiche).unwrap().id;
        let s = Saisie {
            tiers_id: Some(client),
            taux: Taux {
                remise: 0,
                tva: 0,
                retenue: 0,
            },
            date_echeance: Some(date(2026, 6, 1)),
            lignes: vec![LigneSaisie {
                designation: "Pelle".into(),
                prix: 100_000,
                quantite: UNE_UNITE,
                ..Default::default()
            }],
            ..Saisie::vide(date(2026, 5, 3))
        };
        let f = documents::creer_brouillon(&mut conn, &admin, TypeDocument::Facture, &s).unwrap();
        let numero = documents::valider(&mut conn, &admin, f).unwrap();
        let reglement = SaisieReglement {
            date: date(2026, 5, 10),
            montant: 40_000,
            mode_paiement: "Espèce".into(),
            reference: String::new(),
        };
        ajouter(&mut conn, &admin, f, &reglement).unwrap();
        assert!(
            lettre(&conn, f, date(2026, 6, 1)).is_err(),
            "pas encore échue"
        );
        let l = lettre(&conn, f, date(2026, 6, 11)).unwrap();
        assert_eq!(l.titre, "LETTRE DE RELANCE");
        assert_eq!(l.objet, format!("Objet : facture n° {numero} impayée"));
        assert_eq!(
            l.avant,
            format!(
                "Madame, Monsieur,\n\nSauf erreur ou omission de notre part, la facture \
                 n° {numero} du 03/05/2026, arrivée à échéance le 01/06/2026, reste impayée \
                 à ce jour pour un montant de 60\u{a0}000 FCFA, soit 10 jours de retard."
            )
        );
        assert_eq!(l.client.nom, "Client Alpha");
        assert_eq!(
            (
                l.facture.montant_du,
                l.facture.regle,
                l.facture.reste_a_payer
            ),
            (100_000, 40_000, 60_000)
        );
        assert_eq!(l.signataire, "LA COMPTABILITE");
    }

    #[test]
    fn textes_regles_et_droit() {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        let compta = compte(&conn, "compta", Role::Comptable);
        assert_eq!(lire_textes(&conn).unwrap(), TextesRelance::default());
        let mut t = TextesRelance {
            titre: " RAPPEL ".into(),
            ..Default::default()
        };
        assert!(enregistrer_textes(&conn, &compta, &t).is_err());
        enregistrer_textes(&conn, &admin, &t).unwrap();
        assert_eq!(lire_textes(&conn).unwrap().titre, "RAPPEL");
        t.titre = " ".into();
        assert!(enregistrer_textes(&conn, &admin, &t).is_err());
        let vide = TextesRelance {
            avant: String::new(),
            apres: " ".into(),
            ..TextesRelance::default()
        };
        assert!(enregistrer_textes(&conn, &admin, &vide).is_err());
    }
}
