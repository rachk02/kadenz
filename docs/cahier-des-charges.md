# Kadenz - Cahier des charges fonctionnel et technique

| Version | Date | Changement |
|---|---|---|
| 1.0 | non datée | Version initiale (.NET 8, WPF, EF Core, SQLite), conservée hors dépôt (`docs_of_interest/`, non versionné) |
| 2.0 | 2026-11-09 | Passage en Rust (Slint, rusqlite), mono-poste avec comptes et rôles, module Paiements commun Facture et Bon de commande, modularité, corrections des incohérences de la v1 |
| 2.1 | 2026-11-09 | Tour 1 validé par l'utilisateur : socle et modules activables par clé de licence, réglages modifiables, avoir, export Excel, import CSV/Excel, licence par client, installateur et mises à jour |
| 2.2 | 2026-11-09 | Tour 2 validé : écrans et ergonomie, fiche document, bordereaux issus des factures, règlements, relances avec lettre, volet achats, sauvegarde automatique (10 copies), licence perpétuelle signée et liée au poste, préfixe A de l'avoir, mise à jour vérifiée au démarrage |
| 2.3 | 2026-11-09 | Règles PDF multi-pages validées (en-têtes et pieds répétés, report, filigranes, deux logos, export des registres), arrondi et quantités fractionnaires tranchés, orthographe du montant en lettres par réglage, ordre des jalons J1 à J10 validé |
| 2.4 | 2026-11-09 | Jalon J2 : logos de l'entreprise stockés dans la base, matrice des droits modifiable, import CSV/Excel détaillé (section 6.8), crate `kadenz-import`, feuille de route J1 à J10 reportée en section 19, question des délais par défaut ajoutée |

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
- Assistant de premier lancement, dans l'ordre : clé de licence, fiche entreprise, compte Admin, taux et modes de paiement, import facultatif des produits et des tiers.
- Chaque type de document a deux écrans : Liste (registre) et Fiche (création, consultation), comme « Création / Liste » dans l'Excel.
- Chaque liste se filtre (numéro, tiers, période, statut, état de paiement) et se trie par colonne. Colonnes : celles de l'Excel (n°, date, tiers, montant TTC, état de paiement, mode de paiement, document), plus le statut.
- Raccourcis clavier : Ctrl+N nouveau, Ctrl+S enregistrer, Ctrl+P imprimer, Ctrl+F rechercher.
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
| Numérotation | Préfixe par type, remise à zéro du compteur chaque mois (comme l'Excel) ou chaque année, nombre de chiffres du rang |
| Textes des documents | Titre par type (ex. « FACTURE DEFINITIVE »), phrase « Arrêté la présente ... à la somme de », libellé du signataire (ex. « LA COMPTABILITE »), mentions de pied de page |
| Apparence des PDF | Couleur principale, position des logos |
| Montant en lettres | Orthographe rectifiée de 1990 ou traditionnelle |
| Dossiers d'export | Un dossier par type de document (comme l'Excel) |
| Délais | Durée de validité des devis, délai de paiement des factures (date d'échéance calculée, modifiable sur le document) |
| Rôles | Matrice des droits par rôle (section 10.1) |

L'identité de l'entreprise (section 5.1) est éditée dans le même écran de paramètres.

---

## 6. Documents commerciaux (spécifié)

### 6.1 Règles communes

**Numérotation** : format `{Préfixe}{AAAA}-{MM}-{####}`, ex. `D2026-05-0004`. Préfixes par défaut : D (Devis), P (Proforma), C (Bon de commande), L (Livraison), F (Facture), A (Avoir). Compteur par type + année + mois de la date du document, incrémenté dans la transaction de validation. Au-delà de 9999 dans un mois, le numéro s'allonge sans perte.

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

### 6.6 Facture (F-...)
Structure identique au Devis. Document définitif, porte un état de paiement (section 8). Champ date d'échéance, calculé à partir du délai de paiement paramétré.

### 6.7 Avoir (A-...)
Document lié à une Facture validée, qui la corrige ou l'annule en tout ou partie (section 7.2). Préfixe A, même format et même compteur par type, année et mois que les autres documents (ex. `A2026-05-0001`).

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
| Avoir | Même cycle que la Facture. Montants stockés en positif ; le montant dû de la Facture d'origine est diminué des avoirs validés (jalon J5) |

L'état de paiement (Facture et Bon de commande) est un axe séparé du statut de workflow (section 8).

### 7.2 Règles d'édition selon le statut

- **Brouillon** : entièrement modifiable (tiers, lignes, quantités, remise, TVA, retenue). Recalcul des totaux à chaque changement. Aucun numéro attribué.
- **Validé** : le numéro est attribué à ce moment précis, jamais avant, pour éviter les trous de séquence dus aux brouillons abandonnés. Lignes et montants figés. Seuls les champs administratifs restent modifiables (mode de paiement, observations).
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

### 7.5 Sessions et utilisateurs

- Mono-poste : pas de verrouillage optimiste ni pessimiste entre postes (le `RowVersion` de la v1 est retiré).
- Chaque session est ouverte par un compte utilisateur ; toutes les actions journalisées portent ce compte.

### 7.6 Snapshot des données tierces

- À la création d'un document, les informations du Tiers (nom, adresse, IFU, RCCM, régime fiscal) et des lignes Produit (désignation, prix) sont copiées dans le document.
- Une modification ultérieure de la fiche Tiers ou du catalogue n'affecte jamais un document existant.
- Le lien vers le Tiers ou le Produit d'origine est conservé pour la navigation ; l'affichage utilise toujours le snapshot.

### 7.7 Conversion entre documents

- **Devis/Proforma → Facture** : copie figée des lignes dans une Facture indépendante. Le document source passe en Converti et référence la Facture créée.
- **Facture → Bordereau(x) de livraison** : voir section 6.5.
- **Facture → Avoir** : voir section 6.7.
- **Bon de commande → réception** : pas de nouveau document. Les quantités reçues sont mises à jour ligne par ligne sur le Bon de commande (Non reçu / Partiel / Complet), ce qui permet des livraisons fournisseur en plusieurs fois.

---

## 8. Paiements (spécifié)

Le module Paiements s'applique à deux types de documents :
- la **Facture** : créance client, les règlements sont des encaissements ;
- le **Bon de commande** : dette fournisseur, les règlements sont des décaissements (l'Excel affichait déjà un état et un mode de paiement sur la liste des bons de commande).

Règles communes :
- Un document peut recevoir plusieurs règlements (paiements échelonnés), seulement à partir du statut Validé.
- État de paiement calculé à partir des règlements, jamais saisi : Impayé (aucun règlement), Partiel (0 < payé < montant dû), Payé (payé = montant dû).
- Un règlement ne peut pas porter le total payé au-delà du montant dû.
- Saisie depuis la fiche du document : date, montant, mode, référence ; le montant proposé par défaut est le reste à payer.
- Un règlement saisi par erreur peut être supprimé ; la suppression est tracée dans le journal d'audit avec la valeur supprimée.
- Registre des encaissements et des décaissements, par période et par mode de paiement.
- Suivi fournisseurs : liste des Bons de commande non payés.

Relances (module Relances) :
- Liste des Factures échues non payées : jours de retard, reste dû ; export Excel.
- Impression d'une lettre de relance par facture (PDF), à l'identité de l'entreprise.
- Envoi par email hors périmètre V1.

---

## 9. Pilotage (spécifié)

Indicateurs recalculés en direct par requête sur les tables Document, LigneDocument et Reglement :
- Ventes (repris de l'Excel) : CA HT, TVA, CA TTC, total payé, total impayé, CA par état de paiement, CA par mode de paiement, tendance des ventes, top 5 clients, 4 dernières factures.
- Achats : total des Bons de commande, montants payés et restant dus aux fournisseurs.
- Filtre par année ou par période (comme le filtre « DATE Années » de l'Excel).

Slint n'a pas de composant graphique prêt à l'emploi : les graphiques (secteurs, courbes, barres) sont dessinés par l'application (chemins Slint ou image générée côté Rust). À l'étude : choix entre ces deux techniques.

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

## 12. Génération des documents PDF (à l'étude)

Exigences (spécifié) :
- Un seul moteur paramétré par un modèle par type de document.
- Contenu : en-tête entreprise (logo, coordonnées, RCCM/IFU), identification du tiers, tableau des lignes, totaux, montant en lettres, pied de page (comptes bancaires, signataires).
- Mise en page reprise de l'Excel : bande verticale de couleur à gauche, logo, bloc entreprise, bloc tiers, tableau, totaux, montant en lettres, signataire, pied de page avec adresse, RCCM, IFU, comptes bancaires, site et email. Couleur principale paramétrable.
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
- Pagination automatique.

Choix technique (à l'étude) : c'est la partie la plus délicate en Rust pur (modèles, pagination, logo, polices). La bibliothèque sera choisie après un essai sur une page de facture complète.

---

## 13. Exigences non fonctionnelles (spécifié)

- Windows 10/11 64 bits, fonctionnement 100 % local, exécutable natif. Seule exception : la vérification facultative de mise à jour (section 16), qui échoue sans gêne hors connexion.
- Interface et documents en français uniquement (V1).
- Devise FCFA uniquement (V1).
- Base stockée dans le dossier de données de l'utilisateur Windows (`%APPDATA%`).
- Migrations appliquées automatiquement au démarrage.
- Objectifs chiffrés de performance (temps de démarrage, temps d'ouverture d'un registre) : à définir.

---

## 14. Licences (spécifié)

- Slint est utilisé sous la licence Slint Royalty-free 2.0, gratuite pour une application desktop à condition d'afficher le composant `AboutSlint` dans un écran ou dialogue "À propos" accessible depuis le menu principal (ou, à défaut, d'afficher le badge Slint sur la page web de téléchargement). Une licence commerciale payante existe si cette attribution n'est pas souhaitée.
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

- Installateur Windows classique pour le déploiement chez le client.
- Au démarrage, l'application vérifie s'il existe une version plus récente parmi les releases GitHub et propose de l'installer ; l'utilisateur confirme. Hors connexion, la vérification échoue sans message bloquant.
- À l'étude : technologie d'installateur, signature du code de l'exécutable.

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
3. **Montant en lettres** : l'Excel mélangeait deux usages ("Soixante dix mille sept cent quarante", "Cent-cinquante mille"). Les deux orthographes sont implémentées et choisies par réglage : rectifiée de 1990 (traits d'union partout, valeur par défaut proposée) ou traditionnelle. Valeur par défaut à confirmer.
4. **Modularité commerciale** : tranchée le 2026-11-09, modules activés par clé de licence (sections 3.2 et 15).
5. **Avoir** : tranchée le 2026-11-09, préfixe A (section 6.7).
6. **Conversion** : tranchée le 2026-11-09, conversion possible dès Validé.
7. **Quantités fractionnaires** : tranchée le 2026-11-09, oui (quantités en millièmes).
8. **Distribution** : technologie d'installateur et signature du code (à l'étude, section 16).
9. **Écrans non vus** : aucune capture des formulaires Devis et Proforma ni des listes Factures, Livraisons, Proforma et Devis ; leur contenu est supposé identique à la Facture et à la liste des bons de commande.
10. **Délais par défaut** : le cahier ne fixe ni durée de validité des devis ni délai de paiement des factures. Les deux réglages existent (section 5.4) et sont vides par défaut, donc sans date d'échéance calculée. Valeurs par défaut à confirmer.

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
| J9 | Licence : outil éditeur, vérification, activation des modules |
| J10 | Installateur, mises à jour, export Excel |
