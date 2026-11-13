"""Propositions de papeterie pour l'identité « Quatre pièces » (2026-11-13).

Proposition seulement : rien n'est intégré à la planche 08 tant que l'utilisateur n'a pas choisi.
Papeterie de l'éditeur : les documents produits par le logiciel gardent l'identité du client.
Les coordonnées restent entre crochets : aucune n'est inventée.

Usage : python generer_papeterie.py <racine du dépôt>
Sorties, à côté de ce fichier : index.html (planche) et propositions-papeterie.png (rendu Chrome).
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
sc = charger("scenes_signaletique", MARQUE / "planches" / "scenes_signaletique.py")
sc.gm = gm
img, imbrique, rect, texte = sc.img, sc.imbrique, sc.rect, sc.texte

NUIT, PIECES, INDIGO, AMBRE = "#1A2259", "#232C6B", "#2D3A8C", "#E3A23B"
BRUME, BLANC, TRAIT, ARDOISE, ENCRE = "#F4F5F9", "#FFFFFF", "#D9DCE3", "#52606D", "#1F2933"
CLAIR_NUIT, PIECE_CLAIRE = "#C5CAE0", "#ECEEF4"
W, H = 400, 500


def ombre(i):
    return (f'<defs><filter id="o{i}" x="-20%" y="-20%" width="140%" height="140%">'
            f'<feDropShadow dx="0" dy="4" stdDeviation="6" flood-color="#0F172A" flood-opacity=".14"/></filter></defs>')


def trois_temps(x, y, l=18, e=2.5, g=5, clair=BLANC):
    return rect(x, y, l, e, clair) + rect(x + l + g, y, l, e, clair) + rect(x + 2 * (l + g), y, l, e, AMBRE)


def miroir(s, x, y, w, h):
    """Un motif retourné de gauche à droite : le Fondu s'efface alors vers la gauche."""
    return f'<g transform="translate({x + w} {y}) scale(-1 1)">{imbrique(s, 0, 0, w, h)}</g>'


def lignes(x, y, largeurs, pas=9, couleur="#E3E6EC", e=3):
    return "".join(rect(x, y + i * pas, l, e, couleur) for i, l in enumerate(largeurs))


def verso_carte(i, x, y, w, h, motif=""):
    out = [rect(x, y, w, h, BLANC, 3, f'filter="url(#o{i})"'), motif]
    out.append(img("kadenz-symbole.svg", x + 20, y + 18, 26, 26))
    out.append(texte(x + 20, y + 74, "[Prénom Nom]", 17, ENCRE, 700, "start", "Space Grotesk"))
    out.append(texte(x + 20, y + 92, "[Fonction]", 11, ARDOISE, 400, "start"))
    out.append(trois_temps(x + 20, y + 106, 14, 2.5, 4, INDIGO))
    for k, t in enumerate(("[Téléphone]", "[E-mail]", "[Site]")):
        out.append(texte(x + 20, y + 140 + k * 16, t, 10.5, ENCRE, 400, "start"))
    return out


# --- les propositions -------------------------------------------------------------------
def carte_tournant(i):
    x, y, w, h = 40, 50, 320, 207
    out = [ombre(i), rect(x, y, w, h, NUIT, 3, f'filter="url(#o{i})"')]
    out.append(imbrique(gm.motif_tournant(NUIT, PIECES, 140, h, 34.5), x, y, 140, h))
    out.append(img("kadenz-logo-horizontal-negatif.svg", x + 158, y + 82, 146, 42))
    out += verso_carte(i, x, 275, w, h)
    return out


def carte_fondu(i):
    x, y, w, h = 40, 50, 320, 207
    out = [ombre(i), rect(x, y, w, h, NUIT, 3, f'filter="url(#o{i})"')]
    out.append(imbrique(gm.motif_fondu(NUIT, PIECES, w, h, 23, 0.55), x, y, w, h))
    out.append(img("kadenz-logo-horizontal-negatif.svg", x + 150, y + 82, 150, 42))
    verso_motif = miroir(gm.motif_fondu(BLANC, PIECE_CLAIRE, 150, 207, 23, 0.8).replace("<rect", '<rect opacity="0"', 1),
                         x + 170, 275, 150, 207)
    out += verso_carte(i, x, 275, w, h, verso_motif)
    return out


def papier(i):
    x, y, w, h = 70, 56, 260, 368
    out = [ombre(i), rect(x, y, w, h, BLANC, 2, f'filter="url(#o{i})"')]
    out.append(miroir(gm.motif_fondu(BLANC, PIECE_CLAIRE, w, 52, 13, 0.6).replace("<rect", '<rect opacity="0"', 1), x, y, w, 52))
    out.append(img("kadenz-logo-horizontal.svg", x + 20, y + 16, 92, 24))
    out.append(lignes(x + 24, y + 86, (70, 96)))
    out.append(lignes(x + 24, y + 124, (212, 212, 212, 150), 8))
    out.append(lignes(x + 24, y + 166, (212, 212, 120), 8))
    out.append(rect(x + 24, y + h - 34, w - 48, 1, TRAIT))
    out.append(texte(x + w / 2, y + h - 20, "[Adresse]  ·  [Téléphone]  ·  [E-mail]  ·  [Site]", 7, ARDOISE, 400))
    return out


def enveloppe(i):
    x, w, h = 40, 320, 160
    out = [ombre(i)]
    y = 46
    out.append(rect(x, y, w, h, BLANC, 2, f'filter="url(#o{i})"'))
    out.append(img("kadenz-logo-horizontal.svg", x + 16, y + 14, 84, 22))
    out.append(texte(x + 16, y + 50, "[Adresse de retour]", 8, ARDOISE, 400, "start"))
    out.append(lignes(x + 160, y + 92, (110, 130, 90), 9))
    y2 = 300
    out.append(f'<defs><clipPath id="r{i}"><path d="M{x} {y2} L{x + w} {y2} L{x + w / 2} {y2 - 92} Z"/></clipPath></defs>')
    out.append(rect(x, y2, w, h, BLANC, 2, f'filter="url(#o{i})"'))
    out.append(f'<path d="M{x} {y2} L{x + w} {y2} L{x + w / 2} {y2 - 92} Z" fill="{NUIT}"/>')
    out.append(f'<g clip-path="url(#r{i})">{imbrique(gm.motif_tournant(NUIT, PIECES, w, 92, 23), x, y2 - 92, w, 92)}</g>')
    out.append(f'<path d="M{x} {y2 + h} L{x + w / 2} {y2 + 70} L{x + w} {y2 + h}" fill="none" stroke="{TRAIT}"/>')
    out.append(texte(x + w / 2, y2 + h + 22, "rabat ouvert : intérieur imprimé", 10, ARDOISE, 400))
    return out


def chemise(i):
    x, y, w, h = 80, 56, 240, 340
    out = [ombre(i), rect(x + 12, y + 8, w, h, INDIGO, 2), rect(x, y, w, h, NUIT, 2, f'filter="url(#o{i})"')]
    out.append(imbrique(gm.motif_tournant(NUIT, PIECES, w, 190, 30), x, y, w, 190))
    out.append(img("kadenz-logo-horizontal-negatif.svg", x + 24, y + 238, 130, 36))
    out.append(trois_temps(x + 26, y + 296, 22, 3, 6))
    return out


def badge(i):
    x, y, w, h = 110, 150, 180, 240
    out = [ombre(i)]
    out.append(f'<path d="M150 0 L186 {y - 8} M250 0 L214 {y - 8}" stroke="{INDIGO}" stroke-width="12" fill="none"/>')
    out.append(rect(188, y - 18, 24, 26, "#9AA3B5", 3))
    out.append(rect(x, y, w, h, BLANC, 4, f'filter="url(#o{i})"'))
    out.append(rect(x, y, w, 120, NUIT, 4) + rect(x, y + 110, w, 10, NUIT))
    out.append(imbrique(gm.motif_fondu(NUIT, PIECES, w, 120, 20, 0.75, "bas"), x, y, w, 120))
    out.append(img("kadenz-logo-horizontal-negatif.svg", x + 30, y + 76, 120, 32))
    out.append(texte(x + w / 2, y + 160, "[Prénom Nom]", 17, ENCRE, 700, police="Space Grotesk"))
    out.append(texte(x + w / 2, y + 180, "[Fonction]", 11, ARDOISE, 400))
    out.append(trois_temps(x + w / 2 - 27, y + 200, 14, 2.5, 6, INDIGO))
    return out


def signature(i):
    x, y, w, h = 30, 90, 340, 300
    out = [ombre(i), rect(x, y, w, h, BLANC, 3, f'filter="url(#o{i})"')]
    out.append(rect(x, y, w, 30, "#EEF0F5", 3) + rect(x, y + 27, w, 3, "#EEF0F5"))
    for k, t in enumerate(("De", "Objet")):
        out.append(texte(x + 16, y + 52 + k * 18, t, 9, ARDOISE, 500, "start"))
        out.append(rect(x + 56, y + 46 + k * 18, 150 - k * 40, 4, "#E3E6EC"))
    out.append(lignes(x + 16, y + 100, (300, 300, 220), 10))
    sy = y + 170
    out.append(rect(x + 16, sy - 12, w - 32, 1, TRAIT))
    out.append(img("kadenz-icone.svg", x + 16, sy + 4, 48, 48))
    out.append(texte(x + 76, sy + 18, "[Prénom Nom]", 14, ENCRE, 700, "start", "Space Grotesk"))
    out.append(texte(x + 76, sy + 34, "[Fonction] · Kadenz", 10, ARDOISE, 400, "start"))
    out.append(trois_temps(x + 76, sy + 44, 12, 2.5, 4, INDIGO))
    out.append(texte(x + 76, sy + 66, "[Téléphone] · [Site]", 10, ENCRE, 400, "start"))
    return out


def logo_encre():
    """Logo horizontal d'une seule couleur, Indigo : l'encre du tampon."""
    p, _ = gm.pieces()
    return gm.horizontal(gm.symbole(p, INDIGO, INDIGO), INDIGO).replace('role="img" aria-label="Kadenz"><title>Kadenz</title>', 'aria-hidden="true">')


def tampon(i):
    out = [ombre(i)]
    out.append(f'<g transform="rotate(-4 200 300)">{rect(60, 150, 280, 300, BLANC, 2, f'filter="url(#o{i})"')}'
               f'{lignes(84, 180, (200, 230, 230, 160), 10)}'
               f'<g opacity=".82"><rect x="104" y="300" width="192" height="84" fill="none" stroke="{INDIGO}" stroke-width="2.5"/>'
               f'{imbrique(logo_encre(), 118, 312, 110, 30)}'
               f'{texte(118, 360, "[Adresse]", 9, INDIGO, 500, "start")}{texte(118, 374, "[Téléphone]", 9, INDIGO, 500, "start")}</g></g>')
    out.append(rect(150, 40, 100, 34, "#3A4150", 4) + rect(185, 74, 30, 40, "#5B6475", 2) + rect(140, 114, 120, 18, "#3A4150", 2))
    return out


def carnet(i):
    x, y, w, h = 60, 60, 200, 290
    out = [ombre(i), rect(x, y, w, h, NUIT, 4, f'filter="url(#o{i})"')]
    out.append(imbrique(gm.motif_tournant(NUIT, PIECES, w, 210, 25), x, y, w, 210))
    out.append(img("kadenz-logo-horizontal-negatif.svg", x + 20, y + 236, 110, 32))
    out.append(rect(x + w - 26, y, 8, h, "#11173D"))
    out.append(f'<circle cx="320" cy="380" r="52" fill="{BLANC}" filter="url(#o{i})"/>')
    out.append(img("kadenz-symbole.svg", 290, 350, 60, 60))
    out.append(f'<g filter="url(#o{i})">{img("kadenz-icone.svg", 266, 196, 92, 92)}</g>')
    return out


PROPOSITIONS = [
    ("Carte de visite Tournant", carte_tournant,
     "Recto Nuit : motif Tournant à gauche, logo blanc sur la partie calme ; verso blanc nominatif avec les trois temps.",
     "85 x 55 mm."),
    ("Carte de visite Fondu", carte_fondu,
     "Recto Nuit au motif Fondu, logo sur la zone calme ; au verso, le Fondu ton sur ton s'efface vers le texte.",
     "85 x 55 mm."),
    ("Papier à en-tête", papier,
     "Bandeau Fondu très clair qui s'efface vers le logo ; coordonnées en pied ; aucune barre latérale.",
     "A4 ; pour l'éditeur seulement, jamais pour les documents des clients."),
    ("Enveloppe à intérieur imprimé", enveloppe,
     "Recto sobre (logo, adresse de retour) ; à l'ouverture, l'intérieur du rabat est Nuit au motif Tournant.",
     "DL 220 x 110 mm."),
    ("Chemise à rabats", chemise,
     "Chemise de présentation Nuit : Tournant en tête, logo blanc et trois temps en bas, intérieur Indigo.",
     "A4, deux rabats."),
    ("Badge et tour de cou", badge,
     "Haut Nuit au Fondu qui descend vers le logo, nom en bas sur blanc ; tour de cou Indigo.",
     "90 x 120 mm."),
    ("Signature de courriel", signature,
     "Icône sur tuile, nom, fonction et trois temps, en HTML léger, lisible sans les images.",
     "Largeur 340 px environ."),
    ("Tampon", tampon,
     "Le logo dans un cadre, en une seule couleur d'encre, pour les bons et courriers de l'éditeur.",
     "Environ 50 x 22 mm, encre Indigo ou noire."),
    ("Carnet et autocollants", carnet,
     "Carnet Nuit au motif Tournant, élastique Nuit ; autocollants du symbole et de l'icône.",
     "Carnet A5, autocollants de 50 mm."),
]


def page(cartes):
    return f"""<!doctype html>
<html lang="fr"><head><meta charset="utf-8"><title>Papeterie : propositions</title>
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
<header><h1><span>08</span>Papeterie : propositions</h1>
<div class="sous">Quatre pièces, 2026-11-13. Proposition : rien n'est intégré à l'identité.</div></header>
<p class="constat"><b>Constat.</b> L'ancienne planche avait un papier à en-tête A4, une enveloppe au liseré Trois temps et une touche d'Ambre ; la nouvelle n'a plus de papier à en-tête, plus d'Ambre, et seulement le motif Quatre pièces.
Voici neuf pièces avec les motifs retenus (Tournant, Fondu) et les trois temps. Papeterie de l'éditeur seulement ; les coordonnées restent entre crochets.</p>
<div class="grille">{"".join(cartes)}</div>
</body></html>
"""


if __name__ == "__main__":
    gm.D_MOT, gm.L_MOT = gm.mot()
    cartes = [f'<div><div class="panneau"><img src="../planches/images/08-stationery.png" alt=""></div>'
              f'<div class="num">Référence</div><h2>Planche 08 actuelle</h2>'
              f'<p>Carte de visite, enveloppe, signature, badge, pour comparer.</p></div>']
    for i, (nom, fn, idee, dim) in enumerate(PROPOSITIONS, 1):
        dessin = (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}" aria-hidden="true">'
                  + "".join(fn(i)) + "</svg>")
        cartes.append(f'<div><div class="panneau">{dessin}</div><div class="num">Proposition {i}</div>'
                      f'<h2>{nom}</h2><p>{idee}</p><p class="dim">{dim}</p></div>')
    html = ICI / "index.html"
    html.write_text(page(cartes), encoding="utf-8")
    png = ICI / "propositions-papeterie.png"
    subprocess.run([CHROME, "--headless=new", "--hide-scrollbars", "--force-device-scale-factor=1",
                    "--allow-file-access-from-files", f"--screenshot={png}", "--window-size=2400,1960", html.as_uri()],
                   check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    print("ok", len(PROPOSITIONS))
