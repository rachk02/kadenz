//! Message de refus composé : un modèle de phrase complète, où chaque `{}`
//! reçoit une valeur, pour que l'interface traduise le modèle puis les valeurs.
//!
//! Règle : une valeur n'est jamais un fragment de phrase à accorder (un libellé
//! de type, un préfixe, un nombre), sans quoi la traduction serait fausse.

use std::fmt;

/// Refus traduisible : le modèle sert de clé de traduction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageCompose {
    pub modele: &'static str,
    pub valeurs: Vec<String>,
}

impl MessageCompose {
    pub fn new(modele: &'static str, valeurs: &[&dyn fmt::Display]) -> Self {
        MessageCompose {
            modele,
            valeurs: valeurs.iter().map(|v| v.to_string()).collect(),
        }
    }
}

/// Composition en français : chaque `{}` reçoit la valeur suivante.
impl fmt::Display for MessageCompose {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut valeurs = self.valeurs.iter();
        let mut reste = self.modele;
        while let Some(position) = reste.find("{}") {
            f.write_str(&reste[..position])?;
            match valeurs.next() {
                Some(v) => f.write_str(v)?,
                None => f.write_str("{}")?,
            }
            reste = &reste[position + 2..];
        }
        f.write_str(reste)
    }
}

impl std::error::Error for MessageCompose {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composition() {
        let m = MessageCompose::new("les types « {} » et « {} »", &[&"Devis", &3]);
        assert_eq!(m.to_string(), "les types « Devis » et « 3 »");
        assert_eq!(m.valeurs, ["Devis", "3"]);
        let m = MessageCompose::new("{} et {}", &[&1]);
        assert_eq!(m.to_string(), "1 et {}");
    }
}
