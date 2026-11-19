"""Maquettes des pages intérieures du portail des licences (2026-11-19).

Écrit interieur-*.html à côté de ce script : python generer_interieur.py
Libellés repris de crates/portail/src/pages ; toutes les données sont des
exemples (entreprises, montants, codes, identifiants), marquées comme telles.
Aucun style en ligne ni script : identite.css puis interieur.css.
"""
import pathlib

ICI = pathlib.Path(__file__).resolve().parent

SYMBOLE = (
    '<svg class="symbole" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" aria-hidden="true">'
    '<path fill="#FFFFFF" d="M16 16 L120 16 L120 120 L16 120 Z M136 120 L136 14.44 A105.56 105.56 0 0 1 241.56 120 Z '
    'M16 136 L120 240 L16 240 Z M134.44 188 A53.56 53.56 0 1 1 241.56 188 A53.56 53.56 0 1 1 134.44 188 Z"/></svg>'
)

MENU = [
    ("entreprises", "interieur-entreprises.html", "Entreprises"),
    ("codes", "interieur-code.html", "Codes à vendre"),
    ("comptes", "interieur-revendeurs.html", "Revendeurs"),
    ("journal", "interieur-journal.html", "Journal"),
]


def page(titre, rubrique, contenu):
    liens = "".join(
        f'<a class="menu-lien" href="{href}"{" aria-current=\"page\"" if cle == rubrique else ""}>{texte}</a>'
        for cle, href, texte in MENU
    )
    return f"""<!DOCTYPE html>
<!-- Maquette du 2026-11-19, pages intérieures du portail : données d'exemple. -->
<html lang="fr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="robots" content="noindex, nofollow">
<title>{titre} - Portail des licences Kadenz (maquette, données d'exemple)</title>
<link rel="stylesheet" href="identite.css">
<link rel="stylesheet" href="interieur.css">
</head>
<body>
<header class="bandeau">
<div class="bandeau-interieur">
<a class="marque" href="interieur-entreprises.html">{SYMBOLE}<span class="marque-nom">Portail des licences</span></a>
<nav class="menu" aria-label="Menu">{liens}</nav>
<div class="compte">
<a class="compte-nom" href="#" title="Changer de mot de passe">Éditeur exemple<span class="compte-role">Éditeur</span></a>
<form class="deconnexion" method="post" action="#"><button class="bouton bouton-sombre" type="submit">Se déconnecter</button></form>
</div>
</div>
</header>
<main class="contenu">
{contenu}
</main>
</body>
</html>
"""


def titre_page(h1, actions="", surtitre="", identifiant=""):
    sur = f'<span class="surtitre-page">{surtitre}</span>' if surtitre else ""
    ident = f'<span class="identifiant">{identifiant}</span>' if identifiant else ""
    return (
        f'<div class="titre-page"><div>{sur}<h1>{h1}</h1>{ident}</div>'
        f'<div class="titre-actions">{actions}</div></div>'
    )


def chiffres(*elements):
    lis = "".join(
        f'<li class="chiffre{" chiffre-accent" if accent else ""}">'
        f'<span class="chiffre-valeur{" chiffre-date" if date else ""}">{valeur}</span>'
        f'<span class="chiffre-libelle">{libelle}</span></li>'
        for valeur, libelle, accent, date in elements
    )
    return f'<ul class="chiffres">{lis}</ul>'


def tableau(entetes, lignes):
    """entetes : [(texte, classe)] ; lignes : [[(contenu, classe)]]."""
    th = "".join(f'<th{f" class=\"{c}\"" if c else ""}>{t}</th>' for t, c in entetes)
    corps = ""
    for ligne in lignes:
        tds = "".join(
            f'<td{f" class=\"{c}\"" if c else ""} data-libelle="{entetes[i][0]}">{v}</td>'
            for i, (v, c) in enumerate(ligne)
        )
        corps += f"<tr>{tds}</tr>"
    return (
        f'<div class="carte"><div class="tableau-cadre"><table><thead><tr>{th}</tr></thead>'
        f"<tbody>{corps}</tbody></table></div></div>"
    )


ETAT_NEUTRE = '<span class="etat etat-neutre">{}</span>'
ETAT_ATTENTE = '<span class="etat etat-attente">{}</span>'
ETAT_OK = '<span class="etat etat-ok">{}</span>'

MODULES = ["Devis", "Proformas", "Bons de commande", "Livraisons", "Paiements", "Tableau de bord", "Relances"]


def cases(cochees):
    return "".join(
        f'<label class="case"><input type="checkbox" name="module_{i}" value="1"{" checked" if m in cochees else ""}>{m}</label>'
        for i, m in enumerate(MODULES)
    )


# Données d'exemple
E1 = ("Exemple Quincaillerie du Centre", "3f6c2a1e-8b4d-4c7a-9e21-5d0b7f3a6c18")
E2 = ("Exemple Pharmacie des Lacs", "a91d4e70-2c5b-4f88-b3a6-0e7c19d2f455")
E3 = ("Exemple Imprimerie Moderne", "6e0b8c33-71fa-4d29-8c5e-b2a4d9f01e7c")
E4 = ("Exemple Transit du Sahel", "c2f7a5d8-0e93-4b61-a7d4-38e6b5c9f210")
REVENDEUR = "Revendeur exemple Nord"
CODE = "H7KQM-3RTWX-P9DFN-2YVBC-8LSAE"


def entreprises():
    lignes = [
        [(f'<a href="interieur-fiche.html">{E1[0]}</a>', ""), (f'<span class="identifiant">{E1[1]}</span>', ""),
         (REVENDEUR, ""), ("31/12/2026", "court"), ("3", "nombre"), (ETAT_ATTENTE.format("1 paiement à utiliser"), "court")],
        [(f'<a href="interieur-fiche.html">{E2[0]}</a>', ""), (f'<span class="identifiant">{E2[1]}</span>', ""),
         ("Client direct", ""), ("15/03/2027", "court"), ("5", "nombre"), (ETAT_NEUTRE.format("Aucun à utiliser"), "court")],
        [(f'<a href="interieur-fiche.html">{E3[0]}</a>', ""), (f'<span class="identifiant">{E3[1]}</span>', ""),
         ("Client direct", ""), (ETAT_NEUTRE.format("Aucune licence"), "court"), ("-", "nombre"),
         (ETAT_ATTENTE.format("1 paiement à utiliser"), "court")],
        [(f'<a href="interieur-fiche.html">{E4[0]}</a>', ""), (f'<span class="identifiant">{E4[1]}</span>', ""),
         (REVENDEUR, ""), ("30/06/2027", "court"), ("2", "nombre"), (ETAT_NEUTRE.format("Aucun à utiliser"), "court")],
    ]
    contenu = (
        titre_page("Entreprises", '<a class="bouton bouton-principal" href="#">Nouvelle entreprise</a>')
        + chiffres(("4", "entreprises", False, False), ("2", "paiements à utiliser", True, False),
                   ("1", "entreprise sans licence", False, False))
        + '<p class="texte">Toutes les entreprises clientes, directes ou rattachées à un revendeur. '
          "Ouvrez une entreprise pour enregistrer un paiement et émettre sa licence.</p>"
        + tableau([("Client", ""), ("Identifiant de l'entreprise", ""), ("Revendeur", ""), ("Échéance", ""),
                   ("Postes", "nombre"), ("Paiements", "")], lignes)
    )
    return page("Entreprises", "entreprises", contenu)


def fiche():
    paiements = [
        [("02/11/2026", "court"), ("Virement", "court"), ("VIR-EXEMPLE-0412", "court"), ("540&#8239;000 FCFA", "nombre"),
         ("3", "nombre"), ("Annuelle", "court"), ("Éditeur exemple", ""),
         ('<a class="bouton bouton-principal bouton-petit" href="interieur-emission.html">Tirer le code</a>', "court")],
        [("05/01/2026", "court"), ("Chèque", "court"), ("-", "court"), ("480&#8239;000 FCFA", "nombre"),
         ("3", "nombre"), ("Annuelle", "court"), (REVENDEUR, ""), ('<a href="#">Voir la licence</a>', "court")],
    ]
    licences = [
        [("08/01/2026", "court"), ("01/01/2026", "court"), ("31/12/2026", "court"), ("3", "nombre"), ("Annuelle", "court"),
         ("Tous les modules", ""), (REVENDEUR, ""), ('<a href="#">Texte de la licence</a>', "court")],
    ]
    contenu = (
        titre_page(E1[0], '<a class="bouton" href="interieur-entreprises.html">Retour aux entreprises</a>'
                   '<a class="bouton bouton-principal" href="#">Enregistrer un paiement</a>',
                   surtitre="Entreprise", identifiant=E1[1])
        + chiffres(("31/12/2026", "échéance de la dernière licence", False, True), ("3", "postes", False, False),
                   ("1", "paiement à utiliser", True, False))
        + '<div class="carte"><div class="carte-interieur"><dl class="grille-fiche">'
          f'<dt>Identifiant de l\'entreprise</dt><dd><span class="identifiant">{E1[1]}</span></dd>'
          f"<dt>Revendeur</dt><dd>{REVENDEUR}</dd>"
          "<dt>Dernière licence</dt><dd>Jusqu'au 31/12/2026, 3 postes</dd></dl></div></div>"
        + "<h2>Paiements</h2>"
        + tableau([("Reçu le", ""), ("Mode", ""), ("Référence", ""), ("Montant", "nombre"), ("Postes", "nombre"),
                   ("Formule", ""), ("Saisi par", ""), ("Licence", "")], paiements)
        + "<h2>Licences émises</h2>"
        + tableau([("Émise le", ""), ("Début", ""), ("Échéance", ""), ("Postes", "nombre"), ("Formule", ""),
                   ("Modules", ""), ("Émetteur", ""), ("", "")], licences)
    )
    return page(E1[0], "entreprises", contenu)


def emission():
    contenu = (
        titre_page("Tirer le code d'activation", '<a class="bouton" href="interieur-fiche.html">Retour à l\'entreprise</a>',
                   surtitre=E1[0])
        + '<div class="carte"><div class="carte-interieur"><dl class="grille-fiche">'
          f"<dt>Client</dt><dd>{E1[0]}</dd>"
          f'<dt>Identifiant de l\'entreprise</dt><dd><span class="identifiant">{E1[1]}</span></dd>'
          "<dt>Paiement</dt><dd>540&#8239;000 FCFA, Virement, reçu le 02/11/2026 (VIR-EXEMPLE-0412)</dd>"
          "<dt>Postes</dt><dd>3</dd><dt>Formule</dt><dd>Annuelle</dd></dl></div></div>"
        + "<h2>Code d'activation</h2>"
        + '<div class="carte"><div class="carte-interieur"><form class="formulaire" method="post" action="interieur-code.html">'
          '<p class="aide">Échéance si le code est activé aujourd\'hui : 31/12/2027. Elle est calculée le jour de '
          "l'activation : l'échéance de la dernière licence si elle court encore, sinon la veille, plus un mois ou un an "
          "selon la formule. Dernier jour couvert, sans délai de grâce.</p>"
          f'<fieldset><legend class="legende">Modules</legend><div class="cases">{cases(MODULES)}</div>'
          '<p class="aide">Repris de la dernière licence de l\'entreprise ; tous les modules pour une première licence.</p></fieldset>'
          '<p class="aide">Le code s\'affiche une seule fois, à remettre au client. Quand il le saisit dans Kadenz, '
          "le portail émet la licence, signée par sa clé, et la renvoie au programme.</p>"
          '<div class="actions"><button class="bouton bouton-principal" type="submit">Tirer le code</button>'
          '<a class="bouton" href="interieur-fiche.html">Annuler</a></div></form></div></div>'
    )
    return page("Tirer le code d'activation", "entreprises", contenu)


def code():
    contenu = (
        titre_page("Code d'activation", '<a class="bouton" href="interieur-fiche.html">Retour à l\'entreprise</a>',
                   surtitre=E1[0])
        + '<div class="code-une-fois"><span class="etat">Affiché une seule fois</span>'
          '<label for="code">Code à remettre au client</label>'
          f'<input id="code" class="code-activation" type="text" readonly value="{CODE}" spellcheck="false">'
          '<p class="aide">Notez ou copiez ce code maintenant : il ne s\'affichera plus. Le client le saisit dans Kadenz '
          "(assistant de KadenzAdmin, ou Paramètres, rubrique « Abonnement », sur un poste Admin) ; la licence est émise "
          "à ce moment-là, avec une échéance calculée le jour de l'activation.</p></div>"
        + '<div class="carte"><div class="carte-interieur"><dl class="grille-fiche">'
          '<dt>Code</dt><dd><span class="identifiant identifiant-court">…8LSAE</span></dd>'
          "<dt>Gamme</dt><dd>Entreprise (serveur et postes)</dd><dt>État</dt><dd>à activer</dd>"
          f'<dt>Identifiant de l\'entreprise</dt><dd><span class="identifiant">{E1[1]}</span></dd>'
          "<dt>Postes</dt><dd>3</dd><dt>Formule</dt><dd>Annuelle</dd><dt>Modules</dt><dd>Tous les modules</dd>"
          "<dt>Tiré le</dt><dd>19/11/2026 10:42:07 par Éditeur exemple</dd></dl></div></div>"
    )
    return page("Code d'activation", "entreprises", contenu)


def journal():
    actions = [
        ("19/11/2026 10:42:07", "Éditeur exemple", "Code tiré", "3f6c2a1e...", "3 postes, annuelle, tous les modules"),
        ("19/11/2026 10:39:51", "Éditeur exemple", "Paiement enregistré", "3f6c2a1e...", "540 000 FCFA, virement, VIR-EXEMPLE-0412"),
        ("19/11/2026 10:38:12", "Éditeur exemple", "Connexion", "-", "adresse 198.51.100.7 (exemple)"),
        ("18/11/2026 16:05:44", REVENDEUR, "Licence émise", "c2f7a5d8...", "2 postes, jusqu'au 30/06/2027"),
        ("18/11/2026 15:58:30", "-", "Connexion refusée", "-", "identifiant exemple, adresse 203.0.113.9 (exemple)"),
    ]
    lignes = [
        [(h, "court"), (c, ""), (a, "court"),
         (f'<span class="identifiant identifiant-court">{cible}</span>' if cible != "-" else "-", "court"), (d, "details")]
        for h, c, a, cible, d in actions
    ]
    contenu = (
        titre_page("Journal")
        + '<p class="texte">Chaque action du portail, de la plus récente à la plus ancienne (500 au plus). '
          "Les heures sont en temps universel (UTC).</p>"
        + '<div class="carte"><div class="carte-interieur"><div class="chaine">'
          '<span class="chaine-etat">Chaîne intacte</span>'
          '<p class="chaine-detail">128 entrées vérifiées. Sceau : '
          '<span class="identifiant">5b1e9c07d4a2f86e3c0b7a19d58e2f4c6a903b7e1d25c8f04a6e9b3d7c1f2a58</span></p>'
          '<p class="aide">Le sceau est l\'empreinte de la dernière entrée vérifiée. Noté hors du serveur, il permet de '
          "constater plus tard que rien n'a changé avant lui.</p></div></div></div>"
        + "<h2>Actions</h2>"
        + tableau([("Heure (UTC)", ""), ("Compte", ""), ("Action", ""), ("Cible", ""), ("Détails", "")], lignes)
    )
    return page("Journal", "journal", contenu)


def revendeurs():
    comptes = [
        ("Éditeur exemple", "editeur", "Éditeur", True, False),
        (REVENDEUR, "revendeur.nord", "Revendeur", True, True),
        ("Revendeur exemple Sud", "revendeur.sud", "Revendeur", False, True),
    ]
    lignes = []
    for nom, ident, role, actif, action in comptes:
        bouton = ""
        if action:
            bouton = (f'<form class="en-ligne" method="post" action="#"><button class="bouton bouton-petit" type="submit">'
                      f'{"Désactiver" if actif else "Réactiver"}</button></form>')
        lignes.append([(nom, ""), (f'<span class="identifiant">{ident}</span>', ""), (role, "court"),
                       (ETAT_OK.format("Actif") if actif else ETAT_NEUTRE.format("Désactivé"), "court"), (bouton, "court")])
    contenu = (
        titre_page("Revendeurs")
        + '<p class="texte">Un revendeur ne voit que les entreprises qui lui sont rattachées. Son nom est repris comme '
          "émetteur dans les licences qu'il émet. Un compte désactivé ne peut plus se connecter et ses sessions sont fermées.</p>"
        + tableau([("Nom", ""), ("Identifiant", ""), ("Rôle", ""), ("État", ""), ("", "")], lignes)
        + "<h2>Nouveau revendeur</h2>"
        + '<div class="carte"><div class="carte-interieur"><form class="formulaire" method="post" action="#">'
          '<div class="rangee"><div class="champ"><label for="identifiant">Identifiant de connexion</label>'
          '<input id="identifiant" type="text" name="identifiant" required autocomplete="off" spellcheck="false"></div>'
          '<div class="champ"><label for="nom">Nom du revendeur</label>'
          '<input id="nom" type="text" name="nom" required autocomplete="organization"></div></div>'
          '<div class="champ"><label for="mot_de_passe">Mot de passe provisoire</label>'
          '<input id="mot_de_passe" type="password" name="mot_de_passe" required minlength="15" autocomplete="new-password">'
          '<p class="aide">15 caractères au moins, pas un mot de passe courant. Transmettez-le au revendeur par un canal sûr ; '
          "il pourra le changer après sa connexion.</p></div>"
          '<div class="actions"><button class="bouton bouton-principal" type="submit">Créer le revendeur</button></div>'
          "</form></div></div>"
    )
    return page("Revendeurs", "comptes", contenu)


PAGES = {
    "interieur-entreprises.html": entreprises,
    "interieur-fiche.html": fiche,
    "interieur-emission.html": emission,
    "interieur-code.html": code,
    "interieur-journal.html": journal,
    "interieur-revendeurs.html": revendeurs,
}

CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"


def capturer(page, png, largeur=1440, hauteur=1100):
    """Capture par Chrome local, sans réseau (kadenz-e6, reprise du 2026-11-19)."""
    import subprocess
    (ICI / "captures").mkdir(exist_ok=True)
    subprocess.run([CHROME, "--headless=new", "--disable-gpu", "--hide-scrollbars",
                    "--allow-file-access-from-files", f"--window-size={largeur},{hauteur}",
                    f"--screenshot={ICI / 'captures' / png}", "--virtual-time-budget=3000",
                    (ICI / page).as_uri()], check=True, capture_output=True)


if __name__ == "__main__":
    for nom, fabrique in PAGES.items():
        (ICI / nom).write_text(fabrique(), encoding="utf-8", newline="\n")
        capturer(nom, nom.replace(".html", ".png"))
        print("écrit", nom)
