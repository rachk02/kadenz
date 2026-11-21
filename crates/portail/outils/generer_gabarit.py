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
    fin = principal.index("  function Ue3(")
    barre = principal[debut:fin]
    # Le portail n'a ni la fenêtre de paiement ni la recherche de l'origine : leurs écouteurs
    # sont retirés (remplacements comptés).
    barre = compter(barre, ", window.addEventListener(`polar:open`, () => $5?.stop()), "
                           "window.addEventListener(`polar:close`, () => $5?.start())", "")
    barre = compter(barre, ", window.addEventListener(`search:open`, () => $5?.stop()), "
                           "window.addEventListener(`search:close`, () => $5?.start())", "")
    # La création de Lenis garde ses écouteurs ; le reste du module principal (diaporamas,
    # lettre d'information, sons) n'est pas repris dans la coque.
    corps = "\n".join([
        "  // Méga-menu (NavMega)", mega,
        "  // Indicateur des pilules (pill-indicator)", pilule,
        "  // Apparitions au défilement (reveal)", apparitions,
        "  // Barre, menu mobile, mesures (main)", barre,
        "  ze3(), Be3(), Ve3(), He3(), document.querySelectorAll(`[data-nav-indicator]`).forEach((e35) => n11(e35));",
    ])
    entete = (
        "/*!\n"
        " * Coque animée du portail des licences Kadenz : modules du gabarit d'origine (méga-menu,\n"
        " * menu mobile, indicateur des pilules, couleur de la barre, apparitions), pris tels quels\n"
        " * dans le paquet de la maquette ; GSAP, ScrollTrigger, SplitText et Lenis sont chargés\n"
        " * avant ce fichier, depuis leurs fichiers officiels (voir MENTIONS.md).\n"
        " */\n"
    )
    prelude = (
        "(() => {\n"
        "  if (!window.gsap) return;\n"
        "  var oi = window.gsap, M5 = window.gsap, $4 = window.ScrollTrigger, P6 = window.SplitText, Te3 = window.Lenis;\n"
        "  oi.registerPlugin($4, P6);\n"
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
    RUST.write_text("\n".join(lignes) + "\n", encoding="utf-8")
    for constante, adresse, _, fichier in fichiers:
        print(f"{constante:18} {adresse}  {(SORTIE / fichier).stat().st_size} octets")


main()
