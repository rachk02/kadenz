# Kadenz phase 2 - Export comptable SYSCOHADA (étape 2, chantier 3.2)

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-16 | Brouillon de kadenz-22 sur le partage convenu avec kadenz-34 (kadenz-22 : brouillon et écrans ; kadenz-34 : stockage, format du fichier, relecture). Comptes et règles relevés sur le texte officiel, page par page ; ce que le texte ne tranche pas reste en question. Rien n'est codé. |
| 0.2 | 2026-11-16 | Relecture de kadenz-34 reportée : clôture complétée (validation refusée dans un mois clos, clôture refusée hors connexion, totaux recalculés par le serveur, pas de réouverture en C1), documents annulés hors du journal, voie B de 4.4 retenue et précisée, détails du CSV, migration du droit nouveau. |
| 0.3 | 2026-11-16 | Interface de C1 fixée par kadenz-34 : mention provisoire hors du CSV (nom du fichier, écran, Excel), nature comptable figée sur la ligne à la validation, mois clos qui garde ses comptes, empreinte du mois envoyée à la clôture. |
| 0.4 | 2026-11-16 | Essai C3 : pièce partagée par les règlements d'une même facture notée en 4.2, question 7 ajoutée ; règle inchangée. |

Statut : **proposé**, relu par kadenz-34. Les choix marqués « retenu » sont ceux des sessions, pris pendant l'absence de l'utilisateur (décision du 2026-11-16) et révisables par lui. La question ouverte 3 du cahier de la phase 2 (logiciel comptable cible, format attendu, qui confirme le plan de comptes) reste à l'utilisateur.

## 1. Source

- **Texte officiel** : Acte uniforme relatif au droit comptable et à l'information financière (AUDCIF) et Système comptable OHADA, Journal officiel de l'OHADA, numéro spécial du 15 février 2017. Notice de la bibliothèque numérique de l'OHADA : https://biblio.ohada.org/pmb/opac_css/index.php?lvl=notice_display&id=4847.
  - `explnum_id=2061` : l'Acte seul, avec une couche de texte.
  - `explnum_id=2063` : version complète signée, 1 246 pages scannées, SYSCOHADA révisé en annexe. Les numéros de page cités ici sont ceux du PDF (page imprimée = page du PDF moins 3).
- **Lecture** : les pages de 2063 sont des images. Elles ont été extraites en Python pur, sans dépendance nouvelle, avec deux scripts gardés hors du dépôt, puis lues par kadenz-22. kadenz-34 a relu six des dix pages du plan de comptes (241, 243, 251, 252, 262, 263) : conformes.
- **Réserve** : une page du plan de comptes prouve qu'un compte existe, pas qu'il convient à une opération. Seules les règles de la section 2.2, lues dans le fonctionnement des comptes, disent quel compte employer. Tout le reste est à confirmer par un comptable.
- **Sources secondaires** (blogs, éditeurs de logiciels) : écartées comme référence ; elles concordent avec ce qui est relevé ici.

## 2. Ce que dit le texte

### 2.1 Comptes existants (plan de comptes, liste des comptes)

| Compte | Libellé officiel | Page |
|---|---|---|
| 401 / 4011 | Fournisseurs, dettes en compte / Fournisseurs | 240 |
| 411 / 4111 | Clients / Clients | 241 |
| 419 / 4191 | Clients créditeurs / Clients, avances et acomptes reçus | 242 |
| 4198 | Clients, rabais, remises, ristournes et autres avoirs à accorder | 242 |
| 443 | État, T.V.A. facturée | 243 |
| 4431 | T.V.A. facturée sur ventes | 243 |
| 4432 | T.V.A. facturée sur prestations de services | 243 |
| 4433 | T.V.A. facturée sur travaux | 243 |
| 4441 | État, T.V.A. due | 244 |
| 447 | État, impôts retenus à la source | 244 |
| 513 | Chèques à encaisser | 250 |
| 515 | Cartes de crédit à encaisser | 250 |
| 521 / 5211 | Banques locales / Banques en monnaie nationale | 250 |
| 552 | Monnaie électronique - téléphone portable | 251 |
| 571 / 5711 | Caisse siège social / Caisse en monnaie nationale | 252 |
| 701 / 7011 | Ventes de marchandises / dans la Région | 262 |
| 7019 | Rabais, remises, ristournes accordés (non ventilés) | 262 |
| 705 / 7051 | Travaux facturés / dans la Région | 263 |
| 706 / 7061 | Services vendus / dans la Région | 263 |

Le sens de « Région » est donné par une note de bas de page qui n'a pas été lue : à confirmer avant de choisir entre 7011 et 7012.

### 2.2 Règles lues dans le fonctionnement des comptes (compte 70, pages 469 à 471)

- **Contenu** (p. 469) : le compte 70 enregistre les ventes de marchandises, les travaux effectués et les services rendus à des tiers. Le compte 701 est ouvert par les entités commerciales (p. 470).
- **Prix de vente net** (p. 470) : « Le prix de vente s'entend du prix facturé, le cas échéant, net de taxes collectées, déductions faites des rabais et remises lorsqu'ils sont déduits sur la facture elle-même. »
- **Vente** (p. 470) : le compte 70 est crédité du montant hors taxes ; le compte 443 est crédité du montant des taxes ; par le débit du compte 41 (clients) ou d'un compte de trésorerie.
- **Retours et rabais hors facture** (p. 471) : le compte 70 est débité des retours hors taxes et des rabais, remises et ristournes accordés hors facture ; le compte 443 est débité des taxes correspondantes ; par le crédit du compte 41.
- **Escomptes de règlement** (p. 470) : au débit du compte 673, même déduits sur la facture. Kadenz n'en a pas.

### 2.3 Valorisation du stock (Acte, article 44)

L'article 44 de l'AUDCIF (explnum_id=2061, page 31 du PDF) permet, pour les biens interchangeables, soit le premier entré premier sorti, soit le coût moyen pondéré, avec la même méthode pour les stocks de nature et d'usage semblables. La décision 64 (coût moyen pondéré) est donc permise ; la question de `spec-stock.md` est close sur ce point.

## 3. Ce qui existe dans Kadenz (relevé du code)

- **Documents de vente** : facture et avoir. Numéro attribué par le serveur à la validation, unique par type (`F2026-05-0001`, `A2026-05-0001` par défaut, préfixes réglables et distincts). Après validation, seuls le mode de paiement et les observations changent.
- **Totaux figés** : HT, remise (taux unique sur le document), base TVA = HT moins remise, TVA (un seul taux par document), TTC. Montants en FCFA entiers.
- **Retenue à la source** : seulement sur un bon de commande, jamais sur une facture de vente.
- **Avoir** : lié à sa facture, même client et mêmes taux, montants positifs, sa propre date et son propre numéro, plafonné au reste à créditer. Le tableau de bord déduit les avoirs de leur facture quelle que soit leur date ; l'export ne doit pas reprendre ce calcul.
- **Annulation** : logique, le numéro reste. Elle n'est permise que sans règlement ni avoir validé. La date d'annulation n'est gardée que dans le journal d'audit, qui n'est pas copié sur les postes.
- **Règlements** : date, montant, mode (texte libre choisi dans la liste `paiement.modes` : Espèce, Chèque, Virement bancaire, Carte bancaire, Moov money, Orange money par défaut). Un règlement peut être supprimé.
- **Tiers** : ni code ni compte comptable.
- **Produits** : rien ne distingue une marchandise d'un service (le produit `POSE` de l'essai S3 est un service).
- **Achats** : pas de facture fournisseur, seulement le bon de commande.
- **Rapports** : calculés par le poste sur sa copie locale ; documents, lignes et règlements y sont copiés.

## 4. Règles proposées

### 4.1 Périmètre (retenu)

- Journal des ventes (factures et avoirs validés ou envoyés), journal des encaissements (règlements de factures), état de la TVA collectée.
- Hors périmètre : achats et décaissements (pas de facture fournisseur, une écriture au 401 n'aurait pas de contrepartie), retenue à la source (absente des ventes), multi-devise (3.5 non commencé), stock et coût des ventes.

### 4.2 Écritures

- **Facture** (règles 2.2) : débit client du TTC ; crédit ventes de la base TVA (HT net de la remise portée sur la facture) ; crédit TVA facturée du montant de TVA. Écriture équilibrée par construction : TTC = base + TVA, la retenue étant nulle sur une facture.
- **Avoir** (règle 2.2, retours et rabais) : écriture inverse à la date de l'avoir : débit ventes de la base, débit TVA facturée, crédit client du TTC. Question pour un comptable : compte de ventes lui-même ou sous-compte 7019 / 7069 « non ventilés ».
- **Règlement d'une facture** : débit du compte de trésorerie de son mode, crédit client. Le fonctionnement des comptes 41 et 5 n'a pas été lu : écriture à confirmer par un comptable.
- **Documents annulés** : une facture ou un avoir annulé n'entre pas au journal ; son numéro manque alors dans la suite (question 5 de la section 7).
- **Nature d'une ligne** : figée sur la ligne à la validation, comme la désignation ; une ligne libre (sans produit) compte en marchandise. Changer la nature d'un produit ne change pas les factures déjà validées.
- **Pièce** : le numéro du document (facture, avoir) ; pour un règlement, le numéro de la facture réglée. Plusieurs règlements d'une même facture partagent donc une pièce, alors que le compteur de l'écran les compte un par un ; un import qui regroupe par pièce les fusionnerait, toujours équilibrés. Gardé jusqu'à la réponse à la question 7 (relevé de l'essai C3).
- **Libellé** : type, numéro et nom du client figé dans le document.

### 4.3 Comptes paramétrables (retenu)

Rubrique « Comptabilité » de Paramètres, réservée au droit de la section 4.7. Valeurs par défaut prises dans 2.1, chacune marquée « à confirmer par un comptable » dans l'écran :

| Paramètre | Défaut proposé | Fondement |
|---|---|---|
| Compte clients | 4111 | existe, p. 241 ; débit du 41 à la vente, p. 470 |
| Ventes de marchandises | 7011 | existe, p. 262 ; sens de « Région » à confirmer |
| Services vendus | 7061 | existe, p. 263 |
| TVA facturée sur ventes | 4431 | existe, p. 243 ; crédit du 443, p. 470 |
| TVA facturée sur prestations de services | 4432 | existe, p. 243 |
| Trésorerie par mode de paiement | Espèce 5711, Chèque 513, Virement bancaire 5211, Carte bancaire 515, Moov money et Orange money 552 | comptes existants, p. 250 à 252 ; rattacher le mobile money au 552 « téléphone portable » est une interprétation de kadenz-22, à confirmer |
| Codes journal | VT ventes, EN encaissements | choix de logiciel, sans source, libres |

Un mode de paiement ajouté plus tard sans compte tombe sur un compte d'attente paramétrable ; l'écran d'export signale les règlements concernés avant d'exporter.

### 4.4 Factures mêlant marchandises et services (voie B retenue)

Le texte distingue 701 et 706, 4431 et 4432 ; Kadenz ne sait pas si un produit est une marchandise ou un service. Deux voies :

- **A** : un seul compte de ventes et un seul compte de TVA pour toute l'entreprise (simple, juste pour une entité qui ne vend que des marchandises ou que des services).
- **B (retenue, codée par kadenz-34 dans C1)** : une nature comptable sur la fiche produit (marchandise, service), défaut marchandise, indépendante de « Suivi en stock » (un produit non suivi n'est pas forcément un service). Une facture est ventilée par nature : la remise du document est répartie par nature avec `appliquer_pourcentage`, puis la TVA de chaque nature est calculée sur sa base au taux du document. L'écart d'arrondi va à la plus grosse part, pour que base, TVA et TTC restent exactement ceux de la facture figée.

### 4.5 Comptes clients (retenu, révisable)

Compte collectif unique (4111 par défaut), avec une colonne « compte tiers » vide. Des comptes individuels demanderaient un code par tiers (champ nouveau, côté kadenz-34) ; à décider avec la question 3, selon ce qu'attend le logiciel cible.

### 4.6 Changements après export : clôture d'une période (retenu)

Un export déjà remis au comptable ne doit plus changer. Or une facture peut être annulée et un règlement supprimé après coup, et la date d'annulation n'est pas sur le document. D'où la **clôture d'une période** (proposition de kadenz-22, complétée par kadenz-34).

- L'utilisateur clôt un mois après l'avoir exporté ; la clôture part au serveur comme une écriture.
- Le serveur refuse ensuite, pour un document ou un règlement daté dans un mois clos : la validation d'une facture ou d'un avoir (une facture antidatée changerait l'export remis), leur annulation, l'ajout ou la suppression d'un règlement. La correction passe par un avoir daté du mois ouvert.
- La clôture se fait en ligne seulement (`EnCoupure::Validation`, comme la validation d'un inventaire). Une écriture faite hors connexion avant la clôture et rejouée après est refusée par le serveur et remonte comme les autres refus.
- L'action de clôture porte l'empreinte (SHA-256) des écritures du mois calculées par le poste, comptes compris. Le serveur la recalcule et refuse si elle diffère (« Copie pas à jour : synchronisez puis recommencez »). Il refuse aussi un mois pas entièrement passé, un mois déjà clos et un mois dont une écriture n'a pas de compte. Un mois clos correspond ainsi toujours aux données du serveur.
- Pas de réouverture dans C1 (question 6 de la section 7).
- Un mois non clos peut être exporté autant de fois qu'on veut ; il est signalé « provisoire » (4.8).
- Un mois clos garde les comptes en vigueur à sa clôture : changer un compte plus tard ne change pas son export.

Variante écartée pour l'instant : contre-écritures datées de l'annulation, qui demandent une date d'annulation stockée et répliquée.

### 4.7 Droit et calcul (retenu)

- Droit nouveau « Exporter la comptabilité et clore une période », donné par défaut aux rôles Administrateur et Comptable.
- Calcul sur le poste, depuis sa copie, comme les autres rapports : documents et règlements y sont déjà. Le calcul est une fonction unique de `kadenz_stockage`, appelée aussi par le serveur pour contrôler une clôture (4.6).
- Le droit nouveau est ajouté aux matrices de droits déjà enregistrées par une migration (`json_insert`, comme 0016), avec son test. Hors connexion, l'export reste possible, avec la date de la dernière synchronisation à l'écran et dans l'Excel ; la clôture, elle, attend le serveur.

### 4.8 Format (retenu en attendant la question 3)

- Journal d'écritures générique, une ligne par écriture : journal, date, pièce, compte, compte tiers, libellé, débit, crédit. Chaque pièce équilibrée ; total débit = total crédit.
- Excel par le moteur `kadenz_export` existant, avec les totaux.
- CSV (choix de kadenz-34, révisables quand le logiciel cible sera connu) :
  - UTF-8 avec BOM (pour qu'Excel ouvre les accents), séparateur point-virgule, fin de ligne CRLF ;
  - une ligne d'en-tête : `Journal;Date;Pièce;Compte;Compte tiers;Libellé;Débit;Crédit` ;
  - date JJ/MM/AAAA ; montants en FCFA entiers, sans séparateur de milliers ni décimale, 0 dans la colonne vide ;
  - champ entre guillemets s'il contient un point-virgule, un guillemet ou un retour à la ligne, guillemet doublé (RFC 4180) ;
  - nom : `journal-AAAA-MM.csv`, ou `journal-AAAA-MM-JJ_AAAA-MM-JJ.csv` pour une période libre, avec le suffixe `-provisoire` quand la période contient un mois non clos ;
  - aucune mention dans les lignes du CSV : « provisoire » et la date de synchronisation sont dans le nom du fichier, à l'écran et dans l'Excel ;
  - pas de ligne de total, que beaucoup d'imports prendraient pour une écriture : les totaux restent à l'écran et dans l'Excel.
- État de la TVA collectée : par mois et par taux, base et TVA des factures, moins celles des avoirs du mois, à l'écran, en PDF et en Excel.

## 5. Écrans (kadenz-22)

- Menu : page « Comptabilité » (groupe Paiements, à côté des registres), visible avec le droit de 4.7.
- Onglet **Journal** : période (mois ou du/au), choix des journaux, aperçu des écritures, totaux débit et crédit, alertes (mode sans compte, mois non clos, hors connexion), export CSV et Excel, bouton « Clore le mois » avec confirmation.
- Onglet **TVA collectée** : tableau par mois et par taux, export PDF et Excel.
- Paramètres, rubrique **Comptabilité** : comptes de 4.3, mention « à confirmer par un comptable » et lien vers la source.
- Fiche produit : nature comptable (marchandise, service), voie B de 4.4.

## 6. Jalons

| Jalon | Contenu | Qui |
|---|---|---|
| C1 | Paramètres comptables, nature des produits, calcul des écritures et de la TVA collectée (fonction unique du poste et du serveur), clôture et refus du serveur, format CSV, migration du droit nouveau ; tests d'équilibre | kadenz-34 |
| C2 | Écrans de la section 5, traductions dans les six langues | kadenz-22 |
| C3 | Essai à l'écran sur un banc neuf, données fictives exactes (factures avec remise, avoir, règlements par trois modes, mois clos puis annulation refusée) | kadenz-22 et kadenz-34, avec l'accord de l'utilisateur pour l'écran |

## 7. Questions pour l'utilisateur

1. Logiciel comptable cible et format attendu (question ouverte 3) ; en attendant, le journal générique de 4.8.
2. Qui confirme le plan de comptes et les écritures (comptable de l'entreprise, expert-comptable) ?
3. Avoirs au compte de ventes ou au 7019 / 7069 (4.2) ; sens de « Région » (2.1).
4. Exigibilité de la TVA (à la facture ou à l'encaissement, notamment pour les services) : règle fiscale du pays, hors SYSCOHADA (question ouverte 8).
5. Le comptable veut-il voir les numéros des documents annulés (une ligne à zéro, ou une liste à part) ?
6. Réouverture d'un mois clos : permise ou non ; si oui, par l'Administrateur seul et journalisée ?
7. Pièce d'un règlement (4.2) : garder le numéro de la facture réglée, qui sert au lettrage du compte client, ou donner à chaque règlement sa propre pièce (par exemple le numéro de la facture suivi de « /R1 », « /R2 ») si le logiciel comptable cible (question 1) exige une pièce par écriture d'encaissement ?
