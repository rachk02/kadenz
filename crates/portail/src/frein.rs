//! Connexions et activations freinées (spécification du portail, sections 6
//! et 9). Après un nombre d'échecs permis par clé, attente de 5 secondes
//! doublée à chaque échec, plafonnée selon la règle de la clé.
//!
//! Le frein est gardé dans la base (table `frein`, schéma 4) : un redémarrage
//! du portail ne l'efface pas. Il reste borné, parce que ses clés viennent en
//! partie de la saisie du client : une ligne sans attente en cours et sans
//! échec depuis un jour est purgée ; au-delà de `LIGNES_MAX`, les plus
//! anciennes lignes sans attente en cours sont retirées. Une attente en cours
//! n'est jamais retirée pour faire de la place : des clés de pacotille ne
//! lèvent pas la pénalité d'une adresse ou d'un compte.
//!
//! Les clés sont construites ici, après validation de la saisie, pour que les
//! appelants (connexion, activation, et la ligne V1 ensuite) suivent la même
//! règle.

use rusqlite::{Connection, OptionalExtension, params};

/// Règle d'une clé : échecs permis avant la première attente, attente
/// maximale en secondes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Regle {
    permis: u32,
    attente_max: i64,
}

/// Adresse, ou identifiant depuis une adresse : 5 échecs, jusqu'à 15 minutes.
pub(crate) const STRICTE: Regle = Regle {
    permis: 5,
    attente_max: 15 * 60,
};

/// Identifiant seul, toutes adresses confondues : freine une attaque répartie
/// sur beaucoup d'adresses sans laisser un tiers bloquer le titulaire, qui
/// n'attend au pire qu'une minute.
pub(crate) const LARGE: Regle = Regle {
    permis: 50,
    attente_max: 60,
};

/// Liens de téléchargement depuis une adresse : chaque lien compte ; vingt
/// sans attente (un bureau derrière une même adresse), puis jusqu'à 15 minutes.
pub(crate) const TELECHARGEMENT: Regle = Regle {
    permis: 20,
    attente_max: 15 * 60,
};

const ATTENTE_MIN: i64 = 5;
/// Lignes gardées au plus (hors attentes en cours).
pub(crate) const LIGNES_MAX: i64 = 10_000;
/// Une ligne sans échec depuis un jour, et sans attente en cours, est purgée.
const OUBLI: i64 = 24 * 3600;

/// Clé du frein avec sa règle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Cle {
    texte: String,
    regle: Regle,
}

/// Clés d'une connexion : l'adresse, l'identifiant depuis cette adresse
/// (règle stricte) et l'identifiant seul (règle large). Cinq échecs depuis
/// une adresse ne bloquent donc pas le titulaire qui se connecte d'ailleurs.
pub(crate) fn cles_connexion(identifiant: &str, adresse: &str) -> Vec<Cle> {
    vec![
        Cle {
            texte: format!("ip:{adresse}"),
            regle: STRICTE,
        },
        Cle {
            texte: format!("id-ip:{identifiant}|{adresse}"),
            regle: STRICTE,
        },
        Cle {
            texte: format!("id:{identifiant}"),
            regle: LARGE,
        },
    ]
}

/// Titulaire d'une activation, déjà validé : entreprise (identifiant de la
/// base, gamme entreprise) ou poste (identifiant de poste, ligne V1).
#[derive(Clone, Copy, Debug)]
pub(crate) enum Titulaire<'a> {
    Entreprise(&'a str),
    Poste(&'a str),
}

/// Clés d'une activation : l'adresse, puis le titulaire seulement s'il est
/// valide (sinon la clé viendrait d'un texte quelconque).
pub(crate) fn cles_activation(titulaire: Option<Titulaire>, adresse: &str) -> Vec<Cle> {
    let mut cles = vec![Cle {
        texte: format!("activation:ip:{adresse}"),
        regle: STRICTE,
    }];
    let titulaire = match titulaire {
        Some(Titulaire::Entreprise(e)) => Some(format!("ent:{e}")),
        Some(Titulaire::Poste(p)) => Some(format!("poste:{p}")),
        None => None,
    };
    if let Some(t) = titulaire {
        cles.push(Cle {
            texte: format!("activation:{t}|{adresse}"),
            regle: STRICTE,
        });
        cles.push(Cle {
            texte: format!("activation:{t}"),
            regle: LARGE,
        });
    }
    cles
}

/// Clés d'un état de licence (décision 80) : comme l'activation, l'adresse et
/// l'entreprise ; seuls les échecs comptent.
pub(crate) fn cles_etat(titulaire: Option<Titulaire>, adresse: &str) -> Vec<Cle> {
    cles_activation(titulaire, adresse)
        .into_iter()
        .map(|c| Cle {
            texte: c.texte.replacen("activation:", "etat:", 1),
            regle: c.regle,
        })
        .collect()
}

/// Clé d'une demande de lien de téléchargement : l'adresse seule.
pub(crate) fn cles_telechargement(adresse: &str) -> Vec<Cle> {
    vec![Cle {
        texte: format!("telechargement:ip:{adresse}"),
        regle: TELECHARGEMENT,
    }]
}

/// Clé d'une demande d'essai : l'adresse seule, chaque demande compte (un
/// programme la renvoie à chaque démarrage tant qu'elle n'est pas confirmée).
pub(crate) fn cles_essai(adresse: &str) -> Vec<Cle> {
    vec![Cle {
        texte: format!("essai:ip:{adresse}"),
        regle: TELECHARGEMENT,
    }]
}

/// Secondes d'attente restantes pour l'une des clés, s'il y en a.
pub(crate) fn attente(
    conn: &Connection,
    cles: &[Cle],
    maintenant: i64,
) -> rusqlite::Result<Option<u64>> {
    let mut plus_longue: Option<u64> = None;
    for cle in cles {
        let jusqua: Option<i64> = conn
            .query_row(
                "SELECT jusqua FROM frein WHERE cle = ?1",
                [&cle.texte],
                |r| r.get(0),
            )
            .optional()?
            .flatten();
        if let Some(j) = jusqua.filter(|j| *j > maintenant) {
            let reste = (j - maintenant) as u64;
            plus_longue = Some(plus_longue.map_or(reste, |p| p.max(reste)));
        }
    }
    Ok(plus_longue)
}

/// Refus pendant une attente : vrai si l'une des clés en attente n'a pas
/// encore de refus noté pour cette attente, qui est alors marquée. Le
/// journal reçoit ainsi au plus une entrée par clé et par attente : une
/// attaque en force reste visible sans noyer le journal.
pub(crate) fn refus_a_noter(
    conn: &Connection,
    cles: &[Cle],
    maintenant: i64,
) -> rusqlite::Result<bool> {
    let mut nouveau = false;
    for cle in cles {
        let marquees = conn.execute(
            "UPDATE frein SET signale = jusqua
             WHERE cle = ?1 AND jusqua > ?2 AND (signale IS NULL OR signale <> jusqua)",
            params![cle.texte, maintenant],
        )?;
        nouveau |= marquees > 0;
    }
    Ok(nouveau)
}

/// Compte un échec pour chaque clé, puis borne la table.
pub(crate) fn echec(conn: &Connection, cles: &[Cle], maintenant: i64) -> rusqlite::Result<()> {
    for cle in cles {
        let echecs: u32 = conn
            .query_row(
                "SELECT echecs FROM frein WHERE cle = ?1",
                [&cle.texte],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or(0u32)
            .saturating_add(1);
        let jusqua = (echecs >= cle.regle.permis).then(|| {
            let rang = (echecs - cle.regle.permis).min(16);
            maintenant + (ATTENTE_MIN << rang).min(cle.regle.attente_max)
        });
        conn.execute(
            "INSERT INTO frein (cle, echecs, jusqua, derniere) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (cle) DO UPDATE SET echecs = ?2, jusqua = ?3, derniere = ?4",
            params![cle.texte, echecs, jusqua, maintenant],
        )?;
    }
    borner(conn, maintenant)
}

/// Une réussite efface les clés de la demande.
pub(crate) fn reussite(conn: &Connection, cles: &[Cle]) -> rusqlite::Result<()> {
    for cle in cles {
        conn.execute("DELETE FROM frein WHERE cle = ?1", [&cle.texte])?;
    }
    Ok(())
}

/// Purge les lignes oubliées, puis retire les plus anciennes lignes sans
/// attente en cours au-delà de `LIGNES_MAX`.
fn borner(conn: &Connection, maintenant: i64) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM frein WHERE derniere < ?1 AND (jusqua IS NULL OR jusqua <= ?2)",
        params![maintenant - OUBLI, maintenant],
    )?;
    let libres: i64 = conn.query_row(
        "SELECT count(*) FROM frein WHERE jusqua IS NULL OR jusqua <= ?1",
        [maintenant],
        |r| r.get(0),
    )?;
    if libres > LIGNES_MAX {
        conn.execute(
            "DELETE FROM frein WHERE cle IN (
                 SELECT cle FROM frein WHERE jusqua IS NULL OR jusqua <= ?1
                 ORDER BY derniere, cle LIMIT ?2)",
            params![maintenant, libres - LIGNES_MAX],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(crate::base::TABLE_FREIN).unwrap();
        conn
    }

    fn cle(texte: &str, regle: Regle) -> Vec<Cle> {
        vec![Cle {
            texte: texte.into(),
            regle,
        }]
    }

    #[test]
    fn cinq_echecs_puis_attente_doublee() {
        let conn = base();
        let cles = cle("ip:a", STRICTE);
        let t = 1_000;
        for _ in 0..4 {
            echec(&conn, &cles, t).unwrap();
            assert_eq!(attente(&conn, &cles, t).unwrap(), None);
        }
        echec(&conn, &cles, t).unwrap();
        assert_eq!(attente(&conn, &cles, t).unwrap(), Some(5));
        echec(&conn, &cles, t).unwrap();
        assert_eq!(attente(&conn, &cles, t).unwrap(), Some(10));
        for _ in 0..20 {
            echec(&conn, &cles, t).unwrap();
        }
        assert_eq!(attente(&conn, &cles, t).unwrap(), Some(15 * 60));
        assert_eq!(attente(&conn, &cles, t + 15 * 60).unwrap(), None);
        reussite(&conn, &cles).unwrap();
        assert_eq!(attente(&conn, &cles, t).unwrap(), None);
    }

    #[test]
    fn regle_large_plafonnee_a_une_minute() {
        let conn = base();
        let cles = cle("id:editeur", LARGE);
        for _ in 0..49 {
            echec(&conn, &cles, 0).unwrap();
        }
        assert_eq!(attente(&conn, &cles, 0).unwrap(), None);
        for _ in 0..30 {
            echec(&conn, &cles, 0).unwrap();
        }
        assert_eq!(attente(&conn, &cles, 0).unwrap(), Some(60));
    }

    /// Des clés de pacotille, au-delà du plafond, ne lèvent pas une attente
    /// en cours ; la table reste bornée.
    #[test]
    fn table_bornee_sans_lever_une_attente() {
        let conn = base();
        let penalisee = cle("ip:attaquant", STRICTE);
        for _ in 0..5 {
            echec(&conn, &penalisee, 0).unwrap();
        }
        assert_eq!(attente(&conn, &penalisee, 0).unwrap(), Some(5));
        // Lignes libres déjà au plafond, puis encore cent clés nouvelles.
        conn.execute_batch("BEGIN").unwrap();
        for k in 0..LIGNES_MAX {
            conn.execute(
                "INSERT INTO frein (cle, echecs, jusqua, derniere) VALUES (?1, 1, NULL, 0)",
                [format!("id:pacotille{k}")],
            )
            .unwrap();
        }
        conn.execute_batch("COMMIT").unwrap();
        for k in 0..100 {
            echec(&conn, &cle(&format!("id:nouvelle{k}"), STRICTE), 1).unwrap();
        }
        let libres: i64 = conn
            .query_row(
                "SELECT count(*) FROM frein WHERE jusqua IS NULL OR jusqua <= 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(libres, LIGNES_MAX);
        assert_eq!(attente(&conn, &penalisee, 1).unwrap(), Some(4));
    }

    #[test]
    fn lignes_oubliees_apres_un_jour() {
        let conn = base();
        echec(&conn, &cle("id:ancien", STRICTE), 0).unwrap();
        echec(&conn, &cle("id:recent", STRICTE), OUBLI + 1).unwrap();
        let restantes: Vec<String> = conn
            .prepare("SELECT cle FROM frein ORDER BY cle")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(restantes, vec!["id:recent".to_string()]);
    }

    #[test]
    fn cle_d_entreprise_seulement_si_valide() {
        assert_eq!(cles_activation(None, "1.2.3.4").len(), 1);
        assert_eq!(
            cles_activation(
                Some(Titulaire::Entreprise(
                    "0f8e1d2c-3b4a-4596-8778-695a4b3c2d1e"
                )),
                "1.2.3.4"
            )
            .len(),
            3
        );
        let poste = cles_activation(Some(Titulaire::Poste("ABCD-EFGH-JKMN-PQRS")), "1.2.3.4");
        assert_eq!(poste[2].texte, "activation:poste:ABCD-EFGH-JKMN-PQRS");
    }
}
