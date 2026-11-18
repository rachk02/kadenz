"""Planche des écrans actuels de Kadenz 1.0.0 : présentation, inscription du
poste, assistant de création du serveur, connexion.

Les captures (dossier ecrans/) viennent de la version 1.0.0 lancée sur des
dossiers d'essai, le 2026-11-18. Les mesures citées dans les notes ont été
relevées sur ces captures (boîte des contenus, en pixels de la fenêtre).

Usage : py -3.13 generer_planche.py   (Chrome requis)
"""
import re
import subprocess
from pathlib import Path

ICI = Path(__file__).resolve().parent
CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
LARGEUR = 2640

PRESENTATION = [
    ("p1-bienvenue", "1 sur 4", "Bienvenue dans Kadenz", "31 mots, haut du bloc à 270 px"),
    ("p2-vos-documents", "2 sur 4", "Vos documents", "65 mots, haut du bloc à 252 px"),
    ("p3-vos-paiements", "3 sur 4", "Vos paiements", "56 mots, haut du bloc à 261 px"),
    ("p4-vos-donnees", "4 sur 4", "Vos données", "45 mots + 10, haut du bloc à 252 px"),
]

ASSISTANT = [
    ("a1-licence", "1 sur 6", "Licence", "identifiant, clé de licence ; bloc à 304 px du bord, 672 px de large"),
    ("a2-entreprise", "2 sur 6", "Entreprise", "13 champs, seul le nom est obligatoire ; bloc à 462 px, 356 px de large"),
    ("a3-administrateur", "3 sur 6", "Administrateur", "4 champs ; bloc à 446 px, 389 px de large"),
    ("a4-taux-et-paiements", "4 sur 6", "Taux et paiements", "4 champs préremplis et une case ; bloc à 320 px"),
    ("a5-sauvegardes", "5 sur 6", "Sauvegardes", "dossier, heure, mot de passe, confirmation ; bloc à 304 px"),
    ("a6-recapitulatif", "6 sur 6", "Récapitulatif", "7 lignes de résumé ; bloc à 338 px"),
]


def figure(nom, numero, titre, note, classe=""):
    return f"""<figure class="{classe}">
  <img src="ecrans/{nom}.png" alt="{titre}">
  <figcaption><b>{numero}</b><span>{titre}</span><small>{note}</small></figcaption>
</figure>"""


def notes(*lignes):
    return "<ul class=\"notes\">" + "".join(f"<li>{l}</li>" for l in lignes) + "</ul>"


def section(numero, titre, ou, corps, remarques):
    return f"""<section>
  <header><span class="num">{numero}</span><h2>{titre}</h2><p class="ou">{ou}</p></header>
  {corps}
  {remarques}
</section>"""


def planche():
    pres = "<div class=\"rang quatre\">" + "".join(figure(*p) for p in PRESENTATION) + "</div>"
    assist = "<div class=\"rang trois\">" + "".join(figure(*a) for a in ASSISTANT) + "</div>"
    inscription = "<div class=\"rang deux\">" + figure(
        "inscription-poste", "Poste", "Rejoindre l'entreprise",
        "2 paragraphes d'explication, code d'inscription, adresse du serveur") + figure(
        "connexion", "Poste", "Connexion",
        "identifiant, mot de passe ; le titre est le nom de l'entreprise") + "</div>"

    s1 = section("01", "Présentation", "Poste, premier lancement : avant l'inscription",
                 pres,
                 notes("<b>Remarque de l'utilisateur :</b> trop de texte sur les quatre écrans.",
                       "<b>Remarque de l'utilisateur :</b> le texte n'est pas fixe, il monte et redescend d'une page à "
                       "l'autre. Mesure : le haut du bloc passe de 270 à 252, 261 puis 252 px, parce que le bloc est "
                       "centré verticalement et que la longueur du paragraphe change (31 à 65 mots).",
                       "La page 4 parle d'une base locale sur le poste et annonce « l'assistant qui suit » ; en 1.0, "
                       "le poste passe ensuite à l'inscription, pas à l'assistant. Texte à revoir avec la refonte."))
    s2 = section("02", "Inscription et connexion", "Poste : rejoindre l'entreprise, puis se connecter",
                 inscription,
                 notes("Inscription : un sur-titre, un titre, deux paragraphes, deux champs, deux boutons.",
                       "Connexion : formulaire court au centre de la zone blanche, panneau Nuit à gauche avec le "
                       "motif Quatre pièces et « Gestion commerciale ».",
                       "<b>Demande de l'utilisateur :</b> modèles aimés, fond sombre, formulaire court à gauche, grande "
                       "image tramée ou photo à droite ; et, pour l'inscription, carte sombre avec un panneau image "
                       "d'un côté et le formulaire de l'autre."))
    s3 = section("03", "Assistant de création", "KadenzAdmin, fenêtre du serveur : six étapes",
                 assist,
                 notes("<b>Remarque de l'utilisateur :</b> trop de détails à l'installation.",
                       "Le bloc change de place et de largeur à chaque étape : bord gauche de 304 à 462 px, haut de "
                       "90 à 228 px, largeur de 356 à 672 px. Même défaut que la présentation.",
                       "Étape 2 : 13 champs sur une seule page, dont 12 facultatifs. Étapes 1 et 5 : paragraphes "
                       "d'explication de trois lignes.",
                       "Le titre « Première installation » se répète sur les six étapes ; le nom de l'étape est "
                       "écrit deux fois (liste à gauche et « Étape n sur 6 »)."))
    s4 = section("04", "Pour la refonte", "Demandes de l'utilisateur à ce jour",
                 "",
                 notes("Moins de texte sur la présentation, l'inscription et l'assistant.",
                       "Un bloc à position fixe d'une page ou d'une étape à l'autre.",
                       "Un mode sombre.",
                       "Connexion et inscription sur les modèles aimés (fond sombre, formulaire court, grande image).",
                       "Plusieurs versions de Kadenz à venir ; l'essentiel, selon l'utilisateur : les pages de création et de connexion."))

    return f"""<!doctype html>
<html lang="fr"><head><meta charset="utf-8">
<title>Kadenz, écrans actuels</title>
<meta name="robots" content="noindex">
<style>
@font-face{{font-family:Inter;src:url(../planches/polices/Inter.woff2) format("woff2");font-weight:100 900}}
@font-face{{font-family:"Space Grotesk";src:url(../planches/polices/SpaceGrotesk.ttf) format("truetype");font-weight:300 700}}
*{{box-sizing:border-box;margin:0;padding:0}}
em,i{{font-style:normal}}
body{{background:#F4F5F9;color:#1F2933;font:18px/1.5 Inter,sans-serif;width:{LARGEUR}px;padding:96px 120px 120px}}
h1,h2{{font-family:"Space Grotesk",sans-serif;font-weight:700;letter-spacing:-.01em}}
.tete{{display:flex;justify-content:space-between;align-items:flex-end;padding-bottom:40px;border-bottom:1px solid #D9DCE3;margin-bottom:72px}}
.tete h1{{font-size:64px;line-height:1}}
.tete p{{color:#52606D;font-size:20px;text-align:right}}
.tete .sur{{font:600 15px Inter;letter-spacing:.14em;text-transform:uppercase;color:#2D3A8C;margin-bottom:18px}}
section{{margin-bottom:96px}}
section header{{display:flex;align-items:baseline;gap:24px;margin-bottom:32px}}
.num{{font:700 22px "Space Grotesk";color:#E3A23B}}
section h2{{font-size:40px}}
.ou{{color:#52606D;font-size:20px}}
.rang{{display:grid;gap:32px}}
.quatre{{grid-template-columns:repeat(4,1fr)}}
.trois{{grid-template-columns:repeat(3,1fr)}}
.deux{{grid-template-columns:repeat(2,1fr)}}
figure{{background:#fff;border:1px solid #D9DCE3;border-radius:4px;overflow:hidden}}
figure img{{display:block;width:100%;height:auto;border-bottom:1px solid #D9DCE3}}
figcaption{{display:grid;grid-template-columns:auto 1fr;column-gap:16px;padding:18px 22px 22px}}
figcaption b{{font:600 14px Inter;letter-spacing:.1em;text-transform:uppercase;color:#2D3A8C;padding-top:4px}}
figcaption span{{font:700 22px "Space Grotesk"}}
figcaption small{{grid-column:2;color:#52606D;font-size:16px}}
.notes{{list-style:none;margin-top:32px;display:grid;grid-template-columns:repeat(2,1fr);gap:16px 48px}}
.notes li{{position:relative;padding-left:28px;font-size:19px}}
.notes li::before{{content:"";position:absolute;left:0;top:11px;width:10px;height:10px;background:#2D3A8C}}
.notes b{{font-weight:600}}
</style></head>
<body>
<div class="tete">
  <div><p class="sur" style="text-align:left">Kadenz 1.0.0</p><h1>Écrans actuels : création et connexion</h1></div>
  <p>Captures du 2026-11-18, sur dossiers d'essai<br>Fenêtre de 1200 x 760 (poste) et 1040 x 720 (serveur)</p>
</div>
{s1}
{s2}
{s3}
{s4}
</body></html>"""


def chrome(*args):
    return subprocess.run([CHROME, "--headless=new", "--hide-scrollbars", "--virtual-time-budget=5000", *args],
                          check=True, capture_output=True, text=True, encoding="utf-8")


if __name__ == "__main__":
    html = ICI / "index.html"
    contenu = planche()
    html.write_text(contenu, encoding="utf-8")
    mesure = ICI / "_mesure.html"
    mesure.write_text(contenu.replace("</body>", "<script>document.fonts.ready.then(()=>document.body.setAttribute("
                                      "'data-h',document.documentElement.scrollHeight))</script></body>"),
                      encoding="utf-8")
    h = int(re.search(r'data-h="(\d+)"', chrome(f"--window-size={LARGEUR},1000", "--dump-dom",
                                                 mesure.as_uri()).stdout).group(1))
    mesure.unlink()
    png = ICI / "planche-ecrans-actuels.png"
    chrome("--force-device-scale-factor=1", f"--window-size={LARGEUR},{h}", f"--screenshot={png}", html.as_uri())
    print(png, h)
