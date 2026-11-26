"""Recadre les captures du manuel (copies de travail, jamais docs/manuel/captures).

Usage : py -3.13 recadrer_captures.py   (Pillow et numpy requis, Python 3.13)
Entrée : docs/manuel/captures/<chapitre>/*.png
Sortie : _captures/<chapitre>/*.png et _captures/recadrages.json (largeur, hauteur, densité de chaque copie)

Seule la zone blanche inutile du bas est retirée : on garde tout le haut (barre de titre, menu) et tout le
contenu significatif, avec 28 px de marge sous le dernier élément. Aucun élément de l'écran n'est coupé.
La densité (part de pixels non blancs) sert à agrandir un peu les captures chargées dans le manuel.
"""
import json
from pathlib import Path

import numpy as np
from PIL import Image

ICI = Path(__file__).resolve().parent
SRC = ICI.parents[2] / "docs" / "manuel" / "captures"
OUT = ICI / "_captures"
MARGE = 28
SEUIL = 246


def main():
    infos = {}
    for p in sorted(SRC.glob("*/*.png")):
        im = Image.open(p).convert("RGB")
        a = np.asarray(im.convert("L"))
        h, w = a.shape
        m = a < SEUIL
        comptes = m[:, 6:-6].sum(axis=1)
        bas = max([y for y in range(40, h - 6) if comptes[y] > 0] or [h - 7])
        haut = min(h, bas + MARGE)
        copie = im.crop((0, 0, w, haut))
        dest = OUT / p.parent.name / p.name
        dest.parent.mkdir(parents=True, exist_ok=True)
        copie.save(dest, optimize=True)
        zone = m[:haut]
        infos[f"{p.parent.name}/{p.name}"] = {"w": w, "h": haut, "dense": round(float(zone.mean()), 4)}
    (OUT / "recadrages.json").write_text(json.dumps(infos, indent=0), encoding="utf-8")
    hs = np.array([v["h"] for v in infos.values()])
    ds = np.array([v["dense"] for v in infos.values()])
    print(len(infos), "captures ; hauteur min/médiane/max", hs.min(), int(np.median(hs)), hs.max(),
          "; densité quantiles", np.round(np.percentile(ds, [10, 25, 50, 75, 90]), 3))


if __name__ == "__main__":
    main()
