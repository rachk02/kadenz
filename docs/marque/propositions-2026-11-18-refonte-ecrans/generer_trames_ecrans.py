"""Trames des écrans de création, d'inscription et de connexion (refonte acceptée
le 2026-11-18) : une pièce du symbole en points ambre sur un champ de points
indigo, comme sur les maquettes. Écrites dans crates/app/ui/icones/, sans fond :
le panneau de l'écran donne la couleur (jeton panneau de Theme).

Usage : py -3.13 generer_trames_ecrans.py
"""
from pathlib import Path

from generer_refonte import trame

ICI = Path(__file__).resolve().parent
ICONES = ICI.parents[2] / "crates" / "app" / "ui" / "icones"

# nom, largeur, hauteur, pièce, x, y, taille (mêmes valeurs que les maquettes)
TRAMES = [
    ("trame-carre", 640, 760, "carre", 150, 150, 400),
    ("trame-quart", 640, 760, "quart", 150, 150, 400),
    ("trame-triangle", 640, 760, "triangle", 150, 150, 400),
    ("trame-disque", 640, 760, "disque", 150, 150, 400),
    ("trame-etapes", 240, 720, "triangle", -40, 420, 330),
]

# Mode sombre (palettes grises du 2026-11-18) : points du champ en gris neutre au lieu
# d'indigo, pour qu'aucun bleu ne reste sur le panneau presque noir.
POINTS_SOMBRE = "#4A4D52"

if __name__ == "__main__":
    for nom, w, h, piece, x, y, t in TRAMES:
        pas = 12 if nom == "trame-etapes" else 14
        for suffixe, points in (("", "#2D3A8C"), ("-sombre", POINTS_SOMBRE)):
            svg = trame(w, h, piece, x, y, t, pas=pas, fond_points=points).replace(' style="display:block"', "")
            svg = svg.replace("<svg ", '<svg xmlns="http://www.w3.org/2000/svg" ', 1)
            (ICONES / f"{nom}{suffixe}.svg").write_text(svg, encoding="utf-8")
            print(ICONES / f"{nom}{suffixe}.svg", len(svg))
