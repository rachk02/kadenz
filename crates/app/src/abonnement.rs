//! Abonnement de l'entreprise sur un poste relié (phase 2, jalon F3 ; écrans,
//! section 7) : rubrique « Abonnement » des Paramètres, bandeau d'avertissement
//! avant l'échéance et bandeau permanent de la lecture seule.
//!
//! Le serveur seul juge de l'état (sa date ne recule jamais) ; le poste affiche
//! le dernier état reçu, gardé dans sa copie locale pour être lu hors connexion.
//! Un poste sous licence V1 perpétuelle n'a pas d'état reçu : il garde la
//! rubrique « Licence ».

use chrono::{NaiveDate, NaiveDateTime};
use kadenz_domaine::module::Module;
use kadenz_protocole::abonnement::{Abonnement, Emetteur, Formule, Statut};
use slint::{ComponentHandle, ModelRc, VecModel};

use crate::factures::date_en_texte;
use crate::traduction::{tr, trf};
use crate::{AbonnementApi, Ecriture, FenetrePrincipale, LigneAbonnementUi};

/// Ce que montrent la rubrique et le bandeau, textes déjà traduits.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct VueAbonnement {
    pub lecture_seule: bool,
    /// Lignes de la rubrique : libellé, valeur.
    pub lignes: Vec<(String, String)>,
    /// Texte du bandeau ; vide, pas de bandeau pour ce compte.
    pub bandeau: String,
    /// Bandeau de la lecture seule (sinon, avertissement avant l'échéance).
    pub bandeau_grave: bool,
    /// Suite du bandeau : chemin du renouvellement, ou à qui s'adresser.
    pub bandeau_aide: String,
}

/// `AAAA-MM-JJ` en JJ/MM/AAAA ; un texte illisible est rendu tel quel.
fn date(texte: &str) -> String {
    NaiveDate::parse_from_str(texte, "%Y-%m-%d")
        .map(date_en_texte)
        .unwrap_or_else(|_| texte.to_string())
}

/// `AAAA-MM-JJTHH:MM:SS` en JJ/MM/AAAA HH:MM ; un texte illisible est rendu tel quel.
fn date_heure(texte: &str) -> String {
    NaiveDateTime::parse_from_str(texte, "%Y-%m-%dT%H:%M:%S")
        .map(|d| format!("{} {}", date_en_texte(d.date()), d.format("%H:%M")))
        .unwrap_or_else(|_| texte.to_string())
}

const SUITE_LECTURE_SEULE: &str =
    "Consultation, recherche, impression et exports restent possibles.";

/// Raison du blocage des écritures (décisions 40 et 46), ou `None` si la
/// saisie est permise. Un poste V1, qui n'a jamais reçu d'abonnement, n'est
/// jamais bloqué. Le serveur injoignable s'y ajoutera avec le synchroniseur.
fn raison_blocage(a: Option<&Abonnement>) -> Option<String> {
    a.filter(|a| a.lecture_seule())
        .map(|_| tr("Kadenz est en lecture seule : aucune saisie n'est possible avant le renouvellement de l'abonnement."))
}

/// Raison du blocage des écritures sur ce poste, ou `None` si la saisie est
/// permise : seul point de décision pour les écrans.
pub(crate) fn blocage(conn: &kadenz_stockage::Connexion) -> Option<String> {
    raison_blocage(source(conn).as_ref().map(|(a, _)| a))
}

/// Jours restants à afficher. L'état gardé vieillit hors connexion : ses jours
/// restants sont ceux du jour de réception. Pour l'affichage seulement, ils
/// sont recalculés depuis l'échéance avec la date du poste, jamais avant la
/// date du serveur (horloge reculée) ni en dessous de zéro ; la lecture seule
/// et le palier restent ceux décidés par le serveur.
fn jours_affiches(a: &Abonnement, poste: NaiveDate) -> Option<i64> {
    let echeance = NaiveDate::parse_from_str(&a.licence.as_ref()?.echeance, "%Y-%m-%d").ok();
    let serveur = NaiveDate::parse_from_str(&a.aujourdhui, "%Y-%m-%d").ok();
    match (echeance, serveur) {
        (Some(e), Some(s)) => Some((e - poste.max(s)).num_days().max(0)),
        _ => a.jours_restants,
    }
}

/// Vue de l'abonnement pour le compte connecté. `gere` : le compte gère les
/// paramètres (l'Admin de la décision 26), lui seul voit l'avertissement à 30
/// jours et peut renouveler. `recu_le` : date de réception de l'état ;
/// `poste` : date du jour sur ce poste, pour l'affichage seulement.
pub(crate) fn vue(a: &Abonnement, recu_le: &str, gere: bool, poste: NaiveDate) -> VueAbonnement {
    let mut v = VueAbonnement {
        lecture_seule: a.lecture_seule(),
        ..VueAbonnement::default()
    };
    let echeance = a.licence.as_ref().map(|l| date(&l.echeance));

    let etat = match a.statut {
        Statut::Actif => tr("Actif"),
        Statut::Echu => tr("Échu : lecture seule"),
        Statut::Absent => tr("Aucun abonnement : lecture seule"),
        Statut::Invalide => tr("Licence refusée : lecture seule"),
    };
    v.lignes.push((tr("État"), etat));
    if let Some(refus) = &a.erreur {
        let valeurs: Vec<&dyn std::fmt::Display> = refus
            .valeurs
            .iter()
            .map(|s| s as &dyn std::fmt::Display)
            .collect();
        v.lignes.push((tr("Raison"), trf(&refus.modele, &valeurs)));
    }
    if let Some(l) = &a.licence {
        v.lignes.push((tr("Client"), l.client.clone()));
        v.lignes.push((
            tr("Formule"),
            match l.formule {
                Formule::Mensuelle => tr("Mensuelle"),
                Formule::Annuelle => tr("Annuelle"),
            },
        ));
        v.lignes.push((
            tr("Période"),
            trf("du {} au {}", &[&date(&l.debut), &date(&l.echeance)]),
        ));
        if let (Statut::Actif, Some(jours)) = (a.statut, jours_affiches(a, poste)) {
            v.lignes.push((tr("Jours restants"), jours.to_string()));
        }
        v.lignes.push((
            tr("Postes"),
            // Accord en français : singulier pour 0 et 1.
            if a.postes_utilises <= 1 {
                trf("{} utilisé sur {}", &[&a.postes_utilises, &l.postes])
            } else {
                trf("{} utilisés sur {}", &[&a.postes_utilises, &l.postes])
            },
        ));
        let modules: Vec<String> = l
            .modules
            .iter()
            .filter_map(|c| Module::depuis_code(c))
            .map(|m| tr(m.libelle()))
            .collect();
        v.lignes.push((
            tr("Modules"),
            if modules.is_empty() {
                tr("socle seul")
            } else {
                modules.join(", ")
            },
        ));
        v.lignes.push((
            tr("Émetteur"),
            match &l.emetteur {
                Emetteur::Editeur => tr("Éditeur"),
                Emetteur::Revendeur { nom, .. } => trf("Revendeur : {}", &[nom]),
            },
        ));
        v.lignes.push((tr("Émise le"), date(&l.emise_le)));
    } else {
        v.lignes.push((
            tr("Postes"),
            if a.postes_utilises <= 1 {
                trf("{} inscrit", &[&a.postes_utilises])
            } else {
                trf("{} inscrits", &[&a.postes_utilises])
            },
        ));
    }
    v.lignes.push((tr("Date du serveur"), date(&a.aujourdhui)));
    v.lignes.push((tr("État reçu le"), date_heure(recu_le)));

    // Bandeau : permanent en lecture seule, pour tous ; avant l'échéance, selon
    // le palier (l'Admin seul à 30 jours, tous les comptes ensuite).
    v.bandeau = match (a.statut, &a.avertissement) {
        (Statut::Echu, _) => format!(
            "{} {}",
            trf(
                "Abonnement échu le {} : Kadenz est en lecture seule.",
                &[&echeance.clone().unwrap_or_default()],
            ),
            tr(SUITE_LECTURE_SEULE)
        ),
        (Statut::Absent, _) => format!(
            "{} {}",
            tr("Aucun abonnement enregistré sur le serveur : Kadenz est en lecture seule."),
            tr(SUITE_LECTURE_SEULE)
        ),
        (Statut::Invalide, _) => format!(
            "{} {}",
            tr("Licence d'abonnement refusée par le serveur : Kadenz est en lecture seule."),
            tr(SUITE_LECTURE_SEULE)
        ),
        (Statut::Actif, Some(av)) if av.tous_les_comptes || gere => {
            let echeance = echeance.clone().unwrap_or_default();
            match jours_affiches(a, poste) {
                Some(0) => trf(
                    "L'abonnement prend fin aujourd'hui, le {} : Kadenz passera en lecture seule dès demain.",
                    &[&echeance],
                ),
                Some(1) => trf(
                    "L'abonnement prend fin demain, le {} : Kadenz passera ensuite en lecture seule.",
                    &[&echeance],
                ),
                jours => trf(
                    "L'abonnement prend fin le {} (dans {} jours) : Kadenz passera ensuite en lecture seule.",
                    &[&echeance, &jours.unwrap_or_default()],
                ),
            }
        }
        (Statut::Actif, _) => String::new(),
    };
    v.bandeau_grave = v.lecture_seule;
    if !v.bandeau.is_empty() {
        v.bandeau_aide = if gere {
            tr("Pour renouveler : Paramètres, rubrique « Abonnement ».")
        } else {
            tr("Adressez-vous à l'administrateur pour renouveler l'abonnement.")
        };
    }
    v
}

/// Dernier état reçu du serveur. En développement, `KADENZ_ABONNEMENT_JSON`
/// (chemin d'un fichier JSON d'un `Abonnement`) le remplace, pour essayer les
/// écrans sans serveur ; une version publiée ne lit jamais cette variable.
fn source(conn: &kadenz_stockage::Connexion) -> Option<(Abonnement, String)> {
    #[cfg(debug_assertions)]
    if let Some(chemin) = std::env::var_os("KADENZ_ABONNEMENT_JSON") {
        let recu_le = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        return std::fs::read_to_string(chemin)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .map(|a| (a, recu_le));
    }
    kadenz_stockage::abonnement::dernier_recu(conn)
        .ok()
        .flatten()
}

/// Recopie l'abonnement dans les écrans ; vrai si le poste en a un (poste
/// relié), auquel cas la rubrique « Abonnement » remplace « Licence ». Sans
/// compte connecté, rien n'est montré.
pub(crate) fn afficher(
    f: &FenetrePrincipale,
    conn: &kadenz_stockage::Connexion,
    connecte: bool,
    gere: bool,
) -> bool {
    let api = f.global::<AbonnementApi>();
    let recu = if connecte { source(conn) } else { None };
    let v = recu
        .as_ref()
        .map(|(a, recu_le)| vue(a, recu_le, gere, chrono::Local::now().date_naive()))
        .unwrap_or_default();
    api.set_disponible(recu.is_some());
    api.set_lecture_seule(v.lecture_seule);
    f.global::<Ecriture>()
        .set_permise(raison_blocage(recu.as_ref().map(|(a, _)| a)).is_none());
    api.set_lignes(ModelRc::new(VecModel::from(
        v.lignes
            .into_iter()
            .map(|(libelle, valeur)| LigneAbonnementUi {
                libelle: libelle.into(),
                valeur: valeur.into(),
            })
            .collect::<Vec<_>>(),
    )));
    api.set_bandeau(v.bandeau.into());
    api.set_bandeau_grave(v.bandeau_grave);
    api.set_bandeau_aide(v.bandeau_aide.into());
    api.set_peut_ouvrir(gere);
    recu.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kadenz_protocole::abonnement::{Avertissement, LicenceAbonnement, Refus};

    fn licence(echeance: &str) -> LicenceAbonnement {
        LicenceAbonnement {
            entreprise: "e".into(),
            client: "Société d'essai".into(),
            formule: Formule::Annuelle,
            postes: 5,
            modules: vec!["devis".into(), "inconnu".into()],
            debut: "2026-01-01".into(),
            echeance: echeance.into(),
            emetteur: Emetteur::Revendeur {
                identifiant: "r1".into(),
                nom: "Revendeur d'essai".into(),
            },
            emise_le: "2025-12-20".into(),
        }
    }

    fn jour() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 12).unwrap()
    }

    fn actif(jours: i64) -> Abonnement {
        let echeance = (jour() + chrono::Days::new(jours.max(0) as u64))
            .format("%Y-%m-%d")
            .to_string();
        Abonnement {
            statut: Statut::Actif,
            aujourdhui: "2026-11-12".into(),
            jours_restants: Some(jours),
            avertissement: Avertissement::pour(jours),
            postes_utilises: 3,
            licence: Some(licence(&echeance)),
            erreur: None,
        }
    }

    fn valeur<'a>(v: &'a VueAbonnement, libelle: &str) -> Option<&'a str> {
        v.lignes
            .iter()
            .find(|(l, _)| l == libelle)
            .map(|(_, val)| val.as_str())
    }

    const RECU: &str = "2026-11-12T08:30:00";

    #[test]
    fn actif_sans_avertissement() {
        let v = vue(&actif(49), RECU, true, jour());
        assert!(!v.lecture_seule);
        assert!(v.bandeau.is_empty() && v.bandeau_aide.is_empty());
        assert_eq!(valeur(&v, "État"), Some("Actif"));
        assert_eq!(valeur(&v, "Formule"), Some("Annuelle"));
        assert_eq!(valeur(&v, "Période"), Some("du 01/01/2026 au 31/12/2026"));
        assert_eq!(valeur(&v, "Jours restants"), Some("49"));
        assert_eq!(valeur(&v, "Postes"), Some("3 utilisés sur 5"));
        assert_eq!(
            valeur(&v, "Émetteur"),
            Some("Revendeur : Revendeur d'essai")
        );
        assert_eq!(valeur(&v, "État reçu le"), Some("12/11/2026 08:30"));
        // Un code de module inconnu de cette version est ignoré.
        assert_eq!(
            valeur(&v, "Modules"),
            Some(tr(Module::Devis.libelle()).as_str())
        );
    }

    #[test]
    fn trente_jours_pour_l_admin_seul() {
        let admin = vue(&actif(20), RECU, true, jour());
        assert!(admin.bandeau.contains("02/12/2026") && admin.bandeau.contains("dans 20 jours"));
        assert!(!admin.bandeau_grave);
        assert!(admin.bandeau_aide.contains("Abonnement"));
        let autre = vue(&actif(20), RECU, false, jour());
        assert!(autre.bandeau.is_empty() && autre.bandeau_aide.is_empty());
    }

    #[test]
    fn derniers_jours_pour_tous() {
        let sept = vue(&actif(5), RECU, false, jour());
        assert!(sept.bandeau.contains("dans 5 jours"));
        assert!(sept.bandeau_aide.contains("administrateur"));
        assert!(
            vue(&actif(1), RECU, false, jour())
                .bandeau
                .contains("demain")
        );
        assert!(
            vue(&actif(0), RECU, false, jour())
                .bandeau
                .contains("aujourd'hui")
        );
    }

    #[test]
    fn jours_recalcules_hors_connexion() {
        // État reçu à 20 jours de l'échéance, relu dix jours plus tard.
        let a = actif(20);
        let plus_tard = jour() + chrono::Days::new(10);
        // Palier décidé par le serveur à la réception (30 jours) : Admin seul.
        let v = vue(&a, RECU, true, plus_tard);
        assert_eq!(valeur(&v, "Jours restants"), Some("10"));
        assert!(v.bandeau.contains("dans 10 jours"));
        // Horloge du poste reculée : jamais avant la date du serveur.
        let avant = jour() - chrono::Days::new(30);
        assert_eq!(
            valeur(&vue(&a, RECU, true, avant), "Jours restants"),
            Some("20")
        );
        // Échéance dépassée hors connexion : la lecture seule reste celle du serveur.
        let apres = jour() + chrono::Days::new(25);
        let v = vue(&a, RECU, true, apres);
        assert!(!v.lecture_seule);
        assert_eq!(valeur(&v, "Jours restants"), Some("0"));
    }

    #[test]
    fn echu_en_lecture_seule() {
        let mut a = actif(0);
        a.licence = Some(licence("2026-11-11"));
        a.jours_restants = Some(-1);
        a.statut = Statut::Echu;
        a.avertissement = None;
        let v = vue(&a, RECU, false, jour());
        assert!(v.lecture_seule && v.bandeau_grave);
        assert!(v.bandeau.starts_with("Abonnement échu le 11/11/2026"));
        assert!(
            v.bandeau
                .contains("impression et exports restent possibles")
        );
        assert_eq!(valeur(&v, "Jours restants"), None);
    }

    #[test]
    fn absent_et_refusee_en_lecture_seule() {
        let mut a = actif(0);
        a.statut = Statut::Absent;
        a.jours_restants = None;
        a.avertissement = None;
        a.licence = None;
        let v = vue(&a, RECU, true, jour());
        assert!(v.lecture_seule && v.bandeau_grave);
        assert_eq!(valeur(&v, "Postes"), Some("3 inscrits"));
        assert_eq!(valeur(&v, "Client"), None);

        a.statut = Statut::Invalide;
        a.erreur = Some(Refus {
            modele: "licence d'abonnement émise pour une autre entreprise ({})".into(),
            valeurs: vec!["X".into()],
        });
        let v = vue(&a, RECU, true, jour());
        assert!(v.lecture_seule);
        assert_eq!(
            valeur(&v, "Raison"),
            Some("licence d'abonnement émise pour une autre entreprise (X)")
        );
        assert!(v.bandeau.contains("refusée"));
    }

    #[test]
    fn saisie_bloquee_en_lecture_seule_seulement() {
        // Poste V1 : aucun abonnement reçu, jamais bloqué.
        assert_eq!(raison_blocage(None), None);
        // Abonnement actif, même la veille de l'échéance : saisie permise.
        assert_eq!(raison_blocage(Some(&actif(1))), None);
        // Échu, absent ou refusé : bloqué, avec la raison.
        for statut in [Statut::Echu, Statut::Absent, Statut::Invalide] {
            let mut a = actif(0);
            a.statut = statut;
            assert!(a.lecture_seule(), "{statut:?}");
            let raison = raison_blocage(Some(&a)).unwrap();
            assert!(raison.contains("lecture seule"), "{raison}");
        }
    }

    #[test]
    fn textes_traduits() {
        let mut textes = vec![SUITE_LECTURE_SEULE];
        textes.extend_from_slice(kadenz_licence::entreprise::MODELES_ERREUR);
        textes.extend_from_slice(kadenz_stockage::inscription::MODELES_ERREUR);
        textes.push("le nom du poste est obligatoire");
        let manquants = crate::traduction::sans_traduction(&textes);
        assert!(manquants.is_empty(), "sans traduction : {manquants:?}");
    }
}
