# Marque Kadenz

Identité de l'éditeur et du logiciel. Elle n'apparaît jamais sur les documents produits pour les clients (cahier des charges, section 1) : chaque installation garde l'identité de l'entreprise utilisatrice.

Statut : **proposition à valider**.

## Planches de présentation

Les 9 planches (primary logo, logomark, tone & mood, color system, brand pattern, typography, signage, stationery, brand mockups) sont dans `planches/` et s'ouvrent sans compte ni connexion :

- `planches/index.html` : les 9 planches sur une page, à ouvrir dans un navigateur ;
- `planches/kadenz-identite.pdf` : une planche par page ;
- `planches/images/` : une image PNG par planche (2400 x 1520 px) ;
- `planches/01-primary-logo.html` à `09-brand-mockups.html` : chaque planche seule ;
- `planches/polices/` : Inter et Manrope en WOFF2, avec leur licence OFL.

`canevas/` conserve les sources d'origine du canevas de design (fichiers `.dc.html` et `canvas.json`), pour le réimporter dans un éditeur de canevas au besoin.

## Symbole

Le K cadencé : un fût, un bras, et une jambe découpée en trois temps égaux. Grille de 48 unités, fût de 9 unités, pente des bras 1,25. Sous 48 px, la version réduite (jambe pleine) remplace le symbole découpé.

## Fichiers

| Fichier | Usage |
|---|---|
| `kadenz-logo-horizontal.svg` | Logo principal, sur fond clair |
| `kadenz-logo-horizontal-negatif.svg` | Logo principal, sur Indigo ou Nuit |
| `kadenz-logo-horizontal-mono.svg` | Noir, tampon et impression monochrome |
| `kadenz-logo-empile.svg`, `kadenz-logo-empile-negatif.svg` | Formats carrés et verticaux |
| `kadenz-symbole.svg`, `-blanc.svg`, `-noir.svg` | Symbole seul, sans tuile |
| `kadenz-icone.svg` | Icône d'application, 48 px et plus |
| `kadenz-icone-petite.svg` | Icône réduite, 16 à 32 px |
| `generer.py` | Regénère tous les SVG (fontTools et Manrope requis) |

Zone de protection : un quart de la hauteur de la tuile autour du logo. Taille minimale : 120 px ou 30 mm de large en horizontal, 64 px ou 18 mm en empilé.

## Couleurs

| Nom | HEX | Rôle | Jeton Slint |
|---|---|---|---|
| Indigo | `#2D3A8C` | Couleur de marque | `accent` |
| Nuit | `#1A2259` | Fonds sombres | (ajout proposé) |
| Ambre | `#E3A23B` | Touche d'accent, jamais en texte sur blanc | (ajout proposé) |
| Ambre foncé | `#A8650F` | Ambre en texte sur blanc (4,6:1) | (ajout proposé) |
| Encre | `#1F2933` | Texte | `texte` |
| Ardoise | `#52606D` | Texte secondaire | `texte-secondaire` |
| Trait | `#D9DCE3` | Bordures | `bordure` |
| Lavande | `#E8EAF6` | Survol | `accent-survol` |
| Brume | `#F4F5F9` | Fond latéral | `fond-lateral` |
| Succès | `#067647` | États réussis | `succes` |
| Erreur | `#B42318` | Erreurs | `erreur` |

## Typographie

- **Manrope** (500, 700, 800) : titres et mot-symbole. SIL Open Font License 1.1.
- **Inter** (400, 500, 600) : texte, interface, montants en chiffres tabulaires. SIL Open Font License 1.1.

Les deux familles peuvent être embarquées dans l'application Slint (`import "…ttf";`).
