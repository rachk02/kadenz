//! Analyse d'une table lue : reconnaissance des colonnes, conversion de chaque
//! ligne en fiche avec les règles du domaine, repérage des doublons.
//!
//! L'aperçu d'import affiche ce résultat avant tout enregistrement (cahier des
//! charges, section 6.8) ; seules les lignes valides sont ensuite enregistrées.

use std::collections::{HashMap, HashSet};

use kadenz_domaine::referentiel::{FicheProduit, FicheTiers, Sens, TypeTiers};
use kadenz_domaine::saisie;
use kadenz_domaine::texte::cle_comparaison;

use crate::lecture::Table;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statut {
    Valide,
    /// Fiche déjà présente, en base ou plus haut dans le fichier : ignorée.
    Doublon(String),
    /// Ligne refusée : la raison est destinée à l'utilisateur.
    Erreur(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LigneAnalysee<F> {
    /// Numéro de la ligne dans le fichier, à partir de 1.
    pub numero: usize,
    /// Nom du tiers ou désignation du produit, tel que lu.
    pub libelle: String,
    pub fiche: Option<F>,
    pub statut: Statut,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Analyse<F> {
    pub lignes: Vec<LigneAnalysee<F>>,
    /// En-têtes reconnus, avec le champ auquel chacun est associé.
    pub colonnes_reconnues: Vec<(String, &'static str)>,
    /// En-têtes non reconnus : leur contenu n'est pas importé.
    pub colonnes_ignorees: Vec<String>,
}

impl<F: Clone> Analyse<F> {
    /// Fiches des lignes valides, dans l'ordre du fichier.
    pub fn fiches_valides(&self) -> Vec<F> {
        self.lignes
            .iter()
            .filter(|l| l.statut == Statut::Valide)
            .filter_map(|l| l.fiche.clone())
            .collect()
    }

    /// Nombre de lignes valides, en doublon et en erreur.
    pub fn comptes(&self) -> (usize, usize, usize) {
        self.lignes
            .iter()
            .fold((0, 0, 0), |(v, d, e), l| match l.statut {
                Statut::Valide => (v + 1, d, e),
                Statut::Doublon(_) => (v, d + 1, e),
                Statut::Erreur(_) => (v, d, e + 1),
            })
    }
}

/// Clé de doublon d'un tiers : son nom, à la casse, aux accents et aux espaces près.
pub fn cle_tiers(nom: &str) -> String {
    cle_comparaison(nom)
}

/// Clé de doublon d'un produit : sa référence, à la casse près (comme la contrainte
/// d'unicité de la base, insensible à la casse des lettres non accentuées).
pub fn cle_produit(reference: &str) -> String {
    reference.trim().to_ascii_lowercase()
}

/// Forme de comparaison d'un en-tête : « ADRESSE / BOÎTE POSTALE » et
/// « adresse/boite postale » se valent.
fn cle_entete(entete: &str) -> String {
    cle_comparaison(entete)
        .replace(" /", "/")
        .replace("/ ", "/")
}

/// Résultat de la reconnaissance des en-têtes : rang de colonne par champ,
/// en-têtes reconnus avec leur champ, en-têtes ignorés.
type Association<C> = (HashMap<C, usize>, Vec<(String, &'static str)>, Vec<String>);

/// Rang de colonne de chaque champ reconnu. Si deux colonnes désignent le même
/// champ, la première l'emporte et la seconde est ignorée.
fn associer<C: Copy + Eq + std::hash::Hash>(
    entetes: &[String],
    alias: &[(&str, C, &'static str)],
) -> Association<C> {
    let mut colonnes = HashMap::new();
    let mut reconnues = Vec::new();
    let mut ignorees = Vec::new();
    for (rang, entete) in entetes.iter().enumerate() {
        if entete.trim().is_empty() {
            continue;
        }
        let cle = cle_entete(entete);
        match alias.iter().find(|(a, _, _)| *a == cle) {
            Some((_, champ, nom)) if !colonnes.contains_key(champ) => {
                colonnes.insert(*champ, rang);
                reconnues.push((entete.clone(), *nom));
            }
            _ => ignorees.push(entete.clone()),
        }
    }
    (colonnes, reconnues, ignorees)
}

fn cellule<C: Eq + std::hash::Hash>(
    colonnes: &HashMap<C, usize>,
    champ: C,
    ligne: &[String],
) -> String {
    colonnes
        .get(&champ)
        .and_then(|&rang| ligne.get(rang))
        .map(|c| c.trim().to_string())
        .unwrap_or_default()
}

fn exiger_colonnes<C: Eq + std::hash::Hash>(
    colonnes: &HashMap<C, usize>,
    obligatoires: &[(C, &str)],
    entetes: &[String],
) -> Result<(), String> {
    let absentes: Vec<&str> = obligatoires
        .iter()
        .filter(|(c, _)| !colonnes.contains_key(c))
        .map(|(_, nom)| *nom)
        .collect();
    if absentes.is_empty() {
        return Ok(());
    }
    Err(format!(
        "colonne obligatoire absente : {} (en-têtes lus : {})",
        absentes.join(", "),
        if entetes.is_empty() {
            "aucun".to_string()
        } else {
            entetes.join(", ")
        }
    ))
}

/// Statut final d'une ligne convertie : doublon si la clé est déjà connue
/// (en base ou plus haut dans le fichier).
fn statut_final(
    cle: String,
    existantes: &HashSet<String>,
    vues: &mut HashMap<String, usize>,
    numero: usize,
) -> Statut {
    if existantes.contains(&cle) {
        return Statut::Doublon("existe déjà".into());
    }
    if let Some(premiere) = vues.get(&cle) {
        return Statut::Doublon(format!("déjà présent ligne {premiere}"));
    }
    vues.insert(cle, numero);
    Statut::Valide
}

// ---------------------------------------------------------------- Tiers

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ChampTiers {
    Nom,
    Adresse,
    Telephone,
    Localisation,
    Ifu,
    Rccm,
    Regime,
    Type,
}

/// En-têtes reconnus, après mise en forme de comparaison. Ceux de l'outil Excel
/// d'origine (NOM, ADRESSE/BOÎTE POSTALE, TELEPHONE, Localisation, IFU) y figurent.
const ALIAS_TIERS: &[(&str, ChampTiers, &str)] = &[
    ("nom", ChampTiers::Nom, "nom"),
    ("raison sociale", ChampTiers::Nom, "nom"),
    ("adresse", ChampTiers::Adresse, "adresse"),
    ("adresse/boite postale", ChampTiers::Adresse, "adresse"),
    ("boite postale", ChampTiers::Adresse, "adresse"),
    ("telephone", ChampTiers::Telephone, "téléphone"),
    ("tel", ChampTiers::Telephone, "téléphone"),
    ("localisation", ChampTiers::Localisation, "localisation"),
    ("ifu", ChampTiers::Ifu, "IFU"),
    ("rccm", ChampTiers::Rccm, "RCCM"),
    (
        "regime d'imposition",
        ChampTiers::Regime,
        "régime d'imposition",
    ),
    ("regime", ChampTiers::Regime, "régime d'imposition"),
    ("type", ChampTiers::Type, "type"),
];

fn type_depuis_texte(texte: &str, defaut: TypeTiers) -> Result<TypeTiers, String> {
    match cle_comparaison(texte).as_str() {
        "" => Ok(defaut),
        "client" => Ok(TypeTiers::Client),
        "fournisseur" => Ok(TypeTiers::Fournisseur),
        "les deux" | "les_deux" | "client et fournisseur" => Ok(TypeTiers::LesDeux),
        _ => Err(format!(
            "type « {texte} » inconnu : Client, Fournisseur ou Les deux"
        )),
    }
}

/// Analyse une table de tiers. `type_par_defaut` s'applique aux lignes sans
/// type ; `existants` contient les clés (`cle_tiers`) des tiers déjà en base.
pub fn analyser_tiers(
    table: &Table,
    type_par_defaut: TypeTiers,
    existants: &HashSet<String>,
) -> Result<Analyse<FicheTiers>, String> {
    let (colonnes, colonnes_reconnues, colonnes_ignorees) = associer(&table.entetes, ALIAS_TIERS);
    exiger_colonnes(&colonnes, &[(ChampTiers::Nom, "Nom")], &table.entetes)?;
    let mut vues = HashMap::new();
    let lignes = table
        .lignes
        .iter()
        .map(|(numero, ligne)| {
            let c = |champ| cellule(&colonnes, champ, ligne);
            let libelle = c(ChampTiers::Nom);
            let conversion =
                type_depuis_texte(&c(ChampTiers::Type), type_par_defaut).and_then(|type_tiers| {
                    FicheTiers {
                        nom: c(ChampTiers::Nom),
                        adresse: c(ChampTiers::Adresse),
                        telephone: c(ChampTiers::Telephone),
                        localisation: c(ChampTiers::Localisation),
                        ifu: c(ChampTiers::Ifu),
                        rccm: c(ChampTiers::Rccm),
                        regime_imposition: c(ChampTiers::Regime),
                        type_tiers,
                    }
                    .nettoyee()
                });
            match conversion {
                Ok(fiche) => LigneAnalysee {
                    numero: *numero,
                    libelle,
                    statut: statut_final(cle_tiers(&fiche.nom), existants, &mut vues, *numero),
                    fiche: Some(fiche),
                },
                Err(raison) => LigneAnalysee {
                    numero: *numero,
                    libelle,
                    fiche: None,
                    statut: Statut::Erreur(raison),
                },
            }
        })
        .collect();
    Ok(Analyse {
        lignes,
        colonnes_reconnues,
        colonnes_ignorees,
    })
}

// ---------------------------------------------------------------- Produits

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ChampProduit {
    Designation,
    Reference,
    Prix,
    Sens,
}

/// En-têtes reconnus ; ceux de l'outil Excel d'origine (DÉSIGNATION, RÉFÉRENCE,
/// PRIX) y figurent.
const ALIAS_PRODUITS: &[(&str, ChampProduit, &str)] = &[
    ("designation", ChampProduit::Designation, "désignation"),
    ("libelle", ChampProduit::Designation, "désignation"),
    ("reference", ChampProduit::Reference, "référence"),
    ("ref", ChampProduit::Reference, "référence"),
    ("code", ChampProduit::Reference, "référence"),
    ("prix", ChampProduit::Prix, "prix"),
    ("prix unitaire", ChampProduit::Prix, "prix"),
    ("prix (fcfa)", ChampProduit::Prix, "prix"),
    ("prix unitaire (fcfa)", ChampProduit::Prix, "prix"),
    ("sens", ChampProduit::Sens, "sens"),
    ("sens d'utilisation", ChampProduit::Sens, "sens"),
];

fn sens_depuis_texte(texte: &str, defaut: Sens) -> Result<Sens, String> {
    match cle_comparaison(texte).as_str() {
        "" => Ok(defaut),
        "vente" => Ok(Sens::Vente),
        "achat" => Ok(Sens::Achat),
        "les deux" | "les_deux" | "vente et achat" => Ok(Sens::LesDeux),
        _ => Err(format!(
            "sens « {texte} » inconnu : Vente, Achat ou Les deux"
        )),
    }
}

fn prix_depuis_texte(texte: &str) -> Result<i64, String> {
    if texte.is_empty() {
        return Err("prix absent".into());
    }
    saisie::montant_depuis_texte(texte)
        .ok_or_else(|| format!("prix « {texte} » invalide : montant entier en FCFA attendu"))
}

/// Analyse une table de produits. `sens_par_defaut` s'applique aux lignes sans
/// sens ; `existantes` contient les clés (`cle_produit`) des références en base.
pub fn analyser_produits(
    table: &Table,
    sens_par_defaut: Sens,
    existantes: &HashSet<String>,
) -> Result<Analyse<FicheProduit>, String> {
    let (colonnes, colonnes_reconnues, colonnes_ignorees) =
        associer(&table.entetes, ALIAS_PRODUITS);
    exiger_colonnes(
        &colonnes,
        &[
            (ChampProduit::Designation, "Désignation"),
            (ChampProduit::Reference, "Référence"),
            (ChampProduit::Prix, "Prix"),
        ],
        &table.entetes,
    )?;
    let mut vues = HashMap::new();
    let lignes = table
        .lignes
        .iter()
        .map(|(numero, ligne)| {
            let c = |champ| cellule(&colonnes, champ, ligne);
            let libelle = c(ChampProduit::Designation);
            let conversion = prix_depuis_texte(&c(ChampProduit::Prix)).and_then(|prix| {
                let sens = sens_depuis_texte(&c(ChampProduit::Sens), sens_par_defaut)?;
                FicheProduit {
                    designation: c(ChampProduit::Designation),
                    reference: c(ChampProduit::Reference),
                    prix,
                    sens,
                }
                .nettoyee()
            });
            match conversion {
                Ok(fiche) => LigneAnalysee {
                    numero: *numero,
                    libelle,
                    statut: statut_final(
                        cle_produit(&fiche.reference),
                        existantes,
                        &mut vues,
                        *numero,
                    ),
                    fiche: Some(fiche),
                },
                Err(raison) => LigneAnalysee {
                    numero: *numero,
                    libelle,
                    fiche: None,
                    statut: Statut::Erreur(raison),
                },
            }
        })
        .collect();
    Ok(Analyse {
        lignes,
        colonnes_reconnues,
        colonnes_ignorees,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lecture::lire_csv;

    #[test]
    fn tiers_aux_entetes_de_l_outil_excel() {
        let csv = "NOM;ADRESSE/BOÎTE POSTALE;TELEPHONE;Localisation;IFU;Commentaire\n\
                   Alpha Essai;01 BP 1 Ville;70 00 00 01;Zone A;000123A;x\n\
                   ;sans nom;;;;\n\
                   alpha  essai;doublon;;;;\n\
                   Béta Essai;;;;;\n";
        let table = lire_csv(csv.as_bytes()).unwrap();
        let existants = HashSet::from([cle_tiers("Béta Essai")]);
        let a = analyser_tiers(&table, TypeTiers::Client, &existants).unwrap();
        assert_eq!(a.colonnes_ignorees, ["Commentaire"]);
        assert_eq!(a.colonnes_reconnues.len(), 5);
        assert_eq!(a.comptes(), (1, 2, 1));
        let alpha = a.lignes[0].fiche.as_ref().unwrap();
        assert_eq!(alpha.adresse, "01 BP 1 Ville");
        assert_eq!(alpha.type_tiers, TypeTiers::Client);
        assert!(matches!(a.lignes[1].statut, Statut::Erreur(_)));
        assert_eq!(
            a.lignes[2].statut,
            Statut::Doublon("déjà présent ligne 2".into())
        );
        assert_eq!(a.lignes[3].statut, Statut::Doublon("existe déjà".into()));
        assert_eq!(a.fiches_valides().len(), 1);
    }

    #[test]
    fn type_lu_ou_par_defaut() {
        let csv = "Nom,Type\nA,Fournisseur\nB,\nC,les deux\nD,partenaire\n";
        let table = lire_csv(csv.as_bytes()).unwrap();
        let a = analyser_tiers(&table, TypeTiers::Client, &HashSet::new()).unwrap();
        let types: Vec<_> = a
            .lignes
            .iter()
            .map(|l| l.fiche.as_ref().map(|f| f.type_tiers))
            .collect();
        assert_eq!(
            types,
            [
                Some(TypeTiers::Fournisseur),
                Some(TypeTiers::Client),
                Some(TypeTiers::LesDeux),
                None
            ]
        );
    }

    #[test]
    fn produits_et_prix() {
        let csv = "DÉSIGNATION;RÉFÉRENCE;PRIX\n\
                   Article un;ESS-1;485 000,00 CFA\n\
                   Article deux;ESS-2;12,5\n\
                   Article trois;ess-1;100\n\
                   Article quatre;ESS-4;\n\
                   Article cinq;ESS-5;1250000\n";
        let table = lire_csv(csv.as_bytes()).unwrap();
        let a =
            analyser_produits(&table, Sens::Vente, &HashSet::from([cle_produit("ESS-5")])).unwrap();
        assert_eq!(a.lignes[0].fiche.as_ref().unwrap().prix, 485_000);
        assert!(
            matches!(a.lignes[1].statut, Statut::Erreur(_)),
            "prix non entier"
        );
        assert!(matches!(a.lignes[2].statut, Statut::Doublon(_)));
        assert!(
            matches!(a.lignes[3].statut, Statut::Erreur(_)),
            "prix absent"
        );
        assert!(matches!(a.lignes[4].statut, Statut::Doublon(_)));
        assert_eq!(a.comptes(), (1, 2, 2));
    }

    #[test]
    fn colonne_obligatoire_absente() {
        let table = lire_csv("Désignation;Prix\nA;10\n".as_bytes()).unwrap();
        let erreur = analyser_produits(&table, Sens::Vente, &HashSet::new()).unwrap_err();
        assert!(erreur.contains("Référence"), "{erreur}");
    }
}
