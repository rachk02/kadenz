"""Propositions de motifs pour l'identité « Quatre pièces » (2026-11-13).

Proposition seulement : rien n'est intégré à l'identité tant que l'utilisateur n'a pas choisi.
Chaque motif reprend la géométrie du symbole (carré, quart de disque, triangle, disque) et la
palette de la charte (docs/marque/README.md).

Usage : python generer_motifs.py
Sorties, à côté de ce fichier : motifs/*.svg (un SVG par motif), index.html (planche),
propositions-motifs.png (planche rendue par Chrome sans interface).
"""

import random
import subprocess
from pathlib import Path

ICI = Path(__file__).resolve().parent
CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"

# Palette de la charte
NUIT, PIECES, INDIGO, AMBRE = "#1A2259", "#232C6B", "#2D3A8C", "#E3A23B"
BRUME, BLANC, PALE = "#F4F5F9", "#FFFFFF", "#E8EAF6"

W, H = 400, 500   # taille d'un panneau de la planche


def f(v):
    return f"{v:.2f}".rstrip("0").rstrip(".")


def poly(points):
    return "M" + " L".join(f"{f(x)} {f(y)}" for x, y in points) + " Z"


def cercle(cx, cy, r):
    return f"M{f(cx - r)} {f(cy)} A{f(r)} {f(r)} 0 1 1 {f(cx + r)} {f(cy)} A{f(r)} {f(r)} 0 1 1 {f(cx - r)} {f(cy)} Z"


def piece(sorte, x, y, k):
    """Une pièce dans la case (x, y, k), orientée comme dans le symbole."""
    if sorte == 0:
        return poly([(x, y), (x + k, y), (x + k, y + k), (x, y + k)])
    if sorte == 1:
        return f"M{f(x)} {f(y + k)} L{f(x)} {f(y)} A{f(k)} {f(k)} 0 0 1 {f(x + k)} {f(y + k)} Z"
    if sorte == 2:
        return poly([(x, y), (x + k, y + k), (x, y + k)])
    return cercle(x + k / 2, y + k / 2, k / 2)


def symbole_d(x, y, k):
    """Les quatre pièces du symbole dans un carré (x, y, k), proportions du canevas de 256."""
    e = k / 256
    c, a0, a1 = 104 * e, x + 16 * e, x + 136 * e
    b0, b1 = y + 16 * e, y + 136 * e
    return [piece(0, a0, b0, c), piece(1, a1, b0, c), piece(2, a0, b1, c), piece(3, a1, b1, c)]


def fond(couleur):
    return f'<rect width="{W}" height="{H}" fill="{couleur}"/>'


def chemin(d, couleur):
    return f'<path fill="{couleur}" d="{d}"/>'


def contour(d, couleur, t):
    return f'<path fill="none" stroke="{couleur}" stroke-width="{t}" stroke-linejoin="round" d="{d}"/>'


# --- les motifs -----------------------------------------------------------------------
def actuel():
    s, m = 40, 7
    out = [fond(NUIT)]
    for r in range(H // s + 1):
        for c in range(W // s + 1):
            out.append(chemin(piece((c + r) % 4, c * s + m, r * s + m, s - 2 * m), PIECES))
    return out


def mosaique():
    s, k = 100, 64
    out = [fond(NUIT)]
    for r in range(H // s + 1):
        for c in range(W // s + 1):
            out += [chemin(d, PIECES) for d in symbole_d(c * s + (s - k) / 2, r * s + (s - k) / 2, k)]
    return out


def trois_temps():
    s, m = 40, 8
    out = [fond(BRUME)]
    for r in range(H // s + 1):
        for c in range(W // s + 1):
            g = (c + r) % 4
            if g == 3:
                continue
            out.append(chemin(piece((c + r) % 4 if g < 2 else 3, c * s + m, r * s + m, s - 2 * m),
                              AMBRE if g == 2 else INDIGO))
    return out


def tournant():
    s = 50
    alea = random.Random(7)
    out = [fond(NUIT)]
    for r in range(H // s + 1):
        for c in range(W // s + 1):
            x, y = c * s, r * s
            d = f"M{x} {y} L{x + s} {y} A{s} {s} 0 0 1 {x} {y + s} Z"
            out.append(f'<path fill="{PIECES}" transform="rotate({90 * alea.randrange(4)} {x + s / 2} {y + s / 2})" d="{d}"/>')
    return out


def chevrons():
    s, m = 40, 5
    out = [fond(NUIT)]
    for r in range(H // s + 1):
        for c in range(W // s + 1):
            x, y, k = c * s + m, r * s + m, s - 2 * m
            if c % 2 == 0:
                d = poly([(x, y), (x + k, y + k), (x, y + k)])
            else:
                d = poly([(x + k, y), (x + k, y + k), (x, y + k)])
            out.append(chemin(d, PIECES))
    return out


def au_trait():
    s, m = 40, 9
    out = [fond(BLANC)]
    for r in range(H // s + 1):
        for c in range(W // s + 1):
            out.append(contour(piece((c + r) % 4, c * s + m, r * s + m, s - 2 * m), INDIGO, 2))
    return out


def plein_et_trait():
    s, k = 100, 72
    out = [fond(BRUME)]
    for r in range(H // s + 1):
        for c in range(W // s + 1):
            plein = (c + r) % 4
            for i, d in enumerate(symbole_d(c * s + (s - k) / 2, r * s + (s - k) / 2, k)):
                out.append(chemin(d, INDIGO) if i == plein else contour(d, INDIGO, 2.5))
    return out


def grand_format():
    out = [fond(NUIT)]
    out.append(f'<g transform="translate(-30 -10) scale(1.85)">'
               + "".join(chemin(d, PIECES) for d in symbole_d(0, 0, 256)) + "</g>")
    return out


def pulsation():
    s = 32
    out = [fond(NUIT)]
    for r in range(H // s + 1):
        for c in range(W // s + 1):
            fort = (c + r) % 3 == 0
            out.append(chemin(cercle(c * s + s / 2, r * s + s / 2, 10 if fort else 4.5), INDIGO if fort else PIECES))
    return out


def fondu():
    s, m = 40, 7
    out = [fond(NUIT)]
    for r in range(H // s + 1):
        for c in range(W // s + 1):
            e = max(0.0, 1 - (c * s) / (W * 0.62))
            if e <= 0.08:
                continue
            k = (s - 2 * m) * e
            x = c * s + (s - k) / 2
            y = r * s + (s - k) / 2
            out.append(chemin(piece((c + r) % 4, x, y, k), PIECES))
    return out


MOTIFS = [
    # (clé, nom, fonction, idée, usages, règle 1, logo en surimpression)
    ("actuel", "Quatre pièces (actuel)", actuel,
     "Les quatre pièces en rangées, décalées d'un temps : le motif de la charte, pour comparer.",
     "Bandeaux de l'application, couvertures, kakémono.", "Ton sur ton", False),
    ("mosaique", "Mosaïque", mosaique,
     "Le symbole entier répété en trame : l'équivalent de la Mosaïque de l'ancienne identité.",
     "Verso de carte, papier de soie, fond d'écran, emballage.", "Ton sur ton", False),
    ("trois-temps", "Trois temps", trois_temps,
     "Les pièces par groupes de trois, la troisième en Ambre, un silence entre les groupes : la cadence rendue visible.",
     "Papeterie, bandeaux clairs, réseaux sociaux.", "Deux couleurs : demande d'élargir la règle 1", False),
    ("tournant", "Tournant", tournant,
     "Le quart de disque seul, tourné au hasard d'une case à l'autre : arcs, vagues et disques se forment d'eux-mêmes.",
     "Grands fonds, écrans d'accueil, couvertures.", "Ton sur ton", False),
    ("chevrons", "Chevrons", chevrons,
     "Le triangle seul, retourné une case sur deux : des colonnes en zigzag, plus nerveuses.",
     "Bandeaux étroits, en-têtes de site, signalétique.", "Ton sur ton", False),
    ("au-trait", "Au trait", au_trait,
     "Le motif actuel en contour fin Indigo sur blanc, comme les marques de modules.",
     "Papier à en-tête de l'éditeur, fonds de page, impression en une couleur.", "Une couleur sur blanc", False),
    ("plein-et-trait", "Plein et trait", plein_et_trait,
     "Le symbole au trait avec une pièce pleine qui avance d'une case à l'autre : les quatre modules en cadence.",
     "Pages des modules, plaquettes commerciales, site.", "Une couleur sur Brume", False),
    ("grand-format", "Grand format", grand_format,
     "Le symbole agrandi et recadré par le bord : une seule forme, très grande, ton sur ton.",
     "Couvertures, kakémono, diapositive de titre.", "Ton sur ton", False),
    ("pulsation", "Pulsation", pulsation,
     "Une trame de points où un disque sur trois grossit : le temps fort du symbole, battu en continu.",
     "Fonds d'écran, chargement, arrière-plans discrets.", "Ton sur ton (Indigo sur Nuit)", False),
    ("fondu", "Fondu", fondu,
     "Le motif actuel dont les pièces rapetissent vers la droite : la zone calme du logo naît du motif lui-même.",
     "Bandeaux avec logo ou titre, connexion, présentation.", "Ton sur ton", True),
]


def svg(elements):
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}" aria-hidden="true">'
            + "".join(elements) + "</svg>")


def page(cartes):
    return f"""<!doctype html>
<html lang="fr"><head><meta charset="utf-8"><title>Motifs de marque : propositions</title>
<style>
@font-face{{font-family:Inter;src:url(../planches/polices/Inter.woff2) format("woff2");font-weight:100 900}}
@font-face{{font-family:"Space Grotesk";src:url(../planches/polices/SpaceGrotesk.ttf) format("truetype");font-weight:300 700}}
*{{box-sizing:border-box;margin:0;font-style:normal}}
body{{background:#fff;color:#1F2933;font-family:Inter,sans-serif;width:2400px;padding:96px 112px}}
header{{display:flex;justify-content:space-between;align-items:baseline;margin-bottom:28px}}
h1{{font-family:"Space Grotesk";font-weight:700;font-size:64px;letter-spacing:-.01em}}
h1 span{{color:#2D3A8C;font-size:30px;margin-right:24px}}
.sous{{color:#52606D;font-size:24px}}
.constat{{font-size:22px;color:#52606D;line-height:1.5;margin-bottom:56px;max-width:2100px}}
.constat b{{color:#1F2933}}
.grille{{display:grid;grid-template-columns:repeat(5,{W}px);justify-content:space-between;row-gap:56px}}
.panneau{{position:relative;width:{W}px;height:{H}px;border-radius:4px;overflow:hidden;border:1px solid #D9DCE3}}
.panneau svg{{display:block}}
.panneau img{{position:absolute;right:28px;top:50%;transform:translateY(-50%);width:200px}}
.num{{color:#2D3A8C;font-weight:600;font-size:18px;margin-top:20px}}
h2{{font-family:"Space Grotesk";font-weight:700;font-size:28px;margin:4px 0 10px}}
p{{font-size:17px;line-height:1.5;color:#52606D}}
p.usage{{margin-top:8px}}
p.regle{{margin-top:8px;color:#1F2933;font-weight:500}}
</style></head><body>
<header><h1><span>05</span>Motifs de marque : propositions</h1>
<div class="sous">Quatre pièces, 2026-11-13. Proposition : rien n'est intégré à l'identité.</div></header>
<p class="constat"><b>Constat.</b> L'ancienne identité avait trois motifs (Cadence, Trois temps, Mosaïque) ; la nouvelle n'en a qu'un (Quatre pièces) et sa version claire.
Voici neuf motifs de plus, tous tirés des quatre pièces et de la palette de la charte, à côté du motif actuel. Les trois règles de la planche 05 s'appliquent ; la règle 1 (ton sur ton) est rappelée sous chaque motif.</p>
<div class="grille">{"".join(cartes)}</div>
</body></html>
"""


if __name__ == "__main__":
    dossier = ICI / "motifs"
    dossier.mkdir(exist_ok=True)
    cartes = []
    for i, (cle, nom, fn, idee, usages, regle, logo) in enumerate(MOTIFS):
        dessin = svg(fn())
        (dossier / f"motif-{cle}.svg").write_text(dessin + "\n", encoding="utf-8")
        surimpression = '<img src="../kadenz-logo-horizontal-negatif.svg" alt="">' if logo else ""
        cartes.append(f'<div><div class="panneau">{dessin}{surimpression}</div>'
                      f'<div class="num">{"Référence" if i == 0 else f"Proposition {i}"}</div>'
                      f'<h2>{nom}</h2><p>{idee}</p><p class="usage">Usages : {usages}</p>'
                      f'<p class="regle">Règle 1 : {regle}</p></div>')
    html = ICI / "index.html"
    html.write_text(page(cartes), encoding="utf-8")
    png = ICI / "propositions-motifs.png"
    subprocess.run([CHROME, "--headless=new", "--hide-scrollbars", "--force-device-scale-factor=1",
                    f"--screenshot={png}", "--window-size=2400,1980", html.as_uri()], check=True,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    print("ok", len(MOTIFS))
