"""Papeterie, deuxième tour (2026-11-13), après les choix de l'utilisateur sur le premier tour.

Retenus tels quels : papier à en-tête (3), badge (6), carnet et autocollants (9).
Ici, pour validation : la carte de visite fusionnée (recto de la 2, verso de la 1), l'enveloppe revue
(Fondu dehors, intérieur entièrement imprimé) ; et des variantes pour la chemise (sans les trois
temps, Tournant et Fondu mêlés), la signature de courriel (avec Fondu ou Tournant) et le tampon.

Usage : python generer_papeterie_tour2.py <racine du dépôt>
Sorties : tour2.html et propositions-papeterie-tour2.png, à côté de ce fichier.
"""

import importlib.util
import subprocess
from pathlib import Path

ICI = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("papeterie_tour1", ICI / "generer_papeterie.py")
t1 = importlib.util.module_from_spec(spec)
spec.loader.exec_module(t1)

gm, img, imbrique, rect, texte = t1.gm, t1.img, t1.imbrique, t1.rect, t1.texte
ombre, trois_temps, miroir, lignes = t1.ombre, t1.trois_temps, t1.miroir, t1.lignes
NUIT, PIECES, INDIGO, AMBRE, BLANC = t1.NUIT, t1.PIECES, t1.INDIGO, t1.AMBRE, t1.BLANC
TRAIT, ARDOISE, ENCRE, BRUME = t1.TRAIT, t1.ARDOISE, t1.ENCRE, t1.BRUME
W, H = t1.W, t1.H
PIECE_ENV = "#E4E7F0"   # pièces du Fondu sur une enveloppe blanche, visibles à l'impression


def tournant_fondu(fond, trace, w, h, s, portee=0.7, sens="bas", graine=7):
    """Tournant dont les quarts de disque rapetissent jusqu'à disparaître (vers le bas ou en diagonale)."""
    import math
    import random
    alea = random.Random(graine)
    f = gm.f
    out = [f'<rect width="{f(w)}" height="{f(h)}" fill="{fond}"/>']
    for r in range(math.ceil(h / s)):
        for c in range(math.ceil(w / s)):
            rot = 90 * alea.randrange(4)
            x, y = c * s, r * s
            pos = y / h if sens == "bas" else (x / w + y / h) / 2
            e = 1 - pos / portee
            if e <= 0.1:
                continue
            k = s * e
            x0, y0 = x + (s - k) / 2, y + (s - k) / 2
            out.append(f'<path fill="{trace}" transform="rotate({rot} {f(x + s / 2)} {f(y + s / 2)})"'
                       f' d="M{f(x0)} {f(y0)} L{f(x0 + k)} {f(y0)} A{f(k)} {f(k)} 0 0 1 {f(x0)} {f(y0 + k)} Z"/>')
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {f(w)} {f(h)}" aria-hidden="true">'
            + "".join(out) + "</svg>\n")


def transparent(s):
    return s.replace("<rect", '<rect opacity="0"', 1)


# --- pour validation ----------------------------------------------------------------------
def carte(i):
    x, y, w, h = 40, 50, 320, 207
    out = [ombre(i), rect(x, y, w, h, NUIT, 3, f'filter="url(#o{i})"')]
    out.append(imbrique(gm.motif_fondu(NUIT, PIECES, w, h, 23, 0.55), x, y, w, h))
    out.append(img("kadenz-logo-horizontal-negatif.svg", x + 150, y + 82, 150, 42))
    out += t1.verso_carte(i, x, 275, w, h)
    return out


def enveloppe(i):
    out = [ombre(i)]
    x, y, w, h = 40, 34, 320, 160
    out.append(rect(x, y, w, h, BLANC, 2, f'filter="url(#o{i})"'))
    out.append(imbrique(transparent(gm.motif_fondu(BLANC, PIECE_ENV, 150, 96, 16, 0.75)), x, y + 64, 150, 96))
    out.append(img("kadenz-logo-horizontal.svg", x + 16, y + 14, 84, 22))
    out.append(texte(x + 16, y + 50, "[Adresse de retour]", 8, ARDOISE, 400, "start"))
    out.append(lignes(x + 170, y + 100, (110, 130, 90), 9))
    # patron à plat, face intérieure : tout est imprimé
    bx, by, bw, bh = 110, 300, 180, 90
    contour = (f"M{bx} {by} L{bx + bw / 2} {by - 58} L{bx + bw} {by} L{bx + bw + 46} {by + 12} L{bx + bw + 46} {by + bh - 12}"
               f" L{bx + bw} {by + bh} L{bx + bw - 12} {by + bh + 64} L{bx + 12} {by + bh + 64} L{bx} {by + bh}"
               f" L{bx - 46} {by + bh - 12} L{bx - 46} {by + 12} Z")
    out.append(f'<defs><clipPath id="p{i}"><path d="{contour}"/></clipPath></defs>')
    out.append(f'<path d="{contour}" fill="{NUIT}" filter="url(#o{i})"/>')
    out.append(f'<g clip-path="url(#p{i})">{imbrique(gm.motif_tournant(NUIT, PIECES, 272, 212, 22.7), bx - 46, by - 58, 272, 212)}</g>')
    out.append(f'<rect x="{bx}" y="{by}" width="{bw}" height="{bh}" fill="none" stroke="#3A4580" stroke-dasharray="4 3"/>')
    out.append(texte(200, 478, "patron à plat, face intérieure", 10, ARDOISE, 400))
    return out


# --- chemise : trois variantes sans les trois temps --------------------------------------
def chemise_degrade(i):
    x, y, w, h = 80, 56, 240, 340
    out = [ombre(i), rect(x + 12, y + 8, w, h, INDIGO, 2), rect(x, y, w, h, NUIT, 2, f'filter="url(#o{i})"')]
    out.append(imbrique(tournant_fondu(NUIT, PIECES, w, h, 30, 0.72, "bas"), x, y, w, h))
    out.append(img("kadenz-logo-horizontal-negatif.svg", x + 24, y + 270, 140, 40))
    return out


def chemise_diagonale(i):
    x, y, w, h = 80, 56, 240, 340
    out = [ombre(i), rect(x + 12, y + 8, w, h, INDIGO, 2), rect(x, y, w, h, NUIT, 2, f'filter="url(#o{i})"')]
    out.append(imbrique(tournant_fondu(NUIT, PIECES, w, h, 30, 0.8, "diagonale"), x, y, w, h))
    out.append(img("kadenz-logo-horizontal-negatif.svg", x + 84, y + 278, 136, 38))
    return out


def chemise_dedans_dehors(i):
    out = [ombre(i)]
    x1, x2, y, w, h = 24, 206, 110, 170, 240
    out.append(rect(x1, y, w, h, NUIT, 2, f'filter="url(#o{i})"'))
    out.append(imbrique(gm.motif_fondu(NUIT, PIECES, w, h, 21.25, 0.7, "bas"), x1, y, w, h))
    out.append(img("kadenz-logo-horizontal-negatif.svg", x1 + 18, y + 190, 110, 30))
    out.append(rect(x2, y, w, h, NUIT, 2, f'filter="url(#o{i})"'))
    out.append(imbrique(gm.motif_tournant(NUIT, PIECES, w, h, 28.3), x2, y, w, h))
    out.append(rect(x2, y + 160, w, 80, INDIGO, 2) + rect(x2 + 50, y + 168, 70, 3, "#20296A"))
    out.append(texte(x1 + w / 2, y + h + 26, "dehors : Fondu", 11, ARDOISE, 500))
    out.append(texte(x2 + w / 2, y + h + 26, "dedans : Tournant, rabat Indigo", 11, ARDOISE, 500))
    return out


# --- signature : trois variantes ---------------------------------------------------------
def courriel(i, signature):
    x, y, w, h = 30, 70, 340, 360
    out = [ombre(i), rect(x, y, w, h, BLANC, 3, f'filter="url(#o{i})"')]
    out.append(rect(x, y, w, 30, "#EEF0F5", 3) + rect(x, y + 27, w, 3, "#EEF0F5"))
    for k, t in enumerate(("De", "Objet")):
        out.append(texte(x + 16, y + 52 + k * 18, t, 9, ARDOISE, 500, "start"))
        out.append(rect(x + 56, y + 46 + k * 18, 150 - k * 40, 4, "#E3E6EC"))
    out.append(lignes(x + 16, y + 100, (300, 300, 220), 10))
    out.append(rect(x + 16, y + 150, w - 32, 1, TRAIT))
    out += signature(x + 16, y + 166, w - 32)
    return out


def bloc_nom(x, y, couleur=ENCRE, second=ARDOISE, temps=INDIGO):
    return [texte(x, y + 14, "[Prénom Nom]", 14, couleur, 700, "start", "Space Grotesk"),
            texte(x, y + 30, "[Fonction] · Kadenz", 10, second, 400, "start"),
            trois_temps(x, y + 40, 12, 2.5, 4, temps),
            texte(x, y + 62, "[Téléphone] · [Site]", 10, couleur, 400, "start")]


def signature_bandeau_fondu(i):
    def sig(x, y, w):
        out = [img("kadenz-icone.svg", x, y, 48, 48)] + bloc_nom(x + 60, y)
        out.append(rect(x, y + 86, w, 52, NUIT, 2))
        out.append(imbrique(gm.motif_fondu(NUIT, PIECES, w, 52, 13, 0.6), x, y + 86, w, 52))
        out.append(img("kadenz-logo-horizontal-negatif.svg", x + w - 112, y + 98, 100, 28))
        return out
    return courriel(i, sig)


def signature_carte_fondu(i):
    def sig(x, y, w):
        out = [rect(x, y, w, 120, NUIT, 3)]
        out.append(imbrique(gm.motif_fondu(NUIT, PIECES, w, 120, 20, 0.36), x, y, w, 120))
        out += bloc_nom(x + 128, y + 22, BLANC, "#C5CAE0", BLANC)
        return out
    return courriel(i, sig)


def signature_bande_tournant(i):
    def sig(x, y, w):
        out = [img("kadenz-icone.svg", x, y, 48, 48)] + bloc_nom(x + 60, y)
        out.append(rect(x, y + 86, w, 34, NUIT, 2))
        out.append(imbrique(gm.motif_tournant(NUIT, PIECES, w, 34, 17), x, y + 86, w, 34))
        return out
    return courriel(i, sig)


# --- tampon : trois variantes ------------------------------------------------------------
def papier_tampon(i, empreinte):
    out = [ombre(i)]
    out.append(f'<g transform="rotate(-3 200 260)">{rect(50, 60, 300, 400, BLANC, 2, f"filter=" + chr(34) + f"url(#o{i})" + chr(34))}'
               f'{lignes(76, 92, (210, 248, 248, 170), 10)}<g opacity=".85">{"".join(empreinte)}</g></g>')
    return out


def tampon_rond(i):
    cx, cy, r = 200, 280, 82
    arc = f"M{cx - r + 16} {cy} A{r - 16} {r - 16} 0 1 1 {cx + r - 16} {cy} A{r - 16} {r - 16} 0 1 1 {cx - r + 16} {cy}"
    e = [f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="none" stroke="{INDIGO}" stroke-width="3"/>',
         f'<circle cx="{cx}" cy="{cy}" r="{r - 30}" fill="none" stroke="{INDIGO}" stroke-width="1.5"/>',
         f'<defs><path id="c{i}" d="{arc}"/></defs>',
         f'<text font-family="Inter" font-size="10.5" font-weight="600" fill="{INDIGO}" letter-spacing=".06em">'
         f'<textPath href="#c{i}" textLength="{2 * 3.1416 * (r - 16) - 6:.0f}" lengthAdjust="spacing">KADENZ  ·  [ADRESSE]  ·  [TÉLÉPHONE]  ·  [SITE]  ·</textPath></text>',
         img("kadenz-symbole.svg", cx - 30, cy - 30, 60, 60)]
    return papier_tampon(i, e)


def tampon_fondu(i):
    x, y, w, h = 86, 236, 228, 96
    fondu = transparent(gm.motif_fondu(BLANC, INDIGO, 100, h - 12, 12, 0.85))
    e = [f'<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="none" stroke="{INDIGO}" stroke-width="2.5"/>',
         imbrique(fondu, x + 6, y + 6, 100, h - 12),
         imbrique(t1.logo_encre(), x + 104, y + 18, 112, 30),
         texte(x + 106, y + 66, "[Adresse]", 9, INDIGO, 500, "start"),
         texte(x + 106, y + 80, "[Téléphone]", 9, INDIGO, 500, "start")]
    return papier_tampon(i, e)


def tampon_cadre_tournant(i):
    x, y, c = 125, 200, 150
    tour = gm.motif_tournant(BLANC, INDIGO, c, c, 15).replace("<rect", '<rect opacity="0"', 1)
    p, _ = gm.pieces()
    empile = gm.empile(gm.symbole(p, INDIGO, INDIGO), INDIGO).replace('role="img" aria-label="Kadenz"><title>Kadenz</title>', 'aria-hidden="true">')
    e = [imbrique(tour, x, y, c, c),
         f'<rect x="{x + 15}" y="{y + 15}" width="{c - 30}" height="{c - 30}" fill="{BLANC}"/>',
         f'<rect x="{x}" y="{y}" width="{c}" height="{c}" fill="none" stroke="{INDIGO}" stroke-width="2.5"/>',
         imbrique(empile, x + 35, y + 30, c - 70, c - 60)]
    return papier_tampon(i, e)


PROPOSITIONS = [
    ("Pour validation", "Carte de visite fusionnée", carte,
     "Recto de la proposition 2 (Fondu, logo sur la zone calme), verso de la proposition 1 (nominatif, trois temps, sans motif).", "85 x 55 mm."),
    ("Pour validation", "Enveloppe revue", enveloppe,
     "Dehors : Fondu clair sous l'adresse de retour, zone du destinataire libre. Dedans : tout l'intérieur imprimé Nuit au motif Tournant.", "DL 220 x 110 mm."),
    ("Chemise A", "Tournant qui s'efface", chemise_degrade,
     "Le Tournant dont les quarts de disque rapetissent vers le bas : la zone calme du logo vient du Fondu.", "A4, deux rabats, sans les trois temps."),
    ("Chemise B", "Tournant en diagonale", chemise_diagonale,
     "Même principe, en diagonale : dense en haut à gauche, le logo dans le coin calme en bas à droite.", "A4, deux rabats, sans les trois temps."),
    ("Chemise C", "Fondu dehors, Tournant dedans", chemise_dedans_dehors,
     "La couverture au Fondu, l'intérieur au Tournant, rabat Indigo : la chemise change de motif à l'ouverture.", "A4, deux rabats, sans les trois temps."),
    ("Signature A", "Bandeau Fondu", signature_bandeau_fondu,
     "Icône, nom et trois temps, puis un bandeau Nuit au Fondu avec le logo sur la zone calme.", "Bandeau en image, 340 x 52 px environ."),
    ("Signature B", "Carte Fondu", signature_carte_fondu,
     "Toute la signature sur un bloc Nuit : le Fondu à gauche, le texte blanc sur la zone calme.", "Bloc en image, 340 x 120 px environ."),
    ("Signature C", "Bande Tournant", signature_bande_tournant,
     "Icône, nom et trois temps, puis une fine bande Tournant, sans texte ni logo dessus.", "Bande en image, 340 x 34 px environ."),
    ("Tampon A", "Cachet rond", tampon_rond,
     "Le symbole au centre, le nom et les coordonnées autour, entre deux cercles.", "Diamètre 40 mm environ, une couleur."),
    ("Tampon B", "Rectangle Fondu", tampon_fondu,
     "Le Fondu à l'encre qui s'efface vers le logo et les coordonnées.", "Environ 50 x 22 mm, une couleur."),
    ("Tampon C", "Cadre Tournant", tampon_cadre_tournant,
     "Un cadre de quarts de disque Tournant autour du logo empilé.", "Environ 30 x 30 mm, une couleur."),
]


def page(cartes):
    return t1.page(cartes).replace("Papeterie : propositions</title>", "Papeterie : deuxième tour</title>") \
        .replace("<span>08</span>Papeterie : propositions", "<span>08</span>Papeterie : deuxième tour") \
        .replace(t1.page("").split('<p class="constat">')[1].split("</p>")[0],
                 "<b>Choix du premier tour.</b> Retenus tels quels : papier à en-tête, badge et tour de cou, carnet et autocollants. "
                 "Ici : la carte fusionnée et l'enveloppe revue pour validation, puis trois variantes pour la chemise (sans les trois temps), "
                 "la signature de courriel (avec Fondu ou Tournant) et le tampon.")


if __name__ == "__main__":
    gm.D_MOT, gm.L_MOT = gm.mot()
    cartes = []
    for i, (num, nom, fn, idee, dim) in enumerate(PROPOSITIONS, 1):
        dessin = (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}" aria-hidden="true">'
                  + "".join(fn(i)) + "</svg>")
        cartes.append(f'<div><div class="panneau">{dessin}</div><div class="num">{num}</div>'
                      f'<h2>{nom}</h2><p>{idee}</p><p class="dim">{dim}</p></div>')
    html = ICI / "tour2.html"
    html.write_text(page(cartes), encoding="utf-8")
    png = ICI / "propositions-papeterie-tour2.png"
    subprocess.run([t1.CHROME, "--headless=new", "--hide-scrollbars", "--force-device-scale-factor=1",
                    "--allow-file-access-from-files", f"--screenshot={png}", "--window-size=2400,2580", html.as_uri()],
                   check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    print("ok", len(PROPOSITIONS))
