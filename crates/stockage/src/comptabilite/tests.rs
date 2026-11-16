use super::*;
use crate::documents::{self, LigneSaisie, Saisie};
use crate::droits::tests::compte;
use crate::produits::{self, FicheProduit, Sens};
use crate::reglements::{self, SaisieReglement};
use crate::{ouvrir_en_memoire, tiers};
use kadenz_domaine::calcul::{Taux, UNE_UNITE};
use kadenz_domaine::referentiel::{FicheTiers, TypeTiers};
use kadenz_domaine::role::Role;

const U: i64 = UNE_UNITE;

struct Contexte {
    conn: Connection,
    admin: Utilisateur,
    client: i64,
    ciment: i64,
    pose: i64,
}

fn contexte() -> Contexte {
    let conn = ouvrir_en_memoire().unwrap();
    let admin = compte(&conn, "admin", Role::Admin);
    let mut f = FicheTiers::vide(TypeTiers::Client);
    f.nom = "Bâtiment Sahel".into();
    let client = tiers::creer(&conn, &admin, &f).unwrap().id;
    let produit = |reference: &str, prix, nature| {
        let f = FicheProduit {
            designation: format!("Produit {reference}"),
            reference: reference.into(),
            prix,
            sens: Sens::LesDeux,
            suivi_stock: false,
            nature_comptable: nature,
        };
        produits::creer(&conn, &admin, &f).unwrap().id
    };
    let ciment = produit("CIM50", 6_500, NatureComptable::Marchandise);
    let pose = produit("POSE", 15_000, NatureComptable::Service);
    Contexte {
        conn,
        admin,
        client,
        ciment,
        pose,
    }
}

fn ligne(produit_id: i64, prix: Montant, quantite: i64) -> LigneSaisie {
    LigneSaisie {
        produit_id: Some(produit_id),
        reference: format!("R{produit_id}"),
        designation: format!("Produit {produit_id}"),
        prix,
        quantite,
        ..Default::default()
    }
}

fn mois_precedent() -> Mois {
    let m = Mois::de_date(aujourd_hui());
    if m.mois == 1 {
        Mois {
            annee: m.annee - 1,
            mois: 12,
        }
    } else {
        Mois {
            annee: m.annee,
            mois: m.mois - 1,
        }
    }
}

/// Facture validée : remise 10 %, TVA 18 %.
fn facture(c: &mut Contexte, date: NaiveDate, lignes: Vec<LigneSaisie>) -> i64 {
    let s = Saisie {
        tiers_id: Some(c.client),
        taux: Taux {
            remise: 1_000,
            tva: 1_800,
            retenue: 0,
        },
        lignes,
        ..Saisie::vide(date)
    };
    let admin = c.admin.clone();
    let id = documents::creer_brouillon(&mut c.conn, &admin, TypeDocument::Facture, &s).unwrap();
    documents::valider(&mut c.conn, &admin, id).unwrap();
    id
}

/// Brouillon d'avoir de la facture, ramené à `lignes` et daté de `date`.
fn brouillon_avoir(
    c: &mut Contexte,
    facture_id: i64,
    date: NaiveDate,
    lignes: Vec<LigneSaisie>,
) -> i64 {
    let admin = c.admin.clone();
    let id = documents::creer_avoir(&mut c.conn, &admin, facture_id).unwrap();
    let mut s = documents::lire(&c.conn, id).unwrap().unwrap().saisie;
    s.date = date;
    s.lignes = lignes;
    documents::modifier_brouillon(&mut c.conn, &admin, id, &s).unwrap();
    id
}

fn regler(
    c: &mut Contexte,
    facture_id: i64,
    date: NaiveDate,
    montant: Montant,
    mode: &str,
) -> Resultat<i64> {
    let admin = c.admin.clone();
    reglements::ajouter(
        &mut c.conn,
        &admin,
        facture_id,
        &SaisieReglement {
            date,
            montant,
            mode_paiement: mode.into(),
            reference: String::new(),
        },
    )
}

fn resume(j: &JournalComptable) -> Vec<(Journal, &str, Montant, Montant)> {
    j.ecritures
        .iter()
        .map(|e| (e.journal, e.compte.as_str(), e.debit, e.credit))
        .collect()
}

fn message(e: Erreur) -> String {
    match e {
        Erreur::Validation(_) | Erreur::ValidationComposee { .. } => e.to_string(),
        autre => panic!("refus attendu : {autre:?}"),
    }
}

/// Mois précédent : facture mêlant marchandise et service, avoir du service,
/// deux règlements dont un par un mode sans compte.
fn mois_rempli(c: &mut Contexte) -> (Mois, i64, i64) {
    let m = mois_precedent();
    let jour = m.premier_jour();
    // CIM50 45 x 6 500 = 292 500 et POSE 15 000 : HT 307 500, remise 30 750,
    // base 276 750, TVA 49 815, TTC 326 565.
    let f = facture(
        c,
        jour,
        vec![ligne(c.ciment, 6_500, 45 * U), ligne(c.pose, 15_000, U)],
    );
    // Avoir de la pose : base 13 500, TVA 2 430, TTC 15 930.
    let a = brouillon_avoir(
        c,
        f,
        jour.succ_opt().unwrap(),
        vec![ligne(c.pose, 15_000, U)],
    );
    let admin = c.admin.clone();
    documents::valider(&mut c.conn, &admin, a).unwrap();
    let r = regler(c, f, m.dernier_jour(), 100_000, "Espèce").unwrap();
    regler(c, f, m.dernier_jour(), 50_000, "Wave").unwrap();
    (m, f, r)
}

#[test]
fn mois_codes_et_bornes() {
    let m = Mois::depuis_code("2028-02").unwrap();
    assert_eq!(m.code(), "2028-02");
    assert_eq!(m.to_string(), "02/2028");
    assert_eq!(
        m.dernier_jour(),
        NaiveDate::from_ymd_opt(2028, 2, 29).unwrap()
    );
    assert_eq!(
        Mois::depuis_code("2026-12").unwrap().suivant(),
        Mois {
            annee: 2027,
            mois: 1
        }
    );
    assert_eq!(Mois::depuis_code("2026-13"), None);
    assert_eq!(Mois::depuis_code("abc"), None);
}

#[test]
fn csv_et_nom_de_fichier() {
    let e = Ecriture {
        journal: Journal::Ventes,
        code_journal: "VT".into(),
        date: NaiveDate::from_ymd_opt(2026, 10, 5).unwrap(),
        piece: "F2026-10-0001".into(),
        compte: "4111".into(),
        compte_tiers: String::new(),
        libelle: "Facture; \"spéciale\"\nsuite".into(),
        debit: 326_565,
        credit: 0,
    };
    let texte = String::from_utf8(csv(&[e])).unwrap();
    assert_eq!(
        texte,
        "\u{feff}Journal;Date;Pièce;Compte;Compte tiers;Libellé;Débit;Crédit\r\n\
         VT;05/10/2026;F2026-10-0001;4111;;\"Facture; \"\"spéciale\"\"\nsuite\";326565;0\r\n"
    );
    let d = |a, m, j| NaiveDate::from_ymd_opt(a, m, j).unwrap();
    assert_eq!(
        nom_fichier(d(2026, 10, 1), d(2026, 10, 31), false),
        "journal-2026-10.csv"
    );
    assert_eq!(
        nom_fichier(d(2026, 10, 1), d(2026, 11, 15), true),
        "journal-2026-10-01_2026-11-15-provisoire.csv"
    );
}

#[test]
fn journal_ventile_et_equilibre() {
    let mut c = contexte();
    let (m, _, _) = mois_rempli(&mut c);
    let j = journal(
        &c.conn,
        Periode::du_mois(m),
        &[Journal::Ventes, Journal::Encaissements],
    )
    .unwrap();
    use Journal::*;
    assert_eq!(
        resume(&j),
        [
            (Ventes, "4111", 326_565, 0),
            (Ventes, "7011", 0, 263_250),
            (Ventes, "7061", 0, 13_500),
            (Ventes, "4431", 0, 47_385),
            (Ventes, "4432", 0, 2_430),
            (Ventes, "4111", 0, 15_930),
            (Ventes, "7061", 13_500, 0),
            (Ventes, "4432", 2_430, 0),
            (Encaissements, "5711", 100_000, 0),
            (Encaissements, "4111", 0, 100_000),
            (Encaissements, "", 50_000, 0),
            (Encaissements, "4111", 0, 50_000),
        ]
    );
    assert_eq!(j.total_debit, j.total_credit);
    assert_eq!(j.pieces, 4);
    assert_eq!(j.modes_sans_compte, ["Wave"]);
    assert_eq!(j.mois_non_clos, [m]);
    assert!(j.ecritures[0].libelle.starts_with("Facture F"));
    assert!(j.ecritures[0].libelle.ends_with(" - Bâtiment Sahel"));
    assert_eq!(j.ecritures[8].libelle.split(" (").nth(1), Some("Espèce)"));
    // Ventes seules.
    let v = journal(&c.conn, Periode::du_mois(m), &[Journal::Ventes]).unwrap();
    assert_eq!((v.ecritures.len(), v.pieces), (8, 2));
    // TVA collectée : la facture moins l'avoir.
    assert_eq!(
        tva_collectee(&c.conn, Periode::du_mois(m)).unwrap(),
        [LigneTva {
            mois: m,
            taux: 1_800,
            base: 276_750 - 13_500,
            tva: 49_815 - 2_430
        }]
    );
}

#[test]
fn cloture_puis_mois_fige() {
    let mut c = contexte();
    let (m, f, r) = mois_rempli(&mut c);
    let admin = c.admin.clone();
    // Un mode sans compte bloque la clôture.
    let e = empreinte(&c.conn, m).unwrap();
    assert!(message(clore(&c.conn, &admin, m, &e).unwrap_err()).contains("Wave"));
    let mut comptes_test = comptes(&c.conn).unwrap();
    comptes_test.tresorerie.push(("Wave".into(), "552".into()));
    enregistrer_comptes(&c.conn, &admin, &comptes_test).unwrap();
    // Empreinte d'une copie pas à jour.
    assert_eq!(
        message(clore(&c.conn, &admin, m, &e).unwrap_err()),
        MODELES_ERREUR[4]
    );
    let e = empreinte(&c.conn, m).unwrap();
    clore(&c.conn, &admin, m, &e).unwrap();
    assert!(est_clos(&c.conn, m.premier_jour()).unwrap());
    let liste = clotures(&c.conn).unwrap();
    assert_eq!(liste.len(), 1);
    assert_eq!(
        (liste[0].mois, liste[0].empreinte.as_str()),
        (m, e.as_str())
    );
    assert!(!liste[0].par.is_empty() && liste[0].le.contains('T'));
    assert!(message(clore(&c.conn, &admin, m, &e).unwrap_err()).contains("déjà clos"));
    let courant = Mois::de_date(aujourd_hui());
    let e2 = empreinte(&c.conn, courant).unwrap();
    assert!(
        message(clore(&c.conn, &admin, courant, &e2).unwrap_err()).contains("n'est pas terminé")
    );

    // Plus rien ne change dans le mois clos.
    let clos = |r: Resultat<()>| assert!(message(r.unwrap_err()).contains("est clos"));
    clos(documents::annuler(&mut c.conn, &admin, f));
    clos(regler(&mut c, f, m.dernier_jour(), 1_000, "Espèce").map(|_| ()));
    clos(reglements::supprimer(&mut c.conn, &admin, r));
    let s = Saisie {
        tiers_id: Some(c.client),
        lignes: vec![ligne(c.ciment, 6_500, U)],
        ..Saisie::vide(m.premier_jour())
    };
    let brouillon =
        documents::creer_brouillon(&mut c.conn, &admin, TypeDocument::Facture, &s).unwrap();
    clos(documents::valider(&mut c.conn, &admin, brouillon).map(|_| ()));

    // Les comptes et la nature des produits changent : le mois clos garde
    // les siens, le mois ouvert prend les nouveaux.
    let mut autres = comptes(&c.conn).unwrap();
    autres.clients = "411".into();
    enregistrer_comptes(&c.conn, &admin, &autres).unwrap();
    let mut fiche = produits::lire(&c.conn, c.pose).unwrap().unwrap().fiche;
    fiche.nature_comptable = NatureComptable::Marchandise;
    produits::modifier(&c.conn, &admin, c.pose, &fiche).unwrap();
    assert_eq!(empreinte(&c.conn, m).unwrap(), e);
    // Correction par un avoir daté du mois ouvert, sur la facture du mois clos.
    let lignes = vec![ligne(c.ciment, 6_500, U)];
    let a = brouillon_avoir(&mut c, f, aujourd_hui(), lignes);
    documents::valider(&mut c.conn, &admin, a).unwrap();
    let j = journal(&c.conn, Periode::du_mois(courant), &[Journal::Ventes]).unwrap();
    assert_eq!(j.ecritures[0].compte, "411");
    assert_eq!(j.mois_non_clos, [courant]);
}

#[test]
fn droits_et_comptes_obligatoires() {
    let c = contexte();
    let vente = compte(&c.conn, "vente", Role::Commercial);
    assert!(enregistrer_comptes(&c.conn, &vente, &ComptesComptables::default()).is_err());
    let m = mois_precedent();
    let e = empreinte(&c.conn, m).unwrap();
    assert!(clore(&c.conn, &vente, m, &e).is_err());
    let vide = ComptesComptables {
        clients: "  ".into(),
        ..Default::default()
    };
    assert!(
        message(enregistrer_comptes(&c.conn, &c.admin, &vide).unwrap_err())
            .contains("Compte clients")
    );
    let mut double = ComptesComptables::default();
    double.tresorerie.push(("Espèce".into(), "571".into()));
    assert!(enregistrer_comptes(&c.conn, &c.admin, &double).is_err());
    // Aller-retour JSON.
    let d = ComptesComptables::default();
    assert_eq!(comptes_depuis_json(&comptes_en_json(&d)), d);
    assert_eq!(comptes(&c.conn).unwrap(), d);
    // Un mois vide se clôt (aucune écriture).
    clore(&c.conn, &c.admin, m, &e).unwrap();
}

/// Migration 0017 : une matrice enregistrée reçoit le droit nouveau pour
/// l'Admin et le Comptable seulement.
#[test]
fn migration_du_droit_dans_une_matrice_enregistree() {
    let conn = Connection::open_in_memory().unwrap();
    for sql in &crate::MIGRATIONS[..16] {
        conn.execute_batch(sql).unwrap();
    }
    conn.execute(
        "INSERT INTO parametre_systeme (cle, valeur) VALUES ('droits.matrice', ?1)",
        [r#"{"admin":["consulter"],"comptable":["consulter"],"commercial":["consulter"],"consultation":["consulter"]}"#],
    )
    .unwrap();
    conn.execute_batch(crate::MIGRATIONS[16]).unwrap();
    let m = crate::droits::lire_matrice(&conn).unwrap();
    for role in [Role::Admin, Role::Comptable] {
        assert!(m.autorise(role, Droit::ExporterComptabilite), "{role:?}");
    }
    for role in [Role::Commercial, Role::Consultation] {
        assert!(!m.autorise(role, Droit::ExporterComptabilite), "{role:?}");
    }
}
