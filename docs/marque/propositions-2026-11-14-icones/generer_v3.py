"""Troisième tour des icônes (2026-11-14) : le jeu complet, 49 icônes, dans le style retenu.

Choix de l'utilisateur : trait fin de A au repos, duo de B à l'état actif ; Livraisons en L1
(colis livré), Produits en étiquette. Ce tour dessine les 34 icônes restantes et range le jeu
comme dans l'application (menu/, actions/, etat/), chaque icône du menu avec sa version -actif.

Usage : python generer_v3.py ; sorties dans icones/jeu/ et planche-v3.html.
"""

import generer_v2 as v2
from generer import ICI, chemin, cercle, rect

DOC = "M14 2.5H6.5A1.5 1.5 0 0 0 5 4v16a1.5 1.5 0 0 0 1.5 1.5h11A1.5 1.5 0 0 0 19 20V7.5Z"
PLI = "M14 2.5v5h5"

NOUVELLES = {
    # --- menu et rubriques des Paramètres ---
    "proforma": [
        ("trait", chemin("M7.5 6V2.5H20v14.5h-3.5")),
        ("corps", chemin("M4 6h12.5v15l-2.08-1.33L12.33 21l-2.08-1.33L8.17 21l-2.09-1.33L4 21Z")),
        ("detail", chemin("M7 10.5h6.5M7 14h4")),
    ],
    "suivi-fournisseurs": [
        ("corps", rect(4, 4.5, 13, 16.5)),
        ("corps", rect(7.5, 2.5, 6, 4, 1)),
        ("detail", chemin("M7.5 11h6M7.5 14.5h3")),
        ("dessus", cercle(17.5, 17.5, 4.5)),
        ("trait", chemin("M17.5 15.25v2.25l1.5 1")),
    ],
    "a-propos": [
        ("corps", cercle(12, 12, 9.5)),
        ("detail", chemin("M12 11v5.5M12 7.75h.01")),
    ],
    "documents": [
        ("corps", chemin(DOC)), ("detail", chemin(PLI)),
        ("detail", chemin("M10 11v7M13.5 11v7M8 13h7.5M8 16h7.5")),
    ],
    "entreprise": [
        ("corps", chemin("M3.5 21V4.5A1.5 1.5 0 0 1 5 3h8a1.5 1.5 0 0 1 1.5 1.5V21")),
        ("corps", chemin("M14.5 9H19a1.5 1.5 0 0 1 1.5 1.5V21")),
        ("trait", chemin("M2.5 21h19")),
        ("detail", chemin("M7.5 7.5h3M7.5 11h3M7.5 14.5h3M9 21v-3")),
    ],
    "exports": [
        ("corps", chemin("M12 21H6.5A1.5 1.5 0 0 1 5 19.5v-15A1.5 1.5 0 0 1 6.5 3H14l5 5v3")),
        ("detail", chemin("M14 3v5h5M8.5 12h4")),
        ("trait", chemin("M13.5 16.5H21M18 13.5l3 3-3 3")),
    ],
    "licence": [
        ("corps", cercle(7.5, 16.5, 5)),
        ("detail", cercle(7.5, 16.5, 1.5)),
        ("trait", chemin("M11 13 20.5 3.5M17.5 6.5l2.5 2.5M15 9l2 2")),
    ],
    "mises-a-jour": [
        ("trait", chemin("M20 10A8.5 8.5 0 0 0 4.6 7.5M4 3.5v4h4")),
        ("trait", chemin("M4 14a8.5 8.5 0 0 0 15.4 2.5M20 20.5v-4h-4")),
    ],
    "mon-compte": [
        ("corps", cercle(12, 12, 9.5)),
        ("detail", cercle(12, 10, 3)),
        ("detail", chemin("M6.5 18.5a6.5 6.5 0 0 1 11 0")),
    ],
    "navigation": [
        ("corps", rect(2.5, 4, 19, 16)),
        ("detail", chemin("M2.5 8.5h19M8.5 8.5V20")),
    ],
    "paiements": [
        ("corps", rect(2.5, 5, 19, 14)),
        ("detail", chemin("M2.5 9.5h19M6 15h4")),
    ],
    "sauvegardes": [
        ("corps", rect(2.5, 3.5, 19, 5, 1)),
        ("corps", chemin("M4 8.5V19a1.5 1.5 0 0 0 1.5 1.5h13A1.5 1.5 0 0 0 20 19V8.5")),
        ("detail", chemin("M12 11.5V17M9.5 14.5 12 17l2.5-2.5")),
    ],
    "utilisateurs": [
        ("corps", cercle(9, 8, 3.5)),
        ("corps", chemin("M2.5 20a6.5 6.5 0 0 1 13 0Z")),
        ("dessus", chemin("M17.5 11.5l4 1.5v3c0 2.5-1.8 4.3-4 5-2.2-.7-4-2.5-4-5v-3Z")),
        ("trait", chemin("M16 16.25l1 1 2-2")),
    ],
    # --- actions ---
    "annuler": [("trait", chemin("M9 14 4 9l5-5M4 9h10.5a5.5 5.5 0 0 1 0 11H11"))],
    "apercu": [
        ("corps", chemin("M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12Z")),
        ("detail", cercle(12, 12, 3)),
    ],
    "bandeau": [
        ("corps", rect(2.5, 4, 19, 16)),
        ("detail", chemin("M2.5 9h19M6 6.5h.01M9 6.5h.01M12 6.5h.01")),
    ],
    "barre": [
        ("corps", rect(2.5, 4, 19, 16)),
        ("detail", chemin("M9 4v16M5 8h1.5M5 11h1.5")),
    ],
    "convertir": [("trait", chemin("M4 8h15M15.5 4.5 19 8l-3.5 3.5M20 16H5M8.5 12.5 5 16l3.5 3.5"))],
    "copier": [
        ("trait", chemin("M15.5 8.5V4A1.5 1.5 0 0 0 14 2.5H4A1.5 1.5 0 0 0 2.5 4v10A1.5 1.5 0 0 0 4 15.5h4.5")),
        ("corps", rect(8.5, 8.5, 13, 13, 1.5)),
    ],
    "corriger": [
        ("corps", chemin("M15 4.5l4.5 4.5L9 19.5l-5.5 1 1-5.5Z")),
        ("detail", chemin("M12.5 7l4.5 4.5")),
    ],
    "deconnexion": [
        ("corps", cercle(8, 8, 3.5)),
        ("corps", chemin("M2 20a6 6 0 0 1 12 0Z")),
        ("trait", chemin("M16 12.5h6M19 9.5l3 3-3 3")),
    ],
    "enregistrer": [
        ("corps", chemin("M5 3.5h11l4.5 4.5V19a1.5 1.5 0 0 1-1.5 1.5H5A1.5 1.5 0 0 1 3.5 19V5A1.5 1.5 0 0 1 5 3.5Z")),
        ("detail", chemin("M7.5 3.5V8h7V3.5M7 20.5v-6h10v6")),
    ],
    "excel": [
        ("corps", chemin(DOC)), ("detail", chemin(PLI)),
        ("detail", rect(7.5, 11, 9, 7, 0.5)),
        ("detail", chemin("M7.5 14.5h9M12 11v7")),
    ],
    "pdf": [
        ("corps", chemin(DOC)), ("detail", chemin(PLI)),
        ("detail", chemin("M12 10.5v6.5M9.5 14.5 12 17l2.5-2.5")),
    ],
    "fermer": [("trait", chemin("M6 6l12 12M18 6 6 18"))],
    "filtrer": [("corps", chemin("M3 4.5h18l-7 8.5v6l-4 2v-8Z"))],
    "importer": [("trait", chemin("M3.5 15v3.5A1.5 1.5 0 0 0 5 20h14a1.5 1.5 0 0 0 1.5-1.5V15M12 3.5v11M7.5 10l4.5 4.5 4.5-4.5"))],
    "precedent": [("trait", chemin("M15 5l-7 7 7 7"))],
    "suivant": [("trait", chemin("M9 5l7 7-7 7"))],
    "valider": [("trait", chemin("M4.5 12.5l5 5 10-11"))],
    # --- état de la connexion (16 px dans l'application) ---
    "relie": [("trait", chemin("M8.5 11.5l-2.5 2.5a3.54 3.54 0 0 0 5 5l2.5-2.5M15.5 12.5l2.5-2.5a3.54 3.54 0 0 0-5-5l-2.5 2.5M9.5 14.5l5-5"))],
    "coupe": [("trait", chemin("M8.5 11.5l-2.5 2.5a3.54 3.54 0 0 0 5 5l2.5-2.5M15.5 12.5l2.5-2.5a3.54 3.54 0 0 0-5-5l-2.5 2.5"
                                "M4 7.5h2.5M7.5 4v2.5M20 16.5h-2.5M16.5 20v-2.5"))],
    "attente": [
        ("corps", cercle(12, 12, 9.5)),
        ("detail", chemin("M12 7v5l3.5 2")),
    ],
}
v2.TOUTES.update(NOUVELLES)

# Nom dans l'application -> dessin
MENU = {n: n for n in ["tableau-de-bord", "devis", "proforma", "bons-commande", "factures", "avoirs",
                       "reglements", "suivi-fournisseurs", "relances", "tiers", "parametres"]}
MENU.update({"livraisons": "livraisons-colis-livre", "produits": "produits-etiquette"})
ORDRE_MENU = ["tableau-de-bord", "devis", "proforma", "bons-commande", "livraisons", "factures", "avoirs",
              "reglements", "suivi-fournisseurs", "relances", "tiers", "produits", "parametres"]
RUBRIQUES = ["mon-compte", "navigation", "entreprise", "documents", "paiements", "exports", "impression",
             "utilisateurs", "sauvegardes", "licence", "mises-a-jour", "a-propos"]
for r in RUBRIQUES:
    MENU[r] = "imprimer" if r == "impression" else r
ACTIONS = ["annuler", "apercu", "bandeau", "barre", "convertir", "copier", "corriger", "deconnexion",
           "enregistrer", "excel", "fermer", "filtrer", "importer", "imprimer", "nouveau", "pdf",
           "precedent", "rechercher", "suivant", "supprimer", "valider"]
ETAT = ["relie", "attente", "coupe"]

LIB = {
    "tableau-de-bord": "Tableau de bord", "devis": "Devis", "proforma": "Proforma",
    "bons-commande": "Bons de commande", "livraisons": "Livraisons", "factures": "Factures",
    "avoirs": "Avoirs", "reglements": "Règlements", "suivi-fournisseurs": "Suivi fournisseurs",
    "relances": "Relances", "tiers": "Tiers", "produits": "Produits", "parametres": "Paramètres",
    "mon-compte": "Mon compte", "navigation": "Navigation et affichage", "entreprise": "Entreprise",
    "documents": "Documents", "paiements": "Paiements", "exports": "Textes imprimés et exports",
    "impression": "Impression", "utilisateurs": "Utilisateurs", "sauvegardes": "Sauvegardes",
    "licence": "Licence", "mises-a-jour": "Mises à jour", "a-propos": "À propos",
    "annuler": "Annuler", "apercu": "Aperçu", "bandeau": "Bandeau", "barre": "Barre latérale",
    "convertir": "Convertir", "copier": "Copier", "corriger": "Corriger", "deconnexion": "Déconnexion",
    "enregistrer": "Enregistrer", "excel": "Excel", "fermer": "Fermer", "filtrer": "Filtrer",
    "importer": "Importer", "imprimer": "Imprimer", "nouveau": "Nouveau", "pdf": "PDF",
    "precedent": "Précédent", "rechercher": "Rechercher", "suivant": "Suivant",
    "supprimer": "Supprimer", "valider": "Valider", "relie": "Relié", "attente": "En attente",
    "coupe": "Coupé",
}

JEU = ICI / "icones" / "jeu"


def ecrire():
    for dossier, noms, dessin, actif in (("menu", ORDRE_MENU + RUBRIQUES, MENU, True),
                                         ("actions", ACTIONS, {n: n for n in ACTIONS}, False),
                                         ("etat", ETAT, {n: n for n in ETAT}, False)):
        (JEU / dossier).mkdir(parents=True, exist_ok=True)
        for n in noms:
            (JEU / dossier / f"{n}.svg").write_text(v2.icone(dessin[n], False), encoding="utf-8")
            if actif:
                (JEU / dossier / f"{n}-actif.svg").write_text(v2.icone(dessin[n], True), encoding="utf-8")


def s(dossier, n, actif=False):
    return f"icones/jeu/{dossier}/{n}{'-actif' if actif else ''}.svg"


def case_menu(n):
    return (f'<figure><div class="paire"><img src="{s("menu", n)}" width="48" height="48">'
            f'<span class="tuile"><img class="blanc" src="{s("menu", n, True)}" width="48" height="48"></span></div>'
            f'<div class="paire petite"><img src="{s("menu", n)}" width="18" height="18">'
            f'<span class="tuile"><img class="blanc" src="{s("menu", n, True)}" width="18" height="18"></span></div>'
            f'<figcaption>{LIB[n]}</figcaption></figure>')


def case_simple(dossier, n, petit=18):
    return (f'<figure><div class="paire"><img src="{s(dossier, n)}" width="48" height="48"></div>'
            f'<div class="paire petite"><img src="{s(dossier, n)}" width="{petit}" height="{petit}"></div>'
            f'<figcaption>{LIB[n]}</figcaption></figure>')


def liste(noms, actif):
    return "<nav>" + "".join(
        f'<div class="entree{" actif" if n == actif else ""}"><img class="{"blanc" if n == actif else "gris"}" '
        f'src="{s("menu", n, n == actif)}" width="18" height="18"><span>{LIB[n]}</span></div>' for n in noms) + "</nav>"


def etat_barre():
    out = []
    for n, txt, cls in (("relie", "Relié au serveur", "gris"), ("attente", "Reconnexion en cours", "gris"),
                        ("coupe", "Serveur injoignable", "rouge")):
        out.append(f'<div class="etat {cls}"><img class="{cls}" src="{s("etat", n)}" width="16" height="16">'
                   f'<span>{txt}</span></div>')
    return "".join(out)


def planche():
    return f"""<!doctype html>
<html lang="fr"><head><meta charset="utf-8"><title>Icônes : jeu complet</title>
<style>
body {{ margin: 0; background: #f4f5f8; font: 13px/1.4 Inter, "Segoe UI", sans-serif; color: #1f2933; }}
header {{ padding: 28px 32px 8px; }}
h1 {{ font: 600 22px Manrope, "Segoe UI", sans-serif; margin: 0 0 4px; }}
header p, .tete p {{ margin: 0; color: #52606d; }}
section {{ background: #fff; margin: 16px 32px; padding: 20px 24px; border: 1px solid #e3e6ee; border-radius: 4px; }}
.tete {{ display: flex; gap: 16px; align-items: baseline; margin-bottom: 14px; }}
h2 {{ font: 600 16px Manrope, "Segoe UI", sans-serif; margin: 0; }}
.grille {{ display: grid; grid-template-columns: repeat(7, 150px); gap: 16px 6px; }}
.grille.simple {{ grid-template-columns: repeat(11, 96px); }}
figure {{ margin: 0; text-align: center; }}
.paire {{ display: flex; justify-content: center; gap: 8px; align-items: center; margin-bottom: 6px; }}
.tuile {{ display: inline-flex; background: #2d3a8c; border-radius: 4px; padding: 6px; }}
.petite .tuile {{ padding: 5px; }}
figcaption {{ font-size: 11px; color: #52606d; }}
.menus {{ display: flex; gap: 16px; align-items: flex-start; }}
nav {{ width: 230px; flex: none; padding: 8px; border: 1px solid #e3e6ee; border-radius: 4px; }}
.entree {{ height: 36px; display: flex; align-items: center; gap: 10px; padding: 0 12px; border-radius: 4px; color: #52606d; }}
.entree.actif {{ background: #2d3a8c; color: #fff; }}
.etats {{ display: flex; flex-direction: column; gap: 10px; padding: 8px 4px; }}
.etat {{ display: flex; align-items: center; gap: 8px; color: #52606d; }}
.etat.rouge {{ color: #c0392b; }}
img.gris {{ filter: invert(36%) sepia(10%) saturate(700%) hue-rotate(170deg) brightness(92%); }}
img.blanc {{ filter: invert(1); }}
img.rouge {{ filter: invert(27%) sepia(68%) saturate(2400%) hue-rotate(347deg) brightness(88%); }}
</style></head><body>
<header><h1>Jeu d'icônes de Kadenz : les 49 icônes</h1>
<p>Trait fin au repos, duo à l'état actif. Livraisons : colis livré ; Produits : étiquette. 48 px en haut, taille de l'application dessous (18 px, 16 px pour l'état de la connexion).</p></header>
<section><div class="tete"><h2>Menu</h2><p>13 entrées, au repos et actives</p></div>
<div class="grille">{''.join(case_menu(n) for n in ORDRE_MENU)}</div></section>
<section><div class="tete"><h2>Rubriques des Paramètres</h2><p>12 rubriques, au repos et actives</p></div>
<div class="grille">{''.join(case_menu(n) for n in RUBRIQUES)}</div></section>
<section><div class="tete"><h2>Actions</h2><p>21 icônes des boutons</p></div>
<div class="grille simple">{''.join(case_simple("actions", n) for n in ACTIONS)}</div></section>
<section><div class="tete"><h2>État de la connexion</h2><p>16 px, rouge quand le serveur est injoignable</p></div>
<div class="menus"><div class="grille simple" style="grid-template-columns: repeat(3, 96px)">{''.join(case_simple("etat", n, 16) for n in ETAT)}</div>
<div class="etats">{etat_barre()}</div></div></section>
<section><div class="tete"><h2>En situation</h2><p>Menu latéral et liste des rubriques</p></div>
<div class="menus">{liste(ORDRE_MENU, "livraisons")}{liste(RUBRIQUES, "utilisateurs")}</div></section>
</body></html>
"""


if __name__ == "__main__":
    ecrire()
    (ICI / "planche-v3.html").write_text(planche(), encoding="utf-8")
    print("ok")
