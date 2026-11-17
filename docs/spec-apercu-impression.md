# Kadenz - Aperçu et impression des documents

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-16 | Proposition de kadenz-22 après la demande de l'utilisateur pendant l'essai S3 (« permettre pour chaque document d'avoir un aperçu, un bouton, et dans l'aperçu d'autres actions comme imprimer, avec la sélection des imprimantes, ou enregistrer »), avec les points techniques de kadenz-a8. Rien n'est codé. |
| 0.2 | 2026-11-16 | Relecture de kadenz-a8 : rendu de l'aperçu limité aux pages visibles, réglages d'impression gardés pour la session, réduction dès que la zone imprimable ne contient pas la page, envoi en bandes, filigrane du brouillon, initialisation COM à vérifier. |
| 0.3 | 2026-11-16 | A1 livré (f17e1ed) et relu par kadenz-a8 : rendu sur fond blanc par `hayro`, largeur de rendu selon l'échelle de l'écran, temps de rendu mesuré, aperçu de l'état enregistré ; pour A2, image DIB et rendu par bandes précisés. |
| 0.4 | 2026-11-16 | A2 codé et relu par kadenz-a8 : copies non gardées d'une impression à l'autre, rôle réel du drapeau des copies, fenêtre propriétaire obligatoire, code d'erreur de Windows dans le message, fermeture pendant un envoi notée. « Microsoft Print to PDF » ne s'ouvre pas sur le poste de développement (erreur Windows 3, hors de Kadenz aussi) ; l'imprimante PDF de Sage ne s'active que dans Sage ; essai d'envoi reporté à A3 sur décision de l'utilisateur. |
| 0.5 | 2026-11-16 | Décisions de l'utilisateur sur les trois questions (aucune trace d'impression, documents puis états, page réduite avec message) ; jalon A4 : aperçu des six états qui ont un export PDF. |
| 0.6 | 2026-11-17 | A4 essayé à l'écran (étapes 1 et 2 par l'utilisateur, 3 à 5 par kadenz-22). A3 fait par kadenz-22 sans l'utilisateur, à sa demande : envoi réel au pilote PostScript de « Wondershare PDFelement », pages imprimées décodées et regardées, copies assemblées, plage de pages, annulation, message de fin ; « Imprimer dans un fichier » masqué ; conversion des plages sortie et testée. |

Statut : **A1**, **A2**, **A3** et **A4 livrés**. Les questions de la section 6 sont tranchées. Seul reste l'essai sur une imprimante physique, à faire avec l'accord de l'utilisateur (aucune n'est branchée sur le poste de développement).

Essai A3 (2026-11-17, fait par kadenz-22 à la demande de l'utilisateur : « automatise tous les tests », sans son intervention) :

- **Imprimantes du poste** : « Microsoft Print to PDF » ne s'ouvre pas (erreur Windows 3, hors de Kadenz aussi) ; l'imprimante PDF de Sage refuse tout logiciel autre que Sage (erreur 21) ; « Nitro PDF Creator » accepte le travail mais le garde en file sans rien écrire, et l'envoi suivant reste bloqué : à éviter pour les essais. « Wondershare PDFelement » fonctionne : son pilote est le pilote PostScript de Windows (`PScript5.dll`) et il écrit le flux complet dans le fichier demandé par `DOCINFOW`.
- **Contenu imprimé** (test `impression_sur_print_to_pdf` avec `KADENZ_ESSAI_IMPRIMANTE=Wondershare PDFelement`) : le devis d'une page donne une page A4 (`%%Pages: 1`), le devis de 3 pages en 2 copies assemblées par Kadenz en donne 6, dans l'ordre 1, 2, 3, 1, 2, 3. Les images des pages, décodées du flux (8 bandes de 2 480 px, ASCII85 et RunLength), sont complètes, à 100 % sans réduction, filigrane BROUILLON compris.
- **À l'écran** (poste S3, facture F2026-11-0001) : Ctrl+P ouvre l'aperçu, puis la fenêtre d'impression (celle de Windows 11) ; 2 copies réglées par la fenêtre, puis « Imprimer » : « Impression en préparation », bouton grisé, puis « Envoyé à l'imprimante Wondershare PDFelement » et bouton rendu ; le travail passe dans la file de l'imprimante. La fenêtre suivante repropose 1 copie et garde l'imprimante. Plage personnalisée « 1 » : envoyée et confirmée. « Annuler » : aucun message, rien dans la file.
- **Non vérifié** : le fichier final de Wondershare quand Kadenz ne demande pas de fichier (le pilote passe le travail à l'application PDFelement, fermée) ; les copies faites par le pilote se lisent donc seulement dans le `DEVMODE` transmis, pas dans un fichier imprimé.

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
- **Rendu de l'aperçu** (A1) : sur fond blanc, demandé à `hayro` (`PixmapSettings::bg_color`), sinon la page est transparente et ses pixels en alpha prémultiplié. Largeur de rendu : 820 px logiques multipliés par l'échelle de l'écran, entre 1240 px (environ 150 ppp) et 2480 px (300 ppp). Rendu sur le fil de l'interface, mesuré de 8 à 14 ms par page à 1240 px comme à 1640 px (écran à 200 %), rendu seul, `hayro` étant optimisé même en développement (test `pages_de_l_apercu`, `--nocapture`) ; un fil de travail ne s'imposera que si une mesure dépasse 100 ms.
- **État enregistré** : l'aperçu, comme l'export, montre le document tel qu'il est enregistré ; une saisie modifiée et non enregistrée n'y figure pas (Ctrl+S d'abord). La fiche ne suit pas aujourd'hui ses modifications non enregistrées ; prévenir demanderait ce suivi (décision entre kadenz-22 et kadenz-a8, à revoir si l'utilisateur le souhaite).
- L'aperçu, le fichier enregistré et le papier viennent des mêmes octets : le PDF généré une fois par `crate::pdf::generer`, rendu par `hayro` pour l'écran comme pour l'imprimante.

## 3. Impression (Windows)

- **Aucune nouvelle crate.** `windows-sys` 0.61.2 est déjà une dépendance de l'application ; il lui faut de nouvelles fonctionnalités, vérifiées dans la source de la 0.61.2 : `Win32_UI_Controls_Dialogs` (`PrintDlgExW`), `Win32_Graphics_Gdi` (`CreateDCW`, `GetDeviceCaps`, `StretchDIBits`, `DEVMODEW`), `Win32_Storage_Xps` (`StartDocW`, `DOCINFOW`, `DeviceCapabilitiesW`).
- Pas d'impression confiée au lecteur PDF par défaut (`ShellExecute` « print ») : elle dépend d'Edge ou d'Acrobat et ne contrôle ni les copies ni les pages.
- **Réglages gardés pour la session** : après « Appliquer » ou une impression, l'imprimante et ses propriétés (`DEVMODE`, `DEVNAMES`) sont gardées en mémoire jusqu'à la fermeture de Kadenz et reprises au clic suivant ; rien n'est enregistré sur le disque (décision entre kadenz-22 et kadenz-a8). Le nombre de copies n'est pas gardé : la fenêtre suivante repropose une copie, pour qu'une validation trop rapide n'imprime pas de nouveau plusieurs exemplaires d'une facture.
- **COM** : vérifier dans la documentation Microsoft si `PrintDlgExW` demande un fil initialisé par `CoInitializeEx` en mode STA ; sans lui, l'appel peut échouer avec un code `HRESULT`. Essai dès A2 sur « Microsoft Print to PDF ».
- **Fenêtre d'impression** : `PrintDlgExW` sur le fil de l'interface, avec la fenêtre de Kadenz comme propriétaire (poignée tirée de Slint par `raw-window-handle`, déjà activé). Pages de 1 au nombre de pages du document, tableau des plages alloué. Résultats distingués : imprimer, appliquer sans imprimer, annuler.
- **Copies et assemblage** : lus dans le `DEVMODE` rendu (`PD_USEDEVMODECOPIESANDCOLLATE`). Avec ce drapeau, la fenêtre grise le nombre de copies quand le pilote ne sait pas les faire (documentation de `PRINTDLGEXW`) : le pilote fait donc les copies. Garde-fou : si `DeviceCapabilitiesW` (`DC_COPIES`) échoue ou annonce moins de copies que demandé, Kadenz envoie chaque copie et n'en demande qu'une au pilote.
- **« Imprimer dans un fichier »** : case masquée (`PD_HIDEPRINTTOFILE`). Kadenz enverrait quand même le document à l'imprimante, et l'aperçu a déjà « Enregistrer en PDF ».
- **Plages de pages** : numéros saisis bornés au document, dans l'ordre saisi ; une plage à l'envers ne donne que sa première page ; sans plage, tout le document (`sequence_des_pages`, testée).
- **Fenêtre propriétaire** : `PrintDlgExW` exige une fenêtre propriétaire valide ; sans poignée de la fenêtre de Kadenz, un message d'erreur est rendu avant l'appel.
- **Fermeture pendant un envoi** : si Kadenz est fermé pendant qu'un document part à l'imprimante, le fil de travail s'arrête sans `EndDoc` et le spouleur abandonne normalement le travail inachevé. Une confirmation à la fermeture est à l'étude, hors de A2.
- **Envoi** dans un fil de travail : `CreateDCW` à partir du `DEVMODE` et du `DEVNAMES` du dialogue, `StartDocW`, puis pour chaque page `StartPage`, rendu, `StretchDIBits`, `EndPage`, puis `EndDoc` ; `AbortDoc` en cas d'erreur. L'écran reste utilisable et affiche l'avancement, puis le résultat.
- **Mémoire** : une page A4 en couleur occupe environ 35 Mo à 300 ppp et 139 Mo à 600. Le rendu est plafonné à 300 ppp, ou à la résolution du pilote si elle est plus basse, une page à la fois, libérée après son envoi. `hayro::render` alloue la page entière et ramène ses dimensions à `u16` ; l'impression rend donc chaque bande par `render_into`, dans un `RenderContext` de la taille de la bande, avec la transformation de la page translatée de la hauteur des bandes déjà envoyées (assemblage de `render`, `hayro` 0.9, `lib.rs`).
- **Image envoyée** : rendue sur fond blanc (`bg_color`), pixels opaques convertis de RGBA en BGR 24 bits ; lignes du DIB alignées sur 4 octets (`(3 × largeur + 3) & !3`) et `biHeight` négatif pour une image de haut en bas.
- **Position** : impression à 100 %, calée sur l'origine physique de la feuille (`PHYSICALOFFSETX/Y`, `HORZRES`, `VERTRES`, `LOGPIXELSX/Y`), pour respecter les marges réglées dans Kadenz. La règle porte sur le contenu, pas sur la feuille entière : la page reste à 100 % si la partie située à l'intérieur des marges de la mise en page de Kadenz tombe dans la zone imprimable, à 0,5 mm près (une marge blanche coupée ne gêne pas ; comparer le rectangle A4 entier ferait réduire la page sur toute imprimante réelle). Sinon (papier plus petit comme le Letter, orientation Paysage choisie dans la fenêtre, marge du pilote plus grande que celle de Kadenz), la page est réduite pour tenir dans la zone imprimable, jamais agrandie (`HALFTONE`).
- **Envoi en bandes** : certains pilotes refusent une très grande image d'un coup (`GDI_ERROR`) ; chaque page est envoyée en bandes horizontales (8 au départ) plutôt qu'en une seule image.

## 4. Hors du périmètre

- Le journal comptable n'a pas d'export PDF (CSV et Excel seulement) : il n'a pas d'aperçu.
- Aucune modification du contenu ni de la mise en page du PDF.

## 5. Jalons

| Jalon | Contenu | Qui |
|---|---|---|
| A1 | Fenêtre d'aperçu (rendu `hayro` de toutes les pages), bouton « Aperçu » sur les six types de documents, « Enregistrer en PDF » depuis l'aperçu ; textes traduits dans les six langues | kadenz-22 |
| A2 | Impression : fenêtre d'impression de Windows, copies, plage de pages, envoi dans un fil de travail, réduction hors A4 | kadenz-22, relecture kadenz-a8 |
| A3 | Essai à l'écran sur un banc d'essai, d'abord avec une imprimante virtuelle qui fonctionne (pages, copies, plage, annulation) ; l'imprimante réelle de l'utilisateur seulement à la fin, avec son accord | kadenz-22 à la demande de l'utilisateur (imprimante virtuelle) ; imprimante réelle : l'utilisateur |
| A4 | Aperçu étendu aux six états qui ont un export PDF : état du stock et feuille de comptage (Stock), TVA collectée (Comptabilité), registre des règlements, liste des relances et lettre de relance (Paiements). Leur bouton ouvre l'aperçu, d'où l'état s'imprime ou s'enregistre ; « Exporter en PDF » de l'état du stock et de la TVA devient « Aperçu », les autres boutons gardent leur texte | kadenz-22, relecture kadenz-a8 |

## 6. Décisions de l'utilisateur (2026-11-16)

1. **Trace de l'impression** : aucune ; ni journal des actions, ni mention « duplicata ». Aucune source n'a été trouvée sur une obligation OHADA en ce sens.
2. **Portée** : les six types de documents d'abord (A1, A2), puis les états qui ont un export PDF (A4, demandé par l'utilisateur après la livraison de A2 : « étends l'aperçu aux états »).
3. **Page réduite** : réduite pour tenir sur le papier, avec le message « Envoyé à l'imprimante X, page réduite pour tenir sur le papier » (« garde le message »).
