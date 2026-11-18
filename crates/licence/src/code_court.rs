//! Codes de licence courts (décision 74, `docs/spec-codes-licence.md`).
//!
//! Un code d'activation de 25 caractères en 5 groupes,
//! `XXXXX-XXXXX-XXXXX-XXXXX-XXXXX`, tiré dans un alphabet de 32 signes sans O,
//! 0, I ni 1 : 24 caractères au hasard (120 bits) et un caractère de contrôle
//! (Luhn en base 32). Le code ne porte aucune donnée : le portail garde ce
//! qu'il ouvre et renvoie la licence signée à l'activation.

use kadenz_domaine::message::MessageCompose;

/// Alphabet des codes : A à Z sans I ni O, puis 2 à 9.
pub const ALPHABET: &str = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
/// Caractères d'un code complet, contrôle compris.
pub const LONGUEUR: usize = 25;
/// Caractères par groupe.
pub const GROUPE: usize = 5;

const BASE: u32 = 32;

/// Modèles d'erreur de ce module, pour vérifier leurs traductions.
pub const MODELES_ERREUR: &[&str] = &[
    "code incomplet : {} caractères sur 25",
    "code erroné : vérifiez la saisie",
    "les lettres O et I et les chiffres 0 et 1 n'existent pas dans un code : vérifiez la saisie",
];

fn rang(c: char) -> Option<u32> {
    ALPHABET.find(c).map(|i| i as u32)
}

fn signe(r: u32) -> char {
    ALPHABET.as_bytes()[r as usize] as char
}

/// Somme de Luhn en base 32 ; `double_premier` : le caractère le plus à droite
/// est doublé (calcul du contrôle) ou non (vérification d'un code complet).
fn somme(rangs: impl DoubleEndedIterator<Item = u32>, double_premier: bool) -> u32 {
    let mut facteur = if double_premier { 2 } else { 1 };
    let mut total = 0;
    for r in rangs.rev() {
        let a = facteur * r;
        total += a / BASE + a % BASE;
        facteur = if facteur == 2 { 1 } else { 2 };
    }
    total
}

/// Caractère de contrôle des 24 premiers caractères (déjà dans l'alphabet).
fn controle(corps: &str) -> char {
    let s = somme(corps.chars().filter_map(rang), true);
    signe((BASE - s % BASE) % BASE)
}

/// Saisie ramenée aux caractères utiles : majuscules, tout ce qui n'est pas
/// dans l'alphabet retiré (tirets, espaces, retours à la ligne d'un collage),
/// 25 caractères au plus.
pub fn normaliser(saisie: &str) -> String {
    saisie
        .chars()
        .flat_map(char::to_uppercase)
        .filter(|c| rang(*c).is_some())
        .take(LONGUEUR)
        .collect()
}

/// Caractères utiles groupés par 5, séparés par un tiret ; un code partiel
/// garde ses groupes complets suivis d'un tiret, pour que la saisie continue
/// naturellement.
pub fn ecrire(utiles: &str) -> String {
    let mut sortie = String::with_capacity(LONGUEUR + 4);
    for (i, c) in utiles.chars().take(LONGUEUR).enumerate() {
        if i > 0 && i % GROUPE == 0 {
            sortie.push('-');
        }
        sortie.push(c);
    }
    sortie
}

/// Caractère retiré d'une saisie sans rien signaler : les séparateurs.
fn separateur(c: char) -> bool {
    matches!(c, '-' | '_' | ' ' | '\t' | '\r' | '\n' | '\u{a0}' | '\u{2010}' | '\u{2011}' | '\u{2013}')
}

/// Ce que la mise en forme a dû retirer d'autre que des séparateurs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Aucun,
    /// O, 0, I ou 1 : signes absents des codes.
    Ambigu,
    /// Tout autre caractère hors de l'alphabet.
    Autre,
}

/// Résultat de [`mettre_en_forme`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MiseEnForme {
    /// Texte à afficher, groupé par 5.
    pub texte: String,
    /// Position du curseur dans `texte`, en octets (comme Slint).
    pub curseur: usize,
    pub signal: Signal,
}

/// Mise en forme d'un champ en cours de saisie (`edited`), à partir du texte
/// affiché avant la modification et du texte brut après. La fin de la partie
/// tapée, collée ou effacée se trouve par le suffixe commun aux deux textes ;
/// le curseur se place après le même caractère utile dans le texte remis en
/// forme. Les deux textes peuvent contenir de l'UTF-8 (collage).
pub fn mettre_en_forme(precedent: &str, nouveau: &str) -> MiseEnForme {
    let avant: Vec<char> = precedent.chars().collect();
    let apres: Vec<char> = nouveau.chars().collect();
    let commun = avant.iter().rev().zip(apres.iter().rev()).take_while(|(a, b)| a == b).count();
    let fin = apres.len() - commun.min(apres.len());
    let utiles = normaliser(nouveau);
    let n = normaliser(&apres[..fin].iter().collect::<String>()).chars().count().min(utiles.chars().count());
    let signal = apres
        .iter()
        .flat_map(|c| c.to_uppercase())
        .filter(|c| rang(*c).is_none() && !separateur(*c))
        .fold(Signal::Aucun, |s, c| match (s, c) {
            (Signal::Ambigu, _) | (_, 'O' | '0' | 'I' | '1') => Signal::Ambigu,
            _ => Signal::Autre,
        });
    // Après n caractères utiles : n, plus un tiret par groupe complet passé,
    // sauf en fin de groupe, où le curseur reste avant le tiret. Le texte écrit
    // n'a que des caractères ASCII : caractères et octets se confondent.
    let tirets = if n == 0 { 0 } else { (n - 1) / GROUPE };
    MiseEnForme { texte: ecrire(&utiles), curseur: n + tirets, signal }
}

/// Code d'activation contrôlé, sous sa forme écrite (`XXXXX-...`), ou l'erreur
/// à montrer, avant tout appel au portail.
pub fn verifier(saisie: &str) -> Result<String, MessageCompose> {
    let utiles = normaliser(saisie);
    let n = utiles.chars().count();
    if n < LONGUEUR {
        let interdits = saisie.chars().flat_map(char::to_uppercase).any(|c| matches!(c, 'O' | 'I' | '0' | '1'));
        if interdits {
            return Err(MessageCompose::new(MODELES_ERREUR[2], &[]));
        }
        return Err(MessageCompose::new(MODELES_ERREUR[0], &[&n]));
    }
    if somme(utiles.chars().filter_map(rang), false) % BASE != 0 {
        return Err(MessageCompose::new(MODELES_ERREUR[1], &[]));
    }
    Ok(ecrire(&utiles))
}

/// Tire un code neuf (générateur cryptographique du système).
pub fn tirer() -> Result<String, String> {
    let mut octets = [0u8; 24];
    getrandom::fill(&mut octets).map_err(|e| format!("tirage du code impossible : {e}"))?;
    // 256 est un multiple de 32 : prendre l'octet modulo 32 ne biaise aucun signe.
    let corps: String = octets.iter().map(|o| signe(u32::from(*o) % BASE)).collect();
    let mut code = corps.clone();
    code.push(controle(&corps));
    Ok(ecrire(&code))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complet(corps: &str) -> String {
        let mut c = corps.to_string();
        c.push(controle(corps));
        c
    }

    #[test]
    fn alphabet_sans_signes_ambigus() {
        assert_eq!(ALPHABET.chars().count(), 32);
        for c in ['O', 'I', '0', '1'] {
            assert!(!ALPHABET.contains(c));
        }
    }

    #[test]
    fn codes_tires_valides_et_distincts() {
        let mut vus = std::collections::HashSet::new();
        for _ in 0..500 {
            let code = tirer().unwrap();
            assert_eq!(code.len(), LONGUEUR + 4);
            assert_eq!(verifier(&code).unwrap(), code);
            assert!(vus.insert(code));
        }
    }

    #[test]
    fn collages_varies_donnent_le_meme_code() {
        let code = tirer().unwrap();
        let brut: String = code.chars().filter(|c| *c != '-').collect();
        let variantes = [
            code.to_lowercase(),
            brut.clone(),
            brut.to_lowercase(),
            format!("  {}\r\n", code.replace('-', " ")),
            code.replace('-', "\u{2011}"),
            format!("{}\n", brut.as_str().chars().collect::<Vec<_>>().chunks(5).map(|g| g.iter().collect::<String>()).collect::<Vec<_>>().join("\n")),
        ];
        for v in variantes {
            assert_eq!(verifier(&v).unwrap(), code, "variante {v:?}");
        }
        // Trop long : coupé à 25 caractères utiles.
        assert_eq!(normaliser(&format!("{brut}ZZZ")), brut);
    }

    #[test]
    fn substitution_toujours_detectee() {
        let corps = "ABCDEFGHJKLMNPQRSTUVWXYZ";
        let code = complet(corps);
        for pos in 0..LONGUEUR {
            for s in ALPHABET.chars() {
                let mut faux: Vec<char> = code.chars().collect();
                if faux[pos] == s {
                    continue;
                }
                faux[pos] = s;
                let faux: String = faux.into_iter().collect();
                assert!(verifier(&faux).is_err(), "{faux}");
            }
        }
    }

    #[test]
    fn inversion_de_voisins_detectee_sauf_a_et_9() {
        let alphabet: Vec<char> = ALPHABET.chars().collect();
        for x in &alphabet {
            for y in &alphabet {
                if x == y || matches!((x, y), ('A', '9') | ('9', 'A')) {
                    continue;
                }
                // La paire placée à chaque position du corps, puis inversée.
                for pos in 0..23 {
                    let mut corps: Vec<char> = "KADENZKADENZKADENZKADENZ".chars().collect();
                    corps[pos] = *x;
                    corps[pos + 1] = *y;
                    let mut code: Vec<char> = complet(&corps.iter().collect::<String>()).chars().collect();
                    code.swap(pos, pos + 1);
                    let inverse: String = code.into_iter().collect();
                    assert!(verifier(&inverse).is_err(), "{inverse}");
                }
            }
        }
    }

    #[test]
    fn messages_avant_le_portail() {
        assert_eq!(verifier("ABCDE-FGH").unwrap_err().modele, MODELES_ERREUR[0]);
        assert_eq!(verifier("ABCDE-FGH").unwrap_err().valeurs, vec!["8".to_string()]);
        assert_eq!(verifier("ABODE").unwrap_err().modele, MODELES_ERREUR[2]);
        let mut code: Vec<char> = complet("ABCDEFGHJKLMNPQRSTUVWXYZ").chars().collect();
        code[3] = if code[3] == 'Z' { 'Y' } else { 'Z' };
        assert_eq!(verifier(&code.into_iter().collect::<String>()).unwrap_err().modele, MODELES_ERREUR[1]);
    }

    fn forme(precedent: &str, nouveau: &str) -> (String, usize, Signal) {
        let m = mettre_en_forme(precedent, nouveau);
        (m.texte, m.curseur, m.signal)
    }

    #[test]
    fn mise_en_forme_et_curseur() {
        use Signal::*;
        // Frappe au fil de l'eau : le tiret paraît avant le 6e caractère.
        assert_eq!(forme("ABCD", "ABCDe"), ("ABCDE".into(), 5, Aucun));
        assert_eq!(forme("ABCDE", "ABCDEf"), ("ABCDE-F".into(), 7, Aucun));
        // Insertion au milieu : le curseur reste après le caractère tapé
        // (X tapé après le C de « ABCDE-FGH »).
        assert_eq!(forme("ABCDE-FGH", "ABCXDE-FGH"), ("ABCXD-EFGH".into(), 4, Aucun));
        // Collage d'un code entier dans un champ vide, en minuscules et espacé.
        let code = tirer().unwrap();
        let colle = format!("{}\r\n", code.to_lowercase().replace('-', " "));
        assert_eq!(forme("", &colle), (code.clone(), LONGUEUR + 4, Aucun));
        // Collage au milieu d'un texte, avec des caractères accentués (UTF-8).
        assert_eq!(forme("ABCDE-FGH", "ABCé€ZZDE-FGH"), ("ABCZZ-DEFGH".into(), 5, Autre));
        // Effacement du tiret : rien ne change, le curseur reste avant lui.
        assert_eq!(forme("ABCDE-FG", "ABCDEFG"), ("ABCDE-FG".into(), 5, Aucun));
        // Effacement d'un caractère au milieu.
        assert_eq!(forme("ABCDE-FG", "ABCE-FG"), ("ABCEF-G".into(), 3, Aucun));
        // Signes ambigus retirés et signalés.
        assert_eq!(forme("ABC", "ABCO"), ("ABC".into(), 3, Ambigu));
        assert_eq!(forme("", "a1b"), ("AB".into(), 2, Ambigu));
        // Texte vide.
        assert_eq!(forme("A", ""), (String::new(), 0, Aucun));
    }
}
