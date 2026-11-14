"""Deuxième tour des icônes (2026-11-14), après le choix de l'utilisateur : A et B.

Au repos : le trait fin de A (1,5, bouts arrondis). À l'état actif : le duo de B (même trait,
silhouette teintée à 25 %), à la place du plein de A, écarté.
Livraisons : le camion est écarté. Le bordereau de livraison atteste la remise de la marchandise
au client (cahier des charges, 6.5) ; quatre variantes autour du colis remis. Si un colis est
retenu, Produits passe à l'étiquette pour ne pas avoir deux boîtes dans le menu.

Rôles des formes : ceux de generer.py ; un corps « dessus » (pastille, épingle, crayon) laisse un
blanc autour de lui dans les formes qu'il recouvre.
Usage : python generer_v2.py ; sorties dans icones/v2/ et planche-v2.html.
"""

from generer import ICI, ICONES, LIBELLES, chemin, cercle, forme, rect

SORTIE = ICI / "icones" / "v2"
EP = 1.5
TRAIT = (f'fill="none" stroke="#000" stroke-width="{EP}" stroke-linecap="round" '
         f'stroke-linejoin="round"')

NOUVELLES = {
    "livraisons-colis-livre": [
        ("corps", rect(2.5, 4, 14, 13.5)),
        ("detail", chemin("M7.5 4v5l2-1.25 2 1.25V4")),
        ("dessus", cercle(17, 17, 4.5)),
        ("trait", chemin("M15 17l1.5 1.5 2.5-2.75")),
    ],
    "livraisons-colis-en-main": [
        ("corps", rect(8.5, 3, 10, 8.5, 1.5)),
        ("detail", chemin("M12 3v3h3V3")),
        ("trait", chemin("M2.5 14H6l3 2h3.5a1.5 1.5 0 0 1 0 3H9.5")),
        ("trait", chemin("M2.5 20.5H13l6.8-4.1a1.6 1.6 0 0 0-1.7-2.7L14 16.2")),
    ],
    "livraisons-colis-et-trajet": [
        ("corps", rect(2.5, 13, 8, 8, 1.5)),
        ("detail", chemin("M5.5 13v2.5h2V13")),
        ("dessus", chemin("M17.5 2.5a4 4 0 0 1 4 4c0 2.5-4 6.5-4 6.5s-4-4-4-6.5a4 4 0 0 1 4-4Z")),
        ("trait", cercle(17.5, 6.5, 1.25)),
        ("trait", chemin("M12.5 17.5h1.5a3.5 3.5 0 0 0 3.5-3.5v-.5")),
    ],
    "livraisons-bordereau-vise": [
        ("corps", rect(4.5, 2.5, 15, 19, 1.5)),
        ("detail", rect(8, 6, 6.5, 5.5, 1)),
        ("detail", chemin("M10.25 6v2h2V6")),
        ("dessus", cercle(17.5, 17.5, 4)),
        ("trait", chemin("M15.75 17.5l1.25 1.25 2.25-2.5")),
    ],
    "produits-etiquette": [
        ("corps", chemin("M3 12.2V4a1 1 0 0 1 1-1h8.2a1 1 0 0 1 .7.3l8.3 8.3a1 1 0 0 1 0 1.4l-8.2 8.2"
                         "a1 1 0 0 1-1.4 0L3.3 12.9a1 1 0 0 1-.3-.7Z")),
        ("detail", cercle(8, 8, 1.75)),
    ],
}

LIB = dict(LIBELLES)
LIB.update({
    "livraisons-colis-livre": "L1 : colis livré",
    "livraisons-colis-en-main": "L2 : colis remis en main",
    "livraisons-colis-et-trajet": "L3 : colis et destination",
    "livraisons-bordereau-vise": "L4 : bordereau visé",
    "produits-etiquette": "Produits : étiquette",
    "livraisons": "Camion (écarté)",
})
TOUTES = {**ICONES, **NOUVELLES}


def icone(nom, actif):
    parts = TOUTES[nom]
    corps = [f for r, f in parts if r == "corps"]
    details = [f for r, f in parts if r == "detail"]
    traits = [f for r, f in parts if r == "trait"]
    dessus = [f for r, f in parts if r == "dessus"]
    defs, m = "", ""
    if dessus:
        # blanc autour des formes de dessus, dans le trait et la teinte de ce qu'elles recouvrent
        cache = "".join(forme(f, 1, f' fill="#000" stroke="#000" stroke-width="{EP * 3}"') for f in dessus)
        defs = (f'<defs><mask id="d" maskUnits="userSpaceOnUse" x="0" y="0" width="24" height="24">'
                f'<rect width="24" height="24" fill="#fff"/>{cache}</mask></defs>')
        m = ' mask="url(#d)"'
    out = []
    if actif:
        teinte = "".join(forme(f, 1) for f in corps)
        out.append(f'<g{m} fill="#000" fill-opacity=".25">{teinte}</g>')
        if dessus:
            out.append('<g fill="#000" fill-opacity=".25">' + "".join(forme(f, 1) for f in dessus) + "</g>")
    out.append(f"<g{m} {TRAIT}>" + "".join(forme(f, 1) for f in corps + details) + "</g>")
    if dessus:
        out.append(f"<g {TRAIT}>" + "".join(forme(f, 1) for f in dessus) + "</g>")
    if traits:
        out.append(f"<g {TRAIT}>" + "".join(forme(f, 1) for f in traits) + "</g>")
    return ('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24">'
            + defs + "".join(out) + "</svg>\n")


def src(nom, actif=False):
    return f"icones/v2/{nom}{'-actif' if actif else ''}.svg"


def barre(entrees, actif_nom):
    lignes = []
    for nom in entrees:
        a = nom == actif_nom
        lib = LIBELLES.get(nom.split("-")[0], LIB[nom]) if nom.startswith(("livraisons-", "produits-")) else LIB[nom]
        lignes.append(f'<div class="entree{" actif" if a else ""}"><img class="{"blanc" if a else "gris"}" '
                      f'src="{src(nom, a)}" width="18" height="18"><span>{lib}</span></div>')
    return f'<nav>{"".join(lignes)}</nav>'


def case(nom, legende=True):
    lg = f"<figcaption>{LIB[nom]}</figcaption>" if legende else ""
    return (f'<figure><div class="paire"><img src="{src(nom)}" width="48" height="48">'
            f'<span class="tuile"><img class="blanc" src="{src(nom, True)}" width="48" height="48"></span></div>'
            f'<div class="paire petite"><img src="{src(nom)}" width="18" height="18">'
            f'<span class="tuile"><img class="blanc" src="{src(nom, True)}" width="18" height="18"></span></div>{lg}</figure>')


MENU = ["tableau-de-bord", "devis", "factures", "avoirs", "bons-commande", "LIV",
        "reglements", "relances", "tiers", "PROD", "parametres"]


def menu(liv, prod, actif):
    return barre([liv if n == "LIV" else prod if n == "PROD" else n for n in MENU], actif)


def planche():
    jeu = [n for n in ICONES if n != "livraisons"]
    variantes = ["livraisons-colis-livre", "livraisons-colis-en-main", "livraisons-colis-et-trajet",
                 "livraisons-bordereau-vise"]
    menus_liv = "".join(menu(v, "produits-etiquette", v) for v in variantes)
    return f"""<!doctype html>
<html lang="fr"><head><meta charset="utf-8"><title>Icônes : tour 2</title>
<style>
body {{ margin: 0; background: #f4f5f8; font: 13px/1.4 Inter, "Segoe UI", sans-serif; color: #1f2933; }}
header {{ padding: 28px 32px 8px; }}
h1 {{ font: 600 22px Manrope, "Segoe UI", sans-serif; margin: 0 0 4px; }}
header p, .tete p {{ margin: 0; color: #52606d; }}
section {{ background: #fff; margin: 16px 32px; padding: 20px 24px; border: 1px solid #e3e6ee; border-radius: 4px; }}
.tete {{ display: flex; gap: 16px; align-items: baseline; margin-bottom: 14px; }}
h2 {{ font: 600 16px Manrope, "Segoe UI", sans-serif; margin: 0; }}
.corps {{ display: flex; gap: 28px; align-items: flex-start; }}
.grille {{ display: grid; grid-template-columns: repeat(5, 150px); gap: 16px 8px; }}
figure {{ margin: 0; text-align: center; }}
.paire {{ display: flex; justify-content: center; gap: 8px; align-items: center; margin-bottom: 6px; }}
.tuile {{ display: inline-flex; background: #2d3a8c; border-radius: 4px; padding: 6px; }}
.petite .tuile {{ padding: 5px; }}
figcaption {{ font-size: 11px; color: #52606d; }}
.menus {{ display: flex; gap: 16px; flex-wrap: wrap; }}
nav {{ width: 200px; flex: none; padding: 8px; border: 1px solid #e3e6ee; border-radius: 4px; background: #fff; }}
.entree {{ height: 36px; display: flex; align-items: center; gap: 10px; padding: 0 12px; border-radius: 4px; color: #52606d; }}
.entree.actif {{ background: #2d3a8c; color: #fff; }}
img.gris {{ filter: invert(36%) sepia(10%) saturate(700%) hue-rotate(170deg) brightness(92%); }}
img.blanc {{ filter: invert(1); }}
</style></head><body>
<header><h1>Jeu d'icônes de Kadenz : tour 2</h1>
<p>Au repos, le trait fin de A ; à l'état actif, le duo de B (silhouette teintée). Dans chaque case : au repos, puis actif sur l'Indigo ; 48 px en haut, 18 px dessous.</p></header>
<section><div class="tete"><h2>Livraisons : quatre variantes</h2><p>Le bordereau atteste la remise de la marchandise au client, avec son visa</p></div>
<div class="grille">{''.join(case(v) for v in variantes)}{case("produits-etiquette")}</div></section>
<section><div class="tete"><h2>Dans le menu</h2><p>Livraisons active, avec chaque variante ; Produits en étiquette</p></div>
<div class="menus">{menus_liv}</div></section>
<section><div class="tete"><h2>Le jeu, au repos et actif</h2><p>Trait fin au repos, duo à l'état actif</p></div>
<div class="corps"><div class="grille">{''.join(case(n) for n in jeu)}</div>
{menu("livraisons-colis-livre", "produits-etiquette", "factures")}</div></section>
</body></html>
"""


if __name__ == "__main__":
    SORTIE.mkdir(parents=True, exist_ok=True)
    for nom in TOUTES:
        (SORTIE / f"{nom}.svg").write_text(icone(nom, False), encoding="utf-8")
        (SORTIE / f"{nom}-actif.svg").write_text(icone(nom, True), encoding="utf-8")
    (ICI / "planche-v2.html").write_text(planche(), encoding="utf-8")
    print("ok")
