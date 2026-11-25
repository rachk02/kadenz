# Manuel d'utilisation complet de Kadenz, mise en page finale (2026-11-26)

Lot 8 : composition du PDF à partir des 15 chapitres, de l'index et des captures de `docs/manuel/` (lus, jamais modifiés).

## Sorties

| Fichier | Contenu |
| --- | --- |
| `manuel-complet.pdf` | 212 pages A4 (210 x 297 mm), sans grain, environ 17 Mo |
| `planche-manuel-complet.png` | planche de contrôle de 63 pages (couverture, sommaire, ouverture de chaque chapitre, première page de texte, pages à captures, index, quatrième), avec grain |
| `genere_manuel_complet.py` | le script qui produit tout |

## Regénérer

```
py -3.14 -B genere_manuel_complet.py --planche
py -3.14 -B genere_manuel_complet.py --apercu 12,13      # PNG des pages 12 et 13 dans _apercus/ (à supprimer ensuite)
FIG=85 py -3.14 -B genere_manuel_complet.py              # captures à 85 % de la largeur du texte (198 pages)
```

Chrome est requis, aucune bibliothèque Python. Le script écrit un convertisseur Markdown maison (titres, paragraphes, gras, code en ligne, listes imbriquées, tableaux, images, blocs Attention et Astuce), fait mesurer la hauteur de chaque bloc par Chrome, calcule la pagination lui-même, puis insère les numéros de page réels dans le sommaire et l'index avant d'imprimer. Il affiche aussi les défauts de texte constatés et les débordements éventuels (liste vide attendue). Les polices ABC Diatype (essai) sont lues dans `crates/app/ui/polices/`, jamais copiées.

## Mise en page

- Pages 1 et 2 : couverture et sommaire (numéros de page réels). Puis, pour chaque chapitre, une page d'ouverture dans la famille des pièces du 2026-11-18 et les pages de texte ; index (ouverture et 2 pages) ; quatrième de couverture. Total 212.
- Pages courantes sans en-tête, pied de la v2 (titre du manuel, version 0.3.1, numéro de page). La première page de texte d'un chapitre porte le titre et le numéro.
- Captures : pleine largeur du texte, cadre de 1 px Trait sur les quatre côtés, angles à 2 px, légende discrète tirée du texte alternatif Markdown. Une capture reste avec le texte qui l'introduit et n'est jamais coupée. Les tableaux se coupent entre deux lignes avec l'en-tête répété.
- Astuce et Attention : aplat gris Brume sans bordure ni barre, intertitre en gras.
- Aucun emoji, aucun tiret cadratin, aucune italique.

## Ouvertures

01 manuel 01 (carré) ; 02 affiche A3 ; 03 affiche A5 ; 04 affiche A2 (types de documents de la section 4.1, préfixes D F L A et numéro D2026-11-0001 du chapitre) ; 05 page de chapitre du manuel 02 ; 06 manuel 02 (quart) ; 07 affiche A4 (formats de la section 7.1) ; 08 diapositive D2 ; 09 affiche A1 ; 10 quatrième de couverture (motif Tournant en tête) ; 11 manuel (disque) ; 12 diapositive D1 ; 13 manuel 03 (triangle) ; 14 page de chapitre (disque) ; 15 motif Tournant en pied ; Index page de chapitre. Les libellés des listes sont les titres de section du chapitre.

## Écarts connus

- Couverture et ouverture 01 : dégradés grainés d'origine, d'après le dernier choix de l'utilisateur (« je veux l'ancien »). Les variantes à motifs restent dans `propositions-2026-11-25-manuel-v2/`.
- Format A4 exact, sans fond perdu (la v2 était en 216 x 303 mm avec 3 mm de fond perdu). Pages dessinées de la v2 réduites de 2 % pour entrer dans l'A4.
- Les 16 ouvertures reprennent 12 constructions : certaines reviennent (Nuit à pièce, page Brume à grand chiffre, motif Tournant, champ dégradé).
- Mêmes écarts de charte que la v2 : dégradés grainés, pièces Ambre sur Nuit, grand chiffre.
- Index : la phrase d'introduction de `index.md` annonçait des numéros de section et des pages à venir ; elle est remplacée à la composition par « Les numéros renvoient aux pages du manuel. ». Les numéros de page sont ceux du début de chaque section citée (doublons fusionnés), le numéro de section n'est pas repris.
- Le sommaire ne liste que les chapitres et l'index, pas les sections.
- Pas de pages blanches de recto verso.
- Beaucoup de blanc en bas de page là où une capture de 600 px ne rentre pas : réduire `FIG` le diminue (85 : 198 pages).

## Défauts relevés dans les textes (non corrigés)

- 73 renvois « chapitre N » et 34 renvois « section N.M » contrôlés : tous existent et pointent vers le bon sujet.
- Sections jamais citées dans l'index : 1.2, 1.4, 1.8, 2.4, 3.3, 4.6, 6.4, 14.5, 15.1, 15.5.
- Captures présentes sur disque mais jamais utilisées : `04-documents-vente/21-bordereau-confirmation.png`, `05-stock/19-inventaire-valide.png`.
- `index.md` : phrase d'introduction périmée (voir ci-dessus).
