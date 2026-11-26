"""Manuel d'utilisation complet de Kadenz : mise en page finale, version 2 (2026-11-26).

Entrées (lues, jamais modifiées) : docs/manuel/chapitres/01-installation.md à 15-depannage.md, index.md,
docs/manuel/captures/<chapitre>/*.png.
Design : la version 2 du 2026-11-25 (propositions-2026-11-25-manuel-v2/generer_manuel_v2.py), elle-même reprise
des couvertures du 2026-11-18 : couverture, sommaire, ouverture de chaque chapitre, pages courantes sans en-tête,
pied (titre du manuel, version, numéro de page), index, quatrième de couverture.

Méthode (aucune bibliothèque Python pour le document, Chrome headless seulement) :
  1. un convertisseur Markdown maison produit des blocs (titres, paragraphes, listes, tableaux, captures, encadrés) ;
  2. les captures sont recadrées (copies dans _captures/, par recadrer_captures.py sous Python 3.13 avec Pillow) ;
  3. Chrome pagine lui-même (@page nommées, flottants, orphelines et veuves, en-tête de tableau répété) ; les
     captures flottent à droite ou à gauche, le texte les contourne ;
  4. les numéros de page du sommaire et de l'index sont lus dans le PDF : chaque titre de section porte un lien
     invisible, dont la page est lue dans les annotations du PDF ; deuxième passe avec les vrais numéros, troisième
     passe finale sans les liens ;
  5. la planche de contrôle est faite de pages réelles du PDF, rastérisées par rasteriser_pdf.ps1 (WinRT).

Usage : py -3.14 -B genere_manuel_complet.py [--apercu 12,13,40] [--planche]
Sorties : manuel-complet.pdf, planche-manuel-complet.png (et, avec --apercu, des PNG dans _apercus/).
Les polices ABC Diatype sont lues dans crates/app/ui/polices/, jamais copiées.
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

# --- chapitres : blocs Markdown vers HTML, captures flottantes ---------------------------------------------
def rel(chemin):
    return os.path.relpath(chemin, ICI).replace("\\", "/")


RECADRAGES = {}

ANCRES = False                        # passes de repérage : chaque titre porte un lien invisible (voir pages_ancres)


def recadrer():
    """Prépare les copies recadrées des captures (Pillow, Python 3.13)."""
    global RECADRAGES
    fichier = ICI / "_captures" / "recadrages.json"
    if not fichier.exists():
        subprocess.run(["py", "-3.13", "-B", str(ICI / "recadrer_captures.py")], check=True)
    RECADRAGES = json.loads(fichier.read_text(encoding="utf-8"))


LIENS = {}           # identifiant de capture -> identifiant du dernier texte de sa section
SAUTS = set()          # captures dont le texte passe sur la page suivante : saut de page avant leur paragraphe


def figure_html(b, chap_dir, cote, large=None, fid=None):
    """Capture flottante ou en bande : cote d (à droite), g (à gauche) ou p (dans une paire)."""
    cle = f'{b["src"].split("/")[-2]}/{b["src"].split("/")[-1]}'
    info = RECADRAGES[cle]
    src = rel(ICI / "_captures" / cle)
    if large is None:
        large = 52
    style = "" if cote == "p" else f' style="width:{large}%"'
    return (f'<figure class="fl {cote}"{style}><div class="cadre-capture"><img src="{src}" width="{info["w"]}" '
            f'height="{info["h"]}" alt="{_html.escape(b["alt"])}"></div><figcaption>'
            + (f'<a class="anc" href="https://k.invalid/f/{fid}">{inline(b["alt"])}</a>' if fid else inline(b["alt"]))
            + '</figcaption></figure>')


def regrouper(blocs):
    """Les éléments de liste du parseur redeviennent des listes entières."""
    items, i = [], 0
    while i < len(blocs):
        b = blocs[i]
        if b["t"] == "li":
            elems = [b]
            while elems[-1]["k"] < elems[-1]["nk"] - 1:
                i += 1
                elems.append(blocs[i])
            items.append({"t": "list", "ord": b["ord"], "n": b["n"], "elems": elems})
        else:
            items.append(b)
        i += 1
    return items


def rendu_item(it, c, ancres, ancre=None):
    t = it["t"]

    def lien(x):
        """Texte d'un bloc, entouré d'un lien invisible de repérage (voir pages_ancres)."""
        h = inline(x)
        return f'<a class="anc" href="https://k.invalid/t/{ancre}">{h}</a>' if ancre else h

    if t == "h2":
        m = re.match(r"(\d+\.\d+)\s+(.*)", it["x"])
        num, titre = (m.group(1), m.group(2)) if m else ("", it["x"])
        if m:
            n = f'<a class="anc" href="https://k.invalid/s/{num}">{num}</a>' if ancres else num
            return f'<h2><span class="sn">{n}</span>{inline(titre)}</h2>'
        return f"<h2>{inline(titre)}</h2>"
    if t == "h3":
        return f'<h3>{inline(it["x"])}</h3>'
    if t == "p":
        return f'<p>{lien(it["x"])}</p>'
    if t == "list":
        lis = ""
        for e in it["elems"]:
            sub = "<ul>" + "".join(f"<li>{inline(s)}</li>" for s in e["sub"]) + "</ul>" if e["sub"] else ""
            lis += f'<li>{lien(e["x"]) if e is it["elems"][0] else inline(e["x"])}{sub}</li>'
        if it["ord"]:
            return f'<ol start="{it["n"]}">{lis}</ol>'
        return f"<ul>{lis}</ul>"
    if t == "note":
        lab = f'<b class="lab">{inline(it["label"])}</b>' if it["label"] else ""
        return (f'<div class="note">{lab}' + "".join(f"<p>{lien(p) if q == 0 else inline(p)}</p>"
                                                       for q, p in enumerate(it["paras"])) + "</div>")
    if t == "table":
        th = "".join(f"<th>{inline(x)}</th>" for x in it["head"])
        lignes = "".join("<tr>" + "".join(f"<td>{inline(x)}</td>" for x in row) + "</tr>" for row in it["rows"])
        return f"<table><thead><tr>{th}</tr></thead><tbody>{lignes}</tbody></table>"
    raise ValueError(t)


LARG_TEXTE = 793.7008 - 2 * 71.811          # largeur du texte : A4 moins deux marges de 19 mm
ECART = 20.8                                # 5,5 mm entre la capture et le texte
LARGEURS = (0.48, 0.52, 0.55, 0.60)         # largeurs possibles d'une capture flottante
HAUT_UNITE_MAX = 440                        # hauteur d'une unité (capture et texte qui l'entoure) à ne pas dépasser
REF = 0.52


def mesurer(htmls, largeur):
    """Hauteur de chaque morceau de HTML rendu à la largeur donnée (Chrome, polices chargées)."""
    divs = "".join(f'<div class="m" style="width:{largeur:.1f}px">{h}</div>' for h in htmls)
    html = (f'<!doctype html><html lang="fr"><head><meta charset="utf-8"><style>{feuille(False)}'
            f'.m{{display:flow-root}}</style></head><body><div class="mesure">{divs}</div>'
            '<script>Promise.all([document.fonts.load("400 13px Diatype"),document.fonts.load("700 13px Diatype"),'
            'document.fonts.load("400 13px \\"Diatype Mono\\"")]).then(()=>document.fonts.ready).then(()=>{'
            'const r=[...document.querySelectorAll(".m")].map(e=>e.getBoundingClientRect().height);'
            'const p=document.createElement("pre");p.id="res";p.textContent=JSON.stringify(r);document.body.appendChild(p)})'
            '</script></body></html>')
    tmp = ICI / "_mesure.html"
    tmp.write_text(html, encoding="utf-8")
    sortie = chrome("--window-size=1000,1000", "--dump-dom", tmp.as_uri()).stdout
    tmp.unlink()
    return json.loads(re.search(r'<pre id="res">(.*?)</pre>', sortie, re.S).group(1))


def haut_fig(info, f):
    return f * LARG_TEXTE * info["h"] / info["w"] + 26


def choisir_unite(info, hauteurs, dense):
    """Largeur de la capture et nombre de blocs de texte à lui associer : la hauteur du texte, rendu dans la colonne
    restante, doit approcher celle de la capture."""
    options = (0.55, 0.60) if dense else LARGEURS
    meilleur = None
    for f in options:
        col = LARG_TEXTE * (1 - f) - ECART
        for k in range(1, len(hauteurs) + 1):
            t = sum(hauteurs[:k]) * (LARG_TEXTE * (1 - REF) - ECART) / col
            hf = haut_fig(info, f)
            haut = max(t, hf)
            cout = abs(hf - t) + (0 if haut <= HAUT_UNITE_MAX else 400 + haut)
            if t < 0.6 * hf:
                cout += 60
            cle = (cout, k)
            if meilleur is None or cle < meilleur[0]:
                meilleur = (cle, f, k)
    return meilleur[1], meilleur[2]


def chapitre_html(c, ancres):
    """Corps d'un chapitre. Les captures flottent à droite ou à gauche et le texte les contourne (flottants natifs de
    Chrome), au niveau du paragraphe qui les précède. Deux captures séparées par peu de texte forment une paire côte à
    côte : sans cela les flottants s'accumulent plus vite que le texte ne les accompagne.
    Une capture que Chrome repousse après tout le texte de son chapitre (repérée par les liens invisibles, voir
    construire) reçoit un saut de page avant son paragraphe : elle reste ainsi avec le texte qui l'entoure."""
    items = regrouper(c["blocs"])
    n_it = len(items)
    n = c["num"]
    texte_ok = ("p", "list", "note")
    col_ref = LARG_TEXTE * (1 - REF) - ECART
    cand = [t for t in range(n_it) if items[t]["t"] in texte_ok]
    haut = dict(zip(cand, mesurer([typo(rendu_item(items[t], c, False)) for t in cand], col_ref) if cand else []))

    def cle(b):
        return f'{b["src"].split("/")[-2]}/{b["src"].split("/")[-1]}'

    def fin_section(a):
        """Dernier bloc de texte du chapitre : une capture qui passe après lui n'a plus de texte à ses côtés."""
        return cand[-1] if cand else None

    idx_fig = [t for t in range(n_it) if items[t]["t"] == "fig"]
    paires, libres, deja = {}, [], set()
    k = 0
    while k < len(idx_fig):
        a = idx_fig[k]
        b = idx_fig[k + 1] if k + 1 < len(idx_fig) else None
        paire = False
        if b is not None:
            entre = range(a + 1, b)
            types = [items[t]["t"] for t in entre]
            if all(x in texte_ok for x in types):     # ni titre ni tableau entre les deux
                hf = haut_fig(RECADRAGES[cle(items[a])], REF)
                paire = (not types) or sum(haut[t] for t in entre) < 0.8 * hf
        if paire:
            paires[a] = b
            k += 2
        else:
            libres.append(a)
            k += 1
    figs_paires = set(paires) | set(paires.values())
    # repères : identifiant de chaque groupe de captures et du dernier bloc de texte de sa section
    tcles = {}
    ident = {}
    for a in list(paires) + libres:
        ident[a] = f"{n}.{a}"
        e = fin_section(a)
        if e is not None:
            tcles[e] = f"{n}.{e}"
            if ancres:
                LIENS[f"{n}.{a}"] = f"{n}.{e}"

    saut_a = set()       # indices des éléments devant lesquels commence une page (avec leurs titres)

    def saut(a, cible=None):
        """Le saut de page précède les titres qui annoncent la capture, jamais ne les sépare du texte."""
        if ident[a] in SAUTS:
            t = a if cible is None else min(a, cible)
            while t > 0 and items[t - 1]["t"] in ("h2", "h3"):
                t -= 1
            saut_a.add(t)
        return ""

    avant, solos, cote = {}, {}, ["d"]
    for i in libres:
        prec = items[i - 1] if i > 0 else None
        suiv = items[i + 1] if i + 1 < n_it else None
        cible = None
        if prec is not None and prec["t"] == "p" and (i - 1) not in avant:
            cible = i - 1
        elif suiv is not None and suiv["t"] in texte_ok and (i + 1) not in avant:
            cible = i + 1
        elif prec is not None and prec["t"] in texte_ok and (i - 1) not in avant:
            cible = i - 1
        fid = ident[i] if ancres else None
        if cible is None:
            solos[i] = saut(i) + f'<div class="solo">{figure_html(items[i], c["dir"], "d", 52, fid)}</div>'
            continue
        if items[cible]["t"] == "p":
            c_ = cote[0]
            cote[0] = "g" if c_ == "d" else "d"
        else:
            c_ = "d"
        info = RECADRAGES[cle(items[i])]
        large = 60 if 0.17 < info["dense"] < 0.40 else 52
        avant.setdefault(cible, []).append(saut(i, cible) + figure_html(items[i], c["dir"], c_, large, fid))
    titre = inline(c["titre"])
    if ancres:
        titre = f'<a class="anc" href="https://k.invalid/c/{n}">{titre}</a>'
    long_ = len(c["titre"])
    fs = 38 if long_ <= 20 else (30 if long_ <= 36 else 24)
    sortie = (f'<div class="titre-ch"><h1 class="titre" style="font-size:{fs}px">{titre}</h1>'
              f'<span class="chiffre">{n}</span></div>')
    for idx in paires:
        saut(idx)
    for idx, it in enumerate(items):
        if idx in saut_a:
            sortie += '<div class="saut"></div>'
        if idx in paires:
            fid = ident[idx] if ancres else None
            sortie += (figure_html(items[idx], c["dir"], "g", 48, fid)
                       + figure_html(items[paires[idx]], c["dir"], "d", 48))
        if idx in figs_paires or idx in libres:
            sortie += solos.get(idx, "")
            continue
        sortie += "".join(avant.get(idx, []))
        sortie += rendu_item(it, c, ancres, tcles.get(idx) if ancres else None)
    return f'<div class="texte chap">{typo(sortie)}<div class="fin"></div></div>'


def index_html(d, ancres):
    entrees = d["entrees"]
    intro = "Les numéros renvoient aux pages du manuel."
    if ancres:
        intro = f'<a class="anc" href="https://k.invalid/i">{intro}</a>'
    corps = ""
    for kind, a, b in entrees:
        if kind == "lettre":
            corps += f'<div class="il">{a}</div>'
        else:
            pages = sorted({d["sections"].get(s, 0) for s in b})
            txt = ", ".join(str(p) if p else "000" for p in pages)
            corps += f'<div class="ie"><span class="t">{inline(a)}</span><span class="n">{txt}</span></div>'
    return f'<div class="texte index"><p class="ip">{intro}</p><div class="cols">{typo(corps)}</div></div>'


# --- document complet ---------------------------------------------------------------------------------------
def page_plein(interieur):
    return f'<section class="plein"><div class="v2">{typo(interieur)}</div></section>'


def document(d, ancres, grain):
    global GRAIN
    GRAIN = grain
    chap = d["chap"]
    pages_ouv = {c["num"]: d["ouv"].get(c["num"], "000") for c in chap}
    corps = [page_plein(couverture(chap)), page_plein(sommaire(chap, pages_ouv, d["page_index"]))]
    for c in chap:
        corps.append(page_plein(OUVERTURES[c["num"]](c)))
        corps.append(chapitre_html(c, ancres))
    corps.append(page_plein(o_index(d["lettres"])))
    corps.append(index_html(d, ancres))
    corps.append(page_plein(quatrieme()))
    return (f'<!doctype html>\n<html lang="fr"><head><meta charset="utf-8"><title>Manuel d\'utilisation | Kadenz</title>'
            f'<style>{feuille(grain)}</style></head><body>{"".join(corps)}</body></html>\n')


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
@page plein{margin:0}
@page texte{margin:19mm 19mm 28mm 19mm;
  @bottom-left{content:"Manuel d'utilisation";width:33.3%;vertical-align:bottom;padding-bottom:14mm;font:500 13px Diatype;letter-spacing:.07em;text-transform:uppercase;color:$NUIT;background:linear-gradient($NUIT,$NUIT) 0 4mm/100% 2px no-repeat}
  @bottom-center{content:"Kadenz $VERSION";width:33.4%;text-align:center;vertical-align:bottom;padding-bottom:14mm;font:500 13px Diatype;letter-spacing:.07em;text-transform:uppercase;color:$NUIT;background:linear-gradient($NUIT,$NUIT) 0 4mm/100% 2px no-repeat}
  @bottom-right{content:counter(page);width:33.3%;text-align:right;vertical-align:bottom;padding-bottom:14mm;font:500 13px Diatype;letter-spacing:.07em;text-transform:uppercase;color:$NUIT;background:linear-gradient($NUIT,$NUIT) 0 4mm/100% 2px no-repeat;font-variant-numeric:tabular-nums}}
*{box-sizing:border-box;margin:0;padding:0}
em,i,cite,address,dfn,var{font-style:normal}
i{display:block}
html,body{-webkit-print-color-adjust:exact;print-color-adjust:exact}
body{font-family:Diatype,sans-serif;color:$ENCRE;background:#fff}
.plein{page:plein;position:relative;width:210mm;height:297mm;overflow:hidden;break-before:page;break-after:page;background:#fff}
.plein:first-child{break-before:auto}
.plein:last-child{break-after:auto}
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

/* texte courant : dense, une colonne, captures flottantes */
.texte{page:texte;font-size:9.8pt;line-height:1.38;color:$ENCRE}
.texte p{margin:0 0 5px;orphans:2;widows:2}
.titre-ch{display:flex;justify-content:space-between;align-items:flex-start;gap:20px;margin-bottom:12px}
.titre-ch h1{color:$NUIT;max-width:560px}
.titre-ch .chiffre{font-size:38px;color:$NUIT}
.texte h2{font-weight:500;text-transform:uppercase;letter-spacing:.06em;font-size:11pt;line-height:1.25;color:$NUIT;
  margin:12px 0 4px;break-after:avoid}
.texte h2 .sn{display:inline-block;min-width:34px;font-variant-numeric:tabular-nums}
.texte h3{font-weight:700;font-size:10.4pt;line-height:1.25;color:$NUIT;margin:10px 0 2px;break-after:avoid}
ul,ol{padding-left:18px;margin:0 0 5px}
li{margin:0 0 2px;break-inside:avoid;orphans:2;widows:2}
li::marker{color:$NUIT;font-weight:700}
li>ul,li>ol{margin:2px 0 0}
b{font-weight:700}
code{font-family:"Diatype Mono",monospace;font-size:.9em;overflow-wrap:break-word}
a.anc{color:inherit;text-decoration:none}
.saut{break-before:page;height:0}
.fl{break-inside:avoid;margin-top:2px}
.fl.d{float:right;margin:3px 0 8px 5.5mm}
.fl.g{float:left;margin:3px 5.5mm 8px 0}
.cadre-capture{border:1px solid $TRAIT;background:#fff;border-radius:2px;overflow:hidden}
.cadre-capture img{display:block;width:100%;height:auto}
.fl figcaption{font-size:7.2pt;line-height:1.3;color:$ARDOISE;margin-top:3px}
.fl.g[style*="48%"]{margin-right:0}
.fl.d[style*="48%"]{margin-left:0}
.solo{clear:both;margin:6px 0 8px;overflow:hidden}
.solo .fl{width:52%;margin:0 auto;float:none}
.note{display:flow-root;padding:7px 12px 8px;background:$BRUME;border-radius:2px;margin:6px 0 7px;break-inside:avoid}
.note b.lab{display:block;font-size:10pt;color:$ENCRE;margin-bottom:2px}
.note p{margin:0}
.note p+p{margin-top:4px}
table{width:100%;border-collapse:collapse;font-size:8.6pt;line-height:1.3;margin:6px 0 8px;clear:both}
thead{display:table-header-group}
tr{break-inside:avoid}
th{background:$BRUME;color:$NUIT;font-weight:700;text-align:left;padding:4px 8px;vertical-align:top}
td{padding:3.5px 8px;border-bottom:1px solid $TRAIT;vertical-align:top}
.fin{clear:both;height:0}
/* index */
.texte.index .ip{font-size:8.6pt;color:$ARDOISE;margin:0 0 6px}
.texte.index .cols{columns:2;column-gap:9mm;column-fill:auto}
.il{font-weight:700;font-size:15pt;line-height:1;color:$NUIT;margin:12px 0 4px;break-after:avoid;break-inside:avoid}
.il:first-child{margin-top:0}
.ie{display:flex;gap:8px;justify-content:space-between;font-size:8.8pt;line-height:1.3;margin:0 0 2px;break-inside:avoid}
.ie .t{flex:1}
.ie .n{text-align:right;font-weight:500;color:$NUIT;font-variant-numeric:tabular-nums;max-width:45%}
/* planche */
body.planche{background:$BRUME;width:2640px;padding:80px}
.planche h1{font-size:56px;color:$NUIT;letter-spacing:-.01em;font-weight:700}
.planche .intro{font-size:24px;line-height:1.5;color:$ARDOISE;max-width:2000px;margin-top:16px}
.grille{display:grid;grid-template-columns:repeat(8,292px);gap:34px 20px;margin-top:50px}
.grille figure{margin:0}
.grille img{display:block;width:292px;height:413px;box-shadow:0 0 0 1px $TRAIT}
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
                          VERSION=VERSION)


def chrome(*args, timeout=1800):
    return subprocess.run([CHROME, "--headless=new", "--hide-scrollbars", "--allow-file-access-from-files",
                           "--virtual-time-budget=20000", *args],
                          check=True, capture_output=True, text=True, encoding="utf-8", timeout=timeout)


def imprimer(html, nom_pdf):
    tmp = ICI / "_doc.html"
    tmp.write_text(html, encoding="utf-8")
    pdf = ICI / nom_pdf
    chrome("--no-pdf-header-footer", f"--print-to-pdf={pdf}", tmp.as_uri())
    tmp.unlink()
    return pdf


# --- numéros de page réels : lecture du PDF ------------------------------------------------------------------
def pages_ancres(pdf):
    """Page de chaque lien invisible https://k.invalid/... : analyse des objets du PDF (sans bibliothèque)."""
    data = pdf.read_bytes()
    objs = {int(m.group(1)): m.group(2) for m in re.finditer(rb"(?<![0-9])(\d+) 0 obj(.*?)endobj", data, re.S)}
    racine = None
    for num, o in objs.items():
        if b"/Type /Catalog" in o or b"/Type/Catalog" in o:
            racine = int(re.search(rb"/Pages (\d+) 0 R", o).group(1))
    ordre = []

    def descendre(n):
        o = objs[n]
        if re.search(rb"/Type\s*/Pages", o):
            kids = re.search(rb"/Kids\s*\[(.*?)\]", o, re.S).group(1)
            for k in re.findall(rb"(\d+) 0 R", kids):
                descendre(int(k))
        else:
            ordre.append(n)

    descendre(racine)
    res = {}
    for idx, n in enumerate(ordre, 1):
        o = objs[n]
        m = re.search(rb"/Annots\s*(\[.*?\]|(\d+) 0 R)", o, re.S)
        if not m:
            continue
        if m.group(2):
            refs = re.findall(rb"(\d+) 0 R", objs[int(m.group(2))])
        else:
            refs = re.findall(rb"(\d+) 0 R", m.group(1))
        for r in refs:
            ob = objs.get(int(r), b"")
            u = re.search(rb"/URI\s*\(([^)]*)\)", ob)
            if u:
                cle = u.group(1).decode("latin-1").replace("https://k.invalid/", "")
                rect = [float(x) for x in re.search(rb"/Rect\s*\[([^\]]*)\]", ob).group(1).split()]
                haut = rect[3] - rect[1]
                # un lien coupé par un saut de page laisse parfois un fragment de hauteur nulle sur la page précédente
                if haut > 0.5:
                    res.setdefault(cle, []).append((idx, haut))
    # titres : page du fragment le plus haut ; paragraphes (t/) : dernière page où le texte apparaît
    sortie = {}
    for k, v in res.items():
        if k.startswith("t/"):
            sortie[k] = max(i for i, _ in v)
        else:
            sortie[k] = max(v, key=lambda x: (x[1], -x[0]))[0]
    return sortie, len(ordre)


def rasteriser(pdf, pages, dossier, echelle=1.5):
    dossier.mkdir(exist_ok=True)
    subprocess.run(["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", str(ICI / "rasteriser_pdf.ps1"),
                    "-Pdf", str(pdf), "-Out", str(dossier), "-Pages", ",".join(str(p) for p in pages),
                    "-Scale", str(echelle)], check=True, capture_output=True, text=True)

# --- assemblage en deux passes --------------------------------------------------------------------------------
def donnees():
    chap = charger_chapitres()
    _, entrees = charger_index()
    return {"chap": chap, "entrees": entrees, "lettres": [a for k, a, _ in entrees if k == "lettre"],
            "sections": {}, "ouv": {}, "page_index": "000"}


def passe(d, ancres, grain, nom):
    html = document(d, ancres, grain)
    pdf = imprimer(html, nom)
    return pdf


def mettre_a_jour(d, ancres_pdf):
    res, nb = ancres_pdf
    d["sections"] = {k[2:]: v for k, v in res.items() if k.startswith("s/")}
    d["ouv"] = {k[2:]: v - 1 for k, v in res.items() if k.startswith("c/")}
    d["page_index"] = res["i"] - 1
    return nb


def numeros_pages(d):
    return (dict(d["sections"]), dict(d["ouv"]), d["page_index"])


def captures_repoussees(res):
    """Captures dont la page suit celle du dernier texte de leur section."""
    mauvaises = set()
    for fid, tk in LIENS.items():
        pf, pt = res.get("f/" + fid), res.get("t/" + tk)
        if pf is not None and pt is not None and pf > pt:
            mauvaises.add(fid)
    return mauvaises


def construire(grain=False, nom="manuel-complet.pdf"):
    d = donnees()
    recadrer()
    SAUTS.clear()
    for essai in range(6):          # repérage des captures repoussées après leur texte, saut de page avant leur paragraphe
        LIENS.clear()
        pdf = passe(d, True, grain, "_passe1.pdf")
        res, nb1 = pages_ancres(pdf)
        mal = captures_repoussees(res) - SAUTS
        print("  repérage", essai + 1, ":", nb1, "pages,", len(mal), "capture(s) repoussée(s)")
        if not mal:
            break
        SAUTS.update(mal)
    nb1 = mettre_a_jour(d, (res, nb1))
    n1_ = numeros_pages(d)
    pdf = passe(d, True, grain, "_passe2.pdf")
    nb2 = mettre_a_jour(d, pages_ancres(pdf))
    stable = numeros_pages(d) == n1_
    if not stable:
        pdf = passe(d, True, grain, "_passe3.pdf")
        nb3 = mettre_a_jour(d, pages_ancres(pdf))
        stable = numeros_pages(d) == n1_
    (ICI / "_passe1.pdf").unlink(missing_ok=True)
    final = passe(d, False, grain, nom)
    nbf = pages_ancres(final)[1]
    for p in ("_passe2.pdf", "_passe3.pdf"):
        (ICI / p).unlink(missing_ok=True)
    d["stable"] = stable
    d["total"] = nbf
    d["manquantes"] = []
    for kind, a, nums in d["entrees"]:
        if kind == "entree":
            for s in nums:
                if s not in d["sections"]:
                    d["manquantes"].append((a, s))
    d["entrees_index"] = d["entrees"]
    d["sauts"] = len(SAUTS)
    return d, final


# --- planche et aperçus : pages du vrai PDF rastérisées --------------------------------------------------------
def choisir_pages(d):
    ouv = d["ouv"]
    nums = sorted(ouv)
    fins = {}
    for k, n in enumerate(nums):
        fins[n] = (ouv[nums[k + 1]] - 1) if k + 1 < len(nums) else d["page_index"] - 1
    choix = [(1, "Couverture"), (2, "Sommaire")]
    for n in nums:
        deb, fin = ouv[n], fins[n]
        titre = next(c["titre"] for c in d["chap"] if c["num"] == n)
        choix.append((deb, f"{n} {titre}, ouverture"))
        choix.append((deb + 1, f"{n} {titre}, première page"))
        if fin - deb >= 3:
            choix.append((deb + 1 + (fin - deb) // 2, f"{n} {titre}, page courante"))
    choix.append((d["page_index"], "Index, ouverture"))
    choix.append((d["page_index"] + 1, "Index, première page"))
    choix.append((d["total"], "Quatrième de couverture"))
    return choix


def planche(d, pdf, nom="planche-manuel-complet.png"):
    choix = choisir_pages(d)
    dossier = ICI / "_apercus"
    rasteriser(pdf, [n for n, _ in choix], dossier, 0.75)
    figs = ""
    for n, t in choix:
        figs += (f'<figure><img src="_apercus/p{n:03d}.png" alt=""><figcaption><b>{n}</b> {_html.escape(t)}</figcaption></figure>')
    corps = (f'<h1>Manuel d\'utilisation de Kadenz, version complète</h1>'
             f'<p class="intro">Mise en page finale des quinze chapitres, de l\'index et des pages de couverture : {d["total"]} pages A4 '
             f'au total, dont {len(choix)} montrées ici (couverture, sommaire, ouverture de chaque chapitre, première page de texte, '
             f'page courante, index, quatrième). Version {VERSION}.</p>'
             f'<div class="grille">{figs}</div>'
             f'<p class="pied-planche">Planche de contrôle : pages rastérisées à partir du PDF. Polices ABC Diatype lues dans le dépôt privé.</p>')
    html = (f'<!doctype html>\n<html lang="fr"><head><meta charset="utf-8"><title>Planche du manuel complet | Kadenz</title>'
            f'<style>{feuille(False)}</style></head><body class="planche">{corps}</body></html>\n')
    tmp = ICI / "_planche.html"
    tmp.write_text(html.replace("</body>", "<script>document.fonts.ready.then(()=>{const p=document.createElement('pre');"
                                "p.id='res';p.textContent=document.documentElement.scrollHeight;document.body.appendChild(p)})</script></body>"),
                   encoding="utf-8")
    h = int(re.search(r'<pre id="res">(\d+)</pre>', chrome("--window-size=2640,1000", "--dump-dom", tmp.as_uri()).stdout).group(1))
    tmp.write_text(html, encoding="utf-8")
    chrome("--force-device-scale-factor=1", f"--window-size=2640,{h}", f"--screenshot={ICI / nom}", tmp.as_uri())
    tmp.unlink()
    return len(choix), h


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
    t0 = __import__("time").time()
    d, pdf = construire()
    print("pages", d["total"], "poids", pdf.stat().st_size // 1024, "Ko", "numéros stables", d["stable"],
          "durée", int(__import__("time").time() - t0), "s")
    print("ouvertures", d["ouv"], "index", d["page_index"])
    for ligne in controle_textes(d):
        print("TEXTE", ligne)
    if "--apercu" in sys.argv:
        num = [int(x) for x in sys.argv[sys.argv.index("--apercu") + 1].split(",")]
        rasteriser(pdf, num, ICI / "_apercus", 1.6)
    if "--planche" in sys.argv or "--tout" in sys.argv:
        print("planche", planche(d, pdf))
