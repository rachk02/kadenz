//! Licence du poste (jalon J9) : identifiant du poste, état de la licence, modules
//! activés et masquage des écrans des modules désactivés. En 0.2.3 (ligne V1 par
//! codes, docs/spec-codes-licence.md, section 7 bis) : échéance, postes et
//! bandeau d'avertissement, et activation d'un code court auprès du portail
//! (`ActivationUi`).

use kadenz_domaine::document::TypeDocument;
use kadenz_domaine::message::MessageCompose;
use kadenz_domaine::module::Module;
use kadenz_licence::Licence;
use kadenz_stockage::licence::{self as stockage, Avertissement, EtatLicence};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::referentiels::{retour_erreur, retour_ok, texte_erreur};
use crate::traduction::{tr, trf};
use crate::{
    ActivationUi, EtatPartage, FenetrePrincipale, LicenceApi, ModuleUi, Modules, Page, Retour,
    VerificationUi,
};

fn message(m: MessageCompose) -> String {
    texte_erreur(kadenz_stockage::Erreur::from(m))
}

/// Libellés traduits des modules, séparés par des virgules.
fn liste_modules(modules: &[Module]) -> String {
    modules
        .iter()
        .map(|m| tr(m.libelle()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Date AAAA-MM-JJ d'une licence en JJ/MM/AAAA ; telle quelle si elle est illisible.
fn date_lisible(date: &str) -> String {
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map(crate::factures::date_en_texte)
        .unwrap_or_else(|_| date.to_string())
}

/// Échéance en clair : date, ou « Perpétuelle » pour une licence version 1.
fn echeance_en_clair(l: &Licence) -> String {
    match &l.echeance {
        Some(date) => date_lisible(date),
        None => tr("Perpétuelle"),
    }
}

/// Message de refus quand la licence reçue reculerait l'échéance de la licence
/// en cours encore valide (décision du 2026-11-19, défaut vu à l'essai à
/// l'écran de la 0.2.3 : un ancien code plus court remplaçait sans avertir une
/// licence plus longue). Une licence en cours échue n'est jamais protégée.
fn recul_d_echeance(en_cours: &EtatLicence, recue: &Licence) -> Option<String> {
    let EtatLicence::Valide(actuelle, _) = en_cours else {
        return None;
    };
    let jour = |d: &Option<String>| {
        d.as_deref()
            .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
    };
    let (actuelle, recue) = (jour(&actuelle.echeance)?, jour(&recue.echeance)?);
    (recue < actuelle).then(|| {
        crate::phrase(trf(
            "ce code mènerait votre licence au {}, avant son échéance actuelle du {} : licence gardée",
            &[
                &crate::factures::date_en_texte(recue),
                &crate::factures::date_en_texte(actuelle),
            ],
        ))
        .to_string()
    })
}

/// Résumé d'une licence pour l'assistant et le récapitulatif.
fn resume(l: &Licence) -> String {
    match &l.echeance {
        Some(date) => trf(
            "{}, valable jusqu'au {}, modules : {}",
            &[&l.client, &date_lisible(date), &liste_modules(&l.modules)],
        ),
        None => trf(
            "clé valide pour {}, modules : {}",
            &[&l.client, &liste_modules(&l.modules)],
        ),
    }
}

/// Bandeau de la page Licence et son niveau (1 : 30 et 7 jours, 2 : la veille et
/// après l'échéance) ; vide sans avertissement (licence perpétuelle comprise).
fn bandeau(etat: &EtatLicence) -> (String, i32) {
    let echeance = match etat {
        EtatLicence::Valide(l, _) | EtatLicence::Expiree(l) => {
            l.echeance.as_deref().map(date_lisible).unwrap_or_default()
        }
        _ => String::new(),
    };
    match etat.avertissement() {
        Avertissement::Aucun => (String::new(), 0),
        Avertissement::Trente | Avertissement::Sept => (
            trf(
                "La licence arrive à échéance dans {} jours, le {}. Renouvelez-la avec un nouveau code d'activation pour garder les modules.",
                &[&etat.jours_restants().unwrap_or_default(), &echeance],
            ),
            1,
        ),
        Avertissement::Veille => (
            trf(
                "La licence arrive à échéance le {}. Renouvelez-la dès maintenant avec un nouveau code d'activation pour garder les modules.",
                &[&echeance],
            ),
            2,
        ),
        Avertissement::Expiree => (
            trf(
                "La licence est échue depuis le {} : seul le socle est disponible, les données des modules sont gardées. Renouvelez-la avec un nouveau code d'activation.",
                &[&echeance],
            ),
            2,
        ),
    }
}

/// Module dont dépend une page du menu, s'il y en a un (le reste est le socle).
pub(crate) fn module_de_page(page: Page) -> &'static [Module] {
    match page {
        Page::TableauDeBord => &[Module::TableauDeBord],
        Page::Devis => &[Module::Devis],
        Page::Proforma => &[Module::Proforma],
        Page::BonsCommande => &[Module::BonsCommande],
        Page::Livraisons => &[Module::Livraisons],
        Page::Reglements => &[Module::Paiements],
        Page::SuiviFournisseurs => &[Module::Paiements, Module::BonsCommande],
        Page::Relances => &[Module::Relances, Module::Paiements],
        _ => &[],
    }
}

/// Vrai si la licence de ce poste active le module.
pub(crate) fn actif(conn: &kadenz_stockage::Connexion, m: Module) -> bool {
    stockage::module_actif(conn, m).unwrap_or(false)
}

/// Vrai si la page est ouverte par la licence.
pub(crate) fn page_active(conn: &kadenz_stockage::Connexion, page: Page) -> bool {
    module_de_page(page).iter().all(|m| actif(conn, *m))
}

/// Vrai si le type de document est ouvert par la licence (factures et avoirs : socle).
pub(crate) fn type_actif(conn: &kadenz_stockage::Connexion, t: TypeDocument) -> bool {
    t.module().is_none_or(|m| actif(conn, m))
}

/// Recopie l'état de la licence dans les écrans : onglet Licence, modules visibles,
/// types du bas des documents.
pub(crate) fn afficher(
    f: &FenetrePrincipale,
    conn: &kadenz_stockage::Connexion,
    etat: &EtatLicence,
) {
    let api = f.global::<LicenceApi>();
    let (texte, valide, licence) = match etat {
        EtatLicence::Absente => (tr("Aucune licence : socle seul"), false, None),
        EtatLicence::Invalide(m) => (
            trf("Clé invalide, socle seul : {}", &[&message(m.clone())]),
            false,
            None,
        ),
        EtatLicence::Valide(l, _) => (tr("Licence valide"), true, Some(l)),
        EtatLicence::Expiree(l) => (
            trf(
                "Licence échue le {} : socle seul",
                &[&l.echeance.as_deref().map(date_lisible).unwrap_or_default()],
            ),
            false,
            Some(l),
        ),
    };
    api.set_etat(texte.into());
    api.set_valide(valide);
    api.set_client(licence.map(|l| l.client.clone()).unwrap_or_default().into());
    api.set_emise_le(
        licence
            .map(|l| date_lisible(&l.emise_le))
            .unwrap_or_default()
            .into(),
    );
    api.set_datee(licence.is_some_and(|l| !l.perpetuelle()));
    api.set_echeance(licence.map(echeance_en_clair).unwrap_or_default().into());
    api.set_postes(
        licence
            .map(|l| l.postes.to_string())
            .unwrap_or_default()
            .into(),
    );
    let (texte_bandeau, niveau) = bandeau(etat);
    api.set_bandeau(texte_bandeau.into());
    api.set_niveau_bandeau(niveau);
    let actifs = etat.modules();
    api.set_modules(ModelRc::new(VecModel::from(
        Module::TOUS
            .iter()
            .map(|m| ModuleUi {
                libelle: tr(m.libelle()).into(),
                actif: actifs.contains(m),
            })
            .collect::<Vec<_>>(),
    )));

    let modules = f.global::<Modules>();
    modules.set_devis(actif(conn, Module::Devis));
    modules.set_proforma(actif(conn, Module::Proforma));
    modules.set_bons_commande(actif(conn, Module::BonsCommande));
    modules.set_livraisons(actif(conn, Module::Livraisons));
    modules.set_paiements(actif(conn, Module::Paiements));
    modules.set_tableau_de_bord(actif(conn, Module::TableauDeBord));
    modules.set_relances(actif(conn, Module::Relances));
    crate::impression::appliquer_types_bas(f, |t| type_actif(conn, t));
}

/// Identifiant de ce poste ; une erreur de lecture laisse l'identifiant vide (socle seul).
pub(crate) fn poste_local() -> String {
    kadenz_licence::poste_local().unwrap_or_default()
}

/// À l'ouverture de la base : vérifie la clé enregistrée pour ce poste et l'affiche.
pub(crate) fn ouvrir(f: &FenetrePrincipale, conn: &kadenz_stockage::Connexion, poste: &str) {
    f.global::<LicenceApi>().set_poste(poste.into());
    let etat = stockage::appliquer(conn, poste).unwrap_or(EtatLicence::Absente);
    afficher(f, conn, &etat);
}

pub(crate) fn brancher(fenetre: &FenetrePrincipale, etat: &EtatPartage) {
    brancher_activation(fenetre, etat);
    let api = fenetre.global::<LicenceApi>();

    // Étape de l'assistant : la base n'a pas encore de compte, la clé est vérifiée seule.
    let e = etat.clone();
    api.on_verifier(move |cle| {
        let poste = e.borrow().poste.clone();
        let cle: String = cle.chars().filter(|c| !c.is_whitespace()).collect();
        match kadenz_licence::verifier(&cle, &poste) {
            Ok(l) => VerificationUi {
                ok: true,
                message: trf(
                    "clé valide pour {}, modules : {}",
                    &[&l.client, &liste_modules(&l.modules)],
                )
                .into(),
            },
            Err(m) => VerificationUi {
                ok: false,
                message: crate::phrase(message(m)),
            },
        }
    });

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_enregistrer(move |cle| -> Retour {
        let etat = e.borrow();
        let Some(acteur) = &etat.utilisateur else {
            return retour_erreur(tr("aucune session ouverte"));
        };
        match stockage::enregistrer(&etat.conn, acteur, &cle, &etat.poste) {
            Ok(licence) => {
                if let Some(f) = f.upgrade() {
                    afficher(&f, &etat.conn, &licence);
                    crate::appliquer_droits(&f, &etat);
                }
                match licence {
                    EtatLicence::Valide(..) => retour_ok(tr("Clé enregistrée.")),
                    EtatLicence::Expiree(l) => retour_erreur(trf(
                        "Clé enregistrée, mais la licence est échue depuis le {} : seul le socle est disponible.",
                        &[&l.echeance.as_deref().map(date_lisible).unwrap_or_default()],
                    )),
                    EtatLicence::Invalide(m) => retour_erreur(message(m)),
                    EtatLicence::Absente => retour_erreur(tr("Aucune licence : socle seul")),
                }
            }
            Err(erreur) => retour_erreur(texte_erreur(erreur)),
        }
    });
}

/// Activation d'un code court (décision 75) : le portail est appelé dans un fil à
/// part avec l'identifiant de ce poste et la raison sociale. Dans l'assistant
/// (aucune session), la licence reçue et vérifiée est rendue à l'écran, qui la
/// garde jusqu'à la fin de l'installation ; dans la page Licence, elle suit le
/// chemin d'une clé collée (`LicenceApi.enregistrer`), dans la boucle de
/// l'interface, même si la page a été quittée entre-temps.
fn brancher_activation(fenetre: &FenetrePrincipale, etat: &EtatPartage) {
    let (fa, e) = (fenetre.as_weak(), etat.clone());
    fenetre
        .global::<ActivationUi>()
        .on_activer(move |code, client| {
            let Some(f) = fa.upgrade() else {
                return;
            };
            let ui = f.global::<ActivationUi>();
            let (poste, client, en_session) = {
                let etat = e.borrow();
                let client = if client.is_empty() {
                    kadenz_stockage::entreprise::lire(&etat.conn)
                        .ok()
                        .flatten()
                        .map(|en| en.nom)
                        .unwrap_or_default()
                } else {
                    client.to_string()
                };
                (etat.poste.clone(), client, etat.utilisateur.is_some())
            };
            let en_cours = {
                let etat = e.borrow();
                stockage::etat(&etat.conn, &poste).unwrap_or(EtatLicence::Absente)
            };
            ui.set_message(SharedString::new());
            ui.set_etat(1);
            let (code, fa) = (code.to_string(), fa.clone());
            std::thread::spawn(move || {
                let resultat = crate::activation_ui::activer(&code, &poste, &client);
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(f) = fa.upgrade() else {
                        return;
                    };
                    let ui = f.global::<ActivationUi>();
                    match resultat {
                        Ok((_, licence))
                            if en_session && recul_d_echeance(&en_cours, &licence).is_some() =>
                        {
                            let refus = recul_d_echeance(&en_cours, &licence).unwrap_or_default();
                            ui.set_message(refus.into());
                            ui.set_etat(3);
                        }
                        Ok((texte, licence)) if en_session => {
                            ui.set_licence(texte.clone().into());
                            ui.set_resume(resume(&licence).into());
                            let retour = f.global::<LicenceApi>().invoke_enregistrer(texte.into());
                            ui.set_message(if retour.ok {
                                crate::phrase(tr("licence activée et enregistrée sur ce poste"))
                            } else {
                                retour.message
                            });
                            ui.set_etat(if retour.ok { 2 } else { 3 });
                        }
                        Ok((texte, licence)) => {
                            ui.set_licence(texte.into());
                            ui.set_resume(resume(&licence).into());
                            ui.set_etat(2);
                        }
                        Err(m) => {
                            ui.set_message(crate::activation_ui::message(&m).into());
                            ui.set_etat(3);
                        }
                    }
                });
            });
        });
}

/// Enregistre la clé saisie dans l'assistant, une fois l'administrateur créé. Un échec
/// laisse l'installation faite : la clé se saisit alors dans l'onglet Licence.
pub(crate) fn enregistrer_cle_assistant(
    f: &FenetrePrincipale,
    conn: &kadenz_stockage::Connexion,
    admin: &kadenz_stockage::comptes::Utilisateur,
    cle: &str,
    poste: &str,
) {
    if !cle.trim().is_empty()
        && let Ok(etat) = stockage::enregistrer(conn, admin, cle, poste)
    {
        afficher(f, conn, &etat);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Libellés des modules et messages de la licence : six traductions chacun.
    #[test]
    fn textes_du_domaine_traduits() {
        let mut textes: Vec<&str> = Module::TOUS.iter().map(|m| m.libelle()).collect();
        textes.extend(stockage::modeles_erreur());
        let manquants = crate::traduction::sans_traduction(&textes);
        assert!(manquants.is_empty(), "sans traduction : {manquants:?}");
    }

    /// Une licence reçue ne recule pas l'échéance d'une licence en cours
    /// valide ; une licence échue ou absente n'est pas protégée.
    #[test]
    fn licence_recue_plus_courte_refusee() {
        crate::traduction::choisir("fr");
        let licence = |echeance: Option<&str>| Licence {
            client: "Quincaillerie Essai".into(),
            poste: "6VBZ-Z2E3-KFW9-N6QX".into(),
            modules: vec![Module::Devis],
            emise_le: "2026-11-19".into(),
            echeance: echeance.map(str::to_string),
            postes: 1,
        };
        let annuelle = EtatLicence::Valide(licence(Some("2027-11-18")), Some(364));
        let mensuelle = licence(Some("2026-12-18"));
        assert_eq!(
            recul_d_echeance(&annuelle, &mensuelle).as_deref(),
            Some(
                "Ce code mènerait votre licence au 18/12/2026, avant son échéance actuelle du 18/11/2027 : licence gardée."
            )
        );
        // Échéance égale ou plus tardive : acceptée.
        assert_eq!(recul_d_echeance(&annuelle, &licence(Some("2027-11-18"))), None);
        assert_eq!(recul_d_echeance(&annuelle, &licence(Some("2028-11-18"))), None);
        // Licence en cours échue ou absente : jamais protégée.
        let echue = EtatLicence::Expiree(licence(Some("2026-11-01")));
        assert_eq!(recul_d_echeance(&echue, &mensuelle), None);
        assert_eq!(recul_d_echeance(&EtatLicence::Absente, &mensuelle), None);
        // Licence perpétuelle en cours : hors de la règle (aucune échéance).
        let perpetuelle = EtatLicence::Valide(licence(None), None);
        assert_eq!(recul_d_echeance(&perpetuelle, &mensuelle), None);
    }

    /// Textes de l'échéance et de l'activation construits ici (0.2.3).
    #[test]
    fn textes_de_l_echeance_traduits() {
        let textes = [
            "Perpétuelle",
            "{}, valable jusqu'au {}, modules : {}",
            "La licence arrive à échéance dans {} jours, le {}. Renouvelez-la avec un nouveau code d'activation pour garder les modules.",
            "La licence arrive à échéance le {}. Renouvelez-la dès maintenant avec un nouveau code d'activation pour garder les modules.",
            "La licence est échue depuis le {} : seul le socle est disponible, les données des modules sont gardées. Renouvelez-la avec un nouveau code d'activation.",
            "Aucune licence : socle seul",
            "Licence échue le {} : socle seul",
            "Clé enregistrée, mais la licence est échue depuis le {} : seul le socle est disponible.",
            "licence activée et enregistrée sur ce poste",
            "ce code mènerait votre licence au {}, avant son échéance actuelle du {} : licence gardée",
        ];
        let manquants = crate::traduction::sans_traduction(&textes);
        assert!(manquants.is_empty(), "sans traduction : {manquants:?}");
    }
}
