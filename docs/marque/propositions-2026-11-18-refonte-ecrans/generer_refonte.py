"""Refonte des écrans de création, d'inscription et de connexion : maquettes.

Propositions du 2026-11-18, à valider par l'utilisateur ; rien n'est encore
écrit en Slint. Chaque écran est rendu à la taille des captures actuelles
(1200 x 760 pour le poste, 1040 x 720 pour KadenzAdmin), en clair et en
sombre, puis la planche les met en regard des écrans actuels
(../ecrans-actuels-2026-11-18/ecrans/).

Textes : chaque texte raccourci vient d'un texte actuel de presentation.slint,
assistant.slint ou des captures ; la planche donne les deux en regard.
Couleurs : jetons de Theme (crates/app/ui/composants.slint) ; les valeurs
sombres hors charte sont marquées « écart à valider ».

Usage : py -3.13 generer_refonte.py   (Chrome requis ; Pillow pour les mesures)
"""
import math
import re
import subprocess
from pathlib import Path

from PIL import Image, ImageChops

ICI = Path(__file__).resolve().parent
MARQUE = ICI.parent
ACTUELS = "../ecrans-actuels-2026-11-18/ecrans"
CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
LARGEUR_PLANCHE = 2640

# Jetons : nom de Theme, clair (valeur actuelle), sombre (proposé), statut du sombre.
JETONS = [
    ("fond (blanc, aujourd'hui écrit en dur)", "#FFFFFF", "#1A2259", "charte : Nuit"),
    ("fond-lateral", "#F4F5F9", "#232C6B", "charte : pièces du motif"),
    ("nuit (panneaux de marque)", "#1A2259", "#151C4C", "écart à valider : se détache du fond Nuit"),
    ("accent (bouton principal)", "#2D3A8C", "#E3A23B", "charte : Ambre"),
    ("texte sur accent", "#FFFFFF", "#1A2259", "charte : Nuit"),
    ("accent-survol", "#E8EAF6", "#2D3A8C", "charte : Indigo"),
    ("champ (fond des saisies)", "#FFFFFF", "#232C6B", "charte : pièces du motif"),
    ("bordure (séparateurs)", "#D9DCE3", "#3A4795", "écart à valider"),
    ("bordure-champ (contours des champs, boutons secondaires)", "#7B8794", "#6E7BC9", "écart accepté le 2026-11-18 : 3 : 1 atteint sur blanc et sur fond-lateral (actuel #D9DCE3, 1,4 : 1)"),
    ("focus (champ actif, 2 px)", "#2D3A8C", "#E3A23B", "charte"),
    ("texte", "#1F2933", "#F4F5F9", "charte : Brume"),
    ("texte-secondaire", "#52606D", "#C5CAE0", "charte"),
    ("ambre", "#E3A23B", "#E3A23B", "inchangé"),
    ("erreur", "#B42318", "#F97066", "écart à valider"),
    ("attente", "#B54708", "#FDB022", "écart à valider"),
    ("succes", "#067647", "#47CD89", "écart à valider"),
]
T = {
    "clair": dict(fond="#FFFFFF", lateral="#F4F5F9", nuit="#1A2259", accent="#2D3A8C", sur_accent="#FFFFFF",
                  survol="#E8EAF6", champ="#FFFFFF", bordure="#D9DCE3", texte="#1F2933", second="#52606D",
                  ambre="#E3A23B", secondaire_bouton="#FFFFFF", logo="kadenz-logo-horizontal.svg", contour="#7B8794", focus="#2D3A8C"),
    "sombre": dict(fond="#1A2259", lateral="#232C6B", nuit="#1A2259", accent="#E3A23B", sur_accent="#1A2259",
                   survol="#2D3A8C", champ="#232C6B", bordure="#3A4795", texte="#F4F5F9", second="#C5CAE0",
                   ambre="#E3A23B", secondaire_bouton="#232C6B", logo="kadenz-logo-horizontal-negatif.svg", contour="#6E7BC9", focus="#E3A23B"),
}
NUIT, INDIGO, AMBRE, MOTIF = "#1A2259", "#2D3A8C", "#E3A23B", "#232C6B"


# ---------------------------------------------------------------- contrastes
def luminance(h):
    c = [int(h[i:i + 2], 16) / 255 for i in (1, 3, 5)]
    c = [v / 12.92 if v <= 0.03928 else ((v + 0.055) / 1.055) ** 2.4 for v in c]
    return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]


def contraste(a, b):
    la, lb = sorted((luminance(a), luminance(b)), reverse=True)
    return (la + 0.05) / (lb + 0.05)


# ---------------------------------------------------------------- trame
def dans_piece(nom, u, v):
    """Pièce du symbole dans le carré unité (u vers la droite, v vers le bas)."""
    if nom == "carre":
        return 0 <= u <= 1 and 0 <= v <= 1
    if nom == "quart":   # quart de disque, centre en bas à gauche
        return u >= 0 and v <= 1 and u * u + (1 - v) ** 2 <= 1
    if nom == "triangle":   # angle droit en bas à gauche
        return 0 <= u <= 1 and 0 <= v <= 1 and v >= u
    if nom == "disque":
        return (u - .5) ** 2 + (v - .5) ** 2 <= .25
    return False


def trame(w, h, piece, x0, y0, taille, pas=14, couleur=AMBRE, fond_points=INDIGO, lumiere=(0.15, 0.1)):
    """Panneau en trame de points : une pièce du symbole, en grands points ambre
    qui grossissent loin de la lumière, sur un champ de petits points indigo."""
    lx, ly = lumiere[0] * w, lumiere[1] * h
    diag = math.hypot(w, h)
    pts_p, pts_f = [], []
    j = 0
    y = pas / 2
    while y < h + pas:
        x = pas / 2 + (pas / 2 if j % 2 else 0)
        while x < w + pas:
            d = math.hypot(x - lx, y - ly) / diag
            u, v = (x - x0) / taille, (y - y0) / taille
            if dans_piece(piece, u, v):
                r = pas * 0.5 * min(1, max(0.42, 0.3 + d * 1.2))
                pts_p.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{r:.2f}"/>')
            else:
                r = pas * 0.16 * min(1, max(0.25, d * 1.4))
                pts_f.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{r:.2f}"/>')
            x += pas
        y += pas * 0.866
        j += 1
    return (f'<svg width="{w}" height="{h}" viewBox="0 0 {w} {h}" aria-hidden="true" style="display:block">'
            f'<g fill="{fond_points}">{"".join(pts_f)}</g><g fill="{couleur}">{"".join(pts_p)}</g></svg>')


def logo(theme, hauteur=30):
    s = (MARQUE / T[theme]["logo"]).read_text(encoding="utf-8")
    return s.replace("<svg ", f'<svg height="{hauteur}" style="display:block" ', 1)


def logo_negatif(hauteur=30):
    s = (MARQUE / "kadenz-logo-horizontal-negatif.svg").read_text(encoding="utf-8")
    return s.replace("<svg ", f'<svg height="{hauteur}" style="display:block" ', 1)


# ---------------------------------------------------------------- composants
def champ(libelle, valeur="", aide="", haut=40, mdp=False, actif=False):
    v = ("&#8226;" * len(valeur)) if (mdp and valeur) else valeur
    a = f'<div class="aide">{aide}</div>' if aide else ""
    return (f'<label class="champ"><span>{libelle}</span>'
            f'<div class="saisie{" actif" if actif else ""}" style="height:{haut}px">{v}</div>{a}</label>')


def paire(a, b):
    return f'<div class="paire">{a}{b}</div>'


def boutons(*b):
    out = []
    for texte, genre in b:
        out.append(f'<div class="bouton {genre}">{texte}</div>')
    return "".join(out)


def css(theme, w, h):
    c = T[theme]
    return f"""
@font-face{{font-family:Inter;src:url(../planches/polices/Inter.woff2) format("woff2");font-weight:100 900}}
@font-face{{font-family:"Space Grotesk";src:url(../planches/polices/SpaceGrotesk.ttf) format("truetype");font-weight:300 700}}
*{{box-sizing:border-box;margin:0;padding:0}}
em,i{{font-style:normal}}
html,body{{width:{w}px;height:{h}px;overflow:hidden}}
body{{background:{c['fond']};color:{c['texte']};font:15px/1.45 Inter,sans-serif;position:relative}}
.titre{{font:700 32px/1.15 "Space Grotesk",sans-serif;letter-spacing:-.01em}}
.sur{{font:600 12px Inter;letter-spacing:.14em;text-transform:uppercase;color:{c['second']}}}
.phrase{{color:{c['second']};font-size:16px}}
.champ{{display:block}}
.champ span{{display:block;white-space:nowrap;font-size:13px;color:{c['second']};margin-bottom:6px}}
.saisie{{background:{c['champ']};border:1px solid {c['contour']};border-radius:4px;padding:9px 12px;
        font-size:15px;color:{c['texte']};overflow:hidden}}
.aide{{font-size:12.5px;color:{c['second']};margin-top:5px}}
.paire{{display:grid;grid-template-columns:1fr 1fr;gap:16px}}
.pile>*+*{{margin-top:16px}}
.bouton{{height:40px;padding:0 20px;border-radius:4px;display:inline-flex;align-items:center;justify-content:center;
        font:600 14.5px Inter;white-space:nowrap}}
.principal{{background:{c['accent']};color:{c['sur_accent']}}}
.second{{background:{c['secondaire_bouton']};color:{c['texte']};border:1px solid {c['contour']}}}
.saisie.actif{{border:2px solid {c['focus']};padding:8px 11px}}
.lien{{color:{c['texte']};padding:0 4px}}
.plein{{display:flex;width:100%}}
.barre{{display:flex;gap:8px}}
.barre i{{display:block;width:44px;height:4px;background:{c['bordure']}}}
.barre i.on{{background:{c['ambre'] if theme == 'sombre' else c['accent']}}}
.pied-marque{{font:700 22px "Space Grotesk";color:#FFFFFF}}
.version{{font-size:12.5px;color:#C5CAE0;margin-top:6px}}
.case{{display:flex;gap:10px;align-items:center;font-size:14px}}
.case b{{width:18px;height:18px;border-radius:3px;background:{c['accent']};display:inline-flex;align-items:center;
        justify-content:center;color:{c['sur_accent']};font-size:12px}}
.repli{{display:flex;align-items:center;justify-content:space-between;padding:10px 14px;border:1px solid {c['bordure']};
       border-radius:4px;background:{c['lateral']};font-size:14px}}
.repli small{{color:{c['second']};font-size:13px}}
.recap{{display:grid;grid-template-columns:170px 1fr;row-gap:10px;column-gap:20px;font-size:14.5px}}
.recap dt{{color:{c['second']}}}
"""


def page(theme, w, h, corps):
    return (f'<!doctype html><html lang="fr"><head><meta charset="utf-8"><style>{css(theme, w, h)}</style></head>'
            f'<body>{corps}</body></html>')


def bloc(x, y, largeur, contenu, classe=""):
    return f'<div class="bloc {classe}" style="position:absolute;left:{x}px;top:{y}px;width:{largeur}px">{contenu}</div>'


# ---------------------------------------------------------------- écrans du poste (1200 x 760)
W, H = 1200, 760
IMAGE_X, IMAGE_W = 560, 640      # panneau image à droite
TEXTE_X, TEXTE_Y, TEXTE_W = 72, 210, 416


def panneau_image(piece, theme, x=IMAGE_X, w=IMAGE_W, pied=True, px=150, py=150, taille=400):
    fond = NUIT if theme == "clair" else "#151C4C"
    t = trame(w, H, piece, px, py, taille)
    p = ""
    if pied:
        p = (f'<div style="position:absolute;left:48px;bottom:44px"><div class="pied-marque">Gestion commerciale</div>'
             f'<div class="version">Version 1.0.0</div></div>')
    return (f'<div style="position:absolute;left:{x}px;top:0;width:{w}px;height:{H}px;background:{fond};overflow:hidden">'
            f'{t}{p}</div>')


def connexion(theme):
    contenu = (f'<div class="sur">Connexion</div>'
               f'<div class="titre" style="margin-top:14px">Quincaillerie Démo</div>'
               f'<div class="pile" style="margin-top:32px">{champ("Identifiant", actif=True)}{champ("Mot de passe")}</div>'
               f'<div style="margin-top:24px">{boutons(("Se connecter", "principal plein"))}</div>')
    corps = (f'<div style="position:absolute;left:{TEXTE_X}px;top:56px">{logo(theme)}</div>'
             + bloc(TEXTE_X, TEXTE_Y + 40, 360, contenu) + panneau_image("quart", theme))
    return page(theme, W, H, corps)


PRESENTATION = [
    ("carre", "Bienvenue dans Kadenz", "La facturation de votre entreprise, en FCFA et en sept langues."),
    ("quart", "Vos documents", "Du devis à l'avoir : chaque document validé reçoit son numéro et s'exporte en PDF."),
    ("triangle", "Vos paiements", "Saisissez un règlement sur la facture : l'état Impayé, Partiel ou Payé se calcule seul."),
    ("disque", "Vos données", "Chacun se connecte avec son propre compte ; chaque validation est inscrite dans l'historique."),
]


def presentation(theme, n):
    piece, titre, phrase = PRESENTATION[n]
    barre = "".join(f'<i class="{"on" if k == n else ""}"></i>' for k in range(4))
    contenu = (f'<div class="barre">{barre}</div>'
               f'<div class="sur" style="margin-top:22px">{n + 1} sur 4</div>'
               f'<div class="titre" style="margin-top:12px">{titre}</div>'
               f'<div class="phrase" style="margin-top:16px">{phrase}</div>')
    if n == 0:
        bas = boutons(("Passer", "lien"), ("Suivant", "principal"))
    elif n < 3:
        bas = boutons(("Précédent", "second")) + '<div style="flex:1"></div>' + boutons(("Passer", "lien"), ("Suivant", "principal"))
    else:
        bas = boutons(("Précédent", "second")) + '<div style="flex:1"></div>' + boutons(("Rejoindre l'entreprise", "principal"))
    if n == 0:
        bas = '<div style="flex:1"></div>' + bas
    corps = (f'<div style="position:absolute;left:{TEXTE_X}px;top:56px">{logo(theme)}</div>'
             + bloc(TEXTE_X, TEXTE_Y, TEXTE_W, contenu)
             + f'<div style="position:absolute;left:{TEXTE_X}px;width:{TEXTE_W}px;top:{H - 96}px;display:flex;gap:10px">{bas}</div>'
             + panneau_image(piece, theme))
    return page(theme, W, H, corps)


def inscription(theme):
    # Modèle 2 : panneau image à gauche, formulaire à droite.
    contenu = (f'<div class="sur">Premier lancement</div>'
               f'<div class="titre" style="margin-top:14px">Rejoindre l\'entreprise</div>'
               f'<div class="phrase" style="margin-top:14px">Demandez un code d\'inscription à l\'Admin de l\'entreprise.</div>'
               f'<div class="pile" style="margin-top:28px">'
               f'{champ("Code d&#39;inscription", haut=64, aide="Code complet, ou code court XXXX-XXXX-XXXX avec l&#39;adresse du serveur.")}'
               f'<div style="display:grid;grid-template-columns:1fr auto;gap:10px;align-items:end">'
               f'{champ("Adresse du serveur (code court seulement)")}{boutons(("Rechercher sur le réseau", "second"))}</div>'
               f'</div><div style="margin-top:24px">{boutons(("Inscrire ce poste", "principal"))}</div>')
    corps = (panneau_image("disque", theme, x=0, w=500, px=70, py=150, taille=360)
             + f'<div style="position:absolute;left:48px;top:56px">{logo_negatif()}</div>'
             + bloc(580, TEXTE_Y, 480, contenu))
    return page(theme, W, H, corps)


# ---------------------------------------------------------------- assistant (1040 x 720)
AW, AH = 1040, 720
ETAPES = ["Licence", "Entreprise", "Administrateur", "Taux et paiements", "Sauvegardes", "Récapitulatif"]
A_X, A_Y, A_W = 300, 72, 620


def colonne_etapes(n, theme):
    lignes = []
    for k, e in enumerate(ETAPES):
        if k < n:
            marque = f'<b style="color:{AMBRE}">&#10003;</b>'
            couleur = "#C5CAE0"
        elif k == n:
            marque = f'<b style="display:inline-block;width:8px;height:8px;background:{AMBRE}"></b>'
            couleur = "#FFFFFF"
        else:
            marque = '<b style="display:inline-block;width:8px"></b>'
            couleur = "#8C94C2"
        poids = 600 if k == n else 400
        lignes.append(f'<div style="display:flex;gap:12px;align-items:center;height:34px;color:{couleur};'
                      f'font-weight:{poids};font-size:14.5px"><span style="width:12px;text-align:center">{marque}</span>{e}</div>')
    fond = NUIT if theme == "clair" else "#151C4C"
    return (f'<div style="position:absolute;left:0;top:0;width:232px;height:{AH}px;background:{fond};overflow:hidden">'
            f'<div style="position:absolute;inset:0;opacity:.9">{trame(232, AH, "triangle", -40, 420, 330, pas=12)}</div>'
            f'<div style="position:absolute;left:32px;top:44px">{logo_negatif(26)}</div>'
            f'<div style="position:absolute;left:28px;top:132px">'
            f'<div class="sur" style="color:#C5CAE0;margin:0 0 14px 4px">Première installation</div>{"".join(lignes)}</div></div>')


def assistant(theme, n, variante=""):
    titre = ETAPES[n]
    if n == 0:
        corps_etape = (f'<div class="phrase">Transmettez cet identifiant pour recevoir la licence, puis collez-la ici. '
                       f'Sans licence, le serveur reste en lecture seule et un seul poste peut s&#39;inscrire.</div>'
                       f'<div class="pile" style="margin-top:24px">'
                       f'<div style="display:grid;grid-template-columns:1fr auto;gap:10px;align-items:end">'
                       f'{champ("Identifiant de l&#39;entreprise", "01a1ec2c-5354-76c7-a313-788688…")}{boutons(("Copier", "second"))}</div>'
                       f'{champ("Clé de licence", haut=88, aide="Vous pourrez aussi l&#39;installer plus tard.")}</div>')
    elif n == 1:
        visibles = (champ("Nom de l'entreprise (obligatoire)", "Quincaillerie Démo")
                    + paire(champ("Téléphone"), champ("Email")) + paire(champ("RCCM"), champ("IFU")))
        if variante == "adresse":
            visibles = (champ("Nom de l'entreprise (obligatoire)", "Quincaillerie Démo")
                        + champ("Adresse / boîte postale")
                        + paire(champ("Téléphone"), champ("Email")) + paire(champ("RCCM"), champ("IFU")))
        if variante == "ouvert":
            repli = (f'<div class="repli"><span>Plus d\'informations</span><small>replier</small></div>'
                     + paire(champ("Adresse / boîte postale"), champ("Site web"))
                     + paire(champ("Localisation"), champ("Régime d'imposition"))
                     + paire(champ("Compte bancaire 1"), champ("Compte bancaire 2"))
                     + paire(champ("Responsable facturation"), champ("Responsable comptabilité")))
        else:
            reste = 7 if variante == "adresse" else 8
            repli = (f'<div class="repli"><span>Plus d\'informations</span><small>{reste} champs facultatifs</small></div>'
                     f'<div class="aide" style="margin-top:8px">Tout se modifie plus tard dans les Paramètres.</div>')
        corps_etape = f'<div class="pile">{visibles}{repli}</div>'
    elif n == 2:
        corps_etape = (f'<div class="phrase">Ce compte administre l\'installation : utilisateurs, droits, sauvegardes.</div>'
                       f'<div class="pile" style="margin-top:24px">'
                       f'{paire(champ("Nom complet", "Awa Traore"), champ("Identifiant de connexion", "admin"))}'
                       f'{paire(champ("Mot de passe", "motdepasse123", mdp=True), champ("Confirmation", "motdepasse123", mdp=True))}'
                       f'<div class="aide" style="margin-top:6px">8 caractères au moins.</div></div>')
    elif n == 3:
        corps_etape = (f'<div class="phrase">En pourcentage, séparés par des points-virgules. Modifiables plus tard.</div>'
                       f'<div class="pile" style="margin-top:24px">'
                       f'<div style="display:grid;grid-template-columns:1fr 1fr 1fr;gap:16px">'
                       f'{champ("TVA (%)", "0 ; 18")}{champ("Remise (%)", "0 ; 5 ; 10 ; 15")}{champ("Retenue (%)", "0 ; 5 ; 10 ; 15")}</div>'
                       f'{champ("Modes de paiement", "Espèce ; Chèque ; Virement bancaire ; Carte bancaire ; Moov money ; Orange money")}'
                       f'<div class="case"><b>&#10003;</b>Montant en lettres en orthographe traditionnelle</div></div>')
    elif n == 4:
        corps_etape = (f'<div class="phrase">Sauvegarde chiffrée chaque jour et à l\'arrêt du service. Sans ce mot de passe, '
                       f'aucune sauvegarde ne peut être restaurée, et l&#39;éditeur ne peut pas le retrouver : notez-le en lieu sûr.</div>'
                       f'<div class="pile" style="margin-top:24px">'
                       f'<div style="display:grid;grid-template-columns:1fr auto;gap:10px;align-items:end">'
                       f'{champ("Dossier des sauvegardes", "C:&#92;ProgramData&#92;Kadenz&#92;Sauvegardes")}{boutons(("Choisir", "second"))}</div>'
                       f'<div class="aide" style="margin-top:4px">De préférence sur un autre disque (externe ou partage réseau).</div>'
                       f'{champ("Heure de la sauvegarde quotidienne", "12:00")}'
                       f'{paire(champ("Mot de passe des sauvegardes", "motdepasse123", mdp=True), champ("Confirmation", "motdepasse123", mdp=True))}</div>')
    else:
        lignes = [("Licence", "aucune pour l'instant, lecture seule"), ("Entreprise", "Quincaillerie Démo"),
                  ("Administrateur", "Awa Traore (admin)"), ("Taux", "TVA 0 ; 18, remise et retenue 0 ; 5 ; 10 ; 15"),
                  ("Paiements", "Espèce, Chèque, Virement, Carte, Moov money, Orange money"),
                  ("Montant en lettres", "orthographe traditionnelle"),
                  ("Sauvegardes", "chaque jour à 12:00, C:&#92;ProgramData&#92;Kadenz&#92;Sauvegardes")]
        corps_etape = ('<dl class="recap">' + "".join(f"<dt>{a}</dt><dd>{b}</dd>" for a, b in lignes) + "</dl>")
    contenu = f'<div class="titre">{titre}</div><div style="margin-top:20px">{corps_etape}</div>'
    if n == 0:
        bas = '<div style="flex:1"></div>' + boutons(("Suivant", "principal"))
    elif n < 5:
        bas = boutons(("Précédent", "second")) + '<div style="flex:1"></div>' + boutons(("Suivant", "principal"))
    else:
        bas = boutons(("Précédent", "second")) + '<div style="flex:1"></div>' + boutons(("Terminer l'installation", "principal"))
    c = T[theme]
    defile = ""
    if variante == "ouvert":   # déplié, la zone défile ; les boutons restent fixes
        defile = (f'<div style="position:absolute;left:{A_X + A_W + 18}px;top:{A_Y}px;width:4px;height:{AH - 100 - A_Y - 16}px;'
                  f'background:{c["lateral"]};border-radius:2px"><div style="height:58%;background:{c["bordure"]};border-radius:2px"></div></div>')
    corps = (colonne_etapes(n, theme)
             + f'<div style="position:absolute;left:{A_X}px;top:{A_Y}px;width:{A_W}px;height:{AH - 100 - A_Y}px;overflow:hidden">'
             + bloc(0, 0, A_W, contenu) + '</div>' + defile
             + f'<div style="position:absolute;left:232px;right:0;top:{AH - 100}px;height:100px;background:{c["lateral"]}"></div>'
             + f'<div style="position:absolute;left:{A_X}px;width:{A_W}px;top:{AH - 72}px;display:flex;gap:10px">{bas}</div>')
    return page(theme, AW, AH, corps)


# ---------------------------------------------------------------- liste des écrans
def ecrans():
    e = []
    for th in ("clair", "sombre"):
        e.append((f"connexion-{th}", connexion(th), W, H))
        e.append((f"inscription-{th}", inscription(th), W, H))
        for n in range(4):
            e.append((f"presentation-{n + 1}-{th}", presentation(th, n), W, H))
    for n in range(6):
        e.append((f"assistant-{n + 1}-clair", assistant("clair", n), AW, AH))
    e.append(("assistant-2-sombre", assistant("sombre", 1), AW, AH))
    e.append(("assistant-2-sombre-ouvert", assistant("sombre", 1, "ouvert"), AW, AH))
    e.append(("assistant-2-clair-adresse", assistant("clair", 1, "adresse"), AW, AH))
    return e


def chrome(*args):
    return subprocess.run([CHROME, "--headless=new", "--hide-scrollbars", "--virtual-time-budget=5000", *args],
                          check=True, capture_output=True, text=True, encoding="utf-8")


def rendre():
    dossier = ICI / "ecrans"
    dossier.mkdir(exist_ok=True)
    for nom, html, w, h in ecrans():
        tmp = ICI / f"_{nom}.html"
        tmp.write_text(html, encoding="utf-8")
        chrome("--force-device-scale-factor=1", f"--window-size={w},{h}", f"--screenshot={dossier / (nom + '.png')}",
               tmp.as_uri())
        tmp.unlink()
        print(nom)


def boite(png, x0, x1, bas, y0=0):
    """Boîte des contenus d'un écran, comme pour les écrans actuels : pixels qui
    s'écartent du fond entre x0 et x1, sous y0 (logo exclu), boutons du bas exclus."""
    im = Image.open(png).convert("RGB")
    w, h = im.size
    zone = im.crop((x0, y0, x1, h - bas))
    fond = Image.new("RGB", zone.size, zone.getpixel((zone.width - 1, zone.height - 1)))
    diff = ImageChops.difference(zone, fond).convert("L").point(lambda v: 255 if v > 20 else 0)
    bb = diff.getbbox()
    return (bb[0] + x0, bb[1] + y0, bb[2] + x0, bb[3] + y0) if bb else None


def mesures():
    d = ICI / "ecrans"
    pres = {th: [boite(d / f"presentation-{n}-{th}.png", 0, IMAGE_X, 110, y0=120) for n in range(1, 5)]
            for th in ("clair", "sombre")}
    assi = [boite(d / f"assistant-{n}-clair.png", 240, AW, 101) for n in range(1, 7)]
    return pres, assi


# ---------------------------------------------------------------- planche
TEXTES_PRESENTATION = [
    ("Bienvenue dans Kadenz",
     "Kadenz tient la facturation de votre entreprise sur ce poste : clients, fournisseurs, produits et documents "
     "commerciaux, montants en FCFA. L'interface existe en sept langues ; chaque compte choisit la sienne. (31 mots)"),
    ("Vos documents",
     "Devis, proformas, bons de commande, bordereaux de livraison, factures et avoirs se préparent dans le même écran. "
     "Un document commence en brouillon et reçoit son numéro à la validation ; [...] Chaque document s'exporte en PDF "
     "à l'identité de votre entreprise. (65 mots)"),
    ("Vos paiements",
     "Les règlements se saisissent dans la fiche d'une facture ou d'un bon de commande ; l'état Impayé, Partiel ou Payé "
     "se calcule tout seul. [...] (56 mots)"),
    ("Vos données",
     "Les données restent sur ce poste, dans une base locale. Chacun se connecte avec son propre compte ; [...] "
     "Validations, annulations et règlements sont inscrits dans l'historique de chaque document. (45 mots), puis "
     "« L'assistant qui suit crée votre entreprise et le compte administrateur. »"),
]
TEXTES_AUTRES = [
    ("Connexion",
     "Saisissez votre identifiant et votre mot de passe.",
     "(supprimé : les deux libellés suffisent)"),
    ("Inscription",
     "Ce poste travaille sur le serveur de l'entreprise. Demandez un code d'inscription à l'Admin : il le crée dans la "
     "fenêtre du serveur, ou depuis un poste déjà inscrit. / Inscription de ce poste / Collez le code complet, ou tapez "
     "le code court (XXXX-XXXX-XXXX) et indiquez l'adresse du serveur.",
     "Demandez un code d'inscription à l'Admin de l'entreprise. / Aide sous le champ : Code complet, ou code court "
     "XXXX-XXXX-XXXX avec l'adresse du serveur."),
    ("Assistant, licence",
     "La licence d'abonnement est liée à l'entreprise. Transmettez l'identifiant ci-dessous pour la recevoir, puis "
     "collez-la ici. Sans licence, le serveur reste en lecture seule et un seul poste peut s'inscrire, pour installer "
     "la licence plus tard depuis ce poste.",
     "Transmettez cet identifiant pour recevoir la licence, puis collez-la ici. Sans licence, le serveur reste en "
     "lecture seule et un seul poste peut s'inscrire. / Aide : Vous pourrez aussi l'installer plus tard. "
     "(Perdu : « liée à l'entreprise ».)"),
    ("Assistant, taux",
     "Valeurs séparées par des points-virgules, en pourcentage (ex. 0 ; 18). Modifiables plus tard dans les paramètres.",
     "En pourcentage, séparés par des points-virgules. Modifiables plus tard."),
    ("Assistant, sauvegardes",
     "Le serveur sauvegarde sa base chaque jour à l'heure choisie, et à l'arrêt du service. Les sauvegardes sont "
     "chiffrées par ce mot de passe : sans lui, aucune ne peut être restaurée, et l'éditeur ne peut pas le retrouver. "
     "Notez-le en lieu sûr. Choisissez de préférence un dossier sur un autre disque que celui du serveur (disque "
     "externe ou partage réseau).",
     "Sauvegarde chiffrée chaque jour et à l'arrêt du service. Sans ce mot de passe, aucune sauvegarde ne peut être "
     "restaurée, et l'éditeur ne peut pas le retrouver : notez-le en lieu sûr. / Aide sous le dossier : De préférence "
     "sur un autre disque (externe ou partage réseau)."),
]


def img(nom, legende, actuel=False):
    src = f"{ACTUELS}/{nom}.png" if actuel else f"ecrans/{nom}.png"
    cls = "fig actuel" if actuel else "fig"
    return f'<figure class="{cls}"><img src="{src}" alt="{legende}"><figcaption>{legende}</figcaption></figure>'


def notes(*l):
    return '<ul class="notes">' + "".join(f"<li>{x}</li>" for x in l) + "</ul>"


def section(num, titre, ou, corps):
    return (f'<section><header><span class="num">{num}</span><h2>{titre}</h2><p class="ou">{ou}</p></header>'
            f'{corps}</section>')


def tableau_textes(lignes, entete=("Écran", "Texte actuel", "Texte proposé")):
    tr = "".join(f"<tr><td><b>{a}</b></td><td>{b}</td><td>{c}</td></tr>" for a, b, c in lignes)
    return f'<table class="textes"><tr><th>{entete[0]}</th><th>{entete[1]}</th><th>{entete[2]}</th></tr>{tr}</table>'


def planche(pres, assi):
    # Jetons et contrastes
    lignes = []
    for nom, cl, so, statut in JETONS:
        lignes.append(f'<tr><td>{nom}</td><td><span class="pastille" style="background:{cl}"></span>{cl}</td>'
                      f'<td><span class="pastille" style="background:{so}"></span>{so}</td><td>{statut}</td></tr>')
    paires_c = [("texte sur fond", "#F4F5F9", "#1A2259"), ("texte-secondaire sur fond", "#C5CAE0", "#1A2259"),
                ("texte-secondaire sur champ", "#C5CAE0", "#232C6B"), ("texte du bouton principal (Nuit sur Ambre)", "#1A2259", "#E3A23B"),
                ("erreur sur fond", "#F97066", "#1A2259"), ("attente sur fond", "#FDB022", "#1A2259"),
                ("succes sur fond", "#47CD89", "#1A2259"), ("bordure-champ sur fond (3 : 1 requis)", "#6E7BC9", "#1A2259"), ("bordure-champ sur champ", "#6E7BC9", "#232C6B"),
                ("focus ambre sur fond", "#E3A23B", "#1A2259"), ("fond du champ sur fond (ne compense pas)", "#232C6B", "#1A2259"),
                ("séparateur #3A4795 sur fond (décoratif)", "#3A4795", "#1A2259"), ("bordure-champ claire sur blanc", "#7B8794", "#FFFFFF"), ("bordure-champ claire sur fond-lateral", "#7B8794", "#F4F5F9"),
                ("rappel clair : bordure actuelle sur blanc", "#D9DCE3", "#FFFFFF"),
                ("rappel clair : texte-secondaire sur blanc", "#52606D", "#FFFFFF")]
    ctr = "".join(f"<tr><td>{a}</td><td>{contraste(b, c):.1f} : 1</td></tr>" for a, b, c in paires_c)
    jetons = (f'<div class="deux-col"><table class="jetons"><tr><th>Jeton de Theme</th><th>Clair (actuel)</th>'
              f'<th>Sombre (proposé)</th><th>Origine du sombre</th></tr>{"".join(lignes)}</table>'
              f'<div><table class="jetons"><tr><th>Contraste en sombre</th><th>Rapport</th></tr>{ctr}</table>'
              + notes("Seuil courant : 4,5 : 1 pour le texte, 3 : 1 pour les repères graphiques.",
                      "Le bouton principal passe d'Indigo à Ambre en sombre : Indigo sur Nuit ne se distingue pas assez.",
                      "Aperçu des documents et PDF : restent blancs (c'est du papier) ; seul l'habillage s'assombrit.",
                      "Environ 27 couleurs écrites en dur hors de Theme seront à passer par ces jetons (relevé de kadenz-f8).")
              + "</div></div>")

    def fmt(b):
        return f"haut {b[1]} px, gauche {b[0]} px" if b else "?"
    p_top = sorted({b[1] for b in pres["clair"] + pres["sombre"]})
    p_left = sorted({b[0] for b in pres["clair"] + pres["sombre"]})
    a_top = sorted({b[1] for b in assi})
    a_left = sorted({b[0] for b in assi})
    mesure_txt = (f"Présentation proposée, 8 écrans : haut du bloc {', '.join(map(str, p_top))} px, bord gauche "
                  f"{', '.join(map(str, p_left))} px, largeur fixée à {TEXTE_W} px (actuel : haut de 252 à 270 px). "
                  f"Assistant proposé, 6 étapes : haut {', '.join(map(str, a_top))} px, bord gauche "
                  f"{', '.join(map(str, a_left))} px, largeur fixée à {A_W} px (actuel : bord gauche de 304 à 462 px, "
                  f"largeur de 356 à 672 px, haut de 90 à 228 px).")

    s0 = section("00", "Mode sombre : jetons", "Chaque couleur porte le nom du jeton de Theme qu'elle remplace", jetons)

    s1 = section("01", "Connexion", "Modèle 1 : fond sombre, formulaire court à gauche, grande trame à droite",
                 '<div class="rang trois">' + img("connexion", "Actuel", True) + img("connexion-clair", "Proposé, clair")
                 + img("connexion-sombre", "Proposé, sombre") + "</div>"
                 + notes("Le titre reste le nom de l'entreprise (ici celui de la base d'essai).",
                         "L'image est une trame générée : une pièce du symbole (le quart) en points ambre sur un champ "
                         "de points indigo. Le symbole entier n'est pas tramé : il ne s'altère jamais.",
                         "Une vraie photo peut remplacer la trame plus tard, si vous en fournissez une.",
                         "Champs, bouton et ordre inchangés : identifiant, mot de passe, Se connecter."))

    s2 = section("02", "Inscription du poste", "Modèle 2 : panneau image à gauche, formulaire à droite",
                 '<div class="rang trois">' + img("inscription-poste", "Actuel", True)
                 + img("inscription-clair", "Proposé, clair") + img("inscription-sombre", "Proposé, sombre") + "</div>"
                 + notes("Une phrase au lieu de trois ; le sous-titre « Inscription de ce poste » disparaît.",
                         "Fonctionnement inchangé : code complet ou code court, adresse du serveur pour le code court, "
                         "recherche sur le réseau.",
                         "Le bloc commence à la même hauteur (210 px) que celui de la présentation."))

    pres_rang = ('<div class="rang quatre">' + "".join(img(f"p{n}-{s}", f"Actuel {n} sur 4", True) for n, s in
                 [(1, "bienvenue"), (2, "vos-documents"), (3, "vos-paiements"), (4, "vos-donnees")]) + "</div>"
                 + '<div class="rang quatre">' + "".join(img(f"presentation-{n}-clair", f"Proposé {n} sur 4, clair")
                                                          for n in range(1, 5)) + "</div>"
                 + '<div class="rang quatre">' + "".join(img(f"presentation-{n}-sombre", f"Proposé {n} sur 4, sombre")
                                                          for n in range(1, 5)) + "</div>")
    s3 = section("03", "Présentation", "Quatre écrans, une phrase chacun, bloc ancré en haut",
                 pres_rang
                 + notes(f"<b>Bloc fixe, mesuré :</b> {mesure_txt.split(' Assistant')[0]}",
                         "Une pièce du symbole par page, au même endroit : carré, quart, triangle, disque.",
                         "Les boutons restent à la même place sur les quatre pages.",
                         "Dernier bouton : « Rejoindre l'entreprise », parce qu'en 1.0 le poste passe ensuite à "
                         "l'inscription (main.rs). En monoposte, ce serait « Commencer l'installation » : à décider.")
                 + tableau_textes([(a, b, PRESENTATION[k][2]) for k, (a, b) in enumerate(TEXTES_PRESENTATION)]))

    assist = ('<div class="rang trois">' + "".join(img(f"a{n}-{s}", f"Actuel, {e}", True) for n, s, e in
              [(1, "licence", "licence"), (2, "entreprise", "entreprise"), (3, "administrateur", "administrateur")]) + "</div>"
              + '<div class="rang trois">' + "".join(img(f"assistant-{n}-clair", f"Proposé, {ETAPES[n - 1].lower()}")
                                                       for n in (1, 2, 3)) + "</div>"
              + '<div class="rang trois">' + "".join(img(f"a{n}-{s}", f"Actuel, {e}", True) for n, s, e in
              [(4, "taux-et-paiements", "taux et paiements"), (5, "sauvegardes", "sauvegardes"),
               (6, "recapitulatif", "récapitulatif")]) + "</div>"
              + '<div class="rang trois">' + "".join(img(f"assistant-{n}-clair", f"Proposé, {ETAPES[n - 1].lower()}")
                                                       for n in (4, 5, 6)) + "</div>"
              + '<div class="rang trois">' + img("assistant-2-sombre", "Entreprise, sombre, replié")
              + img("assistant-2-sombre-ouvert", "Entreprise, sombre, déplié")
              + img("assistant-2-clair-adresse", "Variante : adresse visible") + "</div>")
    s4 = section("04", "Assistant de création", "KadenzAdmin : même place et même largeur à toutes les étapes",
                 assist
                 + notes(f"<b>Bloc fixe, mesuré :</b> Assistant{mesure_txt.split(' Assistant')[1]}",
                         "Le titre est le nom de l'étape ; « Première installation » n'apparaît qu'une fois, au-dessus "
                         "de la liste ; « Étape n sur 6 » disparaît, la liste suffit (étapes faites cochées en ambre).",
                         "Entreprise : nom, téléphone et email, RCCM et IFU visibles ; les 8 autres champs sous "
                         "« Plus d'informations ». Proposition commune avec kadenz-f8.",
                         "Question : l'adresse / boîte postale, souvent dans l'en-tête des factures, reste-t-elle visible "
                         "(variante à droite) ou repliée ?",
                         "Paires sur une ligne : mots de passe et confirmations, TVA, remise et retenue côte à côte.")
                 + tableau_textes(TEXTES_AUTRES))

    s5 = section("05", "À décider", "Points qui reviennent à l'utilisateur",
                 notes("Les textes raccourcis, ligne par ligne (tableaux ci-dessus).",
                       "Adresse / boîte postale : visible ou repliée à l'étape Entreprise.",
                       "Image des écrans : trame générée aux couleurs de la marque, ou photos que vous fourniriez.",
                       "Les valeurs hors charte : panneaux #151C4C, séparateurs #3A4795, contours de champs #6E7BC9 en sombre et #7B8794 en clair (accepté) (l'actuel #D9DCE3 n'atteint que 1,4 : 1), couleurs d'état claires (erreur, attente, succes).",
                       "Textes raccourcis : chaque texte changé est une nouvelle chaîne à traduire dans les six autres langues ; en arabe, les écrans sont mis en miroir (panneau image de l'autre côté).",
                       "Bouton final de la présentation selon l'édition : multiposte « Rejoindre l'entreprise », "
                       "monoposte « Commencer l'installation ».",
                       "Choix du mode : suivre Windows, ou réglage par compte dans les Paramètres."))

    return f"""<!doctype html>
<html lang="fr"><head><meta charset="utf-8">
<title>Kadenz, refonte des écrans</title>
<meta name="robots" content="noindex">
<style>
@font-face{{font-family:Inter;src:url(../planches/polices/Inter.woff2) format("woff2");font-weight:100 900}}
@font-face{{font-family:"Space Grotesk";src:url(../planches/polices/SpaceGrotesk.ttf) format("truetype");font-weight:300 700}}
*{{box-sizing:border-box;margin:0;padding:0}}
em,i{{font-style:normal}}
body{{background:#F4F5F9;color:#1F2933;font:18px/1.5 Inter,sans-serif;width:{LARGEUR_PLANCHE}px;padding:96px 120px 120px}}
h1,h2{{font-family:"Space Grotesk",sans-serif;font-weight:700;letter-spacing:-.01em}}
.tete{{display:flex;justify-content:space-between;align-items:flex-end;padding-bottom:40px;border-bottom:1px solid #D9DCE3;margin-bottom:72px}}
.tete h1{{font-size:64px;line-height:1}}
.tete p{{color:#52606D;font-size:20px;text-align:right}}
.sur{{font:600 15px Inter;letter-spacing:.14em;text-transform:uppercase;color:#2D3A8C;margin-bottom:18px}}
section{{margin-bottom:104px}}
section header{{display:flex;align-items:baseline;gap:24px;margin-bottom:32px}}
.num{{font:700 22px "Space Grotesk";color:#E3A23B}}
section h2{{font-size:40px}}
.ou{{color:#52606D;font-size:20px}}
.rang{{display:grid;gap:28px;margin-bottom:28px}}
.quatre{{grid-template-columns:repeat(4,1fr)}}
.trois{{grid-template-columns:repeat(3,1fr)}}
.fig{{background:#fff;border:1px solid #D9DCE3;border-radius:4px;overflow:hidden}}
.fig img{{display:block;width:100%;height:auto}}
.fig figcaption{{padding:12px 18px 14px;font:600 15px Inter;color:#1F2933}}
.fig.actuel figcaption{{color:#52606D;font-weight:500}}
.notes{{list-style:none;margin-top:28px;display:grid;grid-template-columns:repeat(2,1fr);gap:14px 48px}}
.notes li{{position:relative;padding-left:28px;font-size:19px}}
.notes li::before{{content:"";position:absolute;left:0;top:11px;width:10px;height:10px;background:#2D3A8C}}
.notes b{{font-weight:600}}
table{{border-collapse:collapse;width:100%;font-size:16.5px;background:#fff;border:1px solid #D9DCE3}}
th{{text-align:left;font:600 13px Inter;letter-spacing:.1em;text-transform:uppercase;color:#52606D;padding:12px 16px;border-bottom:1px solid #D9DCE3;background:#F4F5F9}}
td{{padding:11px 16px;border-bottom:1px solid #E6E8EE;vertical-align:top}}
.textes{{margin-top:36px}}
.textes td:first-child{{width:220px}}
.textes td:nth-child(2){{color:#52606D;width:50%}}
.deux-col{{display:grid;grid-template-columns:1.35fr 1fr;gap:48px;align-items:start}}
.deux-col .notes{{grid-template-columns:1fr}}
.pastille{{display:inline-block;width:22px;height:22px;border-radius:3px;border:1px solid #D9DCE3;vertical-align:-5px;margin-right:12px}}
</style></head><body>
<div class="tete">
  <div><p class="sur">Kadenz, propositions du 2026-11-18</p><h1>Refonte : création, inscription, connexion</h1></div>
  <p>Maquettes à la taille des écrans actuels, en clair et en sombre<br>Rien n'est encore écrit en Slint</p>
</div>
{s0}{s1}{s2}{s3}{s4}{s5}
</body></html>"""


if __name__ == "__main__":
    rendre()
    pres, assi = mesures()
    print("presentation", pres)
    print("assistant", assi)
    html = ICI / "index.html"
    contenu = planche(pres, assi)
    html.write_text(contenu, encoding="utf-8")
    mesure = ICI / "_mesure.html"
    mesure.write_text(contenu.replace("</body>", "<script>document.fonts.ready.then(()=>document.body.setAttribute("
                                      "'data-h',document.documentElement.scrollHeight))</script></body>"),
                      encoding="utf-8")
    h = int(re.search(r'data-h="(\d+)"', chrome(f"--window-size={LARGEUR_PLANCHE},1000", "--dump-dom",
                                                 mesure.as_uri()).stdout).group(1))
    mesure.unlink()
    png = ICI / "planche-refonte.png"
    chrome("--force-device-scale-factor=1", f"--window-size={LARGEUR_PLANCHE},{h}", f"--screenshot={png}", html.as_uri())
    print(png, h)
