"""Gabarit animé des pages du compte (demande de l'utilisateur du 2026-11-21 :
« utilise complètement le template Entreprises »).

Copie dans crates/portail/statique/gabarit/, avec une empreinte dans le nom :
- la feuille du gabarit (feuilles de la maquette docs_of_interest/dd/construction,
  dans l'ordre de sa page, polices de titre ramenées à Inter) ;
- les scripts de la maquette (couverture, transitions, motifs) ;
- GSAP, ScrollTrigger, SplitText et Lenis en fichiers officiels de leur
  distribution npm, inchangés (en-têtes d'origine) ;
- coque.js : les modules de l'origine dont la coque a besoin (méga-menu, menu
  mobile, indicateur des pilules, couleur de la barre, apparitions), pris tels
  quels dans le paquet de la maquette et reliés aux globales officielles ;
- Geist Mono et sa licence.
Écrit src/pages/gabarit.rs (adresses, fichiers servis, fragments du balisage).

Usage (Python 3.13) :
  py -3.13 -I generer_gabarit.py <maquette>/construction <dossier gsap> <dossier lenis>
où <dossier gsap> et <dossier lenis> sont les dossiers `package` des archives
npm gsap-3.13.0.tgz et lenis-1.3.4.tgz.
"""
import hashlib
import re
import sys
from pathlib import Path

ICI = Path(__file__).resolve().parent
CAISSE = ICI.parent
SORTIE = CAISSE / "statique" / "gabarit"
RUST = CAISSE / "src" / "pages" / "gabarit.rs"

MAQUETTE, GSAP, LENIS = (Path(a) for a in sys.argv[1:4])
GAB = MAQUETTE / "sortie" / "v2" / "gabarit"
PAGE = (MAQUETTE / "sortie" / "v2" / "entreprises.html").read_text(encoding="utf-8")

CSS_ORDRE = ["polices.css", "base.css", "Account.--NqM65R.css", "AccountWindow.hKb1PI0Q.css",
             "CalmOrder.B9oN5r7u.css", "CanvasCard.DEs0z1H6.css", "FeatureCard.CkUS_mVw.css",
             "Footer.DvTzYVvB.css", "Nav.CDSpRlyR.css", "index.BcLLkTWf.css", "sections.css",
             "attributs.css", "portail.css"]

SURCHARGE = """
/* Portail : polices de titre et de texte en Inter (servie par le portail) ;
   images de l'origine absentes du portail retirées. */
:root{--font-heading:"Inter",Arial,sans-serif;--font-body:"Inter",Arial,sans-serif}
"""

fichiers = []  # (constante, adresse, type, nom de fichier)


def empreinte(octets: bytes) -> str:
    return hashlib.sha256(octets).hexdigest()[:10]


def poser(constante: str, nom: str, ext: str, octets: bytes, type_contenu: str) -> str:
    fichier = f"{nom}-{empreinte(octets)}.{ext}"
    (SORTIE / fichier).write_bytes(octets)
    adresse = f"/gabarit/{fichier}"
    fichiers.append((constante, adresse, type_contenu, fichier))
    return adresse


def compter(texte: str, ancien: str, nouveau: str, n: int = 1) -> str:
    assert texte.count(ancien) == n, (ancien, texte.count(ancien))
    return texte.replace(ancien, nouveau)


def modules(paquet: str) -> dict:
    """Modules du paquet, par nom (le dernier l'emporte si un nom revient)."""
    parts = re.split(r"\n  // ([A-Za-z0-9_.\-]+\.js)\n", paquet)
    return {parts[i]: parts[i + 1] for i in range(1, len(parts), 2)}, parts


# Modules des sections de la page Entreprises (hero, grille, dérive, détails, bandeau).
SECTIONS = ["odometer.B1bsPzO3.js", "img-fade.DXLtvv7E.js",
            "Hero.astro_astro_type_script_index_0_lang.CTRBnflW.js",
            "Slider.astro_astro_type_script_index_0_lang.7UMA2Izn.js",
            "HomeProducts.astro_astro_type_script_index_0_lang.DG2qerjp.js",
            "MockupDrift.astro_astro_type_script_index_0_lang.JkKcWt-F.js",
            "details-art.CBAlSB9q.js",
            "HomeDetails.astro_astro_type_script_index_0_lang.CmGwI_bs.js",
            "words-marquee.BNOHUsUN.js", "parallax.BTz8uNkF.js"]

AIDES = ("__defProp", "__typeError", "__defNormalProp", "__publicField", "__accessCheck",
         "__privateGet", "__privateAdd", "__privateSet", "__privateMethod")


def aides_esbuild() -> str:
    """Aides de classes du paquet (champs privés), reprises de son en-tête telles quelles."""
    paquet = (GAB / "js" / "paquet-provisoire.js").read_text(encoding="utf-8")
    tete = paquet[:paquet.index("  // preload-helper")]
    blocs = tete.split("\n  var ")
    garde = [b for b in blocs[1:] if b.split(" ", 1)[0] in AIDES]
    if len(garde) != len(AIDES):
        raise SystemExit("aides du paquet introuvables")
    return "".join("  var " + b.rstrip() + "\n" for b in garde)


def coque_js() -> bytes:
    paquet = (GAB / "js" / "paquet-provisoire.js").read_text(encoding="utf-8")
    _, parts = modules(paquet)
    par_nom = {}
    for i in range(1, len(parts), 2):
        par_nom.setdefault(parts[i], []).append(parts[i + 1])
    mega = par_nom["NavMega.astro_astro_type_script_index_0_lang.C_r6odtz.js"][0]
    pilule = par_nom["pill-indicator.CHvv9BLf.js"][0]
    apparitions = par_nom["reveal.CP69dGxk.js"][0]
    principal = par_nom["main.BT_gO4qF.js"][-1]
    debut = principal.index("  var Q5 = ")
    fin = principal.index("  function Ge3(")
    barre = principal[debut:fin]
    # Le portail n'a ni la fenêtre de paiement ni la recherche de l'origine : leurs écouteurs
    # sont retirés (remplacements comptés).
    barre = compter(barre, ", window.addEventListener(`polar:open`, () => $5?.stop()), "
                           "window.addEventListener(`polar:close`, () => $5?.start())", "")
    barre = compter(barre, ", window.addEventListener(`search:open`, () => $5?.stop()), "
                           "window.addEventListener(`search:close`, () => $5?.start())", "")
    # La création de Lenis garde ses écouteurs, avec le diaporama du hero (Ue3) et la phrase qui
    # tourne (We3) ; le reste du module principal (lettre d'information, sons, réglages) n'est
    # pas repris. Les sections de la page Entreprises suivent, dans la même portée que les
    # apparitions dont elles se servent, comme dans le paquet d'origine.
    sections = []
    for nom in SECTIONS:
        sections += [f"  // {nom}", par_nom[nom][0]]
    corps = "\n".join([
        "  // Méga-menu (NavMega)", mega,
        "  // Indicateur des pilules (pill-indicator)", pilule,
        "  // Apparitions au défilement (reveal)", apparitions,
        "  // Barre, menu mobile, mesures, diaporama, phrase qui tourne (main)", barre,
        "  ze3(), Be3(), Ve3(), He3(), document.querySelectorAll(`[data-nav-indicator]`).forEach((e35) => n11(e35));",
        *sections,
        "  document.querySelectorAll(`[data-slideshow=\"wrap\"]`).forEach(Ue3), (window.__fontsReady ?? document.fonts.ready).then(We3);",
    ])
    entete = (
        "/*!\n"
        " * Coque animée du portail des licences Kadenz : modules du gabarit d'origine (méga-menu,\n"
        " * menu mobile, indicateur des pilules, couleur de la barre, apparitions, sections de la\n"
        " * page Entreprises), pris tels quels dans le paquet de la maquette ; GSAP et ses modules\n"
        " * (ScrollTrigger, SplitText, Observer, CustomEase) et Lenis sont chargés\n"
        " * avant ce fichier, depuis leurs fichiers officiels (voir MENTIONS-gabarit.md).\n"
        " */\n"
    )
    prelude = (
        "(() => {\n"
        "  if (!window.gsap) return;\n"
        "  var oi = window.gsap, M5 = window.gsap, $4 = window.ScrollTrigger, P6 = window.SplitText, Te3 = window.Lenis,\n"
        "    j5 = window.Observer, Y5 = window.CustomEase;\n"
        "  oi.registerPlugin($4, P6, j5, Y5), Y5.create(`slideshow-wipe`, `0.6, 0.08, 0.02, 0.99`);\n"
        + aides_esbuild()
    )
    # Tirets de la coupure des mots (apparitions) écrits en échappements : même expression.
    corps = compter(corps, "[-‐–—]+", "[-\\u2010\\u2013\\u2014]+")
    return (entete + prelude + corps + "\n})();\n").encode("utf-8")


def svg(fragment: str, classe: str) -> str:
    m = re.search(r'<svg[^>]*class="' + re.escape(classe) + r'"[^>]*>.*?</svg>', fragment, re.S)
    assert m, classe
    return m.group(0)


def premier(fragment: str, motif: str) -> str:
    m = re.search(motif, fragment, re.S)
    assert m, motif
    return m.group(0)


def remplacer(t: str, ancien: str, nouveau: str, n: int = 1) -> str:
    if t.count(ancien) != n:
        raise SystemExit(f"{t.count(ancien)} au lieu de {n} : {ancien[:80]}")
    return t.replace(ancien, nouveau)


def sous(t: str, motif: str, nouveau: str, n: int = 1) -> str:
    r, k = re.subn(motif, nouveau, t, flags=re.S)
    if k != n:
        raise SystemExit(f"{k} au lieu de {n} : {motif[:80]}")
    return r


def section(corps: str, debut: str) -> str:
    i = corps.index(debut)
    return corps[i:corps.index("</section>", i) + len("</section>")]


def decouper(region: str, debut: str) -> list:
    """Éléments frères qui commencent tous par `debut` (le dernier va jusqu'au bout)."""
    pos = [m.start() for m in re.finditer(debut, region)]
    return [region[a:b] for a, b in zip(pos, pos[1:] + [len(region)])]


TOILE = r'data-motif="[^"]*" data-cadre="(?:plein|vignette)" data-piece="[^"]*"(?: data-forme="[^"]*")? data-decalage="[^"]*"'


def gabarits_entreprises(page: str) -> dict:
    """Gabarits à trous de la page Entreprises, pris dans le balisage de la maquette
    (attributs de composant compris). Les trous s'écrivent ⟦nom⟧ ; le portail les remplit
    avec ses vraies données (pages/entreprises.rs), jamais avec les exemples de la maquette."""
    corps = page[page.index("<main"):page.index("</main>")]
    g = {}
    cid = lambda t: re.search(r"data-astro-cid-[a-z0-9]+", t).group(0)

    # Hero : diaporama, légende, actualités, bandeau
    h = section(corps, '<section class="section hero inverted"')
    ch = cid(h)
    g["CID_HERO"] = ch
    i = h.index(f'<div class="img-slider__list" {ch}>') + len(f'<div class="img-slider__list" {ch}>')
    j = h.index(f'<div class="img-slider__nav"')
    liste = h[i:j - len("</div>")]
    diapos = decouper(liste, r'<div class="img-slide(?: is--current)?" ')
    if len(diapos) != 4:
        raise SystemExit("hero : 4 diapositives attendues")

    def diapo(t):
        t = sous(t, r'data-caption="[^"]*" data-href="[^"]*"', 'data-caption="⟦legende⟧" data-href="⟦lien⟧"')
        return sous(t, TOILE, "⟦toile⟧")
    g["DIAPO_PREMIERE"], g["DIAPO"], g["DIAPO_DERNIERE"] = diapo(diapos[0]), diapo(diapos[1]), diapo(diapos[3])
    h = h[:i] + "⟦diapos⟧" + h[i + len(liste):]
    i = h.index(f'<div class="img-slider__nav"')
    i = h.index(">", i) + 1
    j = h.index("</div>", i)
    vignettes = decouper(h[i:j], "<button")
    if len(vignettes) != 4:
        raise SystemExit("hero : 4 vignettes attendues")

    def vignette(t):
        t = sous(t, r'aria-label="Afficher [^"]*"', 'aria-label="Afficher ⟦legende⟧"')
        return sous(t, TOILE, "⟦toile⟧")
    g["VIGNETTE_PREMIERE"], g["VIGNETTE"] = vignette(vignettes[0]), vignette(vignettes[1])
    h = h[:i] + "⟦vignettes⟧" + h[j:]
    h = sous(h, r'(<span class="caption-line" data-caption-line [^>]*>)[^<]*(</span>)', r"\g<1>⟦legende⟧\g<2>")
    a = h.index('<aside class="news"')
    b = h.index("</aside>") + len("</aside>")
    aside = h[a:b]
    h = h[:a] + "⟦actualites⟧" + h[b:]
    i = aside.index(">", aside.index("<ul class=\"news-list\"")) + 1
    j = aside.index("</ul>")
    articles = decouper(aside[i:j], "<li ")
    art = articles[0]
    art = remplacer(art, 'class="st-1 news-item"', 'class="st-⟦rang⟧ news-item"')
    art = remplacer(art, '<a href="#journal" class="news-link" ', '<a href="/journal" class="news-link"⟦attributs⟧ ')
    art = sous(art, r'data-piece="[^"]*"', 'data-piece="⟦piece⟧"')
    art = sous(art, r'(<span class="news-title" [^>]*>)[^<]*', r"\g<1>⟦titre⟧")
    art = sous(art, r'(<span class="news-date" [^>]*>)[^<]*', r"\g<1>⟦date⟧")
    g["ACTUALITE"] = art
    aside = aside[:i] + "⟦articles⟧" + aside[j:]
    aside = sous(aside, r'data-news-id="[^"]*"', 'data-news-id="⟦id⟧"')
    aside = remplacer(aside, 'href="#journal"', 'href="/journal"')
    g["ACTUALITES"] = aside
    a = h.index('<div class="news-bar"')
    i = h.index("data-news-marquee", a)
    i = h.index(">", i) + 1
    fin_items = h.rindex("</a>") + len("</a>") + len("</div>")
    b = fin_items + len("</div></div></div>")
    if h[fin_items:b] != "</div></div></div>":
        raise SystemExit("hero : fin du bandeau inattendue")
    g["BANDEAU"] = h[a:i] + "⟦jeux⟧" + h[fin_items:b]
    h = h[:a] + "⟦bandeau⟧" + h[b:]
    g["HERO"] = h

    # Intro : phrase qui tourne et personne (le compte connecté)
    t = section(corps, '<section class="section intro inverted"')
    ci = cid(t)
    debut = (f'Toutes les entreprises clientes,<span class="intro-br-all" {ci}></span> directes ou rattachées'
             f'<span class="intro-br-m" {ci}></span> à un revendeur.')
    t = remplacer(t, debut, "⟦debut⟧")
    g["INTRO_DEBUT_EDITEUR"] = debut
    g["INTRO_DEBUT_REVENDEUR"] = (f'Les entreprises<span class="intro-br-all" {ci}></span> qui vous sont'
                                  f'<span class="intro-br-m" {ci}></span> rattachées.')
    t = sous(t, rf'(<div class="muted" {ci}>)[^<]*(</div>)', r"\g<1>⟦nom⟧\g<2>")
    t = sous(t, rf'(<div class="label-small muted" {ci}>)[^<]*(</div>)', r"\g<1>⟦role⟧\g<2>")
    g["INTRO"] = t

    # Grille des entreprises : compteurs, onglets, cartes
    t = section(corps, '<section id="product-grid"')
    cp = cid(t)
    g["CID_PRODUITS"] = cp
    t = sous(t, r'data-hp-counts="[^"]*"', 'data-hp-counts="⟦comptes⟧"')
    t = sous(t, r'(<span class="odo-roll" data-odometer data-interval="1.5">).*?<span class="visually-hidden">[^<]*</span>',
             r"\g<1>⟦roulement⟧")
    # Deux lignes par onglet : la première (toutes : le roulement), la seconde (le résumé)
    for cle, jetons in (("toutes", ["ligne2_toutes"]), ("directes", ["ligne_directes", "ligne2_directes"]),
                        ("rattachees", ["ligne_rattachees", "ligne2_rattachees"])):
        trouves = list(re.finditer(rf'(<span class="hp-line(?: is-active)?" data-hp-line="{cle}" {cp}>)([^<]*)(</span>)', t))
        if len(trouves) != len(jetons):
            raise SystemExit(f"grille : lignes de l'onglet {cle}")
        for m, jeton in reversed(list(zip(trouves, jetons))):
            t = t[:m.start(2)] + f"⟦{jeton}⟧" + t[m.end(2):]
    pos = [m for m in re.finditer(r'data-odo="-?\d+"', t)]
    if len(pos) != 3:
        raise SystemExit("grille : 3 chiffres attendus")
    for n, m in reversed(list(enumerate(pos))):
        t = t[:m.start()] + f'data-odo="⟦odo{n}⟧"' + t[m.end():]
    cartes = {}
    for cle in ("toutes", "directes", "rattachees"):
        a = t.index(f'data-hp-panel="{cle}"')
        a = t.index("data-astro-cid-kb4uamjl>", t.index("data-slider ", a)) + len("data-astro-cid-kb4uamjl>")
        b = t.index(f'</div></div><div class="container row hp-buttons" {cp}>', a)
        cartes[cle] = t[a:b]
        t = t[:a] + f"⟦cartes_{cle}⟧" + t[b:]
    t = sous(t, r'href="#(toutes|directes|rattachees)" class="hp-all"', 'href="#liste" class="hp-all"', 3)
    g["PRODUITS"] = t
    carte = decouper(cartes["toutes"], re.escape(f'<div class="hp-slide" {cp}>'))[0]
    cc = cid(carte[carte.index("<article"):])
    g["CID_CARTE"] = cc
    carte = sous(carte, r'canvas-card--web" data-card data-category="[^"]*"',
                 'canvas-card--⟦genre⟧" data-card data-category="⟦categorie⟧"')
    carte = remplacer(carte, 'href="#fiche"', 'href="⟦lien⟧"', 3)
    carte = remplacer(carte, 'href="#paiement"', 'href="⟦paiement⟧"')
    carte = sous(carte, rf'(<span class="visually-hidden" {cc}>)[^<]*', r"\g<1>⟦client⟧")
    carte = sous(carte, rf'(<div class="cc-tags" {cc}>).*?(</div>)', r"\g<1>⟦etiquettes⟧\g<2>")
    carte = sous(carte, rf'(<span class="cc-frame" {cc}>).*?(</span></a>)', r"\g<1>⟦calques⟧\g<2>")
    carte = sous(carte, rf'(class="strong" {cc}>)[^<]*(</a><span class="muted" {cc}>)[^<]*', r"\g<1>⟦client⟧\g<2>⟦revendeur⟧")
    carte = sous(carte, rf'(<span data-price {cc}>)[^<]*', r"\g<1>⟦echeance⟧")
    g["CARTE"] = carte

    # Dérive : le code d'activation ; le bouton mène aux codes à vendre (POST /codes)
    t = section(corps, '<section class="section drift inverted"')
    cd = cid(t)
    t = remplacer(t, '<a href="#emission" class="button-animated" data-text="Après paiement"',
                  '<a href="/codes" class="button-animated" data-text="Codes à vendre"')
    t = sous(t, rf'<a href="#codes" class="button-pill" {cd}>[^<]*</a>', "")
    g["DERIVE"] = t

    # Détails : les règles du portail, texte fixe
    g["DETAILS"] = section(corps, '<section class="section-light hd"')

    # Journal : distinctions (chaîne et sceau, pour l'éditeur) et bandeau des dernières actions
    t = section(corps, '<section class="kw inverted"')
    ck = cid(t)
    g["CID_KW"] = ck
    a = t.index('<ul class="kw-awards"')
    b = t.index("</ul>", a) + len("</ul>")
    d = t[a:b]
    d = sous(d, rf'<span {ck}>128 entrées</span><span class="muted" {ck}>chaîne intacte</span>',
             f'<span {ck}>⟦entrees⟧</span><span class="muted" {ck}>⟦etat⟧</span>')
    d = sous(d, rf'<span {ck}>Sceau 5b1e9c07</span><span class="muted" {ck}>noté hors du serveur</span>',
             f'<span {ck}>⟦sceau⟧</span><span class="muted" {ck}>à noter hors du serveur</span>')
    d = remplacer(d, 'href="#journal"', 'href="/journal"')
    g["KW_DISTINCTIONS"] = d
    t = t[:a] + "⟦distinctions⟧" + t[b:]
    a = t.index(">", t.index('<div class="kw-track"')) + 1
    b = t.index("</div>", t.rindex("</figure>"))
    boites = decouper(t[a:b], '<figure class="kw-box"')
    boite = boites[0]
    boite = remplacer(boite, f'<figure class="kw-box" {ck}>', f'<figure class="kw-box"⟦cache⟧ {ck}>')
    boite = sous(boite, r'data-piece="[^"]*"', 'data-piece="⟦piece⟧"')
    boite = sous(boite, rf'(<span class="kw-name text-small" {ck}><span {ck}>)[^<]*(</span><span class="muted" {ck}>)[^<]*',
                 r"\g<1>⟦action⟧\g<2>⟦quand⟧")
    boite = sous(boite, rf'(<blockquote class="kw-quote" {ck}>)[^<]*', r"\g<1>⟦citation⟧")
    g["KW_BOITE"] = boite
    t = t[:a] + "⟦boites⟧" + t[b:]
    g["KW"] = t
    for cle, v in g.items():
        if "Exemple" in v or "exemple" in v.replace("par exemple", ""):
            raise SystemExit(f"{cle} : exemple de la maquette resté dans le gabarit")
    return g


def main():
    SORTIE.mkdir(parents=True, exist_ok=True)
    for ancien in SORTIE.iterdir():
        ancien.unlink()

    css = []
    for nom in CSS_ORDRE:
        t = (GAB / "css" / nom).read_text(encoding="utf-8")
        if nom == "polices.css":
            t = compter(t, "url(../polices/GeistMono-Variable.woff2)", "url(__GEIST__)")
        t = t.replace('url("../img/account-1.svg")', "none").replace('url("../img/account-2.svg")', "none")
        t = t.replace("url(/images/logos/higgsfield.png)", "none").replace("url(/images/logos/jitter.png)", "none")
        assert "url(" not in t.replace("url(__GEIST__)", ""), nom
        t = t.replace("Diatype", "Inter")  # jamais Diatype dans le portail
        css.append(f"/* {nom} */\n{t}")
    geist = (GAB / "polices" / "GeistMono-Variable.woff2").read_bytes()
    a_geist = poser("POLICE_GEIST", "geist-mono", "woff2", geist, "font/woff2")
    poser("LICENCE_GEIST", "geist-mono-ofl", "txt", (GAB / "polices" / "OFL.txt").read_bytes(),
          "text/plain; charset=utf-8")
    feuille = "\n".join(css).replace("url(__GEIST__)", f"url({a_geist})") + SURCHARGE
    poser("CSS", "gabarit", "css", feuille.encode("utf-8"), "text/css; charset=utf-8")

    js = "text/javascript; charset=utf-8"
    for constante, nom in [("JS_COUVERTURE", "couverture"), ("JS_ID", "id-principal"),
                           ("JS_COULEUR", "couleur-menu"), ("JS_TRANSITIONS", "transitions")]:
        poser(constante, nom, "js", (GAB / "js" / f"{nom}.js").read_bytes(), js)
    poser("JS_MOTIFS", "motifs-kadenz", "js", (MAQUETTE / "motifs-kadenz.js").read_bytes(), js)
    for constante, nom in [("JS_GSAP", "gsap.min"), ("JS_SCROLLTRIGGER", "ScrollTrigger.min"),
                           ("JS_SPLITTEXT", "SplitText.min")]:
        poser(constante, nom.replace(".min", "").lower(), "js", (GSAP / "dist" / f"{nom}.js").read_bytes(), js)
    for constante, nom in [("JS_OBSERVER", "Observer.min"), ("JS_CUSTOMEASE", "CustomEase.min")]:
        poser(constante, nom.replace(".min", "").lower(), "js", (GSAP / "dist" / f"{nom}.js").read_bytes(), js)
    poser("JS_LENIS", "lenis", "js", (LENIS / "dist" / "lenis.min.js").read_bytes(), js)
    poser("JS_COQUE", "coque", "js", coque_js(), js)
    poser("LICENCE_LENIS", "lenis-licence", "txt", (LENIS / "LICENSE").read_bytes(), "text/plain; charset=utf-8")

    # Fragments du balisage, tels que la maquette les a rendus (attributs de composant compris).
    corps = PAGE[PAGE.index("<body"):]
    couverture = corps[corps.index('<div class="pt"'):corps.index("<header")]
    entete = corps[corps.index("<header"):corps.index("</header>") + 9]
    pied = corps[corps.index("<footer"):corps.index("</footer>") + 9]
    fragments = {
        "COUVERTURE": couverture,
        "SVG_LOGO": premier(entete, r'<svg class="nav-logo-svg.*?</svg>|<a href="[^"]*" class="nav-pill nav-logo"[^>]*>(<svg.*?</svg>)'),
        "SVG_INVITE": premier(entete, r'<span class="nav-guest"[^>]*>(<svg.*?</svg>)</span>'),
        "SVG_ICONES_MENU": premier(entete, r'<span class="nav-toggle-icon"[^>]*>.*?</span>'),
        "SVG_CARET": premier(entete, r'<svg class="mm-caret".*?</svg>'),
        "SVG_RETOUR": premier(entete, r'<button type="button" class="mm-back"[^>]*>(<svg.*?</svg>)</button>'),
        "PIED_LOGO": premier(pied, r'<div class="footer-logo".*?</div>'),
    }
    # Garder seulement le SVG des captures à groupe.
    for cle in ("SVG_LOGO", "SVG_INVITE", "SVG_RETOUR"):
        m = re.search(r"<svg.*?</svg>", fragments[cle], re.S)
        fragments[cle] = m.group(0)
    cid = re.search(r"data-astro-cid-[a-z0-9]+", entete).group(0)
    cid_pied = re.search(r"data-astro-cid-[a-z0-9]+", pied).group(0)

    lignes = [
        "//! Gabarit animé des pages du compte. Généré par crates/portail/outils/generer_gabarit.py :",
        "//! ne pas modifier à la main.",
        "",
        f'pub const CID_NAV: &str = "{cid}";',
        f'pub const CID_PIED: &str = "{cid_pied}";',
    ]
    for constante, adresse, _, _ in fichiers:
        lignes.append(f'pub const {constante}: &str = "{adresse}";')
    lignes += ["", "/// Fichiers servis par le portail : adresse, type, contenu.",
               "pub const FICHIERS: &[(&str, &str, &[u8])] = &["]
    for constante, _, type_contenu, fichier in fichiers:
        lignes.append(f'    ({constante}, "{type_contenu}", include_bytes!("../../statique/gabarit/{fichier}")),')
    lignes.append("];")
    for cle, valeur in fragments.items():
        assert '"##' not in valeur
        lignes += ["", f'pub const {cle}: &str = r##"{valeur}"##;']
    lignes += ["", "// Page Entreprises : gabarits à trous (⟦nom⟧), remplis par pages/entreprises.rs."]
    for cle, valeur in gabarits_entreprises(PAGE).items():
        assert '"##' not in valeur
        lignes += ["", f'pub const {cle}: &str = r##"{valeur}"##;']
    RUST.write_text("\n".join(lignes) + "\n", encoding="utf-8")
    for constante, adresse, _, fichier in fichiers:
        print(f"{constante:18} {adresse}  {(SORTIE / fichier).stat().st_size} octets")


main()
