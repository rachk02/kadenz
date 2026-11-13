//! Phase 2, jalon F1 : application des opérations des postes et suite des
//! changements (`docs/spec-synchronisation.md`, sections 3 à 6).
//!
//! Une opération est appliquée par les fonctions habituelles du stockage, avec
//! leurs droits et leurs règles, dans un point de sauvegarde qui contient aussi
//! l'enregistrement de son résultat : l'effet et le résultat sont écrits
//! ensemble ou pas du tout. Une opération renvoyée (même identifiant, même
//! contenu) reçoit le résultat gardé, sans être appliquée une seconde fois ;
//! une validation rend donc toujours le même numéro.
//!
//! Le serveur applique les opérations une à une, sur une seule connexion.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::NaiveDate;
use kadenz_domaine::calcul::Taux;
use kadenz_domaine::document::TypeDocument;
use kadenz_domaine::langue::Langue;
use kadenz_domaine::referentiel::{FicheProduit, FicheTiers, Sens, TypeTiers};
use kadenz_domaine::role::Droit;
use kadenz_domaine::role::Role;
use kadenz_protocole::{
    self as proto, Action, Bilan, Changement, EnCoupure, Entite, Etat, Issue, Motif, Operation,
    SiChangee, Suspendue,
};
use rusqlite::{Connection, OptionalExtension, params};

use crate::comptes::{self, Navigation, NouvelUtilisateur, Utilisateur};
use crate::documents::{self, ChampsLivraison, LigneSaisie, Saisie};
use crate::entreprise::{self, Entreprise, Logo};
use crate::reglements::{self, SaisieReglement};
use crate::{
    BilanImport, Erreur, Resultat, Retrait, droits, maintenant, mise_en_page, parametres, produits,
    relances, replique, tiers,
};

/// Tables dont les lignes portent un identifiant global.
const TABLES: [&str; 6] = [
    "utilisateur",
    "tiers",
    "produit",
    "document",
    "reglement",
    "instantane_entreprise",
];

const CLE_ENTREPRISE: &str = "identifiant_entreprise";

/// Taille au-delà de laquelle une page de changements s'arrête (JSON des
/// changements), sous la taille maximale d'un message : deux logos de 2 Mo en
/// base64 tiennent dans un seul changement. Une page porte toujours au moins
/// un changement.
pub const BUDGET_PAGE: usize = 6 * 1024 * 1024;

/// Modèle du refus pour un droit, avec le droit en valeur (même phrase que
/// l'affichage de `Erreur::Interdit`).
pub const MODELE_INTERDIT: &str = "action non autorisée pour votre rôle : « {} »";

/// Textes fixes que le serveur peut rendre dans `Issue::Refusee`, en plus des
/// messages habituels du stockage : à traduire sur les postes.
pub const MODELES_ERREUR: [&str; 4] = [
    MODELE_INTERDIT,
    "élément inconnu sur le serveur : {}",
    "compte inconnu sur le serveur",
    "opération illisible",
];

/// Résultat rendu à l'opération d'origine quand son conflit est tranché en
/// faveur de la version du serveur (F6).
pub const MODELE_CONFLIT_SERVEUR: &str = "conflit tranché : la version du serveur est gardée";

/// Résultat rendu à une opération en attente rejetée par l'Admin (F6).
pub const MODELE_REJETEE: &str = "opération rejetée par l'Admin";

/// Textes des décisions sur les opérations suspendues (F6) : à traduire sur
/// les postes, comme [`MODELES_ERREUR`].
pub const MODELES_DECISION: [&str; 4] = [
    MODELE_CONFLIT_SERVEUR,
    MODELE_REJETEE,
    "opération suspendue introuvable ou déjà décidée",
    "le résultat ne correspond pas à l'opération en conflit",
];

/// Nouvel identifiant global (UUID version 7).
pub fn nouvel_uid() -> String {
    uuid::Uuid::now_v7().to_string()
}

/// À chaque ouverture : identifiant de l'entreprise créé s'il manque, et
/// identifiant global donné aux lignes qui n'en ont pas encore (base V1
/// migrée, lignes créées sur la machine même).
pub(crate) fn preparer(conn: &Connection) -> Resultat<()> {
    conn.execute(
        "INSERT INTO parametre_systeme (cle, valeur) VALUES (?1, json_quote(?2))
         ON CONFLICT (cle) DO NOTHING",
        params![CLE_ENTREPRISE, nouvel_uid()],
    )?;
    completer_uids(conn)
}

/// Identifiant global donné aux lignes qui n'en ont pas (le serveur l'appelle
/// aussi pour un compte créé depuis l'ouverture de la base).
pub fn completer_uids(conn: &Connection) -> Resultat<()> {
    for table in TABLES {
        let ids: Vec<i64> = conn
            .prepare(&format!("SELECT id FROM {table} WHERE uid IS NULL"))?
            .query_map([], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        for id in ids {
            conn.execute(
                &format!("UPDATE {table} SET uid = ?1 WHERE id = ?2"),
                params![nouvel_uid(), id],
            )?;
        }
    }
    Ok(())
}

/// Identifiant de l'entreprise de cette base, tiré à sa création.
pub fn identifiant_entreprise(conn: &Connection) -> Resultat<String> {
    Ok(conn.query_row(
        "SELECT valeur ->> '$' FROM parametre_systeme WHERE cle = ?1",
        [CLE_ENTREPRISE],
        |r| r.get(0),
    )?)
}

/// Identifiant global d'une ligne (le code de l'entité pour une entité d'une
/// seule ligne).
pub fn uid_de(conn: &Connection, entite: Entite, id: i64) -> Resultat<Option<String>> {
    if entite.unique() {
        return Ok(Some(entite.code().to_string()));
    }
    let Some(table) = replique::nom_table(entite) else {
        return Ok(None);
    };
    Ok(conn
        .query_row(
            &format!("SELECT uid FROM {table} WHERE id = ?1"),
            [id],
            |r| r.get(0),
        )
        .optional()?
        .flatten())
}

/// Identifiant propre à cette base d'une ligne désignée par son identifiant global.
pub fn id_de(conn: &Connection, entite: Entite, uid: &str) -> Resultat<Option<i64>> {
    if entite.unique() {
        return Ok(Some(1));
    }
    let Some(table) = replique::nom_table(entite) else {
        return Ok(None);
    };
    Ok(conn
        .query_row(
            &format!("SELECT id FROM {table} WHERE uid = ?1"),
            [uid],
            |r| r.get(0),
        )
        .optional()?)
}

/// Rang du dernier changement de la base (0 si aucun).
pub fn dernier_changement(conn: &Connection) -> Resultat<u64> {
    Ok(
        conn.query_row("SELECT coalesce(max(seq), 0) FROM changement", [], |r| {
            r.get::<_, i64>(0)
        })? as u64,
    )
}

/// Applique une opération reçue d'un poste et rend son résultat. Une erreur
/// (base, fichier) n'est pas un résultat : rien n'est écrit, et le poste
/// renverra l'opération.
pub fn appliquer(conn: &mut Connection, op: &Operation) -> Resultat<Issue> {
    // Copie gardée : sans mot de passe ni contenu de logo. Le renvoi d'une
    // opération est reconnu sur cette copie.
    let gardee = Operation {
        action: op.action.masquee(),
        ..op.clone()
    };
    let contenu = serde_json::to_string(&gardee).map_err(|e| Erreur::Validation(e.to_string()))?;
    let deja: Option<(String, String)> = conn
        .query_row(
            "SELECT contenu, issue FROM operation WHERE id = ?1",
            [&op.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if let Some((contenu_garde, issue)) = deja {
        if contenu_garde != contenu {
            return Err(refus_op(
                "identifiant d'opération déjà utilisé pour une autre opération",
            ));
        }
        return serde_json::from_str(&issue).map_err(|e| Erreur::Validation(e.to_string()));
    }
    if op.entreprise != identifiant_entreprise(conn)? {
        return Err(refus_op("opération destinée à une autre entreprise"));
    }
    // Seules les écritures permises hors connexion peuvent en venir (décision
    // 50) : la copie gardée d'une opération en attente, rejouée si l'Admin
    // l'accepte, ne contient alors aucun secret masqué.
    if op.hors_connexion && op.action.en_coupure() != EnCoupure::Permise {
        return Err(refus_op("écriture impossible hors connexion"));
    }
    // Lecture seule (abonnement) : après le renvoi d'une opération déjà
    // acceptée, qui garde son résultat, et avant tout effet. Seules passent
    // les écritures du compte sur lui-même (mot de passe, langue, affichage).
    if !op.action.personnelle() {
        crate::abonnement::exiger_ecriture(conn)?;
    }

    conn.execute_batch("SAVEPOINT operation")?;
    let resultat = appliquer_dans(conn, op, &contenu);
    match resultat {
        Ok(issue) => {
            conn.execute_batch("RELEASE operation")?;
            Ok(issue)
        }
        Err(e) => {
            conn.execute_batch("ROLLBACK TO operation; RELEASE operation")?;
            Err(e)
        }
    }
}

fn appliquer_dans(conn: &mut Connection, op: &Operation, contenu: &str) -> Resultat<Issue> {
    let cible = op.action.cible();
    let cle_cible = cible.map(|(_, uid)| uid).unwrap_or_default();
    // Une opération du même poste sur la même ligne attend une décision : celle-ci
    // attend aussi, pour garder l'ordre des actions du poste. Rien n'est gardé, le
    // poste la renverra ; les autres postes ne sont pas bloqués.
    let suspendue = cible.is_some()
        && conn
            .query_row(
                "SELECT 1 FROM operation_suspendue s JOIN operation o ON o.id = s.operation_id
                 WHERE s.cible = ?1 AND s.ouverte = 1 AND o.poste = ?2",
                params![cle_cible, op.poste],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
    if suspendue {
        return Ok(Issue::Bloquee);
    }
    let changee = match (cible, op.action.si_changee()) {
        (Some((entite, uid)), SiChangee::Conflit | SiChangee::Depassee) => {
            modifiee_ailleurs(conn, op, entite, uid)?
        }
        _ => false,
    };
    let issue = if changee {
        match op.action.si_changee() {
            SiChangee::Depassee => Issue::Depassee,
            _ => Issue::EnConflit,
        }
    } else {
        conn.execute(
            "UPDATE synchro_contexte SET poste = ?1 WHERE id = 1",
            [&op.poste],
        )?;
        conn.execute_batch("SAVEPOINT effet")?;
        let effet = executer(conn, op).and_then(|issue| {
            completer_uids(conn)?;
            Ok(issue)
        });
        let issue = match effet {
            Ok(issue) => {
                conn.execute_batch("RELEASE effet")?;
                issue
            }
            Err(e) => {
                conn.execute_batch("ROLLBACK TO effet; RELEASE effet")?;
                issue_du_refus(e, op.hors_connexion)?
            }
        };
        conn.execute("UPDATE synchro_contexte SET poste = NULL WHERE id = 1", [])?;
        issue
    };

    let issue_json =
        serde_json::to_string(&issue).map_err(|e| Erreur::Validation(e.to_string()))?;
    conn.execute(
        "INSERT INTO operation (id, poste, compte, contenu, issue, recue)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            op.id,
            op.poste,
            op.compte,
            contenu,
            issue_json,
            maintenant()
        ],
    )?;
    let motif = match issue {
        Issue::EnAttente => Some("attente"),
        Issue::EnConflit => Some("conflit"),
        _ => None,
    };
    if let Some(motif) = motif {
        conn.execute(
            "INSERT INTO operation_suspendue (operation_id, motif, cible) VALUES (?1, ?2, ?3)",
            params![op.id, motif, cle_cible],
        )?;
    }
    Ok(issue)
}

/// Erreur de protocole : jamais enregistrée comme résultat.
fn refus_op(message: &str) -> Erreur {
    Erreur::Validation(message.into())
}

/// Refus d'une règle de la V1 : rendu au poste, ou mis en attente pour un
/// compte ou un droit hors connexion. Les erreurs techniques remontent.
fn issue_du_refus(e: Erreur, hors_connexion: bool) -> Resultat<Issue> {
    Ok(match e {
        Erreur::Interdit(_) if hors_connexion => Issue::EnAttente,
        // Texte fixe et valeur à part, pour que le poste traduise les deux.
        Erreur::Interdit(droit) => Issue::Refusee {
            modele: MODELE_INTERDIT.into(),
            valeurs: vec![droit.to_string()],
        },
        Erreur::ValidationComposee { modele, valeurs } => Issue::Refusee { modele, valeurs },
        Erreur::Sqlite(_)
        | Erreur::Io(_)
        | Erreur::Hachage(_)
        | Erreur::VersionSchemaInconnue { .. } => return Err(e),
        autre => Issue::Refusee {
            modele: autre.to_string(),
            valeurs: Vec::new(),
        },
    })
}

/// La ligne visée a changé depuis la base de l'opération, et pas seulement par
/// ce poste (deux modifications successives d'un même poste hors connexion ne
/// sont pas en conflit entre elles).
fn modifiee_ailleurs(
    conn: &Connection,
    op: &Operation,
    entite: Entite,
    cible: &str,
) -> Resultat<bool> {
    let Some(id) = id_de(conn, entite, cible)? else {
        return Ok(false);
    };
    Ok(conn
        .query_row(
            "SELECT 1 FROM changement
             WHERE entite = ?1 AND entite_id = ?2 AND seq > ?3 AND poste IS NOT ?4
             LIMIT 1",
            params![entite.code(), id, op.base as i64, op.poste],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn inconnu(entite: Entite, uid: &str) -> Erreur {
    Erreur::ValidationComposee {
        modele: "élément inconnu sur le serveur : {}".into(),
        valeurs: vec![format!("{} {uid}", entite.code())],
    }
}

fn exiger_id(conn: &Connection, entite: Entite, uid: &str) -> Resultat<i64> {
    id_de(conn, entite, uid)?.ok_or_else(|| inconnu(entite, uid))
}

fn acteur(conn: &Connection, uid: &str) -> Resultat<Utilisateur> {
    let id = id_de(conn, Entite::Utilisateur, uid)?
        .ok_or_else(|| Erreur::Validation("compte inconnu sur le serveur".into()))?;
    comptes::lire(conn, id)?
        .ok_or_else(|| Erreur::Validation("compte inconnu sur le serveur".into()))
}

/// Pose l'identifiant choisi par le poste sur une ligne qui vient d'être créée.
fn poser_uid(conn: &Connection, entite: Entite, id: i64, uid: &str) -> Resultat<()> {
    let table = replique::nom_table(entite).ok_or_else(illisible)?;
    if id_de(conn, entite, uid)?.is_some() {
        return Err(Erreur::Validation(
            "identifiant déjà attribué à une autre ligne".into(),
        ));
    }
    conn.execute(
        &format!("UPDATE {table} SET uid = ?1 WHERE id = ?2"),
        params![uid, id],
    )?;
    Ok(())
}

/// Pose l'identifiant de chaque fiche importée : les lignes créées après
/// `avant`, dans l'ordre, sont les fiches qui n'ont pas été ignorées.
fn poser_uids_import(
    conn: &Connection,
    entite: Entite,
    avant: i64,
    uids: &[String],
    bilan: &BilanImport,
) -> Resultat<Issue> {
    let table = replique::nom_table(entite).ok_or_else(illisible)?;
    let creees: Vec<i64> = conn
        .prepare(&format!("SELECT id FROM {table} WHERE id > ?1 ORDER BY id"))?
        .query_map([avant], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let ignorees: Vec<usize> = bilan.ignorees.iter().map(|(rang, _)| *rang).collect();
    let retenus = uids
        .iter()
        .enumerate()
        .filter(|(rang, _)| !ignorees.contains(rang))
        .map(|(_, uid)| uid);
    if creees.len() != bilan.ajoutees {
        return Err(illisible());
    }
    for (id, uid) in creees.into_iter().zip(retenus) {
        poser_uid(conn, entite, id, uid)?;
    }
    Ok(Issue::Acceptee {
        numero: None,
        bilan: Some(Bilan::Import {
            ajoutees: bilan.ajoutees as u32,
            ignorees: bilan
                .ignorees
                .iter()
                .map(|(rang, raison)| (*rang as u32, raison.clone()))
                .collect(),
        }),
    })
}

fn dernier_id(conn: &Connection, entite: Entite) -> Resultat<i64> {
    let table = replique::nom_table(entite).ok_or_else(illisible)?;
    Ok(conn.query_row(
        &format!("SELECT coalesce(max(id), 0) FROM {table}"),
        [],
        |r| r.get(0),
    )?)
}

fn retrait(r: Retrait) -> Issue {
    Issue::Acceptee {
        numero: None,
        bilan: Some(Bilan::Retrait {
            supprimee: r == Retrait::Supprimee,
        }),
    }
}

fn executer(conn: &mut Connection, op: &Operation) -> Resultat<Issue> {
    let qui = acteur(conn, &op.compte)?;
    let acceptee = Issue::acceptee();
    let doc = |conn: &Connection, uid: &str| exiger_id(conn, Entite::Document, uid);
    match &op.action {
        // Tiers.
        Action::CreerTiers { uid, fiche } => {
            let t = tiers::creer(conn, &qui, &fiche_tiers(fiche)?)?;
            poser_uid(conn, Entite::Tiers, t.id, uid)?;
            Ok(acceptee)
        }
        Action::ModifierTiers { uid, fiche } => {
            let id = exiger_id(conn, Entite::Tiers, uid)?;
            tiers::modifier(conn, &qui, id, &fiche_tiers(fiche)?)?;
            Ok(acceptee)
        }
        Action::RetirerTiers { uid } => {
            let id = exiger_id(conn, Entite::Tiers, uid)?;
            Ok(retrait(tiers::retirer(conn, &qui, id)?))
        }
        Action::DefinirActifTiers { uid, actif } => {
            let id = exiger_id(conn, Entite::Tiers, uid)?;
            tiers::definir_actif(conn, &qui, id, *actif)?;
            Ok(acceptee)
        }
        Action::ImporterTiers { fiches, uids } => {
            if fiches.len() != uids.len() {
                return Err(illisible());
            }
            let fiches = fiches
                .iter()
                .map(fiche_tiers)
                .collect::<Resultat<Vec<_>>>()?;
            let avant = dernier_id(conn, Entite::Tiers)?;
            let bilan = tiers::importer(conn, &qui, &fiches)?;
            poser_uids_import(conn, Entite::Tiers, avant, uids, &bilan)
        }

        // Produits.
        Action::CreerProduit { uid, fiche } => {
            let p = produits::creer(conn, &qui, &fiche_produit(fiche)?)?;
            poser_uid(conn, Entite::Produit, p.id, uid)?;
            Ok(acceptee)
        }
        Action::ModifierProduit { uid, fiche } => {
            let id = exiger_id(conn, Entite::Produit, uid)?;
            produits::modifier(conn, &qui, id, &fiche_produit(fiche)?)?;
            Ok(acceptee)
        }
        Action::RetirerProduit { uid } => {
            let id = exiger_id(conn, Entite::Produit, uid)?;
            Ok(retrait(produits::retirer(conn, &qui, id)?))
        }
        Action::DefinirActifProduit { uid, actif } => {
            let id = exiger_id(conn, Entite::Produit, uid)?;
            produits::definir_actif(conn, &qui, id, *actif)?;
            Ok(acceptee)
        }
        Action::ImporterProduits { fiches, uids } => {
            if fiches.len() != uids.len() {
                return Err(illisible());
            }
            let fiches = fiches
                .iter()
                .map(fiche_produit)
                .collect::<Resultat<Vec<_>>>()?;
            let avant = dernier_id(conn, Entite::Produit)?;
            let bilan = produits::importer(conn, &qui, &fiches)?;
            poser_uids_import(conn, Entite::Produit, avant, uids, &bilan)
        }

        // Documents.
        Action::CreerBrouillon {
            uid,
            type_document,
            saisie,
        } => {
            let type_document = TypeDocument::depuis_code(type_document).ok_or_else(illisible)?;
            let s = saisie_locale(conn, saisie)?;
            let id = documents::creer_brouillon(conn, &qui, type_document, &s)?;
            poser_uid(conn, Entite::Document, id, uid)?;
            Ok(acceptee)
        }
        Action::ModifierBrouillon { uid, saisie } => {
            let id = doc(conn, uid)?;
            let s = saisie_locale(conn, saisie)?;
            documents::modifier_brouillon(conn, &qui, id, &s)?;
            Ok(acceptee)
        }
        Action::ModifierLivraison {
            uid,
            visa_client,
            visa_fournisseur,
            date_livraison,
            date_reception,
        } => {
            let id = doc(conn, uid)?;
            let champs = ChampsLivraison {
                visa_client: visa_client.clone(),
                visa_fournisseur: visa_fournisseur.clone(),
                date_livraison: date_livraison.clone(),
                date_reception: date_reception.clone(),
            };
            documents::modifier_livraison(conn, &qui, id, &champs)?;
            Ok(acceptee)
        }
        Action::ModifierAdministratif {
            uid,
            mode_paiement,
            observations,
        } => {
            let id = doc(conn, uid)?;
            documents::modifier_administratif(conn, &qui, id, mode_paiement, observations)?;
            Ok(acceptee)
        }
        Action::Valider { uid } => {
            let id = doc(conn, uid)?;
            let numero = documents::valider(conn, &qui, id)?;
            Ok(Issue::Acceptee {
                numero: Some(numero),
                bilan: None,
            })
        }
        Action::Annuler { uid } => {
            let id = doc(conn, uid)?;
            documents::annuler(conn, &qui, id)?;
            Ok(acceptee)
        }
        Action::SupprimerBrouillon { uid } => {
            let id = doc(conn, uid)?;
            documents::supprimer_brouillon(conn, &qui, id)?;
            Ok(acceptee)
        }
        Action::Dupliquer {
            uid,
            nouveau,
            annuler_original,
        } => {
            let id = doc(conn, uid)?;
            let copie = documents::dupliquer(conn, &qui, id, *annuler_original)?;
            poser_uid(conn, Entite::Document, copie, nouveau)?;
            Ok(acceptee)
        }
        Action::CreerAvoir { uid, nouveau } => {
            let id = doc(conn, uid)?;
            let avoir = documents::creer_avoir(conn, &qui, id)?;
            poser_uid(conn, Entite::Document, avoir, nouveau)?;
            Ok(acceptee)
        }
        Action::ConvertirEnFacture { uid, nouveau } => {
            let id = doc(conn, uid)?;
            let facture = documents::convertir_en_facture(conn, &qui, id)?;
            poser_uid(conn, Entite::Document, facture, nouveau)?;
            Ok(acceptee)
        }
        Action::CreerBordereau { uid, nouveau } => {
            let id = doc(conn, uid)?;
            let bordereau = documents::creer_bordereau(conn, &qui, id)?;
            poser_uid(conn, Entite::Document, bordereau, nouveau)?;
            Ok(acceptee)
        }
        Action::ConfirmerLivraison { uid } => {
            let id = doc(conn, uid)?;
            documents::confirmer_livraison(conn, &qui, id)?;
            Ok(acceptee)
        }
        Action::EnregistrerReception { uid, recues } => {
            let id = doc(conn, uid)?;
            documents::enregistrer_reception(conn, &qui, id, recues)?;
            Ok(acceptee)
        }
        Action::MarquerEnvoye { uid } => {
            let id = doc(conn, uid)?;
            documents::marquer_envoye(conn, &qui, id)?;
            Ok(acceptee)
        }

        // Règlements.
        Action::AjouterReglement {
            uid,
            document,
            date: jour,
            montant,
            mode_paiement,
            reference,
        } => {
            let id = doc(conn, document)?;
            let saisie = SaisieReglement {
                date: date(jour)?,
                montant: *montant,
                mode_paiement: mode_paiement.clone(),
                reference: reference.clone(),
            };
            let reglement = reglements::ajouter(conn, &qui, id, &saisie)?;
            poser_uid(conn, Entite::Reglement, reglement, uid)?;
            Ok(acceptee)
        }
        Action::SupprimerReglement { uid } => {
            let id = exiger_id(conn, Entite::Reglement, uid)?;
            reglements::supprimer(conn, &qui, id)?;
            Ok(acceptee)
        }

        // Comptes.
        Action::CreerCompte {
            uid,
            nom_complet,
            identifiant,
            mot_de_passe,
            role,
        } => {
            let nouveau = NouvelUtilisateur {
                nom_complet: nom_complet.clone(),
                identifiant: identifiant.clone(),
                mot_de_passe: mot_de_passe.clone(),
                role: Role::depuis_code(role).ok_or_else(illisible)?,
            };
            let u = comptes::creer(conn, &qui, &nouveau)?;
            poser_uid(conn, Entite::Utilisateur, u.id, uid)?;
            Ok(acceptee)
        }
        Action::DefinirActifCompte { uid, actif } => {
            let id = exiger_id(conn, Entite::Utilisateur, uid)?;
            comptes::definir_actif(conn, &qui, id, *actif)?;
            Ok(acceptee)
        }
        Action::ChangerRole { uid, role } => {
            let id = exiger_id(conn, Entite::Utilisateur, uid)?;
            let role = Role::depuis_code(role).ok_or_else(illisible)?;
            comptes::changer_role(conn, &qui, id, role)?;
            Ok(acceptee)
        }
        Action::ChangerMotDePasse { ancien, nouveau } => {
            comptes::changer_mot_de_passe(conn, qui.id, ancien, nouveau)?;
            Ok(acceptee)
        }
        Action::ReinitialiserMotDePasse { uid, nouveau } => {
            let id = exiger_id(conn, Entite::Utilisateur, uid)?;
            comptes::reinitialiser_mot_de_passe(conn, &qui, id, nouveau)?;
            Ok(acceptee)
        }
        Action::ChangerLangue { langue } => {
            let langue = Langue::depuis_code(langue).ok_or_else(illisible)?;
            comptes::changer_langue(conn, &qui, langue)?;
            Ok(acceptee)
        }
        Action::ChangerAffichage {
            navigation,
            animations,
        } => {
            let navigation = Navigation::depuis_code(navigation).ok_or_else(illisible)?;
            comptes::changer_affichage(conn, &qui, navigation, *animations)?;
            Ok(acceptee)
        }

        // Réglages partagés.
        Action::EnregistrerParametres { contenu } => {
            // Les dossiers d'export sont ceux du serveur, jamais ceux du poste.
            let mut p = parametres::depuis_json(contenu);
            p.dossiers_export = parametres::lire(conn)?.dossiers_export;
            parametres::enregistrer(conn, &qui, &p)?;
            Ok(acceptee)
        }
        Action::EnregistrerEntreprise { contenu } => {
            entreprise::enregistrer(conn, &qui, &Entreprise::depuis_json(contenu))?;
            Ok(acceptee)
        }
        Action::DefinirLogo { logo, octets } => {
            let logo = Logo::depuis_code(logo).ok_or_else(illisible)?;
            let octets = match octets {
                Some(texte) => Some(BASE64.decode(texte).map_err(|_| illisible())?),
                None => None,
            };
            entreprise::definir_logo(conn, &qui, logo, octets.as_deref())?;
            Ok(acceptee)
        }
        Action::EnregistrerMatrice { contenu } => {
            droits::enregistrer_matrice(conn, &qui, &droits::matrice_depuis_json(contenu))?;
            Ok(acceptee)
        }
        Action::EnregistrerTextesRelance { contenu } => {
            relances::enregistrer_textes(conn, &qui, &relances::depuis_json(contenu))?;
            Ok(acceptee)
        }
        Action::EnregistrerMiseEnPage { contenu } => {
            let m = mise_en_page::pour_editeur(&mise_en_page::depuis_json(contenu));
            mise_en_page::enregistrer(conn, &qui, &m)?;
            Ok(acceptee)
        }
        Action::EnregistrerBas {
            type_document,
            contenu,
        } => {
            let t = TypeDocument::depuis_code(type_document).ok_or_else(illisible)?;
            let bas = mise_en_page::bas_depuis_json(contenu, t);
            mise_en_page::enregistrer_bas(conn, &qui, t, &bas)?;
            Ok(acceptee)
        }

        // Opérations suspendues (F6).
        Action::TrancherConflit {
            operation,
            resultat,
        } => trancher(conn, op, &qui, operation, resultat.as_deref()),
        Action::DeciderAttente {
            operation,
            accepter,
        } => decider(conn, op, &qui, operation, *accepter),
    }
}

/// Opération suspendue encore ouverte, avec le motif voulu : sa copie gardée.
fn suspendue_ouverte(conn: &Connection, id: &str, motif: &str) -> Resultat<Operation> {
    let contenu: Option<String> = conn
        .query_row(
            "SELECT o.contenu FROM operation_suspendue s JOIN operation o ON o.id = s.operation_id
             WHERE s.operation_id = ?1 AND s.motif = ?2 AND s.ouverte = 1",
            params![id, motif],
            |r| r.get(0),
        )
        .optional()?;
    let contenu = contenu.ok_or_else(|| {
        Erreur::Validation("opération suspendue introuvable ou déjà décidée".into())
    })?;
    serde_json::from_str(&contenu).map_err(|_| illisible())
}

/// Ferme une opération suspendue et réécrit le résultat gardé pour elle : son
/// renvoi par le poste d'origine rend alors la décision.
fn clore(conn: &Connection, id: &str, issue: &Issue) -> Resultat<()> {
    let issue = serde_json::to_string(issue).map_err(|e| Erreur::Validation(e.to_string()))?;
    conn.execute(
        "UPDATE operation_suspendue SET ouverte = 0 WHERE operation_id = ?1",
        [id],
    )?;
    conn.execute(
        "UPDATE operation SET issue = ?1 WHERE id = ?2",
        params![issue, id],
    )?;
    Ok(())
}

/// Droit de modifier la ligne visée par une action en conflit.
fn droit_de_modifier(conn: &Connection, action: &Action) -> Resultat<Droit> {
    Ok(match action.cible() {
        Some((Entite::Document, uid)) => {
            let id = exiger_id(conn, Entite::Document, uid)?;
            let d = documents::lire(conn, id)?.ok_or_else(|| inconnu(Entite::Document, uid))?;
            Droit::brouillon(d.type_document)
        }
        _ => Droit::ModifierReferentiel,
    })
}

/// Peut trancher le conflit de l'opération `origine` : l'Admin, ou le compte
/// qui l'a faite s'il a le droit de modifier la ligne (décision 33).
fn exiger_tranche(conn: &Connection, qui: &Utilisateur, origine: &Operation) -> Resultat<()> {
    if qui.role == Role::Admin {
        return Ok(());
    }
    let droit = droit_de_modifier(conn, &origine.action)?;
    if uid_de(conn, Entite::Utilisateur, qui.id)?.as_deref() != Some(origine.compte.as_str()) {
        return Err(Erreur::Interdit(droit));
    }
    droits::exiger(conn, qui, droit)
}

/// Inscrit une décision au journal d'audit, sur la ligne visée quand il y en
/// a une, avec le compte d'origine et le compte qui décide.
fn journaliser_decision(
    conn: &Connection,
    qui: &Utilisateur,
    origine: &Operation,
    action: &str,
    detail: serde_json::Value,
) -> Resultat<()> {
    let (entite, id) = match origine.action.cible() {
        Some((e, uid)) => (e.code(), id_de(conn, e, uid)?.unwrap_or(0)),
        None => ("operation", 0),
    };
    let mut nouvelle = serde_json::json!({
        "operation": origine.id,
        "compte_origine": origine.compte,
        "poste_origine": origine.poste,
    });
    if let (Some(n), serde_json::Value::Object(d)) = (nouvelle.as_object_mut(), detail) {
        n.extend(d);
    }
    crate::audit::journaliser(
        conn,
        crate::audit::Entree {
            entite,
            entite_id: id,
            action,
            ancienne_valeur: serde_json::to_value(&origine.action).ok(),
            nouvelle_valeur: Some(nouvelle),
            utilisateur_id: Some(qui.id),
        },
    )
}

/// Tranche un conflit (décisions 17, 33 et 34) dans le point de sauvegarde de
/// l'opération qui décide : un refus de l'action composée laisse le conflit
/// ouvert.
fn trancher(
    conn: &mut Connection,
    op: &Operation,
    qui: &Utilisateur,
    operation: &str,
    resultat: Option<&Action>,
) -> Resultat<Issue> {
    let origine = suspendue_ouverte(conn, operation, "conflit")?;
    exiger_tranche(conn, qui, &origine)?;
    let (issue, issue_origine, garde) = match resultat {
        None => (
            Issue::acceptee(),
            Issue::Refusee {
                modele: MODELE_CONFLIT_SERVEUR.into(),
                valeurs: Vec::new(),
            },
            "serveur",
        ),
        Some(action) => {
            if std::mem::discriminant(action) != std::mem::discriminant(&origine.action)
                || action.cible() != origine.action.cible()
            {
                return Err(Erreur::Validation(
                    "le résultat ne correspond pas à l'opération en conflit".into(),
                ));
            }
            let composee = Operation {
                action: action.clone(),
                ..op.clone()
            };
            let issue = executer(conn, &composee)?;
            let garde = if action.masquee() == origine.action {
                "poste"
            } else {
                "composee"
            };
            (issue.clone(), issue, garde)
        }
    };
    journaliser_decision(
        conn,
        qui,
        &origine,
        "conflit_tranche",
        serde_json::json!({ "version_gardee": garde }),
    )?;
    clore(conn, &origine.id, &issue_origine)?;
    Ok(issue)
}

/// Accepte ou rejette une opération en attente (décisions 18 et 32), Admin
/// seulement. Acceptée, elle est appliquée au nom de l'Admin, sur l'état
/// présent ; une ligne changée ailleurs depuis sa base la fait passer en
/// conflit (ou dépassée), comme à sa réception. Le résultat rendu est celui
/// de l'opération d'origine ; elle est close dans tous les cas, sauf quand
/// elle passe en conflit, où la décision rend « acceptée » et l'opération
/// d'origine reste suspendue comme conflit à trancher.
fn decider(
    conn: &mut Connection,
    op: &Operation,
    qui: &Utilisateur,
    operation: &str,
    accepter: bool,
) -> Resultat<Issue> {
    let origine = suspendue_ouverte(conn, operation, "attente")?;
    if qui.role != Role::Admin {
        return Err(Erreur::Interdit(Droit::GererUtilisateurs));
    }
    if !accepter {
        let issue = Issue::Refusee {
            modele: MODELE_REJETEE.into(),
            valeurs: Vec::new(),
        };
        journaliser_decision(
            conn,
            qui,
            &origine,
            "attente_rejetee",
            serde_json::json!({}),
        )?;
        clore(conn, &origine.id, &issue)?;
        return Ok(issue);
    }
    let changee = match (origine.action.cible(), origine.action.si_changee()) {
        (Some((entite, uid)), SiChangee::Conflit | SiChangee::Depassee) => {
            modifiee_ailleurs(conn, &origine, entite, uid)?
        }
        _ => false,
    };
    let issue = if changee && origine.action.si_changee() == SiChangee::Conflit {
        // Reste suspendue, à trancher comme conflit.
        conn.execute(
            "UPDATE operation_suspendue SET motif = 'conflit' WHERE operation_id = ?1",
            [&origine.id],
        )?;
        let issue = Issue::EnConflit;
        let texte = serde_json::to_string(&issue).map_err(|e| Erreur::Validation(e.to_string()))?;
        conn.execute(
            "UPDATE operation SET issue = ?1 WHERE id = ?2",
            params![texte, origine.id],
        )?;
        journaliser_decision(
            conn,
            qui,
            &origine,
            "attente_acceptee",
            serde_json::json!({ "resultat": "conflit" }),
        )?;
        // La décision elle-même est faite : elle n'est pas suspendue.
        return Ok(Issue::acceptee());
    } else if changee {
        Issue::Depassee
    } else {
        let au_nom_de_l_admin = Operation {
            compte: op.compte.clone(),
            ..origine.clone()
        };
        conn.execute_batch("SAVEPOINT acceptation")?;
        match executer(conn, &au_nom_de_l_admin) {
            Ok(issue) => {
                conn.execute_batch("RELEASE acceptation")?;
                issue
            }
            Err(e) => {
                conn.execute_batch("ROLLBACK TO acceptation; RELEASE acceptation")?;
                issue_du_refus(e, false)?
            }
        }
    };
    journaliser_decision(
        conn,
        qui,
        &origine,
        "attente_acceptee",
        serde_json::json!({ "resultat": serde_json::to_value(&issue).ok() }),
    )?;
    clore(conn, &origine.id, &issue)?;
    Ok(issue)
}

/// Opérations suspendues que `qui` peut trancher ou décider : toutes pour
/// l'Admin ; pour un autre compte, ses propres conflits sur une ligne qu'il a
/// le droit de modifier (décision 33). Les plus anciennes d'abord.
pub fn suspendues(conn: &Connection, qui: &Utilisateur) -> Resultat<Vec<Suspendue>> {
    let lignes: Vec<(String, String, String)> = conn
        .prepare(
            "SELECT o.contenu, s.motif, o.recue FROM operation_suspendue s
             JOIN operation o ON o.id = s.operation_id
             WHERE s.ouverte = 1 ORDER BY o.recue, o.rowid",
        )?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    let mut sortie = Vec::new();
    for (contenu, motif, recue) in lignes {
        let operation: Operation = serde_json::from_str(&contenu).map_err(|_| illisible())?;
        let motif = if motif == "conflit" {
            Motif::Conflit
        } else {
            Motif::Attente
        };
        let visible = qui.role == Role::Admin
            || (motif == Motif::Conflit && exiger_tranche(conn, qui, &operation).is_ok());
        if visible {
            sortie.push(Suspendue {
                cible: operation
                    .action
                    .cible()
                    .map(|(e, uid)| (e, uid.to_string())),
                operation,
                motif,
                recue,
            });
        }
    }
    Ok(sortie)
}

/// État présent des lignes demandées, pour défaire une écriture provisoire
/// dans la copie d'un poste (F6) : `Supprime` pour une ligne inconnue ; `seq`
/// est le rang du dernier changement de la ligne, 0 si aucun.
pub fn lignes(conn: &Connection, demandees: &[(Entite, String)]) -> Resultat<Vec<Changement>> {
    let mut sortie = Vec::with_capacity(demandees.len());
    for (entite, uid) in demandees {
        let id = id_de(conn, *entite, uid)?;
        let etat = match id {
            Some(id) => replique::etat(conn, *entite, id)?.unwrap_or(Etat::Supprime),
            None => Etat::Supprime,
        };
        let seq: i64 = match id {
            Some(id) => conn.query_row(
                "SELECT coalesce(max(seq), 0) FROM changement WHERE entite = ?1 AND entite_id = ?2",
                params![entite.code(), id],
                |r| r.get(0),
            )?,
            None => conn.query_row(
                "SELECT coalesce(max(seq), 0) FROM changement WHERE entite = ?1 AND uid = ?2",
                params![entite.code(), uid],
                |r| r.get(0),
            )?,
        };
        sortie.push(Changement {
            seq: seq as u64,
            entite: *entite,
            uid: uid.clone(),
            etat,
        });
    }
    Ok(sortie)
}

fn illisible() -> Erreur {
    Erreur::Validation("opération illisible".into())
}

fn fiche_tiers(f: &proto::Tiers) -> Resultat<FicheTiers> {
    Ok(FicheTiers {
        nom: f.nom.clone(),
        adresse: f.adresse.clone(),
        telephone: f.telephone.clone(),
        localisation: f.localisation.clone(),
        ifu: f.ifu.clone(),
        rccm: f.rccm.clone(),
        regime_imposition: f.regime_imposition.clone(),
        type_tiers: TypeTiers::depuis_code(&f.type_tiers).ok_or_else(illisible)?,
    })
}

fn fiche_produit(f: &proto::Produit) -> Resultat<FicheProduit> {
    Ok(FicheProduit {
        designation: f.designation.clone(),
        reference: f.reference.clone(),
        prix: f.prix,
        sens: Sens::depuis_code(&f.sens).ok_or_else(illisible)?,
    })
}

/// Fiche de tiers d'un écran traduite pour une action (`CreerTiers`,
/// `ModifierTiers`, `ImporterTiers`). Réciproque de la lecture du serveur.
pub fn tiers_pour_serveur(f: &FicheTiers) -> proto::Tiers {
    proto::Tiers {
        nom: f.nom.clone(),
        adresse: f.adresse.clone(),
        telephone: f.telephone.clone(),
        localisation: f.localisation.clone(),
        ifu: f.ifu.clone(),
        rccm: f.rccm.clone(),
        regime_imposition: f.regime_imposition.clone(),
        type_tiers: f.type_tiers.code().to_string(),
    }
}

/// Fiche de produit d'un écran traduite pour une action (`CreerProduit`,
/// `ModifierProduit`, `ImporterProduits`). Réciproque de la lecture du serveur.
pub fn produit_pour_serveur(f: &FicheProduit) -> proto::Produit {
    proto::Produit {
        designation: f.designation.clone(),
        reference: f.reference.clone(),
        prix: f.prix,
        sens: f.sens.code().to_string(),
    }
}

const FORMAT_DATE: &str = "%Y-%m-%d";

fn date(texte: &str) -> Resultat<NaiveDate> {
    NaiveDate::parse_from_str(texte, FORMAT_DATE).map_err(|_| illisible())
}

fn saisie_locale(conn: &Connection, s: &proto::Saisie) -> Resultat<Saisie> {
    let tiers_id = match &s.tiers {
        Some(uid) => Some(exiger_id(conn, Entite::Tiers, uid)?),
        None => None,
    };
    let mut lignes = Vec::with_capacity(s.lignes.len());
    for l in &s.lignes {
        let produit_id = match &l.produit {
            Some(uid) => Some(exiger_id(conn, Entite::Produit, uid)?),
            None => None,
        };
        lignes.push(LigneSaisie {
            produit_id,
            reference: l.reference.clone(),
            designation: l.designation.clone(),
            prix: l.prix,
            quantite: l.quantite,
            quantite_livree: l.quantite_livree,
            observations: l.observations.clone(),
        });
    }
    Ok(Saisie {
        date: date(&s.date)?,
        tiers_id,
        libelle: s.libelle.clone(),
        taux: Taux {
            remise: s.remise,
            tva: s.tva,
            retenue: s.retenue,
        },
        mode_paiement: s.mode_paiement.clone(),
        observations: s.observations.clone(),
        date_echeance: s.date_echeance.as_deref().map(date).transpose()?,
        lignes,
        livraison: ChampsLivraison {
            visa_client: s.visa_client.clone(),
            visa_fournisseur: s.visa_fournisseur.clone(),
            date_livraison: s.date_livraison.clone(),
            date_reception: s.date_reception.clone(),
        },
    })
}

/// Saisie d'un écran (identifiants propres à cette base) traduite pour une
/// action `CreerBrouillon` ou `ModifierBrouillon` : tiers et produits désignés
/// par leur identifiant global. Réciproque de la lecture faite par le serveur.
pub fn saisie_pour_serveur(conn: &Connection, s: &Saisie) -> Resultat<proto::Saisie> {
    let global = |entite: Entite, id: i64| -> Resultat<String> {
        uid_de(conn, entite, id)?.ok_or_else(|| {
            Erreur::Validation(format!(
                "élément sans identifiant global : {} {id}",
                entite.code()
            ))
        })
    };
    let tiers = s.tiers_id.map(|id| global(Entite::Tiers, id)).transpose()?;
    let mut lignes = Vec::with_capacity(s.lignes.len());
    for l in &s.lignes {
        lignes.push(proto::Ligne {
            produit: l
                .produit_id
                .map(|id| global(Entite::Produit, id))
                .transpose()?,
            reference: l.reference.clone(),
            designation: l.designation.clone(),
            prix: l.prix,
            quantite: l.quantite,
            quantite_livree: l.quantite_livree,
            observations: l.observations.clone(),
        });
    }
    Ok(proto::Saisie {
        date: s.date.format(FORMAT_DATE).to_string(),
        tiers,
        libelle: s.libelle.clone(),
        remise: s.taux.remise,
        tva: s.taux.tva,
        retenue: s.taux.retenue,
        mode_paiement: s.mode_paiement.clone(),
        observations: s.observations.clone(),
        date_echeance: s.date_echeance.map(|d| d.format(FORMAT_DATE).to_string()),
        lignes,
        visa_client: s.livraison.visa_client.clone(),
        visa_fournisseur: s.livraison.visa_fournisseur.clone(),
        date_livraison: s.livraison.date_livraison.clone(),
        date_reception: s.livraison.date_reception.clone(),
    })
}

/// Changements après le rang `depuis`, au plus `limite`, du plus ancien au plus
/// récent : chaque ligne une seule fois, dans son état présent (une ligne qui a
/// changé plusieurs fois prend le rang de son dernier changement).
pub fn changements_depuis(
    conn: &Connection,
    depuis: u64,
    limite: u32,
) -> Resultat<Vec<Changement>> {
    // Une ligne créée hors d'une opération (fenêtre du serveur) reçoit ici son
    // identifiant, avant d'être décrite.
    completer_uids(conn)?;
    let lignes: Vec<(i64, String, i64)> = conn
        .prepare(
            "SELECT max(seq), entite, entite_id FROM changement WHERE seq > ?1
             GROUP BY entite, entite_id ORDER BY max(seq) LIMIT ?2",
        )?
        .query_map(params![depuis as i64, limite], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?
        .collect::<Result<_, _>>()?;
    let mut sortie = Vec::with_capacity(lignes.len());
    let mut octets = 0;
    for (seq, code, id) in lignes {
        let Some(entite) = Entite::depuis_code(&code) else {
            continue;
        };
        let etat = replique::etat(conn, entite, id)?.unwrap_or(Etat::Supprime);
        // Ligne disparue : son identifiant est celui noté lors de sa suppression.
        let uid = match uid_de(conn, entite, id)? {
            Some(uid) => Some(uid),
            None => conn
                .query_row(
                    "SELECT uid FROM changement WHERE entite = ?1 AND entite_id = ?2
                     AND uid IS NOT NULL ORDER BY seq DESC LIMIT 1",
                    params![code, id],
                    |r| r.get(0),
                )
                .optional()?,
        };
        if let Some(uid) = uid {
            let changement = Changement {
                seq: seq as u64,
                entite,
                uid,
                etat,
            };
            let taille = serde_json::to_vec(&changement).map_or(0, |v| v.len());
            if !sortie.is_empty() && octets + taille > BUDGET_PAGE {
                break;
            }
            octets += taille;
            sortie.push(changement);
        }
    }
    Ok(sortie)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::droits::tests::compte;
    use crate::ouvrir_en_memoire;
    use kadenz_domaine::role::Role;

    struct Banc {
        conn: Connection,
        entreprise: String,
        admin: String,
        vente: String,
    }

    fn banc() -> Banc {
        let conn = ouvrir_en_memoire().unwrap();
        let admin = compte(&conn, "admin", Role::Admin);
        let vente = compte(&conn, "lecture", Role::Consultation);
        completer_uids(&conn).unwrap();
        let entreprise = identifiant_entreprise(&conn).unwrap();
        let admin = uid_de(&conn, Entite::Utilisateur, admin.id)
            .unwrap()
            .unwrap();
        let vente = uid_de(&conn, Entite::Utilisateur, vente.id)
            .unwrap()
            .unwrap();
        Banc {
            conn,
            entreprise,
            admin,
            vente,
        }
    }

    impl Banc {
        fn op(&self, poste: &str, compte: &str, base: u64, action: Action) -> Operation {
            Operation {
                id: nouvel_uid(),
                entreprise: self.entreprise.clone(),
                poste: poste.into(),
                compte: compte.into(),
                hors_connexion: false,
                heure_poste: None,
                base,
                action,
            }
        }

        fn base(&self) -> u64 {
            dernier_changement(&self.conn).unwrap()
        }

        fn appliquer(&mut self, op: &Operation) -> Issue {
            appliquer(&mut self.conn, op).unwrap()
        }

        fn compteur(&self) -> i64 {
            self.conn
                .query_row(
                    "SELECT coalesce(sum(dernier_numero), 0) FROM compteur_numerotation",
                    [],
                    |r| r.get(0),
                )
                .unwrap()
        }

        fn statut(&self, uid: &str) -> String {
            let id = id_de(&self.conn, Entite::Document, uid).unwrap().unwrap();
            documents::lire(&self.conn, id)
                .unwrap()
                .unwrap()
                .statut
                .code()
                .into()
        }
    }

    fn fiche(nom: &str) -> proto::Tiers {
        proto::Tiers {
            nom: nom.into(),
            type_tiers: "client".into(),
            ..proto::Tiers::default()
        }
    }

    fn saisie(tiers: &str) -> proto::Saisie {
        proto::Saisie {
            date: "2026-11-12".into(),
            tiers: Some(tiers.into()),
            lignes: vec![proto::Ligne {
                designation: "Ciment".into(),
                prix: 5000,
                quantite: 2_000,
                ..proto::Ligne::default()
            }],
            ..proto::Saisie::default()
        }
    }

    /// Client puis brouillon de facture pour ce client, créés par le poste `p`.
    fn client_et_brouillon(b: &mut Banc, p: &str) -> (String, String) {
        let (t, d) = (nouvel_uid(), nouvel_uid());
        let base = b.base();
        let admin = b.admin.clone();
        let creer_t = b.op(
            p,
            &admin,
            base,
            Action::CreerTiers {
                uid: t.clone(),
                fiche: fiche("Client Alpha"),
            },
        );
        assert_eq!(b.appliquer(&creer_t), Issue::acceptee());
        let creer_d = b.op(
            p,
            &admin,
            base,
            Action::CreerBrouillon {
                uid: d.clone(),
                type_document: "facture".into(),
                saisie: saisie(&t),
            },
        );
        assert_eq!(b.appliquer(&creer_d), Issue::acceptee());
        (t, d)
    }

    #[test]
    fn migration_et_identifiants() {
        let b = banc();
        let sans_uid: i64 = b
            .conn
            .query_row(
                "SELECT count(*) FROM utilisateur WHERE uid IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(sans_uid, 0);
        assert_eq!(b.entreprise.len(), 36);
        // L'identifiant de l'entreprise ne change pas à la réouverture.
        preparer(&b.conn).unwrap();
        assert_eq!(identifiant_entreprise(&b.conn).unwrap(), b.entreprise);
    }

    #[test]
    fn operation_renvoyee_appliquee_une_fois() {
        let mut b = banc();
        let t = nouvel_uid();
        let admin = b.admin.clone();
        let op = b.op(
            "p1",
            &admin,
            0,
            Action::CreerTiers {
                uid: t.clone(),
                fiche: fiche("Client Alpha"),
            },
        );
        assert_eq!(b.appliquer(&op), Issue::acceptee());
        assert_eq!(b.appliquer(&op), Issue::acceptee());
        let nombre: i64 = b
            .conn
            .query_row(
                "SELECT count(*) FROM tiers WHERE nom = 'Client Alpha'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(nombre, 1);
        assert!(id_de(&b.conn, Entite::Tiers, &t).unwrap().is_some());

        // Même identifiant, autre contenu : refusé, sans rien appliquer.
        let mut autre = op.clone();
        autre.action = Action::CreerTiers {
            uid: nouvel_uid(),
            fiche: fiche("Client Gamma"),
        };
        assert!(appliquer(&mut b.conn, &autre).is_err());
    }

    #[test]
    fn numero_rendu_identique_apres_une_coupure() {
        let mut b = banc();
        let (_, d) = client_et_brouillon(&mut b, "p1");
        let admin = b.admin.clone();
        let base = b.base();
        let valider = b.op("p1", &admin, base, Action::Valider { uid: d.clone() });
        let Issue::Acceptee {
            numero: Some(numero),
            ..
        } = b.appliquer(&valider)
        else {
            panic!("numéro attendu");
        };
        // Réponse perdue : le poste renvoie la même opération.
        assert_eq!(
            b.appliquer(&valider),
            Issue::Acceptee {
                numero: Some(numero.clone()),
                bilan: None,
            }
        );
        assert_eq!(b.compteur(), 1);
        assert_eq!(b.statut(&d), "valide");
    }

    #[test]
    fn lecture_seule_rend_le_resultat_garde() {
        let mut b = banc();
        let (_, d) = client_et_brouillon(&mut b, "p1");
        let admin = b.admin.clone();
        let base = b.base();
        let valider = b.op("p1", &admin, base, Action::Valider { uid: d.clone() });
        let acceptee = b.appliquer(&valider);
        // L'abonnement échoit avant que le poste ait reçu la réponse.
        b.conn
            .execute(
                "INSERT INTO temp.abonnement_etat (id, lecture_seule, echeance)
                 VALUES (1, 1, '2026-12-31')",
                [],
            )
            .unwrap();
        // La validation renvoyée garde son numéro ; une nouvelle opération est refusée.
        assert_eq!(b.appliquer(&valider), acceptee);
        let base = b.base();
        let autre = b.op(
            "p1",
            &admin,
            base,
            Action::CreerTiers {
                uid: nouvel_uid(),
                fiche: fiche("Nouveau"),
            },
        );
        assert!(matches!(
            appliquer(&mut b.conn, &autre),
            Err(Erreur::LectureSeule { .. })
        ));
        assert_eq!(b.compteur(), 1);
    }

    #[test]
    fn panne_apres_la_validation_ne_laisse_rien() {
        let mut b = banc();
        let (_, d) = client_et_brouillon(&mut b, "p1");
        let admin = b.admin.clone();
        let base = b.base();
        let changements = b.base();
        // Panne simulée à l'enregistrement du résultat, après la validation.
        b.conn
            .execute_batch(
                "CREATE TEMP TRIGGER panne BEFORE INSERT ON operation
                 BEGIN SELECT RAISE(ABORT, 'panne'); END;",
            )
            .unwrap();
        let valider = b.op("p1", &admin, base, Action::Valider { uid: d.clone() });
        assert!(appliquer(&mut b.conn, &valider).is_err());
        assert_eq!(b.compteur(), 0);
        assert_eq!(b.statut(&d), "brouillon");
        assert_eq!(b.base(), changements);
        let gardees: i64 = b
            .conn
            .query_row(
                "SELECT count(*) FROM operation WHERE id = ?1",
                [&valider.id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(gardees, 0);
        // Panne réparée : le renvoi valide, avec le premier numéro.
        b.conn.execute_batch("DROP TRIGGER temp.panne").unwrap();
        assert!(matches!(
            b.appliquer(&valider),
            Issue::Acceptee {
                numero: Some(_),
                ..
            }
        ));
        assert_eq!(b.compteur(), 1);
    }

    /// Compte de la base par son identifiant global.
    fn qui(b: &Banc, uid: &str) -> Utilisateur {
        let id = id_de(&b.conn, Entite::Utilisateur, uid).unwrap().unwrap();
        comptes::lire(&b.conn, id).unwrap().unwrap()
    }

    fn nom_du_tiers(b: &Banc, uid: &str) -> String {
        let id = id_de(&b.conn, Entite::Tiers, uid).unwrap().unwrap();
        tiers::lire(&b.conn, id).unwrap().unwrap().fiche.nom
    }

    /// Modification de `t` par `poste` sur une base dépassée par un autre
    /// poste : en conflit.
    fn conflit(b: &mut Banc, t: &str, poste: &str, compte: &str, nom: &str) -> Operation {
        let base = b.base();
        let admin = b.admin.clone();
        let avant = b.op(
            "p1",
            &admin,
            b.base(),
            Action::ModifierTiers {
                uid: t.into(),
                fiche: fiche(&format!("Serveur {nom}")),
            },
        );
        assert_eq!(b.appliquer(&avant), Issue::acceptee());
        let op = b.op(
            poste,
            compte,
            base,
            Action::ModifierTiers {
                uid: t.into(),
                fiche: fiche(nom),
            },
        );
        assert_eq!(b.appliquer(&op), Issue::EnConflit);
        op
    }

    #[test]
    fn conflit_tranche() {
        let mut b = banc();
        let (t, _) = client_et_brouillon(&mut b, "p1");
        let admin = b.admin.clone();
        let vente = b.vente.clone();
        let compta = compte(&b.conn, "compta", Role::Comptable);
        completer_uids(&b.conn).unwrap();
        let compta = uid_de(&b.conn, Entite::Utilisateur, compta.id)
            .unwrap()
            .unwrap();
        let origine = conflit(&mut b, &t, "p2", &compta, "Version du poste");

        // Visible de l'Admin et de son auteur, qui a le droit de modifier.
        for compte in [&admin, &compta] {
            let liste = suspendues(&b.conn, &qui(&b, compte)).unwrap();
            assert_eq!(liste.len(), 1, "{compte}");
            assert_eq!(liste[0].motif, Motif::Conflit);
            assert_eq!(liste[0].cible, Some((Entite::Tiers, t.clone())));
            assert_eq!(liste[0].operation.id, origine.id);
        }
        assert!(suspendues(&b.conn, &qui(&b, &vente)).unwrap().is_empty());

        // Ni un autre compte sans droit, ni un résultat d'un autre genre.
        let par_vente = b.op(
            "p3",
            &vente,
            b.base(),
            Action::TrancherConflit {
                operation: origine.id.clone(),
                resultat: None,
            },
        );
        assert!(matches!(
            b.appliquer(&par_vente),
            Issue::Refusee { modele, .. } if modele == MODELE_INTERDIT
        ));
        let autre_genre = b.op(
            "p1",
            &admin,
            b.base(),
            Action::TrancherConflit {
                operation: origine.id.clone(),
                resultat: Some(Box::new(Action::RetirerTiers { uid: t.clone() })),
            },
        );
        assert!(matches!(
            b.appliquer(&autre_genre),
            Issue::Refusee { modele, .. } if modele == MODELES_DECISION[3]
        ));

        // Version du serveur gardée par l'auteur : le renvoi de l'opération
        // d'origine rend la décision, et la ligne n'est plus bloquée.
        let garder = b.op(
            "p2",
            &compta,
            b.base(),
            Action::TrancherConflit {
                operation: origine.id.clone(),
                resultat: None,
            },
        );
        assert_eq!(b.appliquer(&garder), Issue::acceptee());
        assert!(matches!(
            b.appliquer(&origine),
            Issue::Refusee { modele, .. } if modele == MODELE_CONFLIT_SERVEUR
        ));
        assert_eq!(nom_du_tiers(&b, &t), "Serveur Version du poste");
        assert!(suspendues(&b.conn, &qui(&b, &admin)).unwrap().is_empty());
        // Déjà tranché : refusé, sans rien changer.
        let encore = b.op(
            "p1",
            &admin,
            b.base(),
            Action::TrancherConflit {
                operation: origine.id.clone(),
                resultat: None,
            },
        );
        assert!(matches!(
            b.appliquer(&encore),
            Issue::Refusee { modele, .. } if modele == MODELES_DECISION[2]
        ));

        // Version du poste gardée par l'Admin.
        let origine = conflit(&mut b, &t, "p2", &compta, "Gardée du poste");
        let garder = b.op(
            "p1",
            &admin,
            b.base(),
            Action::TrancherConflit {
                operation: origine.id.clone(),
                resultat: Some(Box::new(origine.action.clone())),
            },
        );
        assert_eq!(b.appliquer(&garder), Issue::acceptee());
        assert_eq!(b.appliquer(&origine), Issue::acceptee());
        assert_eq!(nom_du_tiers(&b, &t), "Gardée du poste");

        // Champ par champ : une fiche composée.
        let origine = conflit(&mut b, &t, "p2", &compta, "Poste");
        let mut composee = fiche("Composée");
        composee.telephone = "70 00 00 00".into();
        let garder = b.op(
            "p1",
            &admin,
            b.base(),
            Action::TrancherConflit {
                operation: origine.id.clone(),
                resultat: Some(Box::new(Action::ModifierTiers {
                    uid: t.clone(),
                    fiche: composee,
                })),
            },
        );
        assert_eq!(b.appliquer(&garder), Issue::acceptee());
        assert_eq!(nom_du_tiers(&b, &t), "Composée");
        assert_eq!(
            crate::audit::compter(&b.conn, "conflit_tranche").unwrap(),
            3
        );
    }

    #[test]
    fn attente_decidee_par_l_admin() {
        let mut b = banc();
        let (t, _) = client_et_brouillon(&mut b, "p1");
        let admin = b.admin.clone();
        let vente = b.vente.clone();
        let hors_ligne = |b: &Banc, nom: &str| {
            let mut op = b.op(
                "p2",
                &vente,
                b.base(),
                Action::ModifierTiers {
                    uid: t.clone(),
                    fiche: fiche(nom),
                },
            );
            op.hors_connexion = true;
            op.heure_poste = Some("2026-11-13T08:00:00+00:00".into());
            op
        };
        let origine = hors_ligne(&b, "Saisi hors ligne");
        assert_eq!(b.appliquer(&origine), Issue::EnAttente);
        let liste = suspendues(&b.conn, &qui(&b, &admin)).unwrap();
        assert_eq!(liste.len(), 1);
        assert_eq!(liste[0].motif, Motif::Attente);
        assert_eq!(
            liste[0].operation.heure_poste.as_deref(),
            Some("2026-11-13T08:00:00+00:00")
        );
        // Une attente n'est montrée qu'à l'Admin, et décidée que par lui.
        assert!(suspendues(&b.conn, &qui(&b, &vente)).unwrap().is_empty());
        let decision = |b: &Banc, compte: &str, accepter: bool| {
            b.op(
                "p1",
                compte,
                b.base(),
                Action::DeciderAttente {
                    operation: origine.id.clone(),
                    accepter,
                },
            )
        };
        let par_vente = decision(&b, &vente, true);
        assert!(matches!(
            b.appliquer(&par_vente),
            Issue::Refusee { modele, .. } if modele == MODELE_INTERDIT
        ));
        // Acceptée : appliquée au nom de l'Admin, et le renvoi le dit.
        let accepter = decision(&b, &admin, true);
        assert_eq!(b.appliquer(&accepter), Issue::acceptee());
        assert_eq!(nom_du_tiers(&b, &t), "Saisi hors ligne");
        assert_eq!(b.appliquer(&origine), Issue::acceptee());
        assert_eq!(
            crate::audit::compter(&b.conn, "attente_acceptee").unwrap(),
            1
        );

        // Rejetée.
        let origine = hors_ligne(&b, "Rejeté");
        assert_eq!(b.appliquer(&origine), Issue::EnAttente);
        let rejeter = b.op(
            "p1",
            &admin,
            b.base(),
            Action::DeciderAttente {
                operation: origine.id.clone(),
                accepter: false,
            },
        );
        assert!(matches!(
            b.appliquer(&rejeter),
            Issue::Refusee { modele, .. } if modele == MODELE_REJETEE
        ));
        assert!(matches!(
            b.appliquer(&origine),
            Issue::Refusee { modele, .. } if modele == MODELE_REJETEE
        ));
        assert_eq!(nom_du_tiers(&b, &t), "Saisi hors ligne");

        // Acceptée après un changement de la fiche ailleurs : en conflit.
        let origine = hors_ligne(&b, "Tardif");
        assert_eq!(b.appliquer(&origine), Issue::EnAttente);
        let ailleurs = b.op(
            "p1",
            &admin,
            b.base(),
            Action::ModifierTiers {
                uid: t.clone(),
                fiche: fiche("Changé ailleurs"),
            },
        );
        assert_eq!(b.appliquer(&ailleurs), Issue::acceptee());
        let accepter = b.op(
            "p1",
            &admin,
            b.base(),
            Action::DeciderAttente {
                operation: origine.id.clone(),
                accepter: true,
            },
        );
        assert_eq!(b.appliquer(&accepter), Issue::acceptee());
        assert_eq!(b.appliquer(&origine), Issue::EnConflit);
        let liste = suspendues(&b.conn, &qui(&b, &admin)).unwrap();
        assert_eq!(liste.len(), 1);
        assert_eq!(liste[0].motif, Motif::Conflit);
        assert_eq!(nom_du_tiers(&b, &t), "Changé ailleurs");
    }

    #[test]
    fn hors_connexion_limite_et_lignes() {
        let mut b = banc();
        let (t, _) = client_et_brouillon(&mut b, "p1");
        let admin = b.admin.clone();
        // Une écriture refusée hors connexion n'est ni appliquée ni gardée.
        let mut compte = b.op(
            "p2",
            &admin,
            b.base(),
            Action::CreerCompte {
                uid: nouvel_uid(),
                nom_complet: "Nouveau".into(),
                identifiant: "nouveau".into(),
                mot_de_passe: "motdepasse1".into(),
                role: "commercial".into(),
            },
        );
        compte.hors_connexion = true;
        assert!(appliquer(&mut b.conn, &compte).is_err());
        let gardees: i64 = b
            .conn
            .query_row(
                "SELECT count(*) FROM operation WHERE id = ?1",
                [&compte.id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(gardees, 0);

        // État présent : la fiche telle qu'elle est, une ligne inconnue supprimée.
        let inconnu = nouvel_uid();
        let rendues = lignes(
            &b.conn,
            &[(Entite::Tiers, t.clone()), (Entite::Tiers, inconnu.clone())],
        )
        .unwrap();
        assert_eq!(rendues.len(), 2);
        assert!(matches!(
            &rendues[0].etat,
            Etat::Ligne { valeurs, .. } if valeurs["nom"] == "Client Alpha"
        ));
        assert!(rendues[0].seq > 0);
        assert_eq!(rendues[1].uid, inconnu);
        assert_eq!(rendues[1].etat, Etat::Supprime);
        assert_eq!(rendues[1].seq, 0);
    }

    #[test]
    fn refus_et_attente() {
        let mut b = banc();
        let (t, _) = client_et_brouillon(&mut b, "p1");
        let vente = b.vente.clone();
        let base = b.base();
        // La Consultation n'a pas le droit de modifier un tiers : refus en ligne.
        let mut op = b.op(
            "p2",
            &vente,
            base,
            Action::ModifierTiers {
                uid: t.clone(),
                fiche: fiche("Client Alpha modifié"),
            },
        );
        assert!(matches!(
            b.appliquer(&op),
            Issue::Refusee { modele, valeurs } if modele == MODELE_INTERDIT && valeurs.len() == 1
        ));
        // Hors connexion : mise en attente.
        op.id = nouvel_uid();
        op.hors_connexion = true;
        assert_eq!(b.appliquer(&op), Issue::EnAttente);
        // Opération suivante du même poste sur la même fiche : bloquée, sans être
        // gardée (le renvoi est réévalué).
        let admin = b.admin.clone();
        let suivante = b.op(
            "p2",
            &admin,
            b.base(),
            Action::ModifierTiers {
                uid: t.clone(),
                fiche: fiche("Autre nom"),
            },
        );
        assert_eq!(b.appliquer(&suivante), Issue::Bloquee);
        assert_eq!(b.appliquer(&suivante), Issue::Bloquee);
        // Un autre poste n'est pas bloqué.
        let ailleurs = b.op(
            "p1",
            &admin,
            b.base(),
            Action::ModifierTiers {
                uid: t.clone(),
                fiche: fiche("Nom de p1"),
            },
        );
        assert_eq!(b.appliquer(&ailleurs), Issue::acceptee());
        // Décision prise sur l'opération en attente : le renvoi, même identifiant,
        // est réévalué ; la fiche ayant changé sur p1 depuis sa base, il est en
        // conflit, et ce résultat-là est gardé.
        b.conn
            .execute("UPDATE operation_suspendue SET ouverte = 0", [])
            .unwrap();
        assert_eq!(b.appliquer(&suivante), Issue::EnConflit);
        assert_eq!(b.appliquer(&suivante), Issue::EnConflit);
        // Une règle métier n'est jamais mise en attente.
        let mut vide = b.op(
            "p1",
            &admin,
            b.base(),
            Action::CreerTiers {
                uid: nouvel_uid(),
                fiche: fiche(""),
            },
        );
        vide.hors_connexion = true;
        assert!(matches!(b.appliquer(&vide), Issue::Refusee { .. }));
        // Référence inconnue : refus avec un message à traduire.
        let inconnue = b.op(
            "p1",
            &admin,
            b.base(),
            Action::Valider { uid: nouvel_uid() },
        );
        assert!(matches!(
            b.appliquer(&inconnue),
            Issue::Refusee { modele, .. } if modele == "élément inconnu sur le serveur : {}"
        ));
    }

    #[test]
    fn conflits_entre_postes_seulement() {
        let mut b = banc();
        let (t, d) = client_et_brouillon(&mut b, "p1");
        let admin = b.admin.clone();
        let base = b.base();
        // Deux modifications successives du même poste, sur la même base : pas de conflit.
        for nom in ["Nom un", "Nom deux"] {
            let op = b.op(
                "p1",
                &admin,
                base,
                Action::ModifierTiers {
                    uid: t.clone(),
                    fiche: fiche(nom),
                },
            );
            assert_eq!(b.appliquer(&op), Issue::acceptee());
        }
        // Un autre poste resté sur l'ancienne base : conflit, rien d'appliqué.
        let op = b.op(
            "p2",
            &admin,
            base,
            Action::ModifierTiers {
                uid: t.clone(),
                fiche: fiche("Nom de p2"),
            },
        );
        assert_eq!(b.appliquer(&op), Issue::EnConflit);
        let id = id_de(&b.conn, Entite::Tiers, &t).unwrap().unwrap();
        assert_eq!(
            tiers::lire(&b.conn, id).unwrap().unwrap().fiche.nom,
            "Nom deux"
        );

        // Brouillon modifié par p2 à jour, puis validation de p1 sur l'ancienne base : dépassée.
        let base_p1 = b.base();
        let s = saisie(&t);
        let modif = b.op(
            "p2",
            &admin,
            b.base(),
            Action::ModifierBrouillon {
                uid: d.clone(),
                saisie: proto::Saisie {
                    libelle: "Changé sur p2".into(),
                    ..s
                },
            },
        );
        assert_eq!(b.appliquer(&modif), Issue::acceptee());
        let valider = b.op("p1", &admin, base_p1, Action::Valider { uid: d.clone() });
        assert_eq!(b.appliquer(&valider), Issue::Depassee);
        assert_eq!(b.compteur(), 0);
    }

    /// Base V1 réelle (schéma 6, sur fichier, en clair comme avant le
    /// chiffrement) : migrée sans perte, identifiants posés, numérotation continue.
    #[test]
    fn base_v1_sur_fichier_migree() {
        use kadenz_domaine::calcul::UNE_UNITE;
        let dossier =
            std::env::temp_dir().join(format!("kadenz-synchro-v1-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(&dossier).unwrap();
        let chemin = dossier.join("kadenz.sqlite");
        let numero_v1 = {
            let mut conn = Connection::open(&chemin).unwrap();
            conn.pragma_update(None, "foreign_keys", "ON").unwrap();
            for (rang, sql) in crate::MIGRATIONS[..6].iter().enumerate() {
                conn.execute_batch(sql).unwrap();
                conn.pragma_update(None, "user_version", rang as u32 + 1)
                    .unwrap();
            }
            crate::licence::preparer(&conn, true).unwrap();
            let admin = compte(&conn, "admin", Role::Admin);
            let mut f = FicheTiers::vide(TypeTiers::Client);
            f.nom = "Client V1".into();
            let t = tiers::creer(&conn, &admin, &f).unwrap();
            let mut s = Saisie::vide(NaiveDate::from_ymd_opt(2026, 11, 2).unwrap());
            s.tiers_id = Some(t.id);
            s.lignes = vec![LigneSaisie {
                designation: "Sable".into(),
                prix: 1000,
                quantite: UNE_UNITE,
                ..LigneSaisie::default()
            }];
            let id =
                documents::creer_brouillon(&mut conn, &admin, TypeDocument::Facture, &s).unwrap();
            documents::valider(&mut conn, &admin, id).unwrap()
        };

        let mut conn = crate::ouvrir(&chemin).unwrap();
        crate::licence::preparer(&conn, true).unwrap();
        let version: u32 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, crate::version_schema_application());
        for table in TABLES {
            let sans: i64 = conn
                .query_row(
                    &format!("SELECT count(*) FROM {table} WHERE uid IS NULL"),
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(sans, 0, "{table} sans identifiant");
        }
        let doc = documents::lire(&conn, 1).unwrap().unwrap();
        assert_eq!(doc.numero.as_deref(), Some(numero_v1.as_str()));
        let client = uid_de(&conn, Entite::Tiers, 1).unwrap().unwrap();
        let admin = uid_de(&conn, Entite::Utilisateur, 1).unwrap().unwrap();
        let entreprise = identifiant_entreprise(&conn).unwrap();

        // Une facture du même mois, créée et validée par opérations : numéro suivant.
        let d = nouvel_uid();
        let op = |action| Operation {
            id: nouvel_uid(),
            entreprise: entreprise.clone(),
            poste: "p1".into(),
            compte: admin.clone(),
            hors_connexion: false,
            heure_poste: None,
            base: 0,
            action,
        };
        let mut s = saisie(&client);
        s.date = "2026-11-20".into();
        let creer = op(Action::CreerBrouillon {
            uid: d.clone(),
            type_document: "facture".into(),
            saisie: s,
        });
        assert_eq!(appliquer(&mut conn, &creer).unwrap(), Issue::acceptee());
        let base = dernier_changement(&conn).unwrap();
        let mut valider = op(Action::Valider { uid: d });
        valider.base = base;
        let Issue::Acceptee {
            numero: Some(numero),
            ..
        } = appliquer(&mut conn, &valider).unwrap()
        else {
            panic!("numéro attendu");
        };
        assert_ne!(numero, numero_v1);
        assert_eq!(
            numero[..numero.len() - 4],
            numero_v1[..numero_v1.len() - 4],
            "même série"
        );
        assert!(numero.ends_with("0002"), "{numero} suit {numero_v1}");
        drop(conn);
        let _ = std::fs::remove_dir_all(dossier);
    }

    #[test]
    fn suite_des_changements() {
        let mut b = banc();
        let depart = b.base();
        let (t, d) = client_et_brouillon(&mut b, "p1");
        let ch = changements_depuis(&b.conn, depart, 100).unwrap();
        let tiers_ch = ch.iter().find(|c| c.uid == t).expect("tiers");
        assert!(matches!(&tiers_ch.etat, Etat::Ligne { valeurs, .. }
            if valeurs["nom"] == "Client Alpha" && valeurs["actif"] == 1));
        let doc = ch.iter().find(|c| c.uid == d).expect("document");
        assert!(matches!(&doc.etat, Etat::Ligne { valeurs, lignes }
            if valeurs["statut"] == "brouillon"
                && valeurs["tiers_id"] == t.as_str()
                && !lignes.is_empty()));
        // Chaque ligne une seule fois, dans l'ordre des rangs.
        assert_eq!(ch.iter().filter(|c| c.uid == d).count(), 1);
        assert!(ch.windows(2).all(|w| w[0].seq < w[1].seq));

        // Suppression d'un brouillon avec ses lignes : une seule suppression.
        let apres = b.base();
        let id = id_de(&b.conn, Entite::Document, &d).unwrap().unwrap();
        let admin = comptes::lire(
            &b.conn,
            id_de(&b.conn, Entite::Utilisateur, &b.admin)
                .unwrap()
                .unwrap(),
        )
        .unwrap()
        .unwrap();
        documents::supprimer_brouillon(&mut b.conn, &admin, id).unwrap();
        let ch = changements_depuis(&b.conn, apres, 100).unwrap();
        assert_eq!(ch.len(), 1);
        assert_eq!(ch[0].uid, d);
        assert_eq!(ch[0].etat, Etat::Supprime);
        assert!(
            changements_depuis(&b.conn, b.base(), 100)
                .unwrap()
                .is_empty()
        );
    }
}

/// Essais du jalon F5b : chaque action d'un poste appliquée par le serveur,
/// puis rejouée sur la copie d'un poste.
#[cfg(test)]
mod essais_actions {
    use super::*;
    use crate::replique::tests::{remplir, synchroniser};
    use crate::{ouvrir_en_memoire, replique};

    const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0];

    struct Banc {
        serveur: Connection,
        poste: Connection,
        admin: String,
        t1: String,
        p1: String,
    }

    impl Banc {
        fn nouveau() -> Banc {
            let mut serveur = ouvrir_en_memoire().unwrap();
            let r = remplir(&mut serveur);
            let mut poste = ouvrir_en_memoire().unwrap();
            synchroniser(&serveur, &mut poste, 500);
            let uid = |e, id| uid_de(&serveur, e, id).unwrap().unwrap();
            Banc {
                admin: uid(Entite::Utilisateur, r.admin.id),
                t1: uid(Entite::Tiers, r.t1),
                p1: uid(Entite::Produit, r.p1),
                serveur,
                poste,
            }
        }

        fn op(&self, compte: &str, action: Action) -> Operation {
            Operation {
                id: nouvel_uid(),
                entreprise: identifiant_entreprise(&self.serveur).unwrap(),
                poste: "poste-1".into(),
                compte: compte.into(),
                hors_connexion: false,
                heure_poste: None,
                base: dernier_changement(&self.serveur).unwrap(),
                action,
            }
        }

        fn faire(&mut self, action: Action) -> Issue {
            let op = self.op(&self.admin.clone(), action);
            appliquer(&mut self.serveur, &op).unwrap()
        }

        /// Action qui doit être acceptée.
        fn ok(&mut self, action: Action) -> Issue {
            let issue = self.faire(action.clone());
            assert!(
                matches!(issue, Issue::Acceptee { .. }),
                "{action:?} : {issue:?}"
            );
            issue
        }

        fn statut(&self, uid: &str) -> String {
            let id = id_de(&self.serveur, Entite::Document, uid)
                .unwrap()
                .unwrap();
            documents::lire(&self.serveur, id)
                .unwrap()
                .unwrap()
                .statut
                .code()
                .to_string()
        }
    }

    fn aujourdhui() -> String {
        chrono::Local::now()
            .date_naive()
            .format(FORMAT_DATE)
            .to_string()
    }

    fn tiers_proto(nom: &str, type_tiers: &str) -> proto::Tiers {
        proto::Tiers {
            nom: nom.into(),
            type_tiers: type_tiers.into(),
            ..proto::Tiers::default()
        }
    }

    fn produit_proto(reference: &str) -> proto::Produit {
        proto::Produit {
            designation: format!("Produit {reference}"),
            reference: reference.into(),
            prix: 2500,
            sens: "vente".into(),
        }
    }

    fn saisie(tiers: &str, produit: Option<&str>) -> proto::Saisie {
        proto::Saisie {
            date: aujourdhui(),
            tiers: Some(tiers.into()),
            libelle: "Essai F5b".into(),
            lignes: vec![proto::Ligne {
                produit: produit.map(String::from),
                reference: "A".into(),
                designation: "Article".into(),
                prix: 2500,
                quantite: 2_000,
                ..proto::Ligne::default()
            }],
            ..proto::Saisie::default()
        }
    }

    #[test]
    fn fiches_traduites_aller_retour() {
        for type_tiers in TypeTiers::TOUS {
            let f = FicheTiers {
                nom: "Client".into(),
                adresse: "Adresse".into(),
                telephone: "Téléphone".into(),
                localisation: "Localisation".into(),
                ifu: "IFU".into(),
                rccm: "RCCM".into(),
                regime_imposition: "Régime".into(),
                type_tiers,
            };
            assert_eq!(fiche_tiers(&tiers_pour_serveur(&f)).unwrap(), f);
        }
        for sens in Sens::TOUS {
            let f = FicheProduit {
                designation: "Ciment".into(),
                reference: "C1".into(),
                prix: 5000,
                sens,
            };
            assert_eq!(fiche_produit(&produit_pour_serveur(&f)).unwrap(), f);
        }
    }

    #[test]
    fn saisie_traduite_aller_retour() {
        let b = Banc::nouveau();
        let mut s = saisie(&b.t1, Some(&b.p1));
        s.remise = 500;
        s.tva = 1800;
        s.retenue = 200;
        s.mode_paiement = "virement".into();
        s.observations = "Livrer le matin".into();
        s.date_echeance = Some(aujourdhui());
        s.visa_client = "Client".into();
        s.date_livraison = "le 14".into();
        s.lignes.push(proto::Ligne {
            designation: "Ligne libre".into(),
            prix: 1000,
            quantite: 1_000,
            quantite_livree: 500,
            observations: "partielle".into(),
            ..proto::Ligne::default()
        });
        // Sur le poste, les identifiants propres diffèrent de ceux du serveur.
        let locale = saisie_locale(&b.poste, &s).unwrap();
        assert_eq!(saisie_pour_serveur(&b.poste, &locale).unwrap(), s);
        let mut orpheline = locale.clone();
        orpheline.tiers_id = Some(999_999);
        assert!(saisie_pour_serveur(&b.poste, &orpheline).is_err());
    }

    #[test]
    fn chaque_action_appliquee_puis_repliquee() {
        let mut b = Banc::nouveau();
        let t1 = b.t1.clone();
        let p1 = b.p1.clone();

        // Tiers.
        let t = nouvel_uid();
        b.ok(Action::CreerTiers {
            uid: t.clone(),
            fiche: tiers_proto("Client Delta", "client"),
        });
        b.ok(Action::ModifierTiers {
            uid: t.clone(),
            fiche: tiers_proto("Client Delta SA", "client"),
        });
        b.ok(Action::DefinirActifTiers {
            uid: t.clone(),
            actif: false,
        });
        b.ok(Action::DefinirActifTiers {
            uid: t.clone(),
            actif: true,
        });
        let importes = [nouvel_uid(), nouvel_uid()];
        let issue = b.ok(Action::ImporterTiers {
            fiches: vec![
                tiers_proto("Client Epsilon", "client"),
                tiers_proto("Client Alpha", "client"),
            ],
            uids: importes.to_vec(),
        });
        assert!(matches!(
            issue,
            Issue::Acceptee { bilan: Some(Bilan::Import { ajoutees: 1, ref ignorees }), .. }
                if ignorees.len() == 1 && ignorees[0].0 == 1
        ));
        assert!(
            id_de(&b.serveur, Entite::Tiers, &importes[0])
                .unwrap()
                .is_some()
        );
        assert!(
            id_de(&b.serveur, Entite::Tiers, &importes[1])
                .unwrap()
                .is_none()
        );
        let issue = b.ok(Action::RetirerTiers { uid: t.clone() });
        assert!(matches!(
            issue,
            Issue::Acceptee {
                bilan: Some(Bilan::Retrait { supprimee: true }),
                ..
            }
        ));

        // Produits.
        let p = nouvel_uid();
        b.ok(Action::CreerProduit {
            uid: p.clone(),
            fiche: produit_proto("Z1"),
        });
        b.ok(Action::ModifierProduit {
            uid: p.clone(),
            fiche: produit_proto("Z2"),
        });
        b.ok(Action::DefinirActifProduit {
            uid: p.clone(),
            actif: false,
        });
        let pi = nouvel_uid();
        b.ok(Action::ImporterProduits {
            fiches: vec![produit_proto("Z3")],
            uids: vec![pi.clone()],
        });
        assert!(id_de(&b.serveur, Entite::Produit, &pi).unwrap().is_some());
        b.ok(Action::RetirerProduit { uid: p.clone() });
        // Produit cité par un document : seulement désactivé.
        let issue = b.ok(Action::RetirerProduit { uid: p1.clone() });
        assert!(matches!(
            issue,
            Issue::Acceptee {
                bilan: Some(Bilan::Retrait { supprimee: false }),
                ..
            }
        ));
        b.ok(Action::DefinirActifProduit {
            uid: p1.clone(),
            actif: true,
        });

        // Devis validé, envoyé, converti ; facture réglée, livrée, avoir.
        let d = nouvel_uid();
        b.ok(Action::CreerBrouillon {
            uid: d.clone(),
            type_document: "devis".into(),
            saisie: saisie(&t1, Some(&p1)),
        });
        let mut s = saisie(&t1, Some(&p1));
        s.libelle = "Essai F5b modifié".into();
        b.ok(Action::ModifierBrouillon {
            uid: d.clone(),
            saisie: s,
        });
        let issue = b.ok(Action::Valider { uid: d.clone() });
        assert!(matches!(
            issue,
            Issue::Acceptee {
                numero: Some(_),
                ..
            }
        ));
        b.ok(Action::MarquerEnvoye { uid: d.clone() });
        b.ok(Action::ModifierAdministratif {
            uid: d.clone(),
            mode_paiement: "Virement".into(),
            observations: "Relu".into(),
        });
        let f = nouvel_uid();
        b.ok(Action::ConvertirEnFacture {
            uid: d.clone(),
            nouveau: f.clone(),
        });
        b.ok(Action::Valider { uid: f.clone() });
        let r = nouvel_uid();
        b.ok(Action::AjouterReglement {
            uid: r.clone(),
            document: f.clone(),
            date: aujourdhui(),
            montant: 1000,
            mode_paiement: "Espèces".into(),
            reference: String::new(),
        });
        b.ok(Action::SupprimerReglement { uid: r.clone() });
        b.ok(Action::AjouterReglement {
            uid: nouvel_uid(),
            document: f.clone(),
            date: aujourdhui(),
            montant: 500,
            mode_paiement: "Espèces".into(),
            reference: "R2".into(),
        });
        let l = nouvel_uid();
        b.ok(Action::CreerBordereau {
            uid: f.clone(),
            nouveau: l.clone(),
        });
        b.ok(Action::Valider { uid: l.clone() });
        b.ok(Action::ModifierLivraison {
            uid: l.clone(),
            visa_client: "Visa".into(),
            visa_fournisseur: String::new(),
            date_livraison: String::new(),
            date_reception: aujourdhui(),
        });
        b.ok(Action::ConfirmerLivraison { uid: l.clone() });
        let a = nouvel_uid();
        b.ok(Action::CreerAvoir {
            uid: f.clone(),
            nouveau: a.clone(),
        });
        let d2 = nouvel_uid();
        b.ok(Action::Dupliquer {
            uid: d.clone(),
            nouveau: d2.clone(),
            annuler_original: false,
        });
        b.ok(Action::SupprimerBrouillon { uid: d2.clone() });
        assert!(id_de(&b.serveur, Entite::Document, &d2).unwrap().is_none());
        let d3 = nouvel_uid();
        b.ok(Action::CreerBrouillon {
            uid: d3.clone(),
            type_document: "devis".into(),
            saisie: saisie(&t1, None),
        });
        b.ok(Action::Valider { uid: d3.clone() });
        b.ok(Action::Annuler { uid: d3.clone() });
        assert_eq!(b.statut(&d3), "annule");

        // Bon de commande envoyé puis reçu.
        let fournisseur = nouvel_uid();
        b.ok(Action::CreerTiers {
            uid: fournisseur.clone(),
            fiche: tiers_proto("Fournisseur Un", "fournisseur"),
        });
        let bc = nouvel_uid();
        b.ok(Action::CreerBrouillon {
            uid: bc.clone(),
            type_document: "bon_commande".into(),
            saisie: saisie(&fournisseur, None),
        });
        b.ok(Action::Valider { uid: bc.clone() });
        b.ok(Action::MarquerEnvoye { uid: bc.clone() });
        b.ok(Action::EnregistrerReception {
            uid: bc.clone(),
            recues: vec![2_000],
        });

        // Comptes.
        let c = nouvel_uid();
        b.ok(Action::CreerCompte {
            uid: c.clone(),
            nom_complet: "Compte Deux".into(),
            identifiant: "deux".into(),
            mot_de_passe: "secret-creation-1".into(),
            role: "commercial".into(),
        });
        b.ok(Action::ChangerRole {
            uid: c.clone(),
            role: "comptable".into(),
        });
        b.ok(Action::DefinirActifCompte {
            uid: c.clone(),
            actif: false,
        });
        b.ok(Action::DefinirActifCompte {
            uid: c.clone(),
            actif: true,
        });
        b.ok(Action::ReinitialiserMotDePasse {
            uid: c.clone(),
            nouveau: "secret-reinit-2".into(),
        });
        b.ok(Action::ChangerMotDePasse {
            ancien: "motdepasse1".into(),
            nouveau: "secret-change-3".into(),
        });
        b.ok(Action::ChangerLangue {
            langue: "en".into(),
        });
        b.ok(Action::ChangerAffichage {
            navigation: "barre".into(),
            animations: false,
        });

        // Réglages partagés.
        let mut prm = parametres::lire(&b.serveur).unwrap();
        prm.modes_paiement.push("Chèque".into());
        prm.dossiers_export[0] = "C:\\poste\\devis".into();
        b.ok(Action::EnregistrerParametres {
            contenu: parametres::en_json(&prm),
        });
        let lu = parametres::lire(&b.serveur).unwrap();
        assert!(lu.modes_paiement.contains(&"Chèque".to_string()));
        assert_eq!(lu.dossiers_export[0], "", "dossiers du poste jamais écrits");
        let mut e = entreprise::lire(&b.serveur).unwrap().unwrap();
        e.telephone = "70 00 00 00".into();
        b.ok(Action::EnregistrerEntreprise {
            contenu: e.en_json(),
        });
        b.ok(Action::DefinirLogo {
            logo: "secondaire".into(),
            octets: Some(BASE64.encode(PNG)),
        });
        let matrice = droits::lire_matrice(&b.serveur).unwrap();
        b.ok(Action::EnregistrerMatrice {
            contenu: droits::matrice_en_json(&matrice),
        });
        let mut textes = relances::lire_textes(&b.serveur).unwrap();
        textes.objet = "Relance de paiement".into();
        b.ok(Action::EnregistrerTextesRelance {
            contenu: relances::en_json(&textes),
        });
        let m = mise_en_page::lire_analysee(&b.serveur).unwrap();
        b.ok(Action::EnregistrerMiseEnPage {
            contenu: mise_en_page::en_json(&m),
        });
        let bas = mise_en_page::lire_bas(&b.serveur, TypeDocument::Facture).unwrap();
        b.ok(Action::EnregistrerBas {
            type_document: "facture".into(),
            contenu: mise_en_page::bas_en_json(&bas),
        });

        // Aucun mot de passe ni logo dans l'historique des opérations.
        let gardes: Vec<String> = b
            .serveur
            .prepare("SELECT contenu FROM operation")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(gardes.len() >= 40);
        for contenu in &gardes {
            assert!(!contenu.contains("secret-"), "{contenu}");
            assert!(!contenu.contains("motdepasse1"), "{contenu}");
            assert!(!contenu.contains(&BASE64.encode(PNG)), "{contenu}");
        }

        // La copie du poste suit tout.
        synchroniser(&b.serveur, &mut b.poste, 7);
        assert_eq!(replique::en_attente(&b.poste).unwrap(), 0);
    }

    #[test]
    fn renvoi_reconnu_sans_mot_de_passe_garde() {
        let mut b = Banc::nouveau();
        let op = b.op(
            &b.admin.clone(),
            Action::ChangerMotDePasse {
                ancien: "motdepasse1".into(),
                nouveau: "secret-nouveau".into(),
            },
        );
        assert_eq!(appliquer(&mut b.serveur, &op).unwrap(), Issue::acceptee());
        // Réponse perdue : le même envoi rend le même résultat, sans rien refaire.
        assert_eq!(appliquer(&mut b.serveur, &op).unwrap(), Issue::acceptee());
        let id = id_de(&b.serveur, Entite::Utilisateur, &b.admin)
            .unwrap()
            .unwrap();
        let admin = comptes::lire(&b.serveur, id).unwrap().unwrap();
        assert!(comptes::connecter(&b.serveur, &admin.identifiant, "secret-nouveau").is_ok());
    }

    #[test]
    fn lecture_seule_laisse_passer_les_reglages_personnels() {
        let mut b = Banc::nouveau();
        b.serveur
            .execute(
                "INSERT INTO temp.abonnement_etat (id, lecture_seule, echeance)
                 VALUES (1, 1, '2026-11-01')",
                [],
            )
            .unwrap();
        let personnelles = [
            Action::ChangerLangue {
                langue: "en".into(),
            },
            Action::ChangerAffichage {
                navigation: "barre".into(),
                animations: true,
            },
            Action::ChangerMotDePasse {
                ancien: "motdepasse1".into(),
                nouveau: "secret-lecture-seule".into(),
            },
        ];
        for action in personnelles {
            b.ok(action);
        }
        let admin = b.admin.clone();
        let t1 = b.t1.clone();
        for action in [
            Action::ReinitialiserMotDePasse {
                uid: admin.clone(),
                nouveau: "secret-refuse".into(),
            },
            Action::ModifierTiers {
                uid: t1,
                fiche: tiers_proto("Refusé", "client"),
            },
            Action::EnregistrerTextesRelance {
                contenu: relances::en_json(&relances::lire_textes(&b.serveur).unwrap()),
            },
        ] {
            let op = b.op(&admin, action);
            assert!(matches!(
                appliquer(&mut b.serveur, &op),
                Err(Erreur::LectureSeule { .. })
            ));
        }
    }

    #[test]
    fn reglages_changes_ailleurs_depasses() {
        let mut b = Banc::nouveau();
        let base = dernier_changement(&b.serveur).unwrap();
        // Un autre poste change les paramètres après la base de celui-ci.
        let mut autre = b.op(
            &b.admin.clone(),
            Action::EnregistrerTextesRelance {
                contenu: relances::en_json(&relances::lire_textes(&b.serveur).unwrap()),
            },
        );
        autre.poste = "poste-2".into();
        assert_eq!(
            appliquer(&mut b.serveur, &autre).unwrap(),
            Issue::acceptee()
        );
        let mut op = b.op(
            &b.admin.clone(),
            Action::EnregistrerParametres {
                contenu: parametres::en_json(&parametres::lire(&b.serveur).unwrap()),
            },
        );
        op.base = base;
        assert_eq!(appliquer(&mut b.serveur, &op).unwrap(), Issue::Depassee);
        // Rien n'est suspendu : une nouvelle tentative à jour passe.
        b.ok(Action::EnregistrerParametres {
            contenu: parametres::en_json(&parametres::lire(&b.serveur).unwrap()),
        });
    }
}
