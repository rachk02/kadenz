//! Reprise d'une base V1 dans un serveur neuf (phase 2, F7 ; écrans,
//! section 8 ; décisions 19 et 53 à 55) : assistant de la rubrique
//! « Serveur », réservé à l'Admin d'un poste relié. Le résumé est lu sur ce
//! poste ; la reprise passe par le serveur, qui seul juge s'il est neuf.

use std::path::PathBuf;

use kadenz_domaine::document::TypeDocument;
use kadenz_poste::{ErreurPoste, Poignee, ResumeReprise, SourceV1};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::referentiels::{retour_erreur, retour_ok};
use crate::traduction::{tr, trf};
use crate::{EtatPartage, FenetrePrincipale, LigneResumeUi, ServeurApi};

/// État de l'assistant (`ServeurApi.reprise-etat`).
const PAS_VERIFIE: i32 = 0;
const NEUF: i32 = 1;
const PAS_NEUF: i32 = 2;
const FAITE: i32 = 3;

/// Libellé d'un type de document dans le résumé : le nom de sa page.
fn libelle_type(code: &str) -> String {
    match TypeDocument::depuis_code(code) {
        Some(TypeDocument::Devis) => tr("Devis"),
        Some(TypeDocument::Proforma) => tr("Proforma"),
        Some(TypeDocument::BonCommande) => tr("Bons de commande"),
        Some(TypeDocument::Livraison) => tr("Livraisons"),
        Some(TypeDocument::Facture) => tr("Factures"),
        Some(TypeDocument::Avoir) => tr("Avoirs"),
        None => code.to_string(),
    }
}

/// Lignes du résumé (ou du bilan) d'une reprise, libellé et valeur, dans
/// l'ordre de l'écran ; les comptes écartés sont nommés à part.
pub(crate) fn lignes_resume(resume: &ResumeReprise) -> Vec<(String, String)> {
    let mut lignes = vec![
        (tr("Entreprise"), resume.entreprise.clone()),
        (tr("Clients"), resume.clients.to_string()),
        (tr("Fournisseurs"), resume.fournisseurs.to_string()),
        (tr("Produits"), resume.produits.to_string()),
        (tr("Règlements"), resume.reglements.to_string()),
    ];
    for d in &resume.documents {
        let valeur = match &d.dernier_numero {
            Some(numero) => trf("{} (dernier numéro {})", &[&d.nombre, numero]),
            None => d.nombre.to_string(),
        };
        lignes.push((libelle_type(&d.type_document), valeur));
    }
    let comptes = if resume.comptes_repris.is_empty() {
        tr("aucun")
    } else {
        resume.comptes_repris.join(", ")
    };
    lignes.push((tr("Comptes repris"), comptes));
    lignes
}

/// Poignée du synchroniseur, pour un appel au serveur hors du fil de
/// l'interface ; `None` sur un poste non relié.
fn poignee(etat: &EtatPartage) -> Option<Poignee> {
    etat.borrow().relie.as_ref().map(|r| r.synchro.poignee())
}

/// Source choisie dans l'assistant : la base V1 de ce poste, ou une
/// sauvegarde `.kzs` et son mot de passe.
fn source(f: &FenetrePrincipale, sauvegarde: bool, mot_de_passe: &str) -> SourceV1 {
    if sauvegarde {
        SourceV1::Sauvegarde {
            chemin: PathBuf::from(f.global::<ServeurApi>().get_reprise_fichier().as_str()),
            mot_de_passe: mot_de_passe.to_string(),
        }
    } else {
        SourceV1::Base(crate::chemin_base())
    }
}

fn texte(e: &ErreurPoste) -> String {
    crate::relie::texte_erreur(e)
}

/// Montre un résumé (ou un bilan) dans l'assistant.
fn montrer(f: &FenetrePrincipale, resume: Option<&ResumeReprise>) {
    let api = f.global::<ServeurApi>();
    let (lignes, ecartes): (Vec<LigneResumeUi>, Vec<SharedString>) = match resume {
        Some(r) => (
            lignes_resume(r)
                .into_iter()
                .map(|(libelle, valeur)| LigneResumeUi {
                    libelle: libelle.into(),
                    valeur: valeur.into(),
                })
                .collect(),
            r.comptes_ecartes.iter().map(SharedString::from).collect(),
        ),
        None => (Vec::new(), Vec::new()),
    };
    api.set_reprise_resume(ModelRc::new(VecModel::from(lignes)));
    api.set_reprise_ecartes(ModelRc::new(VecModel::from(ecartes)));
}

/// Réponse du serveur à « est-il neuf ? ».
fn noter_possible(f: &FenetrePrincipale, resultat: Result<bool, ErreurPoste>) {
    let api = f.global::<ServeurApi>();
    api.set_reprise_occupe(false);
    match resultat {
        Ok(true) => {
            api.set_reprise_etat(NEUF);
            api.set_reprise_retour(retour_ok(String::new()));
        }
        Ok(false) => {
            api.set_reprise_etat(PAS_NEUF);
            api.set_reprise_retour(retour_ok(String::new()));
        }
        Err(e) => api.set_reprise_retour(retour_erreur(texte(&e))),
    }
}

/// Fin d'une reprise. Après un délai dépassé, le serveur dit lui-même si la
/// reprise est passée : elle tient en une transaction, faite ou pas faite.
fn noter_reprise(f: &FenetrePrincipale, resultat: Result<ResumeReprise, ErreurPoste>, p: Poignee) {
    let api = f.global::<ServeurApi>();
    match resultat {
        Ok(bilan) => {
            api.set_reprise_occupe(false);
            api.set_reprise_etat(FAITE);
            montrer(f, Some(&bilan));
            api.set_reprise_retour(retour_ok(tr(
                "Base V1 reprise dans le serveur. Ses données arrivent sur les postes avec la synchronisation.",
            )));
        }
        Err(ErreurPoste::Delai) => {
            let fa = f.as_weak();
            std::thread::spawn(move || {
                let possible = p.reprise_possible();
                let _ = fa.upgrade_in_event_loop(move |f| {
                    let api = f.global::<ServeurApi>();
                    api.set_reprise_occupe(false);
                    match possible {
                        Ok(false) => {
                            api.set_reprise_etat(FAITE);
                            montrer(&f, None);
                            api.set_reprise_retour(retour_ok(tr(
                                "Le serveur a répondu après le délai : la base V1 est reprise. Ses données arrivent sur les postes avec la synchronisation.",
                            )));
                        }
                        Ok(true) => api.set_reprise_retour(retour_erreur(tr(
                            "La reprise n'a pas abouti : le serveur est toujours neuf. Vous pouvez la relancer.",
                        ))),
                        Err(_) => api.set_reprise_retour(retour_erreur(tr(
                            "Le serveur n'a pas encore répondu : la reprise est peut-être en cours. Vérifiez plus tard que le serveur est neuf avant de la relancer.",
                        ))),
                    }
                });
            });
        }
        Err(e) => {
            api.set_reprise_occupe(false);
            if matches!(&e, ErreurPoste::Refus { modele, .. } if modele == kadenz_stockage::reprise::MODELE_PAS_NEUF)
            {
                api.set_reprise_etat(PAS_NEUF);
                montrer(f, None);
            }
            api.set_reprise_retour(retour_erreur(texte(&e)));
        }
    }
}

pub(crate) fn brancher(fenetre: &FenetrePrincipale, etat: &EtatPartage) {
    let api = fenetre.global::<ServeurApi>();
    api.set_reprise_etat(PAS_VERIFIE);

    let (fa, e) = (fenetre.as_weak(), etat.clone());
    api.on_reprise_verifier(move || {
        let (Some(f), Some(p)) = (fa.upgrade(), poignee(&e)) else {
            return;
        };
        f.global::<ServeurApi>().set_reprise_occupe(true);
        let fa = f.as_weak();
        std::thread::spawn(move || {
            let resultat = p.reprise_possible();
            let _ = fa.upgrade_in_event_loop(move |f| noter_possible(&f, resultat));
        });
    });

    let fa = fenetre.as_weak();
    api.on_reprise_choisir_fichier(move || {
        let Some(f) = fa.upgrade() else { return };
        let choisi = rfd::FileDialog::new()
            .set_title(tr("Choisir une sauvegarde"))
            .add_filter(tr("Sauvegardes Kadenz"), &["kzs"])
            .set_parent(&f.window().window_handle())
            .pick_file();
        if let Some(chemin) = choisi {
            let api = f.global::<ServeurApi>();
            api.set_reprise_fichier(chemin.display().to_string().into());
            montrer(&f, None);
        }
    });

    // Résumé lu sur ce poste : déchiffrer une sauvegarde peut prendre une ou
    // deux secondes, d'où le fil à part.
    let (fa, e) = (fenetre.as_weak(), etat.clone());
    api.on_reprise_resumer(move |sauvegarde, mot_de_passe| {
        let (Some(f), Some(p)) = (fa.upgrade(), poignee(&e)) else {
            return;
        };
        let source = source(&f, sauvegarde, &mot_de_passe);
        let api = f.global::<ServeurApi>();
        api.set_reprise_occupe(true);
        montrer(&f, None);
        let fa = f.as_weak();
        std::thread::spawn(move || {
            let resultat = p.resume_reprise(&source);
            let _ = fa.upgrade_in_event_loop(move |f| {
                let api = f.global::<ServeurApi>();
                api.set_reprise_occupe(false);
                match resultat {
                    Ok(resume) => {
                        montrer(&f, Some(&resume));
                        api.set_reprise_retour(retour_ok(String::new()));
                    }
                    Err(e) => api.set_reprise_retour(retour_erreur(texte(&e))),
                }
            });
        });
    });

    let (fa, e) = (fenetre.as_weak(), etat.clone());
    api.on_reprise_lancer(move |sauvegarde, mot_de_passe| {
        let (Some(f), Some(p)) = (fa.upgrade(), poignee(&e)) else {
            return;
        };
        let source = source(&f, sauvegarde, &mot_de_passe);
        let api = f.global::<ServeurApi>();
        api.set_reprise_occupe(true);
        api.set_reprise_retour(retour_ok(String::new()));
        let fa = f.as_weak();
        std::thread::spawn(move || {
            let resultat = p.reprendre(&source);
            let _ = fa.upgrade_in_event_loop(move |f| noter_reprise(&f, resultat, p));
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use kadenz_poste::DocumentsRepris;

    #[test]
    fn resume_en_lignes() {
        let resume = ResumeReprise {
            entreprise: "Entreprise d'essai".into(),
            clients: 3,
            fournisseurs: 1,
            produits: 2,
            reglements: 0,
            documents: vec![
                DocumentsRepris {
                    type_document: "facture".into(),
                    nombre: 4,
                    dernier_numero: Some("F2026-11-0004".into()),
                },
                DocumentsRepris {
                    type_document: "devis".into(),
                    nombre: 1,
                    dernier_numero: None,
                },
            ],
            comptes_repris: vec!["admin".into(), "vente".into()],
            comptes_ecartes: vec![],
        };
        let lignes = lignes_resume(&resume);
        assert_eq!(lignes.len(), 8);
        assert_eq!(lignes[0].1, "Entreprise d'essai");
        assert_eq!(lignes[5].1, "4 (dernier numéro F2026-11-0004)");
        assert_eq!(lignes[6].1, "1");
        assert_eq!(lignes[7].1, "admin, vente");
    }
}
