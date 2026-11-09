# Kadenz - Cahier des charges fonctionnel et technique

| Version | Date | Changement |
|---|---|---|
| 1.0 | non datée | Version initiale (.NET 8, WPF, EF Core, SQLite), conservée dans `docs_of_interest/kadenz-cahier-des-charges.md` |
| 2.0 | 2026-11-09 | Passage en Rust (Slint, rusqlite), mono-poste avec comptes et rôles, module Paiements commun Facture et Bon de commande, modularité, corrections des incohérences de la v1 |

Statut des capacités : chaque bloc est marqué **spécifié** (décidé, à implémenter), **à l'étude** (choix technique ou métier non tranché) ou **en production** (livré). À ce jour, rien n'est en production.

Nom du projet : **Kadenz**, projet personnel sans lien avec une structure existante. Le nom n'apparaît jamais sur les documents générés pour les clients : chaque installation garde l'identité de l'entreprise utilisatrice, saisie à l'installation.

---

## 1. Présentation

**Nature** : logiciel desktop Windows de gestion commerciale (devis, proforma, bons de commande, bordereaux de livraison, factures), vendu et installé indépendamment chez différentes structures clientes (PME, bureaux d'études, coopératives) en contexte Burkina Faso / UEMOA.

**Origine** : refonte d'un outil Excel/VBA existant (captures dans `docs_of_interest/`) en application native robuste, sur vraie base de données relationnelle.

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
| `kadenz-domaine` | Règles métier pures : types de documents, statuts et transitions, calculs, numérotation, état de paiement, droits par rôle. Aucune entrée/sortie, entièrement testable. | rien |
| `kadenz-stockage` | Accès SQLite : ouverture, migrations, dépôts, compteurs de numérotation, sauvegarde. | `kadenz-domaine` |
| `kadenz` (application) | Interface Slint, orchestration des cas d'usage. | les deux précédentes |
| `kadenz-pdf` (à venir) | Génération des documents PDF, voir section 12. | `kadenz-domaine` |

Les règles métier ne vivent que dans `kadenz-domaine` : l'interface et le stockage ne recalculent jamais un total ni ne décident d'une transition de statut par eux-mêmes.

### 3.2 Modules activables

Chaque grand bloc fonctionnel (Devis, Proforma, Bons de commande, Livraisons, Factures, Paiements, Tableau de bord) est un module qui peut être activé ou désactivé par installation, via `ParametreSysteme`. Un module désactivé disparaît de la navigation ; ses données éventuelles restent en base.

Cette modularité est une modularité de configuration et de code (crates, modules activables), pas un système d'extensions chargées à l'exécution.

À l'étude : faut-il vendre ou activer les modules séparément par client (clé de licence, activation) ? La réponse change la conception de l'activation.

---

## 4. Vue d'ensemble des modules

- **Référentiels** : Entreprise, Tiers (clients et fournisseurs), Produits, Paramètres système.
- **Documents commerciaux** : Devis, Proforma, Bon de commande, Livraison, Facture.
- **Paiements** : règlements, registres, relances.
- **Pilotage** : tableau de bord, rapports.
- **Administration** : utilisateurs, droits, sauvegarde, audit (transversal aux blocs ci-dessus).

---

## 5. Référentiels (spécifié)

### 5.1 Entreprise
Une seule fiche par installation, saisie via un assistant de premier lancement : nom, adresse, téléphone, email, site web, localisation, régime d'imposition, RCCM, IFU, jusqu'à 2 comptes bancaires, logo, nom des responsables (facturation, comptabilité). Aucune donnée d'entreprise n'est livrée dans le code ou dans le dépôt.

### 5.2 Tiers (clients et fournisseurs)
Fichier unique servant à la fois de fichier clients et fournisseurs : nom, adresse, téléphone, localisation, IFU, RCCM, régime d'imposition, type (Client / Fournisseur / les deux).

### 5.3 Produits et services
Catalogue unique (l'Excel avait deux tables séparées Produits/Produits4) : désignation, référence (clé unique), prix, sens d'utilisation (Vente / Achat / les deux).

### 5.4 Paramètres système
Table clé-valeur configurable : taux de TVA disponibles (0 %, 18 %), grille de remise (0/5/10/15 %), grille de retenue (0/5/10/15 %), formats de numérotation, dossiers d'export par type de document (l'Excel en avait un par type), modules activés.

---

## 6. Documents commerciaux (spécifié)

### 6.1 Règles communes

**Numérotation** : format `{Préfixe}{AAAA}-{MM}-{####}`, ex. `D2026-05-0004`. Préfixes : D (Devis), P (Proforma), C (Bon de commande), L (Livraison), F (Facture). Compteur par type + année + mois, incrémenté dans la transaction de validation. Au-delà de 9999 dans un mois, le numéro s'allonge sans perte.

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

### 6.6 Facture (F-...)
Structure identique au Devis. Document définitif, porte un état de paiement (section 8).

---

## 7. Statuts et modifications de saisie (spécifié)

### 7.1 Statuts par type de document

| Type | Cycle de statuts |
|---|---|
| Devis / Proforma | Brouillon → Validé → Envoyé → Converti (en Facture), ou → Annulé à tout moment avant conversion |
| Bon de commande | Brouillon → Validé → Envoyé → Réception partielle → Réception complète, ou → Annulé avant toute réception |
| Livraison | Brouillon → Validé → Livraison partielle → Livraison complète, ou → Annulé avant toute livraison |
| Facture | Brouillon → Validé → Envoyé, ou → Annulé |

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
- Relances : liste des Factures envoyées, non payées, avec échéance dépassée, consultable et exportable en V1. Envoi par email hors périmètre V1.
- Suivi fournisseurs : liste des Bons de commande non payés.

---

## 9. Pilotage (spécifié)

Indicateurs de l'Excel recalculés en direct par requête sur les tables Document, LigneDocument et Reglement : CA HT/TVA/TTC, total payé/impayé, répartition par état et par mode de paiement, tendance des ventes, top clients, dernières factures.

Slint n'a pas de composant graphique prêt à l'emploi : les graphiques (secteurs, courbes, barres) sont dessinés par l'application (chemins Slint ou image générée côté Rust). À l'étude : choix entre ces deux techniques.

---

## 10. Administration (spécifié)

### 10.1 Rôles et permissions

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
Mots de passe stockés hachés avec Argon2id (jamais en clair). Le premier compte Admin est créé par l'assistant de premier lancement.

### 10.3 Sauvegarde
Copie horodatée de la base, déclenchable depuis l'application, dans un dossier configurable. La copie utilise l'API de sauvegarde en ligne de SQLite, et non une copie brute du fichier, qui peut produire une sauvegarde corrompue quand la base est ouverte. Pas de sauvegarde cloud en V1.

---

## 11. Modèle de données (spécifié)

Types SQLite : `INTEGER` (identifiants, montants, pourcentages, quantités, booléens), `TEXT` (chaînes, dates ISO 8601, JSON). Les valeurs structurées sont du JSON stocké en `TEXT`.

| Entité | Champs clés | Notes |
|---|---|---|
| **Entreprise** | Id, Nom, Adresse, Telephone, Email, SiteWeb, Localisation, RegimeImposition, RCCM, IFU, Banque1, Banque2, LogoChemin, ResponsableFacturation, ResponsableComptabilite | Une seule ligne (contrainte Id = 1) |
| **Utilisateur** | Id, NomComplet, Identifiant (unique), MotDePasseHache, Role, Actif, DateCreation | |
| **Tiers** | Id, Nom, Adresse, Telephone, Localisation, IFU, RCCM, RegimeImposition, Type, Actif | |
| **Produit** | Id, Designation, Reference (unique), Prix, Sens, Actif | |
| **Document** | Id, Type, Numero (unique par type, nul en Brouillon), Date, TiersId, snapshots Tiers (Nom, Adresse, IFU, RCCM, Regime), Statut, Libelle, MontantHT, MontantRemise, MontantBaseTVA, MontantTVA, MontantRetenue, MontantTTC, PourcentageRemise, PourcentageTVA, PourcentageRetenue, ModePaiement, CheminPdf, DocumentSourceId, DateCreation, DateValidation, DateEnvoi, UtilisateurCreationId, UtilisateurValidationId | Pas de colonne d'état de paiement : il est calculé |
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
- Le PDF n'est généré qu'à partir d'un document Validé.
- Pagination automatique.

Choix technique (à l'étude) : c'est la partie la plus délicate en Rust pur (modèles, pagination, logo, polices). La bibliothèque sera choisie après un essai sur une page de facture complète.

---

## 13. Exigences non fonctionnelles (spécifié)

- Windows 10/11 64 bits, fonctionnement 100 % local, exécutable natif.
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

## 15. Hors périmètre V1

- Gestion de stock réelle (entrées/sorties, valorisation)
- Envoi automatique par email
- Signature électronique
- Export comptable SYSCOHADA (journal des ventes, état de TVA collectée)
- Multi-devise
- Multi-poste en réseau

---

## 16. Questions ouvertes

1. **Mois du numéro** : sur la capture Excel, le bon de commande C2026-05-0003 porte la date 06/07/2026. Le mois du numéro est-il celui de la date du document, de la date de validation, ou autre ?
2. **Arrondi** : règle d'arrondi des remises, TVA et retenues en FCFA (arrondi commercial au franc le plus proche, ou autre). Implémentée provisoirement à l'arrondi commercial, à confirmer.
3. **Montant en lettres** : orthographe à retenir. L'Excel écrit "Soixante dix mille sept cent quarante" (70 740) mais "Cent-cinquante mille" (150 000), deux usages différents des traits d'union.
4. **Modularité commerciale** : modules vendus ou activés séparément par client ? (section 3.2)
5. **Avoir** : préfixe et numérotation de l'avoir sur facture.
6. **Conversion** : un Devis ou une Proforma peut-il être converti dès Validé, ou seulement après Envoyé ? Implémenté provisoirement : dès Validé.
7. **Quantités fractionnaires** : nécessaires (mètres de câble, etc.) ? Le modèle les permet.
8. **Distribution** : installateur Windows, signature du code, mises à jour (à l'étude).
9. **Écrans non vus** : aucune capture des formulaires Devis et Proforma ni des listes Factures, Livraisons, Proforma et Devis ; leur contenu est supposé identique à la Facture et à la liste des bons de commande.
