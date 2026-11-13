"""Propositions de signalétique pour l'identité « Quatre pièces » (2026-11-13).

Proposition seulement : rien n'est intégré à la planche 07 tant que l'utilisateur n'a pas choisi.
Les scènes réutilisent les fichiers de la marque (logos, icône, marques de modules) et les motifs
de generer.py (Quatre pièces, Tournant, Fondu). Dimensions données à titre indicatif.

Usage : python generer_signaletique.py <racine du dépôt>
Sorties, à côté de ce fichier : index.html (planche) et propositions-signaletique.png (rendu Chrome).
"""

import importlib.util
import subprocess
import sys
from pathlib import Path

ICI = Path(__file__).resolve().parent
MARQUE = ICI.parent
CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"

spec = importlib.util.spec_from_file_location("generer_marque", MARQUE / "generer.py")
gm = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gm)

NUIT, PIECES, INDIGO, AMBRE = "#1A2259", "#232C6B", "#2D3A8C", "#E3A23B"
BRUME, BLANC, TRAIT, ARDOISE = "#F4F5F9", "#FFFFFF", "#D9DCE3", "#52606D"
MUR, SOL, GRIS = "#E6E9EF", "#CBD0DA", "#A3AABA"
W, H = 400, 500


def img(nom, x, y, w, h):
    return f'<image href="../{nom}" x="{x}" y="{y}" width="{w}" height="{h}"/>'


def imbrique(s, x, y, w, h):
    """Un SVG de generer.py posé dans la scène, à la taille (w, h) pour laquelle il a été calculé."""
    s = s.replace(' xmlns="http://www.w3.org/2000/svg"', "")
    return s.replace("<svg ", f'<svg x="{x}" y="{y}" width="{w}" height="{h}" ', 1).strip()


def rect(x, y, w, h, fill, rx=0, extra=""):
    return f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" fill="{fill}" {extra}/>'


def texte(x, y, t, taille, couleur, graisse=500, ancre="middle", police="Inter"):
    return (f'<text x="{x}" y="{y}" font-family="{police}" font-size="{taille}" font-weight="{graisse}" '
            f'fill="{couleur}" text-anchor="{ancre}">{t}</text>')


def personne(x, sol, h=150):
    """Silhouette d'échelle : une personne de 1,75 m (h px)."""
    r = h * 0.075
    return (f'<circle cx="{x}" cy="{sol - h + r}" r="{r}" fill="{GRIS}"/>'
            f'<rect x="{x - h * 0.12}" y="{sol - h + 2.25 * r}" width="{h * 0.24}" height="{h - 2.25 * r}" rx="{h * 0.05}" fill="{GRIS}"/>')


def decor(sol=420, mur=MUR):
    return rect(0, 0, W, sol, mur) + rect(0, sol, W, H - sol, SOL)


# --- les scènes ------------------------------------------------------------------------
def facade():
    s, out = 420, []
    out.append(decor(s))
    out.append(rect(40, 60, 320, 96, NUIT, 2))
    out.append(imbrique(gm.motif_fondu(NUIT, PIECES, 320, 96, 24, 0.5), 40, 60, 320, 96))
    out.append(img("kadenz-logo-horizontal-negatif.svg", 190, 86, 150, 44))
    out.append(rect(40, 156, 320, 4, "#11173D"))
    out.append(rect(124, 222, 152, 198, "#8E96A8"))
    out.append(rect(132, 230, 66, 190, "#EEF1F6") + rect(202, 230, 66, 190, "#EEF1F6"))
    out.append(img("kadenz-symbole.svg", 186, 300, 28, 28))
    out.append(personne(330, s))
    return out


def drapeau():
    s, out = 420, []
    out.append(decor(s))
    out.append(rect(0, 0, 70, s, "#D5D9E1"))
    out.append(rect(70, 116, 200, 6, "#7C8496") + rect(70, 106, 10, 26, "#7C8496"))
    out.append(rect(110, 122, 3, 14, "#7C8496") + rect(227, 122, 3, 14, "#7C8496"))
    out.append(rect(90, 136, 160, 160, INDIGO, 2))
    out.append(img("kadenz-symbole-blanc.svg", 120, 160, 100, 100))
    out.append(rect(282, 136, 14, 160, "#20296A", 1))
    out.append(texte(289, 316, "chant", 11, ARDOISE, 400))
    return out


def lettres():
    s, out = 400, []
    out.append(decor(s, "#EEF0F5"))
    out.append(img("kadenz-logo-horizontal.svg", 50, 110, 300, 84))
    out.append(rect(90, 290, 220, 110, BLANC, 2, f'stroke="{TRAIT}"'))
    out.append(rect(90, 290, 220, 14, NUIT, 2))
    out.append(personne(350, s, 175))
    return out


def plaque():
    out = [rect(0, 0, W, H, "#DADDE4")]
    for y in range(0, H, 34):
        out.append(rect(0, y, W, 1, "#CDD1D9"))
    out.append(rect(300, 0, 100, H, "#8E96A8") + rect(312, 0, 88, H, "#EEF1F6"))
    out.append(rect(46, 170, 220, 146, NUIT, 2))
    for cx, cy in ((58, 182), (254, 182), (58, 304), (254, 304)):
        out.append(f'<circle cx="{cx}" cy="{cy}" r="4" fill="#B8BFCC"/>')
    out.append(img("kadenz-logo-horizontal-negatif.svg", 70, 206, 150, 44))
    out.append(rect(76, 266, 30, 3, BLANC) + rect(110, 266, 30, 3, BLANC) + rect(144, 266, 30, 3, AMBRE))
    out.append(texte(76, 292, "Gestion commerciale", 13, "#C5CAE0", 500, "start"))
    return out


def vitrophanie():
    s, out = 440, []
    m = 410 / 2.2   # px par mètre
    out.append(decor(s))
    out.append(rect(30, 30, 270, 410, "#8E96A8"))
    out.append(rect(42, 42, 118, 398, "#DCE3EC") + rect(170, 42, 118, 398, "#DCE3EC"))
    y0, y1 = s - 1.6 * m, s - 1.1 * m
    out.append(rect(42, y0, 246, y1 - y0, BLANC, 0, 'opacity=".82"'))
    out.append(rect(100, (y0 + y1) / 2 - 18, 130, 36, BLANC))
    out.append(img("kadenz-logo-horizontal.svg", 104, (y0 + y1) / 2 - 16, 122, 32))
    out.append(rect(152, y1 + 20, 6, 60, "#7C8496") + rect(172, y1 + 20, 6, 60, "#7C8496"))
    out.append(personne(352, s, 1.75 * m))
    return out


def totem():
    s, out = 430, []
    m = 300 / 2.2
    out.append(decor(s))
    out.append(rect(140, 120, 110, 300, NUIT, 2))
    out.append(imbrique(gm.motif_tournant(NUIT, PIECES, 110, 88, 22), 140, 120, 110, 88))
    out.append(img("kadenz-logo-empile-negatif.svg", 157, 228, 76, 94))
    out.append(rect(160, 350, 20, 3, BLANC) + rect(185, 350, 20, 3, BLANC) + rect(210, 350, 20, 3, AMBRE))
    out.append(rect(130, 420, 130, 10, "#7C8496"))
    out.append(personne(325, s, 1.75 * m))
    return out


def kakemono():
    s, out = 430, []
    m = 300 / 2.0
    x, y, w = 120, 110, 128
    out.append(decor(s))
    out.append(rect(x, y, w, 300, NUIT))
    out.append(imbrique(gm.motif_fondu(NUIT, PIECES, w, 110, 16, 0.85, "bas"), x, y, w, 110))
    out.append(img("kadenz-logo-empile-negatif.svg", x + 30, y + 112, 68, 84))
    out.append(texte(x + w / 2, y + 222, "Gestion commerciale", 10.5, BLANC, 700, police="Space Grotesk"))
    for i, t in enumerate(("Devis et proforma", "Bons de commande", "Bordereaux de livraison", "Factures et paiements")):
        out.append(texte(x + w / 2, y + 240 + i * 13, t, 8.5, "#C5CAE0", 400))
    out.append(rect(x, y + 294, w, 6, AMBRE))
    out.append(rect(x - 8, 410, w + 16, 12, "#7C8496", 2))
    out.append(personne(320, s, 1.75 * m))
    return out


def stand():
    s, out = 430, []
    out.append(decor(s, "#F0F2F6"))
    out.append(rect(20, 70, 360, 260, NUIT))
    out.append(imbrique(gm.motif_tournant(NUIT, PIECES, 170, 260, 34), 20, 70, 170, 260))
    out.append(img("kadenz-logo-horizontal-negatif.svg", 210, 160, 150, 44))
    out.append(rect(230, 330, 120, 100, NUIT, 2))
    out.append(img("kadenz-symbole-blanc.svg", 268, 356, 44, 44))
    out.append(rect(20, 330, 360, 4, "#11173D"))
    out.append(personne(120, s, 182))
    return out


def postes():
    s, out = 430, []
    out.append(decor(s, "#F0F2F6"))
    noms = (("ventes", "Ventes"), ("commandes", "Commandes"), ("livraisons", "Livraisons"), ("paiements", "Paiements"))
    for i, (m, lib) in enumerate(noms):
        x = 14 + i * 96
        out.append(rect(x + 43, 0, 2, 80, "#9AA3B5"))
        out.append(rect(x + 8, 80, 72, 96, BLANC, 2, f'stroke="{TRAIT}"'))
        out.append(img(f"kadenz-module-{m}.svg", x + 20, 90, 48, 48))
        out.append(texte(x + 44, 160, lib, 10, NUIT, 600))
        out.append(rect(x + 4, 340, 80, 6, "#9AA3B5") + rect(x + 10, 346, 4, 84, "#9AA3B5") + rect(x + 74, 346, 4, 84, "#9AA3B5"))
        out.append(rect(x + 22, 300, 44, 30, NUIT, 2) + rect(x + 42, 330, 4, 10, "#9AA3B5"))
    return out


SCENES = [
    ("Façade : enseigne caisson", facade,
     "Caisson Nuit au motif Fondu, logo blanc rétroéclairé sur la zone calme, porte vitrée marquée du symbole.",
     "Dimensions selon la façade ; lettrage en relief ou face lumineuse."),
    ("Drapeau sur potence", drapeau,
     "Enseigne double face perpendiculaire au mur : le symbole seul, blanc sur Indigo, lisible de loin dans la rue.",
     "Environ 600 x 600 mm, potence murale, chant Nuit."),
    ("Lettres boîtier d'accueil", lettres,
     "Le logo couleur en lettres découpées, posé sans caisson sur le mur de l'accueil, éclairage indirect.",
     "Largeur selon le mur ; comptoir blanc à bandeau Nuit."),
    ("Plaque d'entrée", plaque,
     "Plaque Nuit à côté de la porte : logo négatif, les trois temps (blanc, blanc, Ambre) et « Gestion commerciale ».",
     "300 x 200 mm, Dibond ou plexiglas, entretoises inox."),
    ("Vitrophanie de porte", vitrophanie,
     "Bande dépolie à hauteur des yeux (repère de sécurité des portes vitrées), logo couleur au centre.",
     "Adhésif dépoli et impression couleur, hauteur 1,1 à 1,6 m."),
    ("Totem extérieur", totem,
     "Totem autoportant Nuit : motif Tournant en tête, logo empilé blanc, trois temps au-dessous.",
     "Environ 2,2 m de haut, socle lesté."),
    ("Kakémono Fondu", kakemono,
     "Le kakémono actuel avec le motif Fondu en tête et le pied Ambre de l'ancienne identité.",
     "850 x 2000 mm, enrouleur."),
    ("Stand de salon", stand,
     "Mur d'image : motif Tournant à gauche, logo sur la zone calme à droite ; comptoir Nuit au symbole blanc.",
     "Environ 3 x 2,5 m, toile tendue."),
    ("Postes de démonstration", postes,
     "Un panneau suspendu par poste, avec la marque de son module : le visiteur trouve la démonstration qu'il cherche.",
     "Panneaux 400 x 530 mm ; familles de modules encore à valider."),
]


def page(cartes):
    return f"""<!doctype html>
<html lang="fr"><head><meta charset="utf-8"><title>Signalétique : propositions</title>
<style>
@font-face{{font-family:Inter;src:url(../planches/polices/Inter.woff2) format("woff2");font-weight:100 900}}
@font-face{{font-family:"Space Grotesk";src:url(../planches/polices/SpaceGrotesk.ttf) format("truetype");font-weight:300 700}}
*{{box-sizing:border-box;margin:0;font-style:normal}}
body{{background:#fff;color:#1F2933;font-family:Inter,sans-serif;width:2400px;padding:96px 112px}}
header{{display:flex;justify-content:space-between;align-items:baseline;margin-bottom:28px}}
h1{{font-family:"Space Grotesk";font-weight:700;font-size:64px;letter-spacing:-.01em}}
h1 span{{color:#2D3A8C;font-size:30px;margin-right:24px}}
.sous{{color:#52606D;font-size:24px}}
.constat{{font-size:22px;color:#52606D;line-height:1.5;margin-bottom:56px;max-width:2150px}}
.constat b{{color:#1F2933}}
.grille{{display:grid;grid-template-columns:repeat(5,{W}px);justify-content:space-between;row-gap:56px}}
.panneau{{width:{W}px;height:{H}px;border-radius:4px;overflow:hidden;border:1px solid #D9DCE3;background:{BRUME};display:flex;align-items:center;justify-content:center}}
.panneau svg{{display:block}}
.panneau img{{width:{W - 24}px}}
.num{{color:#2D3A8C;font-weight:600;font-size:18px;margin-top:20px}}
h2{{font-family:"Space Grotesk";font-weight:700;font-size:28px;margin:4px 0 10px}}
p{{font-size:17px;line-height:1.5;color:#52606D}}
p.dim{{margin-top:8px;color:#1F2933;font-weight:500}}
</style></head><body>
<header><h1><span>07</span>Signalétique : propositions</h1>
<div class="sous">Quatre pièces, 2026-11-13. Proposition : rien n'est intégré à l'identité.</div></header>
<p class="constat"><b>Constat.</b> L'ancienne planche montrait une vraie scène (sol, porte vitrée, drapeau sur potence, pied Ambre du kakémono) ; la nouvelle a une façade sommaire, un drapeau sur mât qui se lit mal et des dimensions laissées en blanc.
Voici neuf supports, avec les motifs retenus (Fondu, Tournant) et une silhouette de 1,75 m pour l'échelle. Les dimensions sont indicatives, à confirmer avec le fabricant.</p>
<div class="grille">{"".join(cartes)}</div>
</body></html>
"""


if __name__ == "__main__":
    gm.D_MOT, gm.L_MOT = gm.mot()
    cartes = [f'<div><div class="panneau"><img src="../planches/images/07-signage.png" alt=""></div>'
              f'<div class="num">Référence</div><h2>Planche 07 actuelle</h2>'
              f'<p>Enseigne caisson, drapeau et kakémono, pour comparer.</p></div>']
    for i, (nom, fn, idee, dim) in enumerate(SCENES, 1):
        dessin = (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}" aria-hidden="true">'
                  + "".join(fn()) + "</svg>")
        cartes.append(f'<div><div class="panneau">{dessin}</div><div class="num">Proposition {i}</div>'
                      f'<h2>{nom}</h2><p>{idee}</p><p class="dim">{dim}</p></div>')
    html = ICI / "index.html"
    html.write_text(page(cartes), encoding="utf-8")
    png = ICI / "propositions-signaletique.png"
    subprocess.run([CHROME, "--headless=new", "--hide-scrollbars", "--force-device-scale-factor=1",
                    "--allow-file-access-from-files", f"--screenshot={png}", "--window-size=2400,1960", html.as_uri()],
                   check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    print("ok", len(SCENES))
