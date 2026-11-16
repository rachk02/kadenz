# Kadenz phase 2 - Gestion de stock (étape 2, chantier 3.3)

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-16 | Forme proposée par kadenz-34 après les décisions 64 à 67 de l'utilisateur (question ouverte 4 du cahier de la phase 2) : règles, valorisation en entiers, données, synchronisation, droits, écrans, découpage en jalons S1 à S3, avec les ajouts de kadenz-22 (droit de voir la valeur, avertissement avant une confirmation qui rend le stock négatif, inventaire en deux temps, date de la dernière synchronisation). Rien n'est codé. |
| 0.2 | 2026-11-16 | S1 codé (kadenz-34, forme convenue avec kadenz-22) : pas de table `stock` répliquée, le stock d'un produit est celui que garde son dernier mouvement ; aucun mouvement sur la copie d'un poste, même pour une écriture faite hors connexion ; heure de la dernière synchronisation gardée par le poste ; inventaire en cours dans une table locale. |

Statut : **proposé**. Les décisions 64 à 67 sont prises (l'utilisateur a demandé de retenir les choix recommandés). Les choix de la section 3 sont ceux de kadenz-34, retenus pour avancer et révisables par l'utilisateur ; ils sont marqués « retenu ». Rien ici ne s'appuie sur une règle comptable : la conformité de la valorisation au SYSCOHADA est à vérifier avec le chantier 3.2, sur une source.

## 1. Décisions (cahier de la phase 2, section 2)

- **64. Valorisation** : coût moyen pondéré. Le coût moyen d'un produit est recalculé à chaque entrée.
- **65. Dépôts** : un seul dépôt par entreprise. Plusieurs dépôts pourront venir plus tard sans changer les mouvements déjà enregistrés (ils seront tous rattachés au dépôt d'origine).
- **66. Stock négatif** : permis, avec une alerte. Une sortie n'est jamais refusée faute de stock ; le produit est signalé tant que sa quantité est négative. Raison : hors connexion, deux postes peuvent livrer le même dernier article (cahier, 3.3), et un refus du serveur bloquerait la trace d'une livraison qui a déjà eu lieu.
- **67. Sorties** : seule la confirmation d'un bordereau de livraison sort le stock ; la réception d'un bon de commande l'entre. Une facture ne bouge jamais le stock : une vente au comptoir passe par un bordereau créé depuis la facture (J6), ce qui évite de compter deux fois une facture livrée plus tard.

## 2. Ce qui existe en V1 (cahier des charges, J6)

- Produit : désignation, référence, prix, sens (`vente`, `achat`, `les_deux`), actif. Aucune notion de stock ni de service.
- Bon de commande : réception ligne par ligne d'un bon Envoyé ou en Réception partielle ; correction à la baisse permise, retour à zéro après une réception partielle refusé, réception complète définitive ; annulation seulement avant toute réception.
- Bordereau : créé depuis une facture ou vierge ; « Confirmer la livraison » le fait passer en Livraison partielle ou complète, de façon définitive ; annulation seulement avant toute livraison.
- Montants en FCFA entiers, quantités en millièmes, arrondi unique `arrondir_division` (`crates/domaine/src/calcul.rs`).

Conséquence : aucune annulation ne défait une réception ou une livraison, donc aucun mouvement n'est jamais annulé. Une erreur se corrige par un ajustement. Si une annulation après réception ou livraison était un jour permise, elle créerait un mouvement inverse à la valeur du mouvement d'origine, sans jamais le réécrire (proposition de kadenz-22).

## 3. Règles

### 3.1 Produits suivis (retenu)

- Une case « Suivi en stock » sur la fiche produit, décochée par défaut, y compris pour les produits repris d'une base V1. Un service reste décoché.
- Seules les lignes liées à un produit suivi bougent le stock ; une ligne libre ne le bouge jamais.
- Un produit `vente` seulement ne passe pas sur un bon de commande (J6) : s'il est suivi, ses entrées viennent d'un ajustement ou d'un inventaire. Les produits achetés puis revendus sont en `les_deux`.
- Décocher la case est refusé tant que la quantité n'est pas nulle (la ramener à zéro par un ajustement d'abord). Cocher la case part d'un stock nul.

### 3.2 Mouvements

| Nature | Quand | Quantité | Valeur |
|---|---|---|---|
| Réception | réception enregistrée sur un bon de commande (écart avec la quantité déjà reçue) | positive, ou négative pour une correction à la baisse | coût d'entrée (3.3) |
| Livraison | confirmation d'un bordereau | négative (quantité livrée) | valeur de sortie (3.4) |
| Ajustement | saisi à la main, avec un motif obligatoire | positive ou négative | entrée : coût unitaire saisi, le coût moyen du moment proposé ; sortie : valeur de sortie |
| Inventaire | validation d'un inventaire (3.8) | pour chaque produit compté, écart entre la quantité comptée et la quantité du serveur | comme un ajustement |

- Le serveur seul crée les mouvements, en appliquant l'opération qui les cause (réception, confirmation, ajustement, inventaire), dans la même transaction. Un mouvement n'est jamais modifié ni supprimé (ajout seul).
- Date du mouvement : le jour du serveur, dans le fuseau de l'entreprise (décision 37). Chaque mouvement garde aussi le compte qui a agi, le document et la ligne d'origine, et la quantité et la valeur du produit après lui, ce qui donne l'état du stock à n'importe quelle date sans recalcul.
- Avoir : aucun mouvement. Un retour de marchandise se saisit par un ajustement (motif « Retour client »).
- Stock de départ : zéro. Le passé V1 n'est pas rejoué (la V1 n'a jamais gardé de stock initial, le rejouer donnerait de fausses quantités) ; on saisit un inventaire d'ouverture, avec le coût unitaire de chaque produit.

### 3.3 Coût d'entrée (retenu, à revoir avec 3.2)

- Pour une réception de `q` millièmes sur une ligne de prix `p` d'un bon de commande de remise `r` : `brut = total_ligne(p, q)`, puis `coût = brut - appliquer_pourcentage(brut, r)`. Hors TVA ; la retenue n'est pas déduite (c'est une retenue fiscale, pas une baisse du prix).
- Une correction à la baisse sort au même coût d'entrée, calculé sur la quantité retirée, ce qui annule exactement l'entrée d'origine.
- Frais d'approche et TVA non récupérable : non compris, faute de règle ; à trancher avec le chantier 3.2.

### 3.4 Valeur de sortie, en entiers

Chaque produit suivi garde sa quantité `Q` (millièmes) et sa valeur `V` (FCFA), jamais un coût unitaire arrondi, plus son dernier coût unitaire connu `c` (FCFA par unité, arrondi), recalculé quand `Q > 0` après une entrée.

- Entrée de `q` au coût `v` : `Q += q`, `V += v`, puis si `Q > 0`, `c = arrondir_division(V × 1000, Q)`.
- Sortie de `q` (positif) :
  - si `q < Q` : valeur sortie `arrondir_division(V × q, Q)` ;
  - si `q = Q` : valeur sortie `V` (la sortie qui vide le stock prend le reste, la valeur ne dérive pas) ;
  - si `q > Q` (stock qui devient ou reste négatif) : la part positive `max(Q, 0)` sort à la valeur `max(V, 0)`, l'excédent sort au dernier coût connu, `total_ligne(c, excédent)` ; la quantité et la valeur deviennent négatives ensemble.
- Une entrée sur un stock négatif comble d'abord le négatif au coût de l'entrée ; le coût moyen est recalculé dès que `Q > 0`.
- Un produit suivi sans aucune entrée a un dernier coût nul : sa sortie à découvert vaut 0 jusqu'à la première entrée ou au premier inventaire.

### 3.5 Hors connexion (décision 50)

- Réception et confirmation d'un bordereau suivent les règles de F6 telles qu'elles sont : le stock bouge quand le serveur applique l'opération.
- Ajustement : permis hors connexion, mis en file d'envoi comme les autres écritures ; la copie le contrôle (droit, produit suivi, motif) sans créer de mouvement, et le serveur le juge au retour.
- Aucun mouvement n'est créé sur la copie d'un poste, même par une réception ou une confirmation faite hors connexion : la copie reçoit les mouvements du serveur avec leur identifiant global, sans risque de mouvement en double.
- Inventaire : refusé hors connexion (bouton inactif, raison « Serveur injoignable »), parce que l'écart se calcule sur la quantité du serveur au moment du comptage.
- La quantité affichée sur un poste est celle de sa copie, donc celle du serveur à la dernière synchronisation, dont la date et l'heure sont affichées hors connexion ; un ajustement en attente est montré comme tel, sans changer la quantité affichée.
- Pas de calcul du stock sur un poste seul : la phase 2 n'a pas de mode « poste seul » (décision 31), le stock est toujours calculé par le serveur.

### 3.6 Alerte de stock négatif

- Liste des produits : quantité négative signalée (texte et couleur, sans liseré d'un seul côté).
- Tableau de bord : indicateur « Produits en stock négatif » avec leur nombre, qui ouvre la liste filtrée.
- Avant de confirmer un bordereau qui rendrait une quantité négative d'après la copie du poste, un avertissement nomme les produits et les quantités, sans bloquer (décision 66).
- Pas de seuil minimal ni de réapprovisionnement dans S1 à S3 ; un seuil d'alerte de stock bas reste une piste, non demandée.

### 3.7 Droits et modules (retenu)

- Nouveau droit « Ajuster le stock et saisir un inventaire », accordé par défaut à l'Admin et au Comptable, modifiable dans la matrice comme les autres.
- Nouveau droit « Voir la valeur du stock » (coût moyen, valeurs des mouvements et de l'état du stock), accordé par défaut à l'Admin et au Comptable : les autres comptes voient les quantités sans les valeurs. Le serveur envoie quand même les valeurs dans la copie de chaque poste ; ce droit cache les valeurs à l'écran et dans les exports, il ne les retire pas de la base du poste (proposition de kadenz-22).
- Réception et confirmation gardent leur droit actuel (« Valider un document »).
- Cocher « Suivi en stock » suit le droit de modifier un produit.
- Consulter le stock et ses mouvements suit « Consulter tableau de bord et registres ».
- Pas de module de licence propre au stock dans S1 à S3 : les entrées dépendent du module Bons de commande, les sorties du module Livraisons, les ajustements et l'inventaire restent toujours permis. Un module « Stock » vendu à part pourra venir avec la grille de prix (question ouverte 2), au prix d'un changement du format des modules de licence.

### 3.8 Inventaire en deux temps (proposition de kadenz-22)

- Feuille de comptage en PDF : produits suivis, référence, désignation, quantité théorique (case à cocher pour la masquer), case vide pour la quantité comptée.
- Saisie des quantités comptées, gardée sur le poste comme inventaire en cours tant qu'elle n'est pas validée (elle ne part pas au serveur avant).
- Validation : une seule opération portant toutes les lignes saisies, avec un motif ; le serveur crée un mouvement d'écart par produit dont la quantité diffère, sur sa propre quantité au moment de l'appliquer. Refusée hors connexion (3.5).
- Inventaire d'ouverture : le même, avec un coût unitaire à saisir pour chaque produit dont la quantité part de zéro.

## 4. Données et synchronisation

- Produit : colonne `suivi_stock` (0 ou 1), dans la fiche produit (`FicheProduit.suivi_stock`), portée par `CreerProduit` et `ModifierProduit` ; décocher un produit dont la quantité n'est pas nulle est refusé.
- Table `mouvement_stock` : `uid`, rang sur le serveur, produit, nature, date, heure, quantité, valeur, quantité, valeur et dernier coût après le mouvement, document et rang de la ligne d'origine, motif, compte. Ajout seul, répliquée comme entité `mouvement`. Le stock d'un produit est celui de son dernier mouvement (rang le plus grand), le stock à une date celui du dernier mouvement de ce jour ou d'avant : pas de table de totaux, donc rien dans la ligne du produit, dont la version ne bouge pas à chaque mouvement (une fiche modifiée hors connexion n'est pas vue en conflit à tort, décision 17).
- Table `comptage_stock` : inventaire en cours d'une base, jamais répliqué.
- Protocole (version 6) : nouvelles actions `AjusterStock { uid, produit, quantite, cout_unitaire, motif }` et `SaisirInventaire { uid, motif, lignes: [{ uid, produit, quantite_comptee, cout_unitaire }] }`, famille de la gestion commerciale ; l'`uid` de l'ajustement et celui de chaque ligne d'inventaire deviennent ceux des mouvements créés. La version du protocole augmente, avec la règle de F10 (serveur mis à jour avant les postes).
- Poste : heure de la dernière page de changements reçue (`replique::derniere_synchro`), affichée hors connexion à côté des quantités.
- Migration du schéma à la suite des existantes, sans perte ; une base reprise ou ancienne n'a aucun produit suivi.

## 5. Écrans (kadenz-22)

- Fiche produit : case « Suivi en stock », et pour un produit suivi sa quantité, son coût moyen et sa valeur.
- Liste des produits : colonnes quantité et valeur pour les produits suivis, filtre « en stock négatif ».
- Mouvements d'un produit : liste filtrée par période et par nature, avec le document d'origine cliquable, exportable.
- Ajustement : produit, quantité signée, coût unitaire (proposé), motif obligatoire.
- Inventaire (3.8) : feuille de comptage en PDF, saisie en cours, écart montré avant de valider.
- Confirmation d'un bordereau : avertissement de 3.6.
- Hors connexion : date et heure de la dernière synchronisation à côté des quantités.
- Valeurs cachées sans le droit « Voir la valeur du stock ».
- État du stock à une date : quantité, coût moyen et valeur par produit et total, en PDF et en Excel comme les autres registres.
- Tableau de bord : indicateur de 3.6.
- Textes dans les six langues.

## 6. Jalons

| Jalon | Contenu | Qui | Vérification |
|---|---|---|---|
| S1 | Domaine (valorisation en entiers), migration, stockage (mouvements créés par la réception, la confirmation, l'ajustement et l'inventaire), droit nouveau, protocole, serveur, réplique sur le poste | kadenz-34 | tests sans écran : coût moyen sur plusieurs entrées, sortie qui vide le stock sans reste, stock négatif puis entrée, correction de réception à la baisse, inventaire, refus de décocher un produit non nul, ajustement hors connexion envoyé au retour, droits ajoutés à une matrice déjà enregistrée. Fait le 2026-11-16 (498a552, puis les essais de bout en bout) : la copie réelle d'un poste coupé du serveur met en file un ajustement et une confirmation sans créer de mouvement, le serveur les crée au retour (l'ajustement avec l'identifiant du poste) et la copie les reçoit une seule fois ; une livraison au-delà du stock passe sans refus et rend le stock négatif (base unique : le serveur ne refuse jamais une sortie, quel que soit le poste qui l'envoie) |
| S2 | Écrans de la section 5 | kadenz-22 | essais de l'application et revue des textes |
| S3 | Essai à l'écran sur un banc d'essai (jamais sur les données réelles), avec les données exactes à saisir données dans la consigne | l'utilisateur | inventaire d'ouverture, réception, livraison, ajustement, stock négatif signalé, état du stock |
