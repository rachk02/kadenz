"""Aperçu complet de l'identité « Quatre pièces » : les neuf planches sur une seule page.

Reprend les PNG produits par generer_planches.py (à lancer d'abord).
Usage : python docs/marque/planches/generer_apercu_identite.py
Sorties : apercu-identite.html et images/apercu-identite.png, à côté de ce fichier.
"""

import subprocess
from pathlib import Path

ICI = Path(__file__).resolve().parent
CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"

PLANCHES = [
    ("01-primary-logo", "Logo principal"),
    ("02-logomark", "Symbole"),
    ("03-tone-mood", "Ton"),
    ("04-color-system", "Couleurs"),
    ("05-brand-pattern", "Motifs"),
    ("06-typography", "Typographie"),
    ("07-signage", "Signalétique"),
    ("08-stationery", "Papeterie"),
    ("09-brand-mockups", "Mises en situation"),
]


def page():
    cases = "".join(
        f'<figure><img src="images/{f}.png" width="720" height="456" alt="{t}">'
        f'<figcaption><span>{f[:2]}</span>{t}</figcaption></figure>'
        for f, t in PLANCHES)
    return f"""<!doctype html>
<html lang="fr"><head><meta charset="utf-8"><title>Identité Kadenz</title>
<style>
@font-face{{font-family:Inter;src:url(polices/Inter.woff2) format("woff2");font-weight:100 900}}
@font-face{{font-family:"Space Grotesk";src:url(polices/SpaceGrotesk.ttf) format("truetype");font-weight:300 700}}
*{{box-sizing:border-box;margin:0;font-style:normal}}
body{{background:#F4F5F9;color:#1F2933;font-family:Inter,sans-serif;width:2400px;padding:88px 72px}}
header{{display:flex;justify-content:space-between;align-items:center;margin-bottom:56px}}
header img{{height:72px;display:block}}
h1{{font-family:"Space Grotesk";font-weight:700;font-size:56px;letter-spacing:-.01em}}
.sous{{color:#52606D;font-size:22px;text-align:right;margin-top:8px}}
.grille{{display:grid;grid-template-columns:repeat(3,720px);justify-content:space-between;row-gap:48px}}
figure img{{display:block;border-radius:4px;border:1px solid #D9DCE3;background:#fff}}
figcaption{{font-family:"Space Grotesk";font-weight:700;font-size:24px;margin-top:16px}}
figcaption span{{color:#2D3A8C;margin-right:14px}}
</style></head><body>
<header><img src="../kadenz-logo-horizontal.svg" alt="Kadenz">
<div><h1>Identité « Quatre pièces »</h1><p class="sous">Aperçu complet des neuf planches</p></div></header>
<div class="grille">{cases}</div>
</body></html>
"""


if __name__ == "__main__":
    html = ICI / "apercu-identite.html"
    html.write_text(page(), encoding="utf-8")
    png = ICI / "images" / "apercu-identite.png"
    subprocess.run([CHROME, "--headless=new", "--hide-scrollbars", "--force-device-scale-factor=1",
                    "--allow-file-access-from-files", f"--screenshot={png}", "--window-size=2400,1950", html.as_uri()],
                   check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    print("ok")
