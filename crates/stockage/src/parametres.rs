//! Paramètres système modifiables par l'Admin (cahier des charges, section 5.4).
//!
//! Chaque paramètre est une ligne clé/valeur, la valeur étant du JSON. Une clé
//! absente retombe sur la valeur par défaut du cahier des charges.

use kadenz_domaine::calcul::Pourcentage;
use kadenz_domaine::document::TypeDocument;
use kadenz_domaine::lettres::Orthographe;
use kadenz_domaine::numerotation::{Numerotation, RemiseAZero};
use kadenz_domaine::role::Droit;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use crate::audit::{self, Entree};
use crate::comptes::Utilisateur;
use crate::droits::exiger;
use crate::{Erreur, Resultat, refus};

const CLE_TAUX_TVA: &str = "taux.tva";
const CLE_TAUX_REMISE: &str = "taux.remise";
const CLE_TAUX_RETENUE: &str = "taux.retenue";
const CLE_MODES_PAIEMENT: &str = "paiement.modes";
const CLE_ORTHOGRAPHE: &str = "lettres.orthographe";
const CLE_VALIDITE_DEVIS: &str = "delais.validite_devis_jours";
const CLE_DELAI_PAIEMENT: &str = "delais.paiement_jours";
const CLE_NUMEROTATION: &str = "numerotation";
const CLE_TEXTES_DOCUMENTS: &str = "documents.textes";
const CLE_COULEUR_DOCUMENTS: &str = "documents.couleur";
const CLE_DOSSIERS_EXPORT: &str = "documents.dossiers";

/// Couleur principale des documents par défaut : l'Indigo de la marque.
pub const COULEUR_PAR_DEFAUT: [u8; 3] = [0x2d, 0x3a, 0x8c];

/// Textes imprimés d'un type de document (section 5.4, « Textes des documents »).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextesDocument {
    /// Titre imprimé, ex. « FACTURE ».
    pub titre: String,
    /// Début de la phrase du montant en lettres ; vide pour un document sans montants.
    pub phrase_somme: String,
    /// Libellé du signataire, ex. « LA COMPTABILITE » ; vide pour aucun.
    pub signataire: String,
}

impl TextesDocument {
    /// Valeurs par défaut reprises de l'outil Excel d'origine : titre égal au nom
    /// du type en majuscules, phrase « Arrêté le présent ... à la somme de »,
    /// signataire « LA COMPTABILITE ».
    pub fn par_defaut(t: TypeDocument) -> Self {
        let phrase = match t {
            TypeDocument::Devis => "Arrêté le présent devis à la somme de",
            TypeDocument::Proforma => "Arrêté la présente proforma à la somme de",
            TypeDocument::BonCommande => "Arrêté le présent bon de commande à la somme de",
            TypeDocument::Livraison => "",
            TypeDocument::Facture => "Arrêté la présente facture à la somme de",
            TypeDocument::Avoir => "Arrêté le présent avoir à la somme de",
        };
        TextesDocument {
            titre: t.to_string().to_uppercase(),
            phrase_somme: phrase.into(),
            signataire: "LA COMPTABILITE".into(),
        }
    }
}

/// Couleur en texte « #rrggbb ».
pub fn couleur_en_texte(c: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

/// Lit une couleur « #rrggbb » (le « # » est facultatif).
pub fn couleur_depuis_texte(texte: &str) -> Option<[u8; 3]> {
    let h = texte.trim().trim_start_matches('#');
    if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let octet = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
    Some([octet(0)?, octet(2)?, octet(4)?])
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parametres {
    pub taux_tva: Vec<Pourcentage>,
    pub taux_remise: Vec<Pourcentage>,
    pub taux_retenue: Vec<Pourcentage>,
    pub modes_paiement: Vec<String>,
    pub orthographe: Orthographe,
    /// Durée de validité des devis, en jours. Pas de valeur par défaut dans le cahier.
    pub validite_devis_jours: Option<u32>,
    /// Délai de paiement des factures, en jours (date d'échéance). Pas de valeur par défaut.
    pub delai_paiement_jours: Option<u32>,
    /// Préfixes, remise à zéro et nombre de chiffres des numéros (section 5.4).
    pub numerotation: Numerotation,
    /// Textes imprimés, un jeu par type dans l'ordre de `TypeDocument::TOUS`.
    pub textes_documents: [TextesDocument; 6],
    /// Couleur principale des documents (bande, en-tête du tableau, titre).
    pub couleur_documents: [u8; 3],
    /// Dossier proposé à l'enregistrement d'un PDF, un par type dans l'ordre de
    /// `TypeDocument::TOUS` ; vide pour laisser le choix au système.
    pub dossiers_export: [String; 6],
}

impl Default for Parametres {
    /// Valeurs par défaut du cahier des charges et de l'outil Excel d'origine.
    fn default() -> Self {
        Parametres {
            taux_tva: vec![0, 1800],
            taux_remise: vec![0, 500, 1000, 1500],
            taux_retenue: vec![0, 500, 1000, 1500],
            modes_paiement: [
                "Espèce",
                "Chèque",
                "Virement bancaire",
                "Carte bancaire",
                "Moov money",
                "Orange money",
            ]
            .map(String::from)
            .to_vec(),
            orthographe: Orthographe::default(),
            validite_devis_jours: None,
            delai_paiement_jours: None,
            numerotation: Numerotation::default(),
            textes_documents: TypeDocument::TOUS.map(TextesDocument::par_defaut),
            couleur_documents: COULEUR_PAR_DEFAUT,
            dossiers_export: Default::default(),
        }
    }
}

impl Parametres {
    fn valider(&self) -> Resultat<()> {
        let verifier_taux = |message: &str, liste: &[Pourcentage]| {
            if liste.is_empty() || liste.iter().any(|p| !(0..=10_000).contains(p)) {
                Err(Erreur::Validation(message.into()))
            } else {
                Ok(())
            }
        };
        verifier_taux(
            "la liste des taux de TVA doit contenir des valeurs entre 0 et 100 %",
            &self.taux_tva,
        )?;
        verifier_taux(
            "la liste des taux de remise doit contenir des valeurs entre 0 et 100 %",
            &self.taux_remise,
        )?;
        verifier_taux(
            "la liste des taux de retenue doit contenir des valeurs entre 0 et 100 %",
            &self.taux_retenue,
        )?;
        if self.modes_paiement.iter().all(|m| m.trim().is_empty()) {
            return Err(Erreur::Validation(
                "au moins un mode de paiement est nécessaire".into(),
            ));
        }
        if let Some(t) = self
            .textes_documents
            .iter()
            .position(|t| t.titre.trim().is_empty())
        {
            return Err(refus(
                "le titre imprimé du type « {} » est obligatoire",
                &[&TypeDocument::TOUS[t]],
            ));
        }
        self.numerotation.nettoyee()?;
        Ok(())
    }
}

fn numerotation_en_json(n: &Numerotation) -> Value {
    json!({
        "prefixes": n.prefixes,
        "remise": n.remise.code(),
        "chiffres": n.chiffres,
    })
}

/// Valeur illisible ou incomplète : retour au réglage par défaut, comme pour une clé absente.
fn numerotation_depuis_json(valeur: Option<Value>) -> Option<Numerotation> {
    let v = valeur?;
    let prefixes: Vec<String> = liste_textes(v.get("prefixes").cloned())?;
    let prefixes: [String; 6] = prefixes.try_into().ok()?;
    let remise = RemiseAZero::depuis_code(v.get("remise")?.as_str()?)?;
    let chiffres = u8::try_from(v.get("chiffres")?.as_u64()?).ok()?;
    Numerotation {
        prefixes,
        remise,
        chiffres,
    }
    .nettoyee()
    .ok()
}

impl Parametres {
    /// Textes imprimés d'un type de document.
    pub fn textes(&self, t: TypeDocument) -> &TextesDocument {
        let rang = TypeDocument::TOUS
            .iter()
            .position(|x| *x == t)
            .expect("type présent dans TOUS");
        &self.textes_documents[rang]
    }

    /// Dossier d'export d'un type de document ; vide s'il n'est pas réglé.
    pub fn dossier_export(&self, t: TypeDocument) -> &str {
        let rang = TypeDocument::TOUS
            .iter()
            .position(|x| *x == t)
            .expect("type présent dans TOUS");
        self.dossiers_export[rang].trim()
    }
}

fn dossiers_en_json(dossiers: &[String; 6]) -> Value {
    Value::Object(
        TypeDocument::TOUS
            .iter()
            .zip(dossiers)
            .filter(|(_, d)| !d.trim().is_empty())
            .map(|(t, d)| (t.code().to_string(), json!(d.trim())))
            .collect(),
    )
}

/// Dossiers enregistrés, par code de type ; un type absent reste sans dossier.
fn dossiers_depuis_json(valeur: Option<Value>) -> [String; 6] {
    let mut dossiers: [String; 6] = Default::default();
    if let Some(Value::Object(m)) = valeur {
        for (rang, t) in TypeDocument::TOUS.iter().enumerate() {
            if let Some(d) = m.get(t.code()).and_then(Value::as_str) {
                dossiers[rang] = d.to_string();
            }
        }
    }
    dossiers
}

fn textes_en_json(textes: &[TextesDocument; 6]) -> Value {
    Value::Array(
        TypeDocument::TOUS
            .iter()
            .zip(textes)
            .map(|(t, x)| {
                json!({
                    "type": t.code(),
                    "titre": x.titre.trim(),
                    "phrase_somme": x.phrase_somme.trim(),
                    "signataire": x.signataire.trim(),
                })
            })
            .collect(),
    )
}

/// Textes enregistrés ; un type absent ou illisible garde ses valeurs par défaut.
fn textes_depuis_json(valeur: Option<Value>) -> [TextesDocument; 6] {
    let mut textes = TypeDocument::TOUS.map(TextesDocument::par_defaut);
    let Some(Value::Array(elements)) = valeur else {
        return textes;
    };
    for e in elements {
        let Some(rang) = e
            .get("type")
            .and_then(Value::as_str)
            .and_then(|code| TypeDocument::TOUS.iter().position(|t| t.code() == code))
        else {
            continue;
        };
        let champ = |nom: &str| e.get(nom).and_then(Value::as_str).map(String::from);
        if let (Some(titre), Some(phrase_somme), Some(signataire)) =
            (champ("titre"), champ("phrase_somme"), champ("signataire"))
        {
            textes[rang] = TextesDocument {
                titre,
                phrase_somme,
                signataire,
            };
        }
    }
    textes
}

fn lire_valeur(conn: &Connection, cle: &str) -> Resultat<Option<Value>> {
    let texte: Option<String> = conn
        .query_row(
            "SELECT valeur FROM parametre_systeme WHERE cle = ?1",
            [cle],
            |r| r.get(0),
        )
        .optional()?;
    Ok(texte.and_then(|t| serde_json::from_str(&t).ok()))
}

fn ecrire_valeur(conn: &Connection, cle: &str, valeur: &Value) -> Resultat<()> {
    conn.execute(
        "INSERT INTO parametre_systeme (cle, valeur) VALUES (?1, ?2)
         ON CONFLICT (cle) DO UPDATE SET valeur = excluded.valeur",
        params![cle, valeur.to_string()],
    )?;
    Ok(())
}

fn liste_entiers(valeur: Option<Value>) -> Option<Vec<i64>> {
    valeur?.as_array()?.iter().map(Value::as_i64).collect()
}

fn jours(valeur: Option<Value>) -> Option<u32> {
    valeur?.as_u64().and_then(|j| u32::try_from(j).ok())
}

fn liste_textes(valeur: Option<Value>) -> Option<Vec<String>> {
    valeur?
        .as_array()?
        .iter()
        .map(|v| v.as_str().map(String::from))
        .collect()
}

pub fn lire(conn: &Connection) -> Resultat<Parametres> {
    lire_avec(|cle| lire_valeur(conn, cle))
}

/// Paramètres décrits par la valeur de chaque clé : `lire` dans la base, ou
/// [`depuis_json`] pour ceux reçus d'un poste.
fn lire_avec(valeur: impl Fn(&str) -> Resultat<Option<Value>>) -> Resultat<Parametres> {
    let defaut = Parametres::default();
    Ok(Parametres {
        taux_tva: liste_entiers(valeur(CLE_TAUX_TVA)?).unwrap_or(defaut.taux_tva),
        taux_remise: liste_entiers(valeur(CLE_TAUX_REMISE)?).unwrap_or(defaut.taux_remise),
        taux_retenue: liste_entiers(valeur(CLE_TAUX_RETENUE)?).unwrap_or(defaut.taux_retenue),
        modes_paiement: liste_textes(valeur(CLE_MODES_PAIEMENT)?).unwrap_or(defaut.modes_paiement),
        orthographe: valeur(CLE_ORTHOGRAPHE)?
            .and_then(|v| v.as_str().and_then(Orthographe::depuis_code))
            .unwrap_or(defaut.orthographe),
        validite_devis_jours: jours(valeur(CLE_VALIDITE_DEVIS)?),
        delai_paiement_jours: jours(valeur(CLE_DELAI_PAIEMENT)?),
        numerotation: numerotation_depuis_json(valeur(CLE_NUMEROTATION)?)
            .unwrap_or(defaut.numerotation),
        textes_documents: textes_depuis_json(valeur(CLE_TEXTES_DOCUMENTS)?),
        couleur_documents: valeur(CLE_COULEUR_DOCUMENTS)?
            .and_then(|v| v.as_str().and_then(couleur_depuis_texte))
            .unwrap_or(defaut.couleur_documents),
        dossiers_export: dossiers_depuis_json(valeur(CLE_DOSSIERS_EXPORT)?),
    })
}

/// Paramètres partagés en JSON, par clé, pour l'action d'un poste relié : les
/// dossiers d'export, propres à chaque machine, n'y sont pas.
pub fn en_json(p: &Parametres) -> Value {
    json!({
        CLE_TAUX_TVA: p.taux_tva,
        CLE_TAUX_REMISE: p.taux_remise,
        CLE_TAUX_RETENUE: p.taux_retenue,
        CLE_MODES_PAIEMENT: p.modes_paiement,
        CLE_ORTHOGRAPHE: p.orthographe.code(),
        CLE_VALIDITE_DEVIS: p.validite_devis_jours,
        CLE_DELAI_PAIEMENT: p.delai_paiement_jours,
        CLE_NUMEROTATION: numerotation_en_json(&p.numerotation),
        CLE_TEXTES_DOCUMENTS: textes_en_json(&p.textes_documents),
        CLE_COULEUR_DOCUMENTS: couleur_en_texte(p.couleur_documents),
    })
}

/// Paramètres reçus d'un poste ([`en_json`]) ; dossiers d'export vides.
pub fn depuis_json(v: &Value) -> Parametres {
    lire_avec(|cle| Ok(v.get(cle).cloned())).unwrap_or_default()
}

/// Dossiers d'export de cette machine seulement (poste relié : le reste des
/// paramètres part au serveur).
pub fn enregistrer_dossiers(
    conn: &Connection,
    acteur: &Utilisateur,
    dossiers: &[String; 6],
) -> Resultat<()> {
    exiger(conn, acteur, Droit::GererParametres)?;
    ecrire_valeur(conn, CLE_DOSSIERS_EXPORT, &dossiers_en_json(dossiers))
}

/// Réservé au droit « Gérer la fiche entreprise et les paramètres ».
pub fn enregistrer(conn: &Connection, acteur: &Utilisateur, p: &Parametres) -> Resultat<()> {
    exiger(conn, acteur, Droit::GererParametres)?;
    enregistrer_sans_controle(conn, p, Some(acteur.id))
}

/// Écriture sans contrôle de droit : réservée à l'installation initiale.
pub(crate) fn enregistrer_sans_controle(
    conn: &Connection,
    p: &Parametres,
    auteur: Option<i64>,
) -> Resultat<()> {
    p.valider()?;
    let numerotation = numerotation_en_json(
        &p.numerotation
            .nettoyee()
            .map_err(|e| Erreur::Validation(e.to_string()))?,
    );
    let modes: Vec<&str> = p
        .modes_paiement
        .iter()
        .map(|m| m.trim())
        .filter(|m| !m.is_empty())
        .collect();
    ecrire_valeur(conn, CLE_TAUX_TVA, &json!(p.taux_tva))?;
    ecrire_valeur(conn, CLE_TAUX_REMISE, &json!(p.taux_remise))?;
    ecrire_valeur(conn, CLE_TAUX_RETENUE, &json!(p.taux_retenue))?;
    ecrire_valeur(conn, CLE_MODES_PAIEMENT, &json!(modes))?;
    ecrire_valeur(conn, CLE_ORTHOGRAPHE, &json!(p.orthographe.code()))?;
    ecrire_valeur(conn, CLE_VALIDITE_DEVIS, &json!(p.validite_devis_jours))?;
    ecrire_valeur(conn, CLE_DELAI_PAIEMENT, &json!(p.delai_paiement_jours))?;
    ecrire_valeur(conn, CLE_NUMEROTATION, &numerotation)?;
    let textes = textes_en_json(&p.textes_documents);
    let couleur = couleur_en_texte(p.couleur_documents);
    ecrire_valeur(conn, CLE_TEXTES_DOCUMENTS, &textes)?;
    ecrire_valeur(conn, CLE_COULEUR_DOCUMENTS, &json!(couleur))?;
    let dossiers = dossiers_en_json(&p.dossiers_export);
    ecrire_valeur(conn, CLE_DOSSIERS_EXPORT, &dossiers)?;
    audit::journaliser(
        conn,
        Entree {
            entite: "parametres",
            entite_id: 0,
            action: "modification",
            ancienne_valeur: None,
            nouvelle_valeur: Some(json!({
                "taux_tva": p.taux_tva,
                "taux_remise": p.taux_remise,
                "taux_retenue": p.taux_retenue,
                "modes_paiement": modes,
                "orthographe": p.orthographe.code(),
                "validite_devis_jours": p.validite_devis_jours,
                "delai_paiement_jours": p.delai_paiement_jours,
                "numerotation": numerotation,
                "textes_documents": textes,
                "couleur_documents": couleur,
                "dossiers_export": dossiers,
            })),
            utilisateur_id: auteur,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ouvrir_en_memoire;

    #[test]
    fn valeurs_par_defaut_sans_enregistrement() {
        let conn = ouvrir_en_memoire().unwrap();
        assert_eq!(lire(&conn).unwrap(), Parametres::default());
    }

    #[test]
    fn aller_retour() {
        let conn = ouvrir_en_memoire().unwrap();
        let p = Parametres {
            taux_tva: vec![0, 250, 1800],
            modes_paiement: vec!["Espèce".into(), "  ".into(), "Chèque".into()],
            orthographe: Orthographe::Traditionnelle,
            ..Default::default()
        };
        enregistrer_sans_controle(&conn, &p, None).unwrap();
        let lu = lire(&conn).unwrap();
        assert_eq!(lu.taux_tva, vec![0, 250, 1800]);
        assert_eq!(lu.modes_paiement, vec!["Espèce", "Chèque"]);
        assert_eq!(lu.orthographe, Orthographe::Traditionnelle);
    }

    #[test]
    fn numerotation_aller_retour() {
        let conn = ouvrir_en_memoire().unwrap();
        let mut p = Parametres::default();
        p.numerotation.remise = RemiseAZero::Annuelle;
        p.numerotation.chiffres = 5;
        p.numerotation.prefixes[4] = " fa ".into();
        enregistrer_sans_controle(&conn, &p, None).unwrap();
        let n = lire(&conn).unwrap().numerotation;
        assert_eq!(n.remise, RemiseAZero::Annuelle);
        assert_eq!(n.chiffres, 5);
        assert_eq!(n.prefixes[4], "FA");
        p.numerotation.prefixes[4] = "D".into();
        assert!(enregistrer_sans_controle(&conn, &p, None).is_err());
    }

    #[test]
    fn textes_et_couleur() {
        let conn = ouvrir_en_memoire().unwrap();
        let defaut = lire(&conn).unwrap();
        assert_eq!(defaut.textes(TypeDocument::Facture).titre, "FACTURE");
        assert_eq!(
            defaut.textes(TypeDocument::BonCommande).titre,
            "BON DE COMMANDE"
        );
        assert_eq!(
            defaut.textes(TypeDocument::BonCommande).phrase_somme,
            "Arrêté le présent bon de commande à la somme de"
        );
        assert_eq!(defaut.couleur_documents, COULEUR_PAR_DEFAUT);

        let mut p = defaut.clone();
        p.textes_documents[4].titre = "FACTURE DEFINITIVE".into();
        p.couleur_documents = [0x10, 0x80, 0xff];
        enregistrer_sans_controle(&conn, &p, None).unwrap();
        let lu = lire(&conn).unwrap();
        assert_eq!(lu.textes(TypeDocument::Facture).titre, "FACTURE DEFINITIVE");
        assert_eq!(lu.couleur_documents, [0x10, 0x80, 0xff]);

        p.textes_documents[0].titre = " ".into();
        assert!(enregistrer_sans_controle(&conn, &p, None).is_err());
        assert_eq!(couleur_depuis_texte("#2D3A8C"), Some(COULEUR_PAR_DEFAUT));
        assert_eq!(couleur_depuis_texte("2d3a8c"), Some(COULEUR_PAR_DEFAUT));
        assert_eq!(couleur_depuis_texte("#2d3a8"), None);
        assert_eq!(couleur_en_texte(COULEUR_PAR_DEFAUT), "#2d3a8c");
    }

    #[test]
    fn dossiers_export() {
        let conn = ouvrir_en_memoire().unwrap();
        let mut p = lire(&conn).unwrap();
        assert_eq!(p.dossier_export(TypeDocument::Facture), "");
        p.dossiers_export[4] = "  C:\\Documents\\Factures ".into();
        enregistrer_sans_controle(&conn, &p, None).unwrap();
        let lu = lire(&conn).unwrap();
        assert_eq!(
            lu.dossier_export(TypeDocument::Facture),
            "C:\\Documents\\Factures"
        );
        assert_eq!(lu.dossier_export(TypeDocument::Devis), "");
    }

    #[test]
    fn listes_vides_refusees() {
        let conn = ouvrir_en_memoire().unwrap();
        let sans_tva = Parametres {
            taux_tva: vec![],
            ..Default::default()
        };
        assert!(enregistrer_sans_controle(&conn, &sans_tva, None).is_err());
        let sans_mode = Parametres {
            modes_paiement: vec![" ".into()],
            ..Default::default()
        };
        assert!(enregistrer_sans_controle(&conn, &sans_mode, None).is_err());
    }
}
