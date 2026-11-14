"""Propositions pour un nouveau jeu d'icônes de l'application (2026-11-14).

Même dessin pour les trois pistes, rendu selon trois styles :
A : trait fin arrondi (1,5 sur 24), au repos ; plein à l'état actif, comme les icônes de X.
B : duo, trait fin et silhouette teintée à 25 % ; l'état actif garde la teinte.
C : plein, silhouettes pleines aux détails découpés, coins vifs.

Chaque icône est faite de formes à trois rôles :
- corps : contour au repos, rempli à l'état plein ;
- detail : trait au repos, découpé dans le plein ;
- trait : toujours un trait (loupe, croix, second personnage) ;
- dessus : un corps posé sur les autres (le crayon du devis), détaché par un blanc dans le plein.

Grille de 24 x 24, zone utile 2,5 à 21,5. Une seule couleur (#000) : l'application recolore
l'icône (colorize), la teinte du duo passe par l'opacité.
Usage : python generer.py ; sorties dans icones/ et planche.html.
"""

from math import cos, pi, sin
from pathlib import Path

ICI = Path(__file__).resolve().parent
SORTIE = ICI / "icones"


def rect(x, y, w, h, r=2):
    return ("rect", x, y, w, h, r)


def chemin(d):
    return ("path", d)


def cercle(cx, cy, r):
    return ("circle", cx, cy, r)


def engrenage():
    """Huit dents trapézoïdales, rayon 9,5 aux dents, 7,4 au creux."""
    pts = []
    n, ro, ri = 8, 9.5, 7.4
    for k in range(n):
        a = 2 * pi * k / n
        for da, r in ((-0.30, ri), (-0.17, ro), (0.17, ro), (0.30, ri)):
            pts.append((12 + r * cos(a + da), 12 + r * sin(a + da)))
    return "M" + "L".join(f"{x:.2f} {y:.2f}" for x, y in pts) + "Z"


# Chaque icône : liste de (rôle, forme)
ICONES = {
    "tableau-de-bord": [
        ("corps", rect(3, 3, 7.5, 10)), ("corps", rect(13.5, 3, 7.5, 6)),
        ("corps", rect(13.5, 12, 7.5, 9)), ("corps", rect(3, 16, 7.5, 5)),
    ],
    "devis": [
        ("corps", chemin("M12 21H6a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h8l5 5v3.5")),
        ("detail", chemin("M14 3v5h5")),
        ("detail", chemin("M8 9.5h3M8 13h6")),
        ("dessus", chemin("M19 13.5l2.5 2.5-5.5 5.5H13.5V19Z")),
    ],
    "factures": [
        ("corps", chemin("M5 3h14v18l-2.33-1.5L14.33 21 12 19.5 9.67 21 7.33 19.5 5 21Z")),
        ("detail", chemin("M8.5 8h7M8.5 11.5h7M8.5 15h4")),
    ],
    "avoirs": [
        ("corps", chemin("M5 3h14v18l-2.33-1.5L14.33 21 12 19.5 9.67 21 7.33 19.5 5 21Z")),
        ("detail", chemin("M10.5 7.5 8 10l2.5 2.5M8 10h5a2.5 2.5 0 0 1 0 5h-2")),
    ],
    "bons-commande": [
        ("corps", rect(5, 4.5, 14, 16.5)),
        ("corps", rect(9, 2.5, 6, 4, 1)),
        ("detail", chemin("M8.5 11.5l1.5 1.5 2.5-2.5M14 12h1.5M8.5 17h7")),
    ],
    "livraisons": [
        ("corps", chemin("M2.5 5h11v8h-11Z")),
        ("corps", chemin("M13.5 8h4l3.5 3.5V13h-7.5Z")),
        ("corps", cercle(7, 17.5, 2)), ("corps", cercle(17, 17.5, 2)),
    ],
    "reglements": [
        ("corps", rect(2.5, 6, 19, 12)),
        ("detail", cercle(12, 12, 2.75)),
        ("detail", chemin("M6 12h.01M18 12h.01")),
    ],
    "relances": [
        ("corps", chemin("M6 16.5V11a6 6 0 0 1 12 0v5.5l1.5 1.5h-15Z")),
        ("trait", chemin("M10 21h4")),
        ("trait", chemin("M2.5 8a10 10 0 0 1 2.5-4.5M21.5 8A10 10 0 0 0 19 3.5")),
    ],
    "tiers": [
        ("corps", cercle(9, 8, 3.5)),
        ("corps", chemin("M2.5 20a6.5 6.5 0 0 1 13 0Z")),
        ("trait", chemin("M15.5 4.7a3.5 3.5 0 0 1 0 6.6M18 14.2A6.5 6.5 0 0 1 21.5 20")),
    ],
    "produits": [
        ("corps", chemin("M12 2.5l8.5 4.75v9.5L12 21.5l-8.5-4.75v-9.5Z")),
        ("detail", chemin("M3.5 7.25 12 12l8.5-4.75M12 12v9.5")),
    ],
    "parametres": [
        ("corps", chemin(engrenage())),
        ("detail", cercle(12, 12, 3)),
    ],
    "rechercher": [
        ("trait", cercle(10.5, 10.5, 6.5)),
        ("trait", chemin("M15.5 15.5 21 21")),
    ],
    "imprimer": [
        ("corps", chemin("M6 17H4a1.5 1.5 0 0 1-1.5-1.5v-6A1.5 1.5 0 0 1 4 8h16a1.5 1.5 0 0 1 1.5 1.5v6A1.5 1.5 0 0 1 20 17h-2")),
        ("corps", chemin("M6 8V3h12v5")),
        ("corps", rect(6, 13, 12, 8, 1)),
        ("detail", chemin("M6 13h12M17.5 10.5h.01")),
    ],
    "supprimer": [
        ("trait", chemin("M3.5 6h17M9 6V3.5h6V6")),
        ("corps", chemin("M5.5 6l1 15h11l1-15")),
        ("detail", chemin("M10 10v7M14 10v7")),
    ],
    "nouveau": [
        ("trait", chemin("M12 5v14M5 12h14")),
    ],
}

# Libellés des entrées, pour la planche
LIBELLES = {
    "tableau-de-bord": "Tableau de bord", "devis": "Devis", "factures": "Factures", "avoirs": "Avoirs",
    "bons-commande": "Bons de commande", "livraisons": "Livraisons", "reglements": "Règlements",
    "relances": "Relances", "tiers": "Tiers", "produits": "Produits", "parametres": "Paramètres",
    "rechercher": "Rechercher", "imprimer": "Imprimer", "supprimer": "Supprimer", "nouveau": "Nouveau",
}

STYLES = {
    "a": {"ep": 1.5, "cap": "round", "join": "round", "rk": 1.0},
    "b": {"ep": 1.5, "cap": "round", "join": "round", "rk": 1.0},
    "c": {"ep": 1.75, "cap": "square", "join": "miter", "rk": 0.25},
}


def forme(f, rk, attrs=""):
    if f[0] == "rect":
        _, x, y, w, h, r = f
        return f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r * rk:g}"{attrs}/>'
    if f[0] == "circle":
        _, cx, cy, r = f
        return f'<circle cx="{cx}" cy="{cy}" r="{r}"{attrs}/>'
    return f'<path d="{f[1]}"{attrs}/>'


def icone(nom, piste, etat):
    """etat : 'repos' ou 'plein'."""
    st = STYLES[piste]
    ep, rk = st["ep"], st["rk"]
    trait = (f'fill="none" stroke="#000" stroke-width="{ep}" stroke-linecap="{st["cap"]}" '
             f'stroke-linejoin="{st["join"]}"')
    parts = ICONES[nom]
    corps = [f for r, f in parts if r == "corps"]
    details = [f for r, f in parts if r == "detail"]
    traits = [f for r, f in parts if r == "trait"]
    dessus = [f for r, f in parts if r == "dessus"]
    plein = piste == "c" or (piste == "a" and etat == "plein")
    defs, out = "", []
    if plein:
        if details or dessus:
            # les détails sont découpés dans le plein ; un corps de dessus y laisse un blanc autour de lui
            masque = "".join(forme(f, rk) for f in details)
            masque += "".join(forme(f, rk, f' fill="#000" stroke-width="{ep * 2.5}"') for f in dessus)
            defs = (f'<defs><mask id="d" maskUnits="userSpaceOnUse" x="0" y="0" width="24" height="24">'
                    f'<rect width="24" height="24" fill="#fff"/>'
                    f'<g fill="none" stroke="#000" stroke-width="{ep}" stroke-linecap="{st["cap"]}" '
                    f'stroke-linejoin="{st["join"]}">{masque}</g></mask></defs>')
        g = "".join(forme(f, rk) for f in corps)
        m = ' mask="url(#d)"' if details or dessus else ""
        out.append(f'<g{m} fill="#000" stroke="#000" stroke-width="{ep}" stroke-linejoin="{st["join"]}" '
                   f'stroke-linecap="{st["cap"]}">{g}</g>')
        if dessus:
            out.append(f'<g fill="#000" stroke="#000" stroke-width="{ep}" stroke-linejoin="{st["join"]}">'
                       + "".join(forme(f, rk) for f in dessus) + "</g>")
    else:
        corps = corps + dessus
        if piste == "b" and corps:
            out.append('<g fill="#000" fill-opacity=".25" stroke="none">'
                       + "".join(forme(f, rk) for f in corps) + "</g>")
        out.append(f"<g {trait}>" + "".join(forme(f, rk) for f in corps + details) + "</g>")
    if traits:
        out.append(f"<g {trait}>" + "".join(forme(f, rk) for f in traits) + "</g>")
    return ('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24">'
            + defs + "".join(out) + "</svg>\n")


ACTUELLES = Path("../../../crates/app/ui/icones")
ACTUEL = {"tableau-de-bord": "menu", "devis": "menu", "factures": "menu", "avoirs": "menu",
          "bons-commande": "menu", "livraisons": "menu", "reglements": "menu", "relances": "menu",
          "tiers": "menu", "produits": "menu", "parametres": "menu", "rechercher": "actions",
          "imprimer": "actions", "supprimer": "actions", "nouveau": "actions"}

NOMS_PISTES = {
    "actuel": ("Jeu actuel", "Trait 1,75 carré, motif de l'ancien K"),
    "a": ("A : Trait fin", "Trait 1,5 arrondi au repos, plein quand l'entrée est active"),
    "b": ("B : Duo", "Trait 1,5 arrondi et silhouette teintée à 25 %"),
    "c": ("C : Plein", "Silhouettes pleines, détails découpés, coins vifs"),
}


def src(nom, piste, etat="repos"):
    if piste == "actuel":
        return f"{ACTUELLES.as_posix()}/{ACTUEL[nom]}/{nom}.svg"
    return f"icones/{piste}/{nom}{'-plein' if etat == 'plein' else ''}.svg"


MENU = ["tableau-de-bord", "devis", "factures", "avoirs", "bons-commande", "livraisons",
        "reglements", "relances", "tiers", "produits", "parametres"]


def barre(piste):
    """Menu latéral au format de l'application : 18 px, texte secondaire, entrée active Indigo."""
    lignes = []
    for i, nom in enumerate(MENU):
        actif = nom == "factures"
        etat = "plein" if actif and piste == "a" else "repos"
        filtre = "blanc" if actif else "gris"
        lignes.append(f'<div class="entree{" actif" if actif else ""}"><img class="{filtre}" '
                      f'src="{src(nom, piste, etat)}" width="18" height="18"><span>{LIBELLES[nom]}</span></div>')
    return "".join(lignes)


def planche():
    pistes = ["actuel", "a", "b", "c"]
    noms = list(ICONES)
    lignes = []
    for p in pistes:
        titre, sous = NOMS_PISTES[p]
        grands = "".join(f'<figure><img src="{src(n, p)}" width="48" height="48">'
                         f'<img class="petit" src="{src(n, p)}" width="18" height="18">'
                         f'<figcaption>{LIBELLES[n]}</figcaption></figure>' for n in noms)
        pleins = ""
        if p == "a":
            pleins = ('<p class="note">État actif (plein)</p><div class="grille">'
                      + "".join(f'<figure><img src="{src(n, p, "plein")}" width="48" height="48">'
                                f'<img class="petit" src="{src(n, p, "plein")}" width="18" height="18"></figure>'
                                for n in noms) + "</div>")
        lignes.append(f'<section><div class="tete"><h2>{titre}</h2><p>{sous}</p></div>'
                      f'<div class="corps"><div><div class="grille">{grands}</div>{pleins}</div>'
                      f'<nav>{barre(p)}</nav></div></section>')
    return f"""<!doctype html>
<html lang="fr"><head><meta charset="utf-8"><title>Icônes : propositions</title>
<style>
body {{ margin: 0; background: #f4f5f8; font: 13px/1.4 Inter, "Segoe UI", sans-serif; color: #1f2933; }}
header {{ padding: 28px 32px 8px; }}
h1 {{ font: 600 22px Manrope, "Segoe UI", sans-serif; margin: 0 0 4px; }}
header p {{ margin: 0; color: #52606d; }}
section {{ background: #fff; margin: 16px 32px; padding: 20px 24px; border: 1px solid #e3e6ee; border-radius: 4px; }}
.tete {{ display: flex; gap: 16px; align-items: baseline; margin-bottom: 12px; }}
h2 {{ font: 600 16px Manrope, "Segoe UI", sans-serif; margin: 0; }}
.tete p {{ margin: 0; color: #52606d; }}
.corps {{ display: flex; gap: 32px; align-items: flex-start; }}
.grille {{ display: grid; grid-template-columns: repeat(8, 92px); gap: 10px 4px; }}
figure {{ margin: 0; text-align: center; }}
figure img {{ display: block; margin: 0 auto 6px; }}
figure img.petit {{ margin-bottom: 4px; }}
figcaption {{ font-size: 11px; color: #52606d; }}
.note {{ margin: 14px 0 8px; color: #52606d; font-size: 12px; }}
nav {{ width: 210px; flex: none; padding: 8px; border: 1px solid #e3e6ee; border-radius: 4px; }}
.entree {{ height: 36px; display: flex; align-items: center; gap: 10px; padding: 0 12px; border-radius: 4px; color: #52606d; }}
.entree.actif {{ background: #2d3a8c; color: #fff; }}
img.gris {{ filter: invert(36%) sepia(10%) saturate(700%) hue-rotate(170deg) brightness(92%); }}
img.blanc {{ filter: invert(1); }}
</style></head><body>
<header><h1>Jeu d'icônes de Kadenz : trois pistes</h1>
<p>Même dessin pour les trois pistes. En haut de chaque case, 48 px ; dessous, 18 px, la taille de l'application. À droite, le menu latéral.</p></header>
{''.join(lignes)}
</body></html>
"""


if __name__ == "__main__":
    for p in STYLES:
        (SORTIE / p).mkdir(parents=True, exist_ok=True)
        for nom in ICONES:
            (SORTIE / p / f"{nom}.svg").write_text(icone(nom, p, "repos"), encoding="utf-8")
            if p == "a":
                (SORTIE / p / f"{nom}-plein.svg").write_text(icone(nom, p, "plein"), encoding="utf-8")
    (ICI / "planche.html").write_text(planche(), encoding="utf-8")
    print("ok")
