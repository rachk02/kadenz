"""Maquettes des écrans de Kadenz avec les motifs animés retenus le 2026-11-18
(A Trame, A2 Trame de pièces, C Tournant, et les versions du deuxième tour A+ Anneaux,
A2+ Pièces et lignes), en mode clair et en mode sombre (palettes Graphite, Ardoise,
Anthracite), pour que l'utilisateur adopte le design avant l'implémentation.

Écrans : présentation, inscription du poste, connexion, assistant du serveur (KadenzAdmin),
tableau de bord, À propos, liste vide. Les textes sont les chaînes de l'application ;
les montants et le nom « Entreprise d'essai » viennent de la base d'essai. Couleurs : jetons
de crates/app/ui/composants.slint. Icônes du menu : crates/app/ui/icones/menu.

Usage : py -3.13 generer.py   (Chrome requis)
Paramètres de la page : ?fige=1 fige les animations après l'entrée (planche PNG).
"""
import base64
import json
import subprocess
from pathlib import Path

ICI = Path(__file__).resolve().parent
MARQUE = ICI.parent
DEPOT = MARQUE.parents[1]
MENU = DEPOT / "crates" / "app" / "ui" / "icones" / "menu"
CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"


def uri(chemin):
    return "data:image/svg+xml;base64," + base64.b64encode(chemin.read_bytes()).decode("ascii")


LOGOS = {
    "logo": uri(MARQUE / "kadenz-logo-horizontal.svg"),
    "logoN": uri(MARQUE / "kadenz-logo-horizontal-negatif.svg"),
    "sym": uri(MARQUE / "kadenz-symbole.svg"),
    "symN": uri(MARQUE / "kadenz-symbole-negatif.svg"),
}
ICONES = ["tableau-de-bord", "devis", "proforma", "bons-commande", "livraisons", "factures", "avoirs",
          "reglements", "suivi-fournisseurs", "relances", "comptabilite", "tiers", "produits", "parametres",
          "mon-compte", "navigation", "entreprise", "documents", "paiements", "exports", "impression",
          "utilisateurs", "licence", "mises-a-jour", "acces-distant", "a-propos"]

MOTIFS = [("A", "A Trame"), ("A2", "A2 Trame de pièces"), ("C", "C Tournant"),
          ("A+", "A+ Anneaux"), ("A2+", "A2+ Pièces et lignes")]


def ico(nom):
    return f'<i class="ico" style="--i:url(#{nom})" data-ico="{nom}"></i>'


def fenetre(titre, corps, classe=""):
    return f"""<div class="m {classe}"><div class="barre"><img class="mini" data-logo="sym"><span>{titre}</span>
<b class="ctl">&#8211;&#160;&#160;&#9633;&#160;&#160;&#215;</b></div><div class="corps">{corps}</div></div>"""


def panneau(piece, logo=False, cadre="panneau"):
    tete = '<img class="logo-panneau" data-logo="logoN">' if logo else ""
    return f"""<div class="panneau">{tete}<canvas data-piece="{piece}" data-cadre="{cadre}"></canvas>
<div class="pied-panneau"><b>Gestion commerciale</b><small>Version 1.0.0</small></div></div>"""


def ecran_presentation():
    return fenetre("Kadenz", f"""<div class="rang"><div class="col-form">
<img class="logo-col" data-logo="auto">
<div class="bloc" style="top:210px">
  <div class="etapes"><i class="on"></i><i></i><i></i><i></i></div>
  <p class="sur">1 sur 4</p><h1>Bienvenue dans Kadenz</h1>
  <p class="txt">La facturation de votre entreprise, en FCFA et en sept langues.</p>
  <p class="sur" style="margin-top:30px">Mode d'affichage</p>
  <div class="seg"><span>Comme Windows</span><span class="sel-clair">Clair</span><span class="sel-sombre">Sombre</span></div>
  <div class="sombre-seul"><p class="sur" style="margin-top:16px">Palette du mode sombre</p>
  <div class="pastilles"><span data-p="0"><i style="background:#17181b"></i>Graphite</span><span data-p="1"><i style="background:#15191e"></i>Ardoise</span><span data-p="2"><i style="background:#1a1918"></i>Anthracite</span></div></div>
</div>
<div class="boutons"><span class="btn">Passer</span><span class="btn prim">Suivant</span></div>
</div>{panneau("carre")}</div>""")


def ecran_inscription():
    return fenetre("Kadenz", f"""<div class="rang">{panneau("disque", logo=True).replace('class="panneau"', 'class="panneau etroit"')}
<div class="col-form large"><div class="bloc" style="top:200px;left:80px;width:470px">
  <p class="sur">PREMIER LANCEMENT</p><h1>Rejoindre l'entreprise</h1>
  <p class="txt">Demandez un code d'inscription à l'Admin de l'entreprise.</p>
  <label style="margin-top:26px">Code d'inscription</label><div class="champ haut"></div>
  <small class="aide">Code complet, ou code court XXXX-XXXX-XXXX avec l'adresse du serveur.</small>
  <label>Adresse du serveur (code court seulement)</label>
  <div class="ligne"><div class="champ"></div><span class="btn">Rechercher sur le réseau</span></div>
  <span class="btn prim" style="margin-top:14px;align-self:flex-start">Inscrire ce poste</span>
</div></div></div>""")


def ecran_connexion():
    return fenetre("Entreprise d'essai", f"""<div class="rang"><div class="col-form">
<img class="logo-col" data-logo="auto">
<div class="bloc" style="top:250px;width:360px">
  <p class="sur">CONNEXION</p><h1>Entreprise d'essai</h1>
  <label style="margin-top:26px">Identifiant</label><div class="champ"></div>
  <label>Mot de passe</label><div class="champ"></div>
  <span class="btn prim plein" style="margin-top:22px">Se connecter</span>
</div></div>{panneau("quart")}</div>""")


def ecran_assistant():
    etapes = ["Licence", "Entreprise", "Administrateur", "Taux et paiements", "Sauvegardes", "Récapitulatif"]
    lis = "".join(f'<li class="{"fait" if i == 0 else "cours" if i == 1 else ""}">{e}</li>' for i, e in enumerate(etapes))
    paire = lambda a, b: f'<div class="paire"><div><label>{a}</label><div class="champ"></div></div><div><label>{b}</label><div class="champ"></div></div></div>'
    return fenetre("Kadenz - Administration du serveur", f"""<div class="rang">
<div class="colonne-etapes"><canvas data-piece="tri" data-cadre="colonne"></canvas>
  <img class="logo-etapes" data-logo="logoN"><p class="sur clair-sur">PREMIÈRE INSTALLATION</p><ul class="liste-etapes">{lis}</ul></div>
<div class="zone"><div class="defile"><h1>Entreprise</h1>
  <label>Nom de l'entreprise (obligatoire)</label><div class="champ"></div>
  <label>Adresse / boîte postale</label><div class="champ"></div>
  {paire("Téléphone", "Email")}{paire("RCCM", "IFU")}
  <div class="pli"><span>Plus d'informations</span><small>7 champs facultatifs</small></div>
  <small class="aide">Tout se modifie plus tard dans les Paramètres.</small></div>
  <div class="bande-boutons"><span class="btn">Précédent</span><span class="btn prim">Suivant</span></div></div></div>""")


def menu_lateral(actif):
    groupes = [(None, [("tableau-de-bord", "Tableau de bord")]),
               ("Documents", [("devis", "Devis"), ("proforma", "Proforma"), ("bons-commande", "Bons de commande"),
                              ("livraisons", "Livraisons"), ("factures", "Factures"), ("avoirs", "Avoirs")]),
               ("Paiements", [("reglements", "Règlements"), ("suivi-fournisseurs", "Suivi fournisseurs"),
                              ("relances", "Relances"), ("comptabilite", "Comptabilité")]),
               ("Référentiels", [("tiers", "Tiers"), ("produits", "Produits")])]
    h = '<div class="menu"><div class="ent"><img class="mini2" data-logo="symAuto"><b>Entreprise d\'essai</b></div>'
    for g, items in groupes:
        if g:
            h += f'<p class="grp">{g}</p>'
        for nom, lib in items:
            h += f'<div class="item{" actif" if lib == actif else ""}">{ico(nom)}{lib}</div>'
    h += f'<div class="bas"><div class="item{" actif" if actif == "Paramètres" else ""}">{ico("parametres")}Paramètres</div><small>Administrateur (Administrateur)</small></div></div>'
    return h


def ecran_tableau():
    carte = lambda v, l, cl="": f'<div class="carte"><b class="{cl}">{v}</b><small>{l}</small></div>'
    return fenetre("Entreprise d'essai", f"""<div class="rang">{menu_lateral("Tableau de bord")}
<div class="principal">
  <div class="bandeau"><canvas data-piece="disque" data-cadre="bande"></canvas>
    <div class="txt-bandeau"><h1>Tableau de bord</h1><small>Période du 01/01/2026 au 31/12/2026.</small></div></div>
  <h3>Ventes</h3>
  <div class="cartes3">{carte("359 500 FCFA", "Chiffre d'affaires HT")}{carte("0 FCFA", "TVA")}{carte("359 500 FCFA", "Chiffre d'affaires TTC")}</div>
  <div class="cartes3">{carte("0 FCFA", "Total payé", "ok")}{carte("359 500 FCFA", "Total impayé", "ko")}<div></div></div>
  <div class="carte large"><h4>CA par état de paiement</h4><div class="anneau-ca"><div class="donut"><b>0 %</b><small>payé</small></div>
  <ul class="leg"><li><i class="ko"></i>Impayé<span>100 %</span><span>359 500 FCFA</span></li><li><i class="at"></i>Partiel<span>0 %</span><span>0 FCFA</span></li><li><i class="ok"></i>Payé<span>0 %</span><span>0 FCFA</span></li></ul></div></div>
</div></div>""")


def ecran_apropos():
    rub = ["Mon compte", "Navigation et affichage", "Entreprise", "Documents", "Paiements", "Comptabilité",
           "Textes imprimés et exports", "Impression", "Utilisateurs", "Abonnement", "Mises à jour", "Serveur",
           "Accès à distance", "Opérations suspendues", "À propos"]
    icos = ["mon-compte", "navigation", "entreprise", "documents", "paiements", "comptabilite", "exports",
            "impression", "utilisateurs", "licence", "mises-a-jour", "parametres", "acces-distant", "relances", "a-propos"]
    liste = "".join(f'<div class="item{" actif" if r == "À propos" else ""}">{ico(i)}{r}</div>' for r, i in zip(rub, icos))
    return fenetre("Entreprise d'essai", f"""<div class="rang">{menu_lateral("Paramètres")}
<div class="rubriques"><h2>Paramètres</h2>{liste}</div>
<div class="principal"><h1 style="margin-bottom:16px">À propos</h1>
  <div class="carte-apropos"><div class="bandeau court"><canvas data-piece="quart" data-cadre="bande"></canvas>
    <img class="logo-bandeau" data-logo="logoN"></div>
    <div class="contenu"><h4>Gestion commerciale</h4><p class="txt" style="font-size:14px">Devis, proforma, bons de commande, bordereaux de livraison, factures et paiements.</p>
    <div class="lig"><span>Version</span><b>1.0.0</b></div>
    <span class="btn" style="margin-top:14px;align-self:flex-start">Revoir la présentation</span></div></div>
</div></div>""")


def ecran_vide():
    return fenetre("Entreprise d'essai", f"""<div class="rang">{menu_lateral("Factures")}
<div class="principal"><div class="ligne entre"><h1>Factures</h1><div class="ligne"><span class="btn">Exporter en Excel</span><span class="btn prim">Nouvelle facture</span></div></div>
  <div class="ligne filtres"><div class="champ f1">Rechercher : numéro, client, libellé</div><div class="champ f2">Tous les statuts</div><div class="champ f2">Tout paiement</div><div class="champ f3">Du JJ/MM/AAAA</div><div class="champ f3">Au JJ/MM/AAAA</div></div>
  <div class="tableau"><div class="entete-t"><span>N°</span><span>Date</span><span>Client</span><span>Libellé</span><span>Montant TTC</span><span>Statut</span><span>Paiement</span><span>Mode</span><span>Échéance</span></div>
  <div class="vide"><canvas data-piece="carre" data-cadre="vignette"></canvas><p>Aucune facture sur la période.</p></div></div>
</div></div>""")


ECRANS = [
    ("presentation", "Présentation, page 1", "Premier lancement. La pièce de la page se compose à l'arrivée, puis le panneau reste calme ; une pièce différente par page (carré, quart de disque, triangle, disque).", "A2", ecran_presentation),
    ("inscription", "Inscription du poste", "Panneau à gauche avec le logo ; le disque ambre arrive, les anneaux de points tournent lentement.", "A+", ecran_inscription),
    ("connexion", "Connexion", "Le quart de disque se compose en pièces ambre, puis un reflet très lent passe sur le motif, pour ne pas distraire pendant la saisie.", "A2", ecran_connexion),
    ("assistant", "Assistant de création (KadenzAdmin)", "Colonne des étapes : le motif avance d'un temps à chaque étape franchie.", "C", ecran_assistant),
    ("tableau", "Tableau de bord", "Proposition : un bandeau de marque bas, derrière le titre, animé une fois à l'ouverture. Le reste de l'écran ne bouge pas.", "A2", ecran_tableau),
    ("apropos", "Paramètres, À propos", "Proposition : un bandeau de marque en tête de la carte À propos.", "A2+", ecran_apropos),
    ("vide", "Liste vide", "Proposition : une petite pièce animée au-dessus du message, quand une liste est vide.", "A2", ecran_vide),
]

CSS = r"""
@font-face { font-family:Inter; src:url(../planches/polices/Inter.woff2) format("woff2"); font-weight:100 900; }
@font-face { font-family:"Space Grotesk"; src:url(../planches/polices/SpaceGrotesk.ttf) format("truetype"); font-weight:300 700; }
* { box-sizing:border-box; margin:0; }
em, i { font-style:normal; }
:root { --page:#F4F5F9; --encre:#1F2933; --ardoise:#52606D; --trait:#D9DCE3; --nuit:#1A2259; --indigo:#2D3A8C; --ambre:#E3A23B; }
body { background:var(--page); color:var(--encre); font-family:Inter, "Segoe UI", sans-serif; max-width:1680px; margin:0 auto; padding:52px 44px 80px; }
.entete { display:flex; flex-wrap:wrap; gap:24px; justify-content:space-between; align-items:flex-end; border-bottom:1px solid var(--trait); padding-bottom:24px; margin-bottom:22px; }
.entete h1 { font-family:"Space Grotesk", Inter, sans-serif; font-size:44px; color:var(--nuit); }
.entete p { color:var(--ardoise); font-size:17px; line-height:1.5; max-width:820px; margin-top:8px; }
.reglages { display:flex; flex-wrap:wrap; gap:18px; align-items:center; margin-bottom:40px; }
.reglages b { font-size:13px; letter-spacing:1px; text-transform:uppercase; color:var(--ardoise); margin-right:4px; }
.choix { display:inline-flex; gap:6px; flex-wrap:wrap; }
.choix button { font:500 14px Inter, sans-serif; background:white; border:1px solid var(--trait); border-radius:4px; padding:8px 12px; cursor:pointer; color:var(--encre); }
.choix button.on { background:var(--indigo); border-color:var(--indigo); color:white; }
section.ecran { margin-bottom:64px; }
section.ecran header { display:flex; flex-wrap:wrap; gap:16px 28px; align-items:flex-end; justify-content:space-between; margin-bottom:16px; }
section.ecran h2 { font-family:"Space Grotesk", Inter, sans-serif; font-size:28px; color:var(--nuit); }
section.ecran header p { color:var(--ardoise); font-size:15.5px; line-height:1.5; max-width:820px; margin-top:6px; }
.duo { display:grid; grid-template-columns:1fr 1fr; gap:22px; }
.duo figcaption { font-size:13px; letter-spacing:1px; text-transform:uppercase; color:var(--ardoise); font-weight:600; margin-bottom:8px; }
.cadre { position:relative; width:100%; aspect-ratio:1200/760; overflow:hidden; border:1px solid var(--trait); }
.cadre > .m { position:absolute; left:0; top:0; width:1200px; height:760px; transform-origin:0 0; }
footer { border-top:1px solid var(--trait); padding-top:22px; color:var(--ardoise); font-size:14.5px; line-height:1.6; }
@media (max-width: 1000px) { body { padding:28px 16px 56px; } .duo { grid-template-columns:1fr; } }

/* Jetons de l'application (composants.slint) */
.m.clair { --fond:#ffffff; --lateral:#f4f5f9; --surface:#ffffff; --panneau:#1a2259; --accent:#2d3a8c; --sur-accent:#ffffff;
  --survol:#e8eaf6; --champ:#ffffff; --bordure:#d9dce3; --bordure-champ:#7b8794; --texte:#1f2933; --second:#52606d;
  --ok:#067647; --ko:#b42318; --at:#b54708; --barre:#f3f3f3; --barre-texte:#1f2933;
  --points:#2d3a8c; --motif:#232c6b; --motif-clair:#2a3478; --lignes:#f4f5f9; --sur-panneau:#ffffff; --second-panneau:#c5cae0; }
.m.sombre { --ok:#47cd89; --ko:#f97066; --at:#fdb022; --sur-panneau:#eeebe7; --points:#4a4d52; }
body[data-palette="0"] .m.sombre { --fond:#17181b; --lateral:#1e2024; --surface:#1e2024; --panneau:#111214; --accent:#e3a23b; --sur-accent:#17181b;
  --survol:#2b2e34; --champ:#26282d; --bordure:#33363c; --bordure-champ:#6e737d; --texte:#ecedef; --second:#a3a8b0;
  --barre:#1e2024; --barre-texte:#ecedef; --motif:#212327; --motif-clair:#2a2d32; --lignes:#ecedef; --second-panneau:#a3a8b0; }
body[data-palette="1"] .m.sombre { --fond:#15191e; --lateral:#1c2229; --surface:#1f2933; --panneau:#101317; --accent:#8f9bea; --sur-accent:#15191e;
  --survol:#2a3542; --champ:#27313c; --bordure:#334050; --bordure-champ:#6f7d8c; --texte:#f4f5f9; --second:#a9b4c0;
  --barre:#1c2229; --barre-texte:#f4f5f9; --motif:#1c232b; --motif-clair:#253039; --lignes:#f4f5f9; --second-panneau:#a9b4c0; }
body[data-palette="2"] .m.sombre { --fond:#1a1918; --lateral:#211f1e; --surface:#232120; --panneau:#131211; --accent:#e3a23b; --sur-accent:#1a1918;
  --survol:#302d2a; --champ:#2b2927; --bordure:#383533; --bordure-champ:#77716b; --texte:#eeebe7; --second:#ada69f;
  --barre:#211f1e; --barre-texte:#eeebe7; --motif:#22201e; --motif-clair:#2c2a27; --lignes:#eeebe7; --second-panneau:#ada69f; }

.m { background:var(--fond); color:var(--texte); font-family:Inter, sans-serif; font-size:14px; display:flex; flex-direction:column; }
.m .barre { height:32px; flex:none; background:var(--barre); color:var(--barre-texte); display:flex; align-items:center; gap:8px; padding:0 12px; font-size:12.5px; }
.m .barre .ctl { margin-left:auto; font-weight:400; letter-spacing:2px; opacity:.8; }
.m .mini { width:16px; height:16px; }
.m .corps { flex:1; position:relative; overflow:hidden; }
.m .rang { display:flex; height:100%; }
.m .col-form { position:relative; width:47%; flex:none; height:100%; }
.m .col-form.large { width:auto; flex:1; }
.m .logo-col { position:absolute; left:72px; top:56px; height:30px; }
.m .bloc { position:absolute; left:72px; width:416px; display:flex; flex-direction:column; }
.m .etapes { display:flex; gap:8px; margin-bottom:22px; }
.m .etapes i { width:44px; height:4px; background:var(--bordure); }
.m .etapes i.on { background:var(--accent); }
.m .sur { font-size:12px; font-weight:600; letter-spacing:1.6px; color:var(--second); margin-bottom:12px; }
.m h1 { font-family:"Space Grotesk", Inter, sans-serif; font-size:32px; font-weight:700; color:var(--texte); line-height:1.15; }
.m .txt { font-size:16px; color:var(--second); margin-top:14px; line-height:1.45; }
.m label { display:block; font-size:12.5px; color:var(--second); margin:14px 0 6px; }
.m .champ { height:34px; border:1px solid var(--bordure-champ); background:var(--champ); border-radius:4px; color:var(--second); font-size:12.5px; padding:8px 10px; white-space:nowrap; overflow:hidden; }
.m .champ.haut { height:68px; }
.m .aide { display:block; font-size:12px; color:var(--second); margin-top:6px; }
.m .ligne { display:flex; gap:10px; align-items:center; }
.m .ligne .champ { flex:1; }
.m .ligne.entre { justify-content:space-between; margin-bottom:16px; }
.m .btn { display:inline-flex; align-items:center; justify-content:center; height:38px; padding:0 16px; border:1px solid var(--bordure); border-radius:4px; background:var(--surface); color:var(--texte); font-weight:500; font-size:13.5px; white-space:nowrap; }
.m .btn.prim { background:var(--accent); border-color:var(--accent); color:var(--sur-accent); font-weight:600; }
.m .btn.plein { width:100%; }
.m .seg { display:flex; gap:8px; }
.m .seg span { height:34px; padding:0 14px; display:inline-flex; align-items:center; border:1px solid var(--bordure-champ); border-radius:4px; background:var(--champ); font-size:13.5px; }
.m .seg span.on, .m.clair .seg .sel-clair, .m.sombre .seg .sel-sombre { background:var(--accent); border-color:var(--accent); color:var(--sur-accent); font-weight:600; }
.m.clair .sombre-seul, .m.sombre .clair-seul { display:none !important; }
.m .pastilles { display:flex; gap:12px; }
.m .pastilles span { display:flex; flex-direction:column; align-items:center; gap:5px; font-size:12px; color:var(--second); }
.m .pastilles i { width:64px; height:40px; border:1px solid var(--bordure-champ); border-radius:4px; }
.m .boutons { position:absolute; left:72px; width:416px; bottom:56px; display:flex; gap:10px; justify-content:flex-end; }
.m .panneau { position:relative; flex:1; background:var(--panneau); overflow:hidden; }
.m .panneau.etroit { flex:none; width:42%; }
.m .panneau canvas { position:absolute; inset:0; width:100%; height:100%; }
.m .logo-panneau { position:absolute; left:48px; top:56px; height:30px; z-index:1; }
.m .pied-panneau { position:absolute; left:48px; bottom:44px; display:flex; flex-direction:column; gap:6px; }
.m .pied-panneau b { font-family:"Space Grotesk", Inter, sans-serif; font-size:22px; color:var(--sur-panneau); }
.m .pied-panneau small { font-size:12px; color:var(--second-panneau); }
.m .colonne-etapes { position:relative; width:232px; flex:none; background:var(--panneau); overflow:hidden; padding:56px 0 0 28px; }
.m .colonne-etapes canvas { position:absolute; inset:0; width:100%; height:100%; }
.m .logo-etapes { position:relative; height:26px; margin-bottom:62px; }
.m .clair-sur { position:relative; color:var(--second-panneau); }
.m .liste-etapes { position:relative; list-style:none; padding:0; }
.m .liste-etapes li { height:34px; display:flex; align-items:center; gap:12px; color:var(--second-panneau); font-size:14px; padding-left:22px; position:relative; }
.m .liste-etapes li.fait::before { content:""; position:absolute; left:2px; width:8px; height:4px; border-left:2px solid var(--ambre); border-bottom:2px solid var(--ambre); transform:rotate(-45deg); top:13px; }
.m .liste-etapes li.cours { color:var(--sur-panneau); font-weight:600; }
.m .liste-etapes li.cours::before { content:""; position:absolute; left:2px; width:8px; height:8px; background:var(--ambre); }
.m .zone { flex:1; position:relative; display:flex; flex-direction:column; }
.m .defile { flex:1; padding:72px 68px 0; display:flex; flex-direction:column; max-width:756px; }
.m .defile h1 { margin-bottom:6px; }
.m .paire { display:grid; grid-template-columns:1fr 1fr; gap:16px; }
.m .pli { margin-top:18px; height:42px; display:flex; align-items:center; justify-content:space-between; padding:0 14px; background:var(--lateral); border:1px solid var(--bordure); border-radius:4px; }
.m .pli small { color:var(--second); }
.m .bande-boutons { height:100px; flex:none; background:var(--lateral); display:flex; justify-content:space-between; align-items:center; padding:0 68px; max-width:none; }
.m .menu { width:236px; flex:none; background:var(--lateral); padding:12px 10px; display:flex; flex-direction:column; font-size:13.5px; }
.m .menu .ent { display:flex; align-items:center; gap:8px; padding:6px 8px 12px; }
.m .menu .ent b { font-family:"Space Grotesk", Inter, sans-serif; font-size:15px; }
.m .mini2 { width:22px; height:22px; }
.m .menu .grp { font-size:11.5px; font-weight:600; color:var(--second); margin:10px 8px 4px; }
.m .item { display:flex; align-items:center; gap:10px; height:30px; padding:0 8px; border-radius:4px; }
.m .item.actif { background:var(--accent); color:var(--sur-accent); font-weight:600; }
.m .ico { width:17px; height:17px; flex:none; background:currentColor; -webkit-mask:var(--i) center/contain no-repeat; mask:var(--i) center/contain no-repeat; }
.m .menu .bas { margin-top:auto; }
.m .menu .bas small { display:block; color:var(--second); font-size:11.5px; padding:8px; }
.m .principal { flex:1; padding:22px 26px; overflow:hidden; display:flex; flex-direction:column; }
.m .principal h3 { font-size:14px; margin:16px 0 8px; }
.m .bandeau { position:relative; height:118px; flex:none; background:var(--panneau); overflow:hidden; border-radius:4px; }
.m .bandeau.court { height:110px; border-radius:4px 4px 0 0; }
.m .bandeau canvas { position:absolute; inset:0; width:100%; height:100%; }
.m .txt-bandeau { position:absolute; left:24px; top:28px; }
.m .txt-bandeau h1 { color:var(--sur-panneau); font-size:28px; }
.m .txt-bandeau small { color:var(--second-panneau); font-size:12.5px; display:block; margin-top:6px; }
.m .logo-bandeau { position:absolute; left:24px; top:40px; height:28px; }
.m .cartes3 { display:grid; grid-template-columns:repeat(3, 1fr); gap:10px; margin-bottom:10px; }
.m .carte { background:var(--surface); border:1px solid var(--bordure); border-radius:4px; padding:12px 14px; display:flex; flex-direction:column; gap:4px; }
.m .carte b { font-size:18px; font-family:"Space Grotesk", Inter, sans-serif; }
.m .carte small { color:var(--second); font-size:12px; }
.m .carte .ok { color:var(--ok); } .m .carte .ko { color:var(--ko); }
.m .carte.large h4 { font-size:13.5px; }
.m .anneau-ca { display:flex; align-items:center; gap:22px; margin-top:8px; }
.m .donut { width:92px; height:92px; border-radius:50%; border:12px solid var(--ko); display:flex; flex-direction:column; align-items:center; justify-content:center; }
.m .donut b { font-size:15px; } .m .donut small { font-size:11px; color:var(--second); }
.m .leg { list-style:none; padding:0; font-size:12.5px; display:grid; gap:6px; }
.m .leg li { display:grid; grid-template-columns:12px 70px 50px 110px; align-items:center; gap:6px; }
.m .leg i { width:10px; height:10px; } .m .leg i.ko { background:var(--ko); } .m .leg i.at { background:var(--at); } .m .leg i.ok { background:var(--ok); }
.m .rubriques { width:250px; flex:none; border-right:1px solid var(--bordure); padding:20px 12px; font-size:13.5px; display:flex; flex-direction:column; gap:2px; }
.cadre .m h2, .cadre .m h3, .cadre .m h4 { color:var(--texte); }
.m .rubriques h2 { font-family:"Space Grotesk", Inter, sans-serif; font-size:22px; margin:0 8px 12px; }
.m .carte-apropos { border:1px solid var(--bordure); border-radius:4px; background:var(--surface); display:flex; flex-direction:column; max-width:600px; }
.m .carte-apropos .contenu { padding:18px 20px; display:flex; flex-direction:column; }
.m .carte-apropos h4 { font-family:"Space Grotesk", Inter, sans-serif; font-size:18px; }
.m .lig { display:flex; justify-content:space-between; border-top:1px solid var(--bordure); margin-top:14px; padding-top:10px; font-size:13.5px; }
.m .lig span { color:var(--second); }
.m .filtres { margin-bottom:12px; } .m .filtres .f1 { flex:2.2; } .m .filtres .f2 { flex:1.1; } .m .filtres .f3 { flex:1; }
.m .tableau { flex:1; border:1px solid var(--bordure); border-radius:4px; display:flex; flex-direction:column; }
.m .entete-t { display:grid; grid-template-columns:1.1fr .9fr 1.2fr 1.4fr 1fr .8fr .9fr .7fr .9fr; padding:10px 12px; font-size:12px; font-weight:600; color:var(--second); border-bottom:1px solid var(--bordure); }
.m .vide { flex:1; display:flex; flex-direction:column; align-items:center; justify-content:center; gap:14px; color:var(--second); }
.m .vide canvas { width:96px; height:96px; }
"""

SCRIPT = r"""
const LOGOS = %%LOGOS%%;
const ICONES = %%ICONES%%;
const DEFAUTS = %%DEFAUTS%%;
const PI = Math.PI;
const PIECES = {
  carre: new Path2D("M16 16 L120 16 L120 120 L16 120 Z"),
  quart: new Path2D("M136 120 L136 14.44 A105.56 105.56 0 0 1 241.56 120 Z"),
  tri: new Path2D("M16 136 L120 240 L16 240 Z"),
  disque: new Path2D("M134.44 188 A53.56 53.56 0 1 1 241.56 188 A53.56 53.56 0 1 1 134.44 188 Z"),
};
const ORDRE = ["carre", "quart", "tri", "disque"];
const CENTRE = { carre: [68, 68], quart: [188, 68], tri: [68, 188], disque: [188, 188] };
const AMBRE = "#E3A23B";
const clamp = (x, a = 0, b = 1) => Math.max(a, Math.min(b, x));
const lin = (t, a, d) => clamp((t - a) / d);
const eOut = x => 1 - Math.pow(1 - x, 3);
const eInOut = x => x < 0.5 ? 4 * x * x * x : 1 - Math.pow(-2 * x + 2, 3) / 2;
const eBack = x => { const c1 = 1.70158, c3 = c1 + 1; return 1 + c3 * Math.pow(x - 1, 3) + c1 * Math.pow(x - 1, 2); };
const hash = (i, j) => (((i * 73856093) ^ (j * 19349663)) >>> 0) % 4;
function dans(k, u, v) {
  switch (k) {
    case "carre": return u >= 16 && u <= 120 && v >= 16 && v <= 120;
    case "quart": return u >= 136 && v <= 120 && (u - 136) ** 2 + (v - 120) ** 2 <= 105.56 ** 2;
    case "tri": return u >= 16 && v <= 240 && v >= 136 && (u - 16) <= (v - 136);
    case "disque": return (u - 188) ** 2 + (v - 188) ** 2 <= 53.56 ** 2;
  }
  return false;
}
function alea(graine) {
  let a = graine >>> 0;
  return () => { a = (a + 0x6D2B79F5) >>> 0; let t = a; t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
}
function rgba(hex, a) { const n = parseInt(hex.slice(1), 16); return `rgba(${n >> 16},${(n >> 8) & 255},${n & 255},${a})`; }
function unePiece(c, k, cx, cy, s, couleur, e = 1) {
  if (e <= 0.001) return; const [px, py] = CENTRE[k];
  c.save(); c.translate(cx, cy); c.scale(s / 104 * e, s / 104 * e); c.translate(-px, -py); c.fillStyle = couleur; c.fill(PIECES[k]); c.restore();
}

// Placement : la pièce vedette, ses dimensions, et la zone nette (sans motif) de chaque cadre.
function placement(cadre, w, h, piece) {
  let cell, cx, cy, nette;
  if (cadre === "panneau") { cell = 0.6 * Math.min(w, h * 0.8); cx = w / 2; cy = h * 0.42; nette = (x, y) => y > h - 150 || y < 100; }
  else if (cadre === "colonne") { cell = 0.9 * w; cx = w * 0.42; cy = h - 0.55 * w; nette = (x, y) => y < h - 1.25 * w; }
  else if (cadre === "bande") { cell = 0.82 * h; cx = w - 0.75 * h; cy = h / 2; nette = (x, y) => x < w * 0.42; }
  else { cell = 0.72 * w; cx = w / 2; cy = h / 2; nette = () => false; }
  const S = cell * 256 / 104, [px, py] = CENTRE[piece];
  return { cell, S, ox: cx - px / 256 * S, oy: cy - py / 256 * S, nette };
}

const MOTIF = {
  A(c, w, h, T, k, p) {
    const pas = Math.max(10, w / (p.cadre === "vignette" ? 9 : 36)), g = eOut(lin(T, 0.2, 1.4));
    for (let y = pas / 2; y < h; y += pas) for (let x = pas / 2; x < w; x += pas) {
      const u = (x - p.ox) / p.S * 256, v = (y - p.oy) / p.S * 256;
      const onde = 0.5 + 0.5 * Math.sin((x * 0.9 + y * 0.6) * 0.011 - T * 2.4);
      if (dans(p.piece, u, v) && g > 0.01) {
        const d = (u + v) / 512, lum = Math.min(1, Math.max(0.42, 0.3 + d * 1.2));
        c.fillStyle = AMBRE; c.beginPath(); c.arc(x, y, pas * 0.5 * lum * g * (0.86 + 0.14 * onde), 0, 2 * PI); c.fill(); continue;
      }
      if (p.nette(x, y)) continue;
      c.fillStyle = k.points; c.beginPath(); c.arc(x, y, pas * (0.09 + 0.07 * onde), 0, 2 * PI); c.fill();
    }
  },
  A2(c, w, h, T, k, p) {
    const cell = p.cadre === "vignette" ? w / 6 : p.cadre === "bande" ? h / 4 : w / 14, m = cell / 2;
    for (let j = 0; j * cell < h; j++) for (let i = 0; i * cell < w; i++) {
      const cx = i * cell + m, cy = j * cell + m, kk = ORDRE[(i + j) % 4];
      const u = (cx - p.ox) / p.S * 256, v = (cy - p.oy) / p.S * 256;
      if (dans(p.piece, u, v)) {
        const [pcx, pcy] = CENTRE[p.piece], r = clamp(((u - pcx) + (v - pcy)) / 208 + 0.5);
        const e = eBack(clamp((lin(T, 0.2, 1.4) * 1.25 - r) / 0.25));
        if (e > 0.01) { unePiece(c, kk, cx, cy, cell * 0.86, AMBRE, e); continue; }
      }
      if (p.nette(cx, cy)) continue;
      const l = 0.5 + 0.5 * Math.sin((i + j) * 0.45 - T * PI / 2);
      unePiece(c, kk, cx, cy, cell * 0.86, l > 0.8 ? k.motifClair : k.motif);
    }
  },
  C(c, w, h, T, k, p) {
    const cell = p.cadre === "vignette" ? w / 3 : p.cadre === "bande" ? h / 2 : p.cadre === "colonne" ? w / 3 : w / 7;
    const m = cell / 2, R = cell * 0.96;
    for (let j = 0; j * cell < h; j++) for (let i = 0; i * cell < w; i++) {
      const cx = i * cell + m, cy = j * cell + m; if (p.nette(cx, cy)) continue;
      const phase = T / 1.5 - (i + j) * 0.06, n = Math.floor(phase), fr = phase - n;
      const rot = (hash(i, j) + n + eInOut(clamp(fr / 0.35))) * PI / 2;
      const amb = p.cadre !== "bande" && i === 1 && j === (p.cadre === "colonne" ? Math.floor(h / cell) - 1 : Math.floor((h / cell) * 0.6));
      c.save(); c.translate(cx, cy); c.rotate(rot); c.fillStyle = amb ? AMBRE : k.motif;
      c.beginPath(); c.moveTo(-m, m); c.lineTo(-m, m - R); c.arc(-m, m, R, -PI / 2, 0); c.closePath(); c.fill(); c.restore();
    }
  },
  "A+"(c, w, h, T, k, p) {
    const cle = p.cle + k.points + k.lignes;
    if (!CACHE.has(cle)) {
      const rnd = alea(17), m = 24, R = Math.ceil(1.15 * Math.max(w, h * 1.4)), calques = new Map();
      const bandes = [[0.42, 0.52, 2], [0.55, 0.57, 2], [0.61, 0.82, 1], [0.86, 0.9, 1], [0.95, 1.1, 1]];
      const unite = Math.max(w, h * 1.4) * 0.92, aire = w * h;
      for (const [a, b, tours] of bandes) {
        if (!calques.has(tours)) { const t = document.createElement("canvas"); t.width = t.height = 2 * R * NET; t.getContext("2d").scale(NET, NET); calques.set(tours, t); }
        const g = calques.get(tours).getContext("2d"), n = Math.round(aire / 360 * (b - a) * 10);
        for (let i = 0; i < n; i++) {
          const r = (a + (b - a) * rnd()) * unite, th = rnd() * 2 * PI / m, s = (0.35 + 1.6 * Math.pow(rnd(), 2.8)) * (w / 560);
          const am = rnd() < 0.025; g.fillStyle = am ? AMBRE : rgba(k.lignes, 0.15 + 0.6 * rnd());
          for (let j = 0; j < m; j++) { const an = th + j * 2 * PI / m; g.beginPath(); g.arc(R + r * Math.cos(an), R + r * Math.sin(an), s, 0, 2 * PI); g.fill(); }
        }
      }
      CACHE.set(cle, { calques, R });
    }
    const { calques, R } = CACHE.get(cle), cx = -0.32 * w, cy = 0.36 * h, base = T / 12 * 2 * PI / 24;
    c.save(); c.beginPath();
    if (p.cadre === "panneau") c.rect(0, 100, w, h - 250); else if (p.cadre === "bande") c.rect(w * 0.42, 0, w, h);
    else if (p.cadre === "colonne") c.rect(0, h - 1.25 * w, w, h); else c.rect(0, 0, w, h);
    c.clip();
    for (const [tours, t] of calques) { c.save(); c.translate(cx, cy); c.rotate(base * tours); c.drawImage(t, -R, -R, 2 * R, 2 * R); c.restore(); }
    c.restore();
    const e = eBack(lin(T, 0.3, 0.8));
    unePiece(c, p.piece, p.ox + CENTRE[p.piece][0] / 256 * p.S, p.oy + CENTRE[p.piece][1] / 256 * p.S, p.cell * (p.cadre === "panneau" ? 0.62 : 0.8), AMBRE, e);
  },
  "A2+"(c, w, h, T, k, p) {
    const styles = { carre: 1, quart: 0, tri: 2, disque: 3 }, entier = p.cadre === "panneau" || p.cadre === "bande";
    const ordre = entier ? ORDRE : [p.piece];
    let S = p.S, ox = p.ox, oy = p.oy;
    if (p.cadre === "panneau") { S = Math.min(w * 0.7, h * 0.5); ox = (w - S) / 2; oy = h * 0.42 - S / 2; }
    if (p.cadre === "bande") { S = h * 0.8; ox = w - S - h * 0.35; oy = (h - S) / 2; }
    ordre.forEach((kk, i) => {
      const pr = lin(T, 0.3 + i * 0.5, 0.9); if (pr <= 0) return;
      const st = styles[kk]; c.save(); c.translate(ox, oy); c.scale(S / 256, S / 256);
      if (st === 0) { const e = eBack(pr), [px, py] = CENTRE[kk]; c.translate(px, py); c.scale(e, e); c.translate(-px, -py); c.fillStyle = k.lignes; c.fill(PIECES[kk]); }
      else if (st === 3) { const e = eBack(pr), [px, py] = CENTRE[kk]; c.translate(px, py); c.scale(e, e); c.translate(-px, -py);
        c.fillStyle = AMBRE; c.fill(PIECES[kk]); c.fillStyle = k.fond; c.beginPath(); c.arc(px, py, 30, 0, 2 * PI); c.fill();
        c.fillStyle = k.lignes; c.beginPath(); c.arc(px, py, 12, 0, 2 * PI); c.fill(); }
      else lignes(c, kk, eInOut(pr), st === 1 ? 6 : 7, st === 1 ? k.lignes : AMBRE, st === 1 ? 5 : 4);
      c.restore();
    });
  },
};
function lignes(c, k, prog, nb, couleur, ep) {
  c.save(); c.clip(PIECES[k]); c.strokeStyle = couleur; c.lineWidth = ep;
  for (let i = 0; i < nb; i++) {
    const f = clamp(prog * nb - i); if (f <= 0) break;
    c.beginPath(); let L;
    if (k === "carre") { const e = (i + 0.5) * 52 / nb, a = 16 + e, b = 120 - e; L = 4 * (b - a); c.moveTo(a, a); c.lineTo(b, a); c.lineTo(b, b); c.lineTo(a, b); c.closePath(); }
    else if (k === "quart") { const r = 105.56 * (i + 0.5) / nb; L = r * PI / 2; c.arc(136, 120, r, -PI / 2, 0); }
    else if (k === "tri") { const cc = -224 + (i + 0.5) * 104 / nb; L = Math.hypot(224 + cc, 224 + cc); c.moveTo(16, 16 - cc); c.lineTo(240 + cc, 240); }
    else { const r = 53.56 * (i + 0.5) / nb; L = 2 * PI * r; c.arc(188, 188, r, -PI / 2, 1.5 * PI); }
    c.setLineDash([L * f, L + 1]); c.stroke();
  }
  c.restore();
}

const CACHE = new Map();
const params = new URLSearchParams(location.search);
const fige = params.get("fige") === "1";
const choixMotif = Object.assign({}, DEFAUTS);
let origine = performance.now();

// Logos et icônes
function logoPour(el) {
  const m = el.closest(".m"), sombre = m && m.classList.contains("sombre"), n = el.dataset.logo;
  if (n === "auto") return sombre ? LOGOS.logoN : LOGOS.logo;
  if (n === "symAuto") return sombre ? LOGOS.symN : LOGOS.sym;
  return LOGOS[n];
}
document.querySelectorAll("img[data-logo]").forEach(el => el.src = logoPour(el));
document.querySelectorAll("i[data-ico]").forEach(el => el.style.setProperty("--i", `url("${ICONES[el.dataset.ico]}")`));

// Mise à l'échelle des fenêtres
const ro = new ResizeObserver(es => es.forEach(e => { const m = e.target.firstElementChild; m.style.transform = `scale(${e.contentRect.width / 1200})`; }));
document.querySelectorAll(".cadre").forEach(c => ro.observe(c));

// Toiles
const toiles = [...document.querySelectorAll(".m canvas")];
const visibles = new Set();
const io = new IntersectionObserver(es => es.forEach(e => e.isIntersecting ? visibles.add(e.target) : visibles.delete(e.target)), { rootMargin: "200px" });
// clientWidth ignore la transformation : ce sont déjà les dimensions de la fenêtre à 1200 x 760.
const NET = 2;
toiles.forEach(t => {
  t.dataset.w = t.clientWidth || 300; t.dataset.h = t.clientHeight || 300;
  t.width = t.dataset.w * NET; t.height = t.dataset.h * NET;
  t.dataset.cle = Math.random().toString(36).slice(2);
  io.observe(t);
});
function couleurs(t) {
  const s = getComputedStyle(t.closest(".m"));
  const v = n => s.getPropertyValue(n).trim();
  return { fond: v("--panneau"), points: v("--points"), motif: v("--motif"), motifClair: v("--motif-clair"), lignes: v("--lignes") };
}
let cacheCouleurs = new Map();
function dessiner(t) {
  for (const toile of toiles) {
    if (!fige && !visibles.has(toile)) continue;
    const ecran = toile.closest("section").dataset.ecran, motif = choixMotif[ecran];
    if (!cacheCouleurs.has(toile)) cacheCouleurs.set(toile, couleurs(toile));
    const k = cacheCouleurs.get(toile), c = toile.getContext("2d"), w = +toile.dataset.w, h = +toile.dataset.h;
    const piece = toile.dataset.piece, cadre = toile.dataset.cadre, p = placement(cadre, w, h, piece);
    p.piece = piece; p.cadre = cadre; p.cle = toile.dataset.cle;
    const T = fige ? 3.6 : t % 12;
    c.setTransform(NET, 0, 0, NET, 0, 0); c.clearRect(0, 0, w, h);
    if (cadre === "vignette") { c.fillStyle = k.fond; c.fillRect(0, 0, w, h); }
    MOTIF[motif](c, w, h, T, k, p);
  }
}
function boucle(now) { dessiner((now - origine) / 1000); if (!fige) requestAnimationFrame(boucle); else document.body.dataset.pret = "1"; }

// Réglages de la page
document.querySelectorAll("[data-palette-choix]").forEach(b => b.onclick = () => {
  document.body.dataset.palette = b.dataset.paletteChoix; cacheCouleurs = new Map(); CACHE.clear();
  document.querySelectorAll("[data-palette-choix]").forEach(x => x.classList.toggle("on", x === b));
  document.querySelectorAll(".pastilles span").forEach(s => s.querySelector("i").style.outline = s.dataset.p === b.dataset.paletteChoix ? "2px solid var(--accent)" : "none");
});
document.querySelectorAll("[data-motif-choix]").forEach(b => b.onclick = () => {
  const sec = b.closest("section"); choixMotif[sec.dataset.ecran] = b.dataset.motifChoix;
  sec.querySelectorAll("[data-motif-choix]").forEach(x => x.classList.toggle("on", x === b)); origine = performance.now();
});
document.querySelectorAll("[data-tous]").forEach(b => b.onclick = () => {
  document.querySelectorAll("section.ecran").forEach(sec => { choixMotif[sec.dataset.ecran] = b.dataset.tous;
    sec.querySelectorAll("[data-motif-choix]").forEach(x => x.classList.toggle("on", x.dataset.motifChoix === b.dataset.tous)); });
  origine = performance.now();
});
document.querySelectorAll(".pastilles span").forEach(s => s.querySelector("i").style.outline = s.dataset.p === "2" ? "2px solid var(--accent)" : "none");

Promise.all([document.fonts.load('700 30px "Space Grotesk"'), document.fonts.load("400 14px Inter")])
  .then(() => { origine = performance.now(); requestAnimationFrame(boucle); });
"""


def page():
    icones = {n: uri(MENU / f"{n}.svg") for n in ICONES}
    defauts = {cle: motif for cle, _, _, motif, _ in ECRANS}
    sections = []
    for cle, titre, note, motif, fonction in ECRANS:
        boutons = "".join(f'<button data-motif-choix="{m}" class="{"on" if m == motif else ""}">{nom}</button>'
                          for m, nom in MOTIFS)
        html = fonction()
        sections.append(f"""<section class="ecran" data-ecran="{cle}"><header><div><h2>{titre}</h2><p>{note}</p></div>
<div class="choix">{boutons}</div></header>
<div class="duo"><figure><figcaption>Mode clair</figcaption><div class="cadre">{html.replace('class="m ', 'class="m clair ')}</div></figure>
<figure><figcaption>Mode sombre</figcaption><div class="cadre">{html.replace('class="m ', 'class="m sombre ')}</div></figure></div></section>""")
    tous = "".join(f'<button data-tous="{m}">{nom}</button>' for m, nom in MOTIFS)
    script = (SCRIPT.replace("%%LOGOS%%", json.dumps(LOGOS)).replace("%%ICONES%%", json.dumps(icones))
              .replace("%%DEFAUTS%%", json.dumps(defauts)))
    return f"""<!doctype html>
<html lang="fr"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Kadenz, écrans animés</title><style>{CSS}</style></head>
<body data-palette="2">
<div class="entete"><div><h1>Écrans du logiciel, motifs animés</h1>
<p>Sept écrans de Kadenz en mode clair et en mode sombre, avec les motifs retenus. Le bouton actif de chaque
écran est le motif adopté par l'utilisateur le 2026-11-18 ; les autres boutons montrent le même écran avec un autre motif. La palette du mode
sombre se change en haut. Dans le logiciel, l'entrée animée joue une fois ; ici elle se répète toutes les 12 s
pour qu'on la voie.</p></div></div>
<div class="reglages"><b>Palette sombre</b><span class="choix"><button data-palette-choix="0">Graphite</button><button data-palette-choix="1">Ardoise</button><button data-palette-choix="2" class="on">Anthracite</button></span>
<b style="margin-left:18px">Même motif partout</b><span class="choix">{tous}</span></div>
{"".join(sections)}
<footer>Textes : chaînes de l'application. Montants et « Entreprise d'essai » : base d'essai. Couleurs : jetons de
composants.slint (clair, Graphite, Ardoise, Anthracite). En mode sombre, les motifs passent en gris (aucun bleu),
seul l'Ambre reste. Le bandeau du tableau de bord, celui de la carte À propos et la pièce de la liste vide sont
des propositions nouvelles ; les autres écrans gardent la mise en page adoptée le 2026-11-18.</footer>
<script>{script}</script></body></html>
"""


if __name__ == "__main__":
    html = ICI / "index.html"
    html.write_text(page(), encoding="utf-8")
    png = ICI / "planche-ecrans-animes.png"
    subprocess.run([CHROME, "--headless=new", "--hide-scrollbars", "--disable-gpu", "--window-size=1680,5180",
                    f"--screenshot={png}", "--virtual-time-budget=6000", "--allow-file-access-from-files",
                    html.as_uri() + "?fige=1"], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    print(html)
    print(png)
