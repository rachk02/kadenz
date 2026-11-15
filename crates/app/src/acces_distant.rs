//! Accès à distance (phase 2, F8 ; écrans, section 9 ; décisions 16, 57 à
//! 59) : rubrique des Paramètres réservée à l'Admin d'un poste relié. L'état
//! se lit et se règle sur le serveur, hors du fil de l'interface ; la page le
//! relit à l'ouverture, après chaque action et toutes les 15 secondes.

use kadenz_poste::{ErreurPoste, EtatAccesDistant, EtatConnexion, MessageRelais, Poignee};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::referentiels::{retour_erreur, retour_ok};
use crate::traduction::{tr, trf};
use crate::{
    AccesDistantApi, EtatPartage, FenetrePrincipale, PosteDistantUi, ReglagesApi, Rubrique,
};

/// Poignée du synchroniseur quand le poste est connecté au serveur ; `None`
/// hors connexion ou sur un poste non relié.
fn poignee(etat: &EtatPartage) -> Option<Poignee> {
    let e = etat.borrow();
    let r = e.relie.as_ref()?;
    connecte(&r.connexion).then(|| r.synchro.poignee())
}

fn connecte(connexion: &EtatConnexion) -> bool {
    matches!(
        connexion,
        EtatConnexion::Connecte
            | EtatConnexion::Synchronisation
            | EtatConnexion::LectureSeule { .. }
    )
}

/// Date du serveur AAAA-MM-JJ remise en JJ/MM/AAAA ; tout autre texte est
/// gardé tel quel.
fn date_lisible(valeur: &str) -> String {
    chrono::NaiveDate::parse_from_str(valeur, "%Y-%m-%d")
        .map(|d| d.format("%d/%m/%Y").to_string())
        .unwrap_or_else(|_| valeur.to_string())
}

/// Heure de la dernière connexion distante, AAAA-MM-JJTHH:MM dans le fuseau
/// de l'entreprise, en JJ/MM/AAAA HH:MM.
fn heure_lisible(valeur: &str) -> String {
    chrono::NaiveDateTime::parse_from_str(valeur, "%Y-%m-%dT%H:%M")
        .map(|d| d.format("%d/%m/%Y %H:%M").to_string())
        .unwrap_or_else(|_| valeur.to_string())
}

/// Message du relais traduit ; les dates de ses valeurs sont remises en
/// JJ/MM/AAAA.
fn texte_relais(m: &MessageRelais) -> String {
    let valeurs: Vec<String> = m.valeurs.iter().map(|v| date_lisible(v)).collect();
    let texte = trf(
        &m.modele,
        &valeurs
            .iter()
            .map(|v| v as &dyn std::fmt::Display)
            .collect::<Vec<_>>(),
    );
    // Le modèle est un fragment (« relais injoignable : ... ») : seul dans
    // l'encadré, il prend une majuscule.
    let mut lettres = texte.chars();
    match lettres.next() {
        Some(premiere) => premiere.to_uppercase().chain(lettres).collect(),
        None => texte,
    }
}

/// Recopie un état reçu du serveur dans la page.
fn montrer(f: &FenetrePrincipale, etat: &EtatAccesDistant) {
    let api = f.global::<AccesDistantApi>();
    api.set_charge(true);
    api.set_indisponible(SharedString::new());
    api.set_actif(etat.actif);
    api.set_relais(etat.relais.clone().unwrap_or_default().into());
    api.set_joignable(etat.joignable);
    api.set_erreur(
        etat.erreur
            .as_ref()
            .map(texte_relais)
            .unwrap_or_default()
            .into(),
    );
    api.set_derniere(
        etat.derniere_distante
            .as_deref()
            .map(heure_lisible)
            .unwrap_or_default()
            .into(),
    );
    let postes: Vec<PosteDistantUi> = etat
        .postes
        .iter()
        .map(|p| PosteDistantUi {
            empreinte: p.empreinte.clone().into(),
            nom: p.nom.clone().into(),
            autorise: p.autorise,
        })
        .collect();
    api.set_postes(ModelRc::new(VecModel::from(postes)));
}

/// Résultat d'une demande au serveur. `action` : vrai pour un réglage, dont
/// l'échec s'affiche sous la page ; une simple lecture qui échoue garde
/// l'état déjà affiché, ou montre pourquoi rien ne se lit.
fn recevoir(f: &FenetrePrincipale, resultat: Result<EtatAccesDistant, ErreurPoste>, action: bool) {
    let api = f.global::<AccesDistantApi>();
    api.set_occupe(false);
    match resultat {
        Ok(etat) => {
            montrer(f, &etat);
            if action {
                api.set_retour(retour_ok(String::new()));
            }
        }
        Err(e) => {
            let texte = crate::relie::texte_erreur(&e);
            if action {
                api.set_retour(retour_erreur(trf(
                    "Réglage non enregistré : {}.",
                    &[&texte],
                )));
                // Les cases reviennent à l'état du serveur.
                actualiser_depuis(f);
            } else if !api.get_charge() {
                api.set_indisponible(trf("État illisible : {}.", &[&texte]).into());
            }
        }
    }
}

/// Lance un appel au serveur dans un fil ; son résultat revient dans la page.
fn demander(
    f: &FenetrePrincipale,
    e: &EtatPartage,
    action: bool,
    appel: impl FnOnce(&Poignee) -> Result<EtatAccesDistant, ErreurPoste> + Send + 'static,
) {
    let api = f.global::<AccesDistantApi>();
    let Some(p) = poignee(e) else {
        api.set_connecte(false);
        api.set_occupe(false);
        if action || !api.get_charge() {
            api.set_charge(false);
            api.set_indisponible(
                tr("Disponible seulement quand le poste est connecté au serveur.").into(),
            );
        }
        return;
    };
    api.set_connecte(true);
    api.set_occupe(true);
    if action {
        api.set_retour(retour_ok(String::new()));
    }
    let fa = f.as_weak();
    std::thread::spawn(move || {
        let resultat = appel(&p);
        let _ = fa.upgrade_in_event_loop(move |f| recevoir(&f, resultat, action));
    });
}

/// Relit l'état après un réglage refusé : passe par le rappel de la page,
/// enregistré sur le fil de l'interface.
fn actualiser_depuis(f: &FenetrePrincipale) {
    f.global::<AccesDistantApi>().invoke_actualiser();
}

pub(crate) fn brancher(fenetre: &FenetrePrincipale, etat: &EtatPartage) {
    let api = fenetre.global::<AccesDistantApi>();

    let (fa, e) = (fenetre.as_weak(), etat.clone());
    api.on_actualiser(move || {
        let Some(f) = fa.upgrade() else { return };
        if f.global::<AccesDistantApi>().get_occupe() {
            return;
        }
        demander(&f, &e, false, |p| p.acces_distant());
    });

    let (fa, e) = (fenetre.as_weak(), etat.clone());
    api.on_regler(move |actif| {
        let Some(f) = fa.upgrade() else { return };
        demander(&f, &e, true, move |p| p.regler_acces_distant(actif));
    });

    let (fa, e) = (fenetre.as_weak(), etat.clone());
    api.on_autoriser(move |empreinte, autorise| {
        let Some(f) = fa.upgrade() else { return };
        let empreinte = empreinte.to_string();
        demander(&f, &e, true, move |p| {
            p.autoriser_distance(&empreinte, autorise)
        });
    });
}

/// L'état de la connexion a changé : la page ouverte se relit (le retour du
/// serveur la remplit, une coupure la vide).
pub(crate) fn connexion_changee(f: &FenetrePrincipale, e: &EtatPartage) {
    let connecte = e
        .borrow()
        .relie
        .as_ref()
        .is_some_and(|r| connecte(&r.connexion));
    let api = f.global::<AccesDistantApi>();
    api.set_connecte(connecte);
    if f.global::<ReglagesApi>().get_rubrique() == Rubrique::AccesDistant && !api.get_occupe() {
        if connecte {
            api.invoke_actualiser();
        } else {
            api.set_charge(false);
            api.set_indisponible(
                tr("Disponible seulement quand le poste est connecté au serveur.").into(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_et_heures_lisibles() {
        assert_eq!(date_lisible("2026-11-15"), "15/11/2026");
        assert_eq!(date_lisible("0.5"), "0.5");
        assert_eq!(heure_lisible("2026-11-15T07:32"), "15/11/2026 07:32");
        assert_eq!(heure_lisible("hier"), "hier");
    }

    #[test]
    fn message_du_relais_avec_majuscule_et_date_lisible() {
        let m = MessageRelais {
            modele: "abonnement échu le {} : l'accès à distance demande un abonnement actif".into(),
            valeurs: vec!["2026-10-31".into()],
        };
        assert_eq!(
            texte_relais(&m),
            "Abonnement échu le 31/10/2026 : l'accès à distance demande un abonnement actif"
        );
    }
}
