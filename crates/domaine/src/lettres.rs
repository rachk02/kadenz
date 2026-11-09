//! Montant en lettres, en français (cahier des charges, section 6.1).
//!
//! Deux orthographes, choisies par réglage (section 5.4) :
//! - rectifiée de 1990 : traits d'union entre tous les numéraux
//!   (« soixante-dix-mille-sept-cent-quarante ») ;
//! - traditionnelle : traits d'union seulement sous cent
//!   (« soixante-dix mille sept cent quarante »).
//!
//! Dans les deux cas, « million » et « milliard » sont des noms : ils prennent
//! la marque du pluriel et restent séparés par des espaces. « Cent » et « vingt »
//! (dans quatre-vingts) prennent un s quand ils sont multipliés et terminent le
//! nombre, ou précèdent « million » ou « milliard » ; « mille » est invariable.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orthographe {
    #[default]
    Rectifiee,
    Traditionnelle,
}

impl Orthographe {
    pub fn code(self) -> &'static str {
        match self {
            Orthographe::Rectifiee => "rectifiee",
            Orthographe::Traditionnelle => "traditionnelle",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Self> {
        match code {
            "rectifiee" => Some(Orthographe::Rectifiee),
            "traditionnelle" => Some(Orthographe::Traditionnelle),
            _ => None,
        }
    }

    fn separateur(self) -> &'static str {
        match self {
            Orthographe::Rectifiee => "-",
            Orthographe::Traditionnelle => " ",
        }
    }

    /// Liaison de « et un », « et onze » : « vingt et un » ou « vingt-et-un ».
    fn liaison_et(self) -> &'static str {
        match self {
            Orthographe::Rectifiee => "-et-",
            Orthographe::Traditionnelle => " et ",
        }
    }
}

const UNITES: [&str; 17] = [
    "zéro", "un", "deux", "trois", "quatre", "cinq", "six", "sept", "huit", "neuf", "dix", "onze",
    "douze", "treize", "quatorze", "quinze", "seize",
];

const DIZAINES: [&str; 7] = [
    "",
    "",
    "vingt",
    "trente",
    "quarante",
    "cinquante",
    "soixante",
];

/// Nombre de 1 à 99. `accord` : « quatre-vingts » prend son s (fin de nombre).
fn moins_de_cent(n: u64, o: Orthographe, accord: bool) -> String {
    debug_assert!((1..100).contains(&n));
    match n {
        0..=16 => UNITES[n as usize].to_string(),
        17..=19 => format!("dix-{}", UNITES[(n - 10) as usize]),
        20..=69 => {
            let (d, u) = (n / 10, n % 10);
            match u {
                0 => DIZAINES[d as usize].to_string(),
                1 => format!("{}{}un", DIZAINES[d as usize], o.liaison_et()),
                _ => format!("{}-{}", DIZAINES[d as usize], UNITES[u as usize]),
            }
        }
        71 => format!("soixante{}onze", o.liaison_et()),
        70..=79 => format!("soixante-{}", moins_de_cent(n - 60, o, accord)),
        80 if accord => "quatre-vingts".to_string(),
        80 => "quatre-vingt".to_string(),
        _ => format!("quatre-vingt-{}", moins_de_cent(n - 80, o, accord)),
    }
}

/// Nombre de 1 à 999.
fn moins_de_mille(n: u64, o: Orthographe, accord: bool) -> String {
    debug_assert!((1..1000).contains(&n));
    let (c, r) = (n / 100, n % 100);
    let sep = o.separateur();
    let mut s = match c {
        0 => String::new(),
        1 => "cent".to_string(),
        _ => {
            let pluriel = if r == 0 && accord { "s" } else { "" };
            format!("{}{sep}cent{pluriel}", UNITES[c as usize])
        }
    };
    if r > 0 {
        if !s.is_empty() {
            s.push_str(sep);
        }
        s.push_str(&moins_de_cent(r, o, accord));
    }
    s
}

/// Partie strictement inférieure au million : milliers et unités.
fn moins_d_un_million(n: u64, o: Orthographe) -> String {
    let (milliers, reste) = (n / 1000, n % 1000);
    let mut morceaux = Vec::new();
    match milliers {
        0 => {}
        1 => morceaux.push("mille".to_string()),
        _ => morceaux.push(format!(
            "{}{}mille",
            moins_de_mille(milliers, o, false),
            o.separateur()
        )),
    }
    if reste > 0 {
        morceaux.push(moins_de_mille(reste, o, true));
    }
    morceaux.join(o.separateur())
}

fn positif_en_lettres(n: u64, o: Orthographe) -> String {
    const MILLION: u64 = 1_000_000;
    const MILLIARD: u64 = 1_000_000_000;

    let (milliards, reste) = (n / MILLIARD, n % MILLIARD);
    let (millions, reste) = (reste / MILLION, reste % MILLION);

    // Les noms « milliard » et « million » restent séparés par des espaces.
    let mut segments = Vec::new();
    if milliards > 0 {
        let pluriel = if milliards > 1 { "s" } else { "" };
        segments.push(format!(
            "{} milliard{pluriel}",
            positif_en_lettres(milliards, o)
        ));
    }
    if millions > 0 {
        let pluriel = if millions > 1 { "s" } else { "" };
        segments.push(format!(
            "{} million{pluriel}",
            moins_de_mille(millions, o, true)
        ));
    }
    if reste > 0 {
        segments.push(moins_d_un_million(reste, o));
    }
    segments.join(" ")
}

/// Écrit un nombre entier en lettres, en minuscules.
pub fn en_lettres(n: i64, o: Orthographe) -> String {
    if n == 0 {
        return UNITES[0].to_string();
    }
    let absolu = positif_en_lettres(n.unsigned_abs(), o);
    if n < 0 {
        format!("moins {absolu}")
    } else {
        absolu
    }
}

/// Même chose, avec une majuscule initiale (début de la mention « Arrêté la présente... »
/// ou montant isolé sur un document).
pub fn en_lettres_majuscule(n: i64, o: Orthographe) -> String {
    let s = en_lettres(n, o);
    let mut caracteres = s.chars();
    match caracteres.next() {
        Some(premier) => premier.to_uppercase().chain(caracteres).collect(),
        None => s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Orthographe::{Rectifiee as R, Traditionnelle as T};

    fn verifier(n: i64, traditionnelle: &str, rectifiee: &str) {
        assert_eq!(
            en_lettres(n, T),
            traditionnelle,
            "{n} en orthographe traditionnelle"
        );
        assert_eq!(en_lettres(n, R), rectifiee, "{n} en orthographe rectifiée");
    }

    #[test]
    fn unites_et_dizaines() {
        verifier(0, "zéro", "zéro");
        verifier(1, "un", "un");
        verifier(16, "seize", "seize");
        verifier(17, "dix-sept", "dix-sept");
        verifier(21, "vingt et un", "vingt-et-un");
        verifier(22, "vingt-deux", "vingt-deux");
        verifier(70, "soixante-dix", "soixante-dix");
        verifier(71, "soixante et onze", "soixante-et-onze");
        verifier(77, "soixante-dix-sept", "soixante-dix-sept");
        verifier(80, "quatre-vingts", "quatre-vingts");
        verifier(81, "quatre-vingt-un", "quatre-vingt-un");
        verifier(91, "quatre-vingt-onze", "quatre-vingt-onze");
        verifier(99, "quatre-vingt-dix-neuf", "quatre-vingt-dix-neuf");
    }

    #[test]
    fn centaines() {
        verifier(100, "cent", "cent");
        verifier(101, "cent un", "cent-un");
        verifier(200, "deux cents", "deux-cents");
        verifier(201, "deux cent un", "deux-cent-un");
        verifier(280, "deux cent quatre-vingts", "deux-cent-quatre-vingts");
    }

    #[test]
    fn milliers() {
        verifier(1000, "mille", "mille");
        verifier(1001, "mille un", "mille-un");
        verifier(2000, "deux mille", "deux-mille");
        verifier(80_000, "quatre-vingt mille", "quatre-vingt-mille");
        verifier(200_000, "deux cent mille", "deux-cent-mille");
        verifier(
            70_740,
            "soixante-dix mille sept cent quarante",
            "soixante-dix-mille-sept-cent-quarante",
        );
        verifier(150_000, "cent cinquante mille", "cent-cinquante-mille");
    }

    #[test]
    fn millions_et_milliards() {
        verifier(1_000_000, "un million", "un million");
        verifier(2_000_000, "deux millions", "deux millions");
        verifier(
            280_000_000,
            "deux cent quatre-vingts millions",
            "deux-cent-quatre-vingts millions",
        );
        verifier(
            2_300_000,
            "deux millions trois cent mille",
            "deux millions trois-cent-mille",
        );
        verifier(
            1_234_567_890,
            "un milliard deux cent trente-quatre millions cinq cent soixante-sept mille huit cent quatre-vingt-dix",
            "un milliard deux-cent-trente-quatre millions cinq-cent-soixante-sept-mille-huit-cent-quatre-vingt-dix",
        );
        verifier(
            2_000_000_000_000,
            "deux mille milliards",
            "deux-mille milliards",
        );
    }

    #[test]
    fn negatif_et_majuscule() {
        assert_eq!(en_lettres(-21, T), "moins vingt et un");
        assert_eq!(
            en_lettres_majuscule(70_740, R),
            "Soixante-dix-mille-sept-cent-quarante"
        );
    }

    #[test]
    fn codes_orthographe() {
        for o in [R, T] {
            assert_eq!(Orthographe::depuis_code(o.code()), Some(o));
        }
    }
}
