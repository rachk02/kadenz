//! Licence du poste (jalon J9) : identifiant du poste, état de la licence, modules
//! activés et masquage des écrans des modules désactivés.

use kadenz_domaine::document::TypeDocument;
use kadenz_domaine::message::MessageCompose;
use kadenz_domaine::module::Module;
use kadenz_stockage::licence::{self as stockage, EtatLicence};
use slint::{ComponentHandle, ModelRc, VecModel};

use crate::referentiels::{retour_erreur, retour_ok, texte_erreur};
use crate::traduction::{tr, trf};
use crate::{
    EtatPartage, FenetrePrincipale, LicenceApi, ModuleUi, Modules, Page, Retour, VerificationUi,
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
    let (texte, valide, client, emise_le) = match etat {
        EtatLicence::Absente => (
            tr("Aucune clé : socle seul"),
            false,
            String::new(),
            String::new(),
        ),
        EtatLicence::Invalide(m) => (
            trf("Clé invalide, socle seul : {}", &[&message(m.clone())]),
            false,
            String::new(),
            String::new(),
        ),
        EtatLicence::Valide(l) => (
            tr("Licence valide"),
            true,
            l.client.clone(),
            l.emise_le.clone(),
        ),
    };
    api.set_etat(texte.into());
    api.set_valide(valide);
    api.set_client(client.into());
    api.set_emise_le(emise_le.into());
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
                    EtatLicence::Valide(_) => retour_ok(tr("Clé enregistrée.")),
                    EtatLicence::Invalide(m) => retour_erreur(message(m)),
                    EtatLicence::Absente => retour_erreur(tr("Aucune clé : socle seul")),
                }
            }
            Err(erreur) => retour_erreur(texte_erreur(erreur)),
        }
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
}
