"""Scènes de papeterie de la planche 08 (identité « Quatre pièces »), retenues par l'utilisateur le
2026-11-13 après deux tours de propositions (propositions-2026-11-13-papeterie/) :
carte de visite (recto Tournant, verso Fondu clair), papier à en-tête, enveloppe (Fondu dehors,
intérieur entièrement imprimé), chemise (dehors comme le carnet sans l'élastique, dedans Tournant et rabat Indigo),
badge et tour de cou, trois signatures de courriel, tampons par degré (système 1 : entreprise, direction,
responsable, service), carnet et autocollants.

Papeterie de l'éditeur seulement : les documents produits par le logiciel gardent l'identité du client.
Les coordonnées restent entre crochets. Le module generer.py de la marque et celui des scènes de
signalétique (pour les outils de dessin) sont injectés par l'appelant : scenes.gm, scenes.sc.
"""

gm = None
sc = None

NUIT, PIECES, INDIGO, AMBRE = "#1A2259", "#232C6B", "#2D3A8C", "#E3A23B"
BLANC, TRAIT, ARDOISE, ENCRE = "#FFFFFF", "#D9DCE3", "#52606D", "#1F2933"
PIECE_CLAIRE, PIECE_ENV = "#ECEEF4", "#E4E7F0"


# --- outils ------------------------------------------------------------------------------
def img(*a):
    return sc.img(*a)


def imbrique(*a):
    return sc.imbrique(*a)


def rect(*a, **k):
    return sc.rect(*a, **k)


def texte(*a, **k):
    return sc.texte(*a, **k)


def ombre(i):
    return (f'<defs><filter id="{i}" x="-20%" y="-20%" width="140%" height="140%">'
            f'<feDropShadow dx="0" dy="4" stdDeviation="6" flood-color="#0F172A" flood-opacity=".14"/></filter></defs>')


def filtre(i):
    return f'filter="url(#{i})"'


def trois_temps(x, y, l=18, e=2.5, g=5, clair=BLANC):
    return rect(x, y, l, e, clair) + rect(x + l + g, y, l, e, clair) + rect(x + 2 * (l + g), y, l, e, AMBRE)


def transparent(s):
    return s.replace("<rect", '<rect opacity="0"', 1)


def miroir(s, x, y, w, h):
    """Un motif retourné de gauche à droite : le Fondu s'efface alors vers la gauche."""
    return f'<g transform="translate({x + w} {y}) scale(-1 1)">{imbrique(s, 0, 0, w, h)}</g>'


def lignes(x, y, largeurs, pas=9, couleur="#E3E6EC", e=3):
    return "".join(rect(x, y + k * pas, l, e, couleur) for k, l in enumerate(largeurs))


def sans_titre(s):
    return s.replace('role="img" aria-label="Kadenz"><title>Kadenz</title>', 'aria-hidden="true">')


def mot_pret():
    if gm.D_MOT is None:
        gm.D_MOT, gm.L_MOT = gm.mot()


def logo_encre():
    """Logo horizontal d'une seule couleur, Indigo : l'encre des tampons."""
    mot_pret()
    p, _ = gm.pieces()
    return sans_titre(gm.horizontal(gm.symbole(p, INDIGO, INDIGO), INDIGO))


def empile_encre():
    mot_pret()
    p, _ = gm.pieces()
    return sans_titre(gm.empile(gm.symbole(p, INDIGO, INDIGO), INDIGO))


# --- pièces (cadre de 400 x 500) ------------------------------------------------------------
def carte(i):
    """Recto Nuit : Tournant à gauche, logo blanc sur la partie calme ; verso nominatif, Fondu clair à droite."""
    x, w, h = 40, 320, 207
    out = [ombre(i), rect(x, 50, w, h, NUIT, 3, filtre(i))]
    out.append(imbrique(gm.motif_tournant(NUIT, PIECES, 140, h, 34.5), x, 50, 140, h))
    out.append(img("kadenz-logo-horizontal-negatif.svg", x + 158, 132, 146, 42))
    y = 275
    out.append(rect(x, y, w, h, BLANC, 3, filtre(i)))
    out.append(miroir(transparent(gm.motif_fondu(BLANC, PIECE_CLAIRE, 150, h, 23, 0.8)), x + 170, y, 150, h))
    out.append(img("kadenz-symbole.svg", x + 20, y + 18, 26, 26))
    out.append(texte(x + 20, y + 74, "[Prénom Nom]", 17, ENCRE, 700, "start", "Space Grotesk"))
    out.append(texte(x + 20, y + 92, "[Fonction]", 11, ARDOISE, 400, "start"))
    out.append(trois_temps(x + 20, y + 106, 14, 2.5, 4, INDIGO))
    for k, t in enumerate(("[Téléphone]", "[E-mail]", "[Site]")):
        out.append(texte(x + 20, y + 140 + k * 16, t, 10.5, ENCRE, 400, "start"))
    return out


def papier(i):
    x, y, w, h = 70, 56, 260, 368
    out = [ombre(i), rect(x, y, w, h, BLANC, 2, filtre(i))]
    out.append(miroir(transparent(gm.motif_fondu(BLANC, PIECE_CLAIRE, w, 52, 13, 0.6)), x, y, w, 52))
    out.append(img("kadenz-logo-horizontal.svg", x + 20, y + 16, 92, 24))
    out.append(lignes(x + 24, y + 86, (70, 96)))
    out.append(lignes(x + 24, y + 124, (212, 212, 212, 150), 8))
    out.append(lignes(x + 24, y + 166, (212, 212, 120), 8))
    out.append(rect(x + 24, y + h - 34, w - 48, 1, TRAIT))
    out.append(texte(x + w / 2, y + h - 20, "[Adresse]  ·  [Téléphone]  ·  [E-mail]  ·  [Site]", 7, ARDOISE, 400))
    return out


def enveloppe(i):
    out = [ombre(i)]
    x, y, w, h = 40, 34, 320, 160
    out.append(rect(x, y, w, h, BLANC, 2, filtre(i)))
    out.append(imbrique(transparent(gm.motif_fondu(BLANC, PIECE_ENV, 150, 96, 16, 0.75)), x, y + 64, 150, 96))
    out.append(img("kadenz-logo-horizontal.svg", x + 16, y + 14, 84, 22))
    out.append(texte(x + 16, y + 50, "[Adresse de retour]", 8, ARDOISE, 400, "start"))
    out.append(lignes(x + 170, y + 100, (110, 130, 90), 9))
    bx, by, bw, bh = 110, 300, 180, 90
    contour = (f"M{bx} {by} L{bx + bw / 2} {by - 58} L{bx + bw} {by} L{bx + bw + 46} {by + 12} L{bx + bw + 46} {by + bh - 12}"
               f" L{bx + bw} {by + bh} L{bx + bw - 12} {by + bh + 64} L{bx + 12} {by + bh + 64} L{bx} {by + bh}"
               f" L{bx - 46} {by + bh - 12} L{bx - 46} {by + 12} Z")
    out.append(f'<defs><clipPath id="{i}c"><path d="{contour}"/></clipPath></defs>')
    out.append(f'<path d="{contour}" fill="{NUIT}" {filtre(i)}/>')
    out.append(f'<g clip-path="url(#{i}c)">{imbrique(gm.motif_tournant(NUIT, PIECES, 272, 212, 22.7), bx - 46, by - 58, 272, 212)}</g>')
    out.append(f'<rect x="{bx}" y="{by}" width="{bw}" height="{bh}" fill="none" stroke="#3A4580" stroke-dasharray="4 3"/>')
    out.append(texte(200, 478, "patron à plat, face intérieure", 10, ARDOISE, 400))
    return out


def couverture(i, x, y, w, h, elastique=True):
    """Couverture Nuit : Tournant sur les trois quarts, bande calme avec le logo (carnet, chemise)."""
    haut = round(h * 0.72)
    out = [rect(x, y, w, h, NUIT, 3, filtre(i))]
    out.append(imbrique(gm.motif_tournant(NUIT, PIECES, w, haut, w / 8), x, y, w, haut))
    out.append(img("kadenz-logo-horizontal-negatif.svg", x + w * 0.1, y + haut + (h - haut) / 2 - w * 0.08, w * 0.55, w * 0.16))
    if elastique:
        out.append(rect(x + w - w * 0.13, y, w * 0.04, h, "#11173D"))
    return out


def chemise(i):
    out = [ombre(i)]
    x1, x2, y, w, h = 24, 206, 110, 170, 240
    out += couverture(i, x1, y, w, h, elastique=False)
    out.append(rect(x2, y, w, h, NUIT, 2, filtre(i)))
    out.append(imbrique(gm.motif_tournant(NUIT, PIECES, w, h, 28.3), x2, y, w, h))
    out.append(rect(x2, y + 160, w, 80, INDIGO, 2) + rect(x2 + 50, y + 168, 70, 3, "#20296A"))
    out.append(texte(x1 + w / 2, y + h + 26, "dehors", 11, ARDOISE, 500))
    out.append(texte(x2 + w / 2, y + h + 26, "dedans, rabat Indigo", 11, ARDOISE, 500))
    return out


def badge(i):
    x, y, w, h = 110, 150, 180, 240
    out = [ombre(i)]
    out.append(f'<path d="M150 0 L186 {y - 8} M250 0 L214 {y - 8}" stroke="{INDIGO}" stroke-width="12" fill="none"/>')
    out.append(rect(188, y - 18, 24, 26, "#9AA3B5", 3))
    out.append(rect(x, y, w, h, BLANC, 4, filtre(i)))
    out.append(rect(x, y, w, 120, NUIT, 4) + rect(x, y + 110, w, 10, NUIT))
    out.append(imbrique(gm.motif_fondu(NUIT, PIECES, w, 120, 20, 0.75, "bas"), x, y, w, 120))
    out.append(img("kadenz-logo-horizontal-negatif.svg", x + 30, y + 76, 120, 32))
    out.append(texte(x + w / 2, y + 160, "[Prénom Nom]", 17, ENCRE, 700, police="Space Grotesk"))
    out.append(texte(x + w / 2, y + 180, "[Fonction]", 11, ARDOISE, 400))
    out.append(trois_temps(x + w / 2 - 27, y + 200, 14, 2.5, 6, INDIGO))
    return out


def carnet(i):
    out = [ombre(i)]
    out += couverture(i, 60, 60, 200, 290)
    out.append(f'<circle cx="320" cy="380" r="52" fill="{BLANC}" {filtre(i)}/>')
    out.append(img("kadenz-symbole.svg", 290, 350, 60, 60))
    out.append(f'<g {filtre(i)}>{img("kadenz-icone.svg", 266, 196, 92, 92)}</g>')
    return out


# --- signatures de courriel (cadre de 680 x 428) ----------------------------------------------
def bloc_nom(x, y, couleur=ENCRE, second=ARDOISE, temps=INDIGO):
    return [texte(x, y + 14, "[Prénom Nom]", 14, couleur, 700, "start", "Space Grotesk"),
            texte(x, y + 30, "[Fonction] · Kadenz", 10, second, 400, "start"),
            trois_temps(x, y + 40, 12, 2.5, 4, temps),
            texte(x, y + 62, "[Téléphone] · [Site]", 10, couleur, 400, "start")]


def signature_bandeau_fondu(x, y, w):
    out = [img("kadenz-icone.svg", x, y, 48, 48)] + bloc_nom(x + 60, y)
    out.append(rect(x, y + 86, w, 52, NUIT, 2))
    out.append(imbrique(gm.motif_fondu(NUIT, PIECES, w, 52, 13, 0.6), x, y + 86, w, 52))
    out.append(img("kadenz-logo-horizontal-negatif.svg", x + w - 112, y + 98, 100, 28))
    return out


def signature_carte_fondu(x, y, w):
    out = [rect(x, y, w, 120, NUIT, 3)]
    out.append(imbrique(gm.motif_fondu(NUIT, PIECES, w, 120, 20, 0.36), x, y, w, 120))
    out += bloc_nom(x + 128, y + 22, BLANC, "#C5CAE0", BLANC)
    return out


def signature_bande_tournant(x, y, w):
    out = [img("kadenz-icone.svg", x, y, 48, 48)] + bloc_nom(x + 60, y)
    out.append(rect(x, y + 86, w, 34, NUIT, 2))
    out.append(imbrique(gm.motif_tournant(NUIT, PIECES, w, 34, 17), x, y + 86, w, 34))
    return out


def signatures(i):
    out = [ombre(i), rect(10, 10, 660, 408, BLANC, 3, filtre(i))]
    for lettre, fn, x, y in (("A · bandeau Fondu", signature_bandeau_fondu, 30, 50),
                             ("C · bande Tournant", signature_bande_tournant, 30, 250),
                             ("B · carte Fondu", signature_carte_fondu, 350, 50)):
        out.append(texte(x, y - 14, lettre, 11, ARDOISE, 600, "start"))
        out += fn(x, y, 300)
    return out


# --- tampons (cadre de 680 x 428) ------------------------------------------------------------
def tampon_rond(i, cx, cy, r=82):
    arc = f"M{cx - r + 16} {cy} A{r - 16} {r - 16} 0 1 1 {cx + r - 16} {cy} A{r - 16} {r - 16} 0 1 1 {cx - r + 16} {cy}"
    return [f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="none" stroke="{INDIGO}" stroke-width="3"/>',
            f'<circle cx="{cx}" cy="{cy}" r="{r - 30}" fill="none" stroke="{INDIGO}" stroke-width="1.5"/>',
            f'<defs><path id="{i}r" d="{arc}"/></defs>',
            f'<text font-family="Inter" font-size="10.5" font-weight="600" fill="{INDIGO}" letter-spacing=".06em">'
            f'<textPath href="#{i}r" textLength="{2 * 3.1416 * (r - 16) - 6:.0f}" lengthAdjust="spacing">'
            f'KADENZ  ·  [ADRESSE]  ·  [TÉLÉPHONE]  ·  [SITE]  ·</textPath></text>',
            img("kadenz-symbole.svg", cx - 30, cy - 30, 60, 60)]


def tampon_fondu(x, y, w=228, h=96):
    return [f'<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="none" stroke="{INDIGO}" stroke-width="2.5"/>',
            imbrique(transparent(gm.motif_fondu(BLANC, INDIGO, 100, h - 12, 12, 0.85)), x + 6, y + 6, 100, h - 12),
            imbrique(logo_encre(), x + 104, y + 18, 112, 30),
            texte(x + 106, y + 66, "[Adresse]", 9, INDIGO, 500, "start"),
            texte(x + 106, y + 80, "[Téléphone]", 9, INDIGO, 500, "start")]


def tampon_cadre(x, y, c=150):
    return [imbrique(transparent(gm.motif_tournant(BLANC, INDIGO, c, c, 15)), x, y, c, c),
            f'<rect x="{x + 15}" y="{y + 15}" width="{c - 30}" height="{c - 30}" fill="{BLANC}"/>',
            f'<rect x="{x}" y="{y}" width="{c}" height="{c}" fill="none" stroke="{INDIGO}" stroke-width="2.5"/>',
            imbrique(empile_encre(), x + 35, y + 30, c - 70, c - 60)]


def ecrit(x, y, s, taille=11, graisse=600, ancre="middle", esp=".04em"):
    return (f'<text x="{x}" y="{y}" font-family="Inter" font-size="{taille}" font-weight="{graisse}" fill="{INDIGO}" '
            f'text-anchor="{ancre}" letter-spacing="{esp}">{s}</text>')


ENCRE_TRAIT = f'fill="none" stroke="{INDIGO}"'


def tampon_direction(cx, cy, nom="[Direction]"):
    """Degré 2 : rectangle Fondu au nom de la direction, un par direction."""
    w, h = 250, 96
    x, y = cx - w / 2, cy - h / 2
    return [f'<rect x="{x}" y="{y}" width="{w}" height="{h}" {ENCRE_TRAIT} stroke-width="2.5"/>',
            imbrique(transparent(gm.motif_fondu(BLANC, INDIGO, 90, h - 12, 12, 0.85)), x + 6, y + 6, 90, h - 12),
            imbrique(logo_encre(), x + 100, y + 12, 100, 27),
            ecrit(x + 102, y + 58, "DIRECTION", 9, 600, "start", ".12em"),
            ecrit(x + 102, y + 78, nom, 13, 700, "start", "0")]


def tampon_responsable(cx, cy):
    """Degré 3 : rectangle nominatif, cadre Tournant autour du symbole, ligne de signature."""
    w, h, c = 270, 112, 76
    x, y = cx - w / 2, cy - h / 2
    return [f'<rect x="{x}" y="{y}" width="{w}" height="{h}" {ENCRE_TRAIT} stroke-width="2.5"/>',
            imbrique(transparent(gm.motif_tournant(BLANC, INDIGO, c, c, 9.5)), x + 12, y + 18, c, c),
            f'<rect x="{x + 21.5}" y="{y + 27.5}" width="{c - 19}" height="{c - 19}" fill="{BLANC}"/>',
            img("kadenz-symbole.svg", x + 30, y + 36, c - 36, c - 36),
            ecrit(x + 102, y + 30, "[Prénom Nom]", 13, 700, "start", "0"),
            ecrit(x + 102, y + 47, "[Fonction]", 10, 500, "start", "0"),
            ecrit(x + 102, y + 62, "[Direction]", 10, 500, "start", "0"),
            ecrit(x + 102, y + 96, "Signature", 8.5, 600, "start", ".08em"),
            f'<path d="M{x + 152} {y + 96} L{x + w - 14} {y + 96}" {ENCRE_TRAIT} stroke-width="1" stroke-dasharray="2 3"/>']


def tampon_service(cx, cy):
    """Degré 4 : dateur du service (réception, courrier)."""
    w, h = 230, 70
    x, y = cx - w / 2, cy - h / 2
    return [f'<rect x="{x}" y="{y}" width="{w}" height="{h}" {ENCRE_TRAIT} stroke-width="2"/>',
            img("kadenz-symbole.svg", x + 12, y + 15, 40, 40),
            ecrit(x + 64, y + 24, "[Service]", 11, 700, "start", "0"),
            ecrit(x + 64, y + 44, "REÇU LE", 9, 600, "start", ".12em"),
            f'<rect x="{x + 118}" y="{y + 32}" width="{w - 132}" height="18" {ENCRE_TRAIT} stroke-width="1"/>',
            ecrit(x + 118 + (w - 132) / 2, y + 45, "[JJ / MM / AAAA]", 9, 500),
            ecrit(x + 64, y + 62, "PAR", 8, 600, "start", ".12em"),
            f'<path d="M{x + 88} {y + 62} L{x + w - 14} {y + 62}" {ENCRE_TRAIT} stroke-width="1" stroke-dasharray="2 3"/>']


def tampons(i):
    """Système 1, retenu le 2026-11-13 : une forme par degré, une seule couleur d'encre.
    Degré 1 entreprise (un seul), 2 direction (un par direction), 3 responsable (un par responsable),
    4 service (un par service)."""
    out = [ombre(i), rect(10, 10, 660, 408, BLANC, 2, filtre(i))]
    empreintes = (tampon_rond(i, 140, 214) + tampon_direction(480, 82) + tampon_responsable(480, 214)
                  + tampon_service(480, 340))
    out.append(f'<g opacity=".88">{"".join(empreintes)}</g>')
    for x, y, t in ((140, 330, "1 · Entreprise, un seul"), (480, 26, "2 · Direction, une par direction"),
                    (480, 150, "3 · Responsable, nominatif"), (480, 297, "4 · Service : réception, courrier")):
        out.append(texte(x, y, t, 11, ARDOISE, 600))
    return out
