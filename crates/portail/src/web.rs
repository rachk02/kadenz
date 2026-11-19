//! Routes du portail (spécification du portail, sections 2 et 6) : pages
//! servies en HTTP par axum, sessions par cookie, jeton anti-falsification sur
//! chaque formulaire, en-têtes de sécurité, aucun script. Une seule route
//! hors de la console : `POST /api/activation` (décision 74), en JSON, sans
//! session ni jeton, appelée par le programme du client.
//!
//! Chaque appel au portail (base SQLite, synchrone) passe par
//! `spawn_blocking`, pour ne jamais bloquer un fil de tokio.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr, TcpListener as StdListener};
use std::thread::JoinHandle;

use axum::Router;
use axum::body::Bytes;
use axum::extract::{ConnectInfo, Form, Path, State};
use axum::http::header::{
    CACHE_CONTROL, CONTENT_TYPE, COOKIE, LOCATION, REFERRER_POLICY, SET_COOKIE,
    X_CONTENT_TYPE_OPTIONS,
};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode, Uri};
use axum::middleware::map_response;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use kadenz_domaine::module::Module;
use maud::Markup;

use crate::pages::{self, Cadre};
use crate::{
    Compte, DemandeActivation, ErreurPortail, MODELES_ACTIVATION, Message, NouveauPaiement, Portail,
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

const POLICE_INTER: &[u8] = include_bytes!("../../app/ui/polices/Inter-Regular.ttf");
const POLICE_INTER_DEMI: &[u8] = include_bytes!("../../app/ui/polices/Inter-SemiBold.ttf");
const POLICE_SPACE: &[u8] = include_bytes!("../../app/ui/polices/SpaceGrotesk-Bold.ttf");
const FAVICON: &[u8] = include_bytes!("../../../docs/marque/web/favicon.svg");
const ROBOTS: &str = "User-agent: *\nDisallow: /\n";

/// Réglages des routes.
#[derive(Clone, Default)]
pub struct ConfigWeb {
    /// Adresse du mandataire de confiance (Caddy, ou tailscaled pour Funnel) :
    /// pour une requête qui vient de lui, et de lui seul, l'adresse du client
    /// est lue dans la dernière valeur de `X-Forwarded-For`, pour freiner par
    /// adresse. Funnel remplace cet en-tête par l'adresse source réelle
    /// (`addProxyForwardedHeaders`, ipn/ipnlocal/serve.go de Tailscale) ;
    /// Caddy y ajoute la sienne en dernier. Une requête d'une autre adresse
    /// garde son adresse de connexion, quel que soit l'en-tête envoyé.
    pub mandataire: Option<IpAddr>,
}

#[derive(Clone)]
struct Etat {
    portail: Portail,
    config: ConfigWeb,
}

type Champs = HashMap<String, String>;

/// Table des routes du portail.
pub fn routeur(portail: Portail, config: ConfigWeb) -> Router {
    Router::new()
        .route("/", get(accueil))
        .route("/connexion", get(page_connexion).post(connexion))
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
        .route("/comptes", get(page_comptes).post(nouveau_compte))
        .route("/comptes/{id}/actif", post(regler_actif))
        .route("/mot-de-passe", get(page_mot_de_passe).post(mot_de_passe))
        .route("/journal", get(journal))
        .route(
            "/style.css",
            get(|| async { statique("text/css; charset=utf-8", pages::STYLE.as_bytes(), false) }),
        )
        .route(
            "/police/inter-regular.ttf",
            get(|| async { statique("font/ttf", POLICE_INTER, true) }),
        )
        .route(
            "/police/inter-semibold.ttf",
            get(|| async { statique("font/ttf", POLICE_INTER_DEMI, true) }),
        )
        .route(
            "/police/space-grotesk-bold.ttf",
            get(|| async { statique("font/ttf", POLICE_SPACE, true) }),
        )
        .route(
            "/favicon.svg",
            get(|| async { statique("image/svg+xml", FAVICON, true) }),
        )
        .route(
            "/robots.txt",
            get(|| async { statique("text/plain; charset=utf-8", ROBOTS.as_bytes(), false) }),
        )
        // Santé (vérification de Docker, « kadenz-portail sante ») : hors des
        // sessions et du frein, ne dit rien d'autre que « ok ».
        .route(
            "/sante",
            get(|| async { statique("text/plain; charset=utf-8", b"ok", false) }),
        )
        .fallback(introuvable)
        .layer(map_response(entetes))
        .with_state(Etat { portail, config })
}

/// En-têtes de sécurité sur chaque réponse.
async fn entetes(mut reponse: Response) -> Response {
    let h = reponse.headers_mut();
    h.insert(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static(
            "default-src 'self'; script-src 'none'; object-src 'none'; form-action 'self'; \
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

/// Session du compte connecté : compte et jeton anti-falsification.
struct Connecte {
    compte: Compte,
    csrf: String,
}

impl Connecte {
    fn cadre<'a>(&'a self, message: Option<&'a Message>) -> Cadre<'a> {
        Cadre {
            compte: &self.compte,
            csrf: &self.csrf,
            message,
        }
    }
}

async fn connecte(etat: &Etat, entetes: &HeaderMap) -> Option<Connecte> {
    let jeton = jeton_du_cookie(entetes)?;
    let portail = etat.portail.clone();
    let (compte, csrf) = bloquant(move || portail.session(&jeton)).await?;
    Some(Connecte { compte, csrf })
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
async fn exiger(
    etat: &Etat,
    entetes: &HeaderMap,
    champs: Option<&Champs>,
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
    adresse_selon(etat.config.mandataire, entetes, pair).to_string()
}

/// Adresse du client : la dernière valeur de `X-Forwarded-For`, seulement si
/// la connexion vient du mandataire de confiance et que la valeur est une
/// adresse IP ; sinon l'adresse de connexion.
fn adresse_selon(mandataire: Option<IpAddr>, entetes: &HeaderMap, pair: SocketAddr) -> IpAddr {
    let pair_ip = pair.ip().to_canonical();
    if mandataire.map(|m| m.to_canonical()) == Some(pair_ip)
        && let Some(transmise) = entetes
            .get_all("x-forwarded-for")
            .iter()
            .filter_map(|v| v.to_str().ok())
            .next_back()
            .and_then(|v| v.rsplit(',').next())
            .and_then(|v| v.trim().parse::<IpAddr>().ok())
    {
        return transmise.to_canonical();
    }
    pair_ip
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
    let jeton = crate::sessions::alea();
    let mut r = html(statut, pages::connexion(&jeton, erreur));
    if let Ok(v) = HeaderValue::from_str(&format!(
        "{COOKIE_CONNEXION}={jeton}; Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age={DUREE_JETON_CONNEXION}"
    )) {
        r.headers_mut().append(SET_COOKIE, v);
    }
    r
}

/// Demande de connexion légitime : envoyée depuis le portail lui-même (pas
/// de `Sec-Fetch-Site: cross-site` ni `same-site`), avec le jeton du
/// formulaire égal à celui du cookie.
fn connexion_permise(entetes: &HeaderMap, champs: &Champs) -> bool {
    let site = entetes
        .get("sec-fetch-site")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if site == "cross-site" || site == "same-site" {
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
    match bloquant(move || portail.entreprises(&compte)).await {
        Ok(liste) => {
            let m = message_de(&uri);
            html(
                StatusCode::OK,
                pages::entreprises(&c.cadre(m.as_ref()), &liste),
            )
        }
        Err(e) => page_erreur(Some(&c), &e),
    }
}

async fn page_connexion(State(etat): State<Etat>, entetes: HeaderMap) -> Response {
    if connecte(&etat, &entetes).await.is_some() {
        return rediriger("/");
    }
    page_de_connexion(StatusCode::OK, None)
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
        Err(e @ (ErreurPortail::Identifiants | ErreurPortail::Freine(_))) => {
            page_de_connexion(StatusCode::UNAUTHORIZED, Some(&e.to_string()))
        }
        Err(e) => page_erreur(None, &e),
    }
}

async fn deconnexion(
    State(etat): State<Etat>,
    entetes: HeaderMap,
    Form(champs): Form<Champs>,
) -> Response {
    if exiger(&etat, &entetes, Some(&champs)).await.is_ok()
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
    match bloquant(move || portail.journal(&compte)).await {
        Ok(actions) => html(StatusCode::OK, pages::journal(&c.cadre(None), &actions)),
        Err(e) => page_erreur(Some(&c), &e),
    }
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

    fn entetes(valeurs: &[&str]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for v in valeurs {
            h.append("x-forwarded-for", HeaderValue::from_str(v).unwrap());
        }
        h
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
            adresse_selon(
                mandataire,
                &entetes(&["1.1.1.1, 198.51.100.7"]),
                par_mandataire
            ),
            ip("198.51.100.7")
        );
        assert_eq!(
            adresse_selon(
                mandataire,
                &entetes(&["1.1.1.1", "198.51.100.7"]),
                par_mandataire
            ),
            ip("198.51.100.7")
        );
        // En-tête forgé par un client qui joint le port directement : ignoré.
        assert_eq!(
            adresse_selon(mandataire, &entetes(&["198.51.100.7"]), direct),
            ip("203.0.113.9")
        );
        // Sans mandataire configuré : toujours l'adresse de connexion.
        assert_eq!(
            adresse_selon(None, &entetes(&["198.51.100.7"]), par_mandataire),
            ip("127.0.0.1")
        );
        // Valeur qui n'est pas une adresse : l'adresse de connexion.
        assert_eq!(
            adresse_selon(mandataire, &entetes(&["n'importe quoi"]), par_mandataire),
            ip("127.0.0.1")
        );
        // Adresse IPv4 vue en IPv6 (écoute double pile) : reconnue.
        let mappe: SocketAddr = "[::ffff:127.0.0.1]:50000".parse().unwrap();
        assert_eq!(
            adresse_selon(mandataire, &entetes(&["198.51.100.7"]), mappe),
            ip("198.51.100.7")
        );
    }

    #[test]
    fn erreur_interne_sans_detail() {
        let reponse = page_erreur(None, &ErreurPortail::Interne("secret de la base".into()));
        assert_eq!(reponse.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
