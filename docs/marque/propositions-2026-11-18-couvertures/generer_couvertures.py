"""Couvertures, affiches et gabarits Kadenz, propositions du 2026-11-18.

Inspirées des affiches fournies par l'utilisateur (grands titres serrés, listes en capitales,
dégradés grainés, repères de visée, chiffres de série), refaites avec la charte de Kadenz.
Aucune photo : le sujet de chaque pièce est la géométrie de la marque (les quatre pièces,
le disque Ambre, les motifs).

Trois familles :
- affiches 9:16 (1080 x 1920) : présentation du produit, réseaux, écrans ;
- diapositives 16:9 (1920 x 1080) : titre, intercalaire de section, fin ;
- manuels A4 avec 3 mm de fond perdu (216 x 303 mm) : système de couvertures, page de chapitre,
  quatrième de couverture.

Textes : tous tirés du dépôt (README, CHANGELOG 1.0.0, cahier des charges, charte, menus et
traductions de l'application). Les titres des manuels sont des propositions.

Usage : py -3.14 generer_couvertures.py (fontTools pour generer.py, Chrome)
Sorties : index.html, planche-couvertures.png, pieces/*.png et pieces/manuel-*.pdf
"""

import importlib.util
import math
import re
import subprocess
import sys
from pathlib import Path

ICI = Path(__file__).resolve().parent
MARQUE = ICI.parent
CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
LARGEUR_PLANCHE = 2640

if len(sys.argv) < 2:   # generer.py lit la racine du dépôt dans sys.argv[1]
    sys.argv.append(str(MARQUE.parent.parent))
spec = importlib.util.spec_from_file_location("generer_marque", MARQUE / "generer.py")
gm = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gm)

NUIT, INDIGO, AMBRE, BRUME = "#1A2259", "#2D3A8C", "#E3A23B", "#F4F5F9"
PIECES_MOTIF = "#232C6B"
VERSION, DATE_VERSION = "1.0.0", "2026-11-17"


# --- éléments graphiques -----------------------------------------------------------------
def logo(fichier, largeur, extra=""):
    return f'<img src="../{fichier}" style="width:{largeur}px;display:block;{extra}" alt="Kadenz">'


def visee(taille=120, rond=False, point=False, couleur="currentColor", e=2):
    """Repère de visée : une croix fine, avec un cercle ou un point au centre."""
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
    """Une pièce de la marque à grande échelle, remplie d'un dégradé grainé."""
    x1, y1, x2, y2 = angle
    return (f'<svg width="{taille}" height="{taille}" viewBox="0 0 100 100" aria-hidden="true">'
            f'<defs><linearGradient id="{cle}" x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}">'
            f'<stop offset="0" stop-color="{de}"/><stop offset="1" stop-color="{vers}"/></linearGradient>'
            f'<filter id="{cle}g" x="0" y="0" width="1" height="1"><feTurbulence type="fractalNoise" '
            f'baseFrequency="2.2" numOctaves="2" seed="4"/><feColorMatrix type="saturate" values="0"/>'
            f'<feComponentTransfer><feFuncA type="linear" slope=".5"/></feComponentTransfer>'
            f'<feComposite in2="SourceGraphic" operator="in"/></filter></defs>'
            f'<path d="{D_PIECES[sorte]}" fill="url(#{cle})"/>'
            f'<path d="{D_PIECES[sorte]}" fill="#000" filter="url(#{cle}g)" style="mix-blend-mode:overlay" opacity=".55"/>'
            f'</svg>')


def trame_disque(diametre, pas, couleur=NUIT, lumiere=(0.72, 0.30)):
    """Disque en trame de points : les points grossissent loin de la lumière (comme une photo tramée)."""
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


def motif(fonction, w, h, **k):
    s = fonction(NUIT, PIECES_MOTIF, w, h, **k)
    return s.replace("<svg ", f'<svg width="{w}" height="{h}" style="display:block" ', 1)


def liste(lignes, classe="liste"):
    return f'<div class="{classe}">' + "".join(f"<div>{l}</div>" for l in lignes) + "</div>"


def temps_vertical(d=34, g=16, clair="#FFFFFF"):
    return (f'<div style="display:flex;flex-direction:column;gap:{g}px">'
            f'<i class="rond" style="width:{d}px;height:{d}px;background:{clair}"></i>'
            f'<i class="rond" style="width:{d}px;height:{d}px;background:{clair}"></i>'
            f'<i class="rond" style="width:{d}px;height:{d}px;background:{AMBRE}"></i></div>')


DOCUMENTS = ["Devis", "Proforma", "Bons de commande", "Bordereaux de livraison", "Factures", "Règlements"]
NOUVEAUTES_1_0 = ["Serveur chez le client", "Postes reliés", "Travail hors connexion", "Accès à distance",
                  "Stock", "Export comptable SYSCOHADA"]
TON = ["Rigoureux", "Régulier", "Clair", "Sobre", "Fiable"]
LANGUES = [("fr", "FR", "Gestion commerciale"), ("en", "EN", "Sales management"), ("es", "ES", "Gestión comercial"),
           ("tr", "TR", "Ticari yönetim"), ("ar", "AR", "الإدارة التجارية"), ("ja", "JA", "販売管理"),
           ("zh", "ZH", "商务管理")]

PIECES = []   # (famille, nom, titre, html)


def piece(famille, nom, titre, html):
    PIECES.append((famille, nom, titre, html))


# --- affiches 9:16 -------------------------------------------------------------------------
piece("affiche", "A1-gestion-commerciale", "A1 Gestion commerciale", f"""
<div class="cadre affiche grain" style="background:linear-gradient(162deg,{NUIT} 0%,{NUIT} 34%,{INDIGO} 56%,{AMBRE} 78%,#F6E3C2 90%,{BRUME} 100%);color:#fff">
  <div class="titre" style="position:absolute;left:64px;top:84px;font-size:130px">Gestion<br>commerciale.</div>
  <div class="chiffre" style="position:absolute;right:64px;top:92px;font-size:64px">01</div>
  <div style="position:absolute;left:68px;top:500px">{liste(DOCUMENTS, "liste l")}</div>
  <div style="position:absolute;right:120px;top:1180px">{visee(130, point=True, couleur=NUIT)}</div>
  <div style="position:absolute;left:68px;bottom:72px;right:64px;color:{NUIT}" class="rang bas">
    <div class="pile" style="gap:26px">{logo("kadenz-mot.svg", 360)}<div class="liste s">Du devis au règlement</div></div>
    <div class="liste s droite">Version {VERSION}<br>{DATE_VERSION}</div>
    {logo("kadenz-symbole.svg", 130)}
  </div>
</div>""")

piece("affiche", "A2-du-devis-au-reglement", "A2 Du devis au règlement", f"""
<div class="cadre affiche grain papier" style="color:{NUIT}">
  <div class="liste m" style="position:absolute;left:64px;top:84px">Windows 10 | 11<br>64 bits</div>
  <div style="position:absolute;left:470px;top:150px;width:600px;height:600px;border-radius:50%;
    background:radial-gradient(circle at 40% 40%,#EDB65A 0%,{AMBRE} 55%,#D58F27 100%)"></div>
  <div style="position:absolute;left:0;top:330px;width:620px;height:1230px;overflow:hidden">{motif(gm.motif_tournant, 620, 1230, s=88, graine=7)}
    <div style="position:absolute;left:470px;top:-180px;width:600px;height:600px;border-radius:50%;background:{AMBRE};opacity:.5;mix-blend-mode:screen"></div></div>
  <div class="prefixes" style="position:absolute;left:830px;top:790px">D<br>P<br>C<br>L<br>F<br>A</div>
  <div style="position:absolute;left:676px;top:1060px">{visee(110, rond=True)}</div>
  <div class="titre" style="position:absolute;left:676px;top:1440px;font-size:60px;line-height:.95">Du devis<br>au<br>règlement</div>
  <div style="position:absolute;left:64px;bottom:170px;width:560px">
    <div class="regle"><span>F</span><span>2026</span><span>11</span><span>0001</span></div>
    <div class="regle-trait"><i></i><i></i><i></i><i></i></div>
    <div class="regle bas-l"><span>Type</span><span>Année</span><span>Mois</span><span>Rang</span></div>
  </div>
  <div class="liste s" style="position:absolute;left:64px;bottom:72px">Numérotation / F2026-11-0001</div>
  <div class="rang" style="position:absolute;right:64px;bottom:64px;gap:28px">{logo("kadenz-symbole.svg", 120)}<div class="chiffre" style="font-size:110px">02</div></div>
</div>""")

piece("affiche", "A3-version-1-0", "A3 Version 1.0", f"""
<div class="cadre affiche grain" style="background:linear-gradient(90deg,{BRUME} 0%,{BRUME} 44%,#B9C0E4 56%,{INDIGO} 74%,{NUIT} 100%);color:{NUIT}">
  <div class="titre" style="position:absolute;left:30px;top:-10px;font-size:600px;letter-spacing:-.07em;line-height:.9">1.0</div>
  <div style="position:absolute;right:80px;top:200px;width:200px;height:3px;background:#fff"></div>
  <div style="position:absolute;left:70px;top:600px">{liste(NOUVEAUTES_1_0, "liste m")}</div>
  <div style="position:absolute;left:56px;top:960px">{visee(130, rond=True)}</div>
  <div class="titre" style="position:absolute;left:70px;top:1300px;font-size:84px;line-height:.95">Première<br>version de<br>la phase 2</div>
  <div style="position:absolute;right:130px;top:1440px;color:#fff">{visee(150, rond=True, couleur="#fff")}</div>
  <div class="liste m" style="position:absolute;left:70px;bottom:72px">Serveur<br>Postes<br>Relais</div>
  <div class="liste m" style="position:absolute;right:64px;bottom:72px;color:#fff">Kadenz<br>Version {VERSION}<br>{DATE_VERSION}</div>
</div>""")

langues_html = "".join(
    f'<div class="ligne-langue"><span class="code">{c}</span><span lang="{l}" class="{"rtl" if l == "ar" else ""}"'
    f'{" dir=rtl" if l == "ar" else ""}>{t}</span></div>' for l, c, t in LANGUES)
piece("affiche", "A4-sept-langues", "A4 Sept langues", f"""
<div class="cadre affiche grain papier" style="color:{NUIT}">
  <div class="titre" style="position:absolute;left:52px;top:70px;font-size:196px">Sept<br>langues</div>
  <div class="liste m" style="position:absolute;right:64px;top:96px">Interface<br>Langue choisie<br>par compte</div>
  <div class="chiffre" style="position:absolute;left:760px;top:1530px;font-size:110px">07</div>
  <div style="position:absolute;right:64px;top:460px;width:250px;height:3px;background:{NUIT}"></div>
  <div style="position:absolute;right:186px;top:400px;width:3px;height:130px;background:{NUIT}"></div>
  <div style="position:absolute;left:380px;top:540px">{trame_disque(900, 22)}</div>
  <div class="cjk vertical" lang="ja" style="position:absolute;left:58px;top:560px;font-size:150px">販売管理</div>
  <div style="position:absolute;left:300px;top:1300px">{visee(110, rond=True)}</div>
  <div class="grille-bas" style="position:absolute;left:0;right:0;top:1480px;height:440px"></div>
  <div style="position:absolute;left:64px;top:1520px">{langues_html}</div>
  <div style="position:absolute;right:80px;top:1524px">{temps_vertical(36, 18, NUIT)}</div>
  <div class="liste s droite" style="position:absolute;right:64px;bottom:72px">Documents clients<br>en français</div>
</div>""")

piece("affiche", "A5-un-outil-de-travail-sur", "A5 Un outil de travail sûr", f"""
<div class="cadre affiche grain papier" style="color:{NUIT}">
  <div style="position:absolute;left:0;top:0;right:0;height:560px;background:{NUIT}"></div>
  <div class="titre" style="position:absolute;left:58px;top:70px;font-size:112px;color:#fff">Un outil<br>de travail<br>sûr</div>
  <div style="position:absolute;left:700px;top:70px;color:#fff">{visee(130)}</div>
  <div class="liste m" style="position:absolute;left:856px;top:100px;color:#fff">Régulier<br>Sans<br>fioriture</div>
  <div class="blocs" style="position:absolute;left:-60px;top:560px;width:1060px;height:1060px">
    <div style="left:0;top:0;width:500px;height:500px;background:#C9CEDB"></div>
    <div style="left:560px;top:0">{piece_degrade("quart", 500, "a5q", "#8F98B8", "#3A4472", (0, 0, 1, 1))}</div>
    <div style="left:0;top:560px">{piece_degrade("triangle", 500, "a5t", "#DDE1EA", "#9AA2BC", (0, 0, 1, 1))}</div>
    <div style="left:560px;top:560px">{piece_degrade("disque", 500, "a5d", "#5A648E", NUIT, (0, 0, 1, 1))}</div>
  </div>
  <div class="papier" style="position:absolute;left:0;top:1090px;width:520px;height:620px;padding:64px">
    <div class="titre" style="font-size:52px;line-height:1;margin-bottom:28px">Cinq mots</div>
    {liste(TON, "liste m")}
    <div style="margin-top:46px">{visee(90)}</div>
  </div>
  <div class="liste m" style="position:absolute;right:64px;bottom:72px">Kadenz<br>Gestion commerciale<br>Version {VERSION}</div>
</div>""")


# --- diapositives 16:9 ---------------------------------------------------------------------
piece("diapo", "D1-titre", "D1 Diapositive de titre", f"""
<div class="cadre diapo grain" style="background:linear-gradient(118deg,{NUIT} 0%,{NUIT} 42%,{INDIGO} 62%,{AMBRE} 82%,#F6E3C2 92%,{BRUME} 100%);color:#fff">
  <div style="position:absolute;left:84px;top:84px">{logo("kadenz-logo-horizontal-negatif.svg", 300)}</div>
  <div class="chiffre" style="position:absolute;left:880px;top:92px;font-size:56px">01</div>
  <div class="titre" style="position:absolute;left:78px;top:300px;font-size:190px">Gestion<br>commerciale.</div>
  <div style="position:absolute;left:84px;bottom:84px">{liste(DOCUMENTS[:3], "liste m")}</div>
  <div style="position:absolute;left:560px;bottom:84px">{liste(DOCUMENTS[3:], "liste m")}</div>
  <div style="position:absolute;left:1330px;top:330px">{visee(130, point=True, couleur=NUIT)}</div>
  <div class="liste m droite" style="position:absolute;right:84px;bottom:84px;color:{NUIT}">Version {VERSION}<br>{DATE_VERSION}<br>Mcarre</div>
</div>""")

piece("diapo", "D2-intercalaire", "D2 Intercalaire de section", f"""
<div class="cadre diapo grain" style="background:linear-gradient(90deg,{BRUME} 0%,{BRUME} 52%,#B9C0E4 62%,{INDIGO} 80%,{NUIT} 100%);color:{NUIT}">
  <div class="titre" style="position:absolute;left:50px;top:-40px;font-size:640px;letter-spacing:-.07em;line-height:.9">03</div>
  <div class="titre" style="position:absolute;left:84px;top:640px;font-size:104px">Sécurité,<br>fiabilité,<br>technique</div>
  <div style="position:absolute;left:830px;top:690px">{visee(110, rond=True)}</div>
  <div style="position:absolute;right:120px;top:120px;width:220px;height:3px;background:#fff"></div>
  <div style="position:absolute;right:150px;top:470px;color:#fff">{visee(150, rond=True, couleur="#fff")}</div>
  <div class="liste m droite" style="position:absolute;right:84px;bottom:84px;color:#fff">Présentation<br>Kadenz {VERSION}</div>
</div>""")

piece("diapo", "D3-fin", "D3 Diapositive de fin", f"""
<div class="cadre diapo" style="background:{NUIT};color:#fff">
  <div style="position:absolute;left:0;top:0">{motif(gm.motif_fondu, 1100, 1080, s=60, portee=0.95)}</div>
  <div style="position:absolute;left:1080px;top:380px">{logo("kadenz-logo-horizontal-negatif.svg", 640)}</div>
  <div class="rang" style="position:absolute;left:1090px;top:610px;gap:14px">
    <i style="width:60px;height:8px;background:#fff"></i><i style="width:60px;height:8px;background:#fff"></i><i style="width:60px;height:8px;background:{AMBRE}"></i></div>
  <div class="liste m" style="position:absolute;left:1090px;top:680px;color:#C5CAE0">Gestion commerciale</div>
  <div class="liste s" style="position:absolute;left:1090px;bottom:84px;color:#C5CAE0">Logiciel propriétaire, tous droits réservés.<br>Mcarre / version {VERSION}</div>
</div>""")


# --- manuels A4 (216 x 303 mm, fond perdu de 3 mm compris) ---------------------------------
MANUELS = [
    ("01", "Installation", "carre", ["Poste : KadenzSetup", "Serveur : paquet MSI", "Premier lancement",
                                      "Inscription d'un poste", "Mises à jour"]),
    ("02", "Manuel<br>d'utilisation", "quart", ["Tableau de bord", "Documents", "Paiements", "Référentiels",
                                                 "Stock", "Comptabilité", "Paramètres"]),
    ("03", "Administration<br>du serveur", "triangle", ["Fenêtre KadenzAdmin", "Postes et codes d'inscription",
                                                         "Conflits et opérations en attente", "Sauvegardes",
                                                         "Accès à distance", "Mise à jour du serveur"]),
]
POSITION_PIECE = {"carre": (300, 360), "quart": (250, 330), "triangle": (250, 320)}

for num, titre, sorte, contenu in MANUELS:
    x, y = POSITION_PIECE[sorte]
    nom = "manuel-" + num + "-" + re.sub(r"[^a-z]+", "-", titre.replace("<br>", " ").lower()
                                          .replace("é", "e").replace("'", " ")).strip("-")
    piece("manuel", nom, f"Manuel {num} {titre.replace('<br>', ' ')}", f"""
<div class="cadre a4 grain" style="background:linear-gradient(170deg,{NUIT} 0%,{NUIT} 70%,#222B66 100%);color:#fff">
  <div style="position:absolute;left:{x}px;top:{y}px">{piece_degrade(sorte, 640, "m" + num, INDIGO, AMBRE, (0, 0, 1, 1))}</div>
  <div class="titre" style="position:absolute;left:56px;top:62px;font-size:70px">{titre}</div>
  <div class="chiffre" style="position:absolute;right:56px;top:58px;font-size:96px">{num}</div>
  <div style="position:absolute;left:58px;top:{300 if '<br>' in titre else 230}px">{liste(contenu, "liste a4l")}</div>
  <div style="position:absolute;left:0;right:0;bottom:0;height:200px;background:{NUIT}"></div>
  <div style="position:absolute;left:56px;bottom:66px">{logo("kadenz-logo-horizontal-negatif.svg", 220)}</div>
  <div class="liste a4s droite" style="position:absolute;right:56px;bottom:62px;color:#C5CAE0">Version {VERSION}<br>Manuel {num} / 03</div>
</div>""")

piece("manuel", "manuel-02-chapitre-stock", "Page de chapitre (manuel 02)", f"""
<div class="cadre a4 papier grain" style="color:{NUIT}">
  <div class="liste a4s" style="position:absolute;left:56px;top:62px">Chapitre</div>
  <div style="position:absolute;right:56px;top:56px">{piece_degrade("quart", 90, "chq", INDIGO, NUIT)}</div>
  <div class="titre" style="position:absolute;left:40px;top:110px;font-size:380px;letter-spacing:-.07em;line-height:.9">05</div>
  <div class="titre" style="position:absolute;left:56px;top:500px;font-size:110px">Stock</div>
  <div style="position:absolute;left:58px;top:660px">{liste(["État à une date", "Mouvements", "Inventaire en deux temps", "Ajustement"], "liste a4l")}</div>
  <div style="position:absolute;left:600px;top:640px">{visee(80, rond=True)}</div>
  <div style="position:absolute;left:56px;right:56px;bottom:100px;height:2px;background:{NUIT}"></div>
  <div class="rang entre liste a4s" style="position:absolute;left:56px;right:56px;bottom:60px"><span>Manuel d'utilisation</span><span>Kadenz {VERSION}</span></div>
</div>""")

piece("manuel", "manuel-quatrieme", "Quatrième de couverture", f"""
<div class="cadre a4" style="background:{NUIT};color:#fff">
  <div style="position:absolute;left:0;top:0">{motif(gm.motif_tournant, 816, 680, s=68, graine=7)}</div>
  <div style="position:absolute;left:56px;top:760px">{logo("kadenz-logo-empile-negatif.svg", 190)}</div>
  <div class="rang" style="position:absolute;left:64px;top:1000px;gap:8px">
    <i style="width:30px;height:4px;background:#fff"></i><i style="width:30px;height:4px;background:#fff"></i><i style="width:30px;height:4px;background:{AMBRE}"></i></div>
  <div class="liste a4s" style="position:absolute;left:64px;bottom:62px;color:#C5CAE0">Logiciel propriétaire, tous droits réservés.<br>Mcarre</div>
  <div class="liste a4s droite" style="position:absolute;right:56px;bottom:62px;color:#C5CAE0">Gestion commerciale<br>Version {VERSION}</div>
</div>""")


# --- page et exports -----------------------------------------------------------------------
GRAIN = ("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='300' height='300'%3E%3Cfilter id='n'%3E"
         "%3CfeTurbulence type='fractalNoise' baseFrequency='.9' numOctaves='3' stitchTiles='stitch'/%3E"
         "%3CfeColorMatrix type='saturate' values='0'/%3E%3C/filter%3E%3Crect width='300' height='300' filter='url(%23n)'/%3E%3C/svg%3E")

CSS = f"""
@font-face{{font-family:Inter;src:url(../planches/polices/Inter.woff2) format("woff2");font-weight:100 900}}
@font-face{{font-family:"Space Grotesk";src:url(../planches/polices/SpaceGrotesk.ttf) format("truetype");font-weight:300 700}}
*{{box-sizing:border-box;margin:0;padding:0}}
em,i{{font-style:normal}}
i{{display:block}}
body{{font-family:Inter,sans-serif;color:#1F2933}}
body.seule{{background:transparent}}
.cadre{{position:relative;overflow:hidden}}
.affiche{{width:1080px;height:1920px}}
.diapo{{width:1920px;height:1080px}}
.a4{{width:816px;height:1145px}}
.papier{{background:{BRUME}}}
.grain::after{{content:"";position:absolute;inset:0;background:url("{GRAIN}");opacity:.16;mix-blend-mode:overlay;pointer-events:none}}
.papier.grain::after{{opacity:.10;mix-blend-mode:multiply}}
.titre{{font-weight:700;text-transform:uppercase;letter-spacing:-.035em;line-height:.88}}
.chiffre{{font-weight:600;font-variant-numeric:tabular-nums;letter-spacing:-.02em;line-height:1}}
.liste{{text-transform:uppercase;font-weight:500;letter-spacing:.07em;font-variant-numeric:tabular-nums;line-height:1.22}}
.liste.s{{font-size:26px}}.liste.m{{font-size:34px}}.liste.l{{font-size:44px}}
.liste.a4l{{font-size:21px;line-height:1.35}}.liste.a4s{{font-size:13px;line-height:1.35}}
.droite{{text-align:right}}
.rang{{display:flex;align-items:center}}
.rang.bas{{align-items:flex-end;justify-content:space-between}}
.entre{{justify-content:space-between}}
.pile{{display:flex;flex-direction:column}}
.rond{{border-radius:50%}}
.prefixes{{font-family:"Space Grotesk";font-weight:700;font-size:96px;line-height:.98;text-align:center}}
.regle{{display:grid;grid-template-columns:1fr 1fr 1fr 1fr;font-weight:600;font-size:30px;letter-spacing:.04em;font-variant-numeric:tabular-nums}}
.regle span:nth-child(2),.regle span:nth-child(3){{text-align:center}}.regle span:last-child{{text-align:right}}
.regle.bas-l{{font-size:20px;text-transform:uppercase;letter-spacing:.1em;font-weight:500}}
.regle-trait{{position:relative;height:30px;margin:10px 0;border-bottom:3px solid currentColor;transform:translateY(-14px);
  display:flex;justify-content:space-between}}
.regle-trait i{{width:3px;height:30px;background:currentColor;transform:translateY(15px)}}
.cjk{{font-family:"Yu Gothic UI","Yu Gothic","Microsoft YaHei",sans-serif;font-weight:700}}
.vertical{{writing-mode:vertical-rl;letter-spacing:.06em;line-height:1}}
.ligne-langue{{display:flex;gap:26px;font-size:32px;line-height:1.36;font-weight:500;text-transform:uppercase;letter-spacing:.06em}}
.ligne-langue .code{{width:52px;font-weight:700}}
.ligne-langue [lang=ja],.ligne-langue [lang=zh]{{font-family:"Yu Gothic UI","Microsoft YaHei",sans-serif;letter-spacing:.1em}}
.ligne-langue [lang=ar]{{font-family:"Segoe UI",sans-serif;letter-spacing:0}}
.grille-bas{{border-top:3px solid {NUIT};background:
  linear-gradient({NUIT},{NUIT}) 700px 0/3px 100% no-repeat,
  linear-gradient({NUIT},{NUIT}) 700px 220px/380px 3px no-repeat}}
.blocs>div{{position:absolute}}
.blocs svg{{display:block}}

/* planche */
body.planche{{background:{BRUME};width:{LARGEUR_PLANCHE}px;padding:80px}}
.planche h1{{font-family:"Space Grotesk";font-size:56px;color:{NUIT};letter-spacing:-.01em}}
.planche h2{{font-family:"Space Grotesk";font-size:36px;color:{NUIT};margin:70px 0 28px}}
.planche .intro{{font-size:24px;line-height:1.5;color:#52606D;max-width:1800px;margin-top:16px}}
.rangee{{display:flex;justify-content:space-between;align-items:flex-start;gap:20px}}
.rangee figure{{margin:0}}
.vignette{{box-shadow:0 0 0 1px #D9DCE3}}
.legende{{font-size:20px;margin-top:12px;color:#52606D}}
.legende b{{color:{NUIT}}}
.notes{{margin-top:70px;background:#fff;border:1px solid #D9DCE3;padding:46px 56px;font-size:22px;line-height:1.55;
  display:grid;grid-template-columns:1fr 1fr;gap:10px 70px}}
.notes h3{{font-family:"Space Grotesk";font-size:28px;color:{NUIT};margin:16px 0 10px}}
.notes ul{{padding-left:26px}}
"""

ECHELLE = {"affiche": 0.43, "diapo": 0.4, "manuel": 0.575}


def document(corps, classe, titre, page_css=""):
    return (f'<!doctype html>\n<html lang="fr"><head><meta charset="utf-8"><title>{titre}</title>'
            f'<style>{CSS}{page_css}</style></head><body class="{classe}">{corps}</body></html>\n')


def planche():
    def rangee(famille):
        z = ECHELLE[famille]
        return '<div class="rangee">' + "".join(
            f'<figure><div class="vignette" style="zoom:{z}">{html}</div>'
            f'<figcaption class="legende"><b>{titre}</b></figcaption></figure>'
            for f, nom, titre, html in PIECES if f == famille) + "</div>"
    notes = """
<div class="notes">
  <div>
    <h3>Système</h3>
    <ul>
      <li>Grand titre en capitales serrées (Inter 700), listes en capitales espacées, chiffre de série en haut à droite.</li>
      <li>Le sujet est la géométrie de la marque : les quatre pièces, le disque Ambre, les motifs Tournant et Fondu.</li>
      <li>Manuels : une même grille, un numéro, une pièce par manuel (01 carré, 02 quart de disque, 03 triangle ;
          le disque reste libre pour un quatrième). Le logo se pose toujours sur une zone calme.</li>
    </ul>
    <h3>À valider : écarts à la charte</h3>
    <ul>
      <li>Dégradés grainés Nuit, Indigo, Ambre en fond : la charte n'en prévoit pas.</li>
      <li>Pièces remplies d'un dégradé et disque en trame de points (A4) : nouveaux traitements des pièces.</li>
    </ul>
  </div>
  <div>
    <h3>Sources des textes</h3>
    <ul>
      <li>README et CHANGELOG 1.0.0 : documents traités, nouveautés de la version, « première version de la phase 2 ».</li>
      <li>Code et cahier : numéro F2026-11-0001 et sa forme (type, année, mois, rang), préfixes D P C L F A, langues.</li>
      <li>Menus et rubriques de l'application : contenu des manuels, chapitre Stock.</li>
      <li>Charte : les cinq mots, « un outil de travail sûr, régulier, sans fioriture ». « Du devis au règlement » est une
          formule proposée lors des micrographies.</li>
      <li>Traductions de l'interface (A4) : celles de l'application, jamais relues par un locuteur natif.</li>
      <li>Titres des trois manuels et plan du manuel d'utilisation : propositions.</li>
    </ul>
  </div>
</div>"""
    corps = f"""
<h1>Couvertures et gabarits Kadenz, propositions du 2026-11-18</h1>
<p class="intro">Inspirés des affiches fournies : affiches 9:16 pour présenter le produit, gabarits de diapositives 16:9,
système de couvertures pour les manuels (A4, fond perdu de 3 mm compris). Aucune photo : la géométrie de la marque
tient le rôle de l'image.</p>
<h2>Affiches 9:16</h2>{rangee("affiche")}
<h2>Diapositives 16:9</h2>{rangee("diapo")}
<h2>Manuels A4</h2>{rangee("manuel")}
{notes}"""
    return document(corps, "planche", "Couvertures Kadenz")


def chrome(*args):
    return subprocess.run([CHROME, "--headless=new", "--hide-scrollbars", "--virtual-time-budget=5000", *args],
                          check=True, capture_output=True, text=True, encoding="utf-8")


def hauteur_planche(html):
    out = chrome("--window-size=2640,1000", "--dump-dom", html.as_uri()).stdout
    return out  # la hauteur est lue plus bas par un script injecté


TAILLES = {"affiche": (1080, 1920), "diapo": (1920, 1080), "manuel": (816, 1145)}

if __name__ == "__main__":
    html = ICI / "index.html"
    contenu = planche()
    html.write_text(contenu, encoding="utf-8")
    # hauteur de la planche : mesurée par Chrome
    mesure = ICI / "_mesure.html"
    mesure.write_text(contenu.replace("</body>", "<script>document.fonts.ready.then(()=>document.body.setAttribute("
                                      "'data-h',document.documentElement.scrollHeight))</script></body>"), encoding="utf-8")
    h = int(re.search(r'data-h="(\d+)"', chrome("--window-size=2640,1000", "--dump-dom", mesure.as_uri()).stdout).group(1))
    mesure.unlink()
    png = ICI / "planche-couvertures.png"
    chrome("--force-device-scale-factor=1", f"--window-size={LARGEUR_PLANCHE},{h}", f"--screenshot={png}", html.as_uri())
    print(png, h)

    dossier = ICI / "pieces"
    dossier.mkdir(exist_ok=True)
    tmp = ICI / "_seule.html"
    for famille, nom, titre, corps in PIECES:
        w, hh = TAILLES[famille]
        page = "@page{size:216mm 303mm;margin:0}" if famille == "manuel" else ""
        tmp.write_text(document(corps, "seule", titre, page), encoding="utf-8")
        echelle = 2 if famille == "manuel" else 1
        sortie = dossier / f"{nom}.png"
        chrome(f"--force-device-scale-factor={echelle}", f"--window-size={w},{hh}", f"--screenshot={sortie}", tmp.as_uri())
        print(sortie)
        if famille == "manuel":
            pdf = dossier / f"{nom}.pdf"
            chrome("--no-pdf-header-footer", f"--print-to-pdf={pdf}", tmp.as_uri())
            print(pdf)
    tmp.unlink()
