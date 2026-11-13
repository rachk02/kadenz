"""Aperçu complet de la papeterie retenue (identité « Quatre pièces »), en grand format.

Reprend les scènes de scenes_papeterie.py (les mêmes que la planche 08), une par case, avec leur légende.
Usage : python docs/marque/planches/generer_apercu_papeterie.py <racine du dépôt>
Sorties : apercu-papeterie.html et images/apercu-papeterie.png, à côté de ce fichier.
"""

import importlib.util
import subprocess
from pathlib import Path

ICI = Path(__file__).resolve().parent
MARQUE = ICI.parent
CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"


def charger(nom, chemin):
    spec = importlib.util.spec_from_file_location(nom, chemin)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


gm = charger("generer_marque", MARQUE / "generer.py")
sc = charger("scenes_signaletique", ICI / "scenes_signaletique.py")
sp = charger("scenes_papeterie", ICI / "scenes_papeterie.py")
sc.gm = gm
sp.gm, sp.sc = gm, sc

PIECES = [
    (sp.carte, "Carte de visite", "85 x 55 mm. Recto Nuit : motif Tournant, logo blanc sur la partie calme. Verso : symbole, nom, fonction, trois temps, coordonnées ; Fondu très clair à droite.", "0 0 400 500"),
    (sp.papier, "Papier à en-tête", "A4. Bandeau Fondu très clair qui s'efface vers le logo, coordonnées en pied, aucune barre latérale. Pour l'éditeur seulement.", "0 0 400 500"),
    (sp.enveloppe, "Enveloppe", "DL 220 x 110 mm. Dehors : Fondu clair sous l'adresse de retour, zone du destinataire libre. Dedans : tout l'intérieur imprimé au Tournant.", "0 0 400 500"),
    (sp.chemise, "Chemise à rabats", "A4. Dehors comme le carnet : Tournant sur les trois quarts, bande Nuit avec le logo. Dedans : Tournant, rabat Indigo.", "0 0 400 500"),
    (sp.badge, "Badge et tour de cou", "90 x 120 mm. Haut Nuit au Fondu qui descend vers le logo, nom et fonction sur blanc, trois temps ; tour de cou Indigo.", "0 0 400 500"),
    (sp.carnet, "Carnet et autocollants", "Carnet A5 Nuit au Tournant, bande avec le logo, élastique ; autocollants de 50 mm, symbole et icône.", "0 0 400 500"),
    (sp.signatures, "Signatures de courriel", "Trois variantes : A bandeau Fondu avec le logo, B bloc Nuit au Fondu avec le texte en blanc, C fine bande Tournant.", "0 0 680 428"),
    (sp.tampons, "Tampons par degré", "Une forme par degré, une seule couleur d'encre : 1 entreprise, 2 direction, 3 responsable avec signature, 4 service avec date.", "0 0 680 428"),
]


def page():
    cartes = []
    for k, (fn, titre, texte, vb) in enumerate(PIECES):
        large = vb.endswith("428")
        w, h = (1072, 675) if large else (704, 880)
        cartes.append(f'<div class="piece{" large" if large else ""}">'
                      f'<svg viewBox="{vb}" width="{w}" height="{h}" style="display:block;border-radius:4px;background:#F4F5F9">'
                      + "".join(fn(f"ap{k}")) + f'</svg><h2>{titre}</h2><p>{texte}</p></div>')
    return f"""<!doctype html>
<html lang="fr"><head><meta charset="utf-8"><title>Papeterie Kadenz</title>
<style>
@font-face{{font-family:Inter;src:url(polices/Inter.woff2) format("woff2");font-weight:100 900}}
@font-face{{font-family:"Space Grotesk";src:url(polices/SpaceGrotesk.ttf) format("truetype");font-weight:300 700}}
*{{box-sizing:border-box;margin:0;font-style:normal}}
body{{background:#fff;color:#1F2933;font-family:Inter,sans-serif;width:2400px;padding:96px 112px}}
header{{display:flex;justify-content:space-between;align-items:baseline;margin-bottom:56px}}
h1{{font-family:"Space Grotesk";font-weight:700;font-size:64px;letter-spacing:-.01em}}
h1 span{{color:#2D3A8C;font-size:30px;margin-right:24px}}
.sous{{color:#52606D;font-size:24px;max-width:1000px;text-align:right}}
.grille{{display:grid;grid-template-columns:repeat(3,704px);justify-content:space-between;row-gap:64px}}
.duo{{display:grid;grid-template-columns:1072px 1072px;justify-content:space-between;margin-top:64px}}
h2{{font-family:"Space Grotesk";font-weight:700;font-size:30px;margin:22px 0 8px}}
p{{font-size:19px;line-height:1.5;color:#52606D;max-width:1000px}}
</style></head><body>
<header><h1><span>08</span>Papeterie : aperçu complet</h1>
<div class="sous">Papeterie de l'éditeur. Les documents produits par le logiciel gardent l'identité du client. Coordonnées à remplir.</div></header>
<div class="grille">{"".join(c for c in cartes if 'class="piece"' in c)}</div>
<div class="duo">{"".join(c for c in cartes if 'piece large' in c)}</div>
</body></html>
"""


if __name__ == "__main__":
    html = ICI / "apercu-papeterie.html"
    html.write_text(page(), encoding="utf-8")
    png = ICI / "images" / "apercu-papeterie.png"
    subprocess.run([CHROME, "--headless=new", "--hide-scrollbars", "--force-device-scale-factor=1",
                    "--allow-file-access-from-files", f"--screenshot={png}", "--window-size=2400,3200", html.as_uri()],
                   check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    print("ok")
