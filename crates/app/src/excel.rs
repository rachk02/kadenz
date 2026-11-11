//! Export Excel des registres et des documents (sections 5, 6 et 8, jalon J10).
//!
//! Chaque registre exporte les lignes affichées à l'écran (filtre et tri déjà
//! appliqués par l'appelant) ; un document s'exporte sur une feuille complète :
//! en-tête, lignes, totaux. Les modules absents de la licence sont refusés comme
//! les écrans correspondants. Les libellés passent par la traduction.

use kadenz_domaine::calcul::Montant;
use kadenz_domaine::document::TypeDocument;
use kadenz_domaine::module::Module;
use kadenz_domaine::referentiel::TypeTiers;
use kadenz_export::{Colonne, Feuille, Valeur, classeur};
use kadenz_stockage::Connexion as Connection;
use kadenz_stockage::documents::{self, ResumeDocument};
use kadenz_stockage::entreprise;
use kadenz_stockage::licence;
use kadenz_stockage::produits::Produit;
use kadenz_stockage::reglements::{DocumentDu, FiltreRegistre, Registre, Sens, TotalSens};
use kadenz_stockage::tiers::Tiers;

use crate::referentiels::texte_erreur;
use crate::traduction::{tr, trf};

type Export = Result<(Vec<u8>, String), String>;

fn aujourd_hui() -> chrono::NaiveDate {
    chrono::Local::now().date_naive()
}

fn exiger(conn: &Connection, m: Option<Module>) -> Result<(), String> {
    match m {
        Some(m) => licence::exiger(conn, m).map_err(texte_erreur),
        None => Ok(()),
    }
}

fn col(titre: &str, largeur: f64) -> Colonne {
    Colonne::new(tr(titre), largeur)
}

fn info(libelle: &str, valeur: impl Into<Valeur>) -> (String, Valeur) {
    (tr(libelle), valeur.into())
}

/// En-tête commun d'un registre : entreprise et date d'export.
fn entete_registre(conn: &Connection) -> Result<Vec<(String, Valeur)>, String> {
    let nom = entreprise::lire(conn)
        .map_err(texte_erreur)?
        .map(|e| e.nom)
        .unwrap_or_default();
    let mut entete = Vec::new();
    if !nom.trim().is_empty() {
        entete.push(info("Entreprise", nom));
    }
    entete.push(info("Exporté le", Some(aujourd_hui())));
    Ok(entete)
}

fn exporter(feuille: Feuille, nom: &str) -> Export {
    let octets = classeur(&[feuille])?;
    Ok((
        octets,
        format!("{nom}-{}.xlsx", aujourd_hui().format("%Y-%m-%d")),
    ))
}

/// Nom de fichier sans caractère interdit par Windows.
fn nom_sur(nom: &str) -> String {
    nom.chars()
        .map(|c| {
            if "<>:\"/\\|?*".contains(c) || c.is_control() {
                '-'
            } else {
                c
            }
        })
        .collect()
}

fn titre_registre(t: TypeDocument) -> &'static str {
    match t {
        TypeDocument::Devis => "Registre des devis",
        TypeDocument::Proforma => "Registre des proformas",
        TypeDocument::BonCommande => "Registre des bons de commande",
        TypeDocument::Livraison => "Registre des bordereaux de livraison",
        TypeDocument::Facture => "Registre des factures",
        TypeDocument::Avoir => "Registre des avoirs",
    }
}

/// Registre d'un type de document, tel qu'affiché.
pub(crate) fn registre_documents(
    conn: &Connection,
    t: TypeDocument,
    lignes: &[ResumeDocument],
) -> Export {
    exiger(conn, t.module())?;
    let montants = t.a_des_montants();
    let paiement = t.recoit_des_reglements();
    let mut colonnes = vec![
        col("Numéro", 16.0),
        col("Date", 12.0),
        col("Tiers", 30.0),
        col("Libellé", 30.0),
        col("Statut", 18.0),
    ];
    if paiement {
        colonnes.push(col("Mode de paiement", 18.0));
        colonnes.push(col("Échéance", 12.0));
        colonnes.push(col("Paiement", 16.0));
    }
    if montants {
        colonnes.push(col("Montant TTC", 16.0));
    }
    let lignes_xlsx = lignes
        .iter()
        .map(|d| {
            let mut v = vec![
                Valeur::from(d.numero.clone().unwrap_or_default()),
                Valeur::Date(d.date),
                d.tiers_nom.clone().into(),
                d.libelle.clone().into(),
                tr(&d.statut.to_string()).into(),
            ];
            if paiement {
                v.push(d.mode_paiement.clone().into());
                v.push(d.date_echeance.into());
                v.push(
                    d.etat_paiement
                        .map(|e| tr(&e.to_string()))
                        .unwrap_or_default()
                        .into(),
                );
            }
            if montants {
                v.push(Valeur::Montant(d.montant_ttc));
            }
            v
        })
        .collect();
    let mut totaux = Vec::new();
    if montants {
        let total: Montant = lignes.iter().map(|d| d.montant_ttc).sum();
        totaux.push(info("Total TTC", Valeur::Montant(total)));
    }
    let titre = tr(titre_registre(t));
    exporter(
        Feuille {
            nom: tr(&t.to_string()),
            titre: titre.clone(),
            entete: entete_registre(conn)?,
            colonnes,
            lignes: lignes_xlsx,
            totaux,
        },
        &nom_sur(&titre),
    )
}

fn libelle_type_tiers(t: TypeTiers) -> &'static str {
    match t {
        TypeTiers::Client => "Client",
        TypeTiers::Fournisseur => "Fournisseur",
        TypeTiers::LesDeux => "Les deux",
    }
}

/// Liste des clients et fournisseurs, telle qu'affichée.
pub(crate) fn tiers(conn: &Connection, lignes: &[Tiers]) -> Export {
    let titre = tr("Clients et fournisseurs");
    exporter(
        Feuille {
            nom: titre.clone(),
            titre: titre.clone(),
            entete: entete_registre(conn)?,
            colonnes: vec![
                col("Nom", 30.0),
                col("Type", 14.0),
                col("Téléphone", 16.0),
                col("Adresse", 30.0),
                col("Localisation", 20.0),
                col("IFU", 16.0),
                col("RCCM", 18.0),
                col("Régime d'imposition", 18.0),
                col("Actif", 8.0),
            ],
            lignes: lignes
                .iter()
                .map(|t| {
                    let f = &t.fiche;
                    vec![
                        f.nom.clone().into(),
                        tr(libelle_type_tiers(f.type_tiers)).into(),
                        f.telephone.clone().into(),
                        f.adresse.clone().into(),
                        f.localisation.clone().into(),
                        f.ifu.clone().into(),
                        f.rccm.clone().into(),
                        f.regime_imposition.clone().into(),
                        tr(if t.actif { "Oui" } else { "Non" }).into(),
                    ]
                })
                .collect(),
            totaux: Vec::new(),
        },
        &nom_sur(&titre),
    )
}

/// Liste des produits, telle qu'affichée.
pub(crate) fn produits(conn: &Connection, lignes: &[Produit]) -> Export {
    let titre = tr("Produits");
    exporter(
        Feuille {
            nom: titre.clone(),
            titre: titre.clone(),
            entete: entete_registre(conn)?,
            colonnes: vec![
                col("Référence", 16.0),
                col("Désignation", 40.0),
                col("Prix unitaire", 16.0),
                col("Sens", 12.0),
                col("Actif", 8.0),
            ],
            lignes: lignes
                .iter()
                .map(|p| {
                    let f = &p.fiche;
                    vec![
                        f.reference.clone().into(),
                        f.designation.clone().into(),
                        Valeur::Montant(f.prix),
                        tr(&f.sens.to_string()).into(),
                        tr(if p.actif { "Oui" } else { "Non" }).into(),
                    ]
                })
                .collect(),
            totaux: Vec::new(),
        },
        &nom_sur(&titre),
    )
}

fn libelle_sens(s: Sens) -> &'static str {
    match s {
        Sens::Encaissement => "Encaissement",
        Sens::Decaissement => "Décaissement",
    }
}

fn totaux_du_sens(libelle: &str, t: &TotalSens, totaux: &mut Vec<(String, Valeur)>) {
    totaux.push(info(libelle, Valeur::Montant(t.total)));
    for (mode, montant) in &t.par_mode {
        totaux.push((trf("dont {}", &[mode]), Valeur::Montant(*montant)));
    }
}

/// Registre des règlements tel qu'affiché, avec le total et les sous-totaux
/// par mode de chaque sens retenu par le filtre.
pub(crate) fn registre_reglements(
    conn: &Connection,
    r: &Registre,
    filtre: &FiltreRegistre,
) -> Export {
    exiger(conn, Some(Module::Paiements))?;
    let mut entete = entete_registre(conn)?;
    entete.push(info(
        "Sens",
        tr(match filtre.sens {
            Some(Sens::Encaissement) => "Encaissements (factures)",
            Some(Sens::Decaissement) => "Décaissements (bons de commande)",
            None => "Encaissements et décaissements",
        }),
    ));
    if filtre.du.is_some() {
        entete.push(info("Du", filtre.du));
    }
    if filtre.au.is_some() {
        entete.push(info("Au", filtre.au));
    }
    if let Some(mode) = filtre.mode.as_deref().filter(|m| !m.trim().is_empty()) {
        entete.push(info("Mode de paiement", mode.trim()));
    }
    let mut totaux = Vec::new();
    if filtre.sens != Some(Sens::Decaissement) {
        totaux_du_sens("Total des encaissements", &r.encaissements, &mut totaux);
    }
    if filtre.sens != Some(Sens::Encaissement) {
        totaux_du_sens("Total des décaissements", &r.decaissements, &mut totaux);
    }
    let titre = tr("Registre des règlements");
    exporter(
        Feuille {
            nom: titre.clone(),
            titre: titre.clone(),
            entete,
            colonnes: vec![
                col("Date", 12.0),
                col("Sens", 14.0),
                col("Document", 16.0),
                col("Tiers", 30.0),
                col("Mode", 16.0),
                col("Référence", 18.0),
                col("Saisi par", 20.0),
                col("Montant", 16.0),
            ],
            lignes: r
                .lignes
                .iter()
                .map(|l| {
                    vec![
                        Valeur::Date(l.reglement.date),
                        tr(libelle_sens(l.sens)).into(),
                        l.numero.clone().into(),
                        l.tiers.clone().into(),
                        l.reglement.mode_paiement.clone().into(),
                        l.reglement.reference.clone().into(),
                        l.reglement.auteur.clone().into(),
                        Valeur::Montant(l.reglement.montant),
                    ]
                })
                .collect(),
            totaux,
        },
        &nom_sur(&titre),
    )
}

/// Feuille d'une liste de documents qui restent à payer.
fn documents_dus(
    conn: &Connection,
    titre: &str,
    tiers: &str,
    retard: bool,
    lignes: &[DocumentDu],
) -> Export {
    let mut colonnes = vec![
        col("Document", 16.0),
        col("Date", 12.0),
        col(tiers, 30.0),
        col("Échéance", 12.0),
    ];
    if retard {
        colonnes.push(col("Retard (jours)", 12.0));
    }
    colonnes.extend([
        col("Montant dû", 16.0),
        col("Réglé", 16.0),
        col("Reste dû", 16.0),
    ]);
    let reste: Montant = lignes.iter().map(|d| d.reste_a_payer).sum();
    let titre = tr(titre);
    exporter(
        Feuille {
            nom: titre.clone(),
            titre: titre.clone(),
            entete: entete_registre(conn)?,
            colonnes,
            lignes: lignes
                .iter()
                .map(|d| {
                    let mut v = vec![
                        d.numero.clone().into(),
                        Valeur::Date(d.date),
                        d.tiers.clone().into(),
                        d.echeance.into(),
                    ];
                    if retard {
                        v.push(d.jours_de_retard.map_or(Valeur::Vide, Valeur::Entier));
                    }
                    v.extend([
                        Valeur::Montant(d.montant_du),
                        Valeur::Montant(d.regle),
                        Valeur::Montant(d.reste_a_payer),
                    ]);
                    v
                })
                .collect(),
            totaux: vec![info("Reste dû", Valeur::Montant(reste))],
        },
        &nom_sur(&titre),
    )
}

/// Factures échues non payées, telles qu'affichées (relances).
pub(crate) fn factures_echues(conn: &Connection, lignes: &[DocumentDu]) -> Export {
    exiger(conn, Some(Module::Relances))?;
    documents_dus(conn, "Factures échues", "Client", true, lignes)
}

/// Bons de commande non payés, tels qu'affichés (suivi fournisseurs).
pub(crate) fn bons_non_payes(conn: &Connection, lignes: &[DocumentDu]) -> Export {
    exiger(conn, Some(Module::Paiements))?;
    exiger(conn, Some(Module::BonsCommande))?;
    documents_dus(
        conn,
        "Bons de commande non payés",
        "Fournisseur",
        false,
        lignes,
    )
}

/// Un document sur une feuille : en-tête, lignes, totaux.
pub(crate) fn document(conn: &Connection, id: i64) -> Export {
    let d = documents::lire(conn, id)
        .map_err(texte_erreur)?
        .ok_or_else(|| tr("document introuvable"))?;
    let t = d.type_document;
    exiger(conn, t.module())?;
    let s = &d.saisie;
    let montants = t.a_des_montants();
    let livree = matches!(t, TypeDocument::Livraison | TypeDocument::BonCommande);

    let mut entete = Vec::new();
    if let Some(n) = &d.numero {
        entete.push(info("Numéro", n.clone()));
    }
    entete.extend([
        info("Statut", tr(&d.statut.to_string())),
        info("Date", Some(s.date)),
        info(
            match t {
                TypeDocument::BonCommande => "Fournisseur",
                _ => "Client",
            },
            d.tiers.nom.clone(),
        ),
    ]);
    for (libelle, valeur) in [
        ("Adresse", &d.tiers.adresse),
        ("IFU", &d.tiers.ifu),
        ("RCCM", &d.tiers.rccm),
        ("Régime d'imposition", &d.tiers.regime),
        ("Libellé", &s.libelle),
        ("Mode de paiement", &s.mode_paiement),
    ] {
        if !valeur.trim().is_empty() {
            entete.push(info(libelle, valeur.clone()));
        }
    }
    if s.date_echeance.is_some() {
        entete.push(info("Échéance", s.date_echeance));
    }
    if let Some(source) = d.source.as_ref().and_then(|l| l.numero.clone()) {
        entete.push(info("Document d'origine", source));
    }
    if montants {
        entete.push(info("Remise", Valeur::Pourcentage(s.taux.remise)));
        entete.push(info("TVA", Valeur::Pourcentage(s.taux.tva)));
        if t.retenue_applicable() {
            entete.push(info(
                "Retenue à la source",
                Valeur::Pourcentage(s.taux.retenue),
            ));
        }
    }

    let mut colonnes = vec![
        col("Référence", 14.0),
        col("Désignation", 40.0),
        col("Quantité", 10.0),
    ];
    if livree {
        colonnes.push(col(
            if t == TypeDocument::Livraison {
                "Quantité livrée"
            } else {
                "Quantité reçue"
            },
            12.0,
        ));
    }
    // Observations avant les montants : les totaux s'alignent sur les deux
    // dernières colonnes, Prix unitaire et Montant.
    colonnes.push(col("Observations", 30.0));
    if montants {
        colonnes.push(col("Prix unitaire", 14.0));
        colonnes.push(col("Montant", 16.0));
    }
    let lignes = s
        .lignes
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let mut v = vec![
                l.reference.clone().into(),
                l.designation.clone().into(),
                Valeur::Quantite(l.quantite),
            ];
            if livree {
                v.push(Valeur::Quantite(l.quantite_livree));
            }
            v.push(l.observations.clone().into());
            if montants {
                v.push(Valeur::Montant(l.prix));
                v.push(Valeur::Montant(
                    d.totaux_lignes.get(i).copied().unwrap_or_default(),
                ));
            }
            v
        })
        .collect();

    let mut totaux = Vec::new();
    if montants {
        let x = &d.totaux;
        totaux.push(info("Total HT", Valeur::Montant(x.ht)));
        for (libelle, valeur) in [
            ("Remise", x.remise),
            ("Base TVA", x.base_tva),
            ("TVA", x.tva),
            ("Retenue à la source", x.retenue),
        ] {
            if valeur != 0 {
                totaux.push(info(libelle, Valeur::Montant(valeur)));
            }
        }
        totaux.push(info("Total TTC", Valeur::Montant(x.ttc)));
    }
    if !s.observations.trim().is_empty() {
        entete.push(info("Observations", s.observations.clone()));
    }

    let type_libelle = tr(&t.to_string());
    let nom = match &d.numero {
        Some(n) => n.clone(),
        None => format!("Apercu-{}-{}", t.code(), d.id),
    };
    let octets = classeur(&[Feuille {
        nom: d.numero.clone().unwrap_or_else(|| type_libelle.clone()),
        titre: match &d.numero {
            Some(n) => format!("{type_libelle} {n}"),
            None => type_libelle,
        },
        entete,
        colonnes,
        lignes,
        totaux,
    }])?;
    Ok((octets, format!("{}.xlsx", nom_sur(&nom))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noms_de_fichier_surs() {
        assert_eq!(nom_sur("FA/2026:01"), "FA-2026-01");
        assert_eq!(nom_sur("Registre des factures"), "Registre des factures");
    }

    #[test]
    fn registres_exportes() {
        let conn = kadenz_stockage::ouvrir_en_memoire().unwrap();
        let (octets, nom) = registre_documents(&conn, TypeDocument::Facture, &[]).unwrap();
        assert!(octets.starts_with(b"PK"));
        assert!(nom.starts_with("Registre des factures-") && nom.ends_with(".xlsx"));
        assert!(tiers(&conn, &[]).is_ok());
        assert!(produits(&conn, &[]).is_ok());
        assert!(
            registre_reglements(&conn, &Registre::default(), &FiltreRegistre::default()).is_ok()
        );
        assert!(factures_echues(&conn, &[]).is_ok());
        assert!(bons_non_payes(&conn, &[]).is_ok());
    }
}
