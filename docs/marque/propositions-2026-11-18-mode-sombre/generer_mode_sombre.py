"""Mode sombre général de Kadenz : trois palettes neutres, appliquées à toute
l'application (demande de l'utilisateur du 2026-11-18 : « pas un truc bleu foncé,
mais sombre, grisâtre »).

Chaque palette donne une valeur sombre à chaque jeton de Theme
(crates/app/ui/composants.slint) ; les écrans montrés sont la liste des factures,
une fiche de facture avec son aperçu, les Paramètres (Navigation et affichage) et
la connexion. Libellés tirés de l'application ; lignes de facture : données
d'exemple. L'aperçu des documents reste blanc (c'est du papier).

Usage : py -3.13 generer_mode_sombre.py   (Chrome requis)
"""
import re
import subprocess
import sys
from pathlib import Path

ICI = Path(__file__).resolve().parent
sys.path.insert(0, str(ICI.parent / "propositions-2026-11-18-refonte-ecrans"))
from generer_refonte import contraste, trame  # noqa: E402

MARQUE = ICI.parent
CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
LARGEUR_PLANCHE = 2640
W, H = 1200, 760

CLAIR = dict(nom="Clair (actuel)", fond="#FFFFFF", lateral="#F4F5F9", surface="#FFFFFF", champ="#FFFFFF",
             bordure="#D9DCE3", contour="#7B8794", texte="#1F2933", second="#52606D", accent="#2D3A8C",
             sur_accent="#FFFFFF", survol="#E8EAF6", panneau="#1A2259", erreur="#B42318", attente="#B54708",
             succes="#067647", fond_erreur="#FEF3F2", fond_attente="#FFF8EB", fond_succes="#ECFDF3",
             logo="kadenz-logo-horizontal.svg")

PALETTES = [
    dict(cle="graphite", nom="A. Graphite", idee="Gris neutres, sans teinte ; Ambre pour l'action.",
         fond="#17181B", lateral="#1E2024", surface="#1E2024", champ="#26282D", bordure="#33363C", contour="#6E737D",
         texte="#ECEDEF", second="#A3A8B0", accent="#E3A23B", sur_accent="#17181B", survol="#2B2E34",
         panneau="#111214", erreur="#F97066", attente="#FDB022", succes="#47CD89", fond_erreur="#3A2224",
         fond_attente="#3A3020", fond_succes="#1E3329", logo="kadenz-logo-horizontal-negatif.svg"),
    dict(cle="ardoise", nom="B. Ardoise", idee="Gris un peu froids, tirés d'Encre et d'Ardoise (charte) ; Indigo éclairci pour l'action.",
         fond="#15191E", lateral="#1C2229", surface="#1F2933", champ="#27313C", bordure="#334050", contour="#6F7D8C",
         texte="#F4F5F9", second="#A9B4C0", accent="#8F9BEA", sur_accent="#15191E", survol="#2A3542",
         panneau="#101317", erreur="#F97066", attente="#FDB022", succes="#47CD89", fond_erreur="#3A2226",
         fond_attente="#3A3122", fond_succes="#1D332B", logo="kadenz-logo-horizontal-negatif.svg"),
    dict(cle="anthracite", nom="C. Anthracite chaud", idee="Gris légèrement chauds ; Ambre pour l'action.",
         fond="#1A1918", lateral="#211F1E", surface="#232120", champ="#2B2927", bordure="#383533", contour="#77716B",
         texte="#EEEBE7", second="#ADA69F", accent="#E3A23B", sur_accent="#1A1918", survol="#302D2A",
         panneau="#131211", erreur="#F97066", attente="#FDB022", succes="#47CD89", fond_erreur="#3B2322",
         fond_attente="#3B311F", fond_succes="#20322A", logo="kadenz-logo-horizontal-negatif.svg"),
]

NAV = [("", ["Tableau de bord"]),
       ("Documents", ["Devis", "Proforma", "Bons de commande", "Livraisons", "Factures", "Avoirs"]),
       ("Paiements", ["Règlements", "Suivi fournisseurs", "Relances"]),
       ("Référentiels", ["Tiers", "Produits", "Stock"]),
       ("", ["Comptabilité", "Paramètres"])]

LIGNES = [  # données d'exemple
    ("F2026-11-0007", "17/11/2026", "Client A", "Validé", "17/12/2026", "Impayé", "1 250 000"),
    ("F2026-11-0006", "16/11/2026", "Client B", "Validé", "16/12/2026", "Partiel", "486 500"),
    ("F2026-11-0005", "14/11/2026", "Client C", "Validé", "14/12/2026", "Payé", "92 000"),
    ("F2026-11-0004", "12/11/2026", "Client A", "Validé", "12/12/2026", "Payé", "2 310 000"),
    ("F2026-11-0003", "10/11/2026", "Client D", "Validé", "10/12/2026", "Payé", "175 400"),
    ("F2026-11-0002", "07/11/2026", "Client B", "Validé", "07/12/2026", "Impayé", "640 000"),
    ("", "18/11/2026", "Client E", "Brouillon", "", "", "58 000"),
]


def logo(p, h=26):
    s = (MARQUE / p["logo"]).read_text(encoding="utf-8")
    return s.replace("<svg ", f'<svg height="{h}" style="display:block" ', 1)


def css(p):
    return f"""
@font-face{{font-family:Inter;src:url(../planches/polices/Inter.woff2) format("woff2");font-weight:100 900}}
@font-face{{font-family:"Space Grotesk";src:url(../planches/polices/SpaceGrotesk.ttf) format("truetype");font-weight:300 700}}
*{{box-sizing:border-box;margin:0;padding:0}}
em,i{{font-style:normal}}
html,body{{width:{W}px;height:{H}px;overflow:hidden}}
body{{background:{p['fond']};color:{p['texte']};font:14px/1.4 Inter,sans-serif;display:flex}}
.nav{{width:236px;background:{p['lateral']};border-right:1px solid {p['bordure']};padding:22px 14px;flex:none}}
.nav .logo{{padding:0 10px 22px}}
.nav h6{{font:600 11px Inter;letter-spacing:.12em;text-transform:uppercase;color:{p['second']};padding:14px 10px 6px}}
.nav a{{display:block;padding:7px 10px;border-radius:4px;color:{p['texte']};font-size:14px}}
.nav a.on{{background:{p['accent']};color:{p['sur_accent']};font-weight:600}}
.page{{flex:1;padding:26px 32px;overflow:hidden;position:relative}}
.titre{{font:700 24px "Space Grotesk";margin-bottom:18px}}
.outils{{display:flex;gap:10px;align-items:center;margin-bottom:16px}}
.saisie{{background:{p['champ']};border:1px solid {p['contour']};border-radius:4px;height:36px;padding:8px 12px;color:{p['second']};font-size:14px}}
.bouton{{height:36px;padding:0 16px;border-radius:4px;display:inline-flex;align-items:center;font:600 14px Inter;white-space:nowrap}}
.principal{{background:{p['accent']};color:{p['sur_accent']}}}
.second{{background:{p['champ']};color:{p['texte']};border:1px solid {p['contour']}}}
.carte{{background:{p['surface']};border:1px solid {p['bordure']};border-radius:4px;overflow:hidden}}
table{{border-collapse:collapse;width:100%;font-size:13.5px}}
th{{text-align:left;font:600 12px Inter;color:{p['second']};padding:10px 14px;border-bottom:1px solid {p['bordure']};background:{p['lateral']}}}
td{{padding:10px 14px;border-bottom:1px solid {p['bordure']}}}
td.num{{text-align:right;font-variant-numeric:tabular-nums}}
tr.sel td{{background:{p['survol']}}}
.badge{{display:inline-block;padding:2px 8px;border-radius:3px;font-size:12px;font-weight:600}}
.b-erreur{{color:{p['erreur']};background:{p['fond_erreur']}}}
.b-attente{{color:{p['attente']};background:{p['fond_attente']}}}
.b-succes{{color:{p['succes']};background:{p['fond_succes']}}}
.b-neutre{{color:{p['second']};background:{p['champ']}}}
.second-txt{{color:{p['second']}}}
label span{{display:block;font-size:12px;color:{p['second']};margin-bottom:5px}}
.grille{{display:grid;grid-template-columns:1fr 1fr 1fr;gap:14px}}
.papier{{background:#FFFFFF;color:#1F2933;box-shadow:0 2px 14px rgba(0,0,0,.35);border-radius:2px}}
.choix{{display:flex;gap:12px}}
.option{{flex:1;border:1px solid {p['contour']};border-radius:4px;padding:14px 16px;background:{p['surface']}}}
.option.on{{border:2px solid {p['accent']};padding:13px 15px}}
.option b{{display:block;font-weight:600;margin-bottom:4px}}
.option small{{color:{p['second']};font-size:12.5px}}
.rond{{display:inline-block;width:14px;height:14px;border-radius:50%;border:2px solid {p['contour']};vertical-align:-2px;margin-right:8px}}
.option.on .rond{{border:4px solid {p['accent']}}}
.section{{font:700 16px "Space Grotesk";margin:22px 0 10px}}
"""


def nav(p, actif):
    out = [f'<div class="nav"><div class="logo">{logo(p)}</div>']
    for groupe, pages in NAV:
        if groupe:
            out.append(f"<h6>{groupe}</h6>")
        for pg in pages:
            out.append(f'<a class="{"on" if pg == actif else ""}">{pg}</a>')
    out.append("</div>")
    return "".join(out)


def badge(paiement):
    return {"Impayé": "b-erreur", "Partiel": "b-attente", "Payé": "b-succes"}.get(paiement, "")


def liste(p):
    lignes = []
    for k, (n, d, c, s, e, pay, m) in enumerate(LIGNES):
        st = f'<span class="badge b-neutre">{s}</span>'
        pa = f'<span class="badge {badge(pay)}">{pay}</span>' if pay else ""
        lignes.append(f'<tr class="{"sel" if k == 1 else ""}"><td>{n}</td><td>{d}</td><td>{c}</td><td>{st}</td>'
                      f'<td>{e}</td><td>{pa}</td><td class="num">{m}</td></tr>')
    corps = (f'<div class="page"><div class="titre">Factures</div>'
             f'<div class="outils"><div class="saisie" style="width:300px">Rechercher : numéro, client, libellé</div>'
             f'<div style="flex:1"></div><div class="bouton second">Exporter en Excel</div>'
             f'<div class="bouton principal">Nouvelle facture</div></div>'
             f'<div class="carte"><table><tr><th>Numéro</th><th>Date</th><th>Client</th><th>Statut</th><th>Échéance</th>'
             f'<th>Paiement</th><th style="text-align:right">Montant TTC</th></tr>{"".join(lignes)}</table></div></div>')
    return nav(p, "Factures") + corps


def fiche(p):
    lignes = "".join(f'<tr><td>{a}</td><td class="num">{q}</td><td class="num">{u}</td><td class="num">{t}</td></tr>'
                     for a, q, u, t in [("Article 1", "10", "12 500", "125 000"), ("Article 2", "4", "45 000", "180 000"),
                                        ("Article 3", "2", "90 750", "181 500")])
    papier = ('<div class="papier" style="position:absolute;right:32px;top:84px;width:300px;height:424px;padding:22px">'
              '<div style="font:700 15px Space Grotesk;color:#2D3A8C">Quincaillerie Démo</div>'
              '<div style="font-size:10px;color:#52606D;margin-top:4px">Facture F2026-11-0006</div>'
              + "".join(f'<div style="height:7px;background:#E6E8EE;margin-top:{14 if i == 0 else 8}px;width:{w}%"></div>'
                        for i, w in enumerate([100, 86, 92, 70, 100, 64, 80])) +
              '<div style="position:absolute;right:22px;bottom:26px;font:700 13px Inter">486 500</div></div>')
    corps = (f'<div class="page"><div class="titre">Facture F2026-11-0006</div>'
             f'<div style="width:470px"><div class="grille" style="grid-template-columns:1fr 1fr">'
             f'<label><span>Client</span><div class="saisie" style="color:{p["texte"]}">Client B</div></label>'
             f'<label><span>Date</span><div class="saisie" style="color:{p["texte"]}">16/11/2026</div></label>'
             f'<label><span>Mode de paiement</span><div class="saisie" style="color:{p["texte"]}">Virement bancaire</div></label>'
             f'<label><span>Échéance</span><div class="saisie" style="color:{p["texte"]}">16/12/2026</div></label></div>'
             f'<div class="section">Lignes</div><div class="carte"><table><tr><th>Désignation</th><th style="text-align:right">Qté</th>'
             f'<th style="text-align:right">Prix unitaire</th><th style="text-align:right">Total</th></tr>{lignes}</table></div>'
             f'<div style="display:flex;justify-content:space-between;margin-top:14px"><span class="second-txt">Total TTC</span>'
             f'<b style="font-size:16px">486 500</b></div>'
             f'<div style="display:flex;gap:10px;margin-top:22px"><div class="bouton second">Exporter en PDF</div>'
             f'<div class="bouton principal">Valider</div></div></div>'
             f'<div class="second-txt" style="position:absolute;right:32px;top:58px;font-size:12px">Aperçu</div>{papier}</div>')
    return nav(p, "Factures") + corps


def parametres(p):
    def option(t, s, on=False):
        return f'<div class="option {"on" if on else ""}"><b><span class="rond"></span>{t}</b><small>{s}</small></div>'
    corps = (f'<div class="page"><div class="titre">Paramètres</div>'
             f'<div style="display:flex;gap:28px"><div style="width:200px;flex:none">'
             + "".join(f'<div style="padding:8px 12px;border-radius:4px;{"background:" + p["survol"] + ";font-weight:600" if r == "Navigation et affichage" else ""}">{r}</div>'
                       for r in ["Mon compte", "Navigation et affichage", "Entreprise", "Documents", "Paiements",
                                 "Comptabilité", "Impression", "Utilisateurs", "Sauvegardes", "Licence"])
             + f'</div><div style="flex:1;max-width:640px">'
             f'<div class="section" style="margin-top:4px">Navigation</div>'
             f'<div class="choix">{option("Barre latérale", "Menu sur le côté, repliable avec Ctrl+B.", True)}{option("Bandeau en haut", "Menus déroulants en haut de la fenêtre.")}</div>'
             f'<div class="section">Mode d\'affichage</div>'
             f'<div class="second-txt" style="font-size:13px;margin-bottom:10px">Propre à ce poste : sur un autre poste, votre compte garde le mode qui y a été choisi.</div>'
             f'<div class="choix">{option("Comme Windows", "Clair ou sombre selon le réglage de Windows.")}'
             f'{option("Clair", "Fond blanc, toujours.")}{option("Sombre", "Fond sombre, toujours. Les documents et leur aperçu restent sur fond blanc.", True)}</div>'
             f'<div class="section">Animations</div>'
             f'<div style="display:flex;gap:10px;align-items:center"><span style="width:36px;height:20px;border-radius:10px;background:{p["accent"]};display:inline-block;position:relative">'
             f'<span style="position:absolute;right:3px;top:3px;width:14px;height:14px;border-radius:50%;background:{p["sur_accent"]}"></span></span>Animer les transitions, les survols et l\'ouverture des panneaux</div>'
             f'</div></div></div>')
    return nav(p, "Paramètres") + corps


def connexion(p):
    t = trame(560, H, "quart", 120, 150, 380, couleur="#E3A23B", fond_points=p["bordure"])
    return (f'<div style="width:640px;position:relative;background:{p["fond"]}">'
            f'<div style="position:absolute;left:72px;top:56px">{logo(p, 30)}</div>'
            f'<div style="position:absolute;left:72px;top:250px;width:360px">'
            f'<div style="font:600 12px Inter;letter-spacing:.14em;color:{p["second"]}">CONNEXION</div>'
            f'<div style="font:700 32px Space Grotesk;margin-top:14px">Quincaillerie Démo</div>'
            f'<label style="display:block;margin-top:32px"><span>Identifiant</span><div class="saisie" style="height:40px;border:2px solid {p["accent"]}"></div></label>'
            f'<label style="display:block;margin-top:16px"><span>Mot de passe</span><div class="saisie" style="height:40px"></div></label>'
            f'<div class="bouton principal" style="width:100%;justify-content:center;height:40px;margin-top:24px">Se connecter</div></div></div>'
            f'<div style="flex:1;background:{p["panneau"]};position:relative;overflow:hidden">{t}'
            f'<div style="position:absolute;left:48px;bottom:44px"><div style="font:700 22px Space Grotesk;color:#fff">Gestion commerciale</div>'
            f'<div style="font-size:12.5px;color:#C5CAE0;margin-top:6px">Version 1.0.0</div></div></div>')


def page(p, corps):
    return f'<!doctype html><html lang="fr"><head><meta charset="utf-8"><style>{css(p)}</style></head><body>{corps}</body></html>'


def chrome(*args):
    return subprocess.run([CHROME, "--headless=new", "--hide-scrollbars", "--virtual-time-budget=5000", *args],
                          check=True, capture_output=True, text=True, encoding="utf-8")


def rendre():
    d = ICI / "ecrans"
    d.mkdir(exist_ok=True)
    jeux = [("clair-factures", CLAIR, liste)]
    for p in PALETTES:
        for nom, f in [("factures", liste), ("fiche", fiche), ("parametres", parametres), ("connexion", connexion)]:
            jeux.append((f"{p['cle']}-{nom}", p, f))
    for nom, p, f in jeux:
        tmp = ICI / f"_{nom}.html"
        tmp.write_text(page(p, f(p)), encoding="utf-8")
        chrome("--force-device-scale-factor=1", f"--window-size={W},{H}", f"--screenshot={d / (nom + '.png')}", tmp.as_uri())
        tmp.unlink()
        print(nom)


def controles(p):
    paires = [("texte sur fond", "texte", "fond"), ("texte sur surface", "texte", "surface"),
              ("texte secondaire sur fond", "second", "fond"), ("texte secondaire sur champ", "second", "champ"),
              ("texte du bouton principal", "sur_accent", "accent"), ("contour de champ sur fond (3 : 1)", "contour", "fond"),
              ("contour de champ sur champ (3 : 1)", "contour", "champ"), ("erreur sur son fond", "erreur", "fond_erreur"),
              ("attente sur son fond", "attente", "fond_attente"), ("succes sur son fond", "succes", "fond_succes"),
              ("texte sur ligne choisie", "texte", "survol")]
    out = []
    for lib, a, b in paires:
        c = contraste(p[a], p[b])
        seuil = 3 if "3 : 1" in lib else 4.5
        out.append((lib, c, c >= seuil))
    return out


JETONS = [("fond", "fond"), ("fond-lateral (navigation, en-têtes)", "lateral"), ("surface (cartes, tableaux)", "surface"),
          ("champ", "champ"), ("bordure (séparateurs)", "bordure"), ("bordure-champ", "contour"), ("texte", "texte"),
          ("texte-secondaire", "second"), ("accent (bouton principal, sélection)", "accent"),
          ("texte-sur-accent", "sur_accent"), ("accent-survol (ligne choisie)", "survol"),
          ("nuit (panneau de marque)", "panneau"), ("erreur / fond-erreur", "erreur"), ("attente / fond-attente", "attente"),
          ("succes / fond-succes", "succes")]


def planche():
    sections = []
    for p in PALETTES:
        jt = "".join(
            f'<tr><td>{lib}</td><td><span class="pastille" style="background:{CLAIR[k]}"></span>{CLAIR[k]}</td>'
            f'<td><span class="pastille" style="background:{p[k]}"></span>{p[k]}'
            + (f' / <span class="pastille" style="background:{p["fond_" + k]}"></span>{p["fond_" + k]}' if k in ("erreur", "attente", "succes") else "")
            + "</td></tr>" for lib, k in JETONS)
        ct = "".join(f'<tr><td>{l}</td><td>{c:.1f} : 1</td><td>{"oui" if ok else "non"}</td></tr>' for l, c, ok in controles(p))
        imgs = "".join(f'<figure><img src="ecrans/{p["cle"]}-{n}.png" alt="{t}"><figcaption>{t}</figcaption></figure>'
                       for n, t in [("factures", "Liste des factures"), ("fiche", "Fiche de facture et aperçu"),
                                    ("parametres", "Paramètres, Navigation et affichage"), ("connexion", "Connexion")])
        sections.append(f'<section><header><h2>{p["nom"]}</h2><p>{p["idee"]}</p></header>'
                        f'<div class="rang">{imgs}</div>'
                        f'<div class="deux"><table><tr><th>Jeton</th><th>Clair</th><th>Sombre</th></tr>{jt}</table>'
                        f'<table><tr><th>Contraste</th><th>Rapport</th><th>Seuil atteint</th></tr>{ct}</table></div></section>')
    return f"""<!doctype html><html lang="fr"><head><meta charset="utf-8"><title>Kadenz, mode sombre</title>
<meta name="robots" content="noindex"><style>
@font-face{{font-family:Inter;src:url(../planches/polices/Inter.woff2) format("woff2");font-weight:100 900}}
@font-face{{font-family:"Space Grotesk";src:url(../planches/polices/SpaceGrotesk.ttf) format("truetype");font-weight:300 700}}
*{{box-sizing:border-box;margin:0;padding:0}} em,i{{font-style:normal}}
body{{background:#F4F5F9;color:#1F2933;font:18px/1.5 Inter,sans-serif;width:{LARGEUR_PLANCHE}px;padding:96px 120px}}
h1,h2{{font-family:"Space Grotesk";font-weight:700}}
.tete{{display:flex;justify-content:space-between;align-items:flex-end;border-bottom:1px solid #D9DCE3;padding-bottom:40px;margin-bottom:64px}}
.tete h1{{font-size:64px;line-height:1}} .tete p{{color:#52606D;font-size:20px;text-align:right}}
.sur{{font:600 15px Inter;letter-spacing:.14em;text-transform:uppercase;color:#2D3A8C;margin-bottom:18px}}
section{{margin-bottom:100px}} header{{display:flex;gap:24px;align-items:baseline;margin-bottom:28px}}
h2{{font-size:40px}} header p{{color:#52606D;font-size:20px}}
.rang{{display:grid;grid-template-columns:repeat(4,1fr);gap:24px}}
figure{{background:#fff;border:1px solid #D9DCE3;border-radius:4px;overflow:hidden}} figure img{{display:block;width:100%}}
figcaption{{padding:10px 16px 12px;font:600 15px Inter}}
.deux{{display:grid;grid-template-columns:1.2fr 1fr;gap:40px;margin-top:28px;align-items:start}}
table{{border-collapse:collapse;width:100%;font-size:16px;background:#fff;border:1px solid #D9DCE3}}
th{{text-align:left;font:600 13px Inter;letter-spacing:.1em;text-transform:uppercase;color:#52606D;padding:10px 14px;background:#F4F5F9;border-bottom:1px solid #D9DCE3}}
td{{padding:8px 14px;border-bottom:1px solid #E6E8EE}}
.pastille{{display:inline-block;width:20px;height:20px;border-radius:3px;border:1px solid #D9DCE3;vertical-align:-4px;margin-right:10px}}
.ref{{display:grid;grid-template-columns:1fr 3fr;gap:40px;align-items:start;margin-bottom:100px}}
.ref ul{{list-style:none}} .ref li{{position:relative;padding-left:26px;margin-bottom:12px;font-size:19px}}
.ref li::before{{content:"";position:absolute;left:0;top:11px;width:10px;height:10px;background:#2D3A8C}}
</style></head><body>
<div class="tete"><div><p class="sur" style="text-align:left">Kadenz, propositions du 2026-11-18</p><h1>Mode sombre général : trois gris</h1></div>
<p>Toute l'application, pas seulement les écrans de premier lancement<br>Libellés de l'application ; lignes de facture : données d'exemple</p></div>
<div class="ref"><figure><img src="ecrans/clair-factures.png" alt="Clair"><figcaption>Référence : mode clair actuel</figcaption></figure>
<ul><li>Aucune des trois palettes n'est bleu foncé : gris neutres (A), gris un peu froids tirés d'Encre et d'Ardoise de la charte (B), gris un peu chauds (C).</li>
<li>Le bouton principal et l'élément choisi prennent l'accent : Ambre en A et C, Indigo éclairci en B (l'Indigo de la marque ne se lit pas sur un fond sombre).</li>
<li>Le panneau de marque des écrans de connexion devient un gris presque noir, avec la même trame : plus de Nuit en mode sombre.</li>
<li>L'aperçu des documents et les PDF restent blancs : c'est du papier.</li>
<li>Payé, Partiel et Impayé gardent leurs couleurs d'état, éclaircies et posées sur un fond teinté.</li>
<li>Toutes les valeurs sombres sont hors charte, à valider ; chaque palette donne ses contrastes mesurés.</li></ul></div>
{"".join(sections)}
</body></html>"""


if __name__ == "__main__":
    rendre()
    html = ICI / "index.html"
    contenu = planche()
    html.write_text(contenu, encoding="utf-8")
    mesure = ICI / "_mesure.html"
    mesure.write_text(contenu.replace("</body>", "<script>document.fonts.ready.then(()=>document.body.setAttribute("
                                      "'data-h',document.documentElement.scrollHeight))</script></body>"), encoding="utf-8")
    h = int(re.search(r'data-h="(\d+)"', chrome(f"--window-size={LARGEUR_PLANCHE},1000", "--dump-dom", mesure.as_uri()).stdout).group(1))
    mesure.unlink()
    png = ICI / "planche-mode-sombre.png"
    chrome("--force-device-scale-factor=1", f"--window-size={LARGEUR_PLANCHE},{h}", f"--screenshot={png}", html.as_uri())
    print(png, h)
    for p in PALETTES:
        print(p["cle"], [(l, round(c, 2)) for l, c, ok in controles(p) if not ok])
