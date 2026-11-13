"""Scènes de signalétique de la planche 07 (identité « Quatre pièces »), retenues par l'utilisateur
le 2026-11-13 parmi les propositions de propositions-2026-11-13-signaletique/ (toutes sauf les postes
de démonstration ; vitrophanie passée au motif Fondu à sa demande).

Chaque scène est une liste d'éléments SVG dans un cadre de 400 x 500, avec une silhouette de 1,75 m
pour l'échelle. Les chemins des images sont relatifs à un fichier HTML placé dans un sous-dossier
de docs/marque. Le module generer.py de la marque est injecté par l'appelant : scenes.gm = gm.
"""

gm = None

NUIT, PIECES, INDIGO, AMBRE = "#1A2259", "#232C6B", "#2D3A8C", "#E3A23B"
BRUME, BLANC, TRAIT, ARDOISE = "#F4F5F9", "#FFFFFF", "#D9DCE3", "#52606D"
MUR, SOL, GRIS = "#E6E9EF", "#CBD0DA", "#A3AABA"
W, H = 400, 500


def img(nom, x, y, w, h):
    return f'<image href="../{nom}" x="{x}" y="{y}" width="{w}" height="{h}"/>'


def imbrique(s, x, y, w, h):
    """Un SVG de generer.py posé dans la scène, à la taille (w, h) pour laquelle il a été calculé."""
    s = s.replace(' xmlns="http://www.w3.org/2000/svg"', "")
    return s.replace("<svg ", f'<svg x="{x}" y="{y}" width="{w}" height="{h}" ', 1).strip()


def rect(x, y, w, h, fill, rx=0, extra=""):
    return f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" fill="{fill}" {extra}/>'


def texte(x, y, t, taille, couleur, graisse=500, ancre="middle", police="Inter"):
    return (f'<text x="{x}" y="{y}" font-family="{police}" font-size="{taille}" font-weight="{graisse}" '
            f'fill="{couleur}" text-anchor="{ancre}">{t}</text>')


def personne(x, sol, h=150):
    """Silhouette d'échelle : une personne de 1,75 m (h px)."""
    r = h * 0.075
    return (f'<circle cx="{x}" cy="{sol - h + r}" r="{r}" fill="{GRIS}"/>'
            f'<rect x="{x - h * 0.12}" y="{sol - h + 2.25 * r}" width="{h * 0.24}" height="{h - 2.25 * r}" rx="{h * 0.05}" fill="{GRIS}"/>')


def decor(sol=420, mur=MUR):
    return rect(0, 0, W, sol, mur) + rect(0, sol, W, H - sol, SOL)


# --- les scènes ------------------------------------------------------------------------
def facade():
    s, out = 420, []
    out.append(decor(s))
    out.append(rect(40, 60, 320, 96, NUIT, 2))
    out.append(imbrique(gm.motif_fondu(NUIT, PIECES, 320, 96, 24, 0.5), 40, 60, 320, 96))
    out.append(img("kadenz-logo-horizontal-negatif.svg", 190, 86, 150, 44))
    out.append(rect(40, 156, 320, 4, "#11173D"))
    out.append(rect(124, 222, 152, 198, "#8E96A8"))
    out.append(rect(132, 230, 66, 190, "#EEF1F6") + rect(202, 230, 66, 190, "#EEF1F6"))
    out.append(img("kadenz-symbole.svg", 186, 300, 28, 28))
    out.append(personne(330, s))
    return out


def drapeau():
    s, out = 420, []
    out.append(decor(s))
    out.append(rect(0, 0, 70, s, "#D5D9E1"))
    out.append(rect(70, 116, 200, 6, "#7C8496") + rect(70, 106, 10, 26, "#7C8496"))
    out.append(rect(110, 122, 3, 14, "#7C8496") + rect(227, 122, 3, 14, "#7C8496"))
    out.append(rect(90, 136, 160, 160, INDIGO, 2))
    out.append(img("kadenz-symbole-blanc.svg", 120, 160, 100, 100))
    out.append(rect(282, 136, 14, 160, "#20296A", 1))
    out.append(texte(289, 316, "chant", 11, ARDOISE, 400))
    return out


def lettres():
    s, out = 400, []
    out.append(decor(s, "#EEF0F5"))
    out.append(img("kadenz-logo-horizontal.svg", 50, 110, 300, 84))
    out.append(rect(90, 290, 220, 110, BLANC, 2, f'stroke="{TRAIT}"'))
    out.append(rect(90, 290, 220, 14, NUIT, 2))
    out.append(personne(350, s, 175))
    return out


def plaque():
    out = [rect(0, 0, W, H, "#DADDE4")]
    for y in range(0, H, 34):
        out.append(rect(0, y, W, 1, "#CDD1D9"))
    out.append(rect(300, 0, 100, H, "#8E96A8") + rect(312, 0, 88, H, "#EEF1F6"))
    out.append(rect(46, 170, 220, 146, NUIT, 2))
    for cx, cy in ((58, 182), (254, 182), (58, 304), (254, 304)):
        out.append(f'<circle cx="{cx}" cy="{cy}" r="4" fill="#B8BFCC"/>')
    out.append(img("kadenz-logo-horizontal-negatif.svg", 70, 206, 150, 44))
    out.append(rect(76, 266, 30, 3, BLANC) + rect(110, 266, 30, 3, BLANC) + rect(144, 266, 30, 3, AMBRE))
    out.append(texte(76, 292, "Gestion commerciale", 13, "#C5CAE0", 500, "start"))
    return out


def vitrophanie():
    """Bande dépolie entre 1,1 et 1,6 m (repère de sécurité des portes vitrées) : motif Fondu blanc
    sur le dépoli, ton sur ton, logo couleur sur la zone calme à droite."""
    s, out = 440, []
    m = 410 / 2.2   # px par mètre
    out.append(decor(s))
    out.append(rect(30, 30, 270, 410, "#8E96A8"))
    out.append(rect(42, 42, 118, 398, "#DCE3EC") + rect(170, 42, 118, 398, "#DCE3EC"))
    y0, y1 = s - 1.6 * m, s - 1.1 * m
    out.append(rect(42, y0, 246, y1 - y0, "#F2F4F8", 0, 'opacity=".9"'))
    fondu = gm.motif_fondu("#F2F4F8", BLANC, 246, y1 - y0, 18.6, 0.62).replace('<rect', '<rect opacity="0"', 1)
    out.append(imbrique(fondu, 42, y0, 246, y1 - y0))
    out.append(img("kadenz-logo-horizontal.svg", 168, (y0 + y1) / 2 - 15, 112, 30))
    out.append(rect(152, y1 + 20, 6, 60, "#7C8496") + rect(172, y1 + 20, 6, 60, "#7C8496"))
    out.append(personne(352, s, 1.75 * m))
    return out


def totem():
    s, out = 430, []
    m = 300 / 2.2
    out.append(decor(s))
    out.append(rect(140, 120, 110, 300, NUIT, 2))
    out.append(imbrique(gm.motif_tournant(NUIT, PIECES, 110, 88, 22), 140, 120, 110, 88))
    out.append(img("kadenz-logo-empile-negatif.svg", 157, 228, 76, 94))
    out.append(rect(160, 350, 20, 3, BLANC) + rect(185, 350, 20, 3, BLANC) + rect(210, 350, 20, 3, AMBRE))
    out.append(rect(130, 420, 130, 10, "#7C8496"))
    out.append(personne(325, s, 1.75 * m))
    return out


def kakemono():
    s, out = 430, []
    m = 300 / 2.0
    x, y, w = 120, 110, 128
    out.append(decor(s))
    out.append(rect(x, y, w, 300, NUIT))
    out.append(imbrique(gm.motif_fondu(NUIT, PIECES, w, 110, 16, 0.85, "bas"), x, y, w, 110))
    out.append(img("kadenz-logo-empile-negatif.svg", x + 30, y + 112, 68, 84))
    out.append(texte(x + w / 2, y + 222, "Gestion commerciale", 10.5, BLANC, 700, police="Space Grotesk"))
    for i, t in enumerate(("Devis et proforma", "Bons de commande", "Bordereaux de livraison", "Factures et paiements")):
        out.append(texte(x + w / 2, y + 240 + i * 13, t, 8.5, "#C5CAE0", 400))
    out.append(rect(x, y + 294, w, 6, AMBRE))
    out.append(rect(x - 8, 410, w + 16, 12, "#7C8496", 2))
    out.append(personne(320, s, 1.75 * m))
    return out


def stand():
    s, out = 430, []
    out.append(decor(s, "#F0F2F6"))
    out.append(rect(20, 70, 360, 260, NUIT))
    out.append(imbrique(gm.motif_tournant(NUIT, PIECES, 170, 260, 34), 20, 70, 170, 260))
    out.append(img("kadenz-logo-horizontal-negatif.svg", 210, 160, 150, 44))
    out.append(rect(230, 330, 120, 100, NUIT, 2))
    out.append(img("kadenz-symbole-blanc.svg", 268, 356, 44, 44))
    out.append(rect(20, 330, 360, 4, "#11173D"))
    out.append(personne(120, s, 182))
    return out
