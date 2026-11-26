//! Routes du portail (spécification du portail, sections 2 et 6) : pages
//! servies en HTTP par axum, sessions par cookie, jeton anti-falsification sur
//! chaque formulaire, en-têtes de sécurité, un seul script (code de connexion
//! en six cases, servi ici). Une seule route
//! hors de la console : `POST /api/activation` (décision 74), en JSON, sans
//! session ni jeton, appelée par le programme du client.
//!
//! Chaque appel au portail (base SQLite, synchrone) passe par
//! `spawn_blocking`, pour ne jamais bloquer un fil de tokio.

use std::collections::HashMap;
use std::future::Future;
use std::net::{IpAddr, SocketAddr, TcpListener as StdListener};
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::sync::LazyLock;
use std::task::{Context, Poll};
use std::thread::JoinHandle;

use axum::Router;
use axum::body::Bytes;
use axum::extract::{ConnectInfo, DefaultBodyLimit, Form, Path, Request, State};
use axum::http::header::{
    ACCEPT_RANGES, CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE,
    COOKIE, LOCATION, RANGE, REFERRER_POLICY, SET_COOKIE, X_CONTENT_TYPE_OPTIONS,
};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode, Uri};
use axum::middleware::{Next, from_fn, map_response};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use kadenz_domaine::module::Module;
use maud::Markup;

use crate::pages::{self, Cadre};
use crate::telechargement::{self, Plage, Signataire};
use crate::{
    Compte, DemandeActivation, DemandeEssai, ErreurPortail, MODELES_ACTIVATION, Message,
    NouveauPaiement, Portail,
};

/// Nom du cookie de session. Le préfixe `__Host-` oblige le navigateur à ne
/// l'accepter qu'avec `Secure`, `Path=/` et sans `Domain` : un sous-domaine
/// ne peut ni le poser ni le remplacer.
pub const COOKIE_SESSION: &str = "__Host-kadenz_portail";

/// Cookie du jeton de la page de connexion (avant toute session) : le même
/// jeton revient dans le formulaire, sinon la connexion est refusée. Un
/// autre site ne peut donc pas connecter le navigateur à un compte de son
/// choix (falsification de la connexion).
pub const COOKIE_CONNEXION: &str = "__Host-kadenz_connexion";

/// Durée du jeton de connexion, en secondes.
const DUREE_JETON_CONNEXION: u32 = 3600;

/// Cookie de l'étape de connexion, entre le mot de passe accepté et le code
/// de double authentification ; le même jeton revient dans le formulaire du
/// code.
pub const COOKIE_ETAPE: &str = "__Host-kadenz_etape";

/// Durée du cookie d'étape, en secondes (celle de l'étape du portail).
const DUREE_COOKIE_ETAPE: u32 = 300;

const POLICE_DIATYPE: &[u8] = include_bytes!("../statique/polices/ABCDiatype-Regular.woff2");
const POLICE_DIATYPE_MEDIUM: &[u8] = include_bytes!("../statique/polices/ABCDiatype-Medium.woff");
const POLICE_DIATYPE_BOLD: &[u8] = include_bytes!("../statique/polices/ABCDiatype-Bold.woff2");
const POLICE_DIATYPE_MONO: &[u8] =
    include_bytes!("../statique/polices/ABCDiatypeMono-Regular.woff2");
const FAVICON: &[u8] = include_bytes!("../statique/favicon-portail-6c6118946e.svg");
const PILIERS: &[u8] = include_bytes!("../statique/piliers-ff8f49030c.svg");
const SCRIPT_CODE: &[u8] = include_bytes!("../statique/code-connexion-bd47a7a773.js");
const PARTAGE: &[u8] = include_bytes!("../statique/partage-9405c98b03.png");

/// robots.txt : le portail reste hors des moteurs de recherche (choix de
/// l'utilisateur du 2026-11-19). Seule la page de connexion, et ce qu'il faut
/// pour la rendre, peut être lue : un robot y trouve le `noindex` (une page
/// interdite de lecture peut être indexée sans son contenu, sur la foi des
/// liens). `$` limite la règle à `/connexion` même, pas `/connexion/code` ;
/// la feuille de style, sans `$`, garde son empreinte en requête.
static ROBOTS: LazyLock<String> = LazyLock::new(|| {
    format!(
        "User-agent: *\nDisallow: /\nAllow: /connexion$\nAllow: /style.css\nAllow: /police/\nAllow: {}$\nAllow: {}$\nAllow: {}$\n",
        pages::FAVICON,
        pages::IMAGE_PILIERS,
        pages::IMAGE_PARTAGE
    )
});

/// Taille maximale d'un corps de requête. Le plus gros formulaire (une
/// licence collée, un compte) et la demande d'activation tiennent en
/// quelques kilo-octets ; le défaut d'axum, 2 Mio, laissait passer de
/// longs textes vers le journal (audit d'OpenCode, 2026-11-19).
pub const CORPS_MAX: usize = 64 * 1024;

/// En-tête qui porte l'adresse du client, posé par le mandataire.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EnteteClient {
    /// `X-Forwarded-For`, dernière valeur : Caddy y ajoute l'adresse du
    /// client en dernier, Funnel le remplace par l'adresse source réelle
    /// (`addProxyForwardedHeaders`, ipn/ipnlocal/serve.go de Tailscale).
    /// Défaut, pour l'essai local.
    #[default]
    XForwardedFor,
    /// `CF-Connecting-IP`, une seule valeur : derrière Cloudflare Tunnel
    /// (vérifié par OpenCode dans la documentation de Cloudflare).
    CfConnectingIp,
}

impl EnteteClient {
    /// Valeur de l'option `--entete-client`.
    pub fn depuis_option(texte: &str) -> Option<EnteteClient> {
        match texte {
            "x-forwarded-for" => Some(EnteteClient::XForwardedFor),
            "cf-connecting-ip" => Some(EnteteClient::CfConnectingIp),
            _ => None,
        }
    }
}

/// Réglages des routes.
#[derive(Clone, Default)]
pub struct ConfigWeb {
    /// Adresse du mandataire de confiance (Caddy, tailscaled pour Funnel, ou
    /// cloudflared) : pour une requête qui vient de lui, et de lui seul,
    /// l'adresse du client est lue dans l'en-tête `entete_client`, pour
    /// freiner par adresse. Une requête d'une autre adresse garde son adresse
    /// de connexion, quel que soit l'en-tête envoyé.
    pub mandataire: Option<IpAddr>,
    /// En-tête lu pour les requêtes du mandataire.
    pub entete_client: EnteteClient,
    /// Adresse du portail sur Internet (`https://nom`, sans barre finale),
    /// pour l'aperçu des liens partagés de la page de connexion ; rien n'est
    /// émis sans elle (jamais tirée de l'en-tête `Host`).
    pub adresse_publique: Option<String>,
    /// Dépôt des paquets à télécharger (option `--depot`, décision 77) ; sans
    /// lui, les pages de téléchargement répondent 404.
    pub depot: Option<std::path::PathBuf>,
}

/// Adresse publique admise par `--adresse-publique` : `https://` puis un nom
/// d'hôte (lettres, chiffres, points, tirets, port éventuel), barre finale
/// retirée. Rien d'autre, pour qu'elle ne puisse rien glisser dans la page.
pub fn adresse_publique(texte: &str) -> Option<String> {
    let texte = texte.trim().trim_end_matches('/');
    let hote = texte.strip_prefix("https://")?;
    let permis = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':');
    if hote.is_empty() || !hote.chars().all(permis) || hote.starts_with(['.', '-', ':']) {
        return None;
    }
    Some(texte.to_string())
}

#[derive(Clone)]
struct Etat {
    portail: Portail,
    config: ConfigWeb,
    /// Signataire des liens de téléchargement, tiré au démarrage.
    signataire: Signataire,
}

type Champs = HashMap<String, String>;

/// Table des routes du portail.
pub fn routeur(portail: Portail, config: ConfigWeb) -> Router {
    Router::new()
        .route("/", get(accueil))
        .route("/connexion", get(page_connexion).post(connexion))
        .route("/connexion/code", get(page_code).post(confirmer_code))
        .route(
            "/double-authentification",
            get(page_double).post(activer_double),
        )
        .route("/deconnexion", post(deconnexion))
        .route(
            "/entreprises/nouvelle",
            get(page_nouvelle_entreprise).post(nouvelle_entreprise),
        )
        .route("/entreprises/{id}", get(fiche))
        .route(
            "/entreprises/{id}/paiement",
            get(page_paiement).post(paiement),
        )
        .route("/paiements/{id}/licence", get(page_emission).post(emission))
        .route("/licences/{id}", get(licence))
        .route("/codes", get(page_codes).post(tirer_codes))
        .route("/codes/{id}", get(code))
        .route("/codes/{id}/annuler", post(annuler_code))
        .route("/codes/{id}/liberer", post(liberer_poste))
        .route("/api/activation", post(activation))
        .route("/api/essai", post(essai))
        .route("/telecharger/{produit}", get(page_telechargement))
        .route("/fichiers/{nom}", get(fichier))
        .route("/api/releases", get(releases))
        .route("/comptes", get(page_comptes).post(nouveau_compte))
        .route("/comptes/{id}/actif", post(regler_actif))
        .route("/mot-de-passe", get(page_mot_de_passe).post(mot_de_passe))
        .route("/journal", get(journal))
        .route(
            "/style.css",
            get(|| async { statique("text/css; charset=utf-8", pages::STYLE.as_bytes(), false) }),
        )
        .route(
            "/police/diatype-regular.woff2",
            get(|| async { statique("font/woff2", POLICE_DIATYPE, true) }),
        )
        .route(
            "/police/diatype-medium.woff",
            get(|| async { statique("font/woff", POLICE_DIATYPE_MEDIUM, true) }),
        )
        .route(
            "/police/diatype-bold.woff2",
            get(|| async { statique("font/woff2", POLICE_DIATYPE_BOLD, true) }),
        )
        .route(
            "/police/diatype-mono-regular.woff2",
            get(|| async { statique("font/woff2", POLICE_DIATYPE_MONO, true) }),
        )
        .route(
            pages::FAVICON,
            get(|| async { statique("image/svg+xml", FAVICON, true) }),
        )
        // Adresse habituelle, demandée par certains navigateurs sans lire la
        // page : même icône, revalidée à chaque fois.
        .route(
            "/favicon.svg",
            get(|| async { statique("image/svg+xml", FAVICON, false) }),
        )
        .route(
            pages::IMAGE_PILIERS,
            get(|| async { statique("image/svg+xml", PILIERS, true) }),
        )
        .route(
            pages::SCRIPT_CODE,
            get(|| async { statique("text/javascript; charset=utf-8", SCRIPT_CODE, true) }),
        )
        .route(
            "/robots.txt",
            get(|| async { statique("text/plain; charset=utf-8", ROBOTS.as_bytes(), false) }),
        )
        .route(
            pages::IMAGE_PARTAGE,
            get(|| async { statique("image/png", PARTAGE, true) }),
        )
        // Santé (vérification de Docker, « kadenz-portail sante ») : hors des
        // sessions et du frein, ne dit rien d'autre que « ok ».
        .route(
            "/sante",
            get(|| async { statique("text/plain; charset=utf-8", b"ok", false) }),
        )
        .merge(routes_gabarit())
        .fallback(introuvable)
        // Du plus intérieur au plus extérieur : limite lue par les
        // extracteurs, refus sur Content-Length avant toute lecture, panique
        // rendue en erreur 500, en-têtes de sécurité sur toute réponse.
        .layer(DefaultBodyLimit::max(CORPS_MAX))
        .layer(from_fn(borne_corps))
        .layer(from_fn(sans_panique))
        .layer(map_response(entetes))
        .with_state(Etat {
            portail,
            config,
            signataire: Signataire::nouveau().expect("aléa du système indisponible"),
        })
}

/// Corps annoncé plus long que `CORPS_MAX` : 413 sans rien lire. Un corps
/// sans longueur annoncée (envoi par morceaux) est arrêté par
/// `DefaultBodyLimit` dès qu'il dépasse.
async fn borne_corps(requete: Request, suite: Next) -> Response {
    let annonce = requete
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse::<u64>().ok());
    if annonce.is_some_and(|n| n > CORPS_MAX as u64) {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
    }
    suite.run(requete).await
}

/// Futur dont une panique est rattrapée au lieu de couper la connexion sans
/// réponse (générateur aléatoire, tâche bloquante interrompue).
struct SansPanique<F>(Pin<Box<F>>);

impl<F: Future> Future for SansPanique<F> {
    type Output = std::thread::Result<F::Output>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match std::panic::catch_unwind(AssertUnwindSafe(|| self.0.as_mut().poll(cx))) {
            Ok(Poll::Pending) => Poll::Pending,
            Ok(Poll::Ready(v)) => Poll::Ready(Ok(v)),
            Err(panique) => Poll::Ready(Err(panique)),
        }
    }
}

/// Réponse d'une route, ou la page d'erreur 500 (sans détail) si elle panique.
async fn garder(route: impl Future<Output = Response>) -> Response {
    match SansPanique(Box::pin(route)).await {
        Ok(reponse) => reponse,
        Err(_) => page_erreur(None, &ErreurPortail::Interne(String::new())),
    }
}

async fn sans_panique(requete: Request, suite: Next) -> Response {
    garder(suite.run(requete)).await
}

/// En-têtes de sécurité sur chaque réponse. Scripts : ceux du portail
/// seulement (`script-src 'self'`, aujourd'hui le seul code en six cases),
/// jamais en ligne ni évalués.
async fn entetes(mut reponse: Response) -> Response {
    let h = reponse.headers_mut();
    h.insert(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self'; object-src 'none'; form-action 'self'; \
             frame-ancestors 'none'; base-uri 'none'",
        ),
    );
    h.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    h.insert(REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    h.insert(
        HeaderName::from_static("x-robots-tag"),
        HeaderValue::from_static("noindex, nofollow"),
    );
    h.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    if !h.contains_key(CACHE_CONTROL) {
        h.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
    reponse
}

fn statique(type_contenu: &'static str, octets: &'static [u8], long: bool) -> Response {
    let cache = if long {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    (
        [(CONTENT_TYPE, type_contenu), (CACHE_CONTROL, cache)],
        octets,
    )
        .into_response()
}

fn html(statut: StatusCode, page: Markup) -> Response {
    (statut, Html(page.into_string())).into_response()
}

fn rediriger(vers: &str) -> Response {
    (StatusCode::SEE_OTHER, [(LOCATION, vers.to_string())]).into_response()
}

/// Appel synchrone au portail hors des fils de tokio.
async fn bloquant<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    tokio::task::spawn_blocking(f)
        .await
        .expect("tâche du portail interrompue")
}

fn valeur_cookie(entetes: &HeaderMap, cookie: &str) -> Option<String> {
    entetes
        .get_all(COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|morceau| morceau.trim().split_once('='))
        .find(|(nom, _)| *nom == cookie)
        .map(|(_, valeur)| valeur.to_string())
}

fn jeton_du_cookie(entetes: &HeaderMap) -> Option<String> {
    valeur_cookie(entetes, COOKIE_SESSION)
}

/// Session du compte connecté : compte et jeton anti-falsification ;
/// `limitee` : éditeur sans double authentification, qui n'a droit qu'à sa
/// mise en place et à la déconnexion.
struct Connecte {
    compte: Compte,
    csrf: String,
    limitee: bool,
    menus: pages::Menus,
}

impl Connecte {
    fn cadre<'a>(&'a self, message: Option<&'a Message>) -> Cadre<'a> {
        Cadre {
            compte: &self.compte,
            csrf: &self.csrf,
            message,
            menus: &self.menus,
        }
    }
}

async fn connecte(etat: &Etat, entetes: &HeaderMap) -> Option<Connecte> {
    let jeton = jeton_du_cookie(entetes)?;
    let portail = etat.portail.clone();
    let (compte, csrf, limitee, menus) = bloquant(move || {
        let (compte, csrf) = portail.session(&jeton)?;
        let limitee = portail.mise_en_place_exigee(&compte);
        let menus = menus_du_compte(&portail, &compte);
        Some((compte, csrf, limitee, menus))
    })
    .await?;
    Some(Connecte {
        compte,
        csrf,
        limitee,
        menus,
    })
}

/// Listes du menu des pages du compte : entreprises visibles, revendeurs
/// (éditeur), dernières actions du journal. Une lecture en échec laisse la
/// liste vide (lien simple, sans panneau).
fn menus_du_compte(portail: &Portail, compte: &Compte) -> pages::Menus {
    let entreprises = portail.entreprises(compte).unwrap_or_default();
    let revendeurs = if compte.role == crate::Role::Editeur {
        portail.revendeurs(compte).unwrap_or_default()
    } else {
        Vec::new()
    };
    let journal = portail.journal(compte).unwrap_or_default();
    pages::menus(&entreprises, &revendeurs, &journal)
}

/// Fichiers du gabarit animé (`pages::gabarit`, généré) : nom à empreinte,
/// cache long.
fn routes_gabarit() -> Router<Etat> {
    let mut routes = Router::new();
    for &(chemin, type_contenu, octets) in pages::gabarit::FICHIERS {
        routes = routes.route(
            chemin,
            get(move || async move { statique(type_contenu, octets, true) }),
        );
    }
    routes
}

/// Comparaison en temps constant des jetons anti-falsification.
fn egaux(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}

/// Session exigée ; pour un POST, jeton anti-falsification vérifié aussi.
/// Toute route qui agit avec les droits d'un compte passe par ici : une
/// session limitée (éditeur sans double authentification) est renvoyée
/// vers la mise en place.
async fn exiger(
    etat: &Etat,
    entetes: &HeaderMap,
    champs: Option<&Champs>,
) -> Result<Connecte, Box<Response>> {
    exiger_avec(etat, entetes, champs, false).await
}

/// Comme [`exiger`] ; `limitee_permise` : la route sert aussi une session
/// limitée (mise en place de la double authentification, déconnexion).
async fn exiger_avec(
    etat: &Etat,
    entetes: &HeaderMap,
    champs: Option<&Champs>,
    limitee_permise: bool,
) -> Result<Connecte, Box<Response>> {
    let Some(c) = connecte(etat, entetes).await else {
        return Err(Box::new(rediriger("/connexion")));
    };
    if let Some(champs) = champs {
        let recu = champs.get("csrf").map(String::as_str).unwrap_or("");
        if !egaux(recu, &c.csrf) {
            return Err(Box::new(html(
                StatusCode::FORBIDDEN,
                pages::erreur(
                    Some(&c.cadre(None)),
                    403,
                    "formulaire expiré : rechargez la page et recommencez",
                ),
            )));
        }
    }
    if c.limitee && !limitee_permise {
        return Err(Box::new(rediriger("/double-authentification")));
    }
    Ok(c)
}

/// Message affiché après une action, choisi par un code fixe de l'adresse
/// (aucun texte de l'adresse n'est repris dans la page).
fn message_de(uri: &Uri) -> Option<Message> {
    let code = uri.query()?.split('&').find_map(|p| p.strip_prefix("m="))?;
    let texte = match code {
        "entreprise" => "Entreprise enregistrée.",
        "paiement" => "Paiement enregistré.",
        "licence" => "Licence émise : copiez-la dans la rubrique « Abonnement » d'un poste Admin.",
        "compte" => "Compte revendeur créé.",
        "actif" => "Compte mis à jour.",
        "mot_de_passe" => "Mot de passe changé.",
        "code_annule" => "Code annulé : il ne peut plus être activé.",
        "poste_libere" => {
            "Poste libéré : le code peut être activé sur un autre poste, avec la même échéance."
        }
        _ => return None,
    };
    Some(Message {
        ok: true,
        texte: texte.into(),
    })
}

/// Erreur du portail en page : 403, 404 ou 500 (une erreur de saisie est
/// traitée par chaque page, avec son formulaire).
fn page_erreur(c: Option<&Connecte>, e: &ErreurPortail) -> Response {
    let (statut, texte) = match e {
        ErreurPortail::Interdit => (StatusCode::FORBIDDEN, e.to_string()),
        ErreurPortail::Introuvable => (StatusCode::NOT_FOUND, "page introuvable".to_string()),
        // Le détail (base, signature) va dans les traces, jamais dans la page.
        ErreurPortail::Interne(detail) => {
            eprintln!("erreur interne : {detail}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "erreur interne du portail : réessayez plus tard".to_string(),
            )
        }
        _ => (StatusCode::BAD_REQUEST, e.to_string()),
    };
    let cadre = c.map(|c| c.cadre(None));
    html(
        statut,
        pages::erreur(cadre.as_ref(), statut.as_u16(), &texte),
    )
}

fn champ<'a>(champs: &'a Champs, nom: &str) -> &'a str {
    champs.get(nom).map(String::as_str).unwrap_or("")
}

fn adresse_client(etat: &Etat, entetes: &HeaderMap, pair: SocketAddr) -> String {
    adresse_selon(
        etat.config.mandataire,
        etat.config.entete_client,
        entetes,
        pair,
    )
    .to_string()
}

/// Adresse du client, seulement si la connexion vient du mandataire de
/// confiance et que la valeur lue est une adresse IP ; sinon l'adresse de
/// connexion.
/// - `X-Forwarded-For` : la dernière valeur, celle que le mandataire a posée.
/// - `CF-Connecting-IP` : une valeur unique ; plusieurs en-têtes, ou une
///   liste, ne viennent pas de Cloudflare et sont ignorés.
fn adresse_selon(
    mandataire: Option<IpAddr>,
    entete: EnteteClient,
    entetes: &HeaderMap,
    pair: SocketAddr,
) -> IpAddr {
    let pair_ip = pair.ip().to_canonical();
    if mandataire.map(|m| m.to_canonical()) != Some(pair_ip) {
        return pair_ip;
    }
    let transmise = match entete {
        EnteteClient::XForwardedFor => entetes
            .get_all("x-forwarded-for")
            .iter()
            .filter_map(|v| v.to_str().ok())
            .next_back()
            .and_then(|v| v.rsplit(',').next())
            .and_then(|v| v.trim().parse::<IpAddr>().ok()),
        EnteteClient::CfConnectingIp => {
            let mut valeurs = entetes.get_all("cf-connecting-ip").iter();
            match (valeurs.next(), valeurs.next()) {
                (Some(v), None) => v
                    .to_str()
                    .ok()
                    .and_then(|v| v.trim().parse::<IpAddr>().ok()),
                _ => None,
            }
        }
    };
    transmise.map_or(pair_ip, |ip| ip.to_canonical())
}

/// Cookie de session : sans `Max-Age`, il disparaît à la fermeture du
/// navigateur ; le portail borne de son côté la session à 12 heures.
fn cookie_session(jeton: &str) -> String {
    format!("{COOKIE_SESSION}={jeton}; Path=/; HttpOnly; Secure; SameSite=Strict")
}

fn cookie_efface(nom: &str) -> String {
    format!("{nom}=; Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age=0")
}

/// Page de connexion avec un jeton neuf, posé aussi en cookie.
fn page_de_connexion(statut: StatusCode, erreur: Option<&str>) -> Response {
    page_de_connexion_avec(statut, erreur, None)
}

/// Page de connexion avec l'aperçu de partage (GET /connexion seulement).
fn page_de_connexion_avec(
    statut: StatusCode,
    erreur: Option<&str>,
    adresse_publique: Option<&str>,
) -> Response {
    let jeton = crate::sessions::alea();
    let mut r = html(statut, pages::connexion(&jeton, erreur, adresse_publique));
    if let Ok(v) = HeaderValue::from_str(&format!(
        "{COOKIE_CONNEXION}={jeton}; Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age={DUREE_JETON_CONNEXION}"
    )) {
        r.headers_mut().append(SET_COOKIE, v);
    }
    r
}

/// Formulaire envoyé depuis le portail lui-même : pas de
/// `Sec-Fetch-Site: cross-site` ni `same-site`.
fn origine_permise(entetes: &HeaderMap) -> bool {
    let site = entetes
        .get("sec-fetch-site")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    site != "cross-site" && site != "same-site"
}

/// Demande de connexion légitime : envoyée depuis le portail lui-même, avec
/// le jeton du formulaire égal à celui du cookie.
fn connexion_permise(entetes: &HeaderMap, champs: &Champs) -> bool {
    if !origine_permise(entetes) {
        return false;
    }
    let recu = champs.get("csrf").map(String::as_str).unwrap_or("");
    match valeur_cookie(entetes, COOKIE_CONNEXION) {
        Some(attendu) => !attendu.is_empty() && egaux(recu, &attendu),
        None => false,
    }
}

// ----- Connexion -----

async fn accueil(State(etat): State<Etat>, entetes: HeaderMap, uri: Uri) -> Response {
    let c = match exiger(&etat, &entetes, None).await {
        Ok(c) => c,
        Err(r) => return *r,
    };
    let portail = etat.portail.clone();
    let compte = c.compte.clone();
    // Le journal et sa chaîne ne servent qu'aux sections du gabarit : une
    // lecture en échec les laisse vides, la liste des entreprises reste.
    let lu = bloquant(move || {
        let liste = portail.entreprises(&compte)?;
        let journal = portail.journal(&compte).unwrap_or_default();
        let chaine = portail.chaine_du_journal(&compte).ok().flatten();
        Ok::<_, ErreurPortail>((liste, journal, chaine))
    })
    .await;
    match lu {
        Ok((liste, journal, chaine)) => {
            let m = message_de(&uri);
            html(
                StatusCode::OK,
                pages::entreprises(&c.cadre(m.as_ref()), &liste, &journal, chaine.as_ref()),
            )
        }
        Err(e) => page_erreur(Some(&c), &e),
    }
}

async fn page_connexion(State(etat): State<Etat>, entetes: HeaderMap) -> Response {
    if connecte(&etat, &entetes).await.is_some() {
        return rediriger("/");
    }
    page_de_connexion_avec(
        StatusCode::OK,
        None,
        etat.config.adresse_publique.as_deref(),
    )
}

async fn connexion(
    State(etat): State<Etat>,
    ConnectInfo(pair): ConnectInfo<SocketAddr>,
    entetes: HeaderMap,
    Form(champs): Form<Champs>,
) -> Response {
    if !connexion_permise(&entetes, &champs) {
        return page_de_connexion(
            StatusCode::FORBIDDEN,
            Some("formulaire expiré : rechargez la page et recommencez"),
        );
    }
    let adresse = adresse_client(&etat, &entetes, pair);
    let portail = etat.portail.clone();
    let identifiant = champ(&champs, "identifiant").to_string();
    let mot_de_passe = champ(&champs, "mot_de_passe").to_string();
    let ancienne = jeton_du_cookie(&entetes);
    let resultat = bloquant(move || {
        let r = portail.connecter(&identifiant, &mot_de_passe, &adresse);
        // Rotation : une connexion réussie ferme la session que portait le
        // navigateur, et en ouvre une neuve (jamais de jeton repris).
        if r.is_ok()
            && let Some(ancienne) = ancienne
        {
            portail.deconnecter(&ancienne);
        }
        r
    })
    .await;
    match resultat {
        Ok((jeton, _)) => {
            let mut r = rediriger("/");
            for cookie in [cookie_session(&jeton), cookie_efface(COOKIE_CONNEXION)] {
                if let Ok(v) = HeaderValue::from_str(&cookie) {
                    r.headers_mut().append(SET_COOKIE, v);
                }
            }
            r
        }
        Err(ErreurPortail::DeuxiemeFacteur(etape)) => {
            let mut r = rediriger("/connexion/code");
            for cookie in [
                format!(
                    "{COOKIE_ETAPE}={etape}; Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age={DUREE_COOKIE_ETAPE}"
                ),
                cookie_efface(COOKIE_CONNEXION),
            ] {
                if let Ok(v) = HeaderValue::from_str(&cookie) {
                    r.headers_mut().append(SET_COOKIE, v);
                }
            }
            r
        }
        Err(e @ (ErreurPortail::Identifiants | ErreurPortail::Freine(_))) => {
            page_de_connexion(StatusCode::UNAUTHORIZED, Some(&e.to_string()))
        }
        Err(e) => page_erreur(None, &e),
    }
}

/// Page du code de double authentification, pour une étape en cours.
async fn page_code(State(etat): State<Etat>, entetes: HeaderMap) -> Response {
    let Some(etape) = valeur_cookie(&entetes, COOKIE_ETAPE) else {
        return rediriger("/connexion");
    };
    let portail = etat.portail.clone();
    let e = etape.clone();
    if !bloquant(move || portail.etape_valable(&e)).await {
        return rediriger("/connexion");
    }
    html(StatusCode::OK, pages::code_connexion(&etape, None))
}

/// Seconde étape de la connexion : le jeton d'étape du cookie doit revenir
/// dans le formulaire, envoyé depuis le portail lui-même.
async fn confirmer_code(
    State(etat): State<Etat>,
    ConnectInfo(pair): ConnectInfo<SocketAddr>,
    entetes: HeaderMap,
    Form(champs): Form<Champs>,
) -> Response {
    let etape = valeur_cookie(&entetes, COOKIE_ETAPE).unwrap_or_default();
    if !origine_permise(&entetes) || etape.is_empty() || !egaux(champ(&champs, "etape"), &etape) {
        return page_de_connexion(
            StatusCode::FORBIDDEN,
            Some("formulaire expiré : reconnectez-vous"),
        );
    }
    let adresse = adresse_client(&etat, &entetes, pair);
    let portail = etat.portail.clone();
    let code = champ(&champs, "code").to_string();
    let ancienne = jeton_du_cookie(&entetes);
    let e = etape.clone();
    let resultat = bloquant(move || {
        let r = portail.confirmer_code(&e, &code, &adresse);
        // Rotation, comme pour une connexion sans double authentification.
        if r.is_ok()
            && let Some(ancienne) = ancienne
        {
            portail.deconnecter(&ancienne);
        }
        r
    })
    .await;
    match resultat {
        Ok((jeton, _)) => {
            let mut r = rediriger("/");
            for cookie in [cookie_session(&jeton), cookie_efface(COOKIE_ETAPE)] {
                if let Ok(v) = HeaderValue::from_str(&cookie) {
                    r.headers_mut().append(SET_COOKIE, v);
                }
            }
            r
        }
        Err(ErreurPortail::Identifiants) => {
            let mut r = page_de_connexion(
                StatusCode::UNAUTHORIZED,
                Some("délai dépassé ou trop de codes faux : reconnectez-vous"),
            );
            if let Ok(v) = HeaderValue::from_str(&cookie_efface(COOKIE_ETAPE)) {
                r.headers_mut().append(SET_COOKIE, v);
            }
            r
        }
        Err(e @ (ErreurPortail::Validation(_) | ErreurPortail::Freine(_))) => html(
            StatusCode::UNAUTHORIZED,
            pages::code_connexion(&etape, Some(&e.to_string())),
        ),
        Err(e) => page_erreur(None, &e),
    }
}

// ----- Double authentification -----

/// Mise en place (session limitée) ; une fois active, la page le dit.
async fn page_double(State(etat): State<Etat>, entetes: HeaderMap) -> Response {
    let c = match exiger_avec(&etat, &entetes, None, true).await {
        Ok(c) => c,
        Err(r) => return *r,
    };
    if !c.limitee {
        return html(StatusCode::OK, pages::double_active(&c.cadre(None)));
    }
    let portail = etat.portail.clone();
    let compte = c.compte.clone();
    match bloquant(move || portail.preparer_double_authentification(&compte)).await {
        Ok(secret) => html(
            StatusCode::OK,
            pages::mise_en_place(&c.cadre(None), &secret, None),
        ),
        Err(e) => page_erreur(Some(&c), &e),
    }
}

async fn activer_double(
    State(etat): State<Etat>,
    entetes: HeaderMap,
    Form(champs): Form<Champs>,
) -> Response {
    let c = match exiger_avec(&etat, &entetes, Some(&champs), true).await {
        Ok(c) => c,
        Err(r) => return *r,
    };
    if !c.limitee {
        return rediriger("/double-authentification");
    }
    let portail = etat.portail.clone();
    let compte = c.compte.clone();
    let code = champ(&champs, "code").to_string();
    let resultat =
        bloquant(
            move || match portail.activer_double_authentification(&compte, &code) {
                Ok(codes) => Ok(Ok(codes)),
                Err(ErreurPortail::Validation(m)) => portail
                    .preparer_double_authentification(&compte)
                    .map(|secret| Err((secret, m))),
                Err(e) => Err(e),
            },
        )
        .await;
    match resultat {
        Ok(Ok(codes)) => html(StatusCode::OK, pages::codes_secours(&c.cadre(None), &codes)),
        Ok(Err((secret, m))) => html(
            StatusCode::OK,
            pages::mise_en_place(&c.cadre(None), &secret, Some(&m)),
        ),
        Err(e) => page_erreur(Some(&c), &e),
    }
}

async fn deconnexion(
    State(etat): State<Etat>,
    entetes: HeaderMap,
    Form(champs): Form<Champs>,
) -> Response {
    if exiger_avec(&etat, &entetes, Some(&champs), true)
        .await
        .is_ok()
        && let Some(jeton) = jeton_du_cookie(&entetes)
    {
        let portail = etat.portail.clone();
        bloquant(move || portail.deconnecter(&jeton)).await;
    }
    (
        StatusCode::SEE_OTHER,
        [
            (LOCATION, "/connexion".to_string()),
            (SET_COOKIE, cookie_efface(COOKIE_SESSION)),
        ],
    )
        .into_response()
}

// ----- Entreprises -----

async fn page_nouvelle_entreprise(State(etat): State<Etat>, entetes: HeaderMap) -> Response {
    let c = match exiger(&etat, &entetes, None).await {
        Ok(c) => c,
        Err(r) => return *r,
    };
    let revendeurs = revendeurs_de(&etat, &c).await;
    html(
        StatusCode::OK,
        pages::nouvelle_entreprise(&c.cadre(None), &revendeurs, None),
    )
}

/// Revendeurs proposés au rattachement : pour l'éditeur seulement.
async fn revendeurs_de(etat: &Etat, c: &Connecte) -> Vec<Compte> {
    if c.compte.role != crate::Role::Editeur {
        return Vec::new();
    }
    let portail = etat.portail.clone();
    let compte = c.compte.clone();
    bloquant(move || portail.revendeurs(&compte))
        .await
        .unwrap_or_default()
}

async fn nouvelle_entreprise(
    State(etat): State<Etat>,
    entetes: HeaderMap,
    Form(champs): Form<Champs>,
) -> Response {
    let c = match exiger(&etat, &entetes, Some(&champs)).await {
        Ok(c) => c,
        Err(r) => return *r,
    };
    let portail = etat.portail.clone();
    let compte = c.compte.clone();
    let identifiant = champ(&champs, "identifiant").to_string();
    let client = champ(&champs, "client").to_string();
    let revendeur = champ(&champs, "revendeur").trim().parse::<i64>().ok();
    match bloquant(move || portail.creer_entreprise(&compte, &identifiant, &client, revendeur))
        .await
    {
        Ok(e) => rediriger(&format!("/entreprises/{}?m=entreprise", e.identifiant)),
        Err(ErreurPortail::Validation(m)) => {
            let revendeurs = revendeurs_de(&etat, &c).await;
            html(
                StatusCode::UNPROCESSABLE_ENTITY,
                pages::nouvelle_entreprise(&c.cadre(None), &revendeurs, Some(&m)),
            )
        }
        Err(e) => page_erreur(Some(&c), &e),
    }
}

async fn fiche(
    State(etat): State<Etat>,
    entetes: HeaderMap,
    uri: Uri,
    Path(id): Path<String>,
) -> Response {
    let c = match exiger(&etat, &entetes, None).await {
        Ok(c) => c,
        Err(r) => return *r,
    };
    let portail = etat.portail.clone();
    let compte = c.compte.clone();
    match bloquant(move || portail.fiche(&compte, &id)).await {
        Ok(f) => {
            let m = message_de(&uri);
            html(
                StatusCode::OK,
                pages::fiche_entreprise(&c.cadre(m.as_ref()), &f),
            )
        }
        Err(e) => page_erreur(Some(&c), &e),
    }
}

// ----- Paiements -----

async fn page_paiement(
    State(etat): State<Etat>,
    entetes: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let c = match exiger(&etat, &entetes, None).await {
        Ok(c) => c,
        Err(r) => return *r,
    };
    let portail = etat.portail.clone();
    let compte = c.compte.clone();
    match bloquant(move || portail.entreprise(&compte, &id)).await {
        Ok(e) => html(
            StatusCode::OK,
            pages::nouveau_paiement(&c.cadre(None), &e, None),
        ),
        Err(e) => page_erreur(Some(&c), &e),
    }
}

async fn paiement(
    State(etat): State<Etat>,
    entetes: HeaderMap,
    Path(id): Path<String>,
    Form(champs): Form<Champs>,
) -> Response {
    let c = match exiger(&etat, &entetes, Some(&champs)).await {
        Ok(c) => c,
        Err(r) => return *r,
    };
    let saisie = NouveauPaiement {
        recu_le: champ(&champs, "recu_le").into(),
        mode: champ(&champs, "mode").into(),
        montant: champ(&champs, "montant").into(),
        devise: champ(&champs, "devise").into(),
        reference: champ(&champs, "reference").into(),
        postes: champ(&champs, "postes").into(),
        formule: champ(&champs, "formule").into(),
    };
    let portail = etat.portail.clone();
    let compte = c.compte.clone();
    let entreprise = id.clone();
    match bloquant(move || portail.enregistrer_paiement(&compte, &entreprise, &saisie)).await {
        Ok(p) => rediriger(&format!("/entreprises/{}?m=paiement", p.entreprise)),
        Err(ErreurPortail::Validation(m)) => {
            let portail = etat.portail.clone();
            let compte = c.compte.clone();
            match bloquant(move || portail.entreprise(&compte, &id)).await {
                Ok(e) => html(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    pages::nouveau_paiement(&c.cadre(None), &e, Some(&m)),
                ),
                Err(e) => page_erreur(Some(&c), &e),
            }
        }
        Err(e) => page_erreur(Some(&c), &e),
    }
}

// ----- Émission -----

async fn page_emission(
    State(etat): State<Etat>,
    entetes: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    let c = match exiger(&etat, &entetes, None).await {
        Ok(c) => c,
        Err(r) => return *r,
    };
    emission_formulaire(&etat, &c, id, None, StatusCode::OK).await
}

async fn emission_formulaire(
    etat: &Etat,
    c: &Connecte,
    id: i64,
    erreur: Option<String>,
    statut: StatusCode,
) -> Response {
    let portail = etat.portail.clone();
    let compte = c.compte.clone();
    let lu = bloquant(move || {
        let p = portail.paiement(&compte, id)?;
        let e = portail.entreprise(&compte, &p.entreprise)?;
        let prop = portail.proposition(&compte, id)?;
        Ok::<_, ErreurPortail>((e, p, prop))
    })
    .await;
    match lu {
        Ok((_, p, _)) if p.a_servi() && erreur.is_none() => match (p.licence, p.code) {
            (Some(l), _) => rediriger(&format!("/licences/{l}")),
            (None, k) => rediriger(&format!("/codes/{}", k.unwrap_or_default())),
        },
        Ok((e, p, prop)) => html(
            statut,
            pages::emission(&c.cadre(None), &e, &p, &prop, erreur.as_deref()),
        ),
        Err(e) => page_erreur(Some(c), &e),
    }
}

async fn emission(
    State(etat): State<Etat>,
    entetes: HeaderMap,
    Path(id): Path<i64>,
    Form(champs): Form<Champs>,
) -> Response {
    let c = match exiger(&etat, &entetes, Some(&champs)).await {
        Ok(c) => c,
        Err(r) => return *r,
    };
    let modules: Vec<Module> = Module::TOUS
        .into_iter()
        .filter(|m| champ(&champs, &format!("module_{}", m.code())) == "1")
        .collect();
    let portail = etat.portail.clone();
    let compte = c.compte.clone();
    // Code en clair rendu une seule fois : la page s'affiche ici, sans
    // redirection, et n'est pas gardée par le navigateur (Cache-Control).
    match bloquant(move || portail.creer_code(&compte, id, &modules)).await {
        Ok(cree) => {
            let mut r = html(StatusCode::OK, pages::code_cree(&c.cadre(None), &cree));
            r.headers_mut()
                .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
            r
        }
        Err(ErreurPortail::Validation(m)) => {
            emission_formulaire(&etat, &c, id, Some(m), StatusCode::UNPROCESSABLE_ENTITY).await
        }
        Err(e) => page_erreur(Some(&c), &e),
    }
}

async fn licence(
    State(etat): State<Etat>,
    entetes: HeaderMap,
    uri: Uri,
    Path(id): Path<i64>,
) -> Response {
    let c = match exiger(&etat, &entetes, None).await {
        Ok(c) => c,
        Err(r) => return *r,
    };
    let portail = etat.portail.clone();
    let compte = c.compte.clone();
    match bloquant(move || portail.licence(&compte, id)).await {
        Ok(l) => {
            let m = message_de(&uri);
            html(StatusCode::OK, pages::licence(&c.cadre(m.as_ref()), &l))
        }
        Err(e) => page_erreur(Some(&c), &e),
    }
}

async fn code(
    State(etat): State<Etat>,
    entetes: HeaderMap,
    uri: Uri,
    Path(id): Path<i64>,
) -> Response {
    let c = match exiger(&etat, &entetes, None).await {
        Ok(c) => c,
        Err(r) => return *r,
    };
    let portail = etat.portail.clone();
    let compte = c.compte.clone();
    match bloquant(move || portail.code(&compte, id)).await {
        Ok(k) => {
            let m = message_de(&uri);
            html(StatusCode::OK, pages::code(&c.cadre(m.as_ref()), &k))
        }
        Err(e) => page_erreur(Some(&c), &e),
    }
}

async fn annuler_code(
    State(etat): State<Etat>,
    entetes: HeaderMap,
    Path(id): Path<i64>,
    Form(champs): Form<Champs>,
) -> Response {
    let c = match exiger(&etat, &entetes, Some(&champs)).await {
        Ok(c) => c,
        Err(r) => return *r,
    };
    let portail = etat.portail.clone();
    let compte = c.compte.clone();
    match bloquant(move || portail.annuler_code(&compte, id)).await {
        Ok(k) => rediriger(&format!("/codes/{}?m=code_annule", k.id)),
        Err(e) => page_erreur(Some(&c), &e),
    }
}

async fn codes_page(etat: &Etat, c: &Connecte, erreur: Option<String>) -> Response {
    let portail = etat.portail.clone();
    let compte = c.compte.clone();
    match bloquant(move || portail.codes_a_vendre(&compte)).await {
        Ok(liste) => {
            let statut = if erreur.is_some() {
                StatusCode::UNPROCESSABLE_ENTITY
            } else {
                StatusCode::OK
            };
            html(
                statut,
                pages::codes(&c.cadre(None), &liste, erreur.as_deref()),
            )
        }
        Err(e) => page_erreur(Some(c), &e),
    }
}

async fn page_codes(State(etat): State<Etat>, entetes: HeaderMap) -> Response {
    let c = match exiger(&etat, &entetes, None).await {
        Ok(c) => c,
        Err(r) => return *r,
    };
    codes_page(&etat, &c, None).await
}

async fn liberer_poste(
    State(etat): State<Etat>,
    entetes: HeaderMap,
    Path(id): Path<i64>,
    Form(champs): Form<Champs>,
) -> Response {
    let c = match exiger(&etat, &entetes, Some(&champs)).await {
        Ok(c) => c,
        Err(r) => return *r,
    };
    let portail = etat.portail.clone();
    let compte = c.compte.clone();
    match bloquant(move || portail.liberer_poste(&compte, id)).await {
        Ok(k) => rediriger(&format!("/codes/{}?m=poste_libere", k.id)),
        Err(e) => page_erreur(Some(&c), &e),
    }
}

async fn tirer_codes(
    State(etat): State<Etat>,
    entetes: HeaderMap,
    Form(champs): Form<Champs>,
) -> Response {
    let c = match exiger(&etat, &entetes, Some(&champs)).await {
        Ok(c) => c,
        Err(r) => return *r,
    };
    let modules: Vec<Module> = Module::TOUS
        .into_iter()
        .filter(|m| champ(&champs, &format!("module_{}", m.code())) == "1")
        .collect();
    let (gamme, nombre, postes, formule) = (
        champ(&champs, "gamme").to_string(),
        champ(&champs, "nombre").to_string(),
        champ(&champs, "postes").to_string(),
        champ(&champs, "formule").to_string(),
    );
    let gamme = if gamme.is_empty() {
        "entreprise".to_string()
    } else {
        gamme
    };
    let portail = etat.portail.clone();
    let compte = c.compte.clone();
    match bloquant(move || {
        portail.creer_codes_gamme(&compte, &gamme, &nombre, &postes, &formule, &modules)
    })
    .await
    {
        Ok(codes) => {
            let mut r = html(StatusCode::OK, pages::codes_crees(&c.cadre(None), &codes));
            r.headers_mut()
                .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
            r
        }
        Err(ErreurPortail::Validation(m)) => codes_page(&etat, &c, Some(m)).await,
        Err(e) => page_erreur(Some(&c), &e),
    }
}

// ----- Activation (programme du client) -----

fn json(statut: StatusCode, valeur: serde_json::Value) -> Response {
    let mut r = (statut, valeur.to_string()).into_response();
    let entetes = r.headers_mut();
    entetes.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    entetes.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    r
}

/// Refus en JSON : un modèle fixe, traduit par le programme du client, et ses
/// valeurs.
fn refus_json(statut: StatusCode, modele: &str, valeurs: &[String]) -> Response {
    json(
        statut,
        serde_json::json!({ "erreur": modele, "valeurs": valeurs }),
    )
}

async fn activation(
    State(etat): State<Etat>,
    ConnectInfo(pair): ConnectInfo<SocketAddr>,
    entetes: HeaderMap,
    corps: Bytes,
) -> Response {
    let Ok(valeur) = serde_json::from_slice::<serde_json::Value>(&corps) else {
        return refus_json(StatusCode::BAD_REQUEST, MODELES_ACTIVATION[4], &[]);
    };
    let texte = |nom: &str| {
        valeur
            .get(nom)
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    let demande = DemandeActivation {
        code: texte("code"),
        entreprise: texte("entreprise"),
        client: texte("client"),
        version: texte("version"),
        poste: texte("poste"),
        produit: texte("produit"),
    };
    let adresse = adresse_client(&etat, &entetes, pair);
    let portail = etat.portail.clone();
    match bloquant(move || portail.activer(&demande, &adresse)).await {
        Ok(a) => json(
            StatusCode::OK,
            serde_json::json!({
                "licence": a.licence,
                "postes": a.postes,
                "formule": a.formule.code(),
                "debut": a.debut,
                "echeance": a.echeance,
            }),
        ),
        Err(ErreurPortail::Validation(m)) => refus_json(StatusCode::UNPROCESSABLE_ENTITY, &m, &[]),
        Err(ErreurPortail::Freine(s)) => refus_json(
            StatusCode::TOO_MANY_REQUESTS,
            MODELES_ACTIVATION[2],
            &[s.to_string()],
        ),
        Err(e) => {
            eprintln!("activation : {e}");
            refus_json(
                StatusCode::INTERNAL_SERVER_ERROR,
                MODELES_ACTIVATION[3],
                &[],
            )
        }
    }
}

/// Essai gratuit demandé par le programme (décision 78) : JSON en entrée,
/// attestation signée et date de début en sortie.
async fn essai(
    State(etat): State<Etat>,
    ConnectInfo(pair): ConnectInfo<SocketAddr>,
    entetes: HeaderMap,
    corps: Bytes,
) -> Response {
    let Ok(valeur) = serde_json::from_slice::<serde_json::Value>(&corps) else {
        return refus_json(StatusCode::BAD_REQUEST, crate::MODELES_ESSAI[0], &[]);
    };
    let texte = |nom: &str| {
        valeur
            .get(nom)
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    let demande = DemandeEssai {
        produit: texte("produit"),
        poste: texte("poste"),
        version: texte("version"),
    };
    let adresse = adresse_client(&etat, &entetes, pair);
    let portail = etat.portail.clone();
    match bloquant(move || portail.demander_essai(&demande, &adresse)).await {
        Ok(e) => json(
            StatusCode::OK,
            serde_json::json!({ "attestation": e.attestation, "debut": e.debut }),
        ),
        Err(ErreurPortail::Validation(m)) => refus_json(StatusCode::UNPROCESSABLE_ENTITY, &m, &[]),
        Err(ErreurPortail::Freine(s)) => refus_json(
            StatusCode::TOO_MANY_REQUESTS,
            MODELES_ACTIVATION[2],
            &[s.to_string()],
        ),
        Err(e) => {
            eprintln!("essai : {e}");
            refus_json(
                StatusCode::INTERNAL_SERVER_ERROR,
                MODELES_ACTIVATION[3],
                &[],
            )
        }
    }
}

// ----- Comptes -----

async fn comptes_page(
    etat: &Etat,
    c: &Connecte,
    message: Option<Message>,
    erreur: Option<String>,
) -> Response {
    let portail = etat.portail.clone();
    let compte = c.compte.clone();
    match bloquant(move || portail.comptes(&compte)).await {
        Ok(liste) => {
            let statut = if erreur.is_some() {
                StatusCode::UNPROCESSABLE_ENTITY
            } else {
                StatusCode::OK
            };
            html(
                statut,
                pages::comptes(&c.cadre(message.as_ref()), &liste, erreur.as_deref()),
            )
        }
        Err(e) => page_erreur(Some(c), &e),
    }
}

async fn page_comptes(State(etat): State<Etat>, entetes: HeaderMap, uri: Uri) -> Response {
    let c = match exiger(&etat, &entetes, None).await {
        Ok(c) => c,
        Err(r) => return *r,
    };
    comptes_page(&etat, &c, message_de(&uri), None).await
}

async fn nouveau_compte(
    State(etat): State<Etat>,
    entetes: HeaderMap,
    Form(champs): Form<Champs>,
) -> Response {
    let c = match exiger(&etat, &entetes, Some(&champs)).await {
        Ok(c) => c,
        Err(r) => return *r,
    };
    let portail = etat.portail.clone();
    let compte = c.compte.clone();
    let identifiant = champ(&champs, "identifiant").to_string();
    let nom = champ(&champs, "nom").to_string();
    let mot_de_passe = champ(&champs, "mot_de_passe").to_string();
    match bloquant(move || portail.creer_revendeur(&compte, &identifiant, &nom, &mot_de_passe))
        .await
    {
        Ok(_) => rediriger("/comptes?m=compte"),
        Err(ErreurPortail::Validation(m)) => comptes_page(&etat, &c, None, Some(m)).await,
        Err(e) => page_erreur(Some(&c), &e),
    }
}

async fn regler_actif(
    State(etat): State<Etat>,
    entetes: HeaderMap,
    Path(id): Path<i64>,
    Form(champs): Form<Champs>,
) -> Response {
    let c = match exiger(&etat, &entetes, Some(&champs)).await {
        Ok(c) => c,
        Err(r) => return *r,
    };
    let actif = champ(&champs, "actif") == "1";
    let portail = etat.portail.clone();
    let compte = c.compte.clone();
    match bloquant(move || portail.regler_actif(&compte, id, actif)).await {
        Ok(_) => rediriger("/comptes?m=actif"),
        Err(ErreurPortail::Validation(m)) => comptes_page(&etat, &c, None, Some(m)).await,
        Err(e) => page_erreur(Some(&c), &e),
    }
}

async fn page_mot_de_passe(State(etat): State<Etat>, entetes: HeaderMap) -> Response {
    let c = match exiger(&etat, &entetes, None).await {
        Ok(c) => c,
        Err(r) => return *r,
    };
    html(StatusCode::OK, pages::mot_de_passe(&c.cadre(None), None))
}

async fn mot_de_passe(
    State(etat): State<Etat>,
    entetes: HeaderMap,
    Form(champs): Form<Champs>,
) -> Response {
    let c = match exiger(&etat, &entetes, Some(&champs)).await {
        Ok(c) => c,
        Err(r) => return *r,
    };
    let portail = etat.portail.clone();
    let compte = c.compte.clone();
    let ancien = champ(&champs, "ancien").to_string();
    let nouveau = champ(&champs, "nouveau").to_string();
    let confirmation = champ(&champs, "confirmation").to_string();
    match bloquant(move || portail.changer_mot_de_passe(&compte, &ancien, &nouveau, &confirmation))
        .await
    {
        // Les autres sessions du compte sont fermées ; celle-ci est remplacée.
        Ok(jeton) => (
            StatusCode::SEE_OTHER,
            [
                (LOCATION, "/?m=mot_de_passe".to_string()),
                (SET_COOKIE, cookie_session(&jeton)),
            ],
        )
            .into_response(),
        Err(ErreurPortail::Validation(m)) => html(
            StatusCode::UNPROCESSABLE_ENTITY,
            pages::mot_de_passe(&c.cadre(None), Some(&m)),
        ),
        Err(e) => page_erreur(Some(&c), &e),
    }
}

async fn journal(State(etat): State<Etat>, entetes: HeaderMap) -> Response {
    let c = match exiger(&etat, &entetes, None).await {
        Ok(c) => c,
        Err(r) => return *r,
    };
    let portail = etat.portail.clone();
    let compte = c.compte.clone();
    let lu = bloquant(move || {
        let actions = portail.journal(&compte)?;
        Ok::<_, ErreurPortail>((actions, portail.chaine_du_journal(&compte)?))
    })
    .await;
    match lu {
        Ok((actions, chaine)) => html(
            StatusCode::OK,
            pages::journal(&c.cadre(None), &actions, chaine.as_ref()),
        ),
        Err(e) => page_erreur(Some(&c), &e),
    }
}

// ----- Téléchargements (décision 77) -----

/// Page « Votre téléchargement va démarrer » : dernier paquet de l'édition,
/// lien signé de dix minutes, freiné par adresse.
async fn page_telechargement(
    State(etat): State<Etat>,
    ConnectInfo(pair): ConnectInfo<SocketAddr>,
    entetes: HeaderMap,
    Path(produit): Path<String>,
) -> Response {
    let produit = match produit.as_str() {
        "entreprise" => kadenz_licence::entreprise::Produit::Entreprise,
        "monoposte" => kadenz_licence::entreprise::Produit::Monoposte,
        _ => return page_erreur(None, &ErreurPortail::Introuvable),
    };
    let Some(depot) = etat.config.depot.clone() else {
        return page_erreur(None, &ErreurPortail::Introuvable);
    };
    let Some(nom) = bloquant(move || telechargement::dernier(&depot, produit)).await else {
        return html(
            StatusCode::NOT_FOUND,
            pages::erreur(
                None,
                404,
                "Aucun installateur n'est disponible pour le moment. Réessayez plus tard.",
            ),
        );
    };
    let adresse = adresse_client(&etat, &entetes, pair);
    let portail = etat.portail.clone();
    let pour_journal = nom.clone();
    match bloquant(move || portail.demander_telechargement(&pour_journal, &adresse)).await {
        Ok(None) => {}
        Ok(Some(attente)) => {
            let minutes = attente.div_ceil(60).max(1);
            return html(
                StatusCode::TOO_MANY_REQUESTS,
                pages::erreur(
                    None,
                    429,
                    &format!(
                        "Trop de téléchargements depuis votre adresse : réessayez dans {minutes} minute(s)."
                    ),
                ),
            );
        }
        Err(e) => return page_erreur(None, &e),
    }
    let lien = etat.signataire.lien(&nom, crate::secondes_publiques());
    let edition = match produit {
        kadenz_licence::entreprise::Produit::Entreprise => "Kadenz Entreprise",
        kadenz_licence::entreprise::Produit::Monoposte => "Kadenz",
    };
    html(StatusCode::OK, pages::telechargement(edition, &nom, &lien))
}

/// Versions publiées, au format de la liste des releases de GitHub que lit le
/// client de mise à jour (`kadenz_maj::choisir`) : liens signés et absolus,
/// manifeste et signature compris. Sans dépôt ou sans adresse publique : 404.
async fn releases(State(etat): State<Etat>) -> Response {
    let (Some(depot), Some(base)) = (
        etat.config.depot.clone(),
        etat.config.adresse_publique.clone(),
    ) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let liste = bloquant(move || telechargement::publications(&depot, 30)).await;
    let maintenant = crate::secondes_publiques();
    let signataire = &etat.signataire;
    let releases: Vec<serde_json::Value> = liste
        .into_iter()
        .map(|p| {
            let (a, b, c) = p.version;
            let actifs: Vec<serde_json::Value> = p
                .fichiers
                .iter()
                .map(|(sorte, nom)| {
                    // Le client cherche le manifeste et sa signature sous leur nom
                    // de release ; le dépôt les garde avec la version.
                    let nom_release = match sorte {
                        telechargement::Sorte::Manifeste => kadenz_maj_noms::MANIFESTE,
                        telechargement::Sorte::Signature => kadenz_maj_noms::SIGNATURE,
                        _ => nom.as_str(),
                    };
                    serde_json::json!({
                        "name": nom_release,
                        "browser_download_url": format!("{base}{}", signataire.lien(nom, maintenant)),
                    })
                })
                .collect();
            serde_json::json!({
                "tag_name": format!("v{a}.{b}.{c}"),
                "draft": false,
                "prerelease": false,
                "html_url": format!("{base}/telecharger/entreprise"),
                "body": p.notes.unwrap_or_default(),
                "assets": actifs,
            })
        })
        .collect();
    json(StatusCode::OK, serde_json::Value::Array(releases))
}

/// Noms de release du manifeste et de sa signature, ceux que cherche le client
/// (`kadenz_maj::MANIFESTE`, `kadenz_maj::SIGNATURE`).
mod kadenz_maj_noms {
    pub const MANIFESTE: &str = "kadenz-maj.json";
    pub const SIGNATURE: &str = "kadenz-maj.json.sig";
}

/// Paquet servi par un lien signé, en flux, avec la reprise (`Range`).
async fn fichier(
    State(etat): State<Etat>,
    Path(nom): Path<String>,
    uri: Uri,
    entetes: HeaderMap,
) -> Response {
    let refus = || StatusCode::NOT_FOUND.into_response();
    let mut echeance = None;
    let mut signature = None;
    for (cle, valeur) in uri
        .query()
        .unwrap_or("")
        .split('&')
        .filter_map(|p| p.split_once('='))
    {
        match cle {
            "exp" => echeance = valeur.parse::<i64>().ok(),
            "sig" => signature = Some(valeur.to_string()),
            _ => {}
        }
    }
    let (Some(echeance), Some(signature), Some(depot)) =
        (echeance, signature, etat.config.depot.clone())
    else {
        return refus();
    };
    if !etat
        .signataire
        .verifier(&nom, echeance, &signature, crate::secondes_publiques())
    {
        return StatusCode::FORBIDDEN.into_response();
    }
    // Le nom suit le modèle d'un paquet (vérifié par la signature) : aucun
    // séparateur de chemin n'y passe.
    let chemin = depot.join(&nom);
    let Ok(mut fichier) = tokio::fs::File::open(&chemin).await else {
        return refus();
    };
    let Ok(meta) = fichier.metadata().await else {
        return refus();
    };
    if !meta.is_file() {
        return refus();
    }
    let taille = meta.len();
    let demande = entetes.get(RANGE).and_then(|v| v.to_str().ok());
    let (statut, debut, fin) = match telechargement::plage(demande, taille) {
        Plage::Tout => (StatusCode::OK, 0, taille.saturating_sub(1)),
        Plage::Partie(d, f) => (StatusCode::PARTIAL_CONTENT, d, f),
        Plage::Hors => {
            return (
                StatusCode::RANGE_NOT_SATISFIABLE,
                [(CONTENT_RANGE, format!("bytes */{taille}"))],
            )
                .into_response();
        }
    };
    let longueur = if taille == 0 { 0 } else { fin - debut + 1 };
    if debut > 0 {
        use tokio::io::AsyncSeekExt;
        if fichier.seek(std::io::SeekFrom::Start(debut)).await.is_err() {
            return refus();
        }
    }
    let flux = futures_util::stream::try_unfold((fichier, longueur), |(mut f, reste)| async move {
        use tokio::io::AsyncReadExt;
        if reste == 0 {
            return Ok::<_, std::io::Error>(None);
        }
        let mut tampon = vec![0u8; reste.min(64 * 1024) as usize];
        let lus = f.read(&mut tampon).await?;
        if lus == 0 {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
        }
        tampon.truncate(lus);
        Ok(Some((Bytes::from(tampon), (f, reste - lus as u64))))
    });
    let mut reponse = Response::new(axum::body::Body::from_stream(flux));
    *reponse.status_mut() = statut;
    let h = reponse.headers_mut();
    h.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    h.insert(CONTENT_LENGTH, HeaderValue::from(longueur));
    h.insert(ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    h.insert(CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
    if let Ok(v) = HeaderValue::from_str(&format!("attachment; filename=\"{nom}\"")) {
        h.insert(CONTENT_DISPOSITION, v);
    }
    if statut == StatusCode::PARTIAL_CONTENT
        && let Ok(v) = HeaderValue::from_str(&format!("bytes {debut}-{fin}/{taille}"))
    {
        h.insert(CONTENT_RANGE, v);
    }
    reponse
}

async fn introuvable(State(etat): State<Etat>, entetes: HeaderMap) -> Response {
    let c = connecte(&etat, &entetes).await;
    page_erreur(c.as_ref(), &ErreurPortail::Introuvable)
}

// ----- Lancement -----

/// Portail lancé dans son propre fil, avec son exécuteur tokio.
pub struct Lance {
    adresse: SocketAddr,
    arret: Option<tokio::sync::oneshot::Sender<()>>,
    fil: Option<JoinHandle<()>>,
}

impl Lance {
    pub fn adresse(&self) -> SocketAddr {
        self.adresse
    }

    /// Arrête le portail et attend la fin des requêtes en cours.
    pub fn arreter(mut self) {
        self.fermer();
    }

    /// Attend la fin du portail (programme principal : jusqu'à la fin du
    /// processus).
    pub fn attendre(mut self) {
        if let Some(fil) = self.fil.take() {
            let _ = fil.join();
        }
    }

    fn fermer(&mut self) {
        if let Some(arret) = self.arret.take() {
            let _ = arret.send(());
        }
        if let Some(fil) = self.fil.take() {
            let _ = fil.join();
        }
    }
}

impl Drop for Lance {
    fn drop(&mut self) {
        self.fermer();
    }
}

/// Lance le portail sur cette adresse (port 0 : port libre choisi par le
/// système).
pub fn lancer(portail: Portail, ecoute: SocketAddr, config: ConfigWeb) -> std::io::Result<Lance> {
    let ecouteur = StdListener::bind(ecoute)?;
    ecouteur.set_nonblocking(true)?;
    let adresse = ecouteur.local_addr()?;
    let execution = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_io()
        .build()?;
    let (arret, signal) = tokio::sync::oneshot::channel::<()>();
    let routes = routeur(portail, config);
    let fil = std::thread::Builder::new()
        .name("portail".into())
        .spawn(move || {
            execution.block_on(async move {
                let Ok(ecouteur) = tokio::net::TcpListener::from_std(ecouteur) else {
                    return;
                };
                let _ = axum::serve(
                    ecouteur,
                    routes.into_make_service_with_connect_info::<SocketAddr>(),
                )
                .with_graceful_shutdown(async move {
                    let _ = signal.await;
                })
                .await;
            });
        })?;
    Ok(Lance {
        adresse,
        arret: Some(arret),
        fil: Some(fil),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adresse_publique_admise() {
        assert_eq!(
            adresse_publique("https://licences.exemple.fr/").as_deref(),
            Some("https://licences.exemple.fr")
        );
        assert_eq!(
            adresse_publique("https://hote:8443").as_deref(),
            Some("https://hote:8443")
        );
        for refusee in [
            "http://licences.exemple.fr",
            "https://",
            "https://licences.exemple.fr/chemin",
            "https://a\"b",
            "https://a b",
            "licences.exemple.fr",
        ] {
            assert_eq!(adresse_publique(refusee), None, "{refusee}");
        }
    }

    #[test]
    fn apercu_de_partage_avec_l_adresse_publique() {
        let page =
            pages::connexion("jeton", None, Some("https://licences.exemple.fr")).into_string();
        assert!(page.contains("content=\"noindex, nofollow\""));
        assert!(page.contains(&format!(
            "property=\"og:image\" content=\"https://licences.exemple.fr{}\"",
            pages::IMAGE_PARTAGE
        )));
        assert!(page.contains("content=\"https://licences.exemple.fr/connexion\""));
        assert!(!page.contains("rel=\"canonical\""));
    }

    /// Fichiers servis avec un cache d'un an : le nom porte les 10 premiers
    /// chiffres hexadécimaux du SHA-256 du contenu. Un fichier modifié sans
    /// être renommé resterait périmé chez les navigateurs.
    #[test]
    fn empreinte_des_fichiers_dans_leur_nom() {
        for (chemin, octets) in [
            (pages::SCRIPT_CODE, SCRIPT_CODE),
            (pages::IMAGE_PILIERS, PILIERS),
            (pages::FAVICON, FAVICON),
            (pages::IMAGE_PARTAGE, PARTAGE),
        ] {
            let empreinte = ring::digest::digest(&ring::digest::SHA256, octets);
            let hex: String = empreinte
                .as_ref()
                .iter()
                .map(|o| format!("{o:02x}"))
                .collect();
            let attendu = format!("-{}.", &hex[..10]);
            assert!(chemin.contains(&attendu), "{chemin} : attendu {attendu}");
        }
    }

    fn entetes(valeurs: &[&str]) -> HeaderMap {
        entetes_nommes("x-forwarded-for", valeurs)
    }

    fn entetes_nommes(nom: &'static str, valeurs: &[&str]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for v in valeurs {
            h.append(nom, HeaderValue::from_str(v).unwrap());
        }
        h
    }

    /// `X-Forwarded-For`, en-tête par défaut.
    fn adresse_xff(mandataire: Option<IpAddr>, entetes: &HeaderMap, pair: SocketAddr) -> IpAddr {
        adresse_selon(mandataire, EnteteClient::XForwardedFor, entetes, pair)
    }

    /// Derrière Cloudflare Tunnel : `CF-Connecting-IP`, du mandataire
    /// seulement, valeur unique ; `X-Forwarded-For` n'est plus lu.
    #[test]
    fn cf_connecting_ip_seulement_depuis_le_mandataire() {
        let cf = EnteteClient::CfConnectingIp;
        let mandataire = Some(ip("172.31.0.2"));
        let par_mandataire: SocketAddr = "172.31.0.2:50000".parse().unwrap();
        let direct: SocketAddr = "172.30.0.1:50000".parse().unwrap();
        let un = entetes_nommes("cf-connecting-ip", &["198.51.100.7"]);
        assert_eq!(
            adresse_selon(mandataire, cf, &un, par_mandataire),
            ip("198.51.100.7")
        );
        // IPv6 du client.
        assert_eq!(
            adresse_selon(
                mandataire,
                cf,
                &entetes_nommes("cf-connecting-ip", &["2001:db8::7"]),
                par_mandataire
            ),
            ip("2001:db8::7")
        );
        // Connexion d'une autre adresse (127.0.0.1 publié, passerelle) : ignoré.
        assert_eq!(adresse_selon(mandataire, cf, &un, direct), ip("172.30.0.1"));
        // X-Forwarded-For seul : non lu dans ce mode.
        assert_eq!(
            adresse_selon(mandataire, cf, &entetes(&["198.51.100.7"]), par_mandataire),
            ip("172.31.0.2")
        );
        // Deux en-têtes, une liste, ou une valeur qui n'est pas une adresse :
        // l'adresse de connexion.
        for valeurs in [
            &["198.51.100.7", "203.0.113.1"][..],
            &["198.51.100.7, 203.0.113.1"][..],
            &["n'importe quoi"][..],
        ] {
            assert_eq!(
                adresse_selon(
                    mandataire,
                    cf,
                    &entetes_nommes("cf-connecting-ip", valeurs),
                    par_mandataire
                ),
                ip("172.31.0.2"),
                "{valeurs:?}"
            );
        }
        // Option de la ligne de commande.
        assert_eq!(EnteteClient::depuis_option("cf-connecting-ip"), Some(cf));
        assert_eq!(
            EnteteClient::depuis_option("x-forwarded-for"),
            Some(EnteteClient::XForwardedFor)
        );
        assert_eq!(EnteteClient::depuis_option("CF-Connecting-IP"), None);
        assert_eq!(EnteteClient::default(), EnteteClient::XForwardedFor);
    }

    fn ip(texte: &str) -> IpAddr {
        texte.parse().unwrap()
    }

    #[test]
    fn x_forwarded_for_seulement_depuis_le_mandataire() {
        let mandataire = Some(ip("127.0.0.1"));
        let par_mandataire: SocketAddr = "127.0.0.1:50000".parse().unwrap();
        let direct: SocketAddr = "203.0.113.9:50000".parse().unwrap();
        // Par le mandataire : la dernière valeur, celle qu'il a posée.
        assert_eq!(
            adresse_xff(
                mandataire,
                &entetes(&["1.1.1.1, 198.51.100.7"]),
                par_mandataire
            ),
            ip("198.51.100.7")
        );
        assert_eq!(
            adresse_xff(
                mandataire,
                &entetes(&["1.1.1.1", "198.51.100.7"]),
                par_mandataire
            ),
            ip("198.51.100.7")
        );
        // En-tête forgé par un client qui joint le port directement : ignoré.
        assert_eq!(
            adresse_xff(mandataire, &entetes(&["198.51.100.7"]), direct),
            ip("203.0.113.9")
        );
        // Sans mandataire configuré : toujours l'adresse de connexion.
        assert_eq!(
            adresse_xff(None, &entetes(&["198.51.100.7"]), par_mandataire),
            ip("127.0.0.1")
        );
        // Valeur qui n'est pas une adresse : l'adresse de connexion.
        assert_eq!(
            adresse_xff(mandataire, &entetes(&["n'importe quoi"]), par_mandataire),
            ip("127.0.0.1")
        );
        // Adresse IPv4 vue en IPv6 (écoute double pile) : reconnue.
        let mappe: SocketAddr = "[::ffff:127.0.0.1]:50000".parse().unwrap();
        assert_eq!(
            adresse_xff(mandataire, &entetes(&["198.51.100.7"]), mappe),
            ip("198.51.100.7")
        );
    }

    fn paniquer() -> Response {
        panic!("panique d'essai")
    }

    /// Une route qui panique rend la page d'erreur 500, sans détail.
    #[test]
    fn panique_rendue_en_erreur_500() {
        let execution = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let reponse = execution.block_on(garder(async { paniquer() }));
        assert_eq!(reponse.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let sain = execution.block_on(garder(async { StatusCode::OK.into_response() }));
        assert_eq!(sain.status(), StatusCode::OK);
    }

    /// Chaque code de redirection a son message, poste libéré compris.
    #[test]
    fn message_du_poste_libere() {
        let uri: Uri = "/codes/3?m=poste_libere".parse().unwrap();
        let message = message_de(&uri).expect("message du poste libéré");
        assert!(message.ok);
        assert!(
            message.texte.starts_with("Poste libéré"),
            "{}",
            message.texte
        );
        let inconnu: Uri = "/codes/3?m=autre".parse().unwrap();
        assert!(message_de(&inconnu).is_none());
    }

    #[test]
    fn erreur_interne_sans_detail() {
        let reponse = page_erreur(None, &ErreurPortail::Interne("secret de la base".into()));
        assert_eq!(reponse.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
