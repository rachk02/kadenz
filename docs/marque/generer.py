"""Marque Kadenz, « Quatre pièces », version Bleu (validée le 2026-11-11).

Autonome : géométrie, mots vectorisés (fontTools, Inter SemiBold du dépôt) et tous les SVG.
Usage : python generer.py <racine du dépôt>
Sorties : les SVG de docs/marque, à côté de ce fichier. Les PNG, le .ico et les icônes web
sont rendus à part (voir README.md).
"""

import math
import sys
from pathlib import Path

from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont

ICI = Path(__file__).resolve().parent
POLICE = Path(sys.argv[1]) / "crates/app/ui/polices/Inter-SemiBold.ttf"

BLANC = "#FFFFFF"
VERSIONS = {
    # aplat : couleur de la tuile et des surfaces ; clair : (pièces, disque, mot) sur fond clair ;
    # fonce : (pièces, disque, mot) sur l'aplat ou un fond sombre ; module : (traits, pièce pleine).
    "bleu": {"nom": "Bleu", "aplat": "#1F4FD8",
             "clair": ("#1F4FD8", "#1F4FD8", "#1A1033"), "fonce": (BLANC, BLANC, BLANC),
             "module": ("#1F4FD8", "#1F4FD8"), "motif": ("#1F4FD8", "#2A59DF")},
}


# --- outils ----------------------------------------------------------------------
def f(v):
    return f"{v:.2f}".rstrip("0").rstrip(".")


def poly(points):
    return "M" + " L".join(f"{f(x)} {f(y)}" for x, y in points) + " Z"


def cercle(cx, cy, r, sens=1):
    return (f"M{f(cx - r)} {f(cy)} A{f(r)} {f(r)} 0 1 {sens} {f(cx + r)} {f(cy)}"
            f" A{f(r)} {f(r)} 0 1 {sens} {f(cx - r)} {f(cy)} Z")


def svg(corps, w=256, h=256, titre="Kadenz"):
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {f(w)} {f(h)}" role="img" aria-label="{titre}">'
            f'<title>{titre}</title>{corps}</svg>\n')


# --- géométrie -------------------------------------------------------------------
def pieces(marge=16, jeu=16, optique=True):
    """Carré, quart de disque, triangle rectangle, disque sur une grille 2 x 2.
    Débord optique : quart + 1,5 %, disque + 3 %. Petite coupe : marge 10, jeu 26, sans débord."""
    c = (256 - 2 * marge - jeu) / 2
    a0, a1 = marge, marge + c + jeu
    rq = c * (1.015 if optique else 1)
    rd = c / 2 * (1.03 if optique else 1)
    cxq, cyq = a1, a0 + c
    return {
        "carre": poly([(a0, a0), (a0 + c, a0), (a0 + c, a0 + c), (a0, a0 + c)]),
        "quart": f"M{f(cxq)} {f(cyq)} L{f(cxq)} {f(cyq - rq)} A{f(rq)} {f(rq)} 0 0 1 {f(cxq + rq)} {f(cyq)} Z",
        "triangle": poly([(a0, a1), (a0 + c, a1 + c), (a0, a1 + c)]),
        "disque": cercle(a1 + c / 2, a1 + c / 2, rd),
    }, (a0, a1, c, rq, rd)


def anneaux(t=12):
    """Les quatre pièces au trait, construites en surfaces (contour extérieur et creux en sens inverse)."""
    p, (a0, a1, c, rq, rd) = pieces()
    s2 = math.sqrt(2)
    carre = p["carre"] + " " + poly([(a0 + t, a0 + t), (a0 + t, a0 + c - t), (a0 + c - t, a0 + c - t), (a0 + c - t, a0 + t)])
    cxq, cyq = a1, a0 + c
    ri = rq - t
    yi = cyq - math.sqrt(ri * ri - t * t)
    xi = cxq + math.sqrt(ri * ri - t * t)
    quart = p["quart"] + f" M{f(cxq + t)} {f(cyq - t)} L{f(xi)} {f(cyq - t)} A{f(ri)} {f(ri)} 0 0 0 {f(cxq + t)} {f(yi)} Z"
    tri_in = [(a0 + t, a1 + t + t * s2), (a0 + t, a1 + c - t), (a0 + c - t - t * s2, a1 + c - t)][::-1]
    triangle = p["triangle"] + " " + poly(tri_in)
    cx = a1 + c / 2
    disque = p["disque"] + " " + cercle(cx, cx, rd - t, sens=0)
    return {"carre": carre, "quart": quart, "triangle": triangle, "disque": disque}


def symbole(p, pieces_c, disque_c):
    d = " ".join(p[k] for k in ("carre", "quart", "triangle"))
    if pieces_c == disque_c:
        return f'<path fill="{pieces_c}" d="{d} {p["disque"]}"/>'
    return f'<path fill="{pieces_c}" d="{d}"/><path fill="{disque_c}" d="{p["disque"]}"/>'


# --- mot -------------------------------------------------------------------------
HAUTEUR_X = 0.42 * 224          # hauteur d'x du mot dans le logo horizontal (canevas de 256)
X_INTER, ASC_INTER = 1118, 1490  # Inter SemiBold : hauteur d'x et ascendante (unités de la police)


def mot(texte="kadenz", interlettre=-0.01):
    t = TTFont(POLICE)
    gs, cmap = t.getGlyphSet(), t.getBestCmap()
    pen = SVGPathPen(gs)
    x, upm = 0.0, t["head"].unitsPerEm
    for ch in texte:
        g = cmap[ord(ch)]
        gs[g].draw(TransformPen(pen, (1, 0, 0, -1, x, 0)))
        x += gs[g].width + interlettre * upm
    return pen.getCommands(), x - interlettre * upm


D_MOT, L_MOT = None, None


def trace_mot(x, base, e, couleur):
    return f'<path fill="{couleur}" transform="translate({f(x)} {f(base)}) scale({e:.5f})" d="{D_MOT}"/>'


def horizontal(corps_symbole, couleur_mot):
    e = HAUTEUR_X / X_INTER
    x_mot, base = 256 + 24, 128 + HAUTEUR_X / 2
    w = x_mot + L_MOT * e + 16
    return svg(corps_symbole + trace_mot(x_mot, base, e, couleur_mot), w, 256)


def empile(corps_symbole, couleur_mot):
    """Symbole centré au-dessus du mot ; le mot fait 1,15 fois la largeur utile du symbole (224)."""
    e = 1.15 * 224 / L_MOT
    w = max(256, L_MOT * e + 32)
    xs = (w - 256) / 2
    haut_mot = 240 + 40 + ASC_INTER * e        # bas du symbole + écart + ascendante
    h = haut_mot + 16
    return svg(f'<g transform="translate({f(xs)} 0)">{corps_symbole}</g>' + trace_mot((w - L_MOT * e) / 2, haut_mot, e, couleur_mot), w, h)


def mot_seul(couleur):
    e = HAUTEUR_X / X_INTER
    h = ASC_INTER * e + 32
    return svg(trace_mot(16, 16 + ASC_INTER * e, e, couleur), L_MOT * e + 32, h)


def tuile(corps, fond, rx=16, echelle=0.62):
    off = 128 * (1 - echelle)
    return svg(f'<rect width="256" height="256" rx="{rx}" fill="{fond}"/>'
               f'<g transform="translate({f(off)} {f(off)}) scale({echelle})">{corps}</g>')


def motif(fond, trace, w=240, h=800, s=40):
    """Les quatre pièces en rangées, décalées d'un temps à chaque ligne : la cadence, ton sur ton."""
    m = 7
    out = [f'<rect width="{w}" height="{h}" fill="{fond}"/>']
    for r in range(h // s):
        for col in range(w // s):
            x, y = col * s + m, r * s + m
            k = s - 2 * m
            sorte = (col + r) % 4
            if sorte == 0:
                d = poly([(x, y), (x + k, y), (x + k, y + k), (x, y + k)])
            elif sorte == 1:
                d = f"M{f(x)} {f(y + k)} L{f(x)} {f(y)} A{f(k)} {f(k)} 0 0 1 {f(x + k)} {f(y + k)} Z"
            elif sorte == 2:
                d = poly([(x, y), (x + k, y + k), (x, y + k)])
            else:
                d = cercle(x + k / 2, y + k / 2, k / 2)
            out.append(f'<path fill="{trace}" d="{d}"/>')
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" aria-hidden="true">'
            + "".join(out) + "</svg>\n")


if __name__ == "__main__":
    D_MOT, L_MOT = mot()
    p, _ = pieces()
    pp, _ = pieces(marge=10, jeu=26, optique=False)
    an = anneaux()
    familles = {"ventes": "carre", "commandes": "quart", "livraisons": "triangle", "paiements": "disque"}
    for cle, v in VERSIONS.items():
        d = ICI
        ec = lambda nom, contenu: (d / nom).write_text(contenu, encoding="utf-8")
        pc, dc, mc = v["clair"]
        pf, df, mf = v["fonce"]
        clair, fonce = symbole(p, pc, dc), symbole(p, pf, df)
        noir, blanc = symbole(p, "#000000", "#000000"), symbole(p, BLANC, BLANC)
        ec("kadenz-symbole.svg", svg(clair))
        ec("kadenz-symbole-negatif.svg", svg(fonce))
        ec("kadenz-symbole-noir.svg", svg(noir))
        ec("kadenz-symbole-blanc.svg", svg(blanc))
        ec("kadenz-symbole-petit.svg", svg(symbole(pp, pc, dc)))
        ec("kadenz-symbole-petit-negatif.svg", svg(symbole(pp, pf, df)))
        ec("kadenz-icone.svg", tuile(symbole(pp, pf, df), v["aplat"]))
        ec("kadenz-logo-horizontal.svg", horizontal(clair, mc))
        ec("kadenz-logo-horizontal-negatif.svg", horizontal(fonce, mf))
        ec("kadenz-logo-horizontal-mono.svg", horizontal(noir, "#000000"))
        ec("kadenz-logo-empile.svg", empile(clair, mc))
        ec("kadenz-logo-empile-negatif.svg", empile(fonce, mf))
        ec("kadenz-mot.svg", mot_seul(mc))
        ec("kadenz-mot-negatif.svg", mot_seul(BLANC))
        trait, plein = v["module"]
        for nom, piece in familles.items():
            autres = " ".join(an[k] for k in an if k != piece)
            ec(f"kadenz-module-{nom}.svg",
               svg(f'<path fill="{trait}" fill-rule="evenodd" d="{autres}"/><path fill="{plein}" d="{p[piece]}"/>',
                   titre=f"Kadenz, module {nom}"))
        ec("motif-quatre-pieces.svg", motif(*v["motif"]))
    print("ok", L_MOT)
