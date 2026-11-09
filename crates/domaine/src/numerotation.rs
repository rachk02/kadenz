//! Format des numéros de documents (sections 5.4 et 6.1 du cahier des charges).
//!
//! - Compteur mensuel (par défaut) : `{Préfixe}{AAAA}-{MM}-{rang}`, ex. `F2026-05-0001`.
//! - Compteur annuel : `{Préfixe}{AAAA}-{rang}`, ex. `F2026-0001` (question 12 du
//!   cahier, tranchée le 2026-11-09).
//!
//! Le rang est complété de zéros jusqu'au nombre de chiffres réglé ; au-delà, il
//! s'allonge sans perte. Le compteur lui-même est tenu en base (`kadenz-stockage`),
//! dans la transaction de validation. Année et mois sont ceux de la date du
//! document (question 1, tranchée le 2026-11-09).

use crate::document::TypeDocument;

/// Fréquence de remise à zéro du compteur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RemiseAZero {
    #[default]
    Mensuelle,
    Annuelle,
}

impl RemiseAZero {
    pub fn code(self) -> &'static str {
        match self {
            RemiseAZero::Mensuelle => "mensuelle",
            RemiseAZero::Annuelle => "annuelle",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Self> {
        match code {
            "mensuelle" => Some(RemiseAZero::Mensuelle),
            "annuelle" => Some(RemiseAZero::Annuelle),
            _ => None,
        }
    }
}

/// Nombre de chiffres du rang : bornes acceptées par le réglage.
pub const CHIFFRES_MIN: u8 = 1;
pub const CHIFFRES_MAX: u8 = 9;
/// Longueur maximale d'un préfixe.
pub const PREFIXE_MAX: usize = 6;

/// Réglage de la numérotation (section 5.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Numerotation {
    /// Préfixe de chaque type, dans l'ordre de `TypeDocument::TOUS`.
    pub prefixes: [String; 6],
    pub remise: RemiseAZero,
    pub chiffres: u8,
}

impl Default for Numerotation {
    /// Valeurs par défaut du cahier : préfixes D, P, C, L, F, A, compteur mensuel,
    /// rang sur 4 chiffres.
    fn default() -> Self {
        Numerotation {
            prefixes: TypeDocument::TOUS.map(|t| t.prefixe().to_string()),
            remise: RemiseAZero::Mensuelle,
            chiffres: 4,
        }
    }
}

fn rang_du_type(t: TypeDocument) -> usize {
    TypeDocument::TOUS
        .iter()
        .position(|x| *x == t)
        .expect("type présent dans TOUS")
}

impl Numerotation {
    pub fn prefixe(&self, t: TypeDocument) -> &str {
        &self.prefixes[rang_du_type(t)]
    }

    /// Mois du compteur : celui de la date en remise mensuelle, 0 en remise annuelle
    /// (un seul compteur pour l'année).
    pub fn mois_du_compteur(&self, mois: u8) -> u8 {
        match self.remise {
            RemiseAZero::Mensuelle => mois,
            RemiseAZero::Annuelle => 0,
        }
    }

    /// Formate un numéro.
    pub fn formater(&self, t: TypeDocument, annee: u16, mois: u8, rang: u32) -> String {
        let n = usize::from(self.chiffres);
        let prefixe = self.prefixe(t);
        match self.remise {
            RemiseAZero::Mensuelle => {
                debug_assert!((1..=12).contains(&mois));
                format!("{prefixe}{annee:04}-{mois:02}-{rang:0n$}")
            }
            RemiseAZero::Annuelle => format!("{prefixe}{annee:04}-{rang:0n$}"),
        }
    }

    /// Nettoie et contrôle le réglage : préfixes de 1 à 6 lettres non accentuées
    /// (un chiffre se confondrait avec l'année qui suit), distincts entre types ;
    /// nombre de chiffres dans les bornes.
    pub fn nettoyee(&self) -> Result<Numerotation, String> {
        let mut n = self.clone();
        for (rang, p) in n.prefixes.iter_mut().enumerate() {
            let libelle = TypeDocument::TOUS[rang];
            *p = p.trim().to_uppercase();
            if p.is_empty() {
                return Err(format!("le préfixe « {libelle} » est obligatoire"));
            }
            if p.chars().count() > PREFIXE_MAX || !p.chars().all(|c| c.is_ascii_alphabetic()) {
                return Err(format!(
                    "le préfixe « {libelle} » doit compter de 1 à {PREFIXE_MAX} lettres, sans chiffre, espace ni accent"
                ));
            }
        }
        for i in 0..n.prefixes.len() {
            for j in (i + 1)..n.prefixes.len() {
                if n.prefixes[i] == n.prefixes[j] {
                    return Err(format!(
                        "les types « {} » et « {} » ont le même préfixe « {} »",
                        TypeDocument::TOUS[i],
                        TypeDocument::TOUS[j],
                        n.prefixes[i]
                    ));
                }
            }
        }
        if !(CHIFFRES_MIN..=CHIFFRES_MAX).contains(&n.chiffres) {
            return Err(format!(
                "le nombre de chiffres du rang doit être compris entre {CHIFFRES_MIN} et {CHIFFRES_MAX}"
            ));
        }
        Ok(n)
    }
}

/// Format par défaut (compteur mensuel, préfixes du cahier, 4 chiffres).
pub fn formater_numero(type_document: TypeDocument, annee: u16, mois: u8, rang: u32) -> String {
    Numerotation::default().formater(type_document, annee, mois, rang)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exemple_du_cahier() {
        assert_eq!(
            formater_numero(TypeDocument::Devis, 2026, 5, 4),
            "D2026-05-0004"
        );
    }

    #[test]
    fn prefixes() {
        assert_eq!(
            formater_numero(TypeDocument::BonCommande, 2026, 5, 3),
            "C2026-05-0003"
        );
        assert_eq!(
            formater_numero(TypeDocument::Livraison, 2026, 5, 1),
            "L2026-05-0001"
        );
        assert_eq!(
            formater_numero(TypeDocument::Facture, 2026, 12, 1),
            "F2026-12-0001"
        );
        assert_eq!(
            formater_numero(TypeDocument::Proforma, 2026, 1, 1),
            "P2026-01-0001"
        );
    }

    #[test]
    fn rang_au_dela_de_9999() {
        assert_eq!(
            formater_numero(TypeDocument::Facture, 2026, 5, 12_345),
            "F2026-05-12345"
        );
    }

    #[test]
    fn compteur_annuel() {
        let n = Numerotation {
            remise: RemiseAZero::Annuelle,
            ..Default::default()
        };
        assert_eq!(n.formater(TypeDocument::Facture, 2026, 5, 1), "F2026-0001");
        assert_eq!(n.mois_du_compteur(5), 0);
        assert_eq!(Numerotation::default().mois_du_compteur(5), 5);
    }

    #[test]
    fn prefixe_et_chiffres_regles() {
        let mut n = Numerotation {
            chiffres: 6,
            ..Default::default()
        };
        n.prefixes[4] = "fa".into();
        let n = n.nettoyee().unwrap();
        assert_eq!(
            n.formater(TypeDocument::Facture, 2026, 5, 12),
            "FA2026-05-000012"
        );
    }

    #[test]
    fn reglage_controle() {
        let mut n = Numerotation::default();
        n.prefixes[4] = " ".into();
        assert!(n.nettoyee().is_err());
        n.prefixes[4] = "F A".into();
        assert!(n.nettoyee().is_err());
        n.prefixes[4] = "É".into();
        assert!(n.nettoyee().is_err());
        n.prefixes[4] = "F2".into();
        assert!(n.nettoyee().is_err());
        n.prefixes[4] = "d".into();
        let e = n.nettoyee().unwrap_err();
        assert!(e.contains("même préfixe"), "{e}");
        let n = Numerotation {
            chiffres: 0,
            ..Default::default()
        };
        assert!(n.nettoyee().is_err());
        let codes = [RemiseAZero::Mensuelle, RemiseAZero::Annuelle];
        for r in codes {
            assert_eq!(RemiseAZero::depuis_code(r.code()), Some(r));
        }
    }
}
