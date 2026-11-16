# Kadenz - Aperçu et impression des documents

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-16 | Proposition de kadenz-22 après la demande de l'utilisateur pendant l'essai S3 (« permettre pour chaque document d'avoir un aperçu, un bouton, et dans l'aperçu d'autres actions comme imprimer, avec la sélection des imprimantes, ou enregistrer »), avec les points techniques de kadenz-a8. Rien n'est codé. |
| 0.2 | 2026-11-16 | Relecture de kadenz-a8 : rendu de l'aperçu limité aux pages visibles, réglages d'impression gardés pour la session, réduction dès que la zone imprimable ne contient pas la page, envoi en bandes, filigrane du brouillon, initialisation COM à vérifier. |
| 0.3 | 2026-11-16 | A1 livré (f17e1ed) et relu par kadenz-a8 : rendu sur fond blanc par `hayro`, largeur de rendu selon l'échelle de l'écran, temps de rendu mesuré, aperçu de l'état enregistré ; pour A2, image DIB et rendu par bandes précisés. |

Statut : **A1 livré**, A2 en cours. Les questions de la section 6 sont à l'utilisateur.

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
- **Mémoire de l'aperçu** : une page A4 rendue à 150 ppp occupe environ 8,7 Mo, soit environ 175 Mo pour 20 pages. Seules les pages visibles et leurs voisines sont gardées en image ; les autres sont libérées et rendues de nouveau quand elles reviennent à l'écran.
- **Brouillon** : il s'affiche et s'imprime avec le filigrane « BROUILLON », comme son PDF (`crates/pdf/src/rendu.rs`, `Filigrane::Brouillon`) ; une facture brouillon imprimée ne peut pas passer pour une facture validée. De même, un document annulé garde son filigrane.
- **Rendu de l'aperçu** (A1) : sur fond blanc, demandé à `hayro` (`PixmapSettings::bg_color`), sinon la page est transparente et ses pixels en alpha prémultiplié. Largeur de rendu : 820 px logiques multipliés par l'échelle de l'écran, entre 1240 px (environ 150 ppp) et 2480 px (300 ppp). Rendu sur le fil de l'interface, mesuré à environ 80 ms par page à 1240 px (build de développement, `hayro` optimisé) ; un fil de travail ne s'imposera que si une mesure en version optimisée dépasse 100 ms.
- **État enregistré** : l'aperçu, comme l'export, montre le document tel qu'il est enregistré ; une saisie modifiée et non enregistrée n'y figure pas (Ctrl+S d'abord). La fiche ne suit pas aujourd'hui ses modifications non enregistrées ; prévenir demanderait ce suivi (décision entre kadenz-22 et kadenz-a8, à revoir si l'utilisateur le souhaite).
- L'aperçu, le fichier enregistré et le papier viennent des mêmes octets : le PDF généré une fois par `crate::pdf::generer`, rendu par `hayro` pour l'écran comme pour l'imprimante.

## 3. Impression (Windows)

- **Aucune nouvelle crate.** `windows-sys` 0.61.2 est déjà une dépendance de l'application ; il lui faut de nouvelles fonctionnalités, vérifiées dans la source de la 0.61.2 : `Win32_UI_Controls_Dialogs` (`PrintDlgExW`), `Win32_Graphics_Gdi` (`CreateDCW`, `GetDeviceCaps`, `StretchDIBits`, `DEVMODEW`), `Win32_Storage_Xps` (`StartDocW`, `DOCINFOW`, `DeviceCapabilitiesW`).
- Pas d'impression confiée au lecteur PDF par défaut (`ShellExecute` « print ») : elle dépend d'Edge ou d'Acrobat et ne contrôle ni les copies ni les pages.
- **Réglages gardés pour la session** : après « Appliquer » ou une impression, l'imprimante et ses propriétés (`DEVMODE`, `DEVNAMES`) sont gardées en mémoire jusqu'à la fermeture de Kadenz et reprises au clic suivant ; rien n'est enregistré sur le disque (décision entre kadenz-22 et kadenz-a8).
- **COM** : vérifier dans la documentation Microsoft si `PrintDlgExW` demande un fil initialisé par `CoInitializeEx` en mode STA ; sans lui, l'appel peut échouer avec un code `HRESULT`. Essai dès A2 sur « Microsoft Print to PDF ».
- **Fenêtre d'impression** : `PrintDlgExW` sur le fil de l'interface, avec la fenêtre de Kadenz comme propriétaire (poignée tirée de Slint par `raw-window-handle`, déjà activé). Pages de 1 au nombre de pages du document, tableau des plages alloué. Résultats distingués : imprimer, appliquer sans imprimer, annuler.
- **Copies et assemblage** : lus dans le `DEVMODE` rendu (`PD_USEDEVMODECOPIESANDCOLLATE`) ; si le pilote ne gère pas les copies (`DeviceCapabilitiesW`, `DC_COPIES`), Kadenz les envoie lui-même.
- **Envoi** dans un fil de travail : `CreateDCW` à partir du `DEVMODE` et du `DEVNAMES` du dialogue, `StartDocW`, puis pour chaque page `StartPage`, rendu, `StretchDIBits`, `EndPage`, puis `EndDoc` ; `AbortDoc` en cas d'erreur. L'écran reste utilisable et affiche l'avancement, puis le résultat.
- **Mémoire** : une page A4 en couleur occupe environ 35 Mo à 300 ppp et 139 Mo à 600. Le rendu est plafonné à 300 ppp, ou à la résolution du pilote si elle est plus basse, une page à la fois, libérée après son envoi. `hayro::render` alloue la page entière et ramène ses dimensions à `u16` ; l'impression rend donc chaque bande par `render_into`, dans un `RenderContext` de la taille de la bande, avec la transformation de la page translatée de la hauteur des bandes déjà envoyées (assemblage de `render`, `hayro` 0.9, `lib.rs`).
- **Image envoyée** : rendue sur fond blanc (`bg_color`), pixels opaques convertis de RGBA en BGR 24 bits ; lignes du DIB alignées sur 4 octets (`(3 × largeur + 3) & !3`) et `biHeight` négatif pour une image de haut en bas.
- **Position** : impression à 100 %, calée sur l'origine physique de la feuille (`PHYSICALOFFSETX/Y`, `PHYSICALWIDTH/HEIGHT`, `LOGPIXELSX/Y`), pour respecter les marges réglées dans Kadenz. Si la zone imprimable ne contient pas la page A4 (papier plus petit comme le Letter, orientation Paysage choisie dans la fenêtre, marges du pilote plus grandes que celles du PDF, qui couperaient le bord), la page est réduite pour tenir dans la zone imprimable, jamais agrandie (`HALFTONE`).
- **Envoi en bandes** : certains pilotes refusent une très grande image d'un coup (`GDI_ERROR`) ; chaque page est envoyée en bandes horizontales (8 au départ) plutôt qu'en une seule image.

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
3. **Page réduite** (papier plus petit, paysage, marges du pilote) : réduire la page pour qu'elle tienne (proposé), avec ou sans message à l'écran ?
