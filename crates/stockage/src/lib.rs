//! Accès SQLite de Kadenz (cahier des charges, sections 1, 10.3 et 11).

pub mod abonnement;
pub mod audit;
pub mod chiffrement;
pub mod comptes;
pub mod documents;
pub mod droits;
pub mod entreprise;
pub mod impression;
pub mod inscription;
pub mod installation;
pub mod licence;
pub mod mise_en_page;
pub mod parametres;
pub mod pilotage;
pub mod poste_relie;
pub mod postes;
pub mod produits;
pub mod reglements;
pub mod relances;
pub mod replique;
pub mod sauvegarde;
pub mod synchro;
pub mod tiers;

use std::fmt;
use std::path::Path;

use kadenz_domaine::document::TypeDocument;
use kadenz_domaine::role::Droit;
use rusqlite::{Connection, params};

/// Connexion à la base, ré-exportée pour que l'application n'ait pas à dépendre
/// directement de `rusqlite`.
pub type Connexion = Connection;

/// Horodatage local au format ISO 8601, à la seconde.
pub(crate) fn maintenant() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

/// Place chaque valeur dans le `{}` suivant du modèle ; un `{}` sans valeur reste tel quel.
pub fn composer(modele: &str, valeurs: &[String]) -> String {
    let mut sortie = String::with_capacity(modele.len());
    let mut valeurs = valeurs.iter();
    let mut reste = modele;
    while let Some(position) = reste.find("{}") {
        sortie.push_str(&reste[..position]);
        sortie.push_str(valeurs.next().map_or("{}", String::as_str));
        reste = &reste[position + 2..];
    }
    sortie.push_str(reste);
    sortie
}

/// Refus composé : `modele` est un texte fixe à traduire, chaque `{}` reçoit la
/// valeur suivante. Une valeur est un numéro, un nom ou un libellé autonome
/// (statut, type de document), jamais un fragment de phrase à accorder.
pub(crate) fn refus(modele: &str, valeurs: &[&dyn fmt::Display]) -> Erreur {
    Erreur::ValidationComposee {
        modele: modele.to_string(),
        valeurs: valeurs.iter().map(|v| v.to_string()).collect(),
    }
}

/// Migrations embarquées, appliquées dans l'ordre. Le rang d'une migration
/// correspond à `PRAGMA user_version` une fois appliquée. Ne jamais modifier
/// une migration publiée : en ajouter une nouvelle.
const MIGRATIONS: &[&str] = &[
    include_str!("migrations/0001_initial.sql"),
    include_str!("migrations/0002_langue.sql"),
    include_str!("migrations/0003_instantane_entreprise.sql"),
    include_str!("migrations/0004_lien_documents.sql"),
    include_str!("migrations/0005_bordereau.sql"),
    include_str!("migrations/0006_affichage.sql"),
    include_str!("migrations/0007_synchro.sql"),
    include_str!("migrations/0008_postes.sql"),
    include_str!("migrations/0009_inscription.sql"),
    include_str!("migrations/0010_replique.sql"),
    include_str!("migrations/0011_parametres_bas.sql"),
    include_str!("migrations/0012_file_envoi.sql"),
];

#[derive(Debug)]
pub enum Erreur {
    Sqlite(rusqlite::Error),
    Io(std::io::Error),
    /// La base a été créée par une version plus récente de l'application.
    VersionSchemaInconnue {
        base: u32,
        application: u32,
    },
    /// Saisie refusée ; le message, fixe, est destiné à l'utilisateur.
    Validation(String),
    /// Saisie refusée, message composé : `modele` est le texte fixe à traduire,
    /// chaque `{}` reçoit la valeur suivante (voir [`composer`]). Le `Display`
    /// donne la phrase française composée.
    ValidationComposee {
        modele: String,
        valeurs: Vec<String>,
    },
    /// Identifiant inconnu, mot de passe faux ou compte désactivé (sans préciser lequel).
    ConnexionRefusee,
    /// L'opération laisserait l'installation sans administrateur actif.
    DernierAdmin,
    /// L'installation initiale a déjà été faite.
    DejaInstalle,
    Hachage(String),
    /// Le compte qui agit n'a pas le droit demandé.
    Interdit(Droit),
    /// Base chiffrée dont le fichier `kadenz.cle` a disparu.
    CleIntrouvable,
    /// `kadenz.cle` protégée pour un autre poste ou un autre compte Windows.
    CleAutreCompte,
    /// `kadenz.cle` déprotégée mais son contenu n'est pas une clé.
    CleEndommagee,
    /// La clé ne déchiffre pas la base : clé ou fichier de la base endommagé.
    BaseIndechiffrable,
    /// Abonnement échu ou sans licence valide : saisie et validation bloquées
    /// (phase 2, F3) ; `echeance` de la licence en place s'il y en a une.
    LectureSeule {
        echeance: Option<String>,
    },
}

impl fmt::Display for Erreur {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Erreur::Sqlite(e) => write!(f, "erreur de base de données : {e}"),
            Erreur::Io(e) => write!(f, "erreur de fichier : {e}"),
            Erreur::Validation(message) => f.write_str(message),
            Erreur::ValidationComposee { modele, valeurs } => {
                f.write_str(&composer(modele, valeurs))
            }
            Erreur::ConnexionRefusee => f.write_str("identifiant ou mot de passe incorrect"),
            Erreur::DernierAdmin => f.write_str(
                "impossible : l'installation doit garder au moins un administrateur actif",
            ),
            Erreur::DejaInstalle => f.write_str("l'installation initiale a déjà été faite"),
            Erreur::Hachage(e) => write!(f, "erreur de hachage du mot de passe : {e}"),
            Erreur::Interdit(droit) => {
                write!(f, "action non autorisée pour votre rôle : « {droit} »")
            }
            Erreur::CleIntrouvable => f.write_str("clé de la base introuvable"),
            Erreur::CleAutreCompte => f.write_str(
                "clé de la base illisible : elle a été créée sur un autre poste ou un autre compte Windows",
            ),
            Erreur::CleEndommagee => f.write_str("clé de la base endommagée"),
            Erreur::LectureSeule { .. } => f.write_str(abonnement::MODELE_LECTURE_SEULE),
            Erreur::BaseIndechiffrable => f.write_str(
                "la base ne s'ouvre pas avec sa clé : clé ou fichier de la base endommagé",
            ),
            Erreur::VersionSchemaInconnue { base, application } => write!(
                f,
                "la base est au schéma {base}, cette version de l'application ne connaît que le schéma {application}"
            ),
        }
    }
}

impl std::error::Error for Erreur {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Erreur::Sqlite(e) => Some(e),
            Erreur::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for Erreur {
    fn from(e: rusqlite::Error) -> Self {
        Erreur::Sqlite(e)
    }
}

impl From<kadenz_domaine::message::MessageCompose> for Erreur {
    fn from(m: kadenz_domaine::message::MessageCompose) -> Self {
        Erreur::ValidationComposee {
            modele: m.modele.to_owned(),
            valeurs: m.valeurs,
        }
    }
}

impl From<std::io::Error> for Erreur {
    fn from(e: std::io::Error) -> Self {
        Erreur::Io(e)
    }
}

pub type Resultat<T> = Result<T, Erreur>;

/// Issue du retrait d'une fiche de référentiel (section 5.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Retrait {
    /// Aucun document ne la citait : elle a été supprimée.
    Supprimee,
    /// Un document la cite : elle a seulement été désactivée.
    Desactivee,
}

/// Bilan d'un import de fiches.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BilanImport {
    pub ajoutees: usize,
    /// Rang (à partir de 0) de chaque fiche ignorée, avec la raison.
    pub ignorees: Vec<(usize, String)>,
}

/// Ouvre (ou crée) la base chiffrée au chemin donné et applique les migrations
/// en attente. Une base encore en clair est d'abord chiffrée (voir
/// [`chiffrement::chiffrer_base_en_clair`]) ; la clé est créée avec une base
/// neuve, dans `kadenz.cle` à côté d'elle.
pub fn ouvrir(chemin: &Path) -> Resultat<Connection> {
    if let Some(dossier) = chemin.parent() {
        std::fs::create_dir_all(dossier)?;
    }
    let fichier_cle = chiffrement::chemin_cle(chemin);
    let neuve = std::fs::metadata(chemin)
        .map(|m| m.len() == 0)
        .unwrap_or(true);
    let a_chiffrer = chiffrement::en_clair(chemin)?;
    let cle = chiffrement::cle(&fichier_cle, neuve || a_chiffrer)?;
    if a_chiffrer {
        chiffrement::chiffrer_base_en_clair(chemin, &cle)?;
    }
    let mut conn = Connection::open(chemin)?;
    chiffrement::appliquer_cle(&conn, &cle)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    preparer(&mut conn)?;
    // Socle seul tant que la licence n'est pas appliquée (licence::appliquer).
    licence::preparer(&conn, false)?;
    Ok(conn)
}

/// Ouvre (ou crée) la base du serveur de l'entreprise (phase 2, F2) : chiffrée
/// comme celle d'un poste, mais avec une clé protégée par Windows au niveau de
/// la machine, pour que le service la lise sans session ouverte. Les droits du
/// dossier (SYSTEM et Administrateurs) sont posés par le serveur. Tous les
/// modules y sont actifs en attendant la licence d'entreprise (F3).
pub fn ouvrir_serveur(chemin: &Path) -> Resultat<Connection> {
    if let Some(dossier) = chemin.parent() {
        std::fs::create_dir_all(dossier)?;
    }
    let neuve = std::fs::metadata(chemin)
        .map(|m| m.len() == 0)
        .unwrap_or(true);
    let cle = chiffrement::cle_avec(
        &chiffrement::chemin_cle(chemin),
        neuve,
        chiffrement::Portee::Machine,
    )?;
    let mut conn = Connection::open(chemin)?;
    chiffrement::appliquer_cle(&conn, &cle)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    // La fenêtre d'administration ouvre la même base que le service (F4).
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    preparer(&mut conn)?;
    licence::preparer(&conn, true)?;
    Ok(conn)
}

/// Base en mémoire, pour les tests : tous les modules y sont actifs.
pub fn ouvrir_en_memoire() -> Resultat<Connection> {
    let mut conn = Connection::open_in_memory()?;
    preparer(&mut conn)?;
    licence::preparer(&conn, true)?;
    Ok(conn)
}

fn preparer(conn: &mut Connection) -> Resultat<()> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrer(conn)?;
    completer_schema(conn)?;
    synchro::preparer(conn)
}

/// Complète une base créée pendant le développement, avant que les premières
/// migrations ne soient figées (0001_initial.sql a changé le 9 novembre 2026
/// sans nouvelle migration) : une telle base porte le bon `user_version` mais
/// il lui manque des colonnes, d'où « no such column » à l'usage.
///
/// Le schéma de référence est celui que produisent les migrations sur une base
/// vide ; ce qui y manque est ajouté (tables, colonnes, index, déclencheurs),
/// rien n'est retiré ni modifié. Sur une base à jour, rien n'est écrit.
fn completer_schema(conn: &mut Connection) -> Resultat<()> {
    let reference = Connection::open_in_memory()?;
    for sql in MIGRATIONS {
        reference.execute_batch(sql)?;
    }
    let objets = |c: &Connection, types: &str| -> Resultat<Vec<(String, String)>> {
        let mut req = c.prepare(&format!(
            "SELECT name, sql FROM sqlite_master
             WHERE type IN ({types}) AND sql IS NOT NULL AND name NOT LIKE 'sqlite_%'
             ORDER BY rowid"
        ))?;
        let lignes = req
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        Ok(lignes)
    };
    let tables_base: Vec<String> = objets(conn, "'table'")?
        .into_iter()
        .map(|(nom, _)| nom)
        .collect();

    let tx = conn.transaction()?;
    for (table, creation) in objets(&reference, "'table'")? {
        if !tables_base.contains(&table) {
            tx.execute_batch(&creation)?;
            continue;
        }
        let presentes = colonnes(&tx, &table)?;
        for colonne in colonnes(&reference, &table)? {
            if presentes.iter().any(|p| p.nom == colonne.nom) {
                continue;
            }
            // ALTER TABLE n'accepte NOT NULL qu'avec une valeur par défaut.
            let mut declaration = format!(
                "ALTER TABLE \"{table}\" ADD COLUMN \"{}\" {}",
                colonne.nom, colonne.type_sql
            );
            if let Some(defaut) = &colonne.defaut {
                if colonne.non_nulle {
                    declaration.push_str(" NOT NULL");
                }
                declaration.push_str(&format!(" DEFAULT {defaut}"));
            }
            tx.execute_batch(&declaration)?;
        }
    }
    let noms_base: Vec<String> = objets(&tx, "'index', 'trigger', 'view'")?
        .into_iter()
        .map(|(nom, _)| nom)
        .collect();
    for (nom, creation) in objets(&reference, "'index', 'trigger', 'view'")? {
        if !noms_base.contains(&nom) {
            tx.execute_batch(&creation)?;
        }
    }
    tx.commit()?;
    Ok(())
}

struct Colonne {
    nom: String,
    type_sql: String,
    non_nulle: bool,
    defaut: Option<String>,
}

fn colonnes(conn: &Connection, table: &str) -> Resultat<Vec<Colonne>> {
    let mut req =
        conn.prepare("SELECT name, type, \"notnull\", dflt_value FROM pragma_table_info(?1)")?;
    let lignes = req
        .query_map([table], |r| {
            Ok(Colonne {
                nom: r.get(0)?,
                type_sql: r.get(1)?,
                non_nulle: r.get(2)?,
                defaut: r.get(3)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(lignes)
}

/// Version de schéma connue de cette version de l'application.
pub fn version_schema_application() -> u32 {
    MIGRATIONS.len() as u32
}

fn migrer(conn: &mut Connection) -> Resultat<()> {
    let base: u32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    let application = version_schema_application();
    if base > application {
        return Err(Erreur::VersionSchemaInconnue { base, application });
    }
    for (rang, sql) in MIGRATIONS.iter().enumerate().skip(base as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", rang as u32 + 1)?;
        tx.commit()?;
    }
    Ok(())
}

/// Réserve le prochain rang de numérotation pour (type, année, mois).
///
/// À appeler dans la transaction qui valide le document, pour qu'un échec de
/// validation n'entame pas la séquence.
pub fn prochain_rang(
    conn: &Connection,
    type_document: TypeDocument,
    annee: u16,
    mois: u8,
) -> Resultat<u32> {
    let rang = conn.query_row(
        "INSERT INTO compteur_numerotation (type_document, annee, mois, dernier_numero)
         VALUES (?1, ?2, ?3, 1)
         ON CONFLICT (type_document, annee, mois)
         DO UPDATE SET dernier_numero = dernier_numero + 1
         RETURNING dernier_numero",
        params![type_document.code(), annee, mois],
        |r| r.get(0),
    )?;
    Ok(rang)
}

/// Sauvegarde cohérente de la base ouverte vers `destination`, chiffrée par
/// `mot_de_passe` (et non par la clé du poste) pour pouvoir être restaurée sur
/// un autre poste ou un autre compte. Par l'API de sauvegarde en ligne de
/// SQLite, jamais une copie brute du fichier ; un fichier existant est remplacé.
pub fn sauvegarder(conn: &Connection, destination: &Path, mot_de_passe: &str) -> Resultat<()> {
    if mot_de_passe.chars().count() < comptes::LONGUEUR_MIN_MOT_DE_PASSE {
        return Err(refus(
            "le mot de passe de la sauvegarde doit compter au moins {} caractères",
            &[&comptes::LONGUEUR_MIN_MOT_DE_PASSE],
        ));
    }
    if let Some(dossier) = destination.parent() {
        std::fs::create_dir_all(dossier)?;
    }
    match std::fs::remove_file(destination) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.into()),
        _ => {}
    }
    let mut cible = Connection::open(destination)?;
    chiffrement::appliquer_cle(&cible, mot_de_passe)?;
    rusqlite::backup::Backup::new(conn, &mut cible)?.run_to_completion(
        256,
        std::time::Duration::ZERO,
        None,
    )?;
    // La copie reprend l'en-tête de la base, en mode WAL : une sauvegarde
    // ouverte plus tard en lecture seule laisserait des fichiers -wal et -shm
    // à côté d'elle. Journal classique pour le fichier de sauvegarde.
    cible.pragma_update(None, "journal_mode", "DELETE")?;
    Ok(())
}

/// Ouvre une sauvegarde faite par [`sauvegarder`], avec son mot de passe, en
/// lecture seule et immuable : rien n'est écrit, ni dans le fichier ni à côté
/// (pas de -wal ni de -shm, même pour une sauvegarde restée en mode WAL).
pub fn ouvrir_sauvegarde(chemin: &Path, mot_de_passe: &str) -> Resultat<Connection> {
    if !chemin.is_file() {
        return Err(refus("sauvegarde introuvable : {}", &[&chemin.display()]));
    }
    let conn = match uri_immuable(chemin) {
        Some(uri) => Connection::open_with_flags(
            uri,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
        )?,
        None => Connection::open_with_flags(chemin, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?,
    };
    chiffrement::appliquer_cle(&conn, mot_de_passe)
        .map_err(|_| Erreur::Validation("mot de passe de la sauvegarde incorrect".into()))?;
    Ok(conn)
}

/// URI SQLite `file:` du chemin avec `immutable=1` ; `None` si le chemin n'est
/// pas de l'UTF-8 (ouverture ordinaire alors).
fn uri_immuable(chemin: &Path) -> Option<String> {
    let texte = chemin.to_str()?;
    let mut uri = String::from("file:");
    if !texte.starts_with(['/', '\\']) {
        uri.push_str("///");
    }
    for c in texte.chars() {
        match c {
            '\\' => uri.push('/'),
            '%' | '?' | '#' | ' ' => uri.push_str(&format!("%{:02X}", c as u32)),
            autre => uri.push(autre),
        }
    }
    uri.push_str("?immutable=1");
    Some(uri)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uri_des_sauvegardes() {
        assert_eq!(
            uri_immuable(Path::new(r"C:\Mes documents\100% #1?.kzs")).unwrap(),
            "file:///C:/Mes%20documents/100%25%20%231%3F.kzs?immutable=1"
        );
    }

    #[test]
    fn refus_compose() {
        let e = refus("ligne {} : la désignation est obligatoire", &[&3]);
        assert_eq!(e.to_string(), "ligne 3 : la désignation est obligatoire");
        let Erreur::ValidationComposee { modele, valeurs } = e else {
            panic!("refus composé attendu");
        };
        assert_eq!(modele, "ligne {} : la désignation est obligatoire");
        assert_eq!(valeurs, ["3"]);
        // Un {} sans valeur reste visible plutôt que de disparaître.
        assert_eq!(composer("{} vers {}", &["Validé".into()]), "Validé vers {}");
    }

    fn base() -> Connection {
        ouvrir_en_memoire().expect("ouverture de la base en mémoire")
    }

    #[test]
    fn migrations_appliquees() {
        let conn = base();
        let v: u32 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(v, version_schema_application());
    }

    #[test]
    fn schema_de_developpement_complete() {
        // Base au bon user_version mais créée avant des modifications de
        // 0001_initial.sql : colonnes et index manquants.
        let mut conn = base();
        conn.execute_batch(
            "DROP INDEX document_par_source;
             ALTER TABLE document DROP COLUMN date_echeance;
             ALTER TABLE entreprise DROP COLUMN logo2;",
        )
        .unwrap();
        completer_schema(&mut conn).unwrap();
        let noms = |t: &str| -> Vec<String> {
            colonnes(&conn, t)
                .unwrap()
                .into_iter()
                .map(|c| c.nom)
                .collect()
        };
        assert!(noms("document").contains(&"date_echeance".to_string()));
        assert!(noms("entreprise").contains(&"logo2".to_string()));
        let index: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'document_par_source'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(index, 1);
        // Une base à jour reste inchangée.
        completer_schema(&mut conn).unwrap();
    }

    #[test]
    fn compteur_par_type_et_mois() {
        let conn = base();
        let f = TypeDocument::Facture;
        assert_eq!(prochain_rang(&conn, f, 2026, 5).unwrap(), 1);
        assert_eq!(prochain_rang(&conn, f, 2026, 5).unwrap(), 2);
        assert_eq!(prochain_rang(&conn, f, 2026, 6).unwrap(), 1);
        assert_eq!(
            prochain_rang(&conn, TypeDocument::Devis, 2026, 5).unwrap(),
            1
        );
    }

    #[test]
    fn compteur_annule_avec_la_transaction() {
        let mut conn = base();
        {
            let tx = conn.transaction().unwrap();
            assert_eq!(
                prochain_rang(&tx, TypeDocument::Facture, 2026, 5).unwrap(),
                1
            );
            // Pas de commit : la validation a échoué.
        }
        assert_eq!(
            prochain_rang(&conn, TypeDocument::Facture, 2026, 5).unwrap(),
            1
        );
    }

    #[test]
    fn audit_en_ajout_seul() {
        let conn = base();
        conn.execute(
            "INSERT INTO audit_log (entite, entite_id, action, date_action)
             VALUES ('document', 1, 'validation', '2026-11-09T10:00:00')",
            [],
        )
        .unwrap();
        assert!(
            conn.execute("UPDATE audit_log SET action = 'x'", [])
                .is_err()
        );
        assert!(conn.execute("DELETE FROM audit_log", []).is_err());
    }

    fn inserer_document(conn: &Connection, type_document: &str) -> i64 {
        conn.execute(
            "INSERT INTO document (type, date, date_creation) VALUES (?1, '2026-11-09', '2026-11-09T10:00:00')",
            [type_document],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn reglement_limite_a_facture_et_bon_commande() {
        let conn = base();
        let insertion =
            "INSERT INTO reglement (document_id, date, montant) VALUES (?1, '2026-11-09', 1000)";
        let devis = inserer_document(&conn, "devis");
        let facture = inserer_document(&conn, "facture");
        let bc = inserer_document(&conn, "bon_commande");
        assert!(conn.execute(insertion, [devis]).is_err());
        assert!(conn.execute(insertion, [facture]).is_ok());
        assert!(conn.execute(insertion, [bc]).is_ok());
    }

    #[test]
    fn brouillon_sans_numero() {
        let conn = base();
        let r = conn.execute(
            "INSERT INTO document (type, numero, date, date_creation)
             VALUES ('facture', 'F2026-05-0001', '2026-11-09', '2026-11-09T10:00:00')",
            [],
        );
        assert!(r.is_err(), "un brouillon ne doit pas porter de numéro");
    }

    fn dossier_essai(nom: &str) -> std::path::PathBuf {
        let dossier = std::env::temp_dir().join(format!("kadenz-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(&dossier).unwrap();
        dossier
    }

    fn dernier_numero(conn: &Connection) -> u32 {
        conn.query_row(
            "SELECT dernier_numero FROM compteur_numerotation",
            [],
            |r| r.get(0),
        )
        .unwrap()
    }

    #[test]
    fn base_neuve_chiffree() {
        let dossier = dossier_essai("neuve");
        let chemin = dossier.join("kadenz.sqlite");
        let conn = ouvrir(&chemin).unwrap();
        prochain_rang(&conn, TypeDocument::Facture, 2026, 5).unwrap();
        drop(conn);
        assert!(
            chemin.with_extension("cle").is_file(),
            "clé créée à côté de la base"
        );
        assert!(!chiffrement::en_clair(&chemin).unwrap(), "fichier chiffré");
        // Sans la clé, la base ne se lit pas.
        let brute = Connection::open(&chemin).unwrap();
        assert!(
            brute
                .query_row("SELECT count(*) FROM sqlite_master", [], |r| r
                    .get::<_, i64>(0))
                .is_err()
        );
        drop(brute);
        // Avec elle, si : réouverture normale.
        let conn = ouvrir(&chemin).unwrap();
        assert_eq!(dernier_numero(&conn), 1);
        drop(conn);
        // Clé disparue : erreur claire, la base n'est pas recréée.
        std::fs::remove_file(chemin.with_extension("cle")).unwrap();
        assert!(ouvrir(&chemin).is_err());
        let _ = std::fs::remove_dir_all(dossier);
    }

    #[test]
    fn base_en_clair_convertie() {
        let dossier = dossier_essai("conversion");
        let chemin = dossier.join("kadenz.sqlite");
        {
            // Base de l'ancienne version : en clair, en mode WAL.
            let mut conn = Connection::open(&chemin).unwrap();
            conn.pragma_update(None, "journal_mode", "WAL").unwrap();
            preparer(&mut conn).unwrap();
            prochain_rang(&conn, TypeDocument::Facture, 2026, 5).unwrap();
            prochain_rang(&conn, TypeDocument::Facture, 2026, 5).unwrap();
        }
        assert!(chiffrement::en_clair(&chemin).unwrap());
        let conn = ouvrir(&chemin).unwrap();
        assert_eq!(dernier_numero(&conn), 2, "données gardées");
        drop(conn);
        assert!(!chiffrement::en_clair(&chemin).unwrap(), "fichier chiffré");
        assert!(!dossier.join("kadenz.sqlite.en-clair").exists());
        assert!(!dossier.join("kadenz.sqlite.chiffrement").exists());
        let _ = std::fs::remove_dir_all(dossier);
    }

    #[test]
    fn sauvegarde_par_mot_de_passe() {
        let dossier = dossier_essai("sauvegarde");
        let conn = ouvrir(&dossier.join("kadenz.sqlite")).unwrap();
        prochain_rang(&conn, TypeDocument::Facture, 2026, 5).unwrap();
        let cible = dossier.join("copie").join("sauvegarde.kadenz");
        assert!(
            sauvegarder(&conn, &cible, "court").is_err(),
            "mot de passe trop court"
        );
        sauvegarder(&conn, &cible, "motdepasse-sauvegarde").unwrap();
        assert!(
            !chiffrement::en_clair(&cible).unwrap(),
            "sauvegarde chiffrée"
        );
        assert!(ouvrir_sauvegarde(&cible, "mauvais-mot-de-passe").is_err());
        let copie = ouvrir_sauvegarde(&cible, "motdepasse-sauvegarde").unwrap();
        assert_eq!(dernier_numero(&copie), 1);
        drop(copie);
        drop(conn);
        let _ = std::fs::remove_dir_all(dossier);
    }
}
