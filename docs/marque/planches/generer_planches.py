"""Planches de l'identité Kadenz, « Quatre pièces », Indigo et Nuit (couleurs de l'ancienne identité, reprises le 2026-11-12).

Neuf planches HTML de 1200 x 760 (logo principal, symbole, ton, couleurs, motif, typographie,
signalétique, papeterie, mises en situation), la page index.html qui les réunit, une image PNG
par planche (2400 x 1520) et le PDF (une planche par page), rendus par Chrome sans interface.
Usage : python docs/marque/planches/generer_planches.py <racine du dépôt>
"""

import importlib.util
import math
import re
import subprocess
import sys
from pathlib import Path

ICI = Path(__file__).resolve().parent
MARQUE = ICI.parent
CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"

spec = importlib.util.spec_from_file_location("generer_marque", MARQUE / "generer.py")
gm = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gm)   # lit sys.argv[1] (racine du dépôt) pour la police

spec_sc = importlib.util.spec_from_file_location("scenes_signaletique", ICI / "scenes_signaletique.py")
sc = importlib.util.module_from_spec(spec_sc)
spec_sc.loader.exec_module(sc)
sc.gm = gm

spec_sp = importlib.util.spec_from_file_location("scenes_papeterie", ICI / "scenes_papeterie.py")
sp = importlib.util.module_from_spec(spec_sp)
spec_sp.loader.exec_module(sp)
sp.gm, sp.sc = gm, sc

BLEU, NUIT, PALE, ENCRE, ARDOISE, TRAIT, BRUME = "#2d3a8c", "#1a2259", "#e8eaf6", "#1f2933", "#52606d", "#d9dce3", "#f4f5f9"
CLAIR = "#c5cae0"   # texte secondaire sur Nuit (9,1:1)
AMBRE = "#e3a23b"   # troisième temps, touche sur fond sombre

TETE = """<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8">
<title>{titre}</title>
<style>
@font-face{{font-family:Inter;src:url(polices/Inter.woff2) format("woff2");font-weight:100 900;font-display:swap}}
@font-face{{font-family:"Space Grotesk";src:url(polices/SpaceGrotesk.ttf) format("truetype");font-weight:300 700;font-display:swap}}
body{{margin:0}}em,i{{font-style:normal}}
@page{{size:1200px 760px;margin:0}}
.planche{{width:1200px;height:760px;box-sizing:border-box;padding:48px 56px;background:#fff;color:{encre};font-family:Inter,sans-serif;display:flex;flex-direction:column;gap:26px;overflow:hidden;break-after:page}}
.ent{{display:flex;justify-content:space-between;align-items:baseline}}
.num{{font-family:Space Grotesk,sans-serif;font-weight:700;font-size:15px;color:{bleu};letter-spacing:.08em;margin-right:16px}}
h1{{display:inline;margin:0;font-family:Space Grotesk,sans-serif;font-weight:700;font-size:32px;letter-spacing:-.01em}}
.sous{{margin:0;font-size:14px;color:{ardoise};max-width:620px;text-align:right}}
.case{{border-radius:4px;position:relative;display:flex;align-items:center;justify-content:center;box-sizing:border-box}}
.bord{{border:1px solid {trait}}}
.leg{{position:absolute;left:18px;bottom:14px;font-size:12px;color:{ardoise}}}
.leg.c{{color:{clair}}}
.petit{{font-size:12px;color:{ardoise};line-height:1.5}}
.t3{{font-family:Space Grotesk,sans-serif;font-weight:700;font-size:16px;margin:0 0 8px}}
p{{margin:0}}
</style>
</head>
<body>
""".format(titre="{titre}", encre=ENCRE, bleu=BLEU, ardoise=ARDOISE, trait=TRAIT, clair=CLAIR)
PIED = "</body>\n</html>\n"


def interieur(nom):
    s = (MARQUE / nom).read_text(encoding="utf-8")
    vb = re.search(r'viewBox="([^"]+)"', s).group(1)
    corps = re.search(r"<svg[^>]*>(.*)</svg>", s, re.S).group(1)
    return re.sub(r"<title>.*?</title>", "", corps), vb


def img(nom, hauteur, extra=""):
    return f'<img src="../{nom}" alt="" style="height:{hauteur}px;{extra}">'


def motif_calme(fond, trace, w, h, s, calmes=()):
    """Motif à la taille exacte de la case (w x h px) ; les pièces dont la case touche une zone calme
    (x0, y0, x1, y1) sont retirées entières, pour qu'aucune pièce ne soit rognée autour du logo."""
    m, k = 7 * s / 40, s - 2 * 7 * s / 40
    out = [f'<rect width="{w}" height="{h}" fill="{fond}"/>']
    for r in range(math.ceil(h / s)):
        for col in range(math.ceil(w / s)):
            x0, y0 = col * s, r * s
            if any(x0 < z[2] and x0 + s > z[0] and y0 < z[3] and y0 + s > z[1] for z in calmes):
                continue
            x, y = x0 + m, y0 + m
            sorte = (col + r) % 4
            if sorte == 0:
                d = gm.poly([(x, y), (x + k, y), (x + k, y + k), (x, y + k)])
            elif sorte == 1:
                d = f"M{gm.f(x)} {gm.f(y + k)} L{gm.f(x)} {gm.f(y)} A{gm.f(k)} {gm.f(k)} 0 0 1 {gm.f(x + k)} {gm.f(y + k)} Z"
            elif sorte == 2:
                d = gm.poly([(x, y), (x + k, y + k), (x, y + k)])
            else:
                d = gm.cercle(x + k / 2, y + k / 2, k / 2)
            out.append(f'<path fill="{trace}" d="{d}"/>')
    return (f'<svg viewBox="0 0 {w} {h}" style="position:absolute;left:0;top:0;width:{w}px;height:{h}px">'
            + "".join(out) + "</svg>")


def entete(n, titre, sous):
    return (f'<div class="ent"><div><span class="num">{n:02d}</span><h1>{titre}</h1></div>'
            f'<p class="sous">{sous}</p></div>')


def planche(n, titre, sous, corps):
    return f'<div class="planche">{entete(n, titre, sous)}{corps}</div>'


# --- 01 Primary logo ----------------------------------------------------------------
def p01():
    c = f"""<div style="display:grid;grid-template-columns:repeat(3,minmax(0,1fr));grid-template-rows:300px 270px;gap:20px">
<div class="case" style="grid-column:span 2;background:{BRUME}">{img("kadenz-logo-horizontal.svg", 112)}<span class="leg">Version horizontale, usage par défaut</span></div>
<div class="case bord">{img("kadenz-logo-empile.svg", 170)}<span class="leg">Version empilée, formats carrés</span></div>
<div class="case" style="background:{BLEU}">{img("kadenz-logo-horizontal-negatif.svg", 64)}<span class="leg c">Négatif sur Indigo</span></div>
<div class="case bord">{img("kadenz-logo-horizontal-mono.svg", 64)}<span class="leg">Une couleur : tampon, impression noire</span></div>
<div class="case bord" style="flex-direction:column;align-items:flex-start;justify-content:center;padding:26px;gap:14px">
<div><p class="t3">Zone de protection</p><p class="petit">La moitié du côté d'une case du symbole, tout autour du logo. Elle suit la taille du logo.</p></div>
<div><p class="t3">Tailles minimales</p><p class="petit">Horizontal : 120 px ou 30 mm de large. Empilé : 64 px ou 18 mm. Symbole : 16 px ou 6 mm.</p></div>
<div><p class="t3">Mot</p><p class="petit">« kadenz » en Space Grotesk 600, vectorisé, en Nuit sur fond clair.</p></div>
</div></div>"""
    return planche(1, "Primary logo", "Quatre pièces et le mot « kadenz », en une seule couleur : Indigo", c)


# --- 02 Logomark --------------------------------------------------------------------
def construction():
    corps, _ = interieur("kadenz-symbole.svg")
    lignes = ""
    for v in (16, 120, 136, 240):
        lignes += (f'<line x1="{v}" y1="0" x2="{v}" y2="256" stroke="{BLEU}" stroke-opacity=".35" stroke-width=".8"/>'
                   f'<line x1="0" y1="{v}" x2="256" y2="{v}" stroke="{BLEU}" stroke-opacity=".35" stroke-width=".8"/>')
    corps = corps.replace(BLEU.upper(), PALE).replace(BLEU, PALE)
    return (f'<svg viewBox="-28 -28 312 312" style="height:330px">'
            f'<rect x="0" y="0" width="256" height="256" fill="none" stroke="{TRAIT}" stroke-width="1"/>{corps}{lignes}'
            f'<g font-family="Inter" font-size="9" fill="{ARDOISE}">'
            f'<text x="0" y="-10">0</text><text x="12" y="-10">16</text><text x="64" y="-10">case 104</text>'
            f'<text x="121" y="-10">jeu 16</text><text x="222" y="-10">240</text><text x="248" y="-10">256</text></g></svg>')


def p02():
    tailles = "".join(f'<div style="display:flex;flex-direction:column;align-items:center;gap:8px">{img("kadenz-icone.svg", t)}'
                      f'<span class="petit">{t} px</span></div>' for t in (96, 64, 48, 32, 24, 16))
    fonds = "".join(f'<div class="case{" bord" if f == "#ffffff" else ""}" style="background:{f};height:96px">{img(s, 60)}</div>'
                    for f, s in (("#ffffff", "kadenz-symbole.svg"), (BLEU, "kadenz-symbole-blanc.svg"),
                                 (NUIT, "kadenz-symbole-blanc.svg"), ("#ffffff", "kadenz-symbole-noir.svg")))
    c = f"""<div style="display:grid;grid-template-columns:400px 1fr;gap:28px;flex:1">
<div class="case bord" style="flex-direction:column;gap:6px">{construction()}
<p class="petit" style="text-align:center;padding:0 22px">Canevas de 256, marge de 16, jeu de 16, case de 104.<br>Débord optique : quart de disque + 1,5 %, disque + 3 %.</p></div>
<div style="display:flex;flex-direction:column;gap:22px">
<div><p class="t3">Icône d'application, de la plus grande à la plus petite</p>
<div style="display:flex;align-items:flex-end;gap:26px">{tailles}</div>
<p class="petit" style="margin-top:10px">L'icône emploie la petite coupe : jeux de 26 au lieu de 16, sans débord, pour que les pièces restent séparées à 16 px.</p></div>
<div style="display:grid;grid-template-columns:repeat(4,1fr);gap:14px">{fonds}</div>
<div class="bord" style="border-radius:4px;padding:20px 22px"><p class="t3">Ce que dit le symbole</p>
<p class="petit" style="font-size:13px">Quatre formes simples s'emboîtent sur une grille de deux par deux, comme les modules que la licence assemble : ventes, commandes, livraisons, paiements. Le disque marque le temps fort de la cadence. Sans tuile, le symbole s'emploie seul en Bleu vif, en blanc ou en noir, jamais déformé ni pivoté.</p></div>
</div></div>"""
    return planche(2, "Logomark", "Carré, quart de disque, triangle, disque", c)


# --- 03 Tone & Mood -----------------------------------------------------------------
def p03():
    adj = "".join(f'<span style="border:1.5px solid {BLEU};color:{BLEU};border-radius:4px;padding:8px 16px;font-family:Space Grotesk;font-weight:700;font-size:16px">{a}</span>'
                  for a in ("Rigoureux", "Régulier", "Clair", "Sobre", "Fiable"))
    curseurs = "".join(
        f'<div style="display:grid;grid-template-columns:100px 1fr 100px;align-items:center;gap:14px;font-size:14px;color:{ARDOISE}">'
        f'<span>{g}</span><div style="position:relative;height:2px;background:{TRAIT}"><div style="position:absolute;left:{p}%;top:-6px;width:14px;height:14px;border-radius:2px;background:{BLEU}"></div></div>'
        f'<span style="text-align:right">{d}</span></div>'
        for g, d, p in (("Classique", "Moderne", 68), ("Sérieux", "Ludique", 24), ("Discret", "Expressif", 32), ("Technique", "Accessible", 60)))
    sym, _ = interieur("kadenz-symbole-blanc.svg")
    c = f"""<div style="display:grid;grid-template-columns:1fr 1fr;gap:20px;flex:1">
<div style="display:grid;grid-template-columns:1fr 1fr;grid-template-rows:200px 150px 150px;gap:12px">
<div class="case" style="grid-column:span 2;background:{BLEU};overflow:hidden;justify-content:space-between;padding:0 30px">
<p style="font-family:Space Grotesk;font-weight:700;font-size:38px;color:#fff;line-height:1.15">Rigueur<br>et régularité</p>
<svg viewBox="0 0 256 256" style="height:120px">{sym}</svg></div>
<div class="case" style="background:{BRUME};flex-direction:column;align-items:flex-start;justify-content:flex-end;padding:18px;gap:6px">
<div style="width:100%;height:2px;background:{TRAIT}"></div><div style="width:100%;height:2px;background:{TRAIT}"></div><div style="width:64%;height:2px;background:{TRAIT}"></div>
<span class="petit" style="margin-top:12px;font-size:13px">Papier, lignes, colonnes</span></div>
<div class="case" style="background:{PALE};flex-direction:column;align-items:flex-start;justify-content:space-between;padding:18px">
<div style="width:58px;height:58px;border-radius:50%;background:{BLEU}"></div>
<span style="font-size:13px;color:{ENCRE};font-weight:600">Le temps fort : le disque</span></div>
<div class="case" style="background:{NUIT};align-items:flex-end;justify-content:flex-start;padding:18px"><span style="font-size:13px;color:#fff">Calme, concentration, fin de journée</span></div>
<div class="case bord" style="flex-direction:column;align-items:flex-start;justify-content:center;padding:18px">
<span style="font-family:Space Grotesk;font-weight:700;font-size:34px;font-variant-numeric:tabular-nums">0123456789</span><span class="petit" style="font-size:13px">Des chiffres alignés</span></div>
</div>
<div style="display:flex;flex-direction:column;gap:14px">
<div style="display:flex;gap:10px;flex-wrap:wrap">{adj}</div>
<div class="bord" style="border-radius:4px;padding:18px 22px;display:flex;flex-direction:column;gap:14px"><p class="t3" style="margin:0">Position de la marque</p>{curseurs}</div>
<div style="display:grid;grid-template-columns:1fr 1fr;gap:12px;flex:1">
<div class="bord" style="border-radius:4px;padding:18px"><p class="t3" style="color:#067647">Notre voix</p>
<p class="petit" style="font-size:13px">Des verbes d'action, le vocabulaire du métier, des phrases courtes. Comme les libellés déjà présents dans l'application :</p>
<p style="font-size:14px;margin-top:10px;line-height:1.7">Se connecter<br>Enregistrer<br>Mot de passe modifié.</p></div>
<div class="bord" style="border-radius:4px;padding:18px"><p class="t3" style="color:#b42318">À éviter</p>
<p class="petit" style="font-size:13px;line-height:1.6">Le jargon informatique là où un mot du métier existe.<br><br>Les points d'exclamation, les superlatifs, les emoji.<br><br>Les messages vagues : dire ce qui s'est passé et quoi faire.</p></div>
</div></div></div>"""
    return planche(3, "Tone &amp; Mood", "Propositions à valider : un outil de travail sûr, régulier, sans fioriture", c)


# --- 04 Color system ----------------------------------------------------------------
def p04():
    grands = [
        (BLEU, "#fff", "Indigo", "#2D3A8C", "RVB 45 58 140", "Logo, boutons, jeton accent", "Blanc dessus : 10,0:1"),
        (NUIT, "#fff", "Nuit", "#1A2259", "RVB 26 34 89", "Bandeaux de marque, mot du logo", "Blanc dessus : 14,8:1"),
        (AMBRE, ENCRE, "Ambre", "#E3A23B", "RVB 227 162 59", "Troisième temps, touche sur fond sombre", "Sur la Nuit : 6,7:1"),
    ]
    g = "".join(f'<div style="background:{f};color:{t};border-radius:4px;padding:22px;display:flex;flex-direction:column;justify-content:flex-end;gap:4px;height:210px;box-sizing:border-box{";border:1px solid " + TRAIT if f == PALE else ""}">'
                f'<span style="font-family:Space Grotesk;font-weight:700;font-size:22px">{n}</span><span style="font-size:14px">{h} · {r}</span>'
                f'<span style="font-size:13px;opacity:.9">{u}</span><span style="font-size:13px;opacity:.9">{k}</span></div>'
                for f, t, n, h, r, u, k in grands)
    neutres = [("#1F2933", "Encre", "texte · 14,8:1"), ("#52606D", "Ardoise", "texte-secondaire · 6,5:1"), ("#D9DCE3", "Trait", "bordure"),
               ("#F4F5F9", "Brume", "fond-lateral"), ("#FFFFFF", "Blanc", "fond, papier"), ("#067647", "Succès", "succes · 5,7:1"), ("#B42318", "Erreur", "erreur · 6,6:1")]
    n = "".join(f'<div style="display:flex;flex-direction:column;gap:6px"><div style="height:64px;border-radius:4px;background:{h};border:1px solid {TRAIT}"></div>'
                f'<span style="font-weight:600;font-size:13px">{nom}</span><span class="petit">{h}<br>{u}</span></div>' for h, nom, u in neutres)
    barre = (f'<div style="display:flex;height:34px;border-radius:4px;overflow:hidden;border:1px solid {TRAIT}">'
             f'<div style="flex:60;background:{BRUME}"></div><div style="flex:25;background:{BLEU}"></div><div style="flex:15;background:{NUIT}"></div></div>'
             f'<div style="display:flex;font-size:12px;color:{ARDOISE}"><span style="flex:60">Blanc et Brume, 60 %</span><span style="flex:25">Indigo, 25 %</span><span style="flex:15">Nuit et Encre, 15 %</span></div>')
    c = f"""<div style="display:grid;grid-template-columns:repeat(3,1fr);gap:16px">{g}</div>
<div style="display:grid;grid-template-columns:repeat(7,1fr);gap:14px">{n}</div>
<div style="display:flex;flex-direction:column;gap:8px"><p class="t3" style="margin:0">Proportions d'usage (indicatives)</p>{barre}
<p class="petit">Contrastes calculés selon WCAG 2.1, cible 4,5:1 pour le texte. Sur les bandeaux Nuit, textes secondaires en #C5CAE0 (9,1:1) ; l'Ambre jamais en texte sur blanc. CMJN approchés et Pantone à faire établir par l'imprimeur.</p></div>"""
    return planche(4, "Color System", "L'Indigo et la Nuit, une touche d'Ambre, et les neutres déjà en place dans l'application", c)


# --- 05 Brand pattern ---------------------------------------------------------------
def p05():
    mods = "".join(f'<div style="display:flex;flex-direction:column;align-items:center;gap:8px">{img(f"kadenz-module-{m}.svg", 72)}'
                   f'<span class="petit">{lib}</span></div>'
                   for m, lib in (("ventes", "Ventes"), ("commandes", "Commandes"), ("livraisons", "Livraisons"), ("paiements", "Paiements")))
    regles = "".join(f'<div><p class="t3" style="font-size:14px;margin-bottom:2px">{t}</p><p class="petit">{d}</p></div>' for t, d in (
        ("Règle 1", "Le motif reste ton sur ton, dans la Nuit ou dans les neutres, jamais à plusieurs couleurs."),
        ("Règle 2", "Jamais derrière un texte courant ; le logo se pose sur une zone calme."),
        ("Règle 3", "Chaque motif garde sa construction (ordre, pas, rotations) ; on agrandit, on ne déforme pas.")))
    lw, lh = 349, 330   # panneaux du haut

    def absolu(s):
        return s.replace("<svg ", f'<svg style="position:absolute;left:0;top:0;width:{lw}px;height:{lh}px" ', 1)
    logo = f'<div style="position:relative">{img("kadenz-logo-horizontal-negatif.svg", 40, "display:block")}</div>'
    c = f"""<div style="display:grid;grid-template-columns:repeat(3,{lw}px);grid-template-rows:{lh}px 1fr;justify-content:space-between;row-gap:18px;flex:1">
<div class="case" style="overflow:hidden;background:{NUIT}">{motif_calme(NUIT, "#232c6b", lw, lh, 36, [(52, 130, 297, 200), (0, 300, lw, lh)])}{logo}
<span class="leg c">Quatre pièces : bandeaux, couvertures, kakémono</span></div>
<div class="case" style="overflow:hidden;background:{NUIT}">{absolu(gm.motif_tournant(NUIT, "#232c6b", lw, lh, 44))}
<span class="leg c" style="background:{NUIT};padding:2px 6px;left:12px">Tournant : grands fonds, écrans d'accueil</span></div>
<div class="case" style="overflow:hidden;background:{NUIT};justify-content:flex-end;padding-right:26px">{absolu(gm.motif_fondu(NUIT, "#232c6b", lw, lh, 36, 0.55))}{logo}
<span class="leg c">Fondu : bandeaux avec logo ou titre</span></div>
<div class="case" style="overflow:hidden;background:{BRUME}">{motif_calme(BRUME, "#e6e9f0", lw, 240, 30, [(0, 205, 300, 240)])}
<span class="leg">Version claire : fonds de page, papeterie</span></div>
<div class="bord" style="border-radius:4px;padding:16px 18px;display:flex;flex-direction:column"><p class="t3" style="font-size:14px">Marques de modules (proposition)</p>
<div style="display:flex;justify-content:space-between;align-items:center;flex:1">{mods}</div></div>
<div style="display:flex;flex-direction:column;gap:8px">{regles}</div>
</div>"""
    return planche(5, "Brand Pattern", "Trois motifs tirés des quatre pièces : la cadence, le tournant, le fondu ; toujours ton sur ton", c)


# --- 06 Typography ------------------------------------------------------------------
def p06():
    def famille(nom, role, graisses, famille_css):
        return (f'<div class="bord" style="border-radius:4px;padding:20px 22px;display:flex;flex-direction:column;gap:6px">'
                f'<div style="display:flex;justify-content:space-between"><span class="petit">{role}</span><span class="petit">OFL 1.1</span></div>'
                f'<span style="font-family:{famille_css};font-weight:800;font-size:76px;line-height:1">Aa</span>'
                f'<span style="font-family:{famille_css};font-weight:700;font-size:22px">{nom}</span><span class="petit">{graisses}</span></div>')
    echelle = "".join(f'<div style="display:grid;grid-template-columns:220px 1fr;align-items:baseline;gap:16px;border-bottom:1px solid {TRAIT};padding-bottom:8px">'
                      f'<span class="petit">{l}</span><span style="{s}">{t}</span></div>' for l, s, t in (
        ("Titre 1 · Space Grotesk 700 · 36/44", "font-family:Space Grotesk;font-weight:700;font-size:34px", "Gestion commerciale"),
        ("Titre 2 · Space Grotesk 700 · 24/32", "font-family:Space Grotesk;font-weight:700;font-size:24px", "Devis, proforma, factures"),
        ("Titre 3 · Space Grotesk 700 · 18/26", "font-family:Space Grotesk;font-weight:700;font-size:18px", "Bons de commande et bordereaux"),
        ("Corps · Inter 400 · 15/24", "font-family:Inter;font-size:15px", "Logiciel Windows de gestion commerciale, installé indépendamment chez chaque entreprise cliente."),
        ("Libellé · Inter 500 · 13/18", "font-family:Inter;font-weight:500;font-size:13px", "Se connecter · Enregistrer · Mon compte"),
        ("Surtitre · Space Grotesk 700 · 12, capitales", f"font-family:Space Grotesk;font-weight:700;font-size:12px;letter-spacing:.08em;color:{BLEU}", "PAIEMENTS")))
    c = f"""<div style="display:grid;grid-template-columns:330px 1fr;gap:24px;flex:1">
<div style="display:flex;flex-direction:column;gap:14px">
{famille("Space Grotesk", "Titres, mot du logo", "SemiBold 600 (mot) · Bold 700 (titres)", "Space Grotesk")}
{famille("Inter", "Texte, interface, chiffres", "Regular 400 · Medium 500 · SemiBold 600 · Bold 700", "Inter")}
</div>
<div style="display:flex;flex-direction:column;gap:12px">
<div class="bord" style="border-radius:4px;padding:16px 20px;display:flex;justify-content:space-between;align-items:center">
<div><p style="font-size:15px;letter-spacing:.02em">ABCDEFGHIJKLMNOPQRSTUVWXYZ<br>abcdefghijklmnopqrstuvwxyz éèêàçùôî</p></div>
<div style="text-align:right;font-variant-numeric:tabular-nums;font-size:16px;font-weight:600;line-height:1.4">1 111 111<br>888 888<br>4 047</div></div>
<p class="petit">Les montants s'affichent en chiffres tabulaires (tnum) pour rester alignés en colonne.</p>
{echelle}
</div></div>"""
    return planche(6, "Typography", "Deux familles libres (SIL Open Font License 1.1), embarquées dans l'application", c)


# --- 07 Signage ---------------------------------------------------------------------
def p07():
    """Huit supports retenus le 2026-11-13 (scènes de scenes_signaletique.py), silhouette de 1,75 m pour l'échelle."""
    supports = (
        (sc.facade, "Façade : caisson Nuit, motif Fondu"), (sc.drapeau, "Drapeau sur potence, 600 x 600 mm"),
        (sc.lettres, "Lettres boîtier, mur d'accueil"), (sc.plaque, "Plaque d'entrée, 300 x 200 mm"),
        (sc.vitrophanie, "Vitrophanie : bande dépolie, Fondu"), (sc.totem, "Totem extérieur, 2,2 m, Tournant"),
        (sc.kakemono, "Kakémono 850 x 2000 mm, Fondu"), (sc.stand, "Stand de salon, 3 x 2,5 m"))
    cases = "".join(
        f'<div style="display:flex;flex-direction:column;gap:7px">'
        f'<svg viewBox="0 20 400 420" width="250" height="262" style="display:block;border-radius:4px;border:1px solid {TRAIT}">'
        + "".join(fn()) + f'</svg><p class="petit">{leg}</p></div>' for fn, leg in supports)
    c = f"""<div style="display:grid;grid-template-columns:repeat(4,250px);justify-content:space-between;row-gap:12px">{cases}</div>"""
    return planche(7, "Signage design", "Façade, rue, accueil, porte, salon : dimensions indicatives, à confirmer avec le fabricant", c)


# --- 08 Stationery ------------------------------------------------------------------
def p08():
    """Papeterie retenue le 2026-11-13 (scènes de scenes_papeterie.py)."""
    def scene(fn, cle, vb, w, h):
        return (f'<svg viewBox="{vb}" width="{w}" height="{h}" style="display:block;border-radius:4px;background:{BRUME}">'
                + "".join(fn(cle)) + "</svg>")
    haut = ((sp.carte, "Carte de visite 85 x 55 mm"), (sp.papier, "Papier à en-tête A4"),
            (sp.enveloppe, "Enveloppe DL, intérieur imprimé"), (sp.chemise, "Chemise à rabats A4"),
            (sp.badge, "Badge 90 x 120 mm"))
    cases = "".join(f'<div style="display:flex;flex-direction:column;gap:6px">{scene(fn, f"p8h{k}", "0 0 400 500", 205, 256)}'
                    f'<p class="petit">{leg}</p></div>' for k, (fn, leg) in enumerate(haut))
    bas = ((sp.carnet, "Carnet A5 et autocollants", "0 0 400 500", 1), (sp.signatures, "Signatures de courriel, trois variantes", "0 0 680 428", 2),
           (sp.tampons, "Tampons, trois variantes, une seule couleur d'encre", "0 0 680 428", 2))
    cases += "".join(f'<div style="display:flex;flex-direction:column;gap:6px;grid-column:span {n}">'
                     f'{scene(fn, f"p8b{k}", vb, 205 if n == 1 else 426, 256)}<p class="petit">{leg}</p></div>'
                     for k, (fn, leg, vb, n) in enumerate(bas))
    c = f"""<div style="display:grid;grid-template-columns:repeat(5,205px);justify-content:space-between;row-gap:12px">{cases}</div>"""
    return planche(8, "Stationery design", "Papeterie de l'éditeur. Les documents produits par le logiciel gardent l'identité du client.", c)


# --- 09 Brand mockups ---------------------------------------------------------------
def p09():
    etapes = "".join(f'<div style="display:flex;align-items:center;gap:8px"><div style="width:6px;height:6px;background:{"#fff" if i == 0 else "transparent"}"></div>'
                     f'<span style="font-size:13px;color:{"#fff" if i == 0 else CLAIR};font-weight:{600 if i == 0 else 400}">{t}</span></div>'
                     for i, t in enumerate(("Entreprise", "Administrateur", "Taux et paiements", "Récapitulatif")))
    champ = lambda l: (f'<p style="font-size:13px;font-weight:500;margin-bottom:6px">{l}</p>'
                       f'<div style="height:34px;border:1px solid {TRAIT};border-radius:4px;margin-bottom:16px"></div>')
    c = f"""<div style="display:grid;grid-template-columns:1.45fr 1fr;gap:20px;flex:1">
<div style="background:{BRUME};border-radius:4px;padding:26px 30px 0;display:flex;flex-direction:column;gap:10px;position:relative">
<div style="background:#fff;border-radius:4px;box-shadow:0 10px 30px rgba(15,23,42,.14);overflow:hidden">
<div style="height:34px;display:flex;align-items:center;gap:8px;padding:0 12px;border-bottom:1px solid {TRAIT};font-size:12px">{img("kadenz-icone.svg", 16)}Kadenz
<span style="flex:1"></span><span style="color:{ARDOISE};letter-spacing:18px">-□x</span></div>
<div style="display:flex;height:330px">
<div style="width:190px;background:{NUIT};position:relative;overflow:hidden">{motif_calme(NUIT, "#232c6b", 190, 330, 32, [(0, 0, 190, 214)])}
<div style="position:relative;padding:22px 20px;display:flex;flex-direction:column;gap:12px"><div style="align-self:flex-start;padding:2px 4px">{img("kadenz-logo-horizontal-negatif.svg", 26)}</div>
<div style="height:8px"></div><div style="padding:6px;display:flex;flex-direction:column;gap:10px">{etapes}</div></div></div>
<div style="flex:1;padding:24px 28px;display:flex;flex-direction:column">
<p style="font-family:Space Grotesk;font-weight:700;font-size:22px;margin-bottom:6px">Première installation</p>
<p class="petit" style="font-size:13px;margin-bottom:18px">Étape 1 sur 4 : Entreprise</p>
{champ("Nom de l'entreprise (obligatoire)")}{champ("Adresse / boîte postale")}
<div style="flex:1"></div><div style="align-self:flex-end;background:{BLEU};color:#fff;font-weight:600;font-size:14px;padding:9px 22px;border-radius:4px">Suivant</div>
</div></div></div>
<p class="petit">Assistant de premier lancement, libellés réels de l'application</p>
<div style="margin-top:auto;height:44px;border-top:1px solid {TRAIT};display:flex;align-items:center;justify-content:center;gap:12px">
<div style="width:28px;height:28px;background:#cfd4dc;border-radius:4px"></div><div style="width:28px;height:28px;background:#cfd4dc;border-radius:4px"></div>
<div style="background:{PALE};padding:4px;border-radius:4px">{img("kadenz-icone.svg", 28, "display:block")}</div><div style="width:28px;height:28px;background:#cfd4dc;border-radius:4px"></div></div>
</div>
<div style="display:flex;flex-direction:column;gap:12px">
<div class="case" style="height:220px;background:{NUIT};overflow:hidden;justify-content:flex-start">{motif_calme(NUIT, "#232c6b", 436, 220, 220 / 6, [(0, 30, 320, 190)])}
<div style="position:relative;padding:18px 22px;margin-left:24px;display:flex;flex-direction:column;gap:10px">
{img("kadenz-logo-horizontal-negatif.svg", 44)}<span style="color:#fff;font-size:14px;line-height:1.4">Gestion commerciale pour<br>Windows. Rust, Slint, SQLite.</span></div></div>
<p class="petit">Aperçu social du dépôt GitHub, 1280 x 640 px</p>
<div style="display:grid;grid-template-columns:1fr 1fr;gap:12px;flex:1">
<div class="case" style="background:#1f2933;flex-direction:row;gap:26px">
<div style="display:flex;flex-direction:column;align-items:center;gap:6px">{img("kadenz-icone.svg", 56)}<span style="color:#fff;font-size:12px">Kadenz</span></div>
<div style="display:flex;flex-direction:column;align-items:center;gap:6px">{img("kadenz-icone.svg", 56)}<span style="color:#fff;font-size:12px">kadenz.exe</span></div>
<span class="leg" style="color:#c9ced6">Bureau : raccourci et exécutable</span></div>
<div class="case bord" style="flex-direction:column;align-items:flex-start;justify-content:flex-start;padding:20px;gap:12px">
{img("kadenz-logo-horizontal.svg", 30)}<p class="petit" style="font-size:13px">Logiciel propriétaire, tous droits réservés.</p>
<p class="petit">[Version]</p><span class="leg">Écran À propos</span></div>
</div></div></div>"""
    return planche(9, "Brand Mockups", "La marque là où elle apparaît vraiment : poste Windows, assistant d'installation, dépôt GitHub", c)


PLANCHES = [("01-primary-logo", "Kadenz, logo principal", p01), ("02-logomark", "Kadenz, symbole", p02),
            ("03-tone-mood", "Kadenz, ton et ambiance", p03), ("04-color-system", "Kadenz, système de couleurs", p04),
            ("05-brand-pattern", "Kadenz, motif de marque", p05), ("06-typography", "Kadenz, typographie", p06),
            ("07-signage", "Kadenz, signalétique", p07), ("08-stationery", "Kadenz, papeterie", p08),
            ("09-brand-mockups", "Kadenz, mises en situation", p09)]


if __name__ == "__main__":
    (ICI / "images").mkdir(exist_ok=True)
    toutes = []
    for nom, titre, f in PLANCHES:
        html = f()
        toutes.append(html)
        (ICI / f"{nom}.html").write_text(TETE.replace("{titre}", titre) + html + PIED, encoding="utf-8")
        subprocess.run([CHROME, "--headless=new", "--disable-gpu", "--hide-scrollbars", "--force-device-scale-factor=2",
                        "--window-size=1200,760", "--virtual-time-budget=4000",
                        f"--screenshot={ICI / 'images' / (nom + '.png')}", (ICI / f"{nom}.html").as_uri()],
                       check=True, capture_output=True)
    (ICI / "index.html").write_text(TETE.replace("{titre}", "Kadenz, identité Quatre pièces") + "\n".join(toutes) + PIED, encoding="utf-8")
    subprocess.run([CHROME, "--headless=new", "--disable-gpu", "--no-pdf-header-footer", "--virtual-time-budget=6000",
                    f"--print-to-pdf={ICI / 'kadenz-identite.pdf'}", (ICI / "index.html").as_uri()], check=True, capture_output=True)
    print("ok")
