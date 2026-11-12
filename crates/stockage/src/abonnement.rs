//! Abonnement de l'entreprise (phase 2, jalon F3 ; décisions 6, 20, 21, 25,
//! 26 et 29 ; spécification de synchronisation, section 7).
//!
//! Côté serveur : la licence d'abonnement est gardée dans les paramètres
//! système et vérifiée avec les clés du portail. La date retenue ne recule
//! jamais : c'est la plus récente de l'horloge du serveur, de la dernière date
//! vue et de la date d'émission de la licence. Reculer l'horloge est donc sans
//! effet ; une horloge avancée par erreur se rattrape en installant une
//! nouvelle licence. L'état (lecture seule, modules) est tenu dans des tables
//! temporaires de la connexion, comme les modules de la V1.
//!
//! Côté poste : le dernier état reçu du serveur est gardé dans la copie
//! locale, pour l'afficher hors connexion.

use chrono::NaiveDate;
use kadenz_domaine::message::MessageCompose;
use kadenz_domaine::module::Module;
use kadenz_domaine::role::Droit;
use kadenz_licence::VerifyingKey;
use kadenz_licence::entreprise::{self as lic, LicenceEntreprise};
use kadenz_protocole::abonnement::{
    self as proto, Abonnement, Avertissement, LicenceAbonnement, Refus, Statut,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::json;

use crate::audit::{self, Entree};
use crate::comptes::Utilisateur;
use crate::droits::exiger as exiger_droit;
use crate::{Erreur, Resultat, licence, maintenant, postes, synchro};

/// Message des opérations refusées pendant la lecture seule.
pub const MODELE_LECTURE_SEULE: &str = "abonnement échu : Kadenz est en lecture seule";

const CLE: &str = "abonnement.cle";
/// Dernière date retenue (ne recule jamais).
const DATE_VUE: &str = "abonnement.date_vue";
/// Date d'émission de la licence la plus récente installée.
const EMISSION_VUE: &str = "abonnement.emission_vue";
/// Dernier état reçu du serveur, sur un poste.
const RECU: &str = "abonnement.recu";

/// Table temporaire de l'état de la connexion ; vide, rien n'est bloqué (base
/// d'un poste, tests).
pub(crate) fn preparer(conn: &Connection) -> Resultat<()> {
    conn.execute(
        "CREATE TEMP TABLE IF NOT EXISTS abonnement_etat (
             id INTEGER PRIMARY KEY CHECK (id = 1),
             lecture_seule INTEGER NOT NULL,
             echeance TEXT
         )",
        [],
    )?;
    Ok(())
}

fn lire(conn: &Connection, cle: &str) -> Resultat<Option<String>> {
    let valeur: Option<String> = conn
        .query_row(
            "SELECT valeur FROM parametre_systeme WHERE cle = ?1",
            [cle],
            |r| r.get(0),
        )
        .optional()?;
    Ok(valeur.and_then(|v| match serde_json::from_str(&v) {
        Ok(serde_json::Value::String(s)) => Some(s),
        _ => None,
    }))
}

fn ecrire(conn: &Connection, cle: &str, valeur: &str) -> Resultat<()> {
    conn.execute(
        "INSERT INTO parametre_systeme (cle, valeur) VALUES (?1, ?2)
         ON CONFLICT (cle) DO UPDATE SET valeur = excluded.valeur",
        params![cle, serde_json::Value::String(valeur.into()).to_string()],
    )?;
    Ok(())
}

fn date(texte: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(texte, "%Y-%m-%d").ok()
}

fn en_texte(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

/// Date du jour de l'horloge locale de la machine (fuseau de l'entreprise,
/// décision 37).
pub fn horloge() -> NaiveDate {
    chrono::Local::now().date_naive()
}

/// Vérifie une licence pour l'entreprise de cette base, dates du calendrier
/// comprises.
fn verifier(
    conn: &Connection,
    cle: &str,
    cles_portail: &[VerifyingKey],
) -> Resultat<Result<LicenceEntreprise, MessageCompose>> {
    let entreprise = synchro::identifiant_entreprise(conn)?;
    Ok(lic::verifier(cle, &entreprise, cles_portail).and_then(|l| {
        if [&l.debut, &l.echeance, &l.emise_le]
            .iter()
            .all(|d| date(d).is_some())
        {
            Ok(l)
        } else {
            Err(MessageCompose::new("clé de licence illisible", &[]))
        }
    }))
}

/// Date retenue, gardée : jamais avant la dernière date vue ni avant
/// l'émission de la licence en place.
fn date_retenue(
    conn: &Connection,
    horloge: NaiveDate,
    emission: Option<NaiveDate>,
) -> Resultat<NaiveDate> {
    let vue = lire(conn, DATE_VUE)?.as_deref().and_then(date);
    let retenue = [Some(horloge), vue, emission]
        .into_iter()
        .flatten()
        .max()
        .unwrap_or(horloge);
    if vue != Some(retenue) {
        ecrire(conn, DATE_VUE, &en_texte(retenue))?;
    }
    Ok(retenue)
}

fn vers_proto(l: &LicenceEntreprise) -> LicenceAbonnement {
    LicenceAbonnement {
        entreprise: l.entreprise.clone(),
        client: l.client.clone(),
        formule: match l.formule {
            lic::Formule::Mensuelle => proto::Formule::Mensuelle,
            lic::Formule::Annuelle => proto::Formule::Annuelle,
        },
        postes: l.postes,
        modules: l.modules.iter().map(|m| m.code().to_string()).collect(),
        debut: l.debut.clone(),
        echeance: l.echeance.clone(),
        emetteur: match &l.emetteur {
            lic::Emetteur::Editeur => proto::Emetteur::Editeur,
            lic::Emetteur::Revendeur { identifiant, nom } => proto::Emetteur::Revendeur {
                identifiant: identifiant.clone(),
                nom: nom.clone(),
            },
        },
        emise_le: l.emise_le.clone(),
    }
}

/// État de l'abonnement et modules de la licence en place.
fn evaluer(
    conn: &Connection,
    horloge: NaiveDate,
    cles_portail: &[VerifyingKey],
) -> Resultat<(Abonnement, Vec<Module>)> {
    let postes_utilises = postes::nombre_inscrits(conn)?;
    let verifiee = match lire(conn, CLE)?.filter(|c| !c.trim().is_empty()) {
        None => None,
        Some(cle) => Some(verifier(conn, &cle, cles_portail)?),
    };
    let emission = match &verifiee {
        Some(Ok(l)) => date(&l.emise_le),
        _ => None,
    };
    let aujourdhui = date_retenue(conn, horloge, emission)?;
    let mut abonnement = Abonnement {
        statut: Statut::Absent,
        aujourdhui: en_texte(aujourdhui),
        jours_restants: None,
        avertissement: None,
        postes_utilises,
        licence: None,
        erreur: None,
    };
    let modules = match verifiee {
        None => Vec::new(),
        Some(Err(m)) => {
            abonnement.statut = Statut::Invalide;
            abonnement.erreur = Some(Refus {
                modele: m.modele.to_string(),
                valeurs: m.valeurs,
            });
            Vec::new()
        }
        Some(Ok(l)) => {
            // Seule l'échéance compte (décision 6) ; le début est indicatif.
            let echeance = date(&l.echeance).unwrap_or(aujourdhui);
            let jours = (echeance - aujourdhui).num_days();
            abonnement.statut = if jours < 0 {
                Statut::Echu
            } else {
                Statut::Actif
            };
            abonnement.jours_restants = Some(jours);
            abonnement.avertissement = Avertissement::pour(jours);
            abonnement.licence = Some(vers_proto(&l));
            l.modules
        }
    };
    Ok((abonnement, modules))
}

/// État de l'abonnement, avec les clés du portail données.
pub fn etat_avec(
    conn: &Connection,
    horloge: NaiveDate,
    cles_portail: &[VerifyingKey],
) -> Resultat<Abonnement> {
    Ok(evaluer(conn, horloge, cles_portail)?.0)
}

/// État de l'abonnement, avec les clés du portail embarquées.
pub fn etat(conn: &Connection, horloge: NaiveDate) -> Resultat<Abonnement> {
    etat_avec(conn, horloge, &lic::cles_portail())
}

/// Évalue l'abonnement et l'applique à la connexion : lecture seule et
/// modules actifs. Le serveur l'appelle avant chaque opération, pour suivre le
/// changement de jour.
pub fn appliquer_avec(
    conn: &Connection,
    horloge: NaiveDate,
    cles_portail: &[VerifyingKey],
) -> Resultat<Abonnement> {
    let (abonnement, modules) = evaluer(conn, horloge, cles_portail)?;
    conn.execute("DELETE FROM temp.abonnement_etat", [])?;
    conn.execute(
        "INSERT INTO temp.abonnement_etat (id, lecture_seule, echeance) VALUES (1, ?1, ?2)",
        params![
            abonnement.lecture_seule(),
            abonnement.licence.as_ref().map(|l| l.echeance.clone())
        ],
    )?;
    licence::remplir(conn, &modules)?;
    Ok(abonnement)
}

/// Refuse une saisie ou une validation pendant la lecture seule.
pub(crate) fn exiger_ecriture(conn: &Connection) -> Resultat<()> {
    let etat: Option<(bool, Option<String>)> = conn
        .query_row(
            "SELECT lecture_seule, echeance FROM temp.abonnement_etat WHERE id = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    match etat {
        Some((true, echeance)) => Err(Erreur::LectureSeule { echeance }),
        _ => Ok(()),
    }
}

/// Installe une nouvelle licence d'abonnement, au droit « Gérer les
/// paramètres ». Une licence refusée n'est pas enregistrée : celle en place
/// reste. Une licence plus récente que toutes les précédentes ramène la date
/// retenue à l'horloge (horloge avancée par erreur) ; une licence plus
/// ancienne ne le fait pas, pour qu'on ne puisse pas revenir en arrière en
/// réinstallant une ancienne licence.
pub fn enregistrer_avec(
    conn: &Connection,
    acteur: &Utilisateur,
    cle: &str,
    horloge: NaiveDate,
    cles_portail: &[VerifyingKey],
) -> Resultat<Abonnement> {
    exiger_droit(conn, acteur, Droit::GererParametres)?;
    let cle: String = cle.chars().filter(|c| !c.is_whitespace()).collect();
    let licence = verifier(conn, &cle, cles_portail)??;
    let emission = date(&licence.emise_le).unwrap_or(horloge);
    let derniere = lire(conn, EMISSION_VUE)?.as_deref().and_then(date);
    let en_place = lire(conn, CLE)?;
    if derniere.is_none_or(|d| emission > d || (emission == d && en_place.as_deref() != Some(&cle)))
    {
        ecrire(conn, DATE_VUE, &en_texte(horloge.max(emission)))?;
        ecrire(conn, EMISSION_VUE, &licence.emise_le)?;
    }
    ecrire(conn, CLE, &cle)?;
    audit::journaliser(
        conn,
        Entree {
            entite: "parametres",
            entite_id: 0,
            action: "modification_abonnement",
            ancienne_valeur: None,
            nouvelle_valeur: Some(json!({
                "client": licence.client,
                "formule": licence.formule.code(),
                "postes": licence.postes,
                "debut": licence.debut,
                "echeance": licence.echeance,
                "emise_le": licence.emise_le,
            })),
            utilisateur_id: Some(acteur.id),
        },
    )?;
    appliquer_avec(conn, horloge, cles_portail)
}

/// Comme [`enregistrer_avec`], avec les clés du portail embarquées.
pub fn enregistrer(
    conn: &Connection,
    acteur: &Utilisateur,
    cle: &str,
    horloge: NaiveDate,
) -> Resultat<Abonnement> {
    enregistrer_avec(conn, acteur, cle, horloge, &lic::cles_portail())
}

/// Sur un poste : garde le dernier état reçu du serveur.
pub fn garder(conn: &Connection, abonnement: &Abonnement) -> Resultat<()> {
    let valeur = json!({ "abonnement": abonnement, "recu_le": maintenant() });
    ecrire(conn, RECU, &valeur.to_string())
}

/// Sur un poste : dernier état reçu du serveur et sa date de réception.
pub fn dernier_recu(conn: &Connection) -> Resultat<Option<(Abonnement, String)>> {
    let Some(texte) = lire(conn, RECU)? else {
        return Ok(None);
    };
    let valeur: serde_json::Value = match serde_json::from_str(&texte) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    let abonnement = serde_json::from_value(valeur["abonnement"].clone()).ok();
    let recu_le = valeur["recu_le"].as_str().map(str::to_owned);
    Ok(abonnement.zip(recu_le))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::droits::tests::compte;
    use crate::ouvrir_en_memoire;
    use kadenz_domaine::role::Role;
    use kadenz_licence::SigningKey;
    use kadenz_protocole::abonnement::Palier;

    fn cle_portail() -> SigningKey {
        SigningKey::from_bytes(&[3u8; 32])
    }

    fn j(texte: &str) -> NaiveDate {
        date(texte).unwrap()
    }

    fn licence(conn: &Connection, echeance: &str, emise_le: &str) -> String {
        lic::emettre(
            &LicenceEntreprise {
                entreprise: synchro::identifiant_entreprise(conn).unwrap(),
                client: "Entreprise Essai".into(),
                formule: lic::Formule::Mensuelle,
                postes: 3,
                modules: vec![Module::Devis],
                debut: emise_le.into(),
                echeance: echeance.into(),
                emetteur: lic::Emetteur::Editeur,
                emise_le: emise_le.into(),
            },
            &cle_portail(),
        )
        .unwrap()
    }

    #[test]
    fn echeance_avertissements_et_lecture_seule() {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        let cles = [cle_portail().verifying_key()];
        let a = appliquer_avec(&conn, j("2026-11-12"), &cles).unwrap();
        assert_eq!(a.statut, Statut::Absent);
        assert!(exiger_ecriture(&conn).is_err());

        let cle = licence(&conn, "2026-12-31", "2026-11-12");
        let a = enregistrer_avec(&conn, &admin, &cle, j("2026-11-12"), &cles).unwrap();
        assert_eq!(a.statut, Statut::Actif);
        assert_eq!(a.jours_restants, Some(49));
        assert_eq!(a.avertissement, None);
        assert!(exiger_ecriture(&conn).is_ok());
        assert!(licence::module_actif(&conn, Module::Devis).unwrap());
        assert!(!licence::module_actif(&conn, Module::Paiements).unwrap());

        let palier = |jour: &str| {
            appliquer_avec(&conn, j(jour), &cles)
                .unwrap()
                .avertissement
                .map(|a| (a.palier, a.tous_les_comptes))
        };
        assert_eq!(palier("2026-12-01"), Some((Palier::Trente, false)));
        assert_eq!(palier("2026-12-24"), Some((Palier::Sept, true)));
        assert_eq!(palier("2026-12-30"), Some((Palier::Dernier, true)));
        assert_eq!(palier("2026-12-31"), Some((Palier::Dernier, true)));
        assert!(exiger_ecriture(&conn).is_ok());
        let a = appliquer_avec(&conn, j("2027-01-01"), &cles).unwrap();
        assert_eq!(a.statut, Statut::Echu);
        assert!(a.lecture_seule());
        assert!(matches!(
            exiger_ecriture(&conn),
            Err(Erreur::LectureSeule { echeance: Some(e) }) if e == "2026-12-31"
        ));
        // Modules gardés pour la consultation.
        assert!(licence::module_actif(&conn, Module::Devis).unwrap());
    }

    #[test]
    fn horloge_reculee_sans_effet_puis_rattrapage() {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        let cles = [cle_portail().verifying_key()];
        let ancienne = licence(&conn, "2026-12-31", "2026-11-12");
        enregistrer_avec(&conn, &admin, &ancienne, j("2026-11-12"), &cles).unwrap();
        assert_eq!(
            etat_avec(&conn, j("2027-01-05"), &cles).unwrap().statut,
            Statut::Echu
        );
        // Horloge reculée : la date retenue reste le 5 janvier.
        let a = etat_avec(&conn, j("2026-12-01"), &cles).unwrap();
        assert_eq!(a.statut, Statut::Echu);
        assert_eq!(a.aujourdhui, "2027-01-05");
        // Réinstaller l'ancienne licence ne fait pas revenir en arrière.
        let a = enregistrer_avec(&conn, &admin, &ancienne, j("2026-12-01"), &cles).unwrap();
        assert_eq!(a.statut, Statut::Echu);

        // Horloge avancée par erreur jusqu'en 2030...
        let nouvelle = licence(&conn, "2027-12-31", "2027-01-06");
        enregistrer_avec(&conn, &admin, &nouvelle, j("2027-01-06"), &cles).unwrap();
        assert_eq!(
            etat_avec(&conn, j("2030-01-01"), &cles).unwrap().statut,
            Statut::Echu
        );
        assert_eq!(
            etat_avec(&conn, j("2027-01-07"), &cles).unwrap().statut,
            Statut::Echu
        );
        // ... rattrapée par une licence plus récente, émise par l'éditeur.
        let rattrapage = licence(&conn, "2027-12-31", "2027-01-07");
        let a = enregistrer_avec(&conn, &admin, &rattrapage, j("2027-01-07"), &cles).unwrap();
        assert_eq!(a.statut, Statut::Actif);
        assert_eq!(a.aujourdhui, "2027-01-07");
    }

    #[test]
    fn licence_refusee_et_droit() {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        let lecteur = compte(&conn, "lecture", Role::Consultation);
        let cles = [cle_portail().verifying_key()];
        let bonne = licence(&conn, "2026-12-31", "2026-11-12");
        assert!(matches!(
            enregistrer_avec(&conn, &lecteur, &bonne, j("2026-11-12"), &cles),
            Err(Erreur::Interdit(_))
        ));
        // Autre entreprise : refusée, rien d'enregistré.
        let autre = lic::emettre(
            &LicenceEntreprise {
                entreprise: "autre".into(),
                ..lic::verifier(
                    &bonne,
                    &synchro::identifiant_entreprise(&conn).unwrap(),
                    &cles,
                )
                .unwrap()
            },
            &cle_portail(),
        )
        .unwrap();
        match enregistrer_avec(&conn, &admin, &autre, j("2026-11-12"), &cles) {
            Err(Erreur::ValidationComposee { modele, .. }) => assert_eq!(
                modele,
                "licence d'abonnement émise pour une autre entreprise ({})"
            ),
            autre => panic!("{autre:?}"),
        }
        assert_eq!(
            etat_avec(&conn, j("2026-11-12"), &cles).unwrap().statut,
            Statut::Absent
        );
        // Licence enregistrée puis clés du portail changées : invalide.
        enregistrer_avec(&conn, &admin, &bonne, j("2026-11-12"), &cles).unwrap();
        let autre_cle = [SigningKey::from_bytes(&[4u8; 32]).verifying_key()];
        let a = etat_avec(&conn, j("2026-11-12"), &autre_cle).unwrap();
        assert_eq!(a.statut, Statut::Invalide);
        assert_eq!(
            a.erreur.unwrap().modele,
            "clé de signature de la licence inconnue : {}"
        );
    }

    #[test]
    fn dernier_etat_garde_sur_le_poste() {
        let conn = ouvrir_en_memoire().unwrap();
        assert!(dernier_recu(&conn).unwrap().is_none());
        let a = etat_avec(&conn, j("2026-11-12"), &[]).unwrap();
        garder(&conn, &a).unwrap();
        let (lu, recu_le) = dernier_recu(&conn).unwrap().unwrap();
        assert_eq!(lu, a);
        assert!(!recu_le.is_empty());
    }
}
