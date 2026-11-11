"""Régénère les icônes matricielles de Kadenz à partir du K découpé.

Le K découpé (kadenz-icone.svg) sert à toutes les tailles, y compris sous
48 px : décision de l'utilisateur du 2026-11-11, qui remplace la version
réduite à jambe pleine (kadenz-icone-petite.svg) pour les petites icônes.

Sorties :
- crates/app/kadenz.ico (16 à 256 px), icône de kadenz.exe et de l'installateur ;
- crates/app/ui/icones/kadenz-32.png, kadenz-64.png, kadenz-256.png.

Le tracé est lu dans kadenz-icone.svg (tuile arrondie et polygones M/L/Z),
dessiné en suréchantillonnage puis réduit. Usage :
python docs/marque/generer_icones.py (Pillow requis).
"""

import re
from pathlib import Path

from PIL import Image, ImageDraw

ICI = Path(__file__).resolve().parent
RACINE = ICI.parent.parent
TAILLES_ICO = [16, 20, 24, 32, 40, 48, 64, 128, 256]
SURECHANTILLON = 1024


def lire_svg():
    svg = (ICI / "kadenz-icone.svg").read_text(encoding="utf-8")
    tuile = re.search(r'<rect[^>]*rx="([\d.]+)"[^>]*fill="(#[0-9a-fA-F]{6})"', svg)
    trace = re.search(r'<path fill="(#[0-9a-fA-F]{6})" transform="translate\(([-\d.]+) ([-\d.]+)\) scale\(([\d.]+)\)" d="([^"]+)"', svg)
    rx, fond = float(tuile.group(1)), tuile.group(2)
    encre, tx, ty, echelle, d = trace.group(1), float(trace.group(2)), float(trace.group(3)), float(trace.group(4)), trace.group(5)
    polygones = []
    for morceau in d.split("Z"):
        points = re.findall(r"[ML]\s*([-\d.]+)\s+([-\d.]+)", morceau)
        if points:
            polygones.append([(float(x) * echelle + tx, float(y) * echelle + ty) for x, y in points])
    return rx, fond, encre, polygones


def dessiner(taille, rx, fond, encre, polygones):
    f = SURECHANTILLON / 48.0
    image = Image.new("RGBA", (SURECHANTILLON, SURECHANTILLON), (0, 0, 0, 0))
    dessin = ImageDraw.Draw(image)
    dessin.rounded_rectangle((0, 0, SURECHANTILLON - 1, SURECHANTILLON - 1), radius=rx * f, fill=fond)
    for p in polygones:
        dessin.polygon([(x * f, y * f) for x, y in p], fill=encre)
    return image.resize((taille, taille), Image.LANCZOS)


def main():
    rx, fond, encre, polygones = lire_svg()
    images = {t: dessiner(t, rx, fond, encre, polygones) for t in TAILLES_ICO}
    images[256].save(RACINE / "crates" / "app" / "kadenz.ico", sizes=[(t, t) for t in TAILLES_ICO], append_images=[images[t] for t in TAILLES_ICO if t != 256])
    for t in (32, 64, 256):
        images[t].save(RACINE / "crates" / "app" / "ui" / "icones" / f"kadenz-{t}.png")
    print("Icônes régénérées :", ", ".join(f"{t} px" for t in TAILLES_ICO))


if __name__ == "__main__":
    main()
