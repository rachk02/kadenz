//! Réglages d'impression (jalon J8b) : gabarit, marges, en-tête et pied de page
//! en contenu libre, aperçu en direct et feuille de calibrage.

use kadenz_domaine::document::TypeDocument;
use kadenz_domaine::mise_en_page::{
    self, BasDeDocument, Gabarit, LigneTotal, Marges, MiseEnPage, StylePied, Variable,
};
use kadenz_domaine::role::Droit;
use kadenz_stockage::{droits, mise_en_page as reglages};
use slint::{
    ComponentHandle, Image, Model, ModelRc, Rgba8Pixel, SharedPixelBuffer, SharedString, VecModel,
};

use crate::paiements::enregistrer_pdf;
use crate::referentiels::{retour_erreur, retour_ok, texte_erreur};
use crate::traduction::tr;
use crate::{BasUi, EtatPartage, FenetrePrincipale, ImpressionApi, MiseEnPageUi, Retour};

/// Largeur de l'aperçu en pixels (le cadre de l'écran fait 420 px, x 1,5 pour la netteté).
const LARGEUR_APERCU: u32 = 630;

fn libelle_gabarit(g: Gabarit) -> String {
    match g {
        Gabarit::Classique => tr("Classique"),
        Gabarit::Compact => tr("Compact"),
        Gabarit::PreImprime => tr("Papier pré-imprimé"),
    }
}

fn vers_ecran(m: &MiseEnPage) -> MiseEnPageUi {
    MiseEnPageUi {
        gabarit: Gabarit::TOUS
            .iter()
            .position(|g| *g == m.gabarit)
            .unwrap_or(0) as i32,
        haut: m.marges.haut.to_string().into(),
        bas: m.marges.bas.to_string().into(),
        gauche: m.marges.gauche.to_string().into(),
        droite: m.marges.droite.to_string().into(),
        entete: m.entete.as_str().into(),
        pied: m.pied.as_str().into(),
        style_pied: StylePied::TOUS
            .iter()
            .position(|st| *st == m.style_pied)
            .unwrap_or(0) as i32,
    }
}

fn type_au_rang(rang: i32) -> TypeDocument {
    usize::try_from(rang)
        .ok()
        .and_then(|r| TypeDocument::TOUS.get(r).copied())
        .unwrap_or(TypeDocument::Facture)
}

fn bas_vers_ecran(b: &BasDeDocument) -> BasUi {
    BasUi {
        ht: b.imprime(LigneTotal::Ht),
        remise: b.imprime(LigneTotal::Remise),
        base_tva: b.imprime(LigneTotal::BaseTva),
        tva: b.imprime(LigneTotal::Tva),
        retenue: b.imprime(LigneTotal::Retenue),
        ttc: b.imprime(LigneTotal::Ttc),
        masquer_zero: b.masquer_zero,
        montant_en_lettres: b.montant_en_lettres,
        visa_client: b.visa_client,
        visa_fournisseur: b.visa_fournisseur,
        date_reception: b.date_reception,
        date_livraison: b.date_livraison,
        signataire: b.signataire,
    }
}

fn bas_depuis_ecran(b: &BasUi) -> BasDeDocument {
    let cochees = [b.ht, b.remise, b.base_tva, b.tva, b.retenue, b.ttc];
    let mut bas = BasDeDocument {
        totaux: LigneTotal::TOUTES
            .into_iter()
            .zip(cochees)
            .filter_map(|(l, c)| c.then_some(l))
            .collect(),
        masquer_zero: b.masquer_zero,
        montant_en_lettres: b.montant_en_lettres,
        visa_client: b.visa_client,
        visa_fournisseur: b.visa_fournisseur,
        date_reception: b.date_reception,
        date_livraison: b.date_livraison,
        signataire: b.signataire,
    };
    bas.normaliser();
    bas
}

fn modele_bas(bas: impl Fn(TypeDocument) -> BasDeDocument) -> ModelRc<BasUi> {
    ModelRc::new(VecModel::from(
        TypeDocument::TOUS
            .iter()
            .map(|t| bas_vers_ecran(&bas(*t)))
            .collect::<Vec<_>>(),
    ))
}

/// Une marge vide vaut 0 ; un texte qui n'est pas un nombre entier est refusé.
fn marge(texte: &str) -> Option<u8> {
    let t = texte.trim();
    if t.is_empty() {
        return Some(0);
    }
    t.parse::<u8>().ok()
}

/// Saisie de l'écran, ou le message à afficher sous les marges.
fn depuis_ecran(m: &MiseEnPageUi) -> Result<MiseEnPage, String> {
    let lire = |t: &SharedString| {
        marge(t).ok_or_else(|| tr("Chaque marge est un nombre entier de millimètres."))
    };
    Ok(MiseEnPage {
        gabarit: usize::try_from(m.gabarit)
            .ok()
            .and_then(|r| Gabarit::TOUS.get(r).copied())
            .unwrap_or_default(),
        marges: Marges {
            haut: lire(&m.haut)?,
            bas: lire(&m.bas)?,
            gauche: lire(&m.gauche)?,
            droite: lire(&m.droite)?,
        },
        entete: m.entete.to_string(),
        pied: m.pied.to_string(),
        style_pied: usize::try_from(m.style_pied)
            .ok()
            .and_then(|r| StylePied::TOUS.get(r).copied())
            .unwrap_or_default(),
    })
}

fn message(e: kadenz_domaine::message::MessageCompose) -> String {
    texte_erreur(kadenz_stockage::Erreur::from(e))
}

/// Vérifie la saisie zone par zone, affiche les messages, puis recalcule
/// l'aperçu ; une saisie fautive garde le dernier aperçu correct.
fn actualiser(
    f: &FenetrePrincipale,
    etat: &EtatPartage,
    m: &MiseEnPageUi,
    bas: &BasUi,
    rang_type: i32,
) {
    let api = f.global::<ImpressionApi>();
    let mise_en_page = match depuis_ecran(m) {
        Ok(mp) => {
            api.set_erreur_marges(
                mp.marges
                    .verifier()
                    .err()
                    .map(message)
                    .unwrap_or_default()
                    .into(),
            );
            Some(mp)
        }
        Err(texte) => {
            api.set_erreur_marges(texte.into());
            None
        }
    };
    let erreur_zone = |markup: &str| {
        mise_en_page::analyser(markup)
            .err()
            .map(message)
            .unwrap_or_default()
    };
    api.set_erreur_entete(erreur_zone(&m.entete).into());
    api.set_erreur_pied(erreur_zone(&m.pied).into());
    let Some(mp) = mise_en_page.filter(|mp| mp.analyser().is_ok()) else {
        return;
    };
    let etat = etat.borrow();
    let bas = bas_depuis_ecran(bas);
    match crate::pdf::apercu_mise_en_page(
        &etat.conn,
        &mp,
        &bas,
        type_au_rang(rang_type),
        LARGEUR_APERCU,
    ) {
        Ok((rgba, largeur, hauteur)) => {
            let image = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&rgba, largeur, hauteur);
            api.set_apercu(Image::from_rgba8(image));
        }
        Err(erreur) => api.set_erreur_marges(erreur.into()),
    }
}

/// Types proposés dans « Bas des documents » : ceux que `actif` retient (modules de
/// la licence), avec leur rang dans l'ordre des types du domaine.
pub(crate) fn appliquer_types_bas(
    fenetre: &FenetrePrincipale,
    actif: impl Fn(TypeDocument) -> bool,
) {
    let (libelles, rangs): (Vec<SharedString>, Vec<i32>) = TypeDocument::TOUS
        .iter()
        .enumerate()
        .filter(|(_, t)| actif(**t))
        .map(|(rang, t)| (SharedString::from(tr(&t.to_string())), rang as i32))
        .unzip();
    let api = fenetre.global::<ImpressionApi>();
    api.set_types_bas(ModelRc::new(VecModel::from(libelles)));
    api.set_rangs_bas(ModelRc::new(VecModel::from(rangs)));
}

pub(crate) fn brancher(fenetre: &FenetrePrincipale, etat: &EtatPartage) {
    let api = fenetre.global::<ImpressionApi>();
    api.set_gabarits(ModelRc::new(VecModel::from(
        Gabarit::TOUS
            .iter()
            .map(|g| SharedString::from(libelle_gabarit(*g)))
            .collect::<Vec<_>>(),
    )));
    api.set_variables(ModelRc::new(VecModel::from(
        Variable::TOUTES
            .iter()
            .map(|v| SharedString::from(format!("{} ({{{}}})", tr(v.libelle()), v.nom())))
            .collect::<Vec<_>>(),
    )));
    api.set_codes_variables(ModelRc::new(VecModel::from(
        Variable::TOUTES
            .iter()
            .map(|v| SharedString::from(format!("{{{}}}", v.nom())))
            .collect::<Vec<_>>(),
    )));

    let e = etat.clone();
    api.on_ouvrir(move || {
        let etat = e.borrow();
        vers_ecran(&reglages::lire(&etat.conn).unwrap_or_default())
    });

    api.on_par_defaut(|| vers_ecran(&MiseEnPage::default()));

    let e = etat.clone();
    api.on_ouvrir_bas(move || {
        let etat = e.borrow();
        modele_bas(|t| {
            reglages::lire_bas(&etat.conn, t).unwrap_or_else(|_| BasDeDocument::par_defaut(t))
        })
    });

    api.on_bas_par_defaut(|| modele_bas(BasDeDocument::par_defaut));

    let (f, e) = (fenetre.as_weak(), etat.clone());
    api.on_actualiser(move |m, bas, rang_type| {
        if let Some(f) = f.upgrade() {
            actualiser(&f, &e, &m, &bas, rang_type);
        }
    });

    let e = etat.clone();
    api.on_enregistrer(move |m, bas| -> Retour {
        let mise_en_page = match depuis_ecran(&m) {
            Ok(mp) => mp,
            Err(texte) => return retour_erreur(texte),
        };
        let etat = e.borrow();
        let Some(acteur) = &etat.utilisateur else {
            return retour_erreur(tr("aucune session ouverte"));
        };
        if let Err(erreur) = reglages::enregistrer(&etat.conn, acteur, &mise_en_page) {
            return retour_erreur(texte_erreur(erreur));
        }
        for (t, b) in TypeDocument::TOUS.iter().zip(bas.iter()) {
            if let Err(erreur) =
                reglages::enregistrer_bas(&etat.conn, acteur, *t, &bas_depuis_ecran(&b))
            {
                return retour_erreur(texte_erreur(erreur));
            }
        }
        retour_ok(tr("Mise en page enregistrée."))
    });

    let e = etat.clone();
    api.on_tester(move |m| -> Retour {
        let mise_en_page = match depuis_ecran(&m) {
            Ok(mp) => mp,
            Err(texte) => return retour_erreur(texte),
        };
        if let Err(erreur) = mise_en_page.marges.verifier() {
            return retour_erreur(message(erreur));
        }
        let autorise = {
            let etat = e.borrow();
            match &etat.utilisateur {
                Some(u) => droits::possede(&etat.conn, u, Droit::GererParametres).unwrap_or(false),
                None => false,
            }
        };
        if !autorise {
            return retour_erreur(tr("Votre compte n'a pas le droit de gérer les paramètres."));
        }
        match crate::pdf::feuille_calibrage(&mise_en_page.marges) {
            Ok(octets) => enregistrer_pdf(&octets, "calibrage.pdf", None),
            Err(erreur) => retour_erreur(erreur),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bas_aller_retour() {
        for t in TypeDocument::TOUS {
            let bas = BasDeDocument::par_defaut(t);
            assert_eq!(bas_depuis_ecran(&bas_vers_ecran(&bas)), bas, "{t:?}");
        }
    }

    #[test]
    fn marges_saisies() {
        assert_eq!(marge(""), Some(0));
        assert_eq!(marge(" 12 "), Some(12));
        assert_eq!(marge("1,5"), None);
        assert_eq!(marge("-3"), None);
    }

    /// Les libellés et les messages venus du domaine ont leurs six traductions.
    #[test]
    fn textes_du_domaine_traduits() {
        let mut textes: Vec<&str> = Variable::TOUTES.iter().map(|v| v.libelle()).collect();
        textes.extend(mise_en_page::MODELES_ERREUR.iter().copied());
        let manquants = crate::traduction::sans_traduction(&textes);
        assert!(manquants.is_empty(), "sans traduction : {manquants:?}");
    }
}
