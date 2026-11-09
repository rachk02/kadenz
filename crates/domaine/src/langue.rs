//! Langue de l'interface, choisie par compte (décision du 2026-11-09).
//!
//! Elle ne vaut que pour l'interface : les documents remis aux clients (PDF,
//! montant en lettres) restent en français.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Langue {
    #[default]
    Francais,
    Anglais,
    Arabe,
    Japonais,
    Chinois,
    Turc,
    Espagnol,
}

impl Langue {
    pub const TOUTES: [Langue; 7] = [
        Langue::Francais,
        Langue::Anglais,
        Langue::Arabe,
        Langue::Japonais,
        Langue::Chinois,
        Langue::Turc,
        Langue::Espagnol,
    ];

    /// Code ISO 639-1, stocké en base et utilisé pour charger les traductions.
    pub fn code(self) -> &'static str {
        match self {
            Langue::Francais => "fr",
            Langue::Anglais => "en",
            Langue::Arabe => "ar",
            Langue::Japonais => "ja",
            Langue::Chinois => "zh",
            Langue::Turc => "tr",
            Langue::Espagnol => "es",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|l| l.code() == code)
    }

    /// Nom de la langue écrit dans cette langue, pour le sélecteur.
    pub fn nom_natif(self) -> &'static str {
        match self {
            Langue::Francais => "Français",
            Langue::Anglais => "English",
            Langue::Arabe => "العربية",
            Langue::Japonais => "日本語",
            Langue::Chinois => "中文",
            Langue::Turc => "Türkçe",
            Langue::Espagnol => "Español",
        }
    }
}

impl fmt::Display for Langue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.nom_natif())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_stables() {
        assert_eq!(Langue::default().code(), "fr");
        for l in Langue::TOUTES {
            assert_eq!(Langue::depuis_code(l.code()), Some(l));
        }
        assert_eq!(Langue::depuis_code("de"), None);
    }
}
