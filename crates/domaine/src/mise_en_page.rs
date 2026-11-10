//! Réglages d'impression (cahier des charges, section 12, jalon J8b) : gabarit,
//! marges, et zones d'en-tête et de pied de page en contenu libre.
//!
//! Une zone se compose comme une signature de mail, dans un markup restreint :
//! - `**gras**` et `*italique*` à l'intérieur d'une ligne ;
//! - une ligne qui commence par `->` est centrée (un `<-` final est toléré) ;
//! - `{nom}`, `{ifu}`... insèrent une donnée de l'entreprise ; `{logo}` et
//!   `{logo2}` sont seuls sur leur ligne ;
//! - `\*`, `\{` et `\\` écrivent le caractère lui-même.
//!
//! Le markup est analysé en lignes structurées ; c'est cette structure qui est
//! stockée et imprimée, jamais le texte saisi tel quel.

use crate::document::TypeDocument;
use crate::message::MessageCompose;

/// Gabarit A4 : choisi, jamais redessiné champ par champ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Gabarit {
    /// En-tête et pied de page complets, tableau sur toute la largeur.
    #[default]
    Classique,
    /// Texte et interlignes resserrés.
    Compact,
    /// Papier déjà imprimé : ni logo, ni en-tête, ni pied de page.
    PreImprime,
}

impl Gabarit {
    pub const TOUS: [Gabarit; 3] = [Gabarit::Classique, Gabarit::Compact, Gabarit::PreImprime];

    pub fn code(self) -> &'static str {
        match self {
            Gabarit::Classique => "classique",
            Gabarit::Compact => "compact",
            Gabarit::PreImprime => "pre_imprime",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Gabarit> {
        Gabarit::TOUS.into_iter().find(|g| g.code() == code)
    }

    /// Le gabarit imprime-t-il les zones d'en-tête et de pied de page ?
    pub fn imprime_les_zones(self) -> bool {
        self != Gabarit::PreImprime
    }
}

/// Marges de la page en millimètres.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Marges {
    pub haut: u8,
    pub bas: u8,
    pub gauche: u8,
    pub droite: u8,
}

impl Marges {
    /// Marge maximale de chaque côté.
    pub const MAX: u8 = 40;

    pub fn verifier(&self) -> Result<(), MessageCompose> {
        let trop = [self.haut, self.bas, self.gauche, self.droite]
            .into_iter()
            .any(|m| m > Marges::MAX);
        if trop {
            return Err(MessageCompose::new(
                "chaque marge doit être comprise entre 0 et {} mm",
                &[&Marges::MAX],
            ));
        }
        Ok(())
    }
}

/// Marges de la mise en page d'origine (12 mm en haut, 9 en bas, 13 sur les côtés).
impl Default for Marges {
    fn default() -> Self {
        Marges {
            haut: 12,
            bas: 9,
            gauche: 13,
            droite: 13,
        }
    }
}

/// Donnée insérable dans une zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Variable {
    Nom,
    Adresse,
    Telephone,
    Email,
    Site,
    Localisation,
    Regime,
    Rccm,
    Ifu,
    Banque1,
    Banque2,
    Logo,
    Logo2,
}

impl Variable {
    pub const TOUTES: [Variable; 13] = [
        Variable::Nom,
        Variable::Adresse,
        Variable::Telephone,
        Variable::Email,
        Variable::Site,
        Variable::Localisation,
        Variable::Regime,
        Variable::Rccm,
        Variable::Ifu,
        Variable::Banque1,
        Variable::Banque2,
        Variable::Logo,
        Variable::Logo2,
    ];

    /// Nom écrit entre accolades dans le markup.
    pub fn nom(self) -> &'static str {
        match self {
            Variable::Nom => "nom",
            Variable::Adresse => "adresse",
            Variable::Telephone => "telephone",
            Variable::Email => "email",
            Variable::Site => "site",
            Variable::Localisation => "localisation",
            Variable::Regime => "regime",
            Variable::Rccm => "rccm",
            Variable::Ifu => "ifu",
            Variable::Banque1 => "banque1",
            Variable::Banque2 => "banque2",
            Variable::Logo => "logo",
            Variable::Logo2 => "logo2",
        }
    }

    /// Libellé de la liste déroulante (à traduire par l'interface).
    pub fn libelle(self) -> &'static str {
        match self {
            Variable::Nom => "Nom de l'entreprise",
            Variable::Adresse => "Adresse",
            Variable::Telephone => "Téléphone",
            Variable::Email => "Email",
            Variable::Site => "Site web",
            Variable::Localisation => "Localisation",
            Variable::Regime => "Régime d'imposition",
            Variable::Rccm => "RCCM",
            Variable::Ifu => "IFU",
            Variable::Banque1 => "Banque 1",
            Variable::Banque2 => "Banque 2",
            Variable::Logo => "Logo principal",
            Variable::Logo2 => "Logo secondaire",
        }
    }

    pub fn depuis_nom(nom: &str) -> Option<Variable> {
        Variable::TOUTES.into_iter().find(|v| v.nom() == nom)
    }

    pub fn est_un_logo(self) -> bool {
        matches!(self, Variable::Logo | Variable::Logo2)
    }
}

/// Contenu d'un passage : texte littéral ou donnée à insérer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Contenu {
    Texte(String),
    Variable(Variable),
}

/// Passage d'une ligne, avec son style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Passage {
    pub contenu: Contenu,
    pub gras: bool,
    pub italique: bool,
}

/// Ligne d'une zone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ligne {
    /// Ligne de texte ; sans passage, c'est une ligne vide (un espacement).
    Texte {
        centree: bool,
        passages: Vec<Passage>,
    },
    /// Logo seul sur sa ligne (`Variable::Logo` ou `Variable::Logo2`).
    Logo { logo: Variable, centree: bool },
}

/// Zone d'en-tête ou de pied de page analysée.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Zone {
    pub lignes: Vec<Ligne>,
}

/// Passage résolu, prêt à imprimer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassageResolu {
    pub texte: String,
    pub gras: bool,
    pub italique: bool,
}

/// Ligne résolue, prête à imprimer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LigneResolue {
    Texte {
        centree: bool,
        passages: Vec<PassageResolu>,
    },
    Logo {
        logo: Variable,
        centree: bool,
    },
}

const PREFIXE_CENTRE: &str = "->";
const SUFFIXE_CENTRE: &str = "<-";

fn erreur(modele: &'static str, numero: usize) -> MessageCompose {
    MessageCompose::new(modele, &[&numero])
}

/// Ajoute du texte au dernier passage s'il a le même style, sinon en crée un.
fn ajouter_texte(passages: &mut Vec<Passage>, texte: &str, gras: bool, italique: bool) {
    if texte.is_empty() {
        return;
    }
    if let Some(Passage {
        contenu: Contenu::Texte(t),
        gras: g,
        italique: i,
    }) = passages.last_mut()
        && *g == gras
        && *i == italique
    {
        t.push_str(texte);
        return;
    }
    passages.push(Passage {
        contenu: Contenu::Texte(texte.to_owned()),
        gras,
        italique,
    });
}

fn analyser_ligne(source: &str, numero: usize) -> Result<Ligne, MessageCompose> {
    let mut texte = source.trim();
    let centree = texte.starts_with(PREFIXE_CENTRE);
    if centree {
        texte = texte[PREFIXE_CENTRE.len()..].trim();
        if let Some(sans) = texte.strip_suffix(SUFFIXE_CENTRE) {
            texte = sans.trim_end();
        }
    }
    for logo in [Variable::Logo, Variable::Logo2] {
        if texte == format!("{{{}}}", logo.nom()) {
            return Ok(Ligne::Logo { logo, centree });
        }
    }
    let mut passages = Vec::new();
    let (mut gras, mut italique) = (false, false);
    let mut tampon = String::new();
    let mut caracteres = texte.chars().peekable();
    while let Some(c) = caracteres.next() {
        match c {
            '\\' => match caracteres.next() {
                Some(suivant) => tampon.push(suivant),
                None => tampon.push('\\'),
            },
            '*' => {
                ajouter_texte(&mut passages, &tampon, gras, italique);
                tampon.clear();
                if caracteres.peek() == Some(&'*') {
                    caracteres.next();
                    gras = !gras;
                } else {
                    italique = !italique;
                }
            }
            '{' => {
                let mut nom = String::new();
                let mut fermee = false;
                for c in caracteres.by_ref() {
                    if c == '}' {
                        fermee = true;
                        break;
                    }
                    nom.push(c);
                }
                if !fermee {
                    return Err(erreur("ligne {} : accolade « { » non fermée", numero));
                }
                let variable = Variable::depuis_nom(nom.trim()).ok_or_else(|| {
                    MessageCompose::new(
                        "ligne {} : la variable « {} » n'existe pas",
                        &[&numero, &nom.trim()],
                    )
                })?;
                if variable.est_un_logo() {
                    return Err(erreur(
                        "ligne {} : un logo doit être seul sur sa ligne",
                        numero,
                    ));
                }
                ajouter_texte(&mut passages, &tampon, gras, italique);
                tampon.clear();
                passages.push(Passage {
                    contenu: Contenu::Variable(variable),
                    gras,
                    italique,
                });
            }
            _ => tampon.push(c),
        }
    }
    ajouter_texte(&mut passages, &tampon, gras, italique);
    if gras {
        return Err(erreur("ligne {} : gras non fermé (**)", numero));
    }
    if italique {
        return Err(erreur("ligne {} : italique non fermée (*)", numero));
    }
    Ok(Ligne::Texte { centree, passages })
}

/// Analyse le markup d'une zone. Les lignes vides de fin sont ignorées.
pub fn analyser(markup: &str) -> Result<Zone, MessageCompose> {
    let mut lignes = markup
        .lines()
        .enumerate()
        .map(|(i, l)| analyser_ligne(l, i + 1))
        .collect::<Result<Vec<_>, _>>()?;
    while matches!(lignes.last(), Some(Ligne::Texte { passages, .. }) if passages.is_empty()) {
        lignes.pop();
    }
    while matches!(lignes.first(), Some(Ligne::Texte { passages, .. }) if passages.is_empty()) {
        lignes.remove(0);
    }
    Ok(Zone { lignes })
}

fn echapper(texte: &str) -> String {
    let mut s = String::with_capacity(texte.len());
    for c in texte.chars() {
        if matches!(c, '*' | '{' | '\\') {
            s.push('\\');
        }
        s.push(c);
    }
    s
}

impl Zone {
    pub fn est_vide(&self) -> bool {
        self.lignes.is_empty()
    }

    /// Markup équivalent, pour l'éditeur : `analyser(&z.en_markup()) == Ok(z)`.
    pub fn en_markup(&self) -> String {
        let mut lignes = Vec::with_capacity(self.lignes.len());
        for ligne in &self.lignes {
            let mut s = String::new();
            match ligne {
                Ligne::Logo { logo, centree } => {
                    if *centree {
                        s.push_str("-> ");
                    }
                    s.push_str(&format!("{{{}}}", logo.nom()));
                }
                Ligne::Texte { centree, passages } => {
                    if *centree {
                        s.push_str("-> ");
                    }
                    let (mut gras, mut italique) = (false, false);
                    for p in passages {
                        // Jamais « * ** * » à la suite : l'analyse lirait « **** », deux
                        // bascules du gras. L'italique ne se ferme que s'il s'arrête ;
                        // « *** » reste sans ambiguïté, les deux bascules commutent.
                        if italique && !p.italique {
                            s.push('*');
                            italique = false;
                        }
                        if gras != p.gras {
                            s.push_str("**");
                            gras = p.gras;
                        }
                        if p.italique && !italique {
                            s.push('*');
                            italique = true;
                        }
                        match &p.contenu {
                            Contenu::Texte(t) => s.push_str(&echapper(t)),
                            Contenu::Variable(v) => s.push_str(&format!("{{{}}}", v.nom())),
                        }
                    }
                    if italique {
                        s.push('*');
                    }
                    if gras {
                        s.push_str("**");
                    }
                }
            }
            lignes.push(s);
        }
        lignes.join("\n")
    }

    /// Remplace les variables par leurs valeurs. Une ligne qui contient des
    /// variables toutes vides disparaît (« Tél. : {telephone} » sans téléphone).
    /// Les logos sont gardés : l'imprimeur saute un logo absent.
    pub fn resoudre(&self, valeur: impl Fn(Variable) -> String) -> Vec<LigneResolue> {
        let mut resultat = Vec::with_capacity(self.lignes.len());
        for ligne in &self.lignes {
            match ligne {
                Ligne::Logo { logo, centree } => resultat.push(LigneResolue::Logo {
                    logo: *logo,
                    centree: *centree,
                }),
                Ligne::Texte { centree, passages } => {
                    if let Some(passages) = resoudre_ligne(passages, &valeur) {
                        resultat.push(LigneResolue::Texte {
                            centree: *centree,
                            passages,
                        });
                    }
                }
            }
        }
        resultat
    }
}

/// Séparateur des mentions d'une ligne (« Tél. : {telephone} · {email} »).
pub const SEPARATEUR: char = '·';

#[derive(Default)]
struct Segment {
    /// Style du séparateur qui ouvre le segment (aucun pour le premier).
    separateur: Option<(bool, bool)>,
    passages: Vec<PassageResolu>,
    variables: usize,
    remplies: usize,
}

/// Une ligne de texte résolue, ou rien si toutes ses variables sont vides. Une
/// mention (segment entre deux « · ») dont toutes les variables sont vides tombe
/// avec son libellé et son séparateur.
fn resoudre_ligne(
    passages: &[Passage],
    valeur: &impl Fn(Variable) -> String,
) -> Option<Vec<PassageResolu>> {
    let mut segments = vec![Segment::default()];
    let pousser = |segments: &mut Vec<Segment>, texte: &str, p: &Passage| {
        if !texte.is_empty() {
            let courant = segments.last_mut().expect("au moins un segment");
            courant.passages.push(PassageResolu {
                texte: texte.to_owned(),
                gras: p.gras,
                italique: p.italique,
            });
        }
    };
    for p in passages {
        match &p.contenu {
            Contenu::Variable(v) => {
                let s = valeur(*v).trim().to_owned();
                let courant = segments.last_mut().expect("au moins un segment");
                courant.variables += 1;
                if !s.is_empty() {
                    courant.remplies += 1;
                }
                pousser(&mut segments, &s, p);
            }
            Contenu::Texte(t) => {
                for (k, morceau) in t.split(SEPARATEUR).enumerate() {
                    if k > 0 {
                        segments.push(Segment {
                            separateur: Some((p.gras, p.italique)),
                            ..Segment::default()
                        });
                    }
                    pousser(&mut segments, morceau, p);
                }
            }
        }
    }
    let variables: usize = segments.iter().map(|s| s.variables).sum();
    let remplies: usize = segments.iter().map(|s| s.remplies).sum();
    if variables > 0 && remplies == 0 {
        return None;
    }
    let total = segments.len();
    let gardes: Vec<Segment> = segments
        .into_iter()
        .filter(|s| s.variables == 0 || s.remplies > 0)
        .collect();
    let elague = gardes.len() < total;
    let mut resultat = Vec::new();
    for (k, s) in gardes.into_iter().enumerate() {
        if let (true, Some((gras, italique))) = (k > 0, s.separateur) {
            resultat.push(PassageResolu {
                texte: SEPARATEUR.to_string(),
                gras,
                italique,
            });
        }
        resultat.extend(s.passages);
    }
    if elague {
        if let Some(p) = resultat.first_mut() {
            p.texte = p.texte.trim_start().to_owned();
        }
        if let Some(p) = resultat.last_mut() {
            p.texte = p.texte.trim_end().to_owned();
        }
        resultat.retain(|p| !p.texte.is_empty());
    }
    Some(resultat)
}

/// Markup de l'en-tête par défaut : le bloc entreprise de la mise en page
/// d'origine (logo en première ligne à gauche du texte, logo en dernière ligne
/// à droite au-dessus du titre). Il sert aussi aux instantanés antérieurs à
/// J8b : le modifier change la réimpression de ces documents.
pub const ENTETE_PAR_DEFAUT: &str =
    "{logo}\n**{nom}**\n{adresse}\nTél. : {telephone}\n{email}\n{site}\n{logo2}";

/// Markup du pied de page par défaut : les mentions de la mise en page d'origine
/// (même remarque que pour l'en-tête).
pub const PIED_PAR_DEFAUT: &str = "-> {adresse} · Tél. : {telephone} · {email} · {site}\n-> RCCM : {rccm} · IFU : {ifu} · Régime : {regime}\n-> {banque1} · {banque2}";

/// Réglages d'impression : entête et pied en markup, tels que l'éditeur les montre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MiseEnPage {
    pub gabarit: Gabarit,
    pub marges: Marges,
    pub entete: String,
    pub pied: String,
    pub style_pied: StylePied,
}

impl Default for MiseEnPage {
    fn default() -> Self {
        MiseEnPage {
            gabarit: Gabarit::default(),
            marges: Marges::default(),
            entete: ENTETE_PAR_DEFAUT.to_owned(),
            pied: PIED_PAR_DEFAUT.to_owned(),
            style_pied: StylePied::default(),
        }
    }
}

/// Mise en page analysée, prête à imprimer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MiseEnPageAnalysee {
    pub gabarit: Gabarit,
    pub marges: Marges,
    pub entete: Zone,
    pub pied: Zone,
    pub style_pied: StylePied,
}

impl Default for MiseEnPageAnalysee {
    /// La mise en page d'origine.
    fn default() -> Self {
        MiseEnPage::default()
            .analyser()
            .expect("les réglages par défaut s'analysent")
    }
}

impl MiseEnPage {
    /// Analyse et vérifie les réglages ; le message nomme la zone fautive.
    pub fn analyser(&self) -> Result<MiseEnPageAnalysee, MessageCompose> {
        self.marges.verifier()?;
        let entete = analyser(&self.entete).map_err(|e| prefixer("en-tête", e))?;
        let pied = analyser(&self.pied).map_err(|e| prefixer("pied de page", e))?;
        Ok(MiseEnPageAnalysee {
            gabarit: self.gabarit,
            marges: self.marges,
            entete,
            pied,
            style_pied: self.style_pied,
        })
    }
}

/// Style du trait et des mentions du pied de page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StylePied {
    /// Trait fin gris, mentions en gris.
    #[default]
    Discret,
    /// Trait épais de la couleur principale, mentions en gras.
    Accentue,
}

impl StylePied {
    pub const TOUS: [StylePied; 2] = [StylePied::Discret, StylePied::Accentue];

    pub fn code(self) -> &'static str {
        match self {
            StylePied::Discret => "discret",
            StylePied::Accentue => "accentue",
        }
    }

    pub fn depuis_code(code: &str) -> Option<StylePied> {
        StylePied::TOUS.into_iter().find(|s| s.code() == code)
    }
}

/// Ligne du tableau des totaux.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LigneTotal {
    Ht,
    Remise,
    BaseTva,
    Tva,
    Retenue,
    Ttc,
}

impl LigneTotal {
    /// Dans l'ordre d'impression.
    pub const TOUTES: [LigneTotal; 6] = [
        LigneTotal::Ht,
        LigneTotal::Remise,
        LigneTotal::BaseTva,
        LigneTotal::Tva,
        LigneTotal::Retenue,
        LigneTotal::Ttc,
    ];

    pub fn code(self) -> &'static str {
        match self {
            LigneTotal::Ht => "ht",
            LigneTotal::Remise => "remise",
            LigneTotal::BaseTva => "base_tva",
            LigneTotal::Tva => "tva",
            LigneTotal::Retenue => "retenue",
            LigneTotal::Ttc => "ttc",
        }
    }

    pub fn depuis_code(code: &str) -> Option<LigneTotal> {
        LigneTotal::TOUTES.into_iter().find(|l| l.code() == code)
    }

    /// Libellé imprimé (sans le taux, ajouté par le PDF).
    pub fn libelle(self) -> &'static str {
        match self {
            LigneTotal::Ht => "Montant HT",
            LigneTotal::Remise => "Remise",
            LigneTotal::BaseTva => "Base TVA",
            LigneTotal::Tva => "TVA",
            LigneTotal::Retenue => "Retenue",
            LigneTotal::Ttc => "Montant TTC",
        }
    }
}

/// Éléments imprimés au bas d'un document, réglés par type de document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasDeDocument {
    /// Lignes de totaux cochées, dans l'ordre de `LigneTotal::TOUTES`.
    pub totaux: Vec<LigneTotal>,
    /// Masque les lignes de totaux à zéro, et la base TVA quand elle vaut le
    /// montant HT (aucune remise).
    pub masquer_zero: bool,
    pub montant_en_lettres: bool,
    pub visa_client: bool,
    pub visa_fournisseur: bool,
    pub date_reception: bool,
    pub date_livraison: bool,
    /// Libellé du signataire et nom.
    pub signataire: bool,
}

impl BasDeDocument {
    /// Réglage d'origine d'un type : totaux, montant en lettres et signataire
    /// pour un document à montants ; visas, dates et signataire pour un
    /// bordereau (modèle Excel du bordereau).
    pub fn par_defaut(t: TypeDocument) -> BasDeDocument {
        if t.a_des_montants() {
            BasDeDocument {
                totaux: LigneTotal::TOUTES.to_vec(),
                masquer_zero: true,
                montant_en_lettres: true,
                visa_client: false,
                visa_fournisseur: false,
                date_reception: false,
                date_livraison: false,
                signataire: true,
            }
        } else {
            BasDeDocument {
                totaux: Vec::new(),
                masquer_zero: true,
                montant_en_lettres: false,
                visa_client: true,
                visa_fournisseur: true,
                date_reception: true,
                date_livraison: true,
                signataire: true,
            }
        }
    }

    pub fn imprime(&self, l: LigneTotal) -> bool {
        self.totaux.contains(&l)
    }

    /// Remet les lignes cochées dans l'ordre d'impression, sans doublon.
    pub fn normaliser(&mut self) {
        let cochees = std::mem::take(&mut self.totaux);
        self.totaux = LigneTotal::TOUTES
            .into_iter()
            .filter(|l| cochees.contains(l))
            .collect();
    }
}

impl Default for BasDeDocument {
    fn default() -> Self {
        BasDeDocument::par_defaut(TypeDocument::Facture)
    }
}

/// Tous les modèles d'erreur de ce module, pour vérifier leurs traductions.
pub const MODELES_ERREUR: &[&str] = &[
    "chaque marge doit être comprise entre 0 et {} mm",
    "ligne {} : accolade « { » non fermée",
    "ligne {} : la variable « {} » n'existe pas",
    "ligne {} : un logo doit être seul sur sa ligne",
    "ligne {} : gras non fermé (**)",
    "ligne {} : italique non fermée (*)",
    "en-tête, ligne {} : accolade « { » non fermée",
    "en-tête, ligne {} : la variable « {} » n'existe pas",
    "en-tête, ligne {} : un logo doit être seul sur sa ligne",
    "en-tête, ligne {} : gras non fermé (**)",
    "en-tête, ligne {} : italique non fermée (*)",
    "pied de page, ligne {} : accolade « { » non fermée",
    "pied de page, ligne {} : la variable « {} » n'existe pas",
    "pied de page, ligne {} : un logo doit être seul sur sa ligne",
    "pied de page, ligne {} : gras non fermé (**)",
    "pied de page, ligne {} : italique non fermée (*)",
];

/// Le modèle reste une phrase complète : la zone est un modèle à part.
fn prefixer(zone: &'static str, e: MessageCompose) -> MessageCompose {
    let modele = match (zone, e.modele) {
        ("en-tête", "ligne {} : accolade « { » non fermée") => {
            "en-tête, ligne {} : accolade « { » non fermée"
        }
        ("en-tête", "ligne {} : la variable « {} » n'existe pas") => {
            "en-tête, ligne {} : la variable « {} » n'existe pas"
        }
        ("en-tête", "ligne {} : un logo doit être seul sur sa ligne") => {
            "en-tête, ligne {} : un logo doit être seul sur sa ligne"
        }
        ("en-tête", "ligne {} : gras non fermé (**)") => {
            "en-tête, ligne {} : gras non fermé (**)"
        }
        ("en-tête", "ligne {} : italique non fermée (*)") => {
            "en-tête, ligne {} : italique non fermée (*)"
        }
        (_, "ligne {} : accolade « { » non fermée") => {
            "pied de page, ligne {} : accolade « { » non fermée"
        }
        (_, "ligne {} : la variable « {} » n'existe pas") => {
            "pied de page, ligne {} : la variable « {} » n'existe pas"
        }
        (_, "ligne {} : un logo doit être seul sur sa ligne") => {
            "pied de page, ligne {} : un logo doit être seul sur sa ligne"
        }
        (_, "ligne {} : gras non fermé (**)") => "pied de page, ligne {} : gras non fermé (**)",
        (_, "ligne {} : italique non fermée (*)") => {
            "pied de page, ligne {} : italique non fermée (*)"
        }
        (_, autre) => autre,
    };
    MessageCompose {
        modele,
        valeurs: e.valeurs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texte(t: &str, gras: bool, italique: bool) -> Passage {
        Passage {
            contenu: Contenu::Texte(t.into()),
            gras,
            italique,
        }
    }

    #[test]
    fn analyse_du_markup() {
        let z = analyser(
            "\n{logo}\n-> **{nom}** *SARL* <-\nTél. : {telephone}\n\nPrix \\* 2 \\{x}\n\n",
        )
        .unwrap();
        assert_eq!(
            z.lignes.len(),
            5,
            "lignes vides du début et de la fin ignorées"
        );
        assert_eq!(
            z.lignes[0],
            Ligne::Logo {
                logo: Variable::Logo,
                centree: false
            }
        );
        assert_eq!(
            z.lignes[1],
            Ligne::Texte {
                centree: true,
                passages: vec![
                    Passage {
                        contenu: Contenu::Variable(Variable::Nom),
                        gras: true,
                        italique: false
                    },
                    texte(" ", false, false),
                    texte("SARL", false, true),
                ]
            }
        );
        assert_eq!(
            z.lignes[3],
            Ligne::Texte {
                centree: false,
                passages: vec![]
            }
        );
        assert_eq!(
            z.lignes[4],
            Ligne::Texte {
                centree: false,
                passages: vec![texte("Prix * 2 {x}", false, false)]
            }
        );
    }

    #[test]
    fn erreurs_du_markup() {
        let e = |m: &str| analyser(m).unwrap_err().to_string();
        assert_eq!(e("ok\n**gras"), "ligne 2 : gras non fermé (**)");
        assert_eq!(e("*penché"), "ligne 1 : italique non fermée (*)");
        assert_eq!(
            e("{inconnue}"),
            "ligne 1 : la variable « inconnue » n'existe pas"
        );
        assert_eq!(e("{nom"), "ligne 1 : accolade « { » non fermée");
        assert_eq!(
            e("Logo : {logo}"),
            "ligne 1 : un logo doit être seul sur sa ligne"
        );
        let m = MiseEnPage {
            pied: "{ifu".into(),
            ..MiseEnPage::default()
        };
        assert_eq!(
            m.analyser().unwrap_err().to_string(),
            "pied de page, ligne 1 : accolade « { » non fermée"
        );
        let m = MiseEnPage {
            marges: Marges {
                haut: 41,
                ..Marges::default()
            },
            ..MiseEnPage::default()
        };
        assert!(m.analyser().is_err());
    }

    #[test]
    fn aller_retour_du_markup() {
        for m in [
            ENTETE_PAR_DEFAUT,
            PIED_PAR_DEFAUT,
            "-> {logo2}\n***gras penché*** et *penché **puis gras** encore*\nÉchappés : \\* \\{ \\\\",
        ] {
            let z = analyser(m).unwrap();
            assert_eq!(analyser(&z.en_markup()).unwrap(), z, "{m}");
        }
    }

    #[test]
    fn resolution_et_lignes_vides() {
        let z = analyser("**{nom}**\nTél. : {telephone}\n\n{rccm} {ifu}\n{logo}").unwrap();
        let r = z.resoudre(|v| match v {
            Variable::Nom => "Entreprise Essai".into(),
            Variable::Ifu => "IFU-1".into(),
            _ => String::new(),
        });
        assert_eq!(r.len(), 4, "la ligne du téléphone vide disparaît");
        assert_eq!(
            r[0],
            LigneResolue::Texte {
                centree: false,
                passages: vec![PassageResolu {
                    texte: "Entreprise Essai".into(),
                    gras: true,
                    italique: false
                }]
            }
        );
        assert_eq!(
            r[1],
            LigneResolue::Texte {
                centree: false,
                passages: vec![]
            },
            "ligne vide d'espacement gardée"
        );
        assert!(matches!(&r[2], LigneResolue::Texte { passages, .. } if passages.len() == 2));
        assert!(matches!(r[3], LigneResolue::Logo { .. }));
    }

    #[test]
    fn mention_vide_tombe_avec_son_separateur() {
        let texte = |z: &str, remplis: &[Variable]| -> Vec<String> {
            analyser(z)
                .unwrap()
                .resoudre(|v| {
                    if remplis.contains(&v) {
                        format!("<{}>", v.nom())
                    } else {
                        String::new()
                    }
                })
                .into_iter()
                .map(|l| match l {
                    LigneResolue::Texte { passages, .. } => {
                        passages.into_iter().map(|p| p.texte).collect::<String>()
                    }
                    LigneResolue::Logo { .. } => "[logo]".into(),
                })
                .collect()
        };
        let ligne = "RCCM : {rccm} · IFU : {ifu} · Régime : {regime}";
        assert_eq!(texte(ligne, &[Variable::Ifu]), ["IFU : <ifu>"]);
        assert_eq!(
            texte(ligne, &[Variable::Rccm, Variable::Regime]),
            ["RCCM : <rccm> · Régime : <regime>"]
        );
        assert_eq!(texte(ligne, &[Variable::Rccm]), ["RCCM : <rccm>"]);
        assert!(texte(ligne, &[]).is_empty());
        assert_eq!(texte("Merci · à bientôt", &[]), ["Merci · à bientôt"]);
        assert_eq!(texte("Merci · {site}", &[]), Vec::<String>::new());
        assert_eq!(
            texte("**Merci** · {site}", &[Variable::Site]),
            ["Merci · <site>"]
        );
        let pied = texte(PIED_PAR_DEFAUT, &[Variable::Banque2, Variable::Email]);
        assert_eq!(pied, ["<email>", "<banque2>"]);
        let entete = texte(ENTETE_PAR_DEFAUT, &[Variable::Nom]);
        assert_eq!(entete, ["[logo]", "<nom>", "[logo]"]);
    }

    #[test]
    fn bas_par_defaut_et_normalisation() {
        let f = BasDeDocument::par_defaut(TypeDocument::Facture);
        assert!(f.imprime(LigneTotal::Ttc) && f.signataire && !f.visa_client);
        let b = BasDeDocument::par_defaut(TypeDocument::Livraison);
        assert!(b.totaux.is_empty() && b.visa_client && b.date_livraison);
        let mut m = BasDeDocument {
            totaux: vec![LigneTotal::Ttc, LigneTotal::Ht, LigneTotal::Ttc],
            ..f
        };
        m.normaliser();
        assert_eq!(m.totaux, [LigneTotal::Ht, LigneTotal::Ttc]);
        for l in LigneTotal::TOUTES {
            assert_eq!(LigneTotal::depuis_code(l.code()), Some(l));
        }
        for s in StylePied::TOUS {
            assert_eq!(StylePied::depuis_code(s.code()), Some(s));
        }
    }

    #[test]
    fn codes_et_noms() {
        for g in Gabarit::TOUS {
            assert_eq!(Gabarit::depuis_code(g.code()), Some(g));
        }
        for v in Variable::TOUTES {
            assert_eq!(Variable::depuis_nom(v.nom()), Some(v));
        }
        assert!(!Gabarit::PreImprime.imprime_les_zones());
    }

    #[test]
    fn chaque_erreur_produite_est_listee() {
        let fautifs = ["{nom", "{date}", "a {logo}", "**a", "*a"];
        let mut vus = Vec::new();
        for f in fautifs {
            vus.push(analyser(f).unwrap_err().modele);
            for (entete, pied) in [(f, ""), ("", f)] {
                let m = MiseEnPage {
                    entete: entete.into(),
                    pied: pied.into(),
                    ..MiseEnPage::default()
                };
                vus.push(m.analyser().unwrap_err().modele);
            }
        }
        let m = MiseEnPage {
            marges: Marges {
                haut: Marges::MAX + 1,
                ..Marges::default()
            },
            ..MiseEnPage::default()
        };
        vus.push(m.analyser().unwrap_err().modele);
        for v in &vus {
            assert!(MODELES_ERREUR.contains(v), "{v}");
        }
        for m in MODELES_ERREUR {
            assert!(vus.contains(m), "{m}");
        }
    }
}
