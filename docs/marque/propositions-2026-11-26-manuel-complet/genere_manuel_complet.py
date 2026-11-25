"""Manuel d'utilisation complet de Kadenz : mise en page finale (2026-11-26).

Entrées (lues, jamais modifiées) : docs/manuel/chapitres/01-installation.md à 15-depannage.md, index.md,
docs/manuel/captures/<chapitre>/*.png.
Design : la version 2 du 2026-11-25 (propositions-2026-11-25-manuel-v2/generer_manuel_v2.py), elle-même reprise
des couvertures du 2026-11-18 : couverture, sommaire, ouverture de chaque chapitre, pages courantes sans en-tête,
pied (titre du manuel, version, numéro de page), index, quatrième de couverture.

Méthode (aucune bibliothèque Python, Chrome headless seulement) :
  1. un convertisseur Markdown maison produit des blocs (titres, paragraphes, éléments de liste, tableaux,
     figures, encadrés Astuce et Attention) ;
  2. Chrome mesure la hauteur de chaque bloc à la largeur de la page ;
  3. la pagination est calculée ici (titres collés au texte suivant, figure collée au texte qui l'introduit,
     tableaux coupés entre deux lignes avec l'en-tête répété), ce qui donne les numéros de page réels ;
  4. le sommaire et l'index reçoivent ces numéros, puis Chrome écrit le PDF A4 et une planche de contrôle.

Usage : py -3.14 -B genere_manuel_complet.py [--apercu 12,13,40]
Sorties : manuel-complet.pdf, planche-manuel-complet.png (et, avec --apercu, des PNG dans _apercus/).
Les polices ABC Diatype (essai) sont lues dans crates/app/ui/polices/, jamais copiées.
"""

import html as _html
import itertools
import json
import math
import os
import random
import re
import subprocess
import sys
from pathlib import Path
from string import Template

ICI = Path(__file__).resolve().parent
MARQUE = ICI.parent
RACINE = ICI.parents[2]
MANUEL = RACINE / "docs" / "manuel"
CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"

NUIT, INDIGO, AMBRE, BRUME = "#1A2259", "#2D3A8C", "#E3A23B", "#F4F5F9"
ENCRE, ARDOISE, TRAIT = "#1F2933", "#52606D", "#D9DCE3"
PIECES_MOTIF, TEXTE_NUIT = "#232C6B", "#C5CAE0"
VERSION = "0.3.1"
NB_CHAP = 15
FIG_LARGEUR = int(os.environ.get("FIG", "100"))   # largeur des captures en % du texte
GRAIN = True

PAGE_W, PAGE_H = 793.7008, 1122.5197      # A4 en pixels CSS
MARGE = 56
LARG = PAGE_W - 2 * MARGE                  # largeur du texte
HAUT_TOP, HAUT_BAS = 62, 130               # marges haute et basse des pages courantes
CAP = PAGE_H - HAUT_TOP - HAUT_BAS         # hauteur utile d'une page courante
ECHELLE = PAGE_H / 1145                    # les pages dessinées (816 x 1145) sont réduites à l'A4
DECALAGE = (816 * ECHELLE - PAGE_W) / 2


# --- éléments graphiques (repris de generer_manuel_v2.py, lui-même copié de generer_couvertures.py) -------
def logo(fichier, largeur, extra=""):
    return f'<img src="../{fichier}" style="width:{largeur}px;display:block;{extra}" alt="Kadenz">'


def visee(taille=120, rond=False, point=False, couleur="currentColor", e=2):
    c = taille / 2
    s = (f'<svg width="{taille}" height="{taille}" viewBox="0 0 {taille} {taille}" fill="none" '
         f'stroke="{couleur}" stroke-width="{e}" aria-hidden="true">'
         f'<path d="M{c} 0V{taille}M0 {c}H{taille}"/>')
    if rond:
        s += f'<circle cx="{c}" cy="{c}" r="{taille * 0.2}"/>'
    if point:
        s += f'<circle cx="{c}" cy="{c}" r="{taille * 0.07}" fill="{couleur}" stroke="none"/>'
    return s + "</svg>"


D_PIECES = {
    "carre": "M0 0 H100 V100 H0 Z",
    "quart": "M0 100 V0 A100 100 0 0 1 100 100 Z",
    "triangle": "M0 0 L100 100 H0 Z",
    "disque": "M0 50 A50 50 0 1 1 100 50 A50 50 0 1 1 0 50 Z",
}


def piece_degrade(sorte, taille, cle, de=INDIGO, vers=AMBRE, angle=(0, 0, 1, 1)):
    x1, y1, x2, y2 = angle
    grain = ""
    if GRAIN:
        grain = (f'<defs><filter id="{cle}g" x="0" y="0" width="1" height="1"><feTurbulence type="fractalNoise" '
                 f'baseFrequency="2.2" numOctaves="2" seed="4"/><feColorMatrix type="saturate" values="0"/>'
                 f'<feComponentTransfer><feFuncA type="linear" slope=".5"/></feComponentTransfer>'
                 f'<feComposite in2="SourceGraphic" operator="in"/></filter></defs>'
                 f'<path d="{D_PIECES[sorte]}" fill="#000" filter="url(#{cle}g)" style="mix-blend-mode:overlay" opacity=".55"/>')
    return (f'<svg width="{taille}" height="{taille}" viewBox="0 0 100 100" aria-hidden="true">'
            f'<defs><linearGradient id="{cle}" x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}">'
            f'<stop offset="0" stop-color="{de}"/><stop offset="1" stop-color="{vers}"/></linearGradient></defs>'
            f'<path d="{D_PIECES[sorte]}" fill="url(#{cle})"/>{grain}</svg>')


def trame_disque(diametre, pas, couleur=NUIT, lumiere=(0.72, 0.30)):
    r = diametre / 2
    lx, ly = lumiere[0] * diametre, lumiere[1] * diametre
    out = []
    n = int(diametre / pas)
    for i in range(n + 1):
        for j in range(n + 1):
            x, y = i * pas + (pas / 2 if j % 2 else 0), j * pas
            if (x - r) ** 2 + (y - r) ** 2 > r * r:
                continue
            d = math.hypot(x - lx, y - ly) / diametre
            rr = pas * 0.52 * min(1, max(0.08, d * 1.25))
            out.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{rr:.2f}"/>')
    return (f'<svg width="{diametre}" height="{diametre}" viewBox="0 0 {diametre} {diametre}" fill="{couleur}" '
            f'aria-hidden="true">{"".join(out)}</svg>')


def f(v):
    return f"{v:.2f}".rstrip("0").rstrip(".")


def motif_tournant(w, h, s=50, graine=7, fond=NUIT, trace=PIECES_MOTIF):
    alea = random.Random(graine)
    out = [f'<rect width="{f(w)}" height="{f(h)}" fill="{fond}"/>']
    for r in range(math.ceil(h / s)):
        for col in range(math.ceil(w / s)):
            x, y = col * s, r * s
            rot = 90 * alea.randrange(4)
            out.append(f'<path fill="{trace}" transform="rotate({rot} {f(x + s / 2)} {f(y + s / 2)})"'
                       f' d="M{f(x)} {f(y)} L{f(x + s)} {f(y)} A{f(s)} {f(s)} 0 0 1 {f(x)} {f(y + s)} Z"/>')
    return (f'<svg width="{w}" height="{h}" viewBox="0 0 {f(w)} {f(h)}" style="display:block" aria-hidden="true">'
            + "".join(out) + "</svg>")


def liste(lignes, classe="liste", style=""):
    st = f' style="{style}"' if style else ""
    return f'<div class="{classe}"{st}>' + "".join(f"<div>{l}</div>" for l in lignes) + "</div>"


def temps_vertical(d=34, g=16, clair="#FFFFFF"):
    return (f'<div style="display:flex;flex-direction:column;gap:{g}px">'
            f'<span class="rond" style="width:{d}px;height:{d}px;background:{clair}"></span>'
            f'<span class="rond" style="width:{d}px;height:{d}px;background:{clair}"></span>'
            f'<span class="rond" style="width:{d}px;height:{d}px;background:{AMBRE}"></span></div>')


def pied_nuit(num):
    return (f'<div style="position:absolute;left:0;right:0;bottom:0;height:200px;background:{NUIT}"></div>'
            f'<div style="position:absolute;left:56px;bottom:66px">{logo("kadenz-logo-horizontal-negatif.svg", 220)}</div>'
            f'<div class="liste a4s droite" style="position:absolute;right:56px;bottom:62px;color:{TEXTE_NUIT}">'
            f'Version {VERSION}<br>Chapitre {num} / {NB_CHAP}</div>')


# --- mesure approchée du texte pour placer les titres et listes des ouvertures --------------------------
def repartir(mots, k):
    """Répartit les mots sur k lignes en minimisant la plus longue ligne."""
    if k >= len(mots):
        return [m for m in mots]
    meilleur, ml = None, 10 ** 9
    for coupes in itertools.combinations(range(1, len(mots)), k - 1):
        bornes = (0,) + coupes + (len(mots),)
        lignes = [" ".join(mots[bornes[i]:bornes[i + 1]]) for i in range(k)]
        m = max(len(l) for l in lignes)
        if m < ml:
            meilleur, ml = lignes, m
    return meilleur


LH = 0.94   # interligne des titres en capitales : assez pour ne pas heurter les accents


def titre_ajuste(titre, largeur, fs_max, lignes_max, ratio=0.64, hmax=None):
    mots = titre.split(" ")
    best = None
    for k in range(1, min(lignes_max, len(mots)) + 1):
        lignes = repartir(mots, k)
        longest = max(len(l) for l in lignes)
        fs = min(fs_max, largeur / (longest * ratio))
        if hmax and len(lignes) * fs * LH > hmax and best is not None:
            continue
        if best is None or fs > best[1] + 0.01:
            best = (lignes, fs)
        if fs >= fs_max:
            break
    return best


def haut_liste(labels, fs, largeur, interligne=1.35, ratio=0.72):
    n = sum(max(1, math.ceil(len(l) * fs * ratio / largeur)) for l in labels)
    return n * fs * interligne


def bloc_titre(lignes, fs, style="", classe="titre"):
    return (f'<div class="{classe}" style="font-size:{fs:.1f}px;line-height:{LH};{style}">'
            + "<br>".join(lignes) + "</div>")


# --- lecture des chapitres ------------------------------------------------------------------------------
def inline(s):
    codes = []

    def garde(m):
        codes.append(m.group(1))
        return f"\x00{len(codes) - 1}\x00"

    s = re.sub(r"`([^`]+)`", garde, s)
    s = _html.escape(s, quote=False)
    s = re.sub(r"\*\*(.+?)\*\*", r"<b>\1</b>", s)
    s = re.sub(r"\x00(\d+)\x00", lambda m: "<code>" + _html.escape(codes[int(m.group(1))], quote=False) + "</code>", s)
    return s


def typo(h):
    """Espaces insécables à la française, hors balises et hors code."""
    morceaux = re.split(r"(<code>.*?</code>|<[^>]+>)", h)
    for i, m in enumerate(morceaux):
        if i % 2 == 0:
            m = re.sub(r" ([;:?!»])", "&nbsp;\\1", m)
            morceaux[i] = m.replace("« ", "«&nbsp;")
    return "".join(morceaux)


RE_ITEM = re.compile(r"^(\s*)([-*]|\d+[.)])\s+(.*)$")


def parser(texte):
    """Markdown vers une liste de blocs. Éléments de liste : un bloc par élément de premier niveau."""
    lignes = texte.split("\n")
    blocs, titre, i = [], None, 0
    n = len(lignes)

    def debut_bloc(l):
        return (l.startswith("#") or l.startswith(">") or l.startswith("|") or l.startswith("![")
                or RE_ITEM.match(l))

    while i < n:
        l = lignes[i].rstrip()
        if not l.strip():
            i += 1
            continue
        if l.startswith("# "):
            titre = l[2:].strip()
            i += 1
        elif l.startswith("### "):
            blocs.append({"t": "h3", "x": l[4:].strip()})
            i += 1
        elif l.startswith("## "):
            blocs.append({"t": "h2", "x": l[3:].strip()})
            i += 1
        elif l.startswith("!["):
            m = re.match(r"!\[([^\]]*)\]\(([^)]+)\)", l)
            blocs.append({"t": "fig", "alt": m.group(1), "src": m.group(2)})
            i += 1
        elif l.startswith(">"):
            paras, cur = [], []
            while i < n and lignes[i].startswith(">"):
                x = lignes[i][1:].strip()
                if x:
                    cur.append(x)
                elif cur:
                    paras.append(" ".join(cur))
                    cur = []
                i += 1
            if cur:
                paras.append(" ".join(cur))
            m = re.match(r"\*\*([^*]+)\*\*\s*(.*)", paras[0])
            if m:
                paras[0] = m.group(2)
                label = m.group(1)
            else:
                label = ""
            blocs.append({"t": "note", "label": label, "paras": [p for p in paras if p]})
        elif l.startswith("|"):
            rows = []
            while i < n and lignes[i].startswith("|"):
                cells = [c.strip() for c in lignes[i].strip().strip("|").split("|")]
                rows.append(cells)
                i += 1
            rows = [r for r in rows if not all(re.fullmatch(r":?-{2,}:?", c) for c in r)]
            blocs.append({"t": "table", "head": rows[0], "rows": rows[1:]})
        elif RE_ITEM.match(l):
            items = []
            while i < n:
                l = lignes[i].rstrip()
                m = RE_ITEM.match(l)
                if m:
                    items.append((len(m.group(1)), m.group(2), m.group(3)))
                    i += 1
                elif not l.strip():
                    j = i
                    while j < n and not lignes[j].strip():
                        j += 1
                    if j < n and RE_ITEM.match(lignes[j].rstrip()):
                        i = j
                    else:
                        break
                elif lignes[i].startswith(" ") and items:
                    a, b, c = items[-1]
                    items[-1] = (a, b, c + " " + l.strip())
                    i += 1
                else:
                    break
            tops = []
            for ind, mark, txt in items:
                if ind == 0:
                    tops.append({"ord": mark[0].isdigit(), "n": int(mark.rstrip(".)")) if mark[0].isdigit() else 0,
                                 "x": txt, "sub": []})
                elif tops:
                    tops[-1]["sub"].append(txt)
            for k, it in enumerate(tops):
                blocs.append({"t": "li", **it, "k": k, "nk": len(tops)})
        else:
            cur = [l.strip()]
            i += 1
            while i < n and lignes[i].strip() and not debut_bloc(lignes[i]):
                cur.append(lignes[i].strip())
                i += 1
            blocs.append({"t": "p", "x": " ".join(cur)})
    return titre, blocs


def charger_chapitres():
    chap = []
    for p in sorted((MANUEL / "chapitres").glob("[0-9][0-9]-*.md")):
        titre, blocs = parser(p.read_text(encoding="utf-8"))
        m = re.match(r"(\d\d)\s+(.*)", titre)
        num, nom = m.group(1), m.group(2)
        labels = [re.sub(r"^\d+\.\d+\s+", "", b["x"]) for b in blocs if b["t"] == "h2"]
        chap.append({"num": num, "titre": nom, "blocs": blocs, "fichier": p.name,
                     "labels": [l for l in labels if l != "À retenir"], "dir": p.parent})
    return chap


def charger_index():
    texte = (MANUEL / "chapitres" / "index.md").read_text(encoding="utf-8")
    entrees, lettre = [], None
    intro = None
    for l in texte.split("\n"):
        if l.startswith("## "):
            lettre = l[3:].strip()
            entrees.append(("lettre", lettre, []))
        elif l.startswith("- ") and lettre:
            terme, _, nums = l[2:].rpartition(" : ")
            entrees.append(("entree", terme.strip(), [s.strip() for s in nums.split(",")]))
        elif l.strip() and not l.startswith("#") and intro is None:
            intro = l.strip()
    return intro, entrees


# --- rendu HTML d'un bloc --------------------------------------------------------------------------------
def rel(chemin):
    return os.path.relpath(chemin, ICI).replace("\\", "/")


def h1_html(c):
    n = len(c["titre"])
    fs = 54 if n <= 20 else (40 if n <= 36 else 32)
    return (f'<div class="titre-ch"><h1 class="titre" style="font-size:{fs}px">{inline(c["titre"])}</h1>'
            f'<span class="chiffre">{c["num"]}</span></div>')


def rendu_bloc(b, c, piece=None, premier=False):
    t = b["t"]
    cls = f"b b-{t}" + (" prem" if premier else "")
    if t == "h1":
        return f'<div class="{cls}">{h1_html(c)}</div>'
    if t in ("h2", "h3"):
        txt = inline(b["x"])
        if t == "h2":
            m = re.match(r"(\d+\.\d+)\s+(.*)", b["x"])
            if m:
                txt = f'<span class="sn">{m.group(1)}</span>{inline(m.group(2))}'
        return f'<div class="{cls}"><h{t[1]}>{txt}</h{t[1]}></div>'
    if t == "p":
        return f'<div class="{cls}"><p>{inline(b["x"])}</p></div>'
    if t == "li":
        sub = ""
        if b["sub"]:
            sub = "<ul>" + "".join(f"<li>{inline(s)}</li>" for s in b["sub"]) + "</ul>"
        cl = cls + (" l0" if b["k"] == 0 else "") + (" lN" if b["k"] == b["nk"] - 1 else "")
        if b["ord"]:
            return f'<div class="{cl}"><ol start="{b["n"]}"><li>{inline(b["x"])}{sub}</li></ol></div>'
        return f'<div class="{cl}"><ul><li>{inline(b["x"])}{sub}</li></ul></div>'
    if t == "note":
        corps = "".join(f"<p>{inline(p)}</p>" for p in b["paras"])
        lab = f"<b class='lab'>{inline(b['label'])}</b>" if b["label"] else ""
        return f'<div class="{cls}"><div class="note">{lab}{corps}</div></div>'
    if t == "fig":
        src = rel((c["dir"] / b["src"]).resolve())
        return (f'<div class="{cls}"><figure class="fig"><div class="cadre-capture"><img src="{src}" width="1264" '
                f'height="1121" alt="{_html.escape(b["alt"])}"></div>'
                f'<figcaption>{inline(b["alt"])}</figcaption></figure></div>')
    if t == "table":
        r0, r1 = (0, len(b["rows"])) if piece is None else piece
        th = "".join(f"<th>{inline(x)}</th>" for x in b["head"])
        lignes = "".join("<tr>" + "".join(f"<td>{inline(x)}</td>" for x in row) + "</tr>"
                         for row in b["rows"][r0:r1])
        return f'<div class="{cls}"><table><thead><tr>{th}</tr></thead><tbody>{lignes}</tbody></table></div>'
    if t == "ie":      # entrée d'index
        return (f'<div class="{cls}"><span class="t">{inline(b["terme"])}</span>'
                f'<span class="n">{b["pages"]}</span></div>')
    if t == "il":      # lettre d'index
        return f'<div class="{cls}"><span>{b["x"]}</span></div>'
    raise ValueError(t)


def doc_html(corps, titre, classe="", grain=False):
    return (f'<!doctype html>\n<html lang="fr"><head><meta charset="utf-8"><title>{titre} | Kadenz</title>'
            f'<style>{feuille(grain)}</style></head><body class="{classe}">{corps}</body></html>\n')


# --- pagination ------------------------------------------------------------------------------------------
def marquer_garde(blocs):
    """Pose sur chaque bloc l'indicateur « reste avec le suivant »."""
    n = len(blocs)
    for i, b in enumerate(blocs):
        b["keep"] = False
    for i, b in enumerate(blocs):
        t = b["t"]
        if t in ("h1", "h2", "h3"):
            b["keep"] = True
        elif t == "p" and b["x"].rstrip().endswith(":"):
            b["keep"] = True
        elif t == "fig" and i > 0 and blocs[i - 1]["t"] in ("p", "h2", "h3", "h1"):
            blocs[i - 1]["keep"] = True
        elif t == "li" and b["nk"] >= 3 and (b["k"] == 0 or b["k"] == b["nk"] - 2):
            b["keep"] = True
        elif t == "li" and b["nk"] == 2 and b["k"] == 0:
            b["keep"] = True


def paginer(blocs, cap, marge=8):
    """blocs : dicts avec h, pt, keep (et head, rows pour les tableaux). Rend des pages de pièces (i, r0, r1)."""
    cap = cap - marge
    pages, cur = [], []
    used = [0.0]

    def hh(k, premier):
        return blocs[k]["h"] - (blocs[k]["pt"] if premier else 0)

    def nouvelle():
        nonlocal cur
        if cur:
            pages.append(cur)
        cur = []
        used[0] = 0.0

    def poser(k, h, piece=None):
        cur.append((k,) + (piece if piece else (None, None)))
        used[0] += h

    def decoupable(k):
        return blocs[k]["t"] == "table" and blocs[k]["h"] > 520

    def poser_bloc(k):
        b = blocs[k]
        if decoupable(k):
            rows, head = b["hrows"], b["hhead"]
            surcout = b["h"] - head - sum(rows)
            r = 0
            while r < len(rows):
                premier = not cur
                libre = cap - used[0]
                base = surcout - (b["pt"] if premier else 0) + head
                kk, s = r, base
                while kk < len(rows) and s + rows[kk] <= libre:
                    s += rows[kk]
                    kk += 1
                if kk - r < 2 and len(rows) - r >= 2 and cur:
                    nouvelle()
                    continue
                if kk == r:
                    kk = r + 1
                    s = base + rows[r]
                if len(rows) - kk == 1 and kk - r > 2:
                    s -= rows[kk - 1]
                    kk -= 1
                poser(k, s, (r, kk))
                r = kk
                if r < len(rows):
                    nouvelle()
            return
        h = hh(k, not cur)
        if cur and used[0] + h > cap:
            nouvelle()
            h = hh(k, True)
        poser(k, h)

    n = len(blocs)
    i = 0
    while i < n:
        j = i
        while j < n - 1 and blocs[j]["keep"]:
            j += 1
        grp = list(range(i, j + 1))
        i = j + 1
        gh_cur = sum(blocs[k]["h"] for k in grp) - (0 if cur else blocs[grp[0]]["pt"])
        gh_new = sum(blocs[k]["h"] for k in grp) - blocs[grp[0]]["pt"]
        if not decoupable(grp[-1]):
            if used[0] + gh_cur <= cap:
                for k in grp:
                    poser(k, hh(k, not cur))
                continue
            if gh_new <= cap:
                nouvelle()
                for k in grp:
                    poser(k, hh(k, not cur))
                continue
            for k in grp:
                poser_bloc(k)
            continue
        # le groupe se termine par un grand tableau : les blocs qui le précèdent restent avec ses premières lignes
        avant = sum(blocs[k]["h"] for k in grp[:-1])
        tb = blocs[grp[-1]]
        mini = avant + tb["h"] - sum(tb["hrows"]) + sum(tb["hrows"][:2])
        if cur and used[0] + mini - 0 > cap:
            nouvelle()
        for k in grp:
            poser_bloc(k)
    nouvelle()
    return pages


# --- pages ------------------------------------------------------------------------------------------------
def page_flux(n, contenu):
    return (f'<section class="page flux"><div class="corps">{contenu}</div><div class="trait"></div>'
            f'<div class="pied liste a4s"><span>Manuel d\'utilisation</span><span>Kadenz {VERSION}</span>'
            f'<span>{n}</span></div></section>')


def page_plein(interieur):
    return f'<section class="page plein"><div class="v2">{typo(interieur)}</div></section>'


def pied_papier(num=None):
    return (f'<div style="position:absolute;left:56px;right:56px;bottom:100px;height:2px;background:{NUIT}"></div>'
            f'<div class="rang entre liste a4s" style="position:absolute;left:56px;right:56px;bottom:60px">'
            f'<span>Manuel d\'utilisation</span><span>Kadenz {VERSION}</span>'
            + (f"<span>{num}</span>" if num else "") + "</div>")


def couverture(chap):
    titres = [c["titre"] for c in chap] + ["Index"]
    return f"""
<div class="cadre a4 grain" style="background:linear-gradient(170deg,{NUIT} 0%,{NUIT} 70%,#222B66 100%);color:#fff">
  <div style="position:absolute;left:430px;top:340px">{piece_degrade("quart", 640, "cv")}</div>
  <div class="titre" style="position:absolute;left:56px;top:62px;font-size:70px">Manuel<br>d'utilisation</div>
  <div class="chiffre" style="position:absolute;right:56px;top:66px;font-size:46px">{VERSION}</div>
  <div class="liste" style="position:absolute;left:58px;top:262px;font-size:14px;color:{TEXTE_NUIT}">Kadenz et Kadenz Entreprise</div>
  <div style="position:absolute;left:58px;top:318px">{liste(titres, "liste", "font-size:13.5px;line-height:1.38;width:345px;color:#fff;font-weight:500")}</div>
  <div style="position:absolute;left:0;right:0;bottom:0;height:200px;background:{NUIT}"></div>
  <div style="position:absolute;left:56px;bottom:66px">{logo("kadenz-logo-horizontal-negatif.svg", 220)}</div>
  <div class="liste a4s droite" style="position:absolute;right:56px;bottom:62px;color:{TEXTE_NUIT}">Version {VERSION}<br>Gestion commerciale</div>
</div>"""


def sommaire(chap, pages_ouv, page_index):
    lignes = []
    for c in chap:
        lignes.append(f'<div class="rang som"><span class="chiffre" style="width:74px;font-size:30px">{c["num"]}</span>'
                      f'<span class="liste" style="flex:1;font-size:14px;line-height:1.3">{inline(c["titre"])}</span>'
                      f'<span class="chiffre" style="font-size:19px">{pages_ouv[c["num"]]}</span></div>')
    lignes.append(f'<div class="rang som"><span class="chiffre" style="width:74px;font-size:30px"></span>'
                  f'<span class="liste" style="flex:1;font-size:14px;line-height:1.3">Index</span>'
                  f'<span class="chiffre" style="font-size:19px">{page_index}</span></div>')
    return f"""
<div class="cadre a4 papier grain" style="color:{NUIT}">
  <div class="liste a4s" style="position:absolute;left:56px;top:62px">Manuel d'utilisation</div>
  <div style="position:absolute;right:56px;top:56px">{piece_degrade("quart", 90, "sm", INDIGO, NUIT)}</div>
  <div class="titre" style="position:absolute;left:52px;top:130px;font-size:110px">Sommaire</div>
  <div style="position:absolute;left:56px;right:56px;top:292px">{"".join(lignes)}</div>
  {pied_papier(2)}
</div>"""


def quatrieme():
    return f"""
<div class="cadre a4" style="background:{NUIT};color:#fff">
  <div style="position:absolute;left:0;top:0">{motif_tournant(816, 680, 68, 7)}</div>
  <div style="position:absolute;left:56px;top:760px">{logo("kadenz-logo-empile-negatif.svg", 190)}</div>
  <div class="rang" style="position:absolute;left:64px;top:1000px;gap:8px">
    <i style="width:30px;height:4px;background:#fff"></i><i style="width:30px;height:4px;background:#fff"></i><i style="width:30px;height:4px;background:{AMBRE}"></i></div>
  <div class="liste a4s" style="position:absolute;left:64px;bottom:62px;color:{TEXTE_NUIT}">Logiciel propriétaire, tous droits réservés.<br>Mcarre</div>
  <div class="liste a4s droite" style="position:absolute;right:56px;bottom:62px;color:{TEXTE_NUIT}">Kadenz et Kadenz Entreprise<br>Version {VERSION}</div>
</div>"""


# --- ouvertures de chapitre : une construction des pièces du 2026-11-18 par chapitre ----------------------
DOCS_VENTE = ["Devis", "Proforma", "Bons de commande", "Livraisons", "Factures", "Avoirs"]   # section 4.1
POS_PIECE = {"carre": (300, 360), "quart": (250, 330), "triangle": (250, 320), "disque": (260, 360)}


def o_nuit(c, sorte, cle):
    """Manuels 01, 02 et 03 du 2026-11-18 : Nuit, grande pièce en dégradé, bande Nuit en pied."""
    lignes, fs = titre_ajuste(c["titre"].upper(), 480, 70, 4)
    th = len(lignes) * fs * LH
    top_liste = 62 + th + 40
    lfs, lw = 16, 320
    lh = haut_liste(c["labels"], lfs, lw)
    x, y = POS_PIECE[sorte]
    y = min(max(y, top_liste + lh + 30), 640)
    return f"""
<div class="cadre a4 grain" style="background:linear-gradient(170deg,{NUIT} 0%,{NUIT} 70%,#222B66 100%);color:#fff">
  <div style="position:absolute;left:{x}px;top:{y:.0f}px">{piece_degrade(sorte, 640, cle)}</div>
  {bloc_titre(lignes, fs, "position:absolute;left:56px;top:62px")}
  <div class="chiffre" style="position:absolute;right:56px;top:58px;font-size:96px">{c["num"]}</div>
  <div style="position:absolute;left:58px;top:{top_liste:.0f}px">{liste(c["labels"], "liste", f"font-size:{lfs}px;line-height:1.35;width:{lw}px")}</div>
  {pied_nuit(c["num"])}
</div>"""


def o_a3(c):
    """Affiche A3 : grand chiffre, dégradé vertical Brume vers Nuit."""
    lignes, fs = titre_ajuste(c["titre"].upper(), 330, 66, 3)
    lfs, lw = 16, 330
    lh = haut_liste(c["labels"], lfs, lw)
    return f"""
<div class="cadre a4 grain" style="background:linear-gradient(90deg,{BRUME} 0%,{BRUME} 44%,#B9C0E4 56%,{INDIGO} 74%,{NUIT} 100%);color:{NUIT}">
  <div class="titre" style="position:absolute;left:30px;top:20px;font-size:420px;letter-spacing:-.07em;line-height:.9">{c["num"]}</div>
  <div style="position:absolute;right:60px;top:150px;width:150px;height:3px;background:#fff"></div>
  <div style="position:absolute;left:58px;top:455px">{liste(c["labels"], "liste", f"font-size:{lfs}px;line-height:1.35;width:{lw}px")}</div>
  <div style="position:absolute;left:50px;top:{455 + lh + 34:.0f}px">{visee(100, rond=True)}</div>
  {bloc_titre(lignes, fs, "position:absolute;left:58px;bottom:150px")}
  <div style="position:absolute;right:110px;top:900px;color:#fff">{visee(120, rond=True, couleur="#fff")}</div>
  <div class="liste a4s" style="position:absolute;left:58px;bottom:62px">Chapitre {c["num"]} / {NB_CHAP}</div>
  <div class="liste a4s droite" style="position:absolute;right:56px;bottom:62px;color:#fff">Kadenz<br>Version {VERSION}</div>
</div>"""


def o_a5(c):
    """Affiche A5 : bandeau Nuit, quatre pièces en blocs, fiche sur papier."""
    lignes, fs = titre_ajuste(c["titre"].upper(), 540, 90, 3)
    lfs, lw = 17, 318
    return f"""
<div class="cadre a4 papier grain" style="color:{NUIT}">
  <div style="position:absolute;left:0;top:0;right:0;height:360px;background:{NUIT}"></div>
  {bloc_titre(lignes, fs, "position:absolute;left:56px;top:62px;color:#fff")}
  <div style="position:absolute;left:640px;top:62px;color:#fff">{visee(100)}</div>
  <div class="liste a4m" style="position:absolute;left:660px;top:200px;color:#fff;font-size:15px">Chapitre<br>{c["num"]} / {NB_CHAP}</div>
  <div class="blocs" style="position:absolute;left:-50px;top:360px;width:880px;height:800px">
    <div style="left:0;top:0;width:410px;height:380px;background:#C9CEDB"></div>
    <div style="left:460px;top:0">{piece_degrade("quart", 410, "o3q", "#8F98B8", "#3A4472")}</div>
    <div style="left:0;top:430px">{piece_degrade("triangle", 410, "o3t", "#DDE1EA", "#9AA2BC")}</div>
    <div style="left:460px;top:430px">{piece_degrade("disque", 410, "o3d", "#5A648E", NUIT)}</div>
  </div>
  <div class="papier" style="position:absolute;left:0;top:700px;width:430px;height:445px;padding:50px 56px">
    {liste(c["labels"], "liste", f"font-size:{lfs}px;line-height:1.35;width:{lw}px")}
    <div style="margin-top:36px">{visee(80, rond=True)}</div>
  </div>
</div>"""


def o_a2(c):
    """Affiche A2 : motif Tournant, sphère Ambre, préfixes de numérotation (section 4.1)."""
    lignes, fs = titre_ajuste(c["titre"].upper(), 185, 34, 3)
    return f"""
<div class="cadre a4 papier grain" style="color:{NUIT}">
  <div class="liste a4s" style="position:absolute;left:56px;top:62px">{"<br>".join(DOCS_VENTE[:3])}</div>
  <div class="liste a4s" style="position:absolute;left:300px;top:62px">{"<br>".join(DOCS_VENTE[3:])}</div>
  <div style="position:absolute;left:380px;top:150px;width:440px;height:440px;border-radius:50%;
    background:radial-gradient(circle at 40% 40%,#EDB65A 0%,{AMBRE} 55%,#D58F27 100%)"></div>
  <div style="position:absolute;left:0;top:250px;width:480px;height:630px;overflow:hidden">{motif_tournant(480, 630, 70, 7)}
    <div style="position:absolute;left:380px;top:-130px;width:440px;height:440px;border-radius:50%;background:{AMBRE};opacity:.5;mix-blend-mode:screen"></div></div>
  <div class="prefixes" style="position:absolute;left:730px;top:640px;font-size:58px">D<br>F<br>L<br>A</div>
  <div style="position:absolute;left:520px;top:820px">{visee(90, rond=True)}</div>
  {bloc_titre(lignes, fs, "position:absolute;left:504px;top:640px")}
  <div style="position:absolute;left:56px;bottom:170px;width:400px">
    <div class="regle"><span>D</span><span>2026</span><span>11</span><span>0001</span></div>
    <div class="regle-trait"><i></i><i></i><i></i><i></i></div>
    <div class="regle bas-l"><span>Lettre</span><span>Année</span><span>Mois</span><span>Compteur</span></div>
  </div>
  <div class="liste a4s" style="position:absolute;left:56px;bottom:100px">Numéros / D2026-11-0001</div>
  <div class="rang" style="position:absolute;right:56px;bottom:56px;gap:22px">{logo("kadenz-symbole.svg", 80)}<div class="chiffre" style="font-size:76px">{c["num"]}</div></div>
</div>"""


def o_a4(c):
    """Affiche A4 : disque en trame de points, repères de visée (formats de la section 7.1)."""
    lignes, fs = titre_ajuste(c["titre"].upper(), 480, 100, 5, hmax=330)
    return f"""
<div class="cadre a4 papier grain" style="color:{NUIT}">
  {bloc_titre(lignes, fs, "position:absolute;left:52px;top:62px")}
  <div class="liste a4s" style="position:absolute;right:56px;top:76px">Formats lus<br>CSV<br>Excel (xlsx, xlsm, xls)<br>OpenDocument (ods)</div>
  <div style="position:absolute;right:56px;top:420px;width:190px;height:3px;background:{NUIT}"></div>
  <div style="position:absolute;right:142px;top:372px;width:3px;height:100px;background:{NUIT}"></div>
  <div style="position:absolute;left:210px;top:430px">{trame_disque(540, 14)}</div>
  <div style="position:absolute;left:56px;top:870px">{visee(84, rond=True)}</div>
  <div style="position:absolute;left:0;right:0;top:975px;height:170px;border-top:3px solid {NUIT}"></div>
  <div style="position:absolute;left:56px;top:998px">{liste(c["labels"], "liste", "font-size:19px;line-height:1.5;width:480px")}</div>
  <div class="chiffre" style="position:absolute;left:610px;top:990px;font-size:72px">{c["num"]}</div>
  <div style="position:absolute;right:56px;top:1005px">{temps_vertical(26, 12, NUIT)}</div>
  <div class="liste a4s droite" style="position:absolute;right:56px;top:190px">Chapitre {c["num"]} / {NB_CHAP}</div>
</div>"""


def o_d2(c):
    """Diapositive D2 : intercalaire, grand chiffre, dégradé vers la Nuit."""
    lignes, fs = titre_ajuste(c["titre"].upper(), 380, 70, 3)
    top = 600
    top_liste = top + len(lignes) * fs * LH + 34
    return f"""
<div class="cadre a4 grain" style="background:linear-gradient(90deg,{BRUME} 0%,{BRUME} 52%,#B9C0E4 62%,{INDIGO} 80%,{NUIT} 100%);color:{NUIT}">
  <div class="titre" style="position:absolute;left:30px;top:10px;font-size:350px;letter-spacing:-.07em;line-height:.9">{c["num"]}</div>
  {bloc_titre(lignes, fs, f"position:absolute;left:58px;top:{top}px")}
  <div style="position:absolute;left:58px;top:{top_liste:.0f}px">{liste(c["labels"], "liste", "font-size:16px;line-height:1.35;width:340px")}</div>
  <div style="position:absolute;left:340px;top:470px">{visee(90, rond=True)}</div>
  <div style="position:absolute;right:90px;top:90px;width:170px;height:3px;background:#fff"></div>
  <div style="position:absolute;right:110px;top:380px;color:#fff">{visee(120, rond=True, couleur="#fff")}</div>
  <div class="liste a4s droite" style="position:absolute;right:56px;bottom:62px;color:#fff">Chapitre {c["num"]} / {NB_CHAP}<br>Kadenz {VERSION}</div>
</div>"""


def o_a1(c):
    """Affiche A1 : champ dégradé Nuit, Indigo, Ambre."""
    lignes, fs = titre_ajuste(c["titre"].upper(), 560, 72, 4)
    top_liste = 62 + len(lignes) * fs * LH + 60
    return f"""
<div class="cadre a4 grain" style="background:linear-gradient(162deg,{NUIT} 0%,{NUIT} 34%,{INDIGO} 56%,{AMBRE} 78%,#F6E3C2 90%,{BRUME} 100%);color:#fff">
  {bloc_titre(lignes, fs, "position:absolute;left:56px;top:62px")}
  <div class="chiffre" style="position:absolute;right:56px;top:66px;font-size:64px">{c["num"]}</div>
  <div style="position:absolute;left:58px;top:{top_liste:.0f}px">{liste(c["labels"], "liste", "font-size:19px;line-height:1.35;width:430px")}</div>
  <div style="position:absolute;right:110px;top:760px">{visee(110, point=True, couleur=NUIT)}</div>
  <div style="position:absolute;left:58px;bottom:60px;right:56px;color:{NUIT}" class="rang bas">
    <div class="pile" style="gap:20px">{logo("kadenz-mot.svg", 250)}<div class="liste a4s">Chapitre {c["num"]} / {NB_CHAP}</div></div>
    <div class="liste a4s droite">Version {VERSION}</div>
    {logo("kadenz-symbole.svg", 90)}
  </div>
</div>"""


def o_d1(c):
    """Diapositive D1 : titre en grand, dégradé oblique Nuit vers Ambre et Brume, logo en tête."""
    lignes, fs = titre_ajuste(c["titre"].upper(), 560, 74, 4)
    top = 250
    top_liste = top + len(lignes) * fs * LH + 50
    return f"""
<div class="cadre a4 grain" style="background:linear-gradient(118deg,{NUIT} 0%,{NUIT} 42%,{INDIGO} 62%,{AMBRE} 82%,#F6E3C2 92%,{BRUME} 100%);color:#fff">
  <div style="position:absolute;left:56px;top:62px">{logo("kadenz-logo-horizontal-negatif.svg", 220)}</div>
  <div class="chiffre" style="position:absolute;right:56px;top:58px;font-size:56px">{c["num"]}</div>
  {bloc_titre(lignes, fs, f"position:absolute;left:52px;top:{top}px")}
  <div style="position:absolute;left:58px;top:{top_liste:.0f}px">{liste(c["labels"], "liste", "font-size:17px;line-height:1.4;width:400px")}</div>
  <div style="position:absolute;right:110px;top:800px">{visee(110, point=True, couleur=NUIT)}</div>
  <div class="liste a4s droite" style="position:absolute;right:56px;bottom:62px;color:{NUIT}">Chapitre {c["num"]} / {NB_CHAP}<br>Version {VERSION}</div>
</div>"""


def o_brume(c, sorte, cle, variante=0):
    """Page de chapitre du manuel 02 : papier Brume, grand chiffre."""
    lignes, fs = titre_ajuste(c["titre"].upper(), 700, 110, 3)
    top_t = 500
    top_liste = top_t + len(lignes) * fs * 0.9 + 44
    visee_pos = (600, 640) if variante == 0 else (640, 430)
    return f"""
<div class="cadre a4 papier grain" style="color:{NUIT}">
  <div class="liste a4s" style="position:absolute;left:56px;top:62px">Chapitre {c["num"]} / {NB_CHAP}</div>
  <div style="position:absolute;right:56px;top:56px">{piece_degrade(sorte, 90, cle, INDIGO, NUIT)}</div>
  <div class="titre" style="position:absolute;left:40px;top:110px;font-size:380px;letter-spacing:-.07em;line-height:.9">{c["num"]}</div>
  {bloc_titre(lignes, fs, f"position:absolute;left:56px;top:{top_t}px")}
  <div style="position:absolute;left:58px;top:{top_liste:.0f}px">{liste(c["labels"], "liste", "font-size:19px;line-height:1.35;width:520px")}</div>
  <div style="position:absolute;left:{visee_pos[0]}px;top:{visee_pos[1]}px">{visee(80, rond=True)}</div>
  {pied_papier()}
</div>"""


def o_tourn_haut(c):
    """Quatrième de couverture du 2026-11-18 : motif Tournant en tête, Nuit."""
    lignes, fs = titre_ajuste(c["titre"].upper(), 500, 70, 3)
    top = 680
    top_liste = top + len(lignes) * fs * LH + 34
    lh = haut_liste(c["labels"], 16, 640, 1.38)
    barres = min(top_liste + lh + 30, 1040)
    return f"""
<div class="cadre a4" style="background:{NUIT};color:#fff">
  <div style="position:absolute;left:0;top:0">{motif_tournant(816, 612, 68, 7)}</div>
  {bloc_titre(lignes, fs, f"position:absolute;left:56px;top:{top}px")}
  <div class="chiffre" style="position:absolute;right:56px;top:{top - 4}px;font-size:96px">{c["num"]}</div>
  <div style="position:absolute;left:58px;top:{top_liste:.0f}px">{liste(c["labels"], "liste", f"font-size:16px;line-height:1.38;width:640px;color:{TEXTE_NUIT}")}</div>
  <div class="rang" style="position:absolute;left:64px;top:{barres:.0f}px;gap:8px">
    <i style="width:30px;height:4px;background:#fff"></i><i style="width:30px;height:4px;background:#fff"></i><i style="width:30px;height:4px;background:{AMBRE}"></i></div>
  <div class="liste a4s" style="position:absolute;left:64px;bottom:62px;color:{TEXTE_NUIT}">Chapitre {c["num"]} / {NB_CHAP}</div>
  <div class="liste a4s droite" style="position:absolute;right:56px;bottom:62px;color:{TEXTE_NUIT}">Kadenz<br>Version {VERSION}</div>
</div>"""


def o_tourn_bas(c):
    """Même construction, motif Tournant en pied (graine différente), titre en tête."""
    lignes, fs = titre_ajuste(c["titre"].upper(), 480, 70, 3)
    th = len(lignes) * fs * LH
    top_liste = 62 + th + 40
    lh = haut_liste(c["labels"], 17, 560)
    haut_motif = max(480, top_liste + lh + 40)
    return f"""
<div class="cadre a4" style="background:{NUIT};color:#fff">
  <div style="position:absolute;left:0;top:{haut_motif:.0f}px;width:816px;height:{945 - haut_motif:.0f}px;overflow:hidden">{motif_tournant(816, 945 - round(haut_motif), 68, 11)}</div>
  {bloc_titre(lignes, fs, "position:absolute;left:56px;top:62px")}
  <div class="chiffre" style="position:absolute;right:56px;top:58px;font-size:96px">{c["num"]}</div>
  <div style="position:absolute;left:58px;top:{top_liste:.0f}px">{liste(c["labels"], "liste", f"font-size:17px;line-height:1.35;width:560px;color:{TEXTE_NUIT}")}</div>
  {pied_nuit(c["num"])}
</div>"""


def o_index(lettres):
    return f"""
<div class="cadre a4 papier grain" style="color:{NUIT}">
  <div class="liste a4s" style="position:absolute;left:56px;top:62px">Manuel d'utilisation</div>
  <div style="position:absolute;right:56px;top:56px">{piece_degrade("quart", 90, "ix", INDIGO, NUIT)}</div>
  <div class="titre" style="position:absolute;left:44px;top:130px;font-size:200px;letter-spacing:-.05em;line-height:.9">Index</div>
  <div class="liste" style="position:absolute;left:58px;top:430px;font-size:34px;line-height:1.5;width:620px;letter-spacing:.3em">{" ".join(lettres)}</div>
  <div style="position:absolute;left:600px;top:640px">{visee(80, rond=True)}</div>
  {pied_papier()}
</div>"""


OUVERTURES = {
    "01": lambda c: o_nuit(c, "carre", "o01"),
    "02": o_a3,
    "03": o_a5,
    "04": o_a2,
    "05": lambda c: o_brume(c, "quart", "o05"),
    "06": lambda c: o_nuit(c, "quart", "o06"),
    "07": o_a4,
    "08": o_d2,
    "09": o_a1,
    "10": o_tourn_haut,
    "11": lambda c: o_nuit(c, "disque", "o11"),
    "12": o_d1,
    "13": lambda c: o_nuit(c, "triangle", "o13"),
    "14": lambda c: o_brume(c, "disque", "o14", 1),
    "15": o_tourn_bas,
}


# --- styles ------------------------------------------------------------------------------------------------
GRAIN_URI = ("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='300' height='300'%3E%3Cfilter id='n'%3E"
             "%3CfeTurbulence type='fractalNoise' baseFrequency='.9' numOctaves='3' stitchTiles='stitch'/%3E"
             "%3CfeColorMatrix type='saturate' values='0'/%3E%3C/filter%3E%3Crect width='300' height='300' filter='url(%23n)'/%3E%3C/svg%3E")
POL = "../../../crates/app/ui/polices"

CSS = Template("""
@font-face{font-family:Diatype;src:url($POL/ABCDiatype-Regular.otf);font-weight:400}
@font-face{font-family:Diatype;src:url($POL/ABCDiatype-Medium.otf);font-weight:500}
@font-face{font-family:Diatype;src:url($POL/ABCDiatype-Bold.otf);font-weight:700}
@font-face{font-family:"Diatype Mono";src:url($POL/ABCDiatypeMono-Regular.otf);font-weight:400}
@page{size:210mm 297mm;margin:0}
*{box-sizing:border-box;margin:0;padding:0}
em,i,cite,address,dfn,var{font-style:normal}
i{display:block}
html,body{-webkit-print-color-adjust:exact;print-color-adjust:exact}
body{font-family:Diatype,sans-serif;color:$ENCRE;background:#fff}
.page{position:relative;width:210mm;height:297mm;overflow:hidden;break-after:page;background:#fff}
.page:last-child{break-after:auto}
.plein .v2{position:absolute;left:${DEC}px;top:0;width:816px;height:1145px;transform:scale($ECH);transform-origin:0 0}
.cadre{position:relative;overflow:hidden}
.a4{width:816px;height:1145px}
.papier{background:$BRUME}
$GRAINCSS
.titre{font-weight:700;text-transform:uppercase;letter-spacing:-.035em;line-height:.88;word-spacing:.12em}
.chiffre{font-weight:500;font-variant-numeric:tabular-nums;letter-spacing:-.02em;line-height:1}
.liste{text-transform:uppercase;font-weight:500;letter-spacing:.07em;font-variant-numeric:tabular-nums;line-height:1.22}
.liste.a4xl{font-size:30px;line-height:1.3}.liste.a4l{font-size:21px;line-height:1.35}
.liste.a4m{font-size:18px;line-height:1.4}.liste.a4s{font-size:13px;line-height:1.35}
.droite{text-align:right}
.rang{display:flex;align-items:center}
.rang.bas{align-items:flex-end;justify-content:space-between}
.entre{justify-content:space-between}
.pile{display:flex;flex-direction:column}
.rond{border-radius:50%;display:block}
.prefixes{font-weight:700;font-size:80px;line-height:.98;text-align:center}
.regle{display:grid;grid-template-columns:1fr 1fr 1fr 1fr;font-weight:500;font-size:26px;letter-spacing:.04em;font-variant-numeric:tabular-nums}
.regle span:nth-child(2),.regle span:nth-child(3){text-align:center}.regle span:last-child{text-align:right}
.regle.bas-l{font-size:15px;text-transform:uppercase;letter-spacing:.1em}
.regle-trait{position:relative;height:26px;margin:8px 0;border-bottom:3px solid currentColor;transform:translateY(-12px);
  display:flex;justify-content:space-between}
.regle-trait i{width:3px;height:26px;background:currentColor;transform:translateY(13px)}
.blocs>div{position:absolute}
.blocs svg{display:block}
.som{height:46px;border-bottom:1px solid $TRAIT}
.som:last-child{border-bottom:0}

/* pages courantes : texte noir sur blanc, sans en-tête courant */
.flux .corps{position:absolute;left:${MARGE}px;right:${MARGE}px;top:${HT}px;height:${CAP}px}
.flux .trait{position:absolute;left:${MARGE}px;right:${MARGE}px;bottom:98px;height:2px;background:$NUIT}
.flux .pied{position:absolute;left:${MARGE}px;right:${MARGE}px;bottom:58px;color:$NUIT;display:flex;justify-content:space-between}
.b{display:flow-root}
.b.prem{padding-top:0}
.b-h1{padding-bottom:26px}
.titre-ch{display:flex;justify-content:space-between;align-items:flex-start;gap:24px}
.titre-ch h1{color:$NUIT;max-width:590px}
.titre-ch .chiffre{font-size:54px;color:$NUIT}
.b-h2{padding-top:30px;padding-bottom:2px}
.b-h2 h2{font-weight:500;text-transform:uppercase;letter-spacing:.06em;font-size:16.5px;line-height:1.3;color:$NUIT}
.b-h2 .sn{display:inline-block;min-width:42px;font-variant-numeric:tabular-nums}
.b-h3{padding-top:20px}
.b-h3 h3{font-weight:700;font-size:16px;line-height:1.3;color:$NUIT}
.b-p{padding-top:10px}
p,li{font-size:14.5px;line-height:1.5}
.b-li{padding-top:5px}
.b-li.l0{padding-top:9px}.b-li.lN{padding-bottom:4px}
ul,ol{padding-left:22px}
li::marker{color:$NUIT;font-weight:700}
li>ul,li>ol{padding-top:3px}
b{font-weight:700}
code{font-family:"Diatype Mono",monospace;font-size:.92em;overflow-wrap:break-word}
.b-fig{padding-top:14px;padding-bottom:6px}
.cadre-capture{border:1px solid $TRAIT;background:#fff;border-radius:2px;overflow:hidden}
.cadre-capture img{display:block;width:100%;height:auto}
.fig{width:$FIGW%;margin:0 auto}
.fig figcaption{font-size:11.5px;color:$ARDOISE;margin-top:5px;line-height:1.3}
.b-note{padding-top:14px}
.note{padding:13px 18px 14px;background:$BRUME;border-radius:2px}
.note b.lab{display:block;font-size:15px;color:$ENCRE;margin-bottom:3px}
.note p{font-size:14.5px;line-height:1.45}
.note p+p{margin-top:5px}
.b-table{padding-top:14px;padding-bottom:6px}
table{width:100%;border-collapse:collapse;font-size:13px;line-height:1.4}
th{background:$BRUME;color:$NUIT;font-weight:700;text-align:left;padding:7px 10px;vertical-align:top}
td{padding:7px 10px;border-bottom:1px solid $TRAIT;vertical-align:top}
/* index */
.cols{display:grid;grid-template-columns:1fr 1fr;gap:36px;position:absolute;left:${MARGE}px;right:${MARGE}px;top:${HT}px;height:${CAP}px}
.b-il{padding-top:20px;padding-bottom:4px}
.b-il span{font-weight:700;font-size:26px;color:$NUIT;letter-spacing:-.01em;line-height:1}
.b-ie{display:flex;gap:10px;justify-content:space-between;padding-top:5px;font-size:13.5px;line-height:1.35}
.b-ie .t{flex:1}
.b-ie .n{text-align:right;font-weight:500;color:$NUIT;font-variant-numeric:tabular-nums;max-width:45%}
.b-ip{padding-bottom:10px;font-size:13.5px;line-height:1.4;color:$ARDOISE}
/* planche */
body.planche{background:$BRUME;width:2640px;padding:80px}
.planche h1{font-size:56px;color:$NUIT;letter-spacing:-.01em;font-weight:700}
.planche .intro{font-size:24px;line-height:1.5;color:$ARDOISE;max-width:2000px;margin-top:16px}
.grille{display:grid;grid-template-columns:repeat(8,292px);gap:34px 20px;margin-top:50px}
.grille figure{margin:0}
.mini{width:292px;height:413px;overflow:hidden;position:relative;box-shadow:0 0 0 1px $TRAIT;background:#fff}
.mini .page{position:absolute;left:0;top:0;transform:scale(.3679);transform-origin:0 0;break-after:auto}
.grille figcaption{font-size:18px;color:$ARDOISE;margin-top:8px;line-height:1.3}
.grille figcaption b{color:$NUIT;font-weight:500}
.planche .pied-planche{margin-top:56px;font-size:21px;color:$ARDOISE;line-height:1.5;max-width:2100px}
""")


def feuille(grain=False):
    graincss = (f'.grain::after{{content:"";position:absolute;inset:0;background:url("{GRAIN_URI}");opacity:.16;'
                f'mix-blend-mode:overlay;pointer-events:none}}\n'
                f'.papier.grain::after{{opacity:.10;mix-blend-mode:multiply}}') if grain else ""
    return CSS.substitute(NUIT=NUIT, INDIGO=INDIGO, AMBRE=AMBRE, BRUME=BRUME, ENCRE=ENCRE, ARDOISE=ARDOISE,
                          TRAIT=TRAIT, POL=POL, GRAINCSS=graincss, DEC=f"{-DECALAGE:.3f}", ECH=f"{ECHELLE:.5f}",
                          MARGE=MARGE, HT=HAUT_TOP, FIGW=FIG_LARGEUR, CAP=f"{CAP:.2f}")


def chrome(*args, timeout=900):
    return subprocess.run([CHROME, "--headless=new", "--hide-scrollbars", "--allow-file-access-from-files",
                           "--virtual-time-budget=20000", *args],
                          check=True, capture_output=True, text=True, encoding="utf-8", timeout=timeout)


SCRIPT_MESURE = """<script>
Promise.all(['400 14px Diatype','500 14px Diatype','700 14px Diatype','400 14px "Diatype Mono"'].map(x=>document.fonts.load(x))).then(()=>document.fonts.ready).then(()=>{
 const out={};
 document.querySelectorAll('#m .b').forEach(b=>{
  const r={h:b.getBoundingClientRect().height,pt:parseFloat(getComputedStyle(b).paddingTop)};
  if(b.classList.contains('b-table')){const t=b.querySelector('table');
    r.hhead=t.tHead.rows[0].getBoundingClientRect().height;
    r.hrows=[...t.tBodies[0].rows].map(x=>x.getBoundingClientRect().height);
    r.larg=t.scrollWidth;}
  out[b.dataset.i]=r;});
 const p=document.createElement('pre');p.id='res';p.textContent=JSON.stringify(out);document.body.appendChild(p);});
</script>"""


def mesurer(blocs_html, largeur):
    """blocs_html : liste de (indice, html). Rend {indice: mesures}."""
    corps = "".join(h.replace('class="b ', f'data-i="{i}" class="b ', 1) for i, h in blocs_html)
    page = (f'<!doctype html><html lang="fr"><head><meta charset="utf-8"><style>{feuille()}</style></head>'
            f'<body><div id="m" style="width:{largeur}px">{corps}</div>{SCRIPT_MESURE}</body></html>')
    tmp = ICI / "_mesure.html"
    tmp.write_text(typo_doc(page), encoding="utf-8")
    sortie = chrome("--window-size=1000,1000", "--dump-dom", tmp.as_uri()).stdout
    tmp.unlink()
    m = re.search(r'<pre id="res">(.*?)</pre>', sortie, re.S)
    return {int(k): v for k, v in json.loads(_html.unescape(m.group(1))).items()}


def typo_doc(page):
    return page


# --- assemblage --------------------------------------------------------------------------------------------
def construire():
    chap = charger_chapitres()
    intro_index, entrees_index = charger_index()

    # 1. blocs et mesures des chapitres
    tous = []   # (chap, bloc) dans l'ordre
    for ci, c in enumerate(chap):
        c["blocs"] = [{"t": "h1"}] + c["blocs"]
        marquer_garde(c["blocs"])
        for b in c["blocs"]:
            tous.append((ci, b))
    html_blocs = [(i, typo(rendu_bloc(b, chap[ci]))) for i, (ci, b) in enumerate(tous)]
    mes = mesurer(html_blocs, LARG)
    for i, (ci, b) in enumerate(tous):
        b.update(mes[i])

    # 2. pagination par chapitre, numéros de page
    page = 3                       # 1 couverture, 2 sommaire
    pages_ouv, plan, sections = {}, [], {}
    for c in chap:
        pages_ouv[c["num"]] = page
        page += 1
        c["pages"] = paginer(c["blocs"], CAP)
        c["page0"] = page
        for pg in c["pages"]:
            for (k, r0, r1) in pg:
                b = c["blocs"][k]
                if b["t"] == "h2":
                    m = re.match(r"(\d+\.\d+)\s", b["x"])
                    if m:
                        sections[m.group(1)] = page
            page += 1
    page_index = page

    # 3. index : numéros de page réels, mesure à la largeur d'une colonne
    larg_col = (LARG - 36) / 2
    blocs_idx, manquantes = [], []
    if intro_index:
        blocs_idx.append({"t": "ip", "x": "Les numéros renvoient aux pages du manuel."})
    for kind, a, b in entrees_index:
        if kind == "lettre":
            blocs_idx.append({"t": "il", "x": a, "keep": True})
        else:
            pages_e = []
            for s in b:
                if s in sections:
                    pages_e.append(sections[s])
                else:
                    manquantes.append((a, s))
            pages_e = sorted(set(pages_e))
            blocs_idx.append({"t": "ie", "terme": a, "pages": ", ".join(str(p) for p in pages_e), "keep": False})
    # le texte d'introduction de l'index.md est remplacé (voir LISEZMOI) : il annonçait des numéros de section
    h_idx = []
    for i, b in enumerate(blocs_idx):
        if b["t"] == "ip":
            h_idx.append((i, f'<div class="b b-ip">{b["x"]}</div>'))
        else:
            h_idx.append((i, typo(rendu_bloc(b, None))))
    mes_i = mesurer(h_idx, larg_col)
    for i, b in enumerate(blocs_idx):
        b.update(mes_i[i])
        b.setdefault("keep", False)
    colonnes = paginer(blocs_idx, CAP)   # chaque "page" est une colonne
    pages_index = [colonnes[i:i + 2] for i in range(0, len(colonnes), 2)]
    nb_index = len(pages_index)
    total = page_index + 1 + nb_index + 1   # ouverture de l'index, pages d'index, quatrième
    lettres = [a for kind, a, _ in entrees_index if kind == "lettre"]
    return dict(chap=chap, pages_ouv=pages_ouv, page_index=page_index, sections=sections, blocs_idx=blocs_idx,
                pages_index=pages_index, total=total - 0, lettres=lettres, manquantes=manquantes,
                entrees_index=entrees_index)


def assembler(d, grain):
    """Rend la liste de toutes les pages : [(numero, genre, titre, html)]."""
    global GRAIN
    GRAIN = grain
    chap = d["chap"]
    pages = [(1, "couverture", "Couverture", page_plein(couverture(chap))),
             (2, "sommaire", "Sommaire", page_plein(sommaire(chap, d["pages_ouv"], d["page_index"])))]
    n = 3
    for c in chap:
        pages.append((n, "ouverture", f'{c["num"]} {c["titre"]}', page_plein(OUVERTURES[c["num"]](c))))
        n += 1
        for pg in c["pages"]:
            contenu = ""
            for j, (k, r0, r1) in enumerate(pg):
                b = c["blocs"][k]
                piece = (r0, r1) if r0 is not None else None
                contenu += typo(rendu_bloc(b, c, piece, premier=(j == 0)))
            figs = any(c["blocs"][k]["t"] == "fig" for (k, _, _) in pg)
            pages.append((n, "texte-fig" if figs else "texte", f'{c["num"]} {c["titre"]}', page_flux(n, contenu)))
            n += 1
    pages.append((n, "ouverture", "Index", page_plein(o_index(d["lettres"]))))
    n += 1
    for cols in d["pages_index"]:
        html_cols = ""
        for col in cols:
            html_cols += "<div>" + "".join(
                typo(rendu_bloc(d["blocs_idx"][k], None, None, premier=(j == 0))) if d["blocs_idx"][k]["t"] != "ip"
                else f'<div class="b b-ip{" prem" if j == 0 else ""}">{d["blocs_idx"][k]["x"]}</div>'
                for j, (k, _, _) in enumerate(col)) + "</div>"
        pages.append((n, "index", "Index", f'<section class="page flux"><div class="cols">{html_cols}</div>'
                      f'<div class="trait"></div><div class="pied liste a4s"><span>Manuel d\'utilisation</span>'
                      f'<span>Kadenz {VERSION}</span><span>{n}</span></div></section>'))
        n += 1
    pages.append((n, "quatrieme", "Quatrième de couverture", page_plein(quatrieme())))
    return pages


SCRIPT_CONTROLE = """<script>
document.fonts.ready.then(()=>{
 const res={deb:[],larg:[]};
 document.querySelectorAll('.page').forEach((p,i)=>{
  const c=p.querySelector('.corps,.cols');
  if(c){const top=c.getBoundingClientRect().top;
    let bas=0;c.querySelectorAll('.b').forEach(b=>{bas=Math.max(bas,b.getBoundingClientRect().bottom-top)});
    const lim=c.getBoundingClientRect().height;
    if(bas>lim+0.5)res.deb.push([i+1,Math.round(bas-lim)]);
    c.querySelectorAll('table').forEach(t=>{if(t.scrollWidth>t.clientWidth+1)res.larg.push([i+1,'table'])});
    c.querySelectorAll('.b').forEach(b=>{if(b.scrollWidth>b.clientWidth+1)res.larg.push([i+1,'bloc'])});}
 });
 const p=document.createElement('pre');p.id='res';p.textContent=JSON.stringify(res);document.body.appendChild(p);});
</script>"""


def sauver_pdf(pages, nom_pdf):
    corps = "".join(h for _, _, _, h in pages)
    tmp = ICI / "_final.html"
    tmp.write_text(doc_html(corps, "Manuel d'utilisation") , encoding="utf-8")
    pdf = ICI / nom_pdf
    chrome("--no-pdf-header-footer", f"--print-to-pdf={pdf}", tmp.as_uri(), timeout=1800)
    # contrôle de débordement
    tmp.write_text(doc_html(corps, "Manuel d'utilisation").replace("</body>", SCRIPT_CONTROLE + "</body>"),
                   encoding="utf-8")
    sortie = chrome("--window-size=1000,1000", "--dump-dom", tmp.as_uri()).stdout
    tmp.unlink()
    res = json.loads(_html.unescape(re.search(r'<pre id="res">(.*?)</pre>', sortie, re.S).group(1)))
    nb = len(re.findall(rb"/Type\s*/Page[^s]", pdf.read_bytes()))
    return pdf, res, nb


def choisir_planche(pages, chap):
    """Quelques pages par chapitre : ouverture, première page de texte, deux pages à captures."""
    par_titre = {}
    for n, genre, titre, _ in pages:
        par_titre.setdefault(titre, []).append((n, genre))
    choix = [1, 2]
    for c in chap:
        cle = f'{c["num"]} {c["titre"]}'
        liste_ = par_titre[cle]
        ouv = [n for n, g in liste_ if g == "ouverture"]
        texte = [n for n, g in liste_ if g in ("texte", "texte-fig")]
        figs = [n for n, g in liste_ if g == "texte-fig"]
        pick = ouv + texte[:1]
        rest = [n for n in figs if n not in pick]
        if rest:
            pick.append(rest[len(rest) // 2])
            if len(rest) > 3:
                pick.append(rest[-1])
        elif len(texte) > 1:
            pick.append(texte[len(texte) // 2])
        choix += pick
    choix += [n for n, g, t, _ in pages if g in ("index", "quatrieme") or t == "Index"]
    return sorted(set(choix))


def planche(pages, chap, total, nom="planche-manuel-complet.png"):
    choix = choisir_planche(pages, chap)
    parN = {n: (g, t, h) for n, g, t, h in pages}
    figs = ""
    for n in choix:
        g, t, h = parN[n]
        figs += f'<figure><div class="mini">{h}</div><figcaption><b>{n}</b> {_html.escape(t)}</figcaption></figure>'
    corps = (f'<h1>Manuel d\'utilisation de Kadenz, version complète</h1>'
             f'<p class="intro">Mise en page finale des quinze chapitres, de l\'index et des pages de couverture : {total} pages A4 '
             f'au total, dont {len(choix)} montrées ici (couverture, sommaire, ouverture de chaque chapitre, première page de texte, '
             f'pages à captures, index, quatrième). Version {VERSION}.</p>'
             f'<div class="grille">{figs}</div>'
             f'<p class="pied-planche">Planche de contrôle avec grain ; le PDF est sans grain. Polices ABC Diatype (essai) lues dans le dépôt privé.</p>')
    html = (f'<!doctype html>\n<html lang="fr"><head><meta charset="utf-8"><title>Planche du manuel complet | Kadenz</title>'
            f'<style>{feuille(True)}</style></head><body class="planche">{corps}</body></html>\n')
    tmp = ICI / "_planche.html"
    tmp.write_text(html.replace("</body>", "<script>document.fonts.ready.then(()=>{const p=document.createElement('pre');"
                                "p.id='res';p.textContent=document.documentElement.scrollHeight;document.body.appendChild(p)})</script></body>"),
                   encoding="utf-8")
    h = int(re.search(r'<pre id="res">(\d+)</pre>', chrome("--window-size=2640,1000", "--dump-dom", tmp.as_uri()).stdout).group(1))
    tmp.write_text(html, encoding="utf-8")
    chrome("--force-device-scale-factor=1", f"--window-size=2640,{h}", f"--screenshot={ICI / nom}", tmp.as_uri())
    tmp.unlink()
    return len(choix), h


def apercus(pages, numeros, grain=True):
    dossier = ICI / "_apercus"
    dossier.mkdir(exist_ok=True)
    parN = {n: h for n, _, _, h in pages}
    for k in range(0, len(numeros), 2):
        lot = numeros[k:k + 2]
        css = ".page{break-after:auto!important;flex:none}"
        html = (f'<!doctype html><html lang="fr"><head><meta charset="utf-8"><style>{feuille(grain)}{css}'
                f'body{{display:flex;gap:12px;background:#888}}</style></head><body>{"".join(parN[n] for n in lot)}</body></html>')
        tmp = ICI / "_apercu.html"
        tmp.write_text(html, encoding="utf-8")
        nom = "-".join(str(n) for n in lot)
        chrome("--force-device-scale-factor=1", f"--window-size={int((PAGE_W + 12) * len(lot))},{int(PAGE_H) + 2}",
               f"--screenshot={dossier / (nom + '.png')}", tmp.as_uri())
        tmp.unlink()


def controle_textes(d):
    """Défauts constatés dans les textes (rien n'est corrigé dans docs/manuel)."""
    chap = d["chap"]
    out = []
    h2 = {}
    for c in chap:
        for b in c["blocs"]:
            if b["t"] == "h2":
                m = re.match(r"(\d+\.\d+)\s+(.*)", b["x"])
                if m:
                    h2[m.group(1)] = m.group(2)
                else:
                    out.append(f'{c["fichier"]} : titre de section sans numéro : {b["x"]}')
    # numéro de section cohérent avec le fichier
    for c in chap:
        for b in c["blocs"]:
            if b["t"] == "h2":
                m = re.match(r"(\d+)\.", b["x"])
                if m and int(m.group(1)) != int(c["num"]):
                    out.append(f'{c["fichier"]} : la section {b["x"]} ne porte pas le numéro du chapitre {c["num"]}')
    # renvois
    for c in chap:
        textes = []
        for b in c["blocs"]:
            for cle in ("x",):
                if cle in b and isinstance(b[cle], str):
                    textes.append(b[cle])
            if b.get("t") == "li":
                textes += b["sub"]
            if b.get("t") == "note":
                textes += b["paras"]
            if b.get("t") == "table":
                for r in b["rows"]:
                    textes += r
        for t in textes:
            for m in re.finditer(r"chapitres?\s+(\d{1,2})(?!\d)", t):
                if not 1 <= int(m.group(1)) <= NB_CHAP:
                    out.append(f'{c["fichier"]} : renvoi au chapitre {m.group(1)}, qui n\'existe pas : {t[:80]}')
            for m in re.finditer(r"sections?\s+((?:\d+\.\d+(?:\s*(?:,|et|à)\s*)?)+)", t):
                for s in re.findall(r"\d+\.\d+", m.group(1)):
                    if s not in h2:
                        out.append(f'{c["fichier"]} : renvoi à la section {s}, qui n\'existe pas')
            # renvoi "chapitre N, section N.M" : le chapitre et la section doivent concorder
            for m in re.finditer(r"chapitre\s+(\d{1,2}),\s*sections?\s+(\d+)\.\d+", t):
                if int(m.group(1)) != int(m.group(2)):
                    out.append(f'{c["fichier"]} : renvoi incohérent « {m.group(0)} »')
    vus = set()
    for a, s in d["manquantes"]:
        out.append(f'index.md : l\'entrée « {a} » renvoie à la section {s}, absente du manuel')
    cites = set()
    for kind, a, nums in d["entrees_index"]:
        cites.update(nums)
    non_cit = [s for s in h2 if s not in cites and h2[s] != "À retenir"]
    if non_cit:
        out.append("sections jamais citées dans l'index (hors « À retenir ») : " + ", ".join(
            f"{s} {h2[s]}" for s in non_cit))
    # captures inutilisées
    utilisees = set()
    for c in chap:
        for b in c["blocs"]:
            if b["t"] == "fig":
                utilisees.add((c["dir"] / b["src"]).resolve())
    toutes = {p.resolve() for p in (MANUEL / "captures").glob("*/*.png")}
    inutiles = sorted(toutes - utilisees)
    if inutiles:
        out.append(f"{len(inutiles)} captures présentes sur disque mais jamais citées dans un chapitre : "
                   + ", ".join(f"{p.parent.name}/{p.name}" for p in inutiles))
    # index : numéros de page identiques ou non triés ne posent pas de problème ; entrées vides
    for kind, a, nums in d["entrees_index"]:
        if kind == "entree" and not nums:
            out.append(f'index.md : entrée « {a} » sans numéro')
    return out


if __name__ == "__main__":
    d = construire()
    pages_pdf = assembler(d, grain=False)
    pdf, res, nb = sauver_pdf(pages_pdf, "manuel-complet.pdf")
    print("pages prévues", len(pages_pdf), "pages du PDF", nb, pdf.stat().st_size // 1024, "Ko")
    print("débordements", res["deb"], "largeurs", res["larg"])
    print("ouvertures", d["pages_ouv"], "index", d["page_index"])
    for ligne in controle_textes(d):
        print("TEXTE", ligne)
    if "--apercu" in sys.argv:
        num = [int(x) for x in sys.argv[sys.argv.index("--apercu") + 1].split(",")]
        apercus(assembler(d, grain=True), num)
    if "--planche" in sys.argv or "--tout" in sys.argv:
        avec = assembler(d, grain=True)
        print("planche", planche(avec, d["chap"], len(avec)))
