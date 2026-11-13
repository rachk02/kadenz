"""Scènes de la planche 09, mises en situation (identité « Quatre pièces »), retenues par l'utilisateur
le 2026-11-13 parmi les propositions de propositions-2026-11-13-mises-en-situation/ (toutes sauf le site web).

Libellés tirés des écrans réels (connexion.slint, assistant.slint, reglages.slint) et de l'installateur
(installateur/theme/kadenz.wxl) ; le reste est entre crochets. Ces scènes montrent l'identité appliquée :
l'application et l'installateur ne sont pas modifiés par ce fichier.
Les modules generer.py, scenes_signaletique.py et scenes_papeterie.py sont injectés par l'appelant :
scenes.gm, scenes.sc, scenes.sp, puis scenes.prepare().
"""

gm = sc = sp = None


def prepare():
    global img, imbrique, rect, texte, trois_temps, ombre, filtre
    img, imbrique, rect, texte = sc.img, sc.imbrique, sc.rect, sc.texte
    trois_temps, ombre, filtre = sp.trois_temps, sp.ombre, sp.filtre


NUIT, PIECES, INDIGO, AMBRE, BLANC = "#1A2259", "#232C6B", "#2D3A8C", "#E3A23B", "#FFFFFF"
ENCRE, ARDOISE, TRAIT, BRUME, CLAIR = "#1F2933", "#52606D", "#D9DCE3", "#F4F5F9", "#C5CAE0"
W, H = 520, 390


def t(x, y, s, taille=9, couleur=ENCRE, graisse=400, ancre="start", police="Inter"):
    return texte(x, y, s, taille, couleur, graisse, ancre, police)


def fenetre(i, x, y, w, h, titre="Kadenz"):
    """Cadre de fenêtre Windows : barre de titre blanche, icône, boutons."""
    return [ombre(i), rect(x, y, w, h, BLANC, 3, filtre(i)), rect(x, y + 22, w, 1, TRAIT),
            img("kadenz-icone.svg", x + 8, y + 5, 12, 12), t(x + 26, y + 15, titre, 8, ENCRE, 500),
            t(x + w - 54, y + 15, "–", 9, ARDOISE), rect(x + w - 34, y + 7, 8, 8, "none", 0, f'stroke="{ARDOISE}"'),
            t(x + w - 14, y + 15, "x", 8, ARDOISE)]


def champ(x, y, w, libelle):
    return [t(x, y, libelle, 7.5, ENCRE, 500), rect(x, y + 5, w, 18, BLANC, 2, f'stroke="{TRAIT}"')]


def bouton(x, y, w, s):
    return [rect(x, y, w, 22, INDIGO, 3), t(x + w / 2, y + 14.5, s, 8, BLANC, 600, "middle")]


def retourne_haut_bas(s, x, y, w, h):
    return f'<g transform="translate({x} {y + h}) scale(1 -1)">{imbrique(s, 0, 0, w, h)}</g>'


# --- les mises en situation ----------------------------------------------------------------
def connexion(i):
    x, y, w, h = 20, 18, 480, 350
    out = fenetre(i, x, y, w, h)
    px, py, pw, ph = x, y + 23, 200, h - 23
    out.append(rect(px, py, pw, ph, NUIT))
    out.append(imbrique(gm.motif_fondu(NUIT, PIECES, pw, 150, 20, 0.9, "bas"), px, py, pw, 150))
    out.append(img("kadenz-logo-horizontal-negatif.svg", px + 20, py + 160, 120, 34))
    out.append(trois_temps(px + 22, py + 210, 14, 2.5, 4))
    out.append(t(px + 20, py + 234, "Gestion commerciale", 12, BLANC, 700, police="Space Grotesk"))
    for k, ligne in enumerate(("Devis, proforma, bons de commande,", "bordereaux de livraison, factures", "et paiements.")):
        out.append(t(px + 20, py + 252 + k * 12, ligne, 7.5, CLAIR))
    out.append(t(px + 20, py + ph - 14, "Version [x.y.z]", 7, CLAIR))
    fx = px + pw + 40
    out.append(t(fx, py + 70, "CONNEXION", 8, ARDOISE, 600))
    out.append(t(fx, py + 88, "Saisissez votre identifiant et votre mot de passe.", 8, ENCRE))
    out += champ(fx, py + 118, 200, "Identifiant") + champ(fx, py + 160, 200, "Mot de passe")
    out += bouton(fx, py + 204, 200, "Se connecter")
    return out


def assistant(i):
    x, y, w, h = 20, 18, 480, 350
    out = fenetre(i, x, y, w, h)
    px, py, pw, ph = x, y + 23, 160, h - 23
    out.append(rect(px, py, pw, ph, NUIT))
    out.append(retourne_haut_bas(gm.motif_fondu(NUIT, PIECES, pw, 140, 20, 0.9, "bas"), px, py + ph - 140, pw, 140))
    out.append(img("kadenz-logo-horizontal-negatif.svg", px + 18, py + 20, 100, 28))
    for k, etape in enumerate(("Licence", "Entreprise", "Administrateur", "Taux et paiements", "Récapitulatif")):
        actif = k == 1
        if actif:
            out.append(f'<circle cx="{px + 22}" cy="{py + 79 + k * 20}" r="3" fill="{AMBRE}"/>')
        out.append(t(px + 32, py + 82 + k * 20, etape, 8.5, BLANC if actif else CLAIR, 700 if actif else 400))
    fx = px + pw + 28
    out.append(t(fx, py + 42, "Première installation", 15, ENCRE, 700, police="Space Grotesk"))
    out.append(t(fx, py + 60, "Étape 2 sur 5 : Entreprise", 8.5, ARDOISE))
    out += champ(fx, py + 92, 260, "Nom de l'entreprise (obligatoire)") + champ(fx, py + 134, 260, "Adresse / boîte postale")
    out += champ(fx, py + 176, 125, "Téléphone") + champ(fx + 135, py + 176, 125, "Email")
    out += bouton(fx + 180, py + ph - 40, 80, "Suivant")
    out.append(t(fx + 120, py + ph - 25, "Précédent", 8, ARDOISE, 500))
    return out


def installateur(i):
    x, y, w, h = 30, 30, 460, 330
    out = fenetre(i, x, y, w, h)
    px, py, pw, ph = x, y + 23, 150, h - 23
    out.append(rect(px, py, pw, ph, NUIT))
    out.append(imbrique(gm.motif_tournant(NUIT, PIECES, pw, 170, 25), px, py, pw, 170))
    out.append(img("kadenz-logo-empile-negatif.svg", px + 35, py + 186, 80, 88))
    out.append(trois_temps(px + 45, py + 286, 16, 2.5, 4))
    fx = px + pw + 24
    out.append(t(fx, py + 40, "Bienvenue dans l'installation de Kadenz", 11.5, ENCRE, 700, police="Space Grotesk"))
    out.append(t(fx, py + 62, "Kadenz, logiciel de gestion commerciale, va être", 8, ENCRE))
    out.append(t(fx, py + 74, "installé sur cet ordinateur.", 8, ENCRE))
    out.append(t(fx, py + 104, "Dossier d'installation :", 8, ENCRE, 500))
    out.append(rect(fx, py + 110, 190, 18, BLANC, 2, f'stroke="{TRAIT}"') + t(fx + 6, py + 122, "[dossier]", 7.5, ARDOISE))
    out.append(rect(fx + 196, py + 110, 70, 18, BRUME, 2, f'stroke="{TRAIT}"') + t(fx + 231, py + 122, "Parcourir...", 7.5, ENCRE, 500, "middle"))
    for k, s in enumerate(("Créer un raccourci sur le bureau", "Ajouter Kadenz au menu Démarrer")):
        yy = py + 148 + k * 20
        out.append(rect(fx, yy - 8, 10, 10, INDIGO, 2) + f'<path d="M{fx + 2} {yy - 3} L{fx + 4.5} {yy - 0.5} L{fx + 8} {yy - 6}" stroke="#fff" stroke-width="1.5" fill="none"/>')
        out.append(t(fx + 16, yy, s, 8, ENCRE))
    out += bouton(fx + 186, py + ph - 38, 80, "Installer")
    out.append(t(fx + 140, py + ph - 23, "Annuler", 8, ARDOISE, 500, "middle"))
    return out


def github(i):
    x, y, w, h = 20, 75, 480, 240
    out = [ombre(i), rect(x, y, w, h, NUIT, 3, filtre(i))]
    out.append(imbrique(gm.motif_tournant(NUIT, PIECES, 180, h, 30), x, y, 180, h))
    out.append(img("kadenz-logo-horizontal-negatif.svg", x + 210, y + 70, 220, 62))
    out.append(t(x + 214, y + 162, "Gestion commerciale pour", 11, BLANC, 500))
    out.append(t(x + 214, y + 178, "Windows. Rust, Slint, SQLite.", 11, BLANC, 500))
    out.append(t(260, y + h + 28, "1280 x 640 px", 9, ARDOISE, 500, "middle"))
    return out


def bureau(i):
    x, y, w, h = 30, 24, 460, 290
    out = [ombre(i), rect(x - 8, y - 8, w + 16, h + 16, "#2B2F36", 6, filtre(i))]
    out.append(rect(x, y, w, h, NUIT))
    out.append(imbrique(gm.motif_tournant(NUIT, PIECES, w, h - 26, 36), x, y, w, h - 26))
    out.append(rect(x + 14, y + 14, 60, 66, NUIT, 3, 'opacity=".75"'))
    out.append(img("kadenz-icone.svg", x + 24, y + 20, 40, 40) + t(x + 44, y + 72, "Kadenz", 8, BLANC, 500, "middle"))
    out.append(rect(x, y + h - 26, w, 26, "#F3F4F7"))
    for k in range(5):
        cx = x + w / 2 - 70 + k * 30
        if k == 3:
            out.append(rect(cx - 2, y + h - 24, 24, 22, "#E3E6EE", 3) + img("kadenz-icone.svg", cx + 1, y + h - 22, 18, 18)
                       + rect(cx + 6, y + h - 3, 8, 2, INDIGO, 1))
        else:
            out.append(rect(cx + 1, y + h - 22, 18, 18, "#CDD2DC", 3))
    out.append(rect(230, y + h + 8, 60, 30, "#2B2F36") + rect(200, y + h + 36, 120, 8, "#2B2F36", 3))
    return out


def a_propos(i):
    x, y, w, h = 60, 22, 400, 346
    out = fenetre(i, x, y, w, h, "Kadenz : À propos")
    out.append(rect(x, y + 23, w, 110, NUIT))
    out.append(imbrique(gm.motif_fondu(NUIT, PIECES, w, 110, 22, 0.55), x, y + 23, w, 110))
    out.append(img("kadenz-logo-horizontal-negatif.svg", x + w - 176, y + 56, 150, 42))
    out.append(t(x + 24, y + 160, "Gestion commerciale", 13, ENCRE, 700, police="Space Grotesk"))
    out.append(t(x + 24, y + 178, "Version [x.y.z]", 8.5, ARDOISE))
    out.append(trois_temps(x + 24, y + 190, 14, 2.5, 4, INDIGO))
    out.append(rect(x + 24, y + 210, w - 48, 1, TRAIT))
    out.append(t(x + 24, y + 230, "Licence", 8, ARDOISE, 500) + t(x + 120, y + 230, "Logiciel propriétaire, tous droits réservés.", 8, ENCRE))
    out.append(t(x + 24, y + 250, "Composants tiers", 8, ARDOISE, 500) + rect(x + 120, y + 244, 180, 4, "#E3E6EC"))
    out.append(rect(x + 120, y + 254, 140, 4, "#E3E6EC"))
    out.append(rect(x + 24, y + h - 46, 150, 22, BLANC, 3, f'stroke="{INDIGO}"') + t(x + 99, y + h - 31.5, "Revoir la présentation", 8, INDIGO, 600, "middle"))
    return out


def navigateur_mobile(i):
    out = [ombre(i)]
    x, y, w = 20, 40, 300
    out.append(rect(x, y, w, 120, BLANC, 4, filtre(i)) + rect(x, y, w, 34, "#E6E9EF", 4) + rect(x, y + 30, w, 4, "#E6E9EF"))
    out.append(rect(x + 10, y + 8, 150, 26, BLANC, 4) + rect(x + 10, y + 30, 150, 4, BLANC))
    out.append(img("web/favicon.svg", x + 20, y + 14, 14, 14))
    out.append(t(x + 42, y + 25, "Kadenz", 8.5, ENCRE, 500) + t(x + 148, y + 25, "x", 8, ARDOISE))
    out.append(rect(x + 168, y + 12, 18, 18, "#DADDE4", 4))
    out.append(rect(x + 10, y + 46, 280, 8, "#E3E6EC") + rect(x + 10, y + 62, 220, 6, "#EEF0F4") + rect(x + 10, y + 74, 250, 6, "#EEF0F4"))
    out.append(t(x + w / 2, y + 144, "Onglet : favicon", 9, ARDOISE, 500, "middle"))
    px, py, pw, ph = 350, 30, 150, 300
    out.append(rect(px, py, pw, ph, "#2B2F36", 18, filtre(i)) + rect(px + 7, py + 7, pw - 14, ph - 14, NUIT, 13))
    out.append(f'<defs><clipPath id="{i}t"><rect x="{px + 7}" y="{py + 7}" width="{pw - 14}" height="{ph - 14}" rx="13"/></clipPath></defs>')
    out.append(f'<g clip-path="url(#{i}t)">{imbrique(gm.motif_tournant(NUIT, PIECES, pw - 14, ph - 14, 34), px + 7, py + 7, pw - 14, ph - 14)}</g>')
    for r in range(4):
        for c in range(3):
            ix, iy = px + 22 + c * 40, py + 40 + r * 52
            if (r, c) == (1, 1):
                out.append(img("kadenz-icone.svg", ix, iy, 28, 28) + t(ix + 14, iy + 38, "Kadenz", 6.5, BLANC, 500, "middle"))
            else:
                out.append(rect(ix, iy, 28, 28, "#3A4580", 7, 'opacity=".9"'))
    out.append(t(px + pw / 2, py + ph + 20, "Écran d'accueil : icône", 9, ARDOISE, 500, "middle"))
    return out


def reseaux(i):
    x, y, w, h = 30, 30, 460, 330
    out = [ombre(i), rect(x, y, w, h, BLANC, 4, filtre(i))]
    out.append(f'<defs><clipPath id="{i}b"><rect x="{x}" y="{y}" width="{w}" height="120" rx="4"/></clipPath></defs>')
    out.append(f'<g clip-path="url(#{i}b)">{rect(x, y, w, 120, NUIT)}{imbrique(gm.motif_tournant(NUIT, PIECES, w, 120, 30), x, y, w, 120)}</g>')
    out.append(f'<circle cx="{x + 70}" cy="{y + 122}" r="44" fill="{BLANC}"/>')
    out.append(f'<defs><clipPath id="{i}a"><circle cx="{x + 70}" cy="{y + 122}" r="40"/></clipPath></defs>')
    out.append(f'<g clip-path="url(#{i}a)">{rect(x + 30, y + 82, 80, 80, INDIGO)}{img("kadenz-symbole-petit-negatif.svg", x + 46, y + 98, 48, 48)}</g>')
    out.append(t(x + 28, y + 196, "Kadenz", 16, ENCRE, 700, police="Space Grotesk"))
    out.append(t(x + 28, y + 214, "Gestion commerciale", 9, ARDOISE))
    out.append(trois_temps(x + 28, y + 226, 14, 2.5, 4, INDIGO))
    out.append(rect(x + 28, y + 250, 300, 6, "#E3E6EC") + rect(x + 28, y + 264, 240, 6, "#E3E6EC"))
    out.append(rect(x + w - 120, y + 140, 92, 24, INDIGO, 3) + t(x + w - 74, y + 156, "[Bouton]", 8, BLANC, 600, "middle"))
    return out
