"""Planche du mode clair après la refonte des écrans (commit 274eff2) : présentation,
inscription du poste, connexion, assistant de création du serveur, réglage de
l'affichage dans les Paramètres.

Captures (dossier ecrans/) prises le 2026-11-18 sur les exécutables de développement,
sur des dossiers d'essai uniquement ; parametres-affichage.png vient de l'essai des
palettes de la session kadenz-f8. Les noms d'entreprise et de personnes saisis dans
l'assistant sont des données d'essai fictives.

Usage : py -3.13 generer_planche.py   (Chrome requis)
"""
import subprocess
from pathlib import Path

ICI = Path(__file__).resolve().parent
CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
LARGEUR = 2640

POSTE = [
    ("p1-bienvenue", "1 sur 4", "Bienvenue dans Kadenz", "choix du mode d'affichage ; en clair, la rangée des palettes sombres est masquée"),
    ("p2-vos-documents", "2 sur 4", "Vos documents", "une phrase, bloc et boutons à la même place"),
    ("p3-vos-paiements", "3 sur 4", "Vos paiements", "une pièce du symbole tramée par page"),
    ("p4-vos-donnees", "4 sur 4", "Vos données", "dernier bouton « Rejoindre l'entreprise » en multiposte"),
]

ACCES = [
    ("inscription-poste", "Poste", "Rejoindre l'entreprise", "panneau tramé à gauche, formulaire compact"),
    ("connexion", "Poste", "Connexion", "formulaire court sous le nom de l'entreprise"),
]

ASSISTANT = [
    ("a1-licence", "1 sur 6", "Licence", "identifiant à transmettre, clé facultative"),
    ("a2-entreprise", "2 sur 6", "Entreprise", "6 champs visibles, 7 facultatifs repliés"),
    ("a2-entreprise-deplie", "2 sur 6", "Entreprise, dépliée", "la zone défile, les boutons restent en bas"),
    ("a3-administrateur", "3 sur 6", "Administrateur", "champs par paires de largeur égale"),
    ("a4-taux-et-paiements", "4 sur 6", "Taux et paiements", "trois taux côte à côte"),
    ("a5-sauvegardes", "5 sur 6", "Sauvegardes", "dossier, heure, mot de passe"),
    ("a6-recapitulatif", "6 sur 6", "Récapitulatif", "résumé avant de terminer"),
]

REGLAGE = [
    ("parametres-affichage", "Paramètres", "Navigation et affichage",
     "mode Comme Windows, Clair, Sombre ; palettes Graphite, Ardoise, Anthracite"),
]


def figure(nom, numero, titre, note):
    return f"""<figure>
  <img src="ecrans/{nom}.png" alt="{titre}">
  <figcaption><b>{numero}</b><span>{titre}</span><small>{note}</small></figcaption>
</figure>"""


def section(numero, titre, ou, classe, elements):
    corps = "".join(figure(*e) for e in elements)
    return f"""<section>
  <header><span class="num">{numero}</span><h2>{titre}</h2><p class="ou">{ou}</p></header>
  <div class="rang {classe}">{corps}</div>
</section>"""


def planche():
    sections = [
        section("01", "Présentation", "Poste, premier lancement", "quatre", POSTE),
        section("02", "Inscription et connexion", "Poste", "deux", ACCES),
        section("03", "Assistant de création", "Serveur, KadenzAdmin", "quatre", ASSISTANT),
        section("04", "Réglage de l'affichage", "Poste connecté, Paramètres", "deux", REGLAGE),
    ]
    return f"""<!doctype html>
<html lang="fr"><head><meta charset="utf-8">
<title>Kadenz, mode clair</title>
<style>
:root {{ --nuit:#1A2259; --indigo:#2D3A8C; --ambre:#E3A23B; --brume:#F4F5F9;
        --encre:#1F2933; --ardoise:#52606D; --trait:#D9DCE3; }}
* {{ box-sizing:border-box; margin:0; }}
em, i {{ font-style:normal; }}
body {{ background:var(--brume); color:var(--encre); font-family:Inter, "Segoe UI", sans-serif;
        width:{LARGEUR}px; padding:72px 80px 88px; }}
.titre {{ display:flex; align-items:baseline; gap:28px; border-bottom:1px solid var(--trait);
          padding-bottom:28px; margin-bottom:48px; }}
.titre h1 {{ font-family:"Space Grotesk", Inter, sans-serif; font-size:52px; font-weight:700; color:var(--nuit); }}
.titre p {{ font-size:20px; color:var(--ardoise); }}
section {{ margin-bottom:56px; }}
section header {{ display:flex; align-items:baseline; gap:18px; margin-bottom:22px; }}
.num {{ font-family:"Space Grotesk", Inter, sans-serif; font-weight:700; font-size:20px; color:var(--ambre); }}
h2 {{ font-family:"Space Grotesk", Inter, sans-serif; font-size:30px; color:var(--nuit); }}
.ou {{ font-size:17px; color:var(--ardoise); }}
.rang {{ display:grid; gap:28px; }}
.quatre {{ grid-template-columns:repeat(4, 1fr); }}
.deux {{ grid-template-columns:repeat(2, 1fr); max-width:1720px; }}
figure {{ background:white; border:1px solid var(--trait); border-radius:4px; padding:14px; }}
figure img {{ width:100%; display:block; border:1px solid var(--trait); }}
figcaption {{ display:grid; grid-template-columns:auto 1fr; column-gap:12px; row-gap:4px; padding:14px 4px 2px; }}
figcaption b {{ font-size:14px; color:var(--indigo); letter-spacing:.5px; }}
figcaption span {{ font-weight:600; font-size:17px; }}
figcaption small {{ grid-column:1 / -1; font-size:14.5px; color:var(--ardoise); }}
</style></head>
<body>
<div class="titre"><h1>Mode clair</h1><p>Écrans refaits le 2026-11-18, Kadenz 1.0.0, captures sur dossiers d'essai</p></div>
{"".join(sections)}
</body></html>
"""


if __name__ == "__main__":
    html = ICI / "index.html"
    html.write_text(planche(), encoding="utf-8")
    png = ICI / "planche-mode-clair.png"
    subprocess.run([CHROME, "--headless=new", "--hide-scrollbars", "--disable-gpu",
                    f"--window-size={LARGEUR},4200", f"--screenshot={png}",
                    "--virtual-time-budget=4000", html.as_uri()], check=True,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    print(png)
