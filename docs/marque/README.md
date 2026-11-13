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
| `motif-tournant.svg`, `motif-fondu.svg` | Motifs Tournant et Fondu (800 x 400), voir « Motifs » |
| `png/kadenz-16.png` à `kadenz-256.png`, `png/kadenz.ico` | Icône en images (16 à 256 px) |
| `png/installateur-logo.png` | Icône en 64 x 64 px |
| `png/kadenz-logo-horizontal-1200.png`, `-negatif-1200.png` | Logo en image pour les usages sans SVG |
| `web/` | Favicon (.ico, .svg, PNG 16 à 48), icônes 180, 192 et 512, icône masquable, manifeste et extrait `<head>` pour le futur site |
| `planches/01-primary-logo.html` à `09-brand-mockups.html` | Neuf planches de l'identité : logo principal, symbole, ton, couleurs, motif, typographie, signalétique, papeterie, mises en situation |
| `planches/images/`, `planches/kadenz-identite.pdf` | Les neuf planches en PNG (2400 x 1520) et en PDF (une planche par page) |
| `planches/generer_planches.py` | Regénère planches, PNG et PDF : `python docs/marque/planches/generer_planches.py .` (Chrome requis) |
| `planches/scenes_signaletique.py` | Scènes de signalétique de la planche 07, utilisées par `generer_planches.py` |
| `planches/scenes_papeterie.py` | Scènes de papeterie de la planche 08, utilisées par `generer_planches.py` |
| `planches/scenes_mises_en_situation.py` | Mises en situation de la planche 09, utilisées par `generer_planches.py` |
| `planches/apercu-papeterie.html`, `images/apercu-papeterie.png` | Aperçu complet de la papeterie en grand format : `python docs/marque/planches/generer_apercu_papeterie.py .` |
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

## Motifs

Trois motifs, tous tirés des quatre pièces et toujours ton sur ton (pièces `#232C6B` sur la Nuit, ou `#E6E9F0` sur la Brume). Tournant et Fondu ont été retenus par l'utilisateur le 2026-11-13 parmi neuf propositions (`propositions-2026-11-13-motifs/`).

| Motif | Construction | Usages |
|---|---|---|
| Quatre pièces | Les quatre pièces en rangées, décalées d'un temps à chaque ligne | Bandeaux de l'application, couvertures, kakémono ; version claire pour les fonds de page |
| Tournant | Le quart de disque seul, à la taille de la case, tourné d'un quart de tour d'une case à l'autre ; rotations figées par une graine | Grands fonds, écrans d'accueil, couvertures |
| Fondu | Le motif Quatre pièces dont les pièces rapetissent vers la droite (ou vers le bas) jusqu'à disparaître ; la zone calme naît du motif | Bandeaux avec logo ou titre, connexion, présentation |

Règles : le motif reste ton sur ton, jamais à plusieurs couleurs ; jamais derrière un texte courant, le logo se pose sur une zone calme ; chaque motif garde sa construction (ordre, pas, rotations), on agrandit, on ne déforme pas. Dans l'application, seul Quatre pièces est branché aujourd'hui (composant `MotifMarque`).

## Signalétique

Huit supports retenus par l'utilisateur le 2026-11-13 parmi neuf propositions (`propositions-2026-11-13-signaletique/` ; les postes de démonstration par module n'ont pas été retenus). Planche 07, avec une silhouette de 1,75 m pour l'échelle. Les dimensions sont indicatives, à confirmer avec le fabricant.

| Support | Composition | Dimensions indicatives |
|---|---|---|
| Façade, enseigne caisson | Caisson Nuit au motif Fondu, logo blanc rétroéclairé sur la zone calme ; symbole sur la porte vitrée | Selon la façade |
| Drapeau sur potence | Double face, perpendiculaire au mur : symbole blanc sur Indigo, chant Nuit | Environ 600 x 600 mm |
| Lettres boîtier d'accueil | Logo couleur en lettres découpées sur le mur, sans caisson, éclairage indirect | Selon le mur |
| Plaque d'entrée | Plaque Nuit : logo négatif, trois temps (blanc, blanc, Ambre), « Gestion commerciale » | 300 x 200 mm, entretoises inox |
| Vitrophanie de porte | Bande dépolie entre 1,1 et 1,6 m (repère de sécurité des portes vitrées), motif Fondu blanc ton sur ton, logo couleur sur la zone calme | Hauteur de la bande : 0,5 m |
| Totem extérieur | Nuit : motif Tournant en tête, logo empilé blanc, trois temps | Environ 2,2 m de haut, socle lesté |
| Kakémono | Nuit : motif Fondu en tête, logo empilé, quatre familles de documents, pied Ambre | 850 x 2000 mm, enrouleur |
| Stand de salon | Mur d'image Tournant, logo sur la zone calme ; comptoir Nuit au symbole blanc | Environ 3 x 2,5 m |

## Papeterie

Papeterie de l'éditeur seulement : les documents produits par le logiciel gardent l'identité du client. Retenue par l'utilisateur le 2026-11-13 après deux tours de propositions (`propositions-2026-11-13-papeterie/`). Planche 08 ; les coordonnées restent à remplir.

| Pièce | Composition | Format |
|---|---|---|
| Carte de visite | Recto Nuit : motif Tournant à gauche, logo blanc sur la partie calme. Verso blanc : symbole, nom, fonction, trois temps, coordonnées ; Fondu très clair à droite, qui s'efface avant le texte | 85 x 55 mm |
| Papier à en-tête | Bandeau Fondu très clair qui s'efface vers le logo, coordonnées en pied, aucune barre latérale | A4 |
| Enveloppe | Dehors : Fondu clair sous l'adresse de retour, zone du destinataire libre. Dedans : tout l'intérieur imprimé Nuit au motif Tournant | DL 220 x 110 mm |
| Chemise à rabats | Dehors comme le carnet, sans l'élastique (Tournant sur les trois quarts, bande Nuit avec le logo) ; dedans Tournant, rabat Indigo ; sans les trois temps | A4 |
| Badge et tour de cou | Haut Nuit au Fondu qui descend vers le logo, nom sur blanc, trois temps ; tour de cou Indigo | 90 x 120 mm |
| Signatures de courriel | Trois variantes : A bandeau Fondu avec le logo, B bloc Nuit au Fondu avec le texte en blanc, C fine bande Tournant | Largeur 300 à 340 px |
| Tampons par degré | Une forme par degré, une seule couleur d'encre (système 1, retenu le 2026-11-13) : 1 entreprise, cachet rond, un seul ; 2 direction, rectangle Fondu au nom de la direction, un par direction ; 3 responsable, rectangle nominatif avec cadre Tournant et ligne de signature, un par responsable ; 4 service, dateur « Reçu le … par … », un par service | 40 mm de diamètre ; 50 x 20 mm ; 55 x 23 mm ; 47 x 14 mm environ |
| Carnet et autocollants | Carnet Nuit (Tournant, bande avec le logo, élastique) ; autocollants du symbole et de l'icône | A5, 50 mm |

## Mises en situation

Huit mises en situation retenues par l'utilisateur le 2026-11-13 parmi neuf propositions (`propositions-2026-11-13-mises-en-situation/` ; la page d'accueil du futur site n'a pas été retenue). Planche 09. Les libellés viennent des écrans et de l'installateur réels ; ce sont des maquettes, l'application et l'installateur n'ont pas été modifiés pour autant.

| Support | Composition |
|---|---|
| Écran de connexion | Panneau Nuit au motif Fondu, logo négatif, trois temps, « Gestion commerciale » ; formulaire sur fond clair |
| Assistant de premier lancement | Panneau Nuit, Fondu en pied ; les cinq étapes réelles (Licence, Entreprise, Administrateur, Taux et paiements, Récapitulatif), point Ambre sur l'étape en cours |
| Installateur | Panneau Nuit, motif Tournant en tête, logo empilé négatif, trois temps ; textes de l'installateur |
| Écran À propos | Bandeau Nuit au motif Fondu, logo négatif ; « Gestion commerciale », version, trois temps, « Logiciel propriétaire, tous droits réservés. », « Revoir la présentation » |
| Aperçu social du dépôt GitHub | 1280 x 640 px : Nuit, motif Tournant à gauche, logo négatif et description du dépôt |
| Bureau Windows | Fond d'écran Nuit au motif Tournant, icône Kadenz sur le bureau et dans la barre des tâches |
| Onglet et écran d'accueil | Favicon dans l'onglet du navigateur ; téléphone au fond Tournant, icône sur l'écran d'accueil |
| Profil de réseau social | Bannière Nuit au motif Tournant, avatar : symbole blanc sur Indigo |

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
