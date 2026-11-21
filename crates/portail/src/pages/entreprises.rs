//! Liste des entreprises (GET /) : toutes pour l'éditeur, celles qui lui sont
//! rattachées pour un revendeur (filtre fait par le crate).
//!
//! Page au gabarit Entreprises de la maquette (copie fidèle décidée par
//! l'utilisateur le 2026-11-20, « utilise complètement le template
//! Entreprises » le 2026-11-21) : diaporama des entreprises, phrase qui
//! tourne, grille de cartes en onglets, code d'activation, règles du portail,
//! bandeau du journal, puis la liste complète. Les gabarits à trous viennent
//! de `gabarit` (généré) ; tout ce qui les remplit vient des vraies données,
//! et une liste vide donne un état vide, jamais un exemple.

use maud::{Markup, PreEscaped, html};

use super::journal::libelle_action;
use super::{Cadre, Rubrique, chemin_entreprise, date, gabarit as g, heure};
use crate::{Action, EtatChaine, Role, VueEntreprise};

/// Entreprises montrées dans le diaporama du haut.
const DIAPOS: usize = 4;
/// Entrées du journal dans la carte des actualités et dans le bandeau.
const ACTUALITES: usize = 3;
const BOITES: usize = 5;

const FORMES_HERO: [&str; 3] = ["fronton", "chapiteau", "arche"];
const FORMES_P: [&str; 7] = [
    "fronton",
    "colonne",
    "arche",
    "amphore",
    "escalier",
    "chapiteau",
    "ruine",
];
const PIECES: [&str; 4] = ["carre", "quart", "tri", "disque"];

pub fn entreprises(
    c: &Cadre,
    liste: &[VueEntreprise],
    journal: &[Action],
    chaine: Option<&EtatChaine>,
) -> Markup {
    let editeur = c.compte.role == Role::Editeur;
    let mut sections = String::new();
    sections += &hero(liste, journal);
    sections += &remplir(
        g::INTRO,
        &[
            (
                "debut",
                if editeur {
                    g::INTRO_DEBUT_EDITEUR
                } else {
                    g::INTRO_DEBUT_REVENDEUR
                },
            ),
            ("nom", &e(&c.compte.nom)),
            ("role", if editeur { "Éditeur" } else { "Revendeur" }),
        ],
    );
    sections += &grille(liste);
    sections += g::DERIVE;
    sections += g::DETAILS;
    sections += &bandeau_du_journal(journal, chaine);
    let contenu = html! {
        (PreEscaped(sections))
        (liste_complete(editeur, liste))
    };
    super::page_pleine(c, "Entreprises", Rubrique::Entreprises, contenu)
}

/// Remplit les trous `⟦nom⟧` d'un gabarit ; les valeurs sont déjà échappées.
fn remplir(modele: &str, valeurs: &[(&str, &str)]) -> String {
    let mut sortie = modele.to_string();
    for (nom, valeur) in valeurs {
        sortie = sortie.replace(&format!("⟦{nom}⟧"), valeur);
    }
    debug_assert!(!sortie.contains('⟦'), "trou non rempli : {sortie}");
    sortie
}

/// Texte échappé pour le HTML (contenu ou valeur d'attribut).
fn e(texte: &str) -> String {
    html! { (texte) }.into_string()
}

/// Nombre court, à la façon de la maquette : « 0 », « 2.5 », « 0.7 ».
fn nombre(x: f64) -> String {
    let s = format!("{x:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Attributs d'un motif (toile des motifs Kadenz).
fn toile(motif: &str, cadre: &str, piece: &str, forme: Option<&str>, decalage: f64) -> String {
    match forme {
        Some(f) => format!(
            r#"data-motif="{motif}" data-cadre="{cadre}" data-piece="{piece}" data-forme="{f}" data-decalage="{}""#,
            nombre(decalage)
        ),
        None => format!(
            r#"data-motif="{motif}" data-cadre="{cadre}" data-piece="{piece}" data-decalage="{}""#,
            nombre(decalage)
        ),
    }
}

fn libelle_revendeur(v: &VueEntreprise) -> &str {
    match &v.entreprise.revendeur {
        Some((_, nom)) => nom,
        None => "Client direct",
    }
}

fn paiements_a_utiliser(n: usize) -> String {
    match n {
        0 => "Aucun paiement à utiliser".to_string(),
        1 => "1 paiement à utiliser".to_string(),
        n => format!("{n} paiements à utiliser"),
    }
}

/// Titre d'une entrée du journal : son libellé, suivi du client visé quand
/// la cible est une entreprise de la liste.
fn titre_action(a: &Action, liste: &[VueEntreprise]) -> String {
    let libelle = libelle_action(&a.action);
    match a
        .cible
        .as_deref()
        .and_then(|cible| liste.iter().find(|v| v.entreprise.identifiant == cible))
    {
        Some(v) => format!("{libelle}, {}", v.entreprise.client),
        None => libelle.to_string(),
    }
}

/// « 19 nov. 2026 » et « 19 nov. » depuis `AAAA-MM-JJ HH:MM:SS`.
fn jour_court(heure_utc: &str) -> (String, String) {
    const MOIS: [&str; 12] = [
        "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.",
        "déc.",
    ];
    let annee = heure_utc.get(0..4);
    let mois = heure_utc.get(5..7).and_then(|m| m.parse::<usize>().ok());
    let jour = heure_utc.get(8..10).and_then(|j| j.parse::<u32>().ok());
    match (annee, mois, jour) {
        (Some(a), Some(m @ 1..=12), Some(j)) => {
            let court = format!("{j} {}", MOIS[m - 1]);
            (format!("{court} {a}"), court)
        }
        _ => (heure_utc.to_string(), heure_utc.to_string()),
    }
}

fn hero(liste: &[VueEntreprise], journal: &[Action]) -> String {
    // Diaporama : les premières entreprises, trois pierres puis un C Tournant ;
    // sans entreprise, une seule pierre qui mène à la création.
    let mut legendes: Vec<(String, String)> = liste
        .iter()
        .take(DIAPOS)
        .map(|v| {
            (
                v.entreprise.client.clone(),
                chemin_entreprise(&v.entreprise.identifiant),
            )
        })
        .collect();
    if legendes.is_empty() {
        legendes.push((
            "Aucune entreprise pour l'instant".to_string(),
            "/entreprises/nouvelle".to_string(),
        ));
    }
    let n = legendes.len();
    let mut diapos = String::new();
    let mut vignettes = String::new();
    for (k, (legende, lien)) in legendes.iter().enumerate() {
        let decalage = k as f64 * 2.5;
        let (plein, petit) = if k < FORMES_HERO.len() {
            (
                toile("P", "plein", "quart", Some(FORMES_HERO[k]), decalage),
                toile("P", "vignette", "quart", Some(FORMES_HERO[k]), decalage),
            )
        } else {
            (
                toile("C", "plein", "quart", None, decalage),
                toile("C", "vignette", "quart", None, decalage),
            )
        };
        let modele = if k == 0 {
            g::DIAPO_PREMIERE
        } else if k == n - 1 {
            g::DIAPO_DERNIERE
        } else {
            g::DIAPO
        };
        diapos += &remplir(
            modele,
            &[
                ("legende", &e(legende)),
                ("lien", &e(lien)),
                ("toile", &plein),
            ],
        );
        if n > 1 {
            vignettes += &remplir(
                if k == 0 {
                    g::VIGNETTE_PREMIERE
                } else {
                    g::VIGNETTE
                },
                &[("legende", &e(legende)), ("toile", &petit)],
            );
        }
    }

    // Actualités et bandeau : les dernières entrées du journal ; aucune, rien.
    let recentes: Vec<&Action> = journal.iter().take(ACTUALITES).collect();
    let (actualites, bandeau) = if recentes.is_empty() {
        (String::new(), String::new())
    } else {
        let mut articles = String::new();
        let mut jeu = String::new();
        let mut jeu_cache = String::new();
        for (k, a) in recentes.iter().enumerate() {
            let titre = e(&titre_action(a, liste));
            let (jour, court) = jour_court(&a.heure);
            articles += &remplir(
                g::ACTUALITE,
                &[
                    ("rang", &(k + 1).to_string()),
                    ("attributs", if k == 0 { "" } else { r#" tabindex="-1""# }),
                    ("piece", PIECES[k % PIECES.len()]),
                    ("titre", &titre),
                    ("date", &e(&jour)),
                ],
            );
            let cache = r#" tabindex="-1" aria-hidden="true""#;
            jeu += &article_du_bandeau(if k == 0 { "" } else { cache }, &court, &titre);
            jeu_cache += &article_du_bandeau(
                if k == 0 { r#" tabindex="-1""# } else { cache },
                &court,
                &titre,
            );
        }
        let id = format!(
            "journal-{}",
            recentes[0].heure.get(0..10).unwrap_or("entrees")
        );
        let actualites = remplir(g::ACTUALITES, &[("articles", &articles), ("id", &e(&id))]);
        let jeux = format!(
            r#"<div class="news-bar-set" {cid}>{jeu}</div><div class="news-bar-set" aria-hidden="true" {cid}>{jeu_cache}</div>"#,
            cid = g::CID_HERO
        );
        (actualites, remplir(g::BANDEAU, &[("jeux", &jeux)]))
    };
    remplir(
        g::HERO,
        &[
            ("diapos", &diapos),
            ("vignettes", &vignettes),
            ("legende", &e(&legendes[0].0)),
            ("actualites", &actualites),
            ("bandeau", &bandeau),
        ],
    )
}

fn article_du_bandeau(attributs: &str, quand: &str, titre: &str) -> String {
    format!(
        r#"<a href="/journal" class="news-bar-item"{attributs} {cid}><span class="news-bar-kind" {cid}>{}</span>{titre}</a>"#,
        e(quand),
        cid = g::CID_HERO
    )
}

/// Onglets de la grille : clé, genre de carte de la maquette, message vide.
const ONGLETS: [(&str, &str, &str); 3] = [
    ("toutes", "web", "Aucune entreprise pour l'instant."),
    ("directes", "motion", "Aucune entreprise directe."),
    (
        "rattachees",
        "mockup",
        "Aucune entreprise rattachée à un revendeur.",
    ),
];

fn grille(liste: &[VueEntreprise]) -> String {
    let directes: Vec<&VueEntreprise> = liste
        .iter()
        .filter(|v| v.entreprise.revendeur.is_none())
        .collect();
    let rattachees: Vec<&VueEntreprise> = liste
        .iter()
        .filter(|v| v.entreprise.revendeur.is_some())
        .collect();
    let toutes: Vec<&VueEntreprise> = liste.iter().collect();
    let onglets = [&toutes, &directes, &rattachees];

    let compte = |n: usize| format!("{:03}", n.min(999));
    let comptes = format!(
        r#"{{"toutes":"{}","directes":"{}","rattachees":"{}"}}"#,
        compte(toutes.len()),
        compte(directes.len()),
        compte(rattachees.len())
    );
    // Roulement de la première ligne : « Client direct » et les revendeurs.
    let mut mots: Vec<&str> = Vec::new();
    for v in liste {
        let mot = libelle_revendeur(v);
        if !mots.contains(&mot) {
            mots.push(mot);
        }
    }
    if mots.is_empty() {
        mots.push("Aucune entreprise");
    }
    let mut roulement: String = mots
        .iter()
        .map(|m| {
            format!(
                r#"<span class="odo-roll__w" aria-hidden="true">{}</span>"#,
                e(m)
            )
        })
        .collect();
    roulement += &format!(
        r#"<span class="visually-hidden">{}</span>"#,
        e(&mots.join(", "))
    );
    let mut revendeurs: Vec<&str> = Vec::new();
    for v in &rattachees {
        let nom = libelle_revendeur(v);
        if !revendeurs.contains(&nom) {
            revendeurs.push(nom);
        }
    }
    let ligne_rattachees = match revendeurs.len() {
        0 => "Aucun revendeur".to_string(),
        1 => revendeurs[0].to_string(),
        n => format!("{n} revendeurs"),
    };
    let libres =
        |l: &[&VueEntreprise]| paiements_a_utiliser(l.iter().map(|v| v.paiements_libres).sum());
    let chiffres = compte(toutes.len());
    let odo: Vec<String> = chiffres
        .bytes()
        .map(|b| match b - b'0' {
            0 => "0".to_string(),
            d => format!("-{}", u32::from(d) * 10),
        })
        .collect();

    let mut cartes: Vec<String> = Vec::new();
    for ((_, genre, vide), l) in ONGLETS.iter().zip(onglets) {
        if l.is_empty() {
            cartes.push(format!(
                r#"<div class="hp-slide" {cid}><p class="label-small muted" {cid}>{vide}</p></div>"#,
                cid = g::CID_PRODUITS
            ));
        } else {
            cartes.push(
                l.iter()
                    .enumerate()
                    .map(|(n, v)| carte(v, n, genre))
                    .collect(),
            );
        }
    }
    remplir(
        g::PRODUITS,
        &[
            ("comptes", &e(&comptes)),
            ("roulement", &roulement),
            ("ligne_directes", "Client direct"),
            ("ligne_rattachees", &e(&ligne_rattachees)),
            ("ligne2_toutes", &libres(&toutes)),
            ("ligne2_directes", &libres(&directes)),
            ("ligne2_rattachees", &libres(&rattachees)),
            ("odo0", &odo[0]),
            ("odo1", &odo[1]),
            ("odo2", &odo[2]),
            ("cartes_toutes", &cartes[0]),
            ("cartes_directes", &cartes[1]),
            ("cartes_rattachees", &cartes[2]),
        ],
    )
}

/// Une carte d'entreprise sur la carte d'origine du même onglet.
fn carte(v: &VueEntreprise, n: usize, genre: &str) -> String {
    let lien = chemin_entreprise(&v.entreprise.identifiant);
    let mut etiquettes = Vec::new();
    if let Some(l) = &v.derniere {
        etiquettes.push(if l.postes == 1 {
            "1 poste".to_string()
        } else {
            format!("{} postes", l.postes)
        });
    }
    if v.paiements_libres > 0 {
        etiquettes.push(paiements_a_utiliser(v.paiements_libres));
    }
    // Deux étiquettes au plus : la feuille n'a que st-4 et st-5 (--i 0 et 1).
    let etiquettes: String = etiquettes
        .iter()
        .take(2)
        .enumerate()
        .map(|(i, t)| {
            format!(
                r#"<span class="st-{} card-tag cc-tag" {}>{}</span>"#,
                4 + i,
                g::CID_CARTE,
                e(t)
            )
        })
        .collect();
    let decalage = n as f64 * 0.7;
    let pose = if n % 2 == 1 {
        toile(
            "P",
            "vignette",
            "quart",
            Some(FORMES_P[n % FORMES_P.len()]),
            decalage,
        )
    } else {
        toile("A2", "vignette", PIECES[n % PIECES.len()], None, decalage)
    };
    let mut calques = format!(
        r#"<canvas class="cc-layer" {pose} aria-hidden="true" {}></canvas>"#,
        g::CID_CARTE
    );
    if genre != "mockup" {
        calques += &format!(
            r#"<canvas class="cc-layer cc-layer--hover" {} aria-hidden="true" {}></canvas>"#,
            toile(
                "A2",
                "vignette",
                PIECES[(n + 1) % PIECES.len()],
                None,
                decalage
            ),
            g::CID_CARTE
        );
    }
    let echeance = match &v.derniere {
        Some(l) => format!("Échéance {}", date(&l.echeance)),
        None => "Aucune licence".to_string(),
    };
    remplir(
        g::CARTE,
        &[
            ("genre", genre),
            (
                "categorie",
                if v.entreprise.revendeur.is_none() {
                    "direct"
                } else {
                    "revendeur"
                },
            ),
            ("lien", &e(&lien)),
            ("paiement", &e(&format!("{lien}/paiement"))),
            ("client", &e(&v.entreprise.client)),
            ("etiquettes", &etiquettes),
            ("calques", &calques),
            ("revendeur", &e(libelle_revendeur(v))),
            ("echeance", &e(&echeance)),
        ],
    )
}

/// Bandeau des dernières actions ; la chaîne et le sceau pour l'éditeur.
/// Sans action, pas de bandeau.
fn bandeau_du_journal(journal: &[Action], chaine: Option<&EtatChaine>) -> String {
    let dernieres: Vec<&Action> = journal.iter().take(BOITES).collect();
    if dernieres.is_empty() {
        return String::new();
    }
    let distinctions = match chaine {
        Some(ch) => {
            let entrees = if ch.entrees > 1 {
                format!("{} entrées", ch.entrees)
            } else {
                format!("{} entrée", ch.entrees)
            };
            let etat = if ch.rupture.is_none() {
                "chaîne intacte"
            } else {
                "chaîne rompue"
            };
            let sceau: String = ch.sceau.chars().take(8).collect();
            remplir(
                g::KW_DISTINCTIONS,
                &[
                    ("entrees", &e(&entrees)),
                    ("etat", etat),
                    ("sceau", &e(&format!("Sceau {sceau}"))),
                ],
            )
        }
        None => String::new(),
    };
    let boite = |k: usize, a: &Action, cache: bool| {
        let qui = a.compte.as_deref().unwrap_or("sans compte");
        let mut citation = a.details.clone();
        if let Some(cible) = a.cible.as_deref() {
            let court: String = cible.chars().take(8).collect();
            let suite = if cible.chars().count() > 12 {
                format!("entreprise {court}...")
            } else {
                format!("entreprise {cible}")
            };
            citation = if citation.is_empty() {
                suite
            } else {
                format!("{citation}, {suite}")
            };
        }
        remplir(
            g::KW_BOITE,
            &[
                ("cache", if cache { r#" aria-hidden="true""# } else { "" }),
                ("piece", PIECES[k % PIECES.len()]),
                ("action", &e(libelle_action(&a.action))),
                ("quand", &e(&format!("{} (UTC), {qui}", heure(&a.heure)))),
                ("citation", &e(&citation)),
            ],
        )
    };
    let mut boites = String::new();
    for cache in [false, true] {
        for (k, a) in dernieres.iter().enumerate() {
            boites += &boite(k, a, cache);
        }
    }
    remplir(
        g::KW,
        &[("distinctions", &distinctions), ("boites", &boites)],
    )
}

/// La liste complète, sous le gabarit (cible du lien « Tout voir »).
fn liste_complete(editeur: bool, liste: &[VueEntreprise]) -> Markup {
    html! {
        section #liste .liste-entreprises {
            div.liste-tete {
                h2 { "Toutes les entreprises" }
                a.bouton.bouton-principal href="/entreprises/nouvelle" { "Nouvelle entreprise" }
            }
            p.texte {
                @if editeur {
                    "Toutes les entreprises clientes, directes ou rattachées à un revendeur. Ouvrez une entreprise pour enregistrer un paiement et émettre sa licence."
                } @else {
                    "Les entreprises qui vous sont rattachées. Ouvrez une entreprise pour enregistrer un paiement et émettre sa licence."
                }
            }
            div.carte {
                @if liste.is_empty() {
                    p.vide { "Aucune entreprise pour l'instant." }
                } @else {
                    div.tableau-cadre {
                        table {
                            thead {
                                tr {
                                    th { "Client" }
                                    th { "Identifiant de l'entreprise" }
                                    @if editeur { th { "Revendeur" } }
                                    th { "Échéance" }
                                    th.nombre { "Postes" }
                                    th { "Paiements" }
                                }
                            }
                            tbody {
                                @for v in liste {
                                    tr {
                                        td {
                                            a href=(chemin_entreprise(&v.entreprise.identifiant)) {
                                                (v.entreprise.client)
                                            }
                                        }
                                        td { span.identifiant { (v.entreprise.identifiant) } }
                                        @if editeur { td { (libelle_revendeur(v)) } }
                                        @match &v.derniere {
                                            Some(l) => {
                                                td.court { (date(&l.echeance)) }
                                                td.nombre { (l.postes) }
                                            }
                                            None => {
                                                td.court { span.etat.etat-neutre { "Aucune licence" } }
                                                td.nombre { "-" }
                                            }
                                        }
                                        td.court {
                                            @if v.paiements_libres > 0 {
                                                span.etat.etat-attente { (paiements_a_utiliser(v.paiements_libres)) }
                                            } @else {
                                                span.etat.etat-neutre { "Aucun à utiliser" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attributs_des_sections() {
        assert_eq!(g::CID_HERO, "data-astro-cid-ge2uvauf");
        assert_eq!(g::CID_PRODUITS, "data-astro-cid-25qzlwwf");
        assert_eq!(g::CID_CARTE, "data-astro-cid-n5pov4vd");
        assert_eq!(g::CID_KW, "data-astro-cid-d4dfv7pp");
    }

    #[test]
    fn jours_et_nombres() {
        assert_eq!(
            jour_court("2026-11-19 10:42:07"),
            ("19 nov. 2026".to_string(), "19 nov.".to_string())
        );
        assert_eq!(jour_court("2026-02-01 00:00:00").1, "1 févr.");
        assert_eq!(nombre(0.0), "0");
        assert_eq!(nombre(2.5), "2.5");
        assert_eq!(nombre(3.0 * 0.7), "2.1");
        assert_eq!(nombre(7.5), "7.5");
    }
}
