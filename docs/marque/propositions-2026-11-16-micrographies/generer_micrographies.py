"""Micrographies Kadenz, propositions du 2026-11-16.

Petites compositions typographiques (étiquettes, fiches, écussons) dans l'esprit des références
fournies par l'utilisateur, refaites avec la charte de Kadenz : Nuit, blanc, Ambre pour le
troisième temps, Inter et Space Grotesk, les quatre pièces et les motifs.

Règles suivies :
- tous les textes viennent du dépôt (README, cahier des charges, charte, code du domaine) ;
  aucun lieu, aucune date de fondation, aucune mention de dépôt de marque (ni R cerclé ni TM) ;
- le mot et le symbole sont les tracés de la charte (kadenz-mot-negatif.svg,
  kadenz-symbole-negatif.svg), jamais retapés ni déformés ; les effets (points, rayures)
  ne touchent que les chiffres et les autres mots ;
- ni italique, ni tiret cadratin, ni barre d'accent sur un seul côté.

Usage : py -3.14 generer_micrographies.py (fontTools requis, pour generer.py)
Sorties : index.html, propositions-micrographies.png et micrographies/M1-nuit.png à M10-une-couleur.png (Chrome requis).
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
LARGEUR_PAGE = 2640


def charger(nom, chemin):
    spec = importlib.util.spec_from_file_location(nom, chemin)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


if len(sys.argv) < 2:   # generer.py lit la racine du dépôt dans sys.argv[1]
    sys.argv.append(str(MARQUE.parent.parent))
gm = charger("generer_marque", MARQUE / "generer.py")


# --- tracés de la charte, recolorés par currentColor ----------------------------------------
def tracé(fichier):
    s = (MARQUE / fichier).read_text(encoding="utf-8")
    vb = re.search(r'viewBox="([^"]+)"', s).group(1)
    corps = re.sub(r"<title>.*?</title>", "", s.split(">", 1)[1].rsplit("</svg>", 1)[0])
    return vb, corps.replace('fill="#FFFFFF"', 'fill="currentColor"')


VB_MOT, MOT = tracé("kadenz-mot-negatif.svg")
VB_SYM, SYM = tracé("kadenz-symbole-negatif.svg")


def mot(largeur):
    """Le mot de la charte (marge de 16 comprise dans le tracé, rognée ici)."""
    x, y, w, h = (float(v) for v in VB_MOT.split())
    vb = f"{x + 16} {y + 16} {w - 32} {h - 32}"
    return f'<svg class="mot" viewBox="{vb}" style="width:{largeur}px" aria-label="kadenz">{MOT}</svg>'


def symbole(taille):
    return f'<svg viewBox="16 14 226 228" width="{taille}" height="{taille}" aria-hidden="true">{SYM}</svg>'


def piece(sorte, taille, plein=True, trait=None):
    """Une des quatre pièces, sur une case de 100."""
    d = {
        "carre": "M0 0 H100 V100 H0 Z",
        "quart": "M0 100 V0 A100 100 0 0 1 100 100 Z",
        "triangle": "M0 0 L100 100 H0 Z",
        "disque": "M0 50 A50 50 0 1 1 100 50 A50 50 0 1 1 0 50 Z",
    }[sorte]
    if plein:
        dessin = f'<path d="{d}" fill="currentColor"/>'
    else:
        e = trait or 6
        dessin = (f'<path d="{d}" fill="none" stroke="currentColor" stroke-width="{e}" '
                  f'stroke-linejoin="miter" transform="translate({e / 2} {e / 2}) scale({(100 - e) / 100})"/>')
    return f'<svg viewBox="0 0 100 100" width="{taille}" height="{taille}" aria-hidden="true">{dessin}</svg>'


ORDRE = ("carre", "quart", "triangle", "disque")


def frise(n, taille, jeu):
    return (f'<div class="frise" style="gap:{jeu}px">'
            + "".join(piece(ORDRE[i % 4], taille) for i in range(n)) + "</div>")


def temps(l=40, e=6, g=10):
    """Les trois temps : blanc, blanc, Ambre (une seule encre dans la version une couleur)."""
    return (f'<div class="temps" style="gap:{g}px">'
            f'<i style="width:{l}px;height:{e}px"></i><i style="width:{l}px;height:{e}px"></i>'
            f'<i class="ambre" style="width:{l}px;height:{e}px"></i></div>')


def motif(fonction, w, h, **k):
    s = fonction("none", "currentColor", w, h, **k)
    return s.replace("<svg ", f'<svg class="motif" width="{w}" height="{h}" ', 1)


# --- chiffres en points (matrice 5 x 7) ------------------------------------------------------
POINTS = {
    "F": ["11111", "10000", "10000", "11110", "10000", "10000", "10000"],
    "0": ["01110", "10001", "10001", "10001", "10001", "10001", "01110"],
    "1": ["00100", "01100", "00100", "00100", "00100", "00100", "01110"],
    "2": ["01110", "10001", "00001", "00010", "00100", "01000", "11111"],
    "6": ["00110", "01000", "10000", "11110", "10001", "10001", "01110"],
    "-": ["00000", "00000", "00000", "11111", "00000", "00000", "00000"],
}


def matrice(texte, pas):
    """Les cases pleines d'une même ligne se rejoignent en barres arrondies, comme les références."""
    r = pas * 0.36
    out, x0 = [], 0
    for ch in texte:
        for li, ligne in enumerate(POINTS[ch]):
            col = 0
            while col < 5:
                if ligne[col] == "1":
                    fin = col
                    while fin + 1 < 5 and ligne[fin + 1] == "1":
                        fin += 1
                    x = (x0 + col) * pas + pas / 2 - r
                    w = (fin - col) * pas + 2 * r
                    y = li * pas + pas / 2 - r
                    out.append(f'<rect x="{x:.1f}" y="{y:.1f}" width="{w:.1f}" height="{2 * r:.1f}" rx="{r:.1f}"/>')
                    col = fin + 1
                else:
                    col += 1
        x0 += 6
    lw, lh = (x0 - 1) * pas, 7 * pas
    return (f'<svg class="matrice" viewBox="0 0 {lw:.1f} {lh:.1f}" width="{lw:.0f}" height="{lh:.0f}" '
            f'fill="currentColor" aria-label="{texte}">{"".join(out)}</svg>')


# --- les micrographies -----------------------------------------------------------------------
M = []


def micro(code, titre, usages, corps):
    M.append((code, titre, usages, corps))


# M1 Fiche technique (référence AI-OPS)
micro("M1", "Fiche technique", "Couverture de présentation, page « À propos » du futur site, affiche", f"""
<div class="pile" style="gap:44px">
  {frise(13, 40, 8)}
  <div class="grille2">
    <div class="paire"><b>Base SQLite chiffrée</b><span>Sauvegarde automatique, 10 copies</span></div>
    <div class="paire"><b>Licence perpétuelle</b><span>Liée au poste</span></div>
    <div class="paire"><b>Windows 10 | 11</b><span>64 bits</span></div>
    <div class="paire"><b>Interface en 7 langues</b><span>Documents clients en français</span></div>
  </div>
  {mot(640)}
  <div class="rang" style="align-items:stretch;gap:28px">
    <p class="tech" style="flex:1">Logiciel desktop Windows de gestion commerciale : devis, proforma, bons de
    commande, bordereaux de livraison, factures, paiements. Mono-poste, installé chez chaque entreprise cliente.</p>
    <div class="boite" style="width:250px">
      <div class="case" style="padding:14px">{symbole(70)}</div>
      <div class="pile" style="flex:1"><div class="case txt">Mcarre</div><div class="case txt">v0.2.2</div></div>
    </div>
  </div>
  <div class="plein rang entre"><b>Gestion commerciale</b><span class="pointille"></span><b>FCFA, sans décimale</b></div>
  <div class="rang entre pied"><span>Logiciel propriétaire</span><span>/</span><span>Tous droits réservés</span><span>/</span><span>Mcarre</span></div>
</div>""")

# M2 Pièces et mot (référence LAB)
micro("M2", "Pièces et mot", "Bandeau du site, en-tête de diapositive, page de garde du manuel", f"""
<div class="pile" style="gap:34px">
  <div class="rang" style="gap:26px;align-items:flex-end">
    <div class="damier">{''.join(piece(ORDRE[i % 4], 52) for i in range(8))}</div>
    {mot(420)}
  </div>
  <div class="rang entre caps m"><span>Gestion commerciale</span><span>&gt;&gt;&gt;</span><span>Windows 10 | 11</span></div>
  <div class="rang entre caps m"><span>Rust édition 2024</span>{symbole(48)}<span class="plein court">SQLite chiffrée</span></div>
  <div class="rang caps m"><span>Préfixes : D · P · C · L · F · A</span></div>
</div>""")

# M3 Écusson (référence BRS·HDS)
micro("M3", "Écusson", "Autocollant, tee-shirt d'équipe, comptoir du stand", f"""
<div class="pile centre" style="gap:40px">
  <div class="rang entre caps s" style="width:100%"><span>Gestion commerciale</span>{temps(34, 6, 8)}<span>Version 0.2.2</span></div>
  {mot(640)}
  <div class="rang entre caps m" style="width:100%"><span>Windows 10 | 11</span><span class="cadre">Mcarre</span><span>64 bits</span></div>
  <div class="rang entre caps m" style="width:100%"><span class="filet"></span><span style="font-size:17px">Rigoureux · régulier · clair · sobre · fiable</span><span class="filet"></span></div>
  <div class="rang entre" style="width:100%;align-items:center">
    <div class="rang" style="gap:10px">{''.join(piece(s, 46, s == 'carre') for s in ORDRE)}</div>
    {symbole(84)}
    <div class="caps m" style="line-height:1.1">Devis<br>Factures<br>Règlements</div>
    <div class="cadre-motif">{motif(gm.motif_tournant, 130, 100, s=25, graine=7)}</div>
  </div>
</div>""")

# M4 Bloc version (référence ace)
micro("M4", "Bloc version", "Pied de diapositive, notes de version, aperçu social du dépôt", f"""
<div class="pile" style="gap:30px">
  <div class="rang" style="gap:34px;align-items:center">
    {mot(250)}
    <div class="pile" style="gap:8px">
      <div class="gs" style="font-size:52px">0.2.2</div>
      <div class="ligne"></div>
      <div class="caps m">Version publiée</div>
    </div>
    <div class="rond">FCFA</div>
  </div>
  <div class="rang entre" style="align-items:flex-start">
    <div class="pile" style="gap:18px">
      <div class="moyen">Gestion commerciale pour Windows</div>
      <div class="rang caps s" style="gap:70px"><span>Base : SQLite chiffrée</span><span>Rust 2024</span></div>
    </div>
    <div class="moyen gros">Mcarre<br>Éditeur&nbsp;&nbsp;/</div>
  </div>
</div>""")

# M5 Chaîne en colonnes (référence RDG)
micro("M5", "Chaîne en colonnes", "Verso de carte, fond d'écran, intérieur de la chemise", f"""
<div class="pile mono" style="gap:70px">
  <div class="rang entre caps xl"><span>Gestion</span><span>commerciale</span></div>
  <div class="rang entre" style="align-items:center">
    <div class="caps l" style="line-height:1.25">Devis<br>Proforma<br>Bon de commande</div>
    <div class="fleche">{piece('carre', 26)}{piece('quart', 26)}{piece('triangle', 26)}{piece('disque', 26)}{piece('carre', 26)}</div>
    <div class="caps l droite" style="line-height:1.25">Bordereau<br>Facture<br>Avoir</div>
  </div>
  <div class="rang entre caps l"><span>Numéro : F2026-11-0001</span><span>/</span>{symbole(56)}</div>
</div>""")

# M6 FCFA (référence FUEL)
micro("M6", "FCFA", "Page Factures du futur site, diapositive sur les montants", f"""
<div class="pile" style="gap:26px">
  <div class="chevrons">{'&gt;' * 22}</div>
  <div class="gs geant">FCFA</div>
  <div class="moyen" style="font-size:44px;font-weight:400">Montants entiers, sans décimale</div>
  <div class="rang entre caps s"><span>Un seul arrondi</span><span class="crochets">[ Taux au centième de point ]</span></div>
  <div class="rang entre caps s"><span>Gestion commerciale</span><span>/</span><span>Kadenz 0.2.2</span></div>
  <div class="rang" style="gap:22px;align-items:stretch">
    <div class="plein carre-x">{piece('disque', 30)}</div>
    <div class="plein caps s" style="padding:14px 18px;display:flex;align-items:center">En lettres</div>
    <div class="boite" style="flex:1"><div class="inverse caps s">Facture</div><div class="caps s" style="padding:14px 16px;display:flex;align-items:center">F2026-11-0001</div></div>
  </div>
</div>""")

# M7 Matrice de points (référence BH OPS)
micro("M7", "Matrice de points", "Bandeau de section, étui du support d'installation, écran d'attente", f"""
<div class="pile" style="gap:28px">
  <div class="plein bande">{frise(22, 18, 9)}</div>
  <div class="points caps s"><span>Kadenz // gestion commerciale</span><i></i><span>Éditeur : Mcarre</span></div>
  <div class="points caps s"><span>SQLite chiffrée</span><i></i><span>Sauvegarde : 10 copies</span></div>
  <div class="pile" style="gap:22px;padding:18px 0">{matrice('F2026-11', 13.4)}{matrice('0001', 13.4)}</div>
  <div class="points caps s"><span>Préfixes : D P C L F A</span><i></i><span>Compteur mensuel</span></div>
  <div class="points caps s"><span>Interface : 7 langues</span><i></i><span>Documents en français</span></div>
  <div class="plein bande">{frise(22, 18, 9)}</div>
</div>""")

# M8 Ligne des pièces (référence DELIVERY SYSTEMS)
micro("M8", "Ligne des pièces", "Pied de page du futur site, fin de vidéo, signature de courriel", f"""
<div class="pile" style="gap:56px">
  <div class="rang entre"><span class="caps l">[Chaîne des documents]</span>{temps(46, 7, 12)}</div>
  <div class="rang entre" style="align-items:center">
    <div class="ligne-pieces">{''.join(piece(s, 96, False, 5) for s in ORDRE)}</div>
    <div class="moyen gros" style="font-size:40px;line-height:1.05">Du devis<br>au règlement</div>
  </div>
  <div class="boite grande">
    <div class="caps m" style="flex:1;padding:26px 30px">Pour Windows 10 | 11</div>
    <div class="inverse caps l" style="padding:26px 30px">&gt;&gt;&gt;</div>
    <div style="padding:16px 24px;display:flex;align-items:center">{symbole(60)}</div>
  </div>
</div>""")

# M9 Colonnes (référence ENGINEERED FOR)
micro("M9", "Colonnes", "Affiche A3, dos du carnet, mur du stand", f"""
<div class="grille3 mono caps m">
  <div>Gestion<br>commerciale</div><div class="col2">Éditeur Mcarre<br>Version 0.2.2</div>
  <div>Rigoureux<br>Régulier<br>Clair<br>Sobre<br>Fiable</div>
  <div class="tuile">{symbole(84)}</div>
  <div class="cadre-motif grand">{motif(gm.motif_tournant, 160, 120, s=40, graine=7)}</div>
  <div>SQLite<br>chiffrée</div><div>Licence<br>perpétuelle</div><div class="droite">7<br>langues</div>
</div>""")

# M10 Sept langues (référence INTERNAL RESEARCH UNIT)
micro("M10", "Sept langues", "Page Langues du futur site, diapositive, encart du manuel", f"""
<div class="trame">
  <div class="pile" style="gap:40px">
    <div class="rang" style="gap:40px;align-items:center">
      <div class="gs" style="font-size:50px;line-height:.98;text-transform:uppercase;white-space:nowrap">Interface<br>en sept<br>langues</div>
      <div class="pile" style="gap:16px">
        <div class="langues">{''.join(f'<span>{c}</span>' for c in ('FR', 'EN', 'AR', 'JA'))}</div>
        <div class="langues">{''.join(f'<span>{c}</span>' for c in ('ZH', 'TR', 'ES'))}<span class="vide">{piece('disque', 22)}</span></div>
      </div>
    </div>
    <div class="moyen" style="font-size:32px">Langue choisie par compte,<br>documents clients en français</div>
  </div>
</div>""")


# --- page ------------------------------------------------------------------------------------
CSS = """
@font-face{font-family:Inter;src:url(../planches/polices/Inter.woff2) format("woff2");font-weight:100 900}
@font-face{font-family:"Space Grotesk";src:url(../planches/polices/SpaceGrotesk.ttf) format("truetype");font-weight:300 700}
*{box-sizing:border-box;margin:0;padding:0}
.contenu{width:640px}
figure{margin:0}
em,i{font-style:normal}
html,body{color:#1F2933;font-family:Inter,sans-serif}
body.page{background:#F4F5F9;width:2640px;padding:80px}
body.seule{background:transparent;display:inline-block}
body.seule .carte{display:block}
body.seule .clair{background:transparent;border:none}
h1{font-family:"Space Grotesk";font-size:56px;font-weight:700;color:#1A2259;letter-spacing:-.01em}
.intro{font-size:24px;line-height:1.5;color:#52606D;max-width:1700px;margin-top:16px}
h2{font-family:"Space Grotesk";font-size:36px;font-weight:700;color:#1A2259;margin:70px 0 30px}
.planche{display:grid;grid-template-columns:repeat(3,736px);justify-content:space-between;align-items:start;gap:60px 40px;margin-top:50px}
.une-couleur{display:grid;grid-template-columns:repeat(5,368px);justify-content:space-between;align-items:start;gap:40px 20px}
.une-couleur .carte{zoom:.5}
.legende{margin-top:16px;font-size:22px;line-height:1.4}
.legende b{color:#1A2259;font-weight:700}
.legende span{color:#52606D}
.une-couleur .legende{font-size:18px;margin-top:8px}
.notes{grid-column:span 2;background:#FFFFFF;border:1px solid #D9DCE3;padding:50px 56px;font-size:23px;line-height:1.55}
.notes h3{font-family:"Space Grotesk";font-size:30px;color:#1A2259;margin:0 0 14px}
.notes h3+ul{margin-bottom:34px}
.notes ul{padding-left:28px}

/* carte : jetons de couleur, version Nuit par défaut */
.carte{--fond:#1A2259;--encre:#FFFFFF;--ambre:#E3A23B;--doux:#C5CAE0;
  display:inline-block;vertical-align:top;background:var(--fond);color:var(--encre);padding:48px;overflow:hidden}
.clair{--fond:#FFFFFF;--encre:#1A2259;--ambre:#1A2259;--doux:#1A2259;border:1px solid #D9DCE3}
.carte svg{display:block;flex:none}
.pile{display:flex;flex-direction:column}
.centre{align-items:center}
.rang{display:flex;align-items:center}
.entre{justify-content:space-between}
.droite{text-align:right}
.grille2{display:grid;grid-template-columns:1fr 1fr;gap:34px 30px}
.paire{display:flex;flex-direction:column;font-family:"Space Grotesk";font-size:27px;font-weight:700;line-height:1.12;letter-spacing:-.01em}
.paire span{color:var(--doux)}
.frise{display:flex;flex-wrap:nowrap}
.temps{display:flex}
.temps i{display:block;background:var(--encre)}
.temps i.ambre{background:var(--ambre)}
.tech{font-variant-numeric:tabular-nums;font-size:17px;line-height:1.5;letter-spacing:.02em;color:var(--doux)}
.boite{display:flex;border:3px solid var(--encre)}
.boite>.case{border-right:3px solid var(--encre);display:flex;align-items:center}
.boite .pile .case{flex:1;display:flex;align-items:center;padding:0 18px;font-family:"Space Grotesk";font-weight:700;font-size:26px}
.boite .pile .case+.case{border-top:3px solid var(--encre)}
.plein{background:var(--encre);color:var(--fond)}
.plein.rang{padding:22px 26px;font-family:"Space Grotesk";font-size:27px;font-weight:700}
.pointille{flex:1;margin:0 26px;border-top:3px dashed var(--fond);opacity:.7}
.pied{font-family:"Space Grotesk";font-weight:700;font-size:24px;text-transform:uppercase;letter-spacing:-.01em}
.caps{white-space:nowrap;text-transform:uppercase;font-weight:600;letter-spacing:.08em;font-variant-numeric:tabular-nums}
.caps.s{font-size:19px}.caps.m{font-size:24px}.caps.l{font-size:30px}.caps.xl{font-size:44px;font-weight:500;letter-spacing:.12em}
.mono .caps,.mono.caps{font-weight:500}
.court{padding:12px 20px;font-weight:600}
.damier{display:grid;grid-template-columns:repeat(4,52px);gap:6px}
.cadre{border:3px solid var(--encre);padding:10px 24px}
.filet{display:block;width:40px;height:4px;background:var(--encre)}
.cadre-motif{border:3px solid var(--encre);overflow:hidden;display:flex}
.cadre-motif .motif{display:block}
.gs{font-family:"Space Grotesk";font-weight:700;letter-spacing:-.02em;font-variant-numeric:tabular-nums}
.ligne{height:3px;background:var(--encre)}
.rond{flex:none;width:124px;height:124px;border:4px solid var(--encre);border-radius:50%;display:flex;align-items:center;justify-content:center;font-family:"Space Grotesk";font-weight:700;font-size:32px}
.moyen{font-family:"Space Grotesk";font-weight:700;font-size:32px;letter-spacing:-.015em;line-height:1.15}
.moyen.gros{font-size:46px;line-height:1.05}
.fleche{display:grid;grid-template-columns:26px 26px 26px;grid-template-rows:repeat(5,26px);gap:8px}
.fleche svg:nth-child(1){grid-area:1/1}.fleche svg:nth-child(2){grid-area:2/2}.fleche svg:nth-child(3){grid-area:3/3}
.fleche svg:nth-child(4){grid-area:4/2}.fleche svg:nth-child(5){grid-area:5/1}
.chevrons{font-family:"Space Grotesk";font-weight:700;font-size:40px;letter-spacing:.06em;line-height:1;white-space:nowrap;overflow:hidden}
.geant{font-size:236px;line-height:.82;letter-spacing:-.04em}
.crochets{white-space:nowrap}
.carre-x{display:flex;align-items:center;justify-content:center;padding:0 16px}
.inverse{background:var(--encre);color:var(--fond);padding:14px 22px;display:flex;align-items:center}
.boite.grande .inverse{padding:26px 30px}
.bande{padding:14px 18px;display:flex}
.bande .frise{width:100%;justify-content:space-between}
.points{display:flex;align-items:baseline;gap:8px}
.points i{flex:1;border-bottom:3px dotted var(--encre);opacity:.8;transform:translateY(-5px)}
.matrice{margin:0 auto}
.ligne-pieces{display:flex;position:relative;padding:0 34px}
.ligne-pieces::before{content:"";position:absolute;left:0;right:0;top:50%;height:4px;margin-top:-2px;background:var(--encre)}
.ligne-pieces svg{margin-left:-22px;position:relative}
.ligne-pieces svg:first-child{margin-left:0}
.grille3{display:grid;grid-template-columns:250px 160px 160px;justify-content:space-between;gap:60px 34px;align-items:start}
.grille3 .col2{grid-column:span 2}
.grille3 .tuile{border:4px solid var(--encre);padding:18px;justify-self:start}
.grille3 .cadre-motif.grand{justify-self:end}
.trame{position:relative}
.langues{display:flex;gap:10px}
.langues span{width:62px;height:62px;border:3px solid var(--encre);display:flex;align-items:center;justify-content:center;
  font-family:"Space Grotesk";font-weight:700;font-size:24px;letter-spacing:.02em}
.langues span.vide{border-color:transparent;color:var(--ambre)}
"""


MARGE = 48          # marge égale sur les quatre côtés, comme un tampon
CONTENU = 640       # largeur du contenu de chaque micrographie


def carte(corps, classe=""):
    libre = ' style="width:auto"' if 'class="trame"' in corps else ""   # M10 : la trame suit son contenu
    return f'<div class="carte {classe}"><div class="contenu"{libre}>{corps}</div></div>'


MESURE = """<script>document.fonts.ready.then(()=>{const c=document.querySelector('body.seule .carte');
const r=(c||document.body).getBoundingClientRect();
document.body.setAttribute('data-mesure',Math.ceil(r.height)+'x'+Math.ceil(r.width));});</script>"""


def document(corps, classe_body="", titre="Micrographies Kadenz"):
    return (f'<!doctype html>\n<html lang="fr"><head><meta charset="utf-8"><title>{titre}</title>'
            f'<style>{CSS}</style></head><body class="{classe_body}">{corps}{MESURE}</body></html>\n')


def page():
    sombres = "".join(
        f'<figure>{carte(corps)}<figcaption class="legende"><b>{code} {titre}</b><br><span>{usages}</span></figcaption></figure>'
        for code, titre, usages, corps in M)
    notes = """
<div class="notes">
  <h3>Format</h3>
  <ul>
    <li>Chaque micrographie est rognée sur son contenu, avec la même marge sur les quatre côtés, comme un tampon.</li>
    <li>Chacune existe aussi en image séparée, dans le dossier micrographies : version Nuit et version une couleur
        sur fond transparent.</li>
  </ul>
  <h3>Sources des textes</h3>
  <ul>
    <li>README et cahier des charges : gestion commerciale, Windows 10 | 11 64 bits, SQLite chiffrée,
        sauvegarde automatique de 10 copies, licence perpétuelle liée au poste, FCFA sans décimale, sept langues.</li>
    <li>Code du domaine : préfixes D P C L F A et numéro au compteur mensuel par défaut (F2026-11-0001).</li>
    <li>Charte : les cinq mots du ton, Mcarre, « Logiciel propriétaire, tous droits réservés », version 0.2.2.</li>
  </ul>
  <h3>Écarts aux références, voulus</h3>
  <ul>
    <li>Ni R cerclé ni TM : aucune recherche de marque n'a été faite. Le symbole prend leur place.</li>
    <li>Ni lieu, ni année de fondation, ni « status active » : rien de tel dans le dépôt.</li>
    <li>Mot et symbole jamais retapés ni en points : les effets ne touchent que les chiffres.</li>
  </ul>
</div>"""
    clairs = "".join(
        f'<figure>{carte(corps, "clair")}<figcaption class="legende"><b>{code}</b> <span>{titre}</span></figcaption></figure>'
        for code, titre, usages, corps in M)
    return document(f"""
<h1>Micrographies Kadenz, propositions du 2026-11-16</h1>
<p class="intro">Dix compositions dans l'esprit des références fournies, rognées comme des tampons : étiquettes,
fiches et écussons à poser sur les supports de la marque. Fond Nuit, encre blanche, Ambre seulement pour le
troisième temps ; en bas, la même série en une couleur pour l'impression.</p>
<div class="planche">{sombres}{notes}</div>
<h2>Version une couleur</h2>
<div class="une-couleur">{clairs}</div>""", "page")


def chrome(*args):
    return subprocess.run([CHROME, "--headless=new", "--hide-scrollbars", "--virtual-time-budget=4000", *args],
                          check=True, capture_output=True, text=True, encoding="utf-8").stdout


def mesurer(html):
    m = re.search(r'data-mesure="(\d+)x(\d+)"', chrome("--window-size=2640,1000", "--dump-dom", html.as_uri()))
    return int(m.group(1)), int(m.group(2))


def photographier(html, png, largeur, hauteur, echelle=1, transparent=False):
    extra = ["--default-background-color=00000000"] if transparent else []
    chrome(*extra, f"--force-device-scale-factor={echelle}", f"--window-size={largeur},{hauteur}",
           f"--screenshot={png}", html.as_uri())


if __name__ == "__main__":
    html = ICI / "index.html"
    html.write_text(page(), encoding="utf-8")
    png = ICI / "propositions-micrographies.png"
    hauteur, _ = mesurer(html)
    photographier(html, png, LARGEUR_PAGE, hauteur)
    print(png)

    # une image par micrographie, rognée au bord de la carte (deux fois la taille, pour l'impression)
    dossier = ICI / "micrographies"
    dossier.mkdir(exist_ok=True)
    tmp = ICI / "_seule.html"
    for code, titre, usages, corps in M:
        for classe, suffixe, transparent in (("", "nuit", False), ("clair", "une-couleur", True)):
            tmp.write_text(document(carte(corps, classe), "seule", f"{code} {titre}"), encoding="utf-8")
            h, w = mesurer(tmp)
            sortie = dossier / f"{code}-{suffixe}.png"
            photographier(tmp, sortie, w, h, 2, transparent)
            print(sortie, f"{w} x {h}")
    tmp.unlink()
