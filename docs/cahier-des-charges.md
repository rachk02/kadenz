# Kadenz - Cahier des charges fonctionnel et technique

| Version | Date | Changement |
|---|---|---|
| 1.0 | non datée | Version initiale (.NET 8, WPF, EF Core, SQLite), conservée hors dépôt (`docs_of_interest/`, non versionné) |
| 2.0 | 2026-11-09 | Passage en Rust (Slint, rusqlite), mono-poste avec comptes et rôles, module Paiements commun Facture et Bon de commande, modularité, corrections des incohérences de la v1 |
| 2.1 | 2026-11-09 | Tour 1 validé par l'utilisateur : socle et modules activables par clé de licence, réglages modifiables, avoir, export Excel, import CSV/Excel, licence par client, installateur et mises à jour |
| 2.2 | 2026-11-09 | Tour 2 validé : écrans et ergonomie, fiche document, bordereaux issus des factures, règlements, relances avec lettre, volet achats, sauvegarde automatique (10 copies), licence perpétuelle signée et liée au poste, préfixe A de l'avoir, mise à jour vérifiée au démarrage |
| 2.3 | 2026-11-09 | Règles PDF multi-pages validées (en-têtes et pieds répétés, report, filigranes, deux logos, export des registres), arrondi et quantités fractionnaires tranchés, orthographe du montant en lettres par réglage, ordre des jalons J1 à J10 validé |
| 2.4 | 2026-11-09 | Jalon J2 : logos de l'entreprise stockés dans la base, matrice des droits modifiable, import CSV/Excel détaillé (section 6.8), crate `kadenz-import`, feuille de route J1 à J10 reportée en section 19, question des délais par défaut ajoutée |
| 2.5 | 2026-11-09 | Questions 3 (orthographe rectifiée par défaut), 10 (délais vides par défaut) et 11 (import dans l'assistant reporté) tranchées |
| 2.6 | 2026-11-09 | Jalon J3 : périmètre livré et reports (section 19), droits des actions que la matrice ne nommait pas (section 10.1), conditions de validation (section 7.2), question 12 sur le réglage de la numérotation, identifiants jamais réutilisés (section 7.4) |
| 2.7 | 2026-11-09 | Choix de J3 validés par l'utilisateur ; question 12 tranchée : format du compteur annuel `F2026-0001` et réglage de la numérotation (sections 5.4 et 6.1) |
| 2.8 | 2026-11-09 | Interface en sept langues, langue choisie par compte, documents clients en français (section 13) ; résultat de l'essai PDF (section 12) |
| 2.9 | 2026-11-09 | krilla retenue ; données de l'entreprise, textes et couleur figés à la validation ; textes par défaut repris de l'Excel (section 12) ; changement de compteur de numérotation laissé libre, avec mise en garde |
| 2.10 | 2026-11-09 | J4 livré : aperçu et export PDF depuis l'écran Factures, textes imprimés, couleur et dossiers d'export réglables dans Paramètres (section 12, périmètre de J4) ; mise en page de droite à gauche en arabe (section 13) |
| 2.11 | 2026-11-09 | Décisions J5 : devis ou proforma converti à la validation de la facture, avoir aux lignes reprises plafonné au TTC de la facture, facture couverte par ses avoirs inchangée, entrée de menu Avoirs ; règles de refus des documents liés ; état « Payé » pour un montant dû nul (sections 6.7, 7.1, 7.7 et 8) |
| 2.12 | 2026-11-10 | J5 livré : devis, proformas et avoirs dans la fiche commune, conversion en facture, duplication et correction, documents liés, PDF des devis, proformas et avoirs (« Validité » à la place de « Échéance »), raccourcis clavier précisés (section 4.1) et création rapide d'un client (section 19, périmètre de J5) |
| 2.13 | 2026-11-10 | Décisions J6 : bordereau vierge pour un client, « Confirmer la livraison » après le visa du client, dépassement du reste à livrer et de la quantité commandée refusé, reste à livrer sans les avoirs, réception d'un bon de commande au droit « Valider un document » (sections 6.5, 7.7 et 10.1) |
| 2.14 | 2026-11-10 | Décisions J6 (suite) : confirmation de la livraison au droit « Valider un document », annulation d'une facture refusée tant qu'un de ses bordereaux n'est ni confirmé ni annulé (sections 6.5 et 10.1) |
| 2.15 | 2026-11-10 | J6 livré : bons de commande et réception, bordereaux de livraison depuis une facture ou vierges, visas et dates, confirmation de la livraison, PDF du bon de commande et du bordereau (section 19, périmètre de J6, avec les choix de mise en œuvre à confirmer) |
| 2.16 | 2026-11-10 | Choix de mise en œuvre de J6 validés par l'utilisateur (section 19, périmètre de J6) |
| 2.17 | 2026-11-10 | Décisions J7 : date d'un règlement, annulation refusée tant qu'il reste des règlements, suppression au droit « Saisir un règlement », factures sans échéance hors relances, relances sur factures validées ou envoyées, PDF des registres dès J7, lettre de relance proposée puis validée par l'utilisateur, trois entrées de menu, registre avec sous-totaux par mode (sections 8 et 10.1) |
| 2.18 | 2026-11-10 | J7 livré : règlements des factures et des bons de commande, registre des règlements, suivi fournisseurs, relances et lettre de relance, PDF du registre et de la liste des relances (section 19, périmètre de J7) ; décisions de l'utilisateur : installateur Windows avancé avant J8, pages de présentation au premier lancement (sections 4.1, 16 et 19) |
| 2.19 | 2026-11-10 | J8 lancé : règles de calcul des indicateurs du tableau de bord et graphiques dessinés par chemins Slint, choix de mise en œuvre à confirmer par l'utilisateur (section 9) |
| 2.20 | 2026-11-10 | Attribution de Slint par le badge « Made with Slint » du README du dépôt public à la place du composant AboutSlint, retiré de l'écran « À propos » à la demande de l'utilisateur (section 14) |
| 2.21 | 2026-11-10 | Règles de calcul du tableau de bord et graphiques par chemins Slint validés par l'utilisateur (section 9) ; bande verticale de couleur retirée des PDF (section 12) ; réglages d'impression spécifiés (en-tête et pied de page libres, gabarits, papier pré-imprimé, marges et calibrage) pour un nouveau jalon J8b inséré avant J9 (sections 5.4, 12, 18 et 19) |

Statut des capacités : chaque bloc est marqué **spécifié** (décidé, à implémenter), **à l'étude** (choix technique ou métier non tranché) ou **en production** (livré). À ce jour, rien n'est en production.

Nom du projet : **Kadenz**, projet personnel sans lien avec une structure existante. Le nom n'apparaît jamais sur les documents générés pour les clients : chaque installation garde l'identité de l'entreprise utilisatrice, saisie à l'installation.

---

## 1. Présentation

**Nature** : logiciel desktop Windows de gestion commerciale (devis, proforma, bons de commande, bordereaux de livraison, factures), vendu et installé indépendamment chez différentes structures clientes (PME, bureaux d'études, coopératives) en contexte Burkina Faso / UEMOA.

**Origine** : refonte d'un outil Excel/VBA existant (captures conservées hors dépôt, `docs_of_interest/`) en application native robuste, sur vraie base de données relationnelle.

**Décisions actées** :
- Langage : Rust (édition 2024).
- Interface : Slint 1.18, interface native déclarative, compilée dans l'exécutable.
- Base de données : SQLite via `rusqlite` (SQLite embarqué dans l'exécutable, fonction `bundled`), un fichier unique local, aucune configuration serveur.
- Mono-poste : une seule machine accède à la base. Plusieurs personnes peuvent utiliser ce poste à tour de rôle, chacune avec son compte et son rôle.
- Déploiement : une installation indépendante par client, jamais de base partagée entre clients.
- Migration des données Excel existantes : non retenue, on repart de zéro.
- Migrations de schéma embarquées dans l'exécutable et appliquées automatiquement au démarrage.

---

## 2. Glossaire métier

| Terme | Signification |
|---|---|
| IFU | Identifiant Financier Unique (immatriculation fiscale, Burkina Faso) |
| RCCM | Registre du Commerce et du Crédit Mobilier |
| Régime d'imposition | Régime fiscal du contribuable (ex. RSI, DCI) |
| HT | Montant Hors Taxes |
| TTC | Montant Toutes Taxes Comprises |
| TVA | Taxe sur la Valeur Ajoutée (0 % ou 18 % dans le référentiel actuel) |
| Remise | Réduction commerciale appliquée avant taxes |
| Retenue | Retenue à la source, appliquée côté achats (Bon de commande) |
| Devis | Proposition commerciale non engageante, durée de validité limitée |
| Proforma | Facture "simulée" pré-paiement, souvent utilisée pour ordre de virement |
| Facture | Document de vente définitif, seul document à valeur fiscale et comptable pleine |
| Bordereau de livraison | Document logistique sans montant, atteste la remise physique de marchandise |
| Tiers | Terme générique regroupant Client et Fournisseur dans le référentiel |
| Règlement | Paiement enregistré contre une Facture (encaissement) ou un Bon de commande (décaissement) |
| Montant dû | Montant TTC du document, retenue déduite pour un Bon de commande |

---

## 3. Architecture et modularité (spécifié)

### 3.1 Découpage du code

Le projet est un espace de travail Cargo composé de crates aux responsabilités séparées :

| Crate | Rôle | Dépend de |
|---|---|---|
| `kadenz-domaine` | Règles métier pures : types de documents, statuts et transitions, calculs, numérotation, état de paiement, droits par rôle, fiches des référentiels et leurs règles de saisie. Aucune entrée/sortie, entièrement testable. | rien |
| `kadenz-stockage` | Accès SQLite : ouverture, migrations, dépôts, compteurs de numérotation, sauvegarde, contrôle des droits du compte qui agit. | `kadenz-domaine` |
| `kadenz-import` | Lecture des fichiers CSV, Excel et OpenDocument à importer, reconnaissance des colonnes, aperçu ligne par ligne. Aucun accès à la base. | `kadenz-domaine` |
| `kadenz` (application) | Interface Slint, orchestration des cas d'usage. | les trois précédentes |
| `kadenz-pdf` (à venir) | Génération des documents PDF, voir section 12. | `kadenz-domaine` |

Les règles métier ne vivent que dans `kadenz-domaine` : l'interface et le stockage ne recalculent jamais un total ni ne décident d'une transition de statut par eux-mêmes.

### 3.2 Modules activables

| Module | Activation |
|---|---|
| Socle : référentiels (entreprise, tiers, produits), administration (comptes, rôles, sauvegarde, audit), factures | Toujours actif |
| Devis | Par clé de licence |
| Proforma | Par clé de licence |
| Bons de commande | Par clé de licence |
| Livraisons | Par clé de licence |
| Paiements (règlements Facture et Bon de commande) | Par clé de licence |
| Tableau de bord | Par clé de licence |
| Relances | Par clé de licence, exige le module Paiements |

Les modules activés sont ceux que contient la clé de licence du client (section 15). Un module désactivé disparaît de la navigation ; ses données éventuelles restent en base et réapparaissent si le module est réactivé.

Cette modularité est une modularité de configuration et de code (crates, modules activables), pas un système d'extensions chargées à l'exécution.

---

## 4. Vue d'ensemble des modules

- **Référentiels** : Entreprise, Tiers (clients et fournisseurs), Produits, Paramètres système.
- **Documents commerciaux** : Devis, Proforma, Bon de commande, Livraison, Facture.
- **Paiements** : règlements, registres, relances.
- **Pilotage** : tableau de bord, rapports.
- **Administration** : utilisateurs, droits, sauvegarde, audit (transversal aux blocs ci-dessus).

### 4.1 Écrans et ergonomie (spécifié)

- Barre latérale listant uniquement les modules actifs ; en-tête avec le logo et le nom de l'entreprise (comme l'Excel).
- Connexion au démarrage (identifiant et mot de passe), déconnexion, changement de son propre mot de passe.
- Pages de présentation au premier lancement (décision de l'utilisateur du 2026-11-10) : quatre pages qui décrivent ce que fait Kadenz (documents, paiements, sécurité des données), avec Suivant, Précédent et Passer, affichées avant l'assistant sur une base pas encore installée ; « À propos » permet de les revoir. Elles ne décrivent que des fonctions livrées.
- Assistant de premier lancement, dans l'ordre : clé de licence, fiche entreprise, compte Admin, taux et modes de paiement, import facultatif des produits et des tiers.
- Chaque type de document a deux écrans : Liste (registre) et Fiche (création, consultation), comme « Création / Liste » dans l'Excel.
- Chaque liste se filtre (numéro, tiers, période, statut, état de paiement) et se trie par colonne. Colonnes : celles de l'Excel (n°, date, tiers, montant TTC, état de paiement, mode de paiement, document), plus le statut.
- Raccourcis clavier : Ctrl+N nouveau, Ctrl+S enregistrer, Ctrl+P imprimer, Ctrl+F rechercher. Livrés en J5 dans les pages des documents de vente (Devis, Proformas, Factures, Avoirs) : Ctrl+N crée un document depuis le registre seulement, jamais depuis une fiche ouverte, pour ne pas perdre une saisie en cours ; Ctrl+S agit comme le bouton d'enregistrement affiché ; Ctrl+P exporte le PDF d'un document déjà enregistré ; Ctrl+F place le curseur dans la recherche du registre.
- Écran « À propos » accessible depuis la navigation, avec l'attribution Slint (section 14).

### 4.2 Fiche document (spécifié)

S'applique au Devis, à la Proforma, au Bon de commande, à la Facture et à l'Avoir :
- En-tête : tiers choisi dans une liste avec recherche, création rapide d'un tiers absent (selon les droits), date, libellé.
- Lignes : produit choisi par référence ou désignation, ce qui remplit la désignation et le prix ; les deux restent modifiables sur la ligne (le snapshot enregistre la valeur saisie). Ligne libre sans produit possible. Réordonner et supprimer des lignes.
- Totaux en direct : HT, remise, TVA, retenue (Bon de commande), TTC, montant en lettres.
- Actions selon le statut : enregistrer le brouillon, valider, marquer envoyé, annuler, dupliquer, convertir en facture (Devis et Proforma), créer un avoir (Facture), créer un bordereau (Facture), imprimer en PDF, exporter en Excel.
- Statut et état de paiement affichés en bandeau ; historique des changements de statut consultable sur la fiche.

---

## 5. Référentiels (spécifié)

### 5.1 Entreprise
Une seule fiche par installation, saisie via un assistant de premier lancement : nom, adresse, téléphone, email, site web, localisation, régime d'imposition, RCCM, IFU, jusqu'à 2 comptes bancaires, logo, nom des responsables (facturation, comptabilité). Aucune donnée d'entreprise n'est livrée dans le code ou dans le dépôt.

### 5.2 Tiers (clients et fournisseurs)
Fichier unique servant à la fois de fichier clients et fournisseurs : nom, adresse, téléphone, localisation, IFU, RCCM, régime d'imposition, type (Client / Fournisseur / les deux).

### 5.3 Produits et services
Catalogue unique (l'Excel avait deux tables séparées Produits/Produits4) : désignation, référence (clé unique), prix, sens d'utilisation (Vente / Achat / les deux).

Tiers et produits ont chacun une liste et une fiche. Une fiche déjà utilisée dans un document ne se supprime pas : elle se désactive (elle disparaît des choix, l'historique reste intact).

### 5.4 Paramètres système
Table clé-valeur, modifiable par l'Admin sans recompiler :

| Réglage | Contenu |
|---|---|
| Taux | Listes de taux de TVA (0 %, 18 %), de remise et de retenue (0/5/10/15 %) |
| Modes de paiement | Liste modifiable, initialisée avec ceux de l'Excel : Espèce, Chèque, Virement bancaire, Carte bancaire, Moov money, Orange money |
| Numérotation | Préfixe par type (1 à 6 lettres non accentuées, distinctes d'un type à l'autre), remise à zéro du compteur chaque mois (comme l'Excel, par défaut) ou chaque année, nombre de chiffres du rang (1 à 9, 4 par défaut) |
| Textes des documents | Titre par type (ex. « FACTURE DEFINITIVE »), phrase « Arrêté la présente ... à la somme de », libellé du signataire (ex. « LA COMPTABILITE »), mentions de pied de page |
| Apparence des PDF | Couleur principale ; en-tête et pied de page libres, gabarit, marges (section 12, réglages d'impression, jalon J8b) |
| Montant en lettres | Orthographe rectifiée de 1990 ou traditionnelle |
| Dossiers d'export | Un dossier par type de document (comme l'Excel) |
| Délais | Durée de validité des devis, délai de paiement des factures (date d'échéance calculée, modifiable sur le document) |
| Rôles | Matrice des droits par rôle (section 10.1) |

L'identité de l'entreprise (section 5.1) est éditée dans le même écran de paramètres.

---

## 6. Documents commerciaux (spécifié)

### 6.1 Règles communes

**Numérotation** : avec le compteur mensuel (par défaut), format `{Préfixe}{AAAA}-{MM}-{####}`, ex. `D2026-05-0004`, compteur par type + année + mois de la date du document ; avec le compteur annuel, format `{Préfixe}{AAAA}-{####}`, ex. `F2026-0001`, compteur par type + année de la date du document. Préfixes par défaut : D (Devis), P (Proforma), C (Bon de commande), L (Livraison), F (Facture), A (Avoir). Le compteur est incrémenté dans la transaction de validation. Au-delà du nombre de chiffres réglé (`9999` par défaut), le numéro s'allonge sans perte. Un changement de réglage vaut pour les validations suivantes ; les numéros déjà attribués ne changent pas.

**Montants** : la devise est le FCFA, sans décimale. Tous les montants sont stockés et calculés en entiers (`i64`), jamais en nombres à virgule flottante. Les pourcentages sont stockés en centièmes de point (1800 = 18 %), ce qui permet un taux comme 2,5 % sans changer le modèle. Tout arrondi passe par une fonction unique du domaine.

**Quantités** : stockées en millièmes (1000 = 1 unité), pour permettre des quantités fractionnaires (mètres, kilos) sans virgule flottante.

**Calculs** :
```
Total ligne      = arrondi(prix × quantité)
Montant HT       = Σ total ligne
Montant Remise   = arrondi(Montant HT × % Remise)
Base TVA         = Montant HT - Montant Remise
Montant TVA      = arrondi(Base TVA × % TVA)
Montant Retenue  = arrondi(Base TVA × % Retenue)   (Bon de commande uniquement)
Montant TTC      = Base TVA + Montant TVA - Montant Retenue
```

**Montant en lettres** : implémenté dans le domaine (aucune dépendance externe), en français.

**Lignes** : aucune limite de nombre de lignes (l'Excel était contraint par la mise en page : 56 lignes pour Devis/Facture, 16 pour Proforma/Bon de commande). Le générateur de PDF gère la pagination.

### 6.2 Devis (D-...)
Client, lignes produits, calcul HT/Remise/TVA/TTC, statut de workflow (section 7). Convertible en Facture.

### 6.3 Proforma (P-...)
Structure identique au Devis. Convertible en Facture.

### 6.4 Bon de commande (C-...)
Le tiers est un Fournisseur. Utilise le catalogue produits en sens "Achat". Retenue applicable. Porte un état de paiement (section 8).

### 6.5 Bordereau de livraison (L-...)
Pas de montant. Colonnes par ligne : référence, désignation, quantité commandée, quantité livrée, observations. Zones de visa client/fournisseur et dates de réception/livraison, conservées comme champs texte en V1.

Un bordereau se crée soit à partir d'une Facture (reprise des lignes et des quantités commandées, lien vers la Facture), soit vierge. Une même Facture peut recevoir plusieurs bordereaux (livraisons en plusieurs fois) ; la quantité restant à livrer par ligne tient compte des bordereaux validés précédents.

Décisions du 2026-11-10 (jalon J6) : le tiers d'un bordereau vierge est un client (type Client ou Les deux). Validé signifie bordereau émis, pas encore livré : après le visa du client, l'action « Confirmer la livraison » fait passer le bordereau en Livraison partielle ou Livraison complète, selon la quantité livrée comparée à la quantité commandée. Une quantité livrée qui dépasse le reste à livrer de la facture est refusée, avec un message demandant de la corriger. Le reste à livrer suit les seules lignes de la facture : un avoir diminue le montant dû, pas le reste à livrer. Une facture qui a un bordereau en Brouillon ou Validé (pas encore confirmé) ne s'annule pas : il faut d'abord annuler ce bordereau ou confirmer sa livraison ; une fois ses bordereaux confirmés ou annulés, la facture s'annule normalement.

### 6.6 Facture (F-...)
Structure identique au Devis. Document définitif, porte un état de paiement (section 8). Champ date d'échéance, calculé à partir du délai de paiement paramétré.

### 6.7 Avoir (A-...)
Document lié à une Facture validée, qui la corrige ou l'annule en tout ou partie (section 7.2). Préfixe A par défaut, même format et même règle de compteur que les autres documents (ex. `A2026-05-0001`, ou `A2026-0001` avec le compteur annuel).

Règles (décisions de l'utilisateur du 2026-11-09, spécifié) :
- Un avoir se crée seulement depuis la fiche d'une Facture Validée ou Envoyée ; il a son entrée de menu « Avoirs » et son registre.
- Il reprend les lignes de la facture, modifiables : on retire des lignes ou baisse quantités et prix pour un avoir partiel. Il garde le client et les taux (remise, TVA) de la facture, sans quoi son montant serait incohérent avec elle.
- À sa validation, son TTC ne peut pas dépasser le reste à créditer de la facture (TTC de la facture moins les avoirs déjà validés), et la facture doit être encore Validée ou Envoyée.
- Une facture entièrement couverte par ses avoirs garde son statut ; son montant dû tombe à zéro.
- Une facture qui a un avoir validé ne s'annule pas : il faut d'abord annuler l'avoir, ce qui rend son montant à la facture.
- Le PDF de l'avoir porte la référence de la facture d'origine.

### 6.8 Exports et imports (spécifié)
- Export Excel des registres (listes filtrées) et des documents.
- Import CSV ou Excel du catalogue produits et des tiers, avec aperçu et rapport d'erreurs avant enregistrement, proposé au premier lancement et dans les référentiels.
  - Formats : CSV (UTF-8 ou Windows-1252, séparateur point-virgule, virgule ou tabulation détecté), Excel (xlsx, xls) et OpenDocument (ods, première feuille remplie).
  - En-têtes reconnus sans tenir compte de la casse ni des accents, dont ceux de l'outil Excel d'origine (DÉSIGNATION, RÉFÉRENCE, PRIX ; NOM, ADRESSE/BOÎTE POSTALE, TELEPHONE, Localisation, IFU). Une colonne non reconnue est signalée et n'est pas importée.
  - Colonnes obligatoires : Nom pour les tiers ; Désignation, Référence et Prix pour les produits. Le type (tiers) ou le sens (produits) absent du fichier est choisi à l'écran.
  - Chaque ligne est contrôlée par les mêmes règles que la saisie : prix entier en FCFA (un prix décimal est une erreur, jamais arrondi). Un doublon (même nom de tiers, même référence de produit, en base ou plus haut dans le fichier) est signalé et ignoré.
  - Seules les lignes valides sont enregistrées, en une seule transaction ; le bilan indique les fiches ajoutées.

---

## 7. Statuts et modifications de saisie (spécifié)

### 7.1 Statuts par type de document

| Type | Cycle de statuts |
|---|---|
| Devis / Proforma | Brouillon → Validé → Envoyé → Converti (en Facture), ou → Annulé à tout moment avant conversion |
| Bon de commande | Brouillon → Validé → Envoyé → Réception partielle → Réception complète, ou → Annulé avant toute réception |
| Livraison | Brouillon → Validé → Livraison partielle → Livraison complète, ou → Annulé avant toute livraison |
| Facture | Brouillon → Validé → Envoyé, ou → Annulé |
| Avoir | Même cycle que la Facture. Montants stockés en positif ; le montant dû de la Facture d'origine est diminué des avoirs validés (section 6.7) |

L'état de paiement (Facture et Bon de commande) est un axe séparé du statut de workflow (section 8).

### 7.2 Règles d'édition selon le statut

- **Brouillon** : entièrement modifiable (tiers, lignes, quantités, remise, TVA, retenue). Recalcul des totaux à chaque changement. Aucun numéro attribué.
- **Validé** : le numéro est attribué à ce moment précis, jamais avant, pour éviter les trous de séquence dus aux brouillons abandonnés. Lignes et montants figés. Seuls les champs administratifs restent modifiables (mode de paiement, observations).
- **Conditions de validation** : un tiers choisi et au moins une ligne ; chaque ligne a une désignation, un prix entier positif ou nul et une quantité positive. Les totaux sont recalculés dans la transaction de validation, et le snapshot du tiers est repris une dernière fois de sa fiche avant d'être figé.
- **Envoyé** : comme Validé, avec horodatage de la date d'envoi.
- **Correction d'un document Validé ou Envoyé** : jamais d'édition directe des lignes ou montants.
  - Pour une Facture : émission d'un avoir lié (nouveau document référençant l'original), conformément aux usages comptables.
  - Pour un Devis ou une Proforma : duplication en nouveau document avec référence croisée vers l'original, qui passe en Annulé.
- **Annulé** : lecture seule totale. Numéro conservé, jamais réutilisé, visible dans les registres avec mention explicite.

### 7.3 Suppression

- Suppression physique autorisée uniquement sur les Brouillons jamais validés.
- Au-delà, toute annulation est logique (passage en Annulé), pour préserver la séquence de numérotation.

### 7.4 Historique et audit

- Toute transition de statut est journalisée : utilisateur, date et heure, ancien statut, nouveau statut.
- La table d'audit est en ajout seul : des déclencheurs SQLite refusent toute modification et toute suppression, la règle ne repose donc pas sur le seul code applicatif.
- Granularité minimale : qui a créé, validé, annulé, et quand. Pas d'audit frappe par frappe en Brouillon.
- Les identifiants des documents, tiers, produits et règlements ne sont jamais réutilisés (`AUTOINCREMENT`) : l'historique d'une ligne supprimée ne se mêle pas à celui d'une nouvelle.

### 7.5 Sessions et utilisateurs

- Mono-poste : pas de verrouillage optimiste ni pessimiste entre postes (le `RowVersion` de la v1 est retiré).
- Chaque session est ouverte par un compte utilisateur ; toutes les actions journalisées portent ce compte.

### 7.6 Snapshot des données tierces

- À la création d'un document, les informations du Tiers (nom, adresse, IFU, RCCM, régime fiscal) et des lignes Produit (désignation, prix) sont copiées dans le document.
- Une modification ultérieure de la fiche Tiers ou du catalogue n'affecte jamais un document existant.
- Le lien vers le Tiers ou le Produit d'origine est conservé pour la navigation ; l'affichage utilise toujours le snapshot.

### 7.7 Conversion entre documents

- **Devis/Proforma → Facture** : copie figée des lignes dans une Facture indépendante, créée en brouillon depuis un devis ou une proforma Validé ou Envoyé. Le document source passe en Converti à la validation de la facture (décision du 2026-11-09), dans la même transaction, et référence la Facture créée. Tant que la facture est en brouillon, la source indique une facture en préparation : une seconde conversion, l'annulation et la correction de la source sont refusées ; si le brouillon est supprimé, la source redevient convertible. Une facture issue d'une conversion puis annulée laisse la source en Converti : on refait un devis par duplication.
- **Duplication** : copie d'un document en nouveau brouillon du même type (date du jour, échéance recalculée), avec référence vers l'original. En correction d'un devis ou d'une proforma Validé ou Envoyé, l'original passe en Annulé dans la même transaction (section 7.2) ; une facture se corrige par un avoir ; un avoir ne se duplique pas.
- **Facture → Bordereau(x) de livraison** : voir section 6.5.
- **Facture → Avoir** : voir section 6.7.
- **Bon de commande → réception** : pas de nouveau document. Les quantités reçues sont mises à jour ligne par ligne sur le Bon de commande (Non reçu / Partiel / Complet), ce qui permet des livraisons fournisseur en plusieurs fois. Une quantité reçue qui dépasse la quantité commandée est refusée (décision du 2026-11-10) ; l'enregistrement de la réception suit le droit « Valider un document » (section 10.1).

---

## 8. Paiements (spécifié)

Le module Paiements s'applique à deux types de documents :
- la **Facture** : créance client, les règlements sont des encaissements ;
- le **Bon de commande** : dette fournisseur, les règlements sont des décaissements (l'Excel affichait déjà un état et un mode de paiement sur la liste des bons de commande).

Règles communes :
- Un document peut recevoir plusieurs règlements (paiements échelonnés), seulement à partir du statut Validé.
- État de paiement calculé à partir des règlements, jamais saisi : Impayé (aucun règlement), Partiel (0 < payé < montant dû), Payé (payé = montant dû). Le montant dû d'une facture est son TTC moins ses avoirs validés : une facture entièrement couverte par ses avoirs est donc Payé sans règlement, et un avoir validé après un paiement complet laisse un trop-perçu (remboursement hors périmètre V1).
- Un règlement ne peut pas porter le total payé au-delà du montant dû.
- Saisie depuis la fiche du document : date, montant, mode, référence ; le montant proposé par défaut est le reste à payer.
- Un règlement saisi par erreur peut être supprimé ; la suppression est tracée dans le journal d'audit avec la valeur supprimée.
- Registre des encaissements et des décaissements, par période et par mode de paiement.
- Suivi fournisseurs : liste des Bons de commande non payés.

Relances (module Relances) :
- Liste des Factures échues non payées : jours de retard, reste dû ; export Excel.
- Impression d'une lettre de relance par facture (PDF), à l'identité de l'entreprise.
- Envoi par email hors périmètre V1.

Décisions du 2026-11-10 (jalon J7) : la date d'un règlement est comprise entre la date du document et le jour de la saisie ; une facture ou un bon de commande qui a des règlements ne s'annule pas, il faut d'abord supprimer ses règlements ; supprimer un règlement suit le droit « Saisir un règlement ». Une facture est échue le lendemain de sa date d'échéance ; une facture sans date d'échéance n'est jamais échue et n'apparaît pas dans les relances ; une facture Validée ou Envoyée est relançable. Le registre des règlements et la liste des relances s'exportent en PDF dès J7, l'export Excel de la liste des relances suit en J10 avec les autres exports. Le texte de la lettre de relance est proposé puis validé par l'utilisateur. Navigation : trois entrées après Avoirs, « Règlements » (registre des encaissements et des décaissements, total de la sélection et sous-totaux par mode de paiement), « Suivi fournisseurs » et « Relances ».

---

## 9. Pilotage (spécifié)

Indicateurs recalculés en direct par requête sur les tables Document, LigneDocument et Reglement :
- Ventes (repris de l'Excel) : CA HT, TVA, CA TTC, total payé, total impayé, CA par état de paiement, CA par mode de paiement, tendance des ventes, top 5 clients, 4 dernières factures.
- Achats : total des Bons de commande, montants payés et restant dus aux fournisseurs.
- Filtre par année ou par période (comme le filtre « DATE Années » de l'Excel).

Slint n'a pas de composant graphique prêt à l'emploi : les graphiques (secteurs, courbes, barres) sont dessinés par l'application. Technique retenue au lancement de J8 et validée par l'utilisateur le 2026-11-10 : des chemins Slint calculés en Rust à partir des indicateurs (rendu net à toutes les tailles, thème et lecture de droite à gauche suivis, survol possible) ; une image générée côté Rust ne servirait que si le tableau de bord devait sortir en PDF, ce que cette section ne demande pas.

Règles de calcul retenues au lancement de J8, validées par l'utilisateur le 2026-11-10 :
- une facture ou un bon de commande compte s'il est en vigueur (ni Brouillon ni Annulé) et daté dans la période, bornes incluses ;
- les avoirs validés sont déduits de leur facture, quelle que soit leur date : CA HT, TVA et CA TTC sont nets de leurs avoirs, et total payé + total impayé = CA TTC ;
- CA HT = HT après remise (base de la TVA) ; CA TTC = montant dû (retenue déduite) ;
- le payé d'un document est plafonné à son montant dû (un trop-perçu ne compte pas), l'impayé est la somme des restes à payer ;
- CA par état de paiement : montant dû des factures groupé en Impayé, Partiel et Payé ;
- CA par mode de paiement : règlements des factures de la période, par mode (le registre des règlements reste la vue par date de règlement) ;
- tendance : un point par mois de la période, mois vides à zéro ;
- top 5 clients par CA TTC, à égalité par nom ; 4 dernières factures de la période, de la plus récente à la plus ancienne ;
- achats : total TTC des bons de commande de la période, payé et restant dû ;
- filtre par année : années qui ont au moins une facture ou un bon de commande en vigueur.

---

## 10. Administration (spécifié)

### 10.1 Rôles et permissions

Quatre rôles livrés par défaut avec la matrice ci-dessous. L'Admin peut modifier la matrice (réglage « Rôles », section 5.4) ; le droit « Gérer les utilisateurs et droits » reste toujours acquis au rôle Admin, pour qu'aucune installation ne se retrouve sans administrateur.

| Action | Admin | Comptable | Commercial | Consultation |
|---|---|---|---|---|
| Créer/modifier un brouillon (Devis, Proforma, BC, Livraison) | oui | oui | oui | non |
| Créer/modifier un brouillon Facture | oui | oui | non | non |
| Valider un document | oui | oui | non | non |
| Annuler un document validé | oui | oui | non | non |
| Saisir un règlement | oui | oui | non | non |
| Gérer les référentiels (clients, produits) | oui | oui | ajout seul | non |
| Gérer les utilisateurs et droits | oui | non | non | non |
| Consulter tableau de bord / registres | oui | oui | oui | oui |
| Sauvegarder / exporter la base | oui | non | non | non |

Actions que la matrice ne nomme pas (choix du 2026-11-09, jalon J3) : supprimer ou annuler un brouillon et modifier les champs administratifs d'un document validé (mode de paiement, observations) suivent le droit de brouillon du type de document ; marquer un document envoyé suit « Valider un document ». Enregistrer la réception d'un bon de commande suit aussi « Valider un document » (décision de l'utilisateur du 2026-11-10, jalon J6). Confirmer la livraison d'un bordereau suit aussi « Valider un document » (même décision). Supprimer un règlement saisi par erreur suit « Saisir un règlement » (décision de l'utilisateur du 2026-11-10, jalon J7).

### 10.2 Comptes
Mots de passe stockés hachés avec Argon2id (jamais en clair). Le premier compte Admin est créé par l'assistant de premier lancement. L'Admin crée et désactive les comptes et réinitialise les mots de passe.

### 10.3 Sauvegarde
- Copie horodatée de la base dans un dossier configurable, par l'API de sauvegarde en ligne de SQLite (jamais une copie brute du fichier, qui peut produire une sauvegarde corrompue quand la base est ouverte).
- Sauvegarde manuelle à la demande, et sauvegarde automatique à chaque fermeture de l'application ; seules les 10 dernières sauvegardes automatiques sont conservées.
- Restauration depuis une sauvegarde choisie, après confirmation ; la base courante est elle-même sauvegardée juste avant.
- Pas de sauvegarde cloud en V1.

### 10.4 Journal d'audit
Consultable et filtrable par l'Admin (entité, utilisateur, période, action).

---

## 11. Modèle de données (spécifié)

Types SQLite : `INTEGER` (identifiants, montants, pourcentages, quantités, booléens), `TEXT` (chaînes, dates ISO 8601, JSON). Les valeurs structurées sont du JSON stocké en `TEXT`.

| Entité | Champs clés | Notes |
|---|---|---|
| **Entreprise** | Id, Nom, Adresse, Telephone, Email, SiteWeb, Localisation, RegimeImposition, RCCM, IFU, Banque1, Banque2, Logo, Logo2, ResponsableFacturation, ResponsableComptabilite | Une seule ligne (contrainte Id = 1). Les deux logos sont stockés dans la base (BLOB PNG ou JPEG, 2 Mo au plus), la base reste ainsi autonome pour la sauvegarde |
| **Utilisateur** | Id, NomComplet, Identifiant (unique), MotDePasseHache, Role, Actif, DateCreation | |
| **Tiers** | Id, Nom, Adresse, Telephone, Localisation, IFU, RCCM, RegimeImposition, Type, Actif | |
| **Produit** | Id, Designation, Reference (unique), Prix, Sens, Actif | |
| **Document** | Id, Type, Numero (unique par type, nul en Brouillon), Date, TiersId, snapshots Tiers (Nom, Adresse, IFU, RCCM, Regime), Statut, Libelle, MontantHT, MontantRemise, MontantBaseTVA, MontantTVA, MontantRetenue, MontantTTC, PourcentageRemise, PourcentageTVA, PourcentageRetenue, ModePaiement, CheminPdf, DocumentSourceId, DateEcheance, DateCreation, DateValidation, DateEnvoi, UtilisateurCreationId, UtilisateurValidationId | Pas de colonne d'état de paiement : il est calculé |
| **LigneDocument** | Id, DocumentId, ProduitId (nullable), ReferenceSnapshot, DesignationSnapshot, PrixSnapshot, Quantite, QuantiteLivree, Total, Observations, Ordre | |
| **Reglement** | Id, DocumentId (Facture ou BC), Date, Montant, ModePaiement, Reference, UtilisateurId | |
| **CompteurNumerotation** | TypeDocument, Annee, Mois, DernierNumero | Clé (TypeDocument, Annee, Mois) |
| **ParametreSysteme** | Cle, Valeur (JSON en TEXT) | |
| **AuditLog** | Id, Entite, EntiteId, Action, AncienneValeur (JSON), NouvelleValeur (JSON), UtilisateurId, DateAction | Ajout seul, protégé par déclencheurs |

---

## 12. Génération des documents PDF (spécifié)

Exigences (spécifié) :
- Un seul moteur paramétré par un modèle par type de document.
- Contenu : en-tête entreprise (logo, coordonnées, RCCM/IFU), identification du tiers, tableau des lignes, totaux, montant en lettres, pied de page (comptes bancaires, signataires).
- Mise en page reprise de l'Excel : logo, bloc entreprise, bloc tiers, tableau, totaux, montant en lettres, signataire, pied de page avec adresse, RCCM, IFU, comptes bancaires, site et email. Couleur principale paramétrable. La bande verticale de couleur à gauche de l'Excel est retirée de tous les PDF (décision de l'utilisateur du 2026-11-10 : aucune barre d'accent latérale, ni à l'écran ni à l'impression) ; marges gauche et droite égales.
- Documents exportés : Devis, Proforma, Bon de commande, Bordereau de livraison, Facture, Avoir, lettre de relance. Les registres (listes filtrées, encaissements, relances) s'exportent aussi en PDF, en plus de l'export Excel.
- Deux logos : un logo principal et un logo secondaire facultatif (certification, partenaire), comme les deux chemins de logo du réglage Excel.
- Format A4 portrait.

Règles multi-pages (spécifié) :
- Chaque page porte un en-tête : logo(s), bloc entreprise, type et numéro du document, date. Le bloc tiers et le libellé ne figurent qu'en page 1.
- L'en-tête du tableau des lignes se répète en haut de chaque page ; une ligne n'est jamais coupée entre deux pages.
- Pied de page sur chaque page : adresse, RCCM, IFU, comptes bancaires, site, email, et numérotation « Page n/N ».
- Sous-total « à reporter » en bas de chaque page sauf la dernière, et « report » en haut de la page suivante.
- Totaux, montant en lettres et signataire uniquement sur la dernière page ; s'ils ne tiennent pas sous les dernières lignes, ils passent ensemble sur une page supplémentaire, jamais coupés.
- Filigrane « ANNULÉ » sur un document annulé ; filigrane « BROUILLON » sur l'aperçu d'un brouillon (le PDF définitif reste réservé aux documents validés).
- Le PDF n'est généré qu'à partir d'un document Validé.
- Réimpression identique à l'original (décision du 2026-11-09) : à la validation, les données de l'entreprise (coordonnées, RCCM, IFU, banques, logos), les textes du document et la couleur principale sont figés avec le document, comme le snapshot du tiers ; une modification ultérieure de la fiche entreprise ou des réglages ne change pas les documents déjà validés.
- Textes par défaut, repris de l'outil Excel d'origine : titre égal au nom du type en majuscules (« FACTURE », « BON DE COMMANDE »...), libellé « Client » ou « Fournisseur », phrase « Arrêté la présente facture à la somme de <montant en lettres> francs (<montant en chiffres>) FCFA. », signataire « LA COMPTABILITE » suivi du nom du responsable. Tous réglables (section 5.4).
- Pagination automatique.

Réglages d'impression (spécifié, jalon J8b, décision de l'utilisateur du 2026-11-10) :
- En-tête et pied de page ne sont plus figés par le programme : ce sont des zones de contenu libre, pensées comme une signature de mail, composées par l'Admin et copiables. Une zone vide n'imprime rien et le document s'adapte.
- Contenu d'une zone : texte libre multiligne avec un mini-markup restreint (gras, italique, ligne centrée ; l'italique est permise dans ce contenu des documents des clients, par exception à la règle générale de l'interface) et des variables `{...}` insérées depuis une liste déroulante : `{nom}`, `{rccm}`, `{ifu}`, `{banque1}`, `{logo}`... `{logo}` est une variable visuelle : le logo fait partie du contenu d'une zone. Le markup est analysé en données structurées (paragraphes, passages, alignement), jamais stocké tel quel ; aucun moteur HTML : le moteur PDF de Kadenz reste maître du rendu.
- Gabarits A4 codés, choisis et non redessinés champ par champ, avec aperçu en direct sur un document d'exemple : Classique (bande en haut, tableau large), Compact (colonne resserrée), Papier pré-imprimé, et la variante « en-tête répété » sur les pages suivantes. Le gabarit « Bande latérale » de la réflexion d'origine tombe avec la décision de retirer la bande verticale (question 14).
- Papier pré-imprimé : ni bande, ni logo, ni en-tête, ni pied de page ; seul le corps du document s'imprime (tableau, totaux, montants, signataire, visas), calé dans la zone imprimable. Le contenu des zones reste dans les réglages, ce gabarit l'ignore.
- Marges haut, bas, gauche et droite en millimètres ; bouton « Tester l'impression » qui imprime une feuille de calibrage (cadre, croix aux angles) pour les ajuster au papier spécial et à l'imprimante.
- Droits : choix du gabarit, marges, contenu des zones et variantes par type au droit « Gérer les paramètres » (Admin par défaut, délégable par la matrice) ; les autres rôles impriment sans rien configurer.
- Instantané : contenu des zones, gabarit et marges sont figés à la validation avec les données de l'entreprise, les logos et les textes ; les variables sont résolues sur les valeurs figées, la réimpression reste identique. Un brouillon s'imprime en aperçu avec les valeurs du moment.
- Aperçu en direct : le PDF réel est rastérisé en image pour l'écran des Paramètres (essai de hayro, moteur de rendu PDF en Rust pur, au début de J8b ; objectif inférieur à 100 ms par page à 600 px de large, sinon bouton « Actualiser l'aperçu » ; repli sur l'ouverture du PDF dans la visionneuse de Windows si le rendu n'est pas fidèle).

Choix technique (spécifié) : c'est la partie la plus délicate en Rust pur (modèles, pagination, logo, polices). La bibliothèque sera choisie après un essai sur une page de facture complète.

Essai du 2026-11-09 (crate `kadenz-pdf`, données fictives) : bibliothèque krilla 0.8.2 (licence MIT ou Apache-2.0, Rust pur, sans dépendance système), polices Inter et Manrope embarquées et sous-ensemblées, logos PNG ou JPEG lus depuis des octets (deux logos vérifiés, répétés sur chaque page, transparence des PNG respectée). Le texte est façonné une fois (rustybuzz) pour être mesuré puis tracé à l'identique, ce qui rend la pagination exacte. Facture fictive de 46 lignes sur 3 pages A4 : en-tête répété, bloc tiers en page 1, en-tête du tableau répété, lignes jamais coupées, « À reporter » et « Report », totaux, montant en lettres et signataire ensemble sur la dernière page, pied de page avec « Page n/N », filigranes « BROUILLON » et « ANNULÉ » ; environ 30 Ko et 0,1 s par document en mode debug. Choix validé par l'utilisateur le 2026-11-09 : krilla est retenue.

Livré au jalon J4 : depuis la fiche d'une facture, « Aperçu PDF » pour un brouillon (filigrane « BROUILLON », valeurs du moment) et « Exporter en PDF » pour un document validé (instantané figé à la validation, filigrane « ANNULÉ » s'il est annulé). Le nom de fichier proposé est le numéro du document ; la boîte d'enregistrement s'ouvre dans le dossier d'export du type s'il est réglé, puis le PDF s'ouvre dans la visionneuse par défaut. Un document validé avant la migration 0003 (bases d'essai antérieures à la v0.1.0) n'a pas d'instantané et s'imprime avec les valeurs du moment.

---

## 13. Exigences non fonctionnelles (spécifié)

- Windows 10/11 64 bits, fonctionnement 100 % local, exécutable natif. Seule exception : la vérification facultative de mise à jour (section 16), qui échoue sans gêne hors connexion.
- Interface en sept langues : français (langue source), anglais, arabe, japonais, chinois, turc et espagnol (décision du 2026-11-09). La langue se choisit par compte, dans Mon compte ; l'interface s'affiche en français avant la connexion et revient au français à la déconnexion. Traductions rédigées sans relecture par un locuteur natif, choix assumé de l'utilisateur. Mise en page de droite à gauche en arabe : menu à droite, textes alignés à droite, rangées et colonnes des tableaux inversées (tri compris), montants alignés à gauche, bascule immédiate au changement de langue. Limites connues : les cases à cocher, listes déroulantes et onglets gardent leur dessin interne de gauche à droite ; le texte des cellules de tableau reste aligné à gauche ; les documents remis aux clients restent en français. Spécifié.
- Documents remis aux clients (PDF, montant en lettres) en français uniquement pour l'instant.
- Devise FCFA uniquement (V1).
- Base stockée dans le dossier de données de l'utilisateur Windows (`%APPDATA%`).
- Migrations appliquées automatiquement au démarrage.
- Objectifs chiffrés de performance (temps de démarrage, temps d'ouverture d'un registre) : à définir.

---

## 14. Licences (spécifié)

- Slint est utilisé sous la licence Slint Royalty-free 2.0, gratuite pour une application desktop à condition d'afficher le composant `AboutSlint` dans un écran ou dialogue "À propos" accessible depuis le menu principal, ou d'afficher le badge Slint sur une page web publique, de préférence celle du téléchargement. Retenu (décision de l'utilisateur du 2026-11-10, qui a fait retirer `AboutSlint` de l'écran « À propos ») : badge « Made with Slint » en tête du README du dépôt public, d'où se téléchargent les versions ; l'écran « À propos » cite Slint et sa licence parmi les composants tiers. Cette attribution ne vaut que tant que le dépôt reste public : s'il devient privé, il faut remettre `AboutSlint` ou prendre la licence commerciale. Une licence commerciale payante existe si aucune attribution n'est souhaitée.
- SQLite est dans le domaine public ; `rusqlite` est sous licence MIT.

---

## 15. Licence client et activation (spécifié)

- Chaque client reçoit une clé de licence fournie par l'éditeur. Elle contient le nom du client, l'identifiant du poste et la liste des modules activés (section 3.2).
- Licence perpétuelle : pas de date d'expiration.
- La clé est signée numériquement par l'éditeur ; l'application ne contient que la clé publique et vérifie la signature hors ligne. Une clé modifiée ou forgée est refusée.
- Liaison au poste : l'application affiche un identifiant de poste, que le client transmet à l'éditeur ; la clé n'est valide que sur ce poste. Un changement de PC demande une nouvelle clé.
- La clé est saisie dans l'assistant de premier lancement et peut être remplacée plus tard (ajout de modules, changement de poste).
- Outil éditeur séparé, jamais distribué aux clients, pour générer les clés ; la clé privée de signature reste chez l'éditeur et n'est jamais versionnée.
- À l'étude : algorithme de signature, composition de l'identifiant de poste (assez stable pour survivre aux mises à jour Windows), comportement si la clé devient invalide (proposition : lecture seule, sans perte de données).

## 16. Installation et mises à jour (spécifié, forme à l'étude)

- Installateur Windows classique pour le déploiement chez le client : KadenzSetup.exe (assistant d'installation, raccourcis du menu Démarrer et du bureau, désinstallation), publié avec chaque release en plus de kadenz.exe. Avancé après J7, avant J8 (décision de l'utilisateur du 2026-11-10) ; la vérification des mises à jour reste au jalon J10.
- Au démarrage, l'application vérifie s'il existe une version plus récente parmi les releases GitHub et propose de l'installer ; l'utilisateur confirme. Hors connexion, la vérification échoue sans message bloquant.
- Technologie d'installateur : WiX (paquet MSI), outil déjà présent sur le poste de développement. À l'étude : signature du code de l'exécutable et de l'installateur, qui demande un certificat et l'accord de l'utilisateur.

## 17. Hors périmètre V1

- Gestion de stock réelle (entrées/sorties, valorisation)
- Envoi automatique par email
- Signature électronique
- Export comptable SYSCOHADA (journal des ventes, état de TVA collectée)
- Multi-devise
- Multi-poste en réseau

---

## 18. Questions ouvertes

1. **Mois du numéro** : tranchée le 2026-11-09, mois (et année) de la date du document.
2. **Arrondi** : tranchée le 2026-11-09, arrondi au franc le plus proche, la moitié vers le haut.
3. **Montant en lettres** : tranchée le 2026-11-09, orthographe rectifiée de 1990 (traits d'union partout) par défaut ; l'orthographe traditionnelle reste disponible par réglage. L'Excel mélangeait les deux usages ("Soixante dix mille sept cent quarante", "Cent-cinquante mille").
4. **Modularité commerciale** : tranchée le 2026-11-09, modules activés par clé de licence (sections 3.2 et 15).
5. **Avoir** : tranchée le 2026-11-09, préfixe A (section 6.7).
6. **Conversion** : tranchée le 2026-11-09, conversion possible dès Validé.
7. **Quantités fractionnaires** : tranchée le 2026-11-09, oui (quantités en millièmes).
8. **Distribution** : technologie d'installateur et signature du code (à l'étude, section 16).
9. **Écrans non vus** : aucune capture des formulaires Devis et Proforma ni des listes Factures, Livraisons, Proforma et Devis ; leur contenu est supposé identique à la Facture et à la liste des bons de commande.
10. **Délais par défaut** : tranchée le 2026-11-09, la durée de validité des devis et le délai de paiement des factures restent vides par défaut (réglages de la section 5.4) ; aucune date d'échéance n'est calculée tant que l'entreprise ne les renseigne pas.
11. **Import dans l'assistant** : tranchée le 2026-11-09, l'étape d'import des tiers et des produits dans l'assistant de premier lancement est reportée ; l'import reste disponible depuis les écrans Tiers et Produits.
12. **Réglage de la numérotation** : tranchée le 2026-11-09, un numéro à compteur annuel s'écrit `{Préfixe}{AAAA}-{rang}`, ex. `F2026-0001` (section 6.1) ; le réglage complet de la section 5.4 est livré.
13. **Gabarits d'impression** (à l'étude, J8b) : nombre de gabarits au lancement et leurs variantes finales (logo secondaire, dense ou aéré).
14. **Gabarit « Bande latérale »** (à l'étude, J8b) : écarté par la décision du 2026-11-10 de retirer la bande verticale des PDF, sauf avis contraire de l'utilisateur.
15. **Marges et contenu d'impression** (à l'étude, J8b) : marges globales ou par gabarit ; un contenu d'en-tête et de pied commun à tous les documents ou des variantes par type.
16. **Variables d'impression** (à l'étude, J8b) : champs de l'entreprise et logo seulement, ou aussi champs du document (numéro, date, montant TTC) dans le pied de page.

---

## 19. Feuille de route (validée le 2026-11-09)

| Jalon | Contenu |
|---|---|
| J1 | Socle : avoir et échéance dans le modèle de données, montant en lettres, comptes et connexion, assistant de premier lancement |
| J2 | Référentiels : entreprise, tiers, produits, paramètres, import CSV/Excel |
| J3 | Factures : liste, fiche, validation, numérotation, audit |
| J4 | PDF : essai d'une bibliothèque, puis le moteur de mise en page ; publication de la release v0.1.0 |
| J5 | Devis, proforma, conversion en facture, avoir |
| J6 | Bons de commande et réception, bordereaux de livraison |
| J7 | Paiements, registres, relances et lettre de relance |
| J8 | Tableau de bord, ventes et achats |
| J8b | Impression : en-tête et pied de page libres, gabarits, papier pré-imprimé, marges et calibrage (inséré le 2026-11-10) |
| J9 | Licence : outil éditeur, vérification, activation des modules |
| J10 | Mises à jour, export Excel (l'installateur est avancé après J7, décision du 2026-11-10) |

**Périmètre de J3 (Factures).** Livré : registre filtrable (numéro, client, libellé, statut, état de paiement, période) et triable ; fiche avec client choisi dans une liste filtrable, date, échéance, libellé, lignes produit ou libres modifiables, réordonnées et supprimées, totaux et montant en lettres en direct ; enregistrement du brouillon, validation numérotée, marquage envoyé, annulation, suppression du brouillon, mode de paiement et observations après validation ; historique des changements de statut ; fiche en lecture seule pour les rôles sans droit. Reporté : PDF (J4) ; avoir et duplication (J5) ; bordereau depuis une facture (J6) ; règlements (J7, l'état de paiement est déjà calculé) ; export Excel (J10) ; création rapide d'un tiers depuis la fiche et raccourcis clavier (avec la fiche commune à tous les documents, J5). Le réglage de la numérotation (question 12) est livré à la suite de J3, avant la v0.1.0.

**Périmètre de J4 (PDF).** Livré : crate `kadenz-pdf` (krilla) avec pagination multi-pages testée ; impression figée à la validation (données de l'entreprise, logos, textes, couleur, orthographe du montant en lettres) ; aperçu et export PDF depuis la fiche d'une facture ; dans Paramètres, par type de document : titre imprimé, phrase avant le montant en lettres, signataire, dossier d'export, avec retour aux textes par défaut ; couleur principale des documents. Reporté : PDF des autres types de documents (avec leurs écrans, J5 et J6) ; mentions de pied de page libres et position des logos (section 5.4) ; PDF des registres et lettre de relance (avec les écrans concernés) ; enregistrement du chemin du PDF exporté.

**Périmètre de J5 (Devis, proforma, conversion, avoir).** Livré : registre et fiche communs aux Devis, Proformas, Factures et Avoirs (entrée de menu Avoirs, sans création directe), textes accordés au type, validité des devis proposée comme échéance ; conversion d'un devis ou d'une proforma en facture brouillon, la source passant en Converti à la validation de la facture (« Facture en préparation » d'ici là) ; duplication simple et correction d'un devis ou d'une proforma (l'original est annulé), refusées pour un avoir ; avoir créé depuis une facture validée, lignes reprises et modifiables, client, remise et TVA de la facture verrouillés, total des avoirs validés plafonné au TTC de la facture ; montant dû et état de paiement diminués des avoirs validés ; refus d'annuler une facture qui a un avoir validé ou un devis dont la facture est en préparation ; section « Documents liés » de la fiche ; le brouillon créé s'ouvre aussitôt dans la page de son type ; historique traduit, numéros compris ; PDF des devis, proformas et avoirs (facture d'origine sous la date de l'avoir, « Validité » sur un devis ou une proforma) ; raccourcis Ctrl+N (depuis le registre), Ctrl+S, Ctrl+P et Ctrl+F ; création rapide d'un client depuis la fiche, selon le droit d'ajouter un tiers. Reporté : bons de commande et bordereaux de livraison, dont le bordereau issu d'une facture (J6) ; règlements (J7). Comme en J4, les PDF restent en français quelle que soit la langue de l'interface (section 13).

**Périmètre de J6 (Bons de commande et réception, bordereaux de livraison).** Livré : bon de commande dans la fiche commune (fournisseur choisi ou créé depuis la fiche, produits du sens Achat, retenue), statuts de réception dans le filtre du registre ; réception ligne par ligne d'un bon de commande Envoyé ou en Réception partielle (colonne « Reçue », bouton « Enregistrer la réception »), statut Réception partielle ou complète calculé d'après les quantités, quantité reçue au-delà de la commandée refusée ; bordereau créé depuis une facture validée ou envoyée (client et lignes repris et verrouillés, reste à livrer par ligne) ou vierge (client seulement), registre et fiche sans montants, visas et dates en texte libre, action « Confirmer la livraison », refus d'annuler une facture dont un bordereau n'est ni confirmé ni annulé ; documents liés et historique (réception, confirmation, modification des visas) ; PDF du bon de commande (fournisseur, retenue) et du bordereau (quantités commandée et livrée, facture d'origine sous la date, zones de visa et dates à la place du signataire) ; refus du stockage traduits par modèle et valeurs dans les six langues. Reporté : règlements des bons de commande et des factures (J7).

Choix de mise en œuvre de J6, validés par l'utilisateur le 2026-11-10 : la quantité livrée proposée à la création d'un bordereau est le reste à livrer ; le dépassement du reste à livrer est contrôlé à l'enregistrement du brouillon et de nouveau à la validation ; un bordereau compte dans le reste à livrer dès sa validation (bordereau émis), un bordereau annulé ou en brouillon n'y compte pas ; la livraison est complète quand les bordereaux déjà confirmés et celui que l'on confirme couvrent toutes les lignes de la facture (pour un bordereau vierge, ses propres lignes) ; la validation d'un bordereau dont toutes les quantités livrées valent zéro est refusée ; un bordereau issu d'une facture ne se duplique pas ; en réception d'un bon de commande, une correction à la baisse est permise mais le retour à zéro après une réception partielle est refusé, et la réception complète est définitive ; visas et dates restent modifiables sur un bordereau validé jusqu'à la confirmation, puis sont figés.

**Périmètre de J7 (Paiements, registres, relances et lettre de relance).** Livré : section « Règlements » des fiches Facture et Bon de commande (montant dû, réglé, reste à payer, liste des règlements, saisie avec la date du jour et le reste à payer proposés, suppression confirmée et tracée avec la valeur supprimée) ; règlement refusé au-delà du montant dû, hors des dates permises, sans mode de paiement, sur un brouillon ou un document annulé ; annulation refusée tant qu'il reste des règlements ; entrées de menu « Règlements », « Suivi fournisseurs » et « Relances » ; registre des règlements filtré par sens, période et mode, avec le total et les sous-totaux par mode de chaque sens (les encaissements et les décaissements ne s'additionnent pas) ; bons de commande non payés ; factures échues avec jours de retard et reste dû ; lettre de relance par facture, au texte validé par l'utilisateur, réglable dans les paramètres (titre, objet, texte avant et après le tableau récapitulatif, signataire ; champs {numero}, {date}, {echeance}, {reste}, {retard}) ; PDF de la lettre, du registre et de la liste des relances, en français (section 13) ; historique du document (règlement enregistré ou supprimé, avec le montant). Reporté : export Excel de la liste des relances (J10, avec les autres exports) ; envoi par email (hors périmètre V1). Une lettre dont les textes réglés ne tiennent pas sur une page est refusée, avec un message demandant de les raccourcir.
