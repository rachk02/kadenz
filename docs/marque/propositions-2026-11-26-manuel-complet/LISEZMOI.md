# Manuel d'utilisation complet de Kadenz, mise en page finale, version 2 (2026-11-26)

Lot 9 : recomposition du PDF à partir des 15 chapitres, de l'index et des captures de `docs/manuel/` (lus, jamais modifiés). Elle remplace la version du lot 8 (212 pages, captures pleine largeur), refusée.

## Sorties

| Fichier | Contenu |
| --- | --- |
| `manuel-complet.pdf` | 90 pages A4 (210 x 297 mm), sans grain, environ 17 Mo |
| `planche-manuel-complet.png` | planche de contrôle faite de pages réelles du PDF (couverture, sommaire, ouverture de chaque chapitre, première page, page courante, index, quatrième) |
| `genere_manuel_complet.py` | le script qui produit tout |
| `recadrer_captures.py` | recadre les captures en copies de travail (`_captures/`, recréé au besoin) |
| `rasteriser_pdf.ps1` | transforme des pages du PDF en PNG (Windows seulement), pour la planche et les contrôles |

## Regénérer

```
py -3.14 -B genere_manuel_complet.py --planche        # PDF et planche (environ 3 minutes)
py -3.14 -B genere_manuel_complet.py --apercu 12,13   # PNG des pages 12 et 13 dans _apercus/ (à supprimer ensuite)
```

Chrome est requis. Pillow et numpy ne servent qu'au recadrage des captures (`py -3.13`, lancé par le script si `_captures/` manque). Les polices ABC Diatype sont lues dans `crates/app/ui/polices/`, jamais copiées.

## Méthode de pagination

Chrome pagine lui-même : pages nommées (`@page`), captures flottantes, orphelines et veuves de 2 lignes, titres collés au texte suivant, en-tête de tableau répété à chaque page, pied de page en boîtes de marge avec `counter(page)`.

Les numéros de page du sommaire et de l'index sont lus dans le PDF : chaque titre de section porte un lien invisible, dont la page est lue dans les annotations du PDF (analyse des objets en Python pur). Une première passe donne les pages, une deuxième écrit les vrais numéros et vérifie qu'ils ne bougent pas, la dernière passe est sans lien. Une capture que Chrome repousse après tout le texte de son chapitre (flottants qui s'accumulent) reçoit un saut de page avant son paragraphe, détecté de la même façon.

## Mise en page

- Pages 1 et 2 : couverture et sommaire. Puis, pour chaque chapitre, une page d'ouverture (famille des pièces du 2026-11-18, inchangée) et les pages de texte ; index (ouverture et 2 pages) ; quatrième de couverture.
- Texte : une colonne, corps 9,8 pt, interligne 1,38, marges de 19 mm, titres compacts, tableaux resserrés. Le pied de la v2 (titre du manuel, version 0.3.1, numéro) est en boîtes de marge.
- Captures : copies recadrées (zone blanche du bas retirée, barre de titre conservée). Chaque capture flotte à droite ou à gauche, en alternance, au niveau du paragraphe qui la précède dans le Markdown, et le texte l'entoure (5,5 mm entre image et texte). Largeur de 52 % du texte, 60 % pour une capture dense (tableau de bord, grille de droits, journal), cadre neutre de 1 px sur les quatre côtés, angles à 2 px, légende discrète tirée du texte alternatif.
- Deux captures séparées par peu de texte (ou sans texte) forment une paire côte à côte, 48 % chacune.
- Astuce et Attention : aplat gris Brume sans bordure ni barre, intertitre en gras.
- Aucun emoji, aucun tiret cadratin, aucune italique.

## Ouvertures

Inchangées depuis le lot 8 : 01 manuel 01 (carré) ; 02 affiche A3 ; 03 affiche A5 ; 04 affiche A2 ; 05 page de chapitre du manuel 02 ; 06 manuel 02 (quart) ; 07 affiche A4 ; 08 diapositive D2 ; 09 affiche A1 ; 10 motif Tournant en tête ; 11 manuel (disque) ; 12 diapositive D1 ; 13 manuel 03 (triangle) ; 14 page de chapitre (disque) ; 15 motif Tournant en pied ; Index. Les libellés des listes sont les titres de section du chapitre.

## Écarts connus

- Couverture et ouverture 01 : dégradés grainés d'origine, d'après le dernier choix de l'utilisateur. Les variantes à motifs restent dans `propositions-2026-11-25-manuel-v2/`.
- Format A4 exact, sans fond perdu. Pages dessinées de la v2 réduites de 2 % pour entrer dans l'A4.
- Index : la phrase d'introduction de `index.md` est remplacée à la composition par « Les numéros renvoient aux pages du manuel. ». Les numéros de page sont ceux du début de chaque section citée (doublons fusionnés).
- Le sommaire ne liste que les chapitres et l'index, pas les sections.
- Blanc en bas de certaines pages : un tiers des pages de texte se termine avant 80 % de la hauteur, surtout en fin de chapitre et là où une capture repoussée est ramenée avec son texte (saut de page). Plus de captures flottantes que de texte dans les chapitres 3, 4 et 5 : le blanc y est le prix de l'absence de capture seule sur une page.
- Les captures de plusieurs écrans quasi vides gardent leur blanc interne (c'est le contenu de l'écran).

## Défauts relevés dans les textes (non corrigés)

- 73 renvois « chapitre N » et 34 renvois « section N.M » contrôlés : tous existent et pointent vers le bon sujet.
- Index : toutes les sections (hors « À retenir ») sont citées au moins une fois.
- Captures présentes sur disque mais jamais utilisées : `04-documents-vente/21-bordereau-confirmation.png`, `05-stock/19-inventaire-valide.png`.
- `index.md` : phrase d'introduction périmée (voir ci-dessus).
