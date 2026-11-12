# Marque Kadenz

Identité de l'éditeur et du logiciel. Elle n'apparaît jamais sur les documents produits pour les clients (cahier des charges, section 1) : chaque installation garde l'identité de l'entreprise utilisatrice.

Statut : **« Quatre pièces »**, validée par l'utilisateur le 2026-11-11 et intégrée à l'application ; couleurs Indigo et Nuit de l'ancienne identité, motif plein et trois temps (blanc, blanc, Ambre) repris le 2026-11-12 à sa demande (la version Bleu vif reste dans l'historique git). Elle remplace le K cadencé du 2026-11-09, conservé dans `archives/k-cadence-2026-11-09/`.

## Le logo

- **Idée** : quatre formes simples (carré, quart de disque, triangle, disque) s'emboîtent sur une grille de deux par deux, comme les modules que la licence assemble ; le disque marque le temps fort de la cadence.
- **Une seule couleur** : symbole Indigo sur fond clair, symbole tout blanc sur l'Indigo ou la Nuit. Le mot « kadenz » est en Nuit sur fond clair, en blanc sur fond sombre.
- **Versions** : logo horizontal (principal), logo empilé, symbole seul, mot seul, icône (symbole sur tuile).
- **Mot** : Space Grotesk 600 (choix de l'utilisateur du 2026-11-12), interlettrage resserré de 1 %, hampes à la hauteur de l'ancien mot en Inter ; vectorisé (aucune police n'est nécessaire pour afficher le logo).

## Dans l'application

- `crates/app/ui/composants.slint` : jetons de couleur `accent` (Indigo), `accent-survol`, `nuit`, `ambre` ; composants `Symbole` (icône sur tuile), `MotifMarque` (motif) et `TroisTemps` (les trois traits).
- `crates/app/ui/icones/` : `kadenz-icone.svg`, `kadenz-logo-horizontal.svg`, `kadenz-logo-horizontal-negatif.svg`, `kadenz-32.png`, `kadenz-64.png`, `kadenz-256.png`, `motif-quatre-pieces.svg`.
- `crates/app/kadenz.ico` : icône de `kadenz.exe` (16 à 256 px), intégrée par `build.rs` via `winresource`.
- Bandeaux de marque (connexion, présentation, assistant, réglages) : aplat Nuit, motif Quatre pièces serré (pas de 40 px) sur tout le panneau, logo en négatif ; trois temps au-dessus de « Gestion commerciale » (connexion, présentation).
- Installateur : `installateur/theme/logo.png` et `logoside.png` sont produits par `installateur/theme/generer.py` à partir de `crates/app/kadenz.ico`.

## Construction

- Canevas de 256 ; marge de 16 ; jeu de 16 entre les cases ; côté d'une case : 104.
- Débord optique : quart de disque agrandi de 1,5 %, disque de 3 %, pour qu'ils paraissent de la même taille que le carré.
- Petite coupe (32 px et moins) : marge de 10, jeu de 26, sans débord, pour que les jeux restent ouverts.
- Icône : petite coupe à 62 % de la tuile, coins de la tuile arrondis à 16 sur 256.

## Fichiers

| Fichier | Usage |
|---|---|
| `kadenz-logo-horizontal.svg` | Logo principal, sur fond clair |
| `kadenz-logo-horizontal-negatif.svg` | Logo principal, sur l'Indigo, la Nuit ou un fond sombre |
| `kadenz-logo-horizontal-mono.svg` | Noir, tampon et impression en une couleur |
| `kadenz-logo-empile.svg`, `-negatif.svg` | Formats carrés et verticaux |
| `kadenz-symbole.svg`, `-negatif.svg`, `-noir.svg`, `-blanc.svg` | Symbole seul, sans tuile |
| `kadenz-symbole-petit.svg`, `-petit-negatif.svg` | Petite coupe : 32 px et moins |
| `kadenz-icone.svg` | Icône de l'application (petite coupe sur tuile) |
| `kadenz-mot.svg`, `-negatif.svg` | Mot seul, quand le symbole est déjà présent à côté |
| `kadenz-module-*.svg` | Marques des familles de modules (proposition) |
| `motif-quatre-pieces.svg` | Motif ton sur ton des bandeaux de marque (240 x 800) |
| `png/kadenz-16.png` à `kadenz-256.png`, `png/kadenz.ico` | Icône en images (16 à 256 px) |
| `png/installateur-logo.png` | Icône en 64 x 64 px |
| `png/kadenz-logo-horizontal-1200.png`, `-negatif-1200.png` | Logo en image pour les usages sans SVG |
| `web/` | Favicon (.ico, .svg, PNG 16 à 48), icônes 180, 192 et 512, icône masquable, manifeste et extrait `<head>` pour le futur site |
| `planches/01-primary-logo.html` à `09-brand-mockups.html` | Neuf planches de l'identité : logo principal, symbole, ton, couleurs, motif, typographie, signalétique, papeterie, mises en situation |
| `planches/images/`, `planches/kadenz-identite.pdf` | Les neuf planches en PNG (2400 x 1520) et en PDF (une planche par page) |
| `planches/generer_planches.py` | Regénère planches, PNG et PDF : `python docs/marque/planches/generer_planches.py .` (Chrome requis) |
| `generer.py` | Regénère tous les SVG : `python docs/marque/generer.py .` (fontTools requis) |

Les PNG, le `.ico` et les icônes web ont été rendus avec les scripts du skill logo-design (`render_png.py`, `export_variants.py`, rendu par Chrome), sans Pillow.

## Zone de protection et tailles minimales

Autour du logo, laisser libre au moins **la moitié du côté d'une case** du symbole (52 sur un symbole de 256). La zone suit la taille du logo.

| Version | Écran | Impression |
|---|---|---|
| Logo horizontal | 120 px de large (à 96 px, les jeux du symbole se ferment) | 30 mm de large |
| Logo empilé | 64 px de large | 18 mm de large |
| Symbole | 16 px, avec la petite coupe en dessous de 33 px | 6 mm |

## Couleurs

| Nom | HEX | RVB | CMJN (approché) | Contraste sur blanc | Rôle | Jeton Slint |
|---|---|---|---|---|---|---|
| Indigo | `#2D3A8C` | 45 58 140 | 68 59 0 45 | 10,0:1 | Couleur du logo, boutons principaux, accent | `accent` |
| Nuit | `#1A2259` | 26 34 89 | 71 62 0 65 | 14,8:1 | Bandeaux de marque, mot du logo sur fond clair, bulles et alertes sombres | `nuit` |
| Pièces du motif | `#232C6B` | 35 44 107 | 67 59 0 58 | 12,8:1 | Pièces du motif sur la Nuit | (motif-quatre-pieces.svg) |
| Ambre | `#E3A23B` | 227 162 59 | 0 29 74 11 | 2,2:1 | Troisième temps, touche sur fond sombre ; jamais en texte sur blanc | `ambre` |
| Indigo pâle | `#E8EAF6` | 232 234 246 | 6 5 0 4 | 1,2:1 | Survol | `accent-survol` |
| Encre | `#1F2933` | 31 41 51 | 39 20 0 80 | 14,8:1 | Texte | `texte` |
| Ardoise | `#52606D` | 82 96 109 | 25 12 0 57 | 6,5:1 | Texte secondaire | `texte-secondaire` |
| Trait | `#D9DCE3` | 217 220 227 | 4 3 0 11 | 1,4:1 | Bordures | `bordure` |
| Brume | `#F4F5F9` | 244 245 249 | 2 2 0 2 | 1,1:1 | Fond latéral | `fond-lateral` |
| Succès | `#067647` | 6 118 71 | 95 0 40 54 | 5,7:1 | États réussis | `succes` |
| Erreur | `#B42318` | 180 35 24 | 0 81 87 29 | 6,6:1 | Erreurs | `erreur` |

Les valeurs CMJN sont une conversion directe, à faire vérifier par l'imprimeur sur épreuve. Aucune référence Pantone n'est donnée : elle se choisit sur nuancier physique.

Sur les bandeaux Nuit, les textes secondaires sont en `#C5CAE0` (9,1:1), la version en `#9AA3C7` (6,0:1) ; l'Ambre y atteint 6,7:1.

**Associations permises** : logo couleur sur blanc ; logo blanc sur l'Indigo ; logo blanc sur Nuit ; logo noir sur blanc. Sur une photo, utiliser la version blanche ou noire sur une zone calme, ou l'icône sur sa tuile.

## Modules (proposition)

La pièce du module est pleine, les trois autres sont au trait : Ventes (carré : devis, proforma, factures), Commandes (quart de disque : bons de commande), Livraisons (triangle : bons de livraison), Paiements (disque : règlements, relances). Cette répartition est une proposition à valider ; elle ne change rien au découpage réel des licences.

## Typographie

- **Inter** (400, 500, 600, 700) : texte, interface, montants en chiffres tabulaires. SIL Open Font License 1.1.
- **Space Grotesk** (Bold 700) : titres de l'application et des documents PDF (`crates/pdf`, choix de l'utilisateur du 2026-11-12), choisie par l'utilisateur le 2026-11-12 après comparaison avec Manrope, IBM Plex Sans, Kobe et Platform. SIL Open Font License 1.1, sans nom réservé ; instance statique réduite au latin, chiffres tabulaires conservés.
- Point ouvert : le mot du logo utilise Space Grotesk sans retouche ; un dessin de lettres propre à Kadenz reste à l'étude.

## À ne pas faire

- Déformer, étirer, pencher ou faire tourner le logo.
- Changer l'ordre ou l'orientation des pièces, ou les espacer autrement.
- Ajouter une deuxième couleur au symbole, une ombre, un contour, un dégradé ou un effet.
- Poser le logo couleur sur un fond chargé.
- Recomposer le mot en le tapant au clavier : utiliser les fichiers.

## Limites connues

- Aucune recherche de marque n'a été faite ; elle est à confier à un professionnel avant tout dépôt.
- Le bleu est la teinte la plus courante des logiciels : Kadenz se distingue surtout par la forme du symbole.
- Les propositions qui ont mené à ce choix (pistes A à F, palettes) sont restées en fichiers locaux ; le journal `.claude/journal.md` en garde l'historique.
