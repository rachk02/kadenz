"""Identité du portail des licences et pages de connexion, propositions du
2026-11-19 (kadenz-3a), d'après les 7 images de référence de l'utilisateur :
planche de typographie et couleurs (violet, Nuit violette, orange d'accent),
deux pages de connexion (A : cadre sombre, illustration en points ; B : carte,
visuel à gauche), affiches d'une marque tierce dont seul l'esprit est repris
(aplats, arches, étiquettes). Aucun logo, texte ni photo des références.

Les textes des pages sont ceux du portail (crates/portail/src/pages/connexion.rs,
web.rs, lib.rs). Le logo est le symbole Kadenz (docs/marque). Pages
compatibles avec la CSP du portail : aucun script, aucun style en ligne,
aucune image en data:, SVG en ligne seulement.

Usage : py -3.13 generer_identite.py   (Chrome requis pour les captures)
"""
import random
import re
import subprocess
from pathlib import Path

ICI = Path(__file__).resolve().parent
MARQUE = ICI.parent
CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"

# ----- Symbole et mot Kadenz, en currentColor -----


def chemins(fichier):
    texte = (MARQUE / fichier).read_text(encoding="utf-8")
    boite = re.search(r'viewBox="([^"]+)"', texte).group(1)
    corps = re.sub(r"<title>.*?</title>", "", texte.split(">", 1)[1].rsplit("</svg>", 1)[0])
    corps = re.sub(r'fill="[^"]+"', 'fill="currentColor"', corps)
    return boite, corps.strip()


BOITE_SYM, CORPS_SYM = chemins("kadenz-symbole.svg")
BOITE_MOT, CORPS_MOT = chemins("kadenz-mot-negatif.svg")


def symbole(classe="symbole"):
    return (f'<svg class="{classe}" viewBox="{BOITE_SYM}" aria-hidden="true" '
            f'focusable="false">{CORPS_SYM}</svg>')


def mot(classe="mot"):
    return (f'<svg class="{classe}" viewBox="{BOITE_MOT}" role="img" aria-label="Kadenz">'
            f"{CORPS_MOT}</svg>")


# ----- Icônes des champs (trait de 1,5, grille de 24) -----

ICONES = {
    "identifiant": '<circle cx="12" cy="8" r="4"/><path d="M4 21c0-4 3.6-6 8-6s8 2 8 6"/>',
    "mot_de_passe": '<circle cx="8" cy="15" r="4"/><path d="M11 12l9-9M16 7l3 3M14 9l2 2"/>',
    "code": '<rect x="5" y="11" width="14" height="10" rx="1"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/>',
}


def icone(nom):
    return f'<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">{ICONES[nom]}</svg>'


# ----- Arches -----


def arche(x, bas, largeur, hauteur):
    """Arche : montants droits, demi-disque de rayon largeur / 2 en haut."""
    r = largeur / 2
    haut = bas - hauteur
    return (f"M{x:.1f},{bas:.1f} V{haut + r:.1f} "
            f"A{r:.1f},{r:.1f} 0 0 1 {x + largeur:.1f},{haut + r:.1f} V{bas:.1f} Z")


def dans_arche(px, py, x, bas, largeur, hauteur):
    r = largeur / 2
    haut = bas - hauteur
    if py > bas or px < x or px > x + largeur:
        return False
    if py >= haut + r:
        return True
    return (px - (x + r)) ** 2 + (py - (haut + r)) ** 2 <= r * r


def illustration_points():
    """Trois portails en points, à la manière des colonnes de la référence A :
    cadre dense, ouverture presque vide, socle large, poussière autour.
    Le disque orange, temps fort du symbole Kadenz, au-dessus du portail
    central."""
    alea = random.Random(20261119)
    w, h, pas = 760, 900, 7
    portails = [  # x, bas, largeur, hauteur, épaisseur du cadre
        (40, 860, 190, 700, 34),
        (300, 860, 170, 520, 30),
        (530, 860, 190, 720, 34),
    ]
    disque = (385, 280, 22)
    clairs, pales, oranges = [], [], []
    for gy in range(0, h, pas):
        for gx in range(0, w, pas):
            px = gx + alea.uniform(-1.2, 1.2)
            py = gy + alea.uniform(-1.2, 1.2)
            p = 0.006
            ton = pales
            for (x, bas, lg, ht, ep) in portails:
                if dans_arche(px, py, x, bas, lg, ht):
                    interieur = dans_arche(px, py, x + ep, bas, lg - 2 * ep, ht - ep)
                    if interieur:
                        p, ton = 0.035, pales
                    else:
                        # cannelures du cadre : bandes verticales plus denses
                        bande = int((px - x) / 6) % 3
                        p, ton = (0.82 if bande else 0.5), clairs
                    break
                # socle : un peu plus large que le portail, en bas
                if bas - 26 <= py <= bas + 14 and x - 18 <= px <= x + lg + 18:
                    p, ton = 0.6, clairs
                    break
            if (px - disque[0]) ** 2 + (py - disque[1]) ** 2 <= disque[2] ** 2:
                p, ton = 0.9, oranges
            # le haut s'efface, comme une pierre usée
            if ton is clairs and py < 260:
                p *= 0.55 + py / 580
            if alea.random() < p:
                rayon = 1.35 if ton is clairs else 1.1
                ton.append(f'<circle cx="{px:.1f}" cy="{py:.1f}" r="{rayon}"/>')
    return (f'<svg viewBox="0 0 {w} {h}" preserveAspectRatio="xMidYMax slice" aria-hidden="true" focusable="false">'
            f'<g fill="#5A3F80">{"".join(pales)}</g>'
            f'<g fill="#C9B3EE">{"".join(clairs)}</g>'
            f'<g fill="#F56A00">{"".join(oranges)}</g></svg>')


def arches_concentriques():
    """Visuel B : quatre arches concentriques, de la plus grande à la plus
    petite, Nuit, Violet, Nuit, Orange (même centre, anneaux de même
    épaisseur). Jamais deux arches croisées."""
    w, h, cx, cy = 600, 520, 300, 300
    formes = []
    for largeur, couleur in [(560, "#240046"), (420, "#7B2CBF"), (280, "#240046"), (140, "#F56A00")]:
        r = largeur / 2
        formes.append(f'<path fill="{couleur}" d="M{cx - r:.0f},{h} V{cy} '
                      f'A{r:.0f},{r:.0f} 0 0 1 {cx + r:.0f},{cy} V{h} Z"/>')
    return (f'<svg class="arches" viewBox="0 0 {w} {h}" preserveAspectRatio="xMidYMax meet" '
            f'aria-hidden="true" focusable="false">{"".join(formes)}</svg>')


# ----- Pages -----


def document(titre, classe, feuilles, corps):
    liens = "".join(f'<link rel="stylesheet" href="{f}">' for f in feuilles)
    return f"""<!DOCTYPE html>
<html lang="fr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="robots" content="noindex, nofollow">
<title>{titre} | Portail des licences Kadenz</title>
{liens}
</head>
<body class="{classe}">
{corps}
</body>
</html>
"""


DESCRIPTION = ("Enregistrement des paiements et émission des licences d'abonnement Kadenz, "
               "pour l'éditeur et ses revendeurs.")


def formulaire_connexion(bouton, erreur=False):
    message = ('<p class="message message-erreur" role="alert">identifiant ou mot de passe incorrect</p>'
               if erreur else "")
    return f"""<p class="surtitre">Connexion</p>
<h1>Portail des licences</h1>
<p class="texte">Saisissez votre identifiant et votre mot de passe.</p>
{message}
<form method="post" action="/connexion">
<input type="hidden" name="csrf" value="">
<div class="champ">
<label for="identifiant">Identifiant</label>
<div class="saisie">{icone("identifiant")}<input id="identifiant" type="text" name="identifiant" autocomplete="username" required autofocus></div>
</div>
<div class="champ">
<label for="mot_de_passe">Mot de passe</label>
<div class="saisie">{icone("mot_de_passe")}<input id="mot_de_passe" type="password" name="mot_de_passe" autocomplete="current-password" required></div>
</div>
<button class="{bouton}" type="submit">Se connecter</button>
</form>"""


def formulaire_code(bouton):
    return f"""<p class="surtitre">Double authentification</p>
<h1>Code de connexion</h1>
<p class="texte">Saisissez le code à 6 chiffres affiché par votre application d'authentification, ou l'un de vos codes de secours.</p>
<form method="post" action="/connexion/code">
<input type="hidden" name="etape" value="">
<div class="champ">
<label for="code">Code</label>
<div class="saisie">{icone("code")}<input id="code" class="code" type="text" name="code" autocomplete="one-time-code" spellcheck="false" required autofocus></div>
</div>
<button class="{bouton}" type="submit">Valider</button>
</form>
<p class="retour"><a href="/connexion">Revenir à la connexion</a></p>"""


POINTS = illustration_points()


def page_a(formulaire):
    return f"""<div class="cadre sombre">
<main class="colonne">
<div class="connexion-boite">
<div class="marque">{symbole()}</div>
{formulaire}
<div class="separateur" aria-hidden="true">kadenz</div>
<p class="pied">{DESCRIPTION}</p>
</div>
</main>
<div class="illustration" aria-hidden="true">{POINTS}</div>
</div>"""


def page_b(formulaire):
    return f"""<div class="carte">
<section class="visuel">
<div class="logo">{symbole()}{mot()}</div>
<p class="accroche">{DESCRIPTION}</p>
{arches_concentriques()}
</section>
<main class="formulaire sombre">
<div class="connexion-boite">
{formulaire}
</div>
</main>
</div>"""


FEUILLES = ["identite.css", "connexion.css"]
PAGES = {
    "connexion-a.html": ("Connexion", "connexion-a",
                         page_a(formulaire_connexion("bouton bouton-blanc bouton-plein"))),
    "connexion-a-erreur.html": ("Connexion", "connexion-a",
                                page_a(formulaire_connexion("bouton bouton-blanc bouton-plein", erreur=True))),
    "connexion-a-code.html": ("Code de connexion", "connexion-a",
                              page_a(formulaire_code("bouton bouton-blanc bouton-plein"))),
    "connexion-b.html": ("Connexion", "connexion-b",
                         page_b(formulaire_connexion("bouton bouton-accent bouton-plein"))),
    "connexion-b-code.html": ("Code de connexion", "connexion-b",
                              page_b(formulaire_code("bouton bouton-accent bouton-plein"))),
}


# ----- Planche d'identité -----

COULEURS = [
    ("Nuit violette", "#240046", "36 0 70", "Fonds de marque, bandeau ; texte sur clair.", "Blanc dessus 18,1:1", "nuit-violette", "grande"),
    ("Violet", "#7B2CBF", "123 44 191", "Boutons principaux, liens, chiffres sur clair.", "Sur blanc 7,1:1 ; sur Nuit 2,5:1, jamais en texte sur sombre", "violet", "haute"),
    ("Orange", "#F56A00", "245 106 0", "Accent : bouton de connexion B, étiquette active, temps fort.", "Sur blanc 3,0:1, jamais en petit texte sur clair ; Nuit dessus 6,0:1", "orange", ""),
    ("Abysse", "#170030", "23 0 48", "Fond des pages sombres (dérivé).", "Blanc dessus 19,5:1", "abysse", ""),
    ("Lavande", "#C9B3EE", "201 179 238", "Texte secondaire et points sur sombre (dérivé).", "Sur Nuit 9,6:1", "lavande", ""),
    ("Lavande pâle", "#EFE7FB", "239 231 251", "Fond clair des pages intérieures (dérivé).", "Nuit dessus 15,0:1", "lavande-pale", ""),
    ("Blanc", "#FFFFFF", "255 255 255", "Cartes, champs clairs, texte sur sombre.", "", "blanc", ""),
    ("Noir", "#000000", "0 0 0", "Impression en une couleur.", "", "noir", ""),
]


def tuile_couleur(nom, hexa, rvb, role, contraste, jeton, forme):
    return f"""<div class="couleur couleur-{jeton} {forme}">
<p class="couleur-nom">{nom}</p>
<p class="couleur-code">{hexa}</p>
<p class="couleur-detail">RVB {rvb}<br>--{jeton}</p>
<p class="couleur-role">{role}</p>
{f'<p class="couleur-role">{contraste}</p>' if contraste else ''}
</div>"""


def construction_arche():
    """Arche sur la grille du symbole Kadenz : quatre cases de 104 et un jeu
    de 16 sur un canevas de 256. L'arche couvre les deux colonnes ; son
    demi-disque est centré entre les deux rangées ; l'ouverture a la largeur
    d'une case."""
    cases = []
    for x in (16, 136):
        for y in (16, 136):
            cases.append(f'<rect x="{x}" y="{y}" width="104" height="104"/>')
    return f"""<svg class="construction" viewBox="0 0 256 256" aria-hidden="true" focusable="false">
<path fill="#7B2CBF" d="{arche(16, 240, 224, 224)}"/>
<path fill="#240046" d="{arche(76, 240, 104, 150)}"/>
<g fill="none" stroke="#C9B3EE" stroke-width="1" stroke-dasharray="4 4">{''.join(cases)}</g>
<circle cx="128" cy="128" r="3" fill="#F56A00"/>
</svg>"""


def rangee_arches():
    formes = []
    for i in range(8):
        couleur = "#7B2CBF" if i % 2 == 0 else "#240046"
        formes.append(f'<path fill="{couleur}" d="{arche(i * 120, 200, 104, 200)}"/>')
        formes.append(f'<path fill="#EFE7FB" d="{arche(i * 120 + 32, 200, 40, 110)}"/>')
    return (f'<svg class="rangee-arches" viewBox="0 0 944 200" aria-hidden="true" focusable="false">'
            f'{"".join(formes)}</svg>')


def planche():
    couleurs = "".join(tuile_couleur(*c) for c in COULEURS)
    return f"""<header class="planche-tete sombre">
<div class="planche-logo">{symbole()}{mot()}</div>
<div>
<p class="surtitre">Proposition du 2026-11-19</p>
<h1>Portail des licences</h1>
<p class="planche-intro">Identité d'après les références de l'utilisateur : Nuit violette, Violet, Orange d'accent, grande grotesque, l'arche comme portail. Le logo reste le symbole Kadenz.</p>
</div>
</header>

<section class="planche-section typo sombre">
<div class="typo-fond" aria-hidden="true">Aa</div>
<div class="typo-contenu">
<p class="surtitre">Typographie</p>
<p class="typo-alpha">AaBbCc</p>
<p class="typo-chiffres">1234567</p>
<p class="typo-signes">#@%&amp;*</p>
<p class="typo-nom">Inter <span>Caractère principal</span></p>
<p class="typo-note">Inter (licence OFL) est déjà servie par le portail ; repli Helvetica Neue, Helvetica, Arial. Les références sont en Helvetica, que le portail ne peut pas servir (licence payante, absente de Windows). Autre choix à soumettre : Instrument Sans ou Schibsted Grotesk (OFL, fichiers à ajouter). Graisses : 400 texte, 600 libellés et boutons, 700 titres (Inter Bold, route à ajouter).</p>
</div>
</section>

<section class="planche-section couleurs-section">
<p class="surtitre">Couleurs</p>
<h2>Palette</h2>
<div class="couleurs">{couleurs}</div>
<p class="note">Nuit violette : la référence imprime #8BD46B, qui est un vert ; la valeur retenue suit ses composantes RVB (36 0 70). Contrastes calculés selon les WCAG le 2026-11-19. Succès #067647 et Erreur #B42318 restent ceux de la marque Kadenz.</p>
</section>

<section class="planche-section">
<p class="surtitre">Logo</p>
<h2>Le symbole Kadenz, aux couleurs du portail</h2>
<div class="logos">
<div class="logo-tuile fond-nuit">{symbole()}{mot()}<span>Portail des licences</span></div>
<div class="logo-tuile fond-blanc">{symbole()}{mot()}<span>Portail des licences</span></div>
<div class="logo-tuile fond-violet">{symbole()}{mot()}<span>Portail des licences</span></div>
<div class="logo-tuile fond-orange">{symbole()}{mot()}<span>Portail des licences</span></div>
</div>
<p class="note">Écart à valider : sur fond clair, le symbole passe de l'Indigo de la marque Kadenz (#2D3A8C) au Violet, et le mot à la Nuit violette. Sur fond sombre, rien ne change : symbole et mot en blanc.</p>
</section>

<section class="planche-section">
<p class="surtitre">Motif</p>
<h2>L'arche, comme portail</h2>
<div class="motif">
{construction_arche()}
<div>
<p>Montants droits, demi-disque en haut, sur la grille du symbole Kadenz (quatre cases de 104, jeu de 16, en pointillé) : l'arche couvre les deux colonnes, son centre (point orange) est entre les deux rangées, l'ouverture a la largeur d'une case. Une arche se pose seule, en rangée ou en arches concentriques ; jamais deux arches qui se croisent.</p>
<p>Les arches sont les seules courbes de l'identité. Boutons, champs, cartes et étiquettes gardent des angles de 4 px.</p>
</div>
</div>
{rangee_arches()}
</section>

<section class="planche-section">
<p class="surtitre">Composants</p>
<h2>Boutons, champs, états, messages</h2>
<div class="composants">
<div class="composant-clair">
<div class="boutons">
<button class="bouton bouton-principal" type="button">Enregistrer un paiement</button>
<button class="bouton" type="button">Retour</button>
<button class="bouton bouton-principal bouton-petit" type="button">Tirer le code</button>
</div>
<div class="champ"><label for="essai-1">Identifiant</label><div class="saisie">{icone("identifiant")}<input id="essai-1" type="text"></div></div>
<div class="etats">
<span class="etat etat-attente">1 paiement à utiliser</span>
<span class="etat etat-neutre">Aucun à utiliser</span>
<span class="etat etat-ok">activé</span>
</div>
<p class="message message-ok">Paiement enregistré.</p>
</div>
<div class="composant-sombre sombre">
<div class="boutons">
<button class="bouton bouton-blanc" type="button">Se connecter</button>
<button class="bouton bouton-accent" type="button">Se connecter</button>
<button class="bouton bouton-sombre" type="button">Se déconnecter</button>
</div>
<div class="champ"><label for="essai-2">Mot de passe</label><div class="saisie">{icone("mot_de_passe")}<input id="essai-2" type="password"></div></div>
<div class="etats">
<span class="etiquette etiquette-orange">Entreprises</span>
<span class="etiquette etiquette-blanche">Codes à vendre</span>
<span class="etiquette etiquette-violet">Journal</span>
</div>
<p class="message message-erreur">identifiant ou mot de passe incorrect</p>
</div>
</div>
</section>

<section class="planche-section">
<p class="surtitre">Connexion</p>
<h2>Deux mises en page</h2>
<div class="apercus">
<figure><img src="captures/connexion-a.png" alt="Connexion A : cadre sombre, formulaire à gauche, portails en points à droite" width="1440" height="900"><figcaption>A. Cadre sombre, portails en points (référence 2)</figcaption></figure>
<figure><img src="captures/connexion-b.png" alt="Connexion B : carte, visuel violet à arches à gauche, formulaire à droite" width="1440" height="900"><figcaption>B. Carte, visuel à arches (référence 3)</figcaption></figure>
<figure><img src="captures/connexion-a-code.png" alt="Connexion A, code de double authentification" width="1440" height="900"><figcaption>A. Code de connexion</figcaption></figure>
<figure><img src="captures/connexion-b-code.png" alt="Connexion B, code de double authentification" width="1440" height="900"><figcaption>B. Code de connexion</figcaption></figure>
</div>
</section>

<section class="planche-section">
<p class="surtitre">Règles</p>
<h2>Ce qui reste fixe</h2>
<ul class="regles">
<li>Aucun script, aucun style en ligne, aucune image en data: (CSP du portail) : symbole, icônes, points et arches sont des SVG en ligne.</li>
<li>Aucun italique. Aucun liseré de couleur sur un seul côté d'un bloc.</li>
<li>Angles de 4 px au plus ; seules les arches sont courbes. Les étiquettes des références sont en pilule : ici, 4 px.</li>
<li>Orange : jamais en petit texte sur clair. Violet : jamais en texte sur fond sombre.</li>
<li>Rien n'est repris des références d'une marque tierce : ni logo, ni texte, ni photo.</li>
</ul>
</section>"""


def apercu_mobile():
    cadres = "".join(
        f'<figure><iframe src="{nom}" title="{legende}" width="390" height="844"></iframe>'
        f'<figcaption>{legende}</figcaption></figure>'
        for nom, legende in [("connexion-a.html", "A, 390 px"), ("connexion-b.html", "B, 390 px")])
    return f'<div class="apercu-mobile">{cadres}</div>'


def ecrire():
    (ICI / "apercu-mobile.html").write_text(
        document("Aperçu mobile", "planche", ["identite.css", "planche.css"], apercu_mobile()),
        encoding="utf-8", newline="\n")
    for nom, (titre, classe, corps) in PAGES.items():
        (ICI / nom).write_text(document(titre, classe, FEUILLES, corps), encoding="utf-8", newline="\n")
    (ICI / "identite.html").write_text(
        document("Identité", "planche", ["identite.css", "planche.css"], planche()),
        encoding="utf-8", newline="\n")


def capturer(page, png, largeur=1440, hauteur=900):
    (ICI / "captures").mkdir(exist_ok=True)
    subprocess.run([CHROME, "--headless=new", "--disable-gpu", "--hide-scrollbars",
                    "--allow-file-access-from-files", f"--window-size={largeur},{hauteur}",
                    f"--screenshot={ICI / 'captures' / png}", "--virtual-time-budget=3000",
                    (ICI / page).as_uri()], check=True, capture_output=True)


if __name__ == "__main__":
    ecrire()
    for nom in PAGES:
        capturer(nom, nom.replace(".html", ".png"))
    # Chrome sans fenêtre impose une largeur minimale : les vues mobiles
    # passent par des cadres de 390 px (apercu-mobile.html, hors portail).
    capturer("apercu-mobile.html", "connexion-mobile.png", 900, 900)
    capturer("identite.html", "identite.png", 1440, 5640)
    print("ok")
