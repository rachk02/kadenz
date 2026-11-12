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

use chrono::NaiveDate;
use kadenz_domaine::calcul::Taux;
use kadenz_domaine::document::TypeDocument;
use kadenz_domaine::referentiel::{FicheTiers, TypeTiers};
use kadenz_protocole::{self as proto, Action, Changement, Entite, Etat, Issue, Operation};
use rusqlite::{Connection, OptionalExtension, params};

use crate::comptes::{self, Utilisateur};
use crate::documents::{self, ChampsLivraison, LigneSaisie, Saisie};
use crate::{Erreur, Resultat, maintenant, tiers};

/// Tables dont les lignes portent un identifiant global.
const TABLES: [&str; 5] = ["utilisateur", "tiers", "produit", "document", "reglement"];

const CLE_ENTREPRISE: &str = "identifiant_entreprise";

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

fn completer_uids(conn: &Connection) -> Resultat<()> {
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

/// Identifiant global d'une ligne.
pub fn uid_de(conn: &Connection, entite: Entite, id: i64) -> Resultat<Option<String>> {
    Ok(conn
        .query_row(
            &format!("SELECT uid FROM {} WHERE id = ?1", entite.code()),
            [id],
            |r| r.get(0),
        )
        .optional()?
        .flatten())
}

/// Identifiant propre à cette base d'une ligne désignée par son identifiant global.
pub fn id_de(conn: &Connection, entite: Entite, uid: &str) -> Resultat<Option<i64>> {
    Ok(conn
        .query_row(
            &format!("SELECT id FROM {} WHERE uid = ?1", entite.code()),
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
    let contenu = serde_json::to_string(op).map_err(|e| Erreur::Validation(e.to_string()))?;
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

/// Erreur de protocole : jamais enregistrée comme résultat.
fn refus_op(message: &str) -> Erreur {
    Erreur::Validation(message.into())
}

fn appliquer_dans(conn: &mut Connection, op: &Operation, contenu: &str) -> Resultat<Issue> {
    let (entite, cible) = op.action.cible();
    let suspendue = conn
        .query_row(
            "SELECT 1 FROM operation_suspendue WHERE cible = ?1 AND ouverte = 1",
            [cible],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    let issue = if suspendue {
        Issue::Bloquee
    } else if modifiee_ailleurs(conn, op, entite, cible)? {
        match op.action {
            Action::Valider { .. } => Issue::Depassee,
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
            params![op.id, motif, cible],
        )?;
    }
    Ok(issue)
}

/// Refus d'une règle de la V1 : rendu au poste, ou mis en attente pour un
/// compte ou un droit hors connexion. Les erreurs techniques remontent.
fn issue_du_refus(e: Erreur, hors_connexion: bool) -> Resultat<Issue> {
    Ok(match e {
        Erreur::Interdit(_) if hors_connexion => Issue::EnAttente,
        Erreur::ValidationComposee { modele, valeurs } => Issue::Refusee { modele, valeurs },
        Erreur::Sqlite(_) | Erreur::Io(_) | Erreur::Hachage(_) => return Err(e),
        autre => Issue::Refusee {
            modele: autre.to_string(),
            valeurs: Vec::new(),
        },
    })
}

/// Conflit : la ligne a changé depuis la base de l'opération, et pas seulement
/// par ce poste (deux modifications successives d'un même poste hors connexion
/// ne sont pas en conflit entre elles). Une création n'a pas de conflit.
fn modifiee_ailleurs(
    conn: &Connection,
    op: &Operation,
    entite: Entite,
    cible: &str,
) -> Resultat<bool> {
    if matches!(
        op.action,
        Action::CreerTiers { .. } | Action::CreerBrouillon { .. }
    ) {
        return Ok(false);
    }
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
    if id_de(conn, entite, uid)?.is_some() {
        return Err(Erreur::Validation(
            "identifiant déjà attribué à une autre ligne".into(),
        ));
    }
    conn.execute(
        &format!("UPDATE {} SET uid = ?1 WHERE id = ?2", entite.code()),
        params![uid, id],
    )?;
    Ok(())
}

fn executer(conn: &mut Connection, op: &Operation) -> Resultat<Issue> {
    let qui = acteur(conn, &op.compte)?;
    let acceptee = Issue::Acceptee { numero: None };
    match &op.action {
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
        Action::CreerBrouillon {
            uid,
            type_document,
            saisie,
        } => {
            let type_document = TypeDocument::depuis_code(type_document)
                .ok_or_else(|| Erreur::Validation("opération illisible".into()))?;
            let s = saisie_locale(conn, saisie)?;
            let id = documents::creer_brouillon(conn, &qui, type_document, &s)?;
            poser_uid(conn, Entite::Document, id, uid)?;
            Ok(acceptee)
        }
        Action::ModifierBrouillon { uid, saisie } => {
            let id = exiger_id(conn, Entite::Document, uid)?;
            let s = saisie_locale(conn, saisie)?;
            documents::modifier_brouillon(conn, &qui, id, &s)?;
            Ok(acceptee)
        }
        Action::Valider { uid } => {
            let id = exiger_id(conn, Entite::Document, uid)?;
            let numero = documents::valider(conn, &qui, id)?;
            Ok(Issue::Acceptee {
                numero: Some(numero),
            })
        }
    }
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

fn tiers_proto(f: &FicheTiers) -> proto::Tiers {
    proto::Tiers {
        nom: f.nom.clone(),
        adresse: f.adresse.clone(),
        telephone: f.telephone.clone(),
        localisation: f.localisation.clone(),
        ifu: f.ifu.clone(),
        rccm: f.rccm.clone(),
        regime_imposition: f.regime_imposition.clone(),
        type_tiers: f.type_tiers.code().into(),
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

fn saisie_proto(conn: &Connection, s: &Saisie) -> Resultat<proto::Saisie> {
    let uid = |entite, id: Option<i64>| -> Resultat<Option<String>> {
        match id {
            Some(id) => uid_de(conn, entite, id),
            None => Ok(None),
        }
    };
    let mut lignes = Vec::with_capacity(s.lignes.len());
    for l in &s.lignes {
        lignes.push(proto::Ligne {
            produit: uid(Entite::Produit, l.produit_id)?,
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
        tiers: uid(Entite::Tiers, s.tiers_id)?,
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
    for (seq, code, id) in lignes {
        let Some(entite) = Entite::depuis_code(&code) else {
            continue;
        };
        let present = uid_de(conn, entite, id)?;
        let etat = if present.is_none()
            && conn
                .query_row(&format!("SELECT 1 FROM {code} WHERE id = ?1"), [id], |_| {
                    Ok(())
                })
                .optional()?
                .is_none()
        {
            Etat::Supprime
        } else {
            etat(conn, entite, id)?
        };
        // Ligne disparue : son identifiant est celui noté lors de sa suppression.
        let uid = match present {
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
            sortie.push(Changement {
                seq: seq as u64,
                entite,
                uid,
                etat,
            });
        }
    }
    Ok(sortie)
}

fn etat(conn: &Connection, entite: Entite, id: i64) -> Resultat<Etat> {
    Ok(match entite {
        Entite::Tiers => match tiers::lire(conn, id)? {
            Some(t) => Etat::Tiers {
                fiche: tiers_proto(&t.fiche),
                actif: t.actif,
            },
            None => Etat::Supprime,
        },
        Entite::Document => match documents::lire(conn, id)? {
            Some(d) => Etat::Document {
                type_document: d.type_document.code().into(),
                statut: d.statut.code().into(),
                numero: d.numero.clone(),
                saisie: saisie_proto(conn, &d.saisie)?,
            },
            None => Etat::Supprime,
        },
        Entite::Utilisateur | Entite::Produit | Entite::Reglement => Etat::NonTransmis,
    })
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
        assert_eq!(b.appliquer(&creer_t), Issue::Acceptee { numero: None });
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
        assert_eq!(b.appliquer(&creer_d), Issue::Acceptee { numero: None });
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
        assert_eq!(b.appliquer(&op), Issue::Acceptee { numero: None });
        assert_eq!(b.appliquer(&op), Issue::Acceptee { numero: None });
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
        } = b.appliquer(&valider)
        else {
            panic!("numéro attendu");
        };
        // Réponse perdue : le poste renvoie la même opération.
        assert_eq!(
            b.appliquer(&valider),
            Issue::Acceptee {
                numero: Some(numero.clone())
            }
        );
        assert_eq!(b.compteur(), 1);
        assert_eq!(b.statut(&d), "valide");
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
            Issue::Acceptee { numero: Some(_) }
        ));
        assert_eq!(b.compteur(), 1);
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
        assert!(matches!(b.appliquer(&op), Issue::Refusee { .. }));
        // Hors connexion : mise en attente, et la fiche bloquée derrière elle.
        op.id = nouvel_uid();
        op.hors_connexion = true;
        assert_eq!(b.appliquer(&op), Issue::EnAttente);
        let admin = b.admin.clone();
        let suivante = b.op(
            "p1",
            &admin,
            b.base(),
            Action::ModifierTiers {
                uid: t.clone(),
                fiche: fiche("Autre nom"),
            },
        );
        assert_eq!(b.appliquer(&suivante), Issue::Bloquee);
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
            assert_eq!(b.appliquer(&op), Issue::Acceptee { numero: None });
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
        assert_eq!(b.appliquer(&modif), Issue::Acceptee { numero: None });
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
        assert_eq!(
            appliquer(&mut conn, &creer).unwrap(),
            Issue::Acceptee { numero: None }
        );
        let base = dernier_changement(&conn).unwrap();
        let mut valider = op(Action::Valider { uid: d });
        valider.base = base;
        let Issue::Acceptee {
            numero: Some(numero),
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
        assert!(
            matches!(&tiers_ch.etat, Etat::Tiers { fiche, actif: true } if fiche.nom == "Client Alpha")
        );
        let doc = ch.iter().find(|c| c.uid == d).expect("document");
        assert!(matches!(&doc.etat, Etat::Document { statut, saisie, .. }
            if statut == "brouillon" && saisie.tiers.as_deref() == Some(t.as_str())));
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
