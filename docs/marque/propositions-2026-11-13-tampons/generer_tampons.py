"""Tampons par degré et récapitulatif des corrections (2026-11-13).

1. Avant / après des planches 05, 07 et 08 (anciennes images tirées de l'historique git, commit 86dcc17).
2. Tampons par degré, proposition : un tampon général pour l'entreprise, un par direction, un par
   responsable, un par service. Deux systèmes à comparer. Les noms restent entre crochets.

Usage : python generer_tampons.py <racine du dépôt>
Sorties : index.html et propositions-tampons.png, à côté de ce fichier.
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
sp = charger("scenes_papeterie", MARQUE / "planches" / "scenes_papeterie.py")
sc.gm = gm
sp.gm, sp.sc = gm, sc
img, imbrique, texte = sc.img, sc.imbrique, sc.texte
INDIGO, ARDOISE, TRAIT = sp.INDIGO, sp.ARDOISE, sp.TRAIT
ENCRE = f'fill="none" stroke="{INDIGO}"'
LARGE, HAUT = 2176, 820
COLONNES = (720, 1260, 1800)   # trois directions d'exemple
LIGNES = (120, 330, 520, 700)  # centres des quatre degrés


def t(x, y, s, taille=11, graisse=600, ancre="middle", esp=".04em"):
    return (f'<text x="{x}" y="{y}" font-family="Inter" font-size="{taille}" font-weight="{graisse}" fill="{INDIGO}" '
            f'text-anchor="{ancre}" letter-spacing="{esp}">{s}</text>')


def cercle_texte(cle, cx, cy, r, s, taille=10.5):
    rr = r - 16
    arc = f"M{cx - rr} {cy} A{rr} {rr} 0 1 1 {cx + rr} {cy} A{rr} {rr} 0 1 1 {cx - rr} {cy}"
    return (f'<defs><path id="{cle}" d="{arc}"/></defs>'
            f'<text font-family="Inter" font-size="{taille}" font-weight="600" fill="{INDIGO}" letter-spacing=".06em">'
            f'<textPath href="#{cle}" textLength="{2 * 3.1416 * rr - 6:.0f}" lengthAdjust="spacing">{s}</textPath></text>')


# --- système 1 : une forme par degré ------------------------------------------------------
def s1_general(cx, cy):
    return "".join(sp.tampon_rond("s1g", cx, cy))


def s1_direction(cx, cy, nom):
    w, h = 250, 96
    x, y = cx - w / 2, cy - h / 2
    fondu = sp.transparent(gm.motif_fondu("#FFFFFF", INDIGO, 90, h - 12, 12, 0.85))
    return (f'<rect x="{x}" y="{y}" width="{w}" height="{h}" {ENCRE} stroke-width="2.5"/>'
            + imbrique(fondu, x + 6, y + 6, 90, h - 12)
            + imbrique(sp.logo_encre(), x + 100, y + 12, 100, 27)
            + t(x + 102, y + 58, "DIRECTION", 9, 600, "start", ".12em")
            + t(x + 102, y + 78, nom, 13, 700, "start", "0"))


def s1_responsable(cx, cy):
    w, h = 270, 112
    x, y = cx - w / 2, cy - h / 2
    c = 76
    cadre = sp.transparent(gm.motif_tournant("#FFFFFF", INDIGO, c, c, 9.5))
    return (f'<rect x="{x}" y="{y}" width="{w}" height="{h}" {ENCRE} stroke-width="2.5"/>'
            + imbrique(cadre, x + 12, y + 18, c, c)
            + f'<rect x="{x + 21.5}" y="{y + 27.5}" width="{c - 19}" height="{c - 19}" fill="#FFFFFF"/>'
            + img("kadenz-symbole.svg", x + 30, y + 36, c - 36, c - 36)
            + t(x + 102, y + 30, "[Prénom Nom]", 13, 700, "start", "0")
            + t(x + 102, y + 47, "[Fonction]", 10, 500, "start", "0")
            + t(x + 102, y + 62, "[Direction]", 10, 500, "start", "0")
            + t(x + 102, y + 96, "Signature", 8.5, 600, "start", ".08em")
            + f'<path d="M{x + 152} {y + 96} L{x + w - 14} {y + 96}" {ENCRE} stroke-width="1" stroke-dasharray="2 3"/>')


def s1_service(cx, cy):
    w, h = 230, 70
    x, y = cx - w / 2, cy - h / 2
    return (f'<rect x="{x}" y="{y}" width="{w}" height="{h}" {ENCRE} stroke-width="2"/>'
            + img("kadenz-symbole.svg", x + 12, y + 15, 40, 40)
            + t(x + 64, y + 24, "[Service]", 11, 700, "start", "0")
            + t(x + 64, y + 44, "REÇU LE", 9, 600, "start", ".12em")
            + f'<rect x="{x + 118}" y="{y + 32}" width="{w - 132}" height="18" {ENCRE} stroke-width="1"/>'
            + t(x + 118 + (w - 132) / 2, y + 45, "[JJ / MM / AAAA]", 9, 500)
            + t(x + 64, y + 62, "PAR", 8, 600, "start", ".12em")
            + f'<path d="M{x + 88} {y + 62} L{x + w - 14} {y + 62}" {ENCRE} stroke-width="1" stroke-dasharray="2 3"/>')


# --- système 2 : tout en cachets ronds ----------------------------------------------------
def s2_general(cx, cy):
    return "".join(sp.tampon_rond("s2g", cx, cy))


def s2_direction(cx, cy, nom, k):
    r = 72
    return (f'<circle cx="{cx}" cy="{cy}" r="{r}" {ENCRE} stroke-width="3"/>'
            f'<circle cx="{cx}" cy="{cy}" r="{r - 28}" {ENCRE} stroke-width="1"/>'
            + cercle_texte(f"s2d{k}", cx, cy, r, f"DIRECTION  ·  {nom.upper()}  ·  KADENZ  ·", 10)
            + img("kadenz-symbole.svg", cx - 22, cy - 22, 44, 44))


def s2_responsable(cx, cy, k):
    r = 62
    return (f'<circle cx="{cx}" cy="{cy}" r="{r}" {ENCRE} stroke-width="2.5"/>'
            f'<circle cx="{cx}" cy="{cy}" r="{r - 26}" {ENCRE} stroke-width="1"/>'
            + cercle_texte(f"s2r{k}", cx, cy, r, "[PRÉNOM NOM]  ·  [FONCTION]  ·", 9.5)
            + img("kadenz-symbole.svg", cx - 13, cy - 24, 26, 26)
            + t(cx, cy + 18, "VISA", 8, 700, "middle", ".14em"))


def s2_service(cx, cy, k):
    r = 50
    return (f'<circle cx="{cx}" cy="{cy}" r="{r}" {ENCRE} stroke-width="2"/>'
            + cercle_texte(f"s2s{k}", cx, cy, r + 4, "[SERVICE]  ·  REÇU LE  ·", 8.5)
            + t(cx, cy - 2, "[JJ / MM]", 10, 700, "middle", "0")
            + t(cx, cy + 14, "[AAAA]", 9, 500, "middle", "0"))


# --- arbre ---------------------------------------------------------------------------------
def arbre(general, direction, responsable, service):
    out = [f'<rect width="{LARGE}" height="{HAUT}" fill="#FFFFFF"/>']
    for y, (titre, sous) in zip(LIGNES, (("Degré 1 · Entreprise", "Tampon général, un seul"),
                                         ("Degré 2 · Direction", "Un par direction"),
                                         ("Degré 3 · Responsable", "Un par responsable, nominatif"),
                                         ("Degré 4 · Service", "Un par service : réception, courrier"))):
        out.append(f'<text x="40" y="{y - 4}" font-family="Space Grotesk" font-size="22" font-weight="700" fill="#1F2933">{titre}</text>')
        out.append(f'<text x="40" y="{y + 20}" font-family="Inter" font-size="15" fill="{ARDOISE}">{sous}</text>')
    gx = COLONNES[1]
    trait = f'stroke="{TRAIT}" stroke-width="2" fill="none"'
    out.append(f'<path d="M{gx} {LIGNES[0] + 92} L{gx} {LIGNES[0] + 130} M{COLONNES[0]} {LIGNES[0] + 130} L{COLONNES[2]} {LIGNES[0] + 130}" {trait}/>')
    for x in COLONNES:
        out.append(f'<path d="M{x} {LIGNES[0] + 130} L{x} {LIGNES[1] - 80} M{x} {LIGNES[1] + 80} L{x} {LIGNES[2] - 66}'
                   f' M{x} {LIGNES[2] + 66} L{x} {LIGNES[3] - 52}" {trait}/>')
    encre = [general(gx, LIGNES[0])]
    for k, (x, nom) in enumerate(zip(COLONNES, ("[Direction 1]", "[Direction 2]", "[Direction 3]"))):
        encre += [direction(x, LIGNES[1], nom, k), responsable(x, LIGNES[2], k), service(x, LIGNES[3], k)]
    out.append(f'<g opacity=".88">{"".join(encre)}</g>')
    return (f'<svg viewBox="0 0 {LARGE} {HAUT}" width="{LARGE}" height="{HAUT}" style="display:block;border:1px solid #D9DCE3;border-radius:4px">'
            + "".join(out) + "</svg>")


CORRECTIONS = {
    "05-brand-pattern": ("Planche 05, motifs", [
        "Un seul motif : trois maintenant (Quatre pièces, Tournant, Fondu).",
        "Fondu : la zone calme du logo vient du motif lui-même.",
        "Règle 3 élargie aux rotations du Tournant.",
        "Marques de modules recentrées dans leur cadre."]),
    "07-signage": ("Planche 07, signalétique", [
        "Façade sommaire et porte grise : scènes complètes, silhouette de 1,75 m pour l'échelle.",
        "Drapeau sur mât, peu lisible : drapeau sur potence, perpendiculaire au mur.",
        "« [dimensions] » laissé vide : dimensions indicatives partout.",
        "Trois supports : huit (lettres boîtier, plaque, vitrophanie Fondu, totem, stand en plus).",
        "Kakémono : motif Fondu en tête et pied Ambre repris de l'ancienne identité."]),
    "08-stationery": ("Planche 08, papeterie", [
        "Pas de papier à en-tête : papier A4 au Fondu clair, sans barre latérale.",
        "Carte : recto Tournant, verso avec un Fondu clair qui s'efface avant le texte.",
        "Enveloppe : Fondu dehors, intérieur entièrement imprimé au Tournant.",
        "Une signature sans motif : trois variantes avec Fondu ou Tournant.",
        "Ajouts : chemise, trois tampons, carnet et autocollants ; badge refait."]),
}


def page():
    avant_apres = "".join(
        f'<section><h2>{titre}</h2><div class="duo"><figure><img src="avant/{cle}.png" alt=""><figcaption>Avant (planche du dépôt au commit 86dcc17)</figcaption></figure>'
        f'<figure><img src="../planches/images/{cle}.png" alt=""><figcaption>Après</figcaption></figure>'
        f'<ul>{"".join(f"<li>{c}</li>" for c in liste)}</ul></div></section>'
        for cle, (titre, liste) in CORRECTIONS.items())
    s1 = arbre(s1_general, lambda x, y, n, k: s1_direction(x, y, n), lambda x, y, k: s1_responsable(x, y), lambda x, y, k: s1_service(x, y))
    s2 = arbre(s2_general, s2_direction, s2_responsable, s2_service)
    return f"""<!doctype html>
<html lang="fr"><head><meta charset="utf-8"><title>Tampons par degré</title>
<style>
@font-face{{font-family:Inter;src:url(../planches/polices/Inter.woff2) format("woff2");font-weight:100 900}}
@font-face{{font-family:"Space Grotesk";src:url(../planches/polices/SpaceGrotesk.ttf) format("truetype");font-weight:300 700}}
*{{box-sizing:border-box;margin:0;font-style:normal}}
body{{background:#fff;color:#1F2933;font-family:Inter,sans-serif;width:2400px;padding:96px 112px}}
h1{{font-family:"Space Grotesk";font-weight:700;font-size:64px;letter-spacing:-.01em;margin-bottom:12px}}
h1 span{{color:#2D3A8C;font-size:30px;margin-right:24px}}
.intro{{font-size:22px;color:#52606D;line-height:1.5;margin-bottom:48px;max-width:2150px}}
h2{{font-family:"Space Grotesk";font-weight:700;font-size:34px;margin:0 0 18px}}
h3{{font-family:"Space Grotesk";font-weight:700;font-size:28px;margin:44px 0 6px}}
section{{margin-bottom:56px}}
.duo{{display:grid;grid-template-columns:820px 820px 1fr;gap:32px;align-items:start}}
figure img{{width:820px;display:block;border:1px solid #D9DCE3;border-radius:4px}}
figcaption{{font-size:16px;color:#52606D;margin-top:8px}}
ul{{font-size:19px;line-height:1.55;color:#1F2933;padding-left:22px}}
li{{margin-bottom:10px}}
p{{font-size:19px;line-height:1.5;color:#52606D;margin-bottom:16px;max-width:2100px}}
.sep{{border-top:1px solid #D9DCE3;margin:24px 0 48px}}
</style></head><body>
<h1><span>05 07 08</span>Ce qui a été corrigé</h1>
<p class="intro">Chaque planche avant et après la reprise du 2026-11-13, avec la liste des corrections.</p>
{avant_apres}
<div class="sep"></div>
<h1><span>08</span>Tampons par degré : propositions</h1>
<p class="intro">Un tampon général pour l'entreprise, puis un tampon par direction, par responsable et par service. Toujours une seule couleur d'encre. Les noms restent entre crochets ; trois directions servent d'exemple. Proposition : rien n'est intégré à l'identité.</p>
<h3>Système 1 · une forme par degré</h3>
<p>Cachet rond pour l'entreprise ; rectangle Fondu au nom de la direction ; rectangle nominatif avec cadre Tournant et ligne de signature pour le responsable ; dateur « Reçu le » pour le service. Le degré se reconnaît à la forme, de loin.</p>
{s1}
<h3>Système 2 · tout en cachets ronds</h3>
<p>Même forme à tous les degrés : le diamètre baisse avec le degré, deux cercles pour l'entreprise, la direction et le responsable, un seul pour le service. Plus homogène, mais les degrés se distinguent moins vite et le responsable n'a pas de ligne de signature.</p>
{s2}
</body></html>
"""


if __name__ == "__main__":
    html = ICI / "index.html"
    html.write_text(page(), encoding="utf-8")
    png = ICI / "propositions-tampons.png"
    subprocess.run([CHROME, "--headless=new", "--hide-scrollbars", "--force-device-scale-factor=1",
                    "--allow-file-access-from-files", f"--screenshot={png}", "--window-size=2400,4480", html.as_uri()],
                   check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    print("ok")
