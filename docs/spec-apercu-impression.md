# Kadenz - Aperçu et impression des documents

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-16 | Proposition de kadenz-22 après la demande de l'utilisateur pendant l'essai S3 (« permettre pour chaque document d'avoir un aperçu, un bouton, et dans l'aperçu d'autres actions comme imprimer, avec la sélection des imprimantes, ou enregistrer »), avec les points techniques de kadenz-a8. Rien n'est codé. |

Statut : **proposé**. Les questions de la section 6 sont à l'utilisateur.

## 1. Constat

- La fiche d'un document (devis, proforma, bon de commande, bordereau, facture, avoir) n'a qu'un bouton PDF : « Aperçu PDF » sur un brouillon, « Exporter en PDF » sinon. Les deux enregistrent le fichier par une boîte de dialogue, puis l'ouvrent dans la visionneuse de Windows (`crates/app/src/factures.rs`, `on_exporter_pdf`).
- Aucun écran ne permet d'imprimer : l'impression passe par la visionneuse de Windows, quand il y en a une.
- Le moteur `hayro`, déjà utilisé par l'aperçu des réglages d'impression (`crates/app/src/pdf.rs`, `apercu_mise_en_page`), sait rendre une page PDF en image.

## 2. Ce qui change

- **Bouton « Aperçu »** sur la fiche de chaque document, brouillon compris, à côté de « Exporter en PDF ». Sur un brouillon, il remplace « Aperçu PDF ».
- **Fenêtre d'aperçu** dans Kadenz : toutes les pages du document, l'une sous l'autre, avec défilement et le numéro de la page ; zoom ajusté à la largeur.
- Actions dans la fenêtre d'aperçu :
  - **Imprimer** : ouvre la fenêtre d'impression de Windows (choix de l'imprimante, nombre de copies, pages, propriétés de l'imprimante), puis imprime ;
  - **Enregistrer en PDF** : reprend l'export existant (boîte de dialogue, dossier d'export du type de document s'il est réglé), sans le changer ;
  - **Fermer**.
- L'aperçu, le fichier enregistré et le papier viennent des mêmes octets : le PDF généré une fois par `crate::pdf::generer`, rendu par `hayro` pour l'écran comme pour l'imprimante.

## 3. Impression (Windows)

- **Aucune nouvelle crate.** `windows-sys` 0.61.2 est déjà une dépendance de l'application ; il lui faut de nouvelles fonctionnalités, vérifiées dans la source de la 0.61.2 : `Win32_UI_Controls_Dialogs` (`PrintDlgExW`), `Win32_Graphics_Gdi` (`CreateDCW`, `GetDeviceCaps`, `StretchDIBits`, `DEVMODEW`), `Win32_Storage_Xps` (`StartDocW`, `DOCINFOW`, `DeviceCapabilitiesW`).
- Pas d'impression confiée au lecteur PDF par défaut (`ShellExecute` « print ») : elle dépend d'Edge ou d'Acrobat et ne contrôle ni les copies ni les pages.
- **Fenêtre d'impression** : `PrintDlgExW` sur le fil de l'interface, avec la fenêtre de Kadenz comme propriétaire (poignée tirée de Slint par `raw-window-handle`, déjà activé). Pages de 1 au nombre de pages du document, tableau des plages alloué. Résultats distingués : imprimer, appliquer sans imprimer, annuler.
- **Copies et assemblage** : lus dans le `DEVMODE` rendu (`PD_USEDEVMODECOPIESANDCOLLATE`) ; si le pilote ne gère pas les copies (`DeviceCapabilitiesW`, `DC_COPIES`), Kadenz les envoie lui-même.
- **Envoi** dans un fil de travail : `CreateDCW` à partir du `DEVMODE` et du `DEVNAMES` du dialogue, `StartDocW`, puis pour chaque page `StartPage`, rendu, `StretchDIBits`, `EndPage`, puis `EndDoc` ; `AbortDoc` en cas d'erreur. L'écran reste utilisable et affiche l'avancement, puis le résultat.
- **Mémoire** : une page A4 en couleur occupe environ 35 Mo à 300 ppp et 139 Mo à 600. Le rendu est plafonné à 300 ppp, ou à la résolution du pilote si elle est plus basse, une page à la fois, libérée après son envoi.
- **Position** : impression à 100 %, calée sur l'origine physique de la feuille (`PHYSICALOFFSETX/Y`, `PHYSICALWIDTH/HEIGHT`, `LOGPIXELSX/Y`), pour respecter les marges réglées dans Kadenz. Image 24 bits de haut en bas. Si le papier choisi est plus petit que l'A4 (Letter), la page est réduite pour tenir dans la zone imprimable, jamais agrandie (`HALFTONE`).

## 4. Hors du périmètre

- Les états et listes qui ont déjà un export PDF (état du stock, TVA collectée, journal, relances, feuille de comptage) gardent leur export ; l'aperçu pourra leur être étendu ensuite, sur la même fenêtre (question 2).
- Aucune modification du contenu ni de la mise en page du PDF.

## 5. Jalons

| Jalon | Contenu | Qui |
|---|---|---|
| A1 | Fenêtre d'aperçu (rendu `hayro` de toutes les pages), bouton « Aperçu » sur les six types de documents, « Enregistrer en PDF » depuis l'aperçu ; textes traduits dans les six langues | kadenz-22 |
| A2 | Impression : fenêtre d'impression de Windows, copies, plage de pages, envoi dans un fil de travail, réduction hors A4 | kadenz-22, relecture kadenz-a8 |
| A3 | Essai à l'écran sur un banc d'essai, d'abord avec l'imprimante virtuelle « Microsoft Print to PDF » (pages, copies, plage, annulation) ; l'imprimante réelle de l'utilisateur seulement à la fin, avec son accord | l'utilisateur |

## 6. Questions pour l'utilisateur

1. **Trace de l'impression** : l'impression d'un document validé doit-elle laisser une trace (journal des actions, mention « duplicata » à partir de la seconde impression) ? Aucune source n'a été trouvée sur une obligation OHADA en ce sens ; rien n'est ajouté sans la décision de l'utilisateur.
2. **Portée** : l'aperçu et l'impression pour les six types de documents d'abord (proposé), ou aussi tout de suite pour les états et listes (stock, TVA, journal, relances) ?
3. **Papier hors A4** : réduire la page pour qu'elle tienne (proposé), avec ou sans message à l'écran ?
