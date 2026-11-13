"""Dossier de présentation de Kadenz : le produit, où il en est, où il va, et son identité.

Produit kadenz-presentation.html (A4, à ouvrir dans un navigateur) et kadenz-presentation.pdf
(rendu par Chrome sans interface). Tout le contenu vient du dépôt : README, CHANGELOG,
docs/cahier-des-charges.md, docs/cahier-phase2.md, docs/jalons-phase2.md, docs/marque/README.md.
Usage : python docs/presentation/generer_presentation.py
"""

import base64
import subprocess
from pathlib import Path

ICI = Path(__file__).resolve().parent
MARQUE = ICI.parent / "marque"
CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"


def svg(nom):
    return (MARQUE / nom).read_text(encoding="utf-8").strip()


def data_uri(nom):
    return "data:image/svg+xml;base64," + base64.b64encode((MARQUE / nom).read_bytes()).decode()


LOGO_NEG = svg("kadenz-logo-horizontal-negatif.svg")
LOGO = svg("kadenz-logo-horizontal.svg")
SYMBOLE = svg("kadenz-symbole.svg")
SYMBOLE_BLANC = svg("kadenz-symbole-blanc.svg")
LOGO_EMPILE = svg("kadenz-logo-empile.svg")
MOTIF = data_uri("motif-quatre-pieces.svg")
MODULES = [
    ("kadenz-module-ventes.svg", "Ventes", "devis, proforma, factures"),
    ("kadenz-module-commandes.svg", "Commandes", "bons de commande"),
    ("kadenz-module-livraisons.svg", "Livraisons", "bons de livraison"),
    ("kadenz-module-paiements.svg", "Paiements", "règlements, relances"),
]

TROIS_TEMPS = (
    '<div class="temps"><span></span><span></span><span class="ambre"></span></div>'
)

CSS = """
@font-face{font-family:Inter;src:url(../marque/planches/polices/Inter.woff2) format("woff2");font-weight:100 900}
@font-face{font-family:"Space Grotesk";src:url(../marque/planches/polices/SpaceGrotesk.ttf) format("truetype");font-weight:300 700}
:root{--indigo:#2D3A8C;--nuit:#1A2259;--ambre:#E3A23B;--pale:#E8EAF6;--encre:#1F2933;--ardoise:#52606D;--trait:#D9DCE3;--brume:#F4F5F9;--succes:#067647;--clair:#C5CAE0}
@page{size:A4;margin:0}
*{box-sizing:border-box}
html{background:#e6e8ee}
body{margin:0;font-family:Inter,"Segoe UI",sans-serif;color:var(--encre);font-size:10pt;line-height:1.45;-webkit-print-color-adjust:exact;print-color-adjust:exact}
em,i{font-style:normal}
.page{width:210mm;height:297mm;margin:0 auto 10mm;background:#fff;position:relative;overflow:hidden;break-after:page;padding:20mm 18mm 18mm}
.page:last-child{break-after:auto}
@media print{html{background:#fff}.page{margin:0}}
@media screen and (max-width:820px){.page{width:100%;height:auto;min-height:0;padding:24px 16px}}
h1,h2,h3{font-family:"Space Grotesk",Inter,sans-serif;font-weight:700;color:var(--nuit);margin:0}
h1{font-size:30pt;line-height:1.1}
h2{font-size:19pt;margin-bottom:5mm}
h3{font-size:11.5pt;margin:0 0 1.5mm}
p{margin:0 0 3mm}
.chapo{font-size:11.5pt;color:var(--ardoise);max-width:165mm;margin-bottom:5mm}
.num{font-family:"Space Grotesk",sans-serif;font-weight:700;font-size:10pt;color:var(--indigo);letter-spacing:.08em;margin-bottom:2mm}
.pied{position:absolute;left:18mm;right:18mm;bottom:10mm;display:flex;justify-content:space-between;align-items:center;font-size:8pt;color:var(--ardoise);border-top:1px solid var(--trait);padding-top:3mm}
.pied svg{height:4.5mm;width:auto}
.grille2{display:grid;grid-template-columns:minmax(0,1fr) minmax(0,1fr);gap:4mm}
.grille3{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:4mm}
.carte{border:1px solid var(--trait);border-radius:3px;padding:3.2mm 4mm;background:#fff}
.carte.brume{background:var(--brume)}
.carte p:last-child,.carte ul:last-child{margin-bottom:0}
ul{margin:0 0 3mm;padding-left:4.5mm}
li{margin-bottom:.6mm}
table{width:100%;border-collapse:collapse;font-size:9.5pt}
th,td{text-align:left;padding:1.4mm 2.5mm;border:1px solid var(--trait);vertical-align:top}
th{background:var(--brume);font-weight:600;color:var(--nuit)}
.etat{display:inline-block;font-size:8pt;font-weight:600;padding:.4mm 2mm;border-radius:2px;white-space:nowrap;border:1px solid}
.livre{color:var(--succes);border-color:#9fd3bb;background:#ecf7f1}
.cours{color:var(--indigo);border-color:#b9bfe3;background:var(--pale)}
.prevu{color:var(--ardoise);border-color:var(--trait);background:var(--brume)}
.chiffres{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:4mm;margin:0 0 5mm}
.chiffre{border:1px solid var(--trait);border-radius:3px;padding:4mm}
.chiffre b{display:block;font-family:"Space Grotesk",sans-serif;font-size:20pt;color:var(--indigo);line-height:1.1}
.chiffre span{font-size:8.5pt;color:var(--ardoise)}
.temps{display:flex;gap:2.2mm;margin-bottom:5mm}
.temps span{display:block;width:11mm;height:1.6mm;background:#fff;border-radius:1px}
.temps span.ambre{background:var(--ambre)}
/* Couverture */
.couverture{background:var(--nuit) url("MOTIF") repeat;background-size:30mm auto;color:#fff;padding:0}
.couverture .voile{position:absolute;inset:0;background:linear-gradient(180deg,rgba(26,34,89,.15) 0%,rgba(26,34,89,.88) 55%,var(--nuit) 100%)}
.couverture .contenu{position:absolute;left:20mm;right:20mm;bottom:28mm}
.couverture .logo svg{width:105mm;height:auto;display:block;margin-bottom:12mm}
.couverture h1{color:#fff;font-size:34pt;margin-bottom:5mm}
.couverture p{color:var(--clair);font-size:13pt;max-width:150mm}
.couverture .bas{position:absolute;left:20mm;right:20mm;bottom:12mm;display:flex;justify-content:space-between;font-size:9pt;color:var(--clair)}
/* Identité */
.bloc-logo{border:1px solid var(--trait);border-radius:3px;display:flex;align-items:center;justify-content:center;height:30mm}
.bloc-logo.nuit{background:var(--nuit);border-color:var(--nuit)}
.bloc-logo.indigo{background:var(--indigo);border-color:var(--indigo)}
.bloc-logo svg{width:100%;max-height:12mm;height:auto}
.bloc-logo.symb svg{width:auto;max-height:18mm;height:18mm}
.legende{font-size:8.5pt;color:var(--ardoise);margin-top:1.5mm}
.nuancier{display:grid;grid-template-columns:repeat(4,1fr);gap:3mm}
.nuance{border:1px solid var(--trait);border-radius:3px;overflow:hidden;font-size:8.5pt}
.nuance .aplat{height:10mm}
.nuance div.txt{padding:2mm 2.5mm}
.nuance b{display:block;color:var(--nuit)}
.modules{display:grid;grid-template-columns:repeat(4,1fr);gap:3mm}
.module{border:1px solid var(--trait);border-radius:3px;padding:2.5mm 3mm;font-size:8.5pt;display:flex;gap:3mm;align-items:center}
.module svg{width:10mm;height:10mm;flex:none}
.module b{display:block;color:var(--nuit);font-size:9.5pt}
.typo{font-family:"Space Grotesk",sans-serif;font-weight:700;font-size:21pt;color:var(--nuit);line-height:1}
.typo.inter{font-family:Inter,sans-serif;font-weight:500}
.non li{margin-bottom:.8mm}
"""

PIED = """<div class="pied"><span>{logo}</span><span>Dossier de présentation, novembre 2026</span><span>{n} / 7</span></div>"""


def pied(n):
    return PIED.format(logo=LOGO, n=n)


def etat(sorte):
    return {
        "livre": '<span class="etat livre">Livré</span>',
        "cours": '<span class="etat cours">En cours</span>',
        "prevu": '<span class="etat prevu">Prévu</span>',
    }[sorte]


PAGES = []

# 1. Couverture
PAGES.append(f"""
<section class="page couverture">
  <div class="voile"></div>
  <div class="contenu">
    <div class="logo">{LOGO_NEG}</div>
    {TROIS_TEMPS}
    <h1>Gestion commerciale</h1>
    <p>Devis, proforma, bons de commande, bordereaux de livraison, factures et paiements, dans un logiciel Windows natif pensé pour les entreprises de l'espace UEMOA.</p>
  </div>
  <div class="bas"><span>Dossier de présentation</span><span>Novembre 2026</span></div>
</section>""")

# 2. En bref
PAGES.append(f"""
<section class="page">
  <div class="num">01</div>
  <h2>Kadenz en bref</h2>
  <p class="chapo">Kadenz est un logiciel de gestion commerciale pour Windows. Il prend la suite d'un outil Excel et VBA, refait en application native sur une vraie base de données.</p>
  <div class="chiffres">
    <div class="chiffre"><b>6</b><span>types de documents : devis, proforma, bon de commande, bordereau de livraison, facture, avoir</span></div>
    <div class="chiffre"><b>7</b><span>langues d'interface, dont l'arabe de droite à gauche</span></div>
    <div class="chiffre"><b>4</b><span>rôles par défaut, droits modifiables par l'Admin</span></div>
    <div class="chiffre"><b>v0.2.2</b><span>dernière version publiée, le 12 novembre 2026</span></div>
  </div>
  <div class="grille2">
    <div class="carte">
      <h3>Pour qui</h3>
      <p>Des structures clientes comme des PME, des bureaux d'études et des coopératives, d'abord au Burkina Faso et dans l'UEMOA. Chaque entreprise a sa propre installation : jamais de base partagée entre clients.</p>
    </div>
    <div class="carte">
      <h3>Ce qu'il remplace</h3>
      <p>Un classeur Excel et VBA. Kadenz garde ses habitudes (en-tête au logo de l'entreprise, textes imprimés repris des modèles) et y ajoute une base fiable, des statuts, des droits et un journal d'audit.</p>
    </div>
    <div class="carte">
      <h3>Au nom du client</h3>
      <p>Le nom Kadenz n'apparaît jamais sur les documents remis aux clients : chaque installation imprime l'identité de l'entreprise utilisatrice, saisie à l'installation.</p>
    </div>
    <div class="carte">
      <h3>Éditeur</h3>
      <p>Mcarre. Logiciel propriétaire, tous droits réservés. Les modules se vendent par clé de licence signée.</p>
    </div>
  </div>
  <h3 style="margin-top:5mm">Le contexte métier, déjà intégré</h3>
  <p>IFU, RCCM et régime d'imposition sur la fiche de l'entreprise ; TVA à 0 % ou 18 % ; remises avant taxes ; retenue à la source sur les bons de commande ; montants en FCFA arrondis au franc, avec le montant en lettres ; numérotation continue par type et par année (par exemple F2026-0001), sans trou ni doublon.</p>
  {pied(2)}
</section>""")

# 3. Fonctions
PAGES.append(f"""
<section class="page">
  <div class="num">02</div>
  <h2>Ce que fait Kadenz aujourd'hui</h2>
  <p class="chapo">La version 1 est complète : les jalons J1 à J10 sont livrés, de la base de données à l'installateur et aux mises à jour.</p>
  <div class="grille2">
    <div class="carte">
      <h3>Documents commerciaux</h3>
      <ul>
        <li>Devis et proforma convertis en facture à la validation.</li>
        <li>Bons de commande et leur réception ; bordereaux de livraison issus d'une facture ou vierges, avec visas et confirmation.</li>
        <li>Avoirs plafonnés au TTC de la facture.</li>
        <li>Statuts, duplication, correction et documents liés ; données du client figées à la validation.</li>
      </ul>
    </div>
    <div class="carte">
      <h3>Paiements et relances</h3>
      <ul>
        <li>Règlements des factures (encaissements) et des bons de commande (décaissements).</li>
        <li>Registre des règlements avec sous-totaux par mode de paiement.</li>
        <li>Suivi des fournisseurs, liste des relances et lettre de relance.</li>
      </ul>
    </div>
    <div class="carte">
      <h3>Impression et échanges</h3>
      <ul>
        <li>PDF de chaque document, des registres et des lettres : en-têtes et pieds répétés, report, filigranes, deux logos.</li>
        <li>Réglages d'impression : gabarits, papier pré-imprimé, marges et calibrage, bas de document par type.</li>
        <li>Export Excel des registres et des documents ; import des tiers et des produits depuis CSV ou Excel, avec aperçu ligne par ligne.</li>
      </ul>
    </div>
    <div class="carte">
      <h3>Pilotage et confort</h3>
      <ul>
        <li>Tableau de bord avec indicateurs et graphiques.</li>
        <li>Interface en français, anglais, arabe, japonais, chinois, turc et espagnol, au choix de chaque compte.</li>
        <li>Navigation par bandeau ou barre latérale, raccourcis clavier, création rapide d'un client.</li>
      </ul>
    </div>
  </div>
  <h3 style="margin-top:5mm">Modules activés par la licence</h3>
  <table>
    <tr><th style="width:32%">Module</th><th>Contenu</th></tr>
    <tr><td>Socle (toujours actif)</td><td>Entreprise, tiers, produits, comptes et rôles, sauvegarde, audit, factures</td></tr>
    <tr><td>Devis, Proforma</td><td>Propositions commerciales et factures proforma</td></tr>
    <tr><td>Bons de commande, Livraisons</td><td>Achats et réception, bordereaux de livraison</td></tr>
    <tr><td>Paiements, Relances</td><td>Règlements, registres, relances (les relances exigent les paiements)</td></tr>
    <tr><td>Tableau de bord</td><td>Indicateurs et graphiques</td></tr>
  </table>
  <p class="legende">Un module désactivé disparaît de la navigation ; ses données restent en base et reviennent s'il est réactivé.</p>
  {pied(3)}
</section>""")

# 4. Confiance et technique
PAGES.append(f"""
<section class="page">
  <div class="num">03</div>
  <h2>Sécurité, fiabilité, technique</h2>
  <p class="chapo">Les données d'une entreprise restent chez elle, chiffrées, et chaque action est tracée.</p>
  <div class="grille2">
    <div class="carte">
      <h3>Données protégées</h3>
      <ul>
        <li>Base chiffrée (SQLite3 Multiple Ciphers, ChaCha20-Poly1305), clé protégée par Windows et liée au compte.</li>
        <li>Sauvegardes chiffrées par mot de passe, dix copies automatiques gardées, restauration qui met la base courante de côté.</li>
        <li>Écran « Démarrage impossible » qui explique la cause et propose la restauration.</li>
      </ul>
    </div>
    <div class="carte">
      <h3>Contrôle et traçabilité</h3>
      <ul>
        <li>Comptes et rôles : Admin, Comptable, Commercial, Consultation ; matrice des droits modifiable.</li>
        <li>Journal d'audit des actions sensibles.</li>
        <li>Identifiants jamais réutilisés, numéros sans trou ni doublon.</li>
      </ul>
    </div>
    <div class="carte">
      <h3>Licence</h3>
      <ul>
        <li>Clé de licence signée (Ed25519), vérifiée hors ligne, liée au poste.</li>
        <li>Sans clé valide, le socle seul reste disponible.</li>
      </ul>
    </div>
    <div class="carte">
      <h3>Installation et mises à jour</h3>
      <ul>
        <li>Installateur Windows en sept langues, et paquet MSI pour un déploiement par msiexec ou GPO.</li>
        <li>Mise à jour vérifiée au démarrage : manifeste signé, empreinte SHA-256 contrôlée avant installation.</li>
        <li>Les mises à jour ne touchent jamais aux données.</li>
      </ul>
    </div>
  </div>
  <h3 style="margin-top:5mm">Sous le capot</h3>
  <table>
    <tr><th style="width:32%">Élément</th><th>Choix</th></tr>
    <tr><td>Langage</td><td>Rust, édition 2024 ; exécutable natif 64 bits</td></tr>
    <tr><td>Interface</td><td>Slint, interface déclarative compilée dans l'exécutable</td></tr>
    <tr><td>Base de données</td><td>SQLite embarqué et chiffré, migrations appliquées au démarrage</td></tr>
    <tr><td>Documents</td><td>PDF par krilla, Excel par rust_xlsxwriter</td></tr>
    <tr><td>Plateforme</td><td>Windows 10 et 11, 64 bits ; fonctionne sans Internet</td></tr>
    <tr><td>Qualité</td><td>Règles métier isolées dans un crate testé seul ; format, Clippy et tests vérifiés à chaque envoi sur GitHub</td></tr>
  </table>
  {pied(4)}
</section>""")

# 5. Phase 2
PAGES.append(f"""
<section class="page">
  <div class="num">04</div>
  <h2>La suite : plusieurs postes, un serveur</h2>
  <p class="chapo">La phase 2 fait passer Kadenz d'un poste seul à une équipe de 5 à 20 postes, reliés à un serveur installé dans l'entreprise. À terme, Kadenz vise une suite de logiciels comparable à Sage, avec la comptabilité comme produit suivant.</p>
  <div class="grille3">
    <div class="carte brume">
      <h3>Serveur chez le client</h3>
      <p>Installé sur une machine de l'entreprise (service Windows). Il impose les droits, l'heure, la validation et la numérotation, et sauvegarde chaque jour.</p>
    </div>
    <div class="carte brume">
      <h3>Travail hors connexion</h3>
      <p>Serveur ou Internet coupé, on continue à saisir. Au retour, tout se resynchronise ; les conflits sont montrés et tranchés par une personne, rien n'est écarté en silence.</p>
    </div>
    <div class="carte brume">
      <h3>Accès à distance</h3>
      <p>En option, par un relais qui transmet les échanges chiffrés de bout en bout sans rien garder. Les données ne sont jamais stockées chez l'éditeur.</p>
    </div>
  </div>
  <h3 style="margin-top:5mm">Fondations de la phase 2</h3>
  <table>
    <tr><th style="width:9%">Jalon</th><th>Contenu</th><th style="width:15%">État</th></tr>
    <tr><td>F1</td><td>Identifiants globaux et opérations rejouables sans double effet</td><td>{etat("livre")}</td></tr>
    <tr><td>F2</td><td>Serveur en service Windows, base chiffrée, sessions TLS 1.3 épinglées</td><td>{etat("livre")}</td></tr>
    <tr><td>F3</td><td>Licence d'abonnement d'entreprise, avertissements, lecture seule à l'échéance</td><td>{etat("livre")}</td></tr>
    <tr><td>F4</td><td>Fenêtre d'administration, inscription des postes par code, découverte sur le réseau</td><td>{etat("livre")}</td></tr>
    <tr><td>F5</td><td>Poste relié en ligne : copie locale, comptes communs, numérotation par le serveur</td><td>{etat("cours")}</td></tr>
    <tr><td>F6</td><td>Hors connexion : file d'envoi, conflits, opérations en attente pour l'Admin</td><td>{etat("cours")}</td></tr>
    <tr><td>F7</td><td>Reprise d'une base V1, sauvegardes du serveur</td><td>{etat("prevu")}</td></tr>
    <tr><td>F8</td><td>Relais et accès à distance</td><td>{etat("prevu")}</td></tr>
    <tr><td>F9</td><td>Portail web des licences : éditeur et revendeurs</td><td>{etat("prevu")}</td></tr>
    <tr><td>F10</td><td>Mises à jour, signature du code, première version de la phase 2</td><td>{etat("prevu")}</td></tr>
  </table>
  <p class="legende">F5 est codé et vérifié par un essai à deux postes ; il reste l'essai à l'écran. F6 est en cours de développement.</p>
  {pied(5)}
</section>""")

# 6. Feuille de route et modèle
PAGES.append(f"""
<section class="page">
  <div class="num">05</div>
  <h2>Feuille de route et modèle</h2>
  <p class="chapo">Après les fondations viennent les fonctions métier, puis des chantiers indépendants intercalés selon la demande des clients.</p>
  <table>
    <tr><th style="width:30%">Chantier</th><th>Objet</th><th style="width:15%">État</th></tr>
    <tr><td>Gestion de stock</td><td>Entrées à la réception, sorties à la livraison, valorisation</td><td>{etat("prevu")}</td></tr>
    <tr><td>Export SYSCOHADA</td><td>Journal des ventes, TVA collectée ; première brique de la comptabilité</td><td>{etat("prevu")}</td></tr>
    <tr><td>Envoi par email</td><td>Devis, factures et relances envoyés depuis Kadenz</td><td>{etat("prevu")}</td></tr>
    <tr><td>Multi-devise</td><td>Devise de l'entreprise en paramètre, documents en devise étrangère</td><td>{etat("prevu")}</td></tr>
    <tr><td>Signature électronique</td><td>Besoin à préciser (cachet, certificat ou signature à distance)</td><td>{etat("prevu")}</td></tr>
    <tr><td>Site web</td><td>Présentation du produit, puis abonnement et téléchargements</td><td>{etat("prevu")}</td></tr>
  </table>
  <div class="grille2" style="margin-top:5mm">
    <div class="carte">
      <h3>Abonnement</h3>
      <ul>
        <li>Mensuel ou annuel, compté par entreprise, avec des postes inclus et des postes en plus.</li>
        <li>Paiement par virement ou chèque, par carte en ligne, ou en espèces auprès de l'éditeur ou d'un revendeur.</li>
        <li>Avertissements 30 jours, 7 jours et la veille ; à l'échéance, lecture seule : consultation, impression et export restent toujours possibles.</li>
        <li>Les licences V1 déjà émises restent perpétuelles.</li>
      </ul>
    </div>
    <div class="carte">
      <h3>Revendeurs et zones</h3>
      <ul>
        <li>Chaque revendeur a un compte sur le portail : il enregistre les paiements de ses clients et renouvelle leurs licences, l'éditeur garde la vue sur tout.</li>
        <li>Zones visées : UEMOA, CEMAC et autres pays OHADA (Guinée, Comores, RD Congo). Devises, TVA et règles de facturation de chaque pays restent à vérifier sur des sources.</li>
      </ul>
    </div>
  </div>
  {pied(6)}
</section>""")

# 7. Identité
nuances = [
    ("#2D3A8C", "Indigo", "#2D3A8C", "Logo, accent, boutons principaux"),
    ("#1A2259", "Nuit", "#1A2259", "Bandeaux de marque, mot du logo"),
    ("#E3A23B", "Ambre", "#E3A23B", "Troisième temps, sur fond sombre"),
    ("#1F2933", "Encre", "#1F2933", "Texte"),
]
nuancier = "".join(
    f'<div class="nuance"><div class="aplat" style="background:{c}"></div><div class="txt"><b>{n}</b>{h}<br>{r}</div></div>'
    for c, n, h, r in nuances
)
modules = "".join(
    f'<div class="module">{svg(f)}<div><b>{n}</b>{d}</div></div>' for f, n, d in MODULES
)
PAGES.append(f"""
<section class="page">
  <div class="num">06</div>
  <h2>Identité visuelle : « Quatre pièces »</h2>
  <p class="chapo" style="margin-bottom:5mm">Quatre formes simples, un carré, un quart de disque, un triangle et un disque, s'emboîtent sur une grille de deux par deux, comme les modules que la licence assemble. Le disque marque le temps fort de la cadence.</p>
  <div class="grille3">
    <div><div class="bloc-logo" style="padding:0 6mm">{LOGO}</div><div class="legende">Logo horizontal, version principale</div></div>
    <div><div class="bloc-logo nuit" style="padding:0 6mm">{LOGO_NEG}</div><div class="legende">En négatif, sur la Nuit ou l'Indigo</div></div>
    <div><div class="bloc-logo indigo symb">{SYMBOLE_BLANC}</div><div class="legende">Symbole seul, une seule couleur</div></div>
  </div>
  <h3 style="margin-top:5mm">Couleurs</h3>
  <div class="nuancier">{nuancier}</div>
  <div class="grille2" style="margin-top:5mm">
    <div>
      <h3>Typographie</h3>
      <div class="carte">
        <div class="typo">Space Grotesk</div>
        <p class="legende" style="margin-bottom:3mm">Titres de l'application et des documents ; mot du logo (600)</p>
        <div class="typo inter">Inter</div>
        <p class="legende" style="margin-bottom:0">Texte, interface, montants en chiffres tabulaires. Deux polices sous licence SIL OFL 1.1.</p>
      </div>
    </div>
    <div>
      <h3>À ne pas faire</h3>
      <div class="carte">
        <ul class="non">
          <li>Déformer, étirer, pencher ou faire tourner le logo.</li>
          <li>Changer l'ordre, l'orientation ou l'espacement des pièces.</li>
          <li>Ajouter une deuxième couleur, une ombre, un contour ou un dégradé.</li>
          <li>Poser le logo couleur sur un fond chargé.</li>
          <li>Recomposer le mot au clavier : utiliser les fichiers.</li>
        </ul>
      </div>
    </div>
  </div>
  <h3 style="margin-top:5mm">Familles de modules (proposition)</h3>
  <div class="modules">{modules}</div>
  <p class="legende">Charte complète en neuf planches (kadenz-identite.pdf) : motifs Quatre pièces, Tournant et Fondu, signalétique, papeterie, tampons par degré, mises en situation.</p>
  {pied(7)}
</section>""")

HTML = f"""<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Kadenz, dossier de présentation</title>
<style>{CSS.replace("MOTIF", MOTIF)}</style>
</head>
<body>
{"".join(PAGES)}
</body>
</html>
"""

sortie = ICI / "kadenz-presentation.html"
sortie.write_text(HTML, encoding="utf-8")
pdf = ICI / "kadenz-presentation.pdf"
subprocess.run(
    [
        CHROME,
        "--headless=new",
        "--disable-gpu",
        "--no-pdf-header-footer",
        "--virtual-time-budget=4000",
        f"--print-to-pdf={pdf}",
        sortie.as_uri(),
    ],
    check=True,
    capture_output=True,
)
print(sortie)
print(pdf)
