# Kadenz — Cahier des charges fonctionnel et technique

Nom du projet : **Kadenz** — projet personnel, sans lien avec une structure existante. Le nom n'apparaît jamais sur les documents générés pour les clients : chaque installation garde l'identité de l'entreprise utilisatrice (BIEMIE SARL, etc.).

Ce document sert de spécification de référence pour l'implémentation. Il reprend et formalise les décisions prises en amont, avec un accent particulier sur la logique de statuts et de modification de saisie.

---

## 1. Présentation

**Nature** : logiciel desktop Windows de gestion commerciale (devis, proforma, bons de commande, bordereaux de livraison, factures), destiné à être installé indépendamment chez différentes structures clientes (PME, bureaux d'études, coopératives) en contexte Burkina Faso / UEMOA.

**Origine** : refonte d'un outil Excel/VBA existant en application robuste, multi-utilisateur, sur vraie base de données relationnelle.

**Décisions déjà actées :**
- Stack : .NET 8, WPF (MVVM), Entity Framework Core, SQLite
- Base de données : SQLite — fichier unique local, embarqué nativement dans l'application, zéro configuration serveur
- Déploiement : une installation indépendante par client — jamais de base partagée entre clients (pas de multi-tenant)
- Mono/multi-poste : SQLite cible le mono-poste en priorité. Un accès réseau multi-poste (plusieurs utilisateurs simultanés sur des postes différents) est fragile avec un fichier SQLite sur un partage réseau — si ce besoin se confirme, migration vers PostgreSQL ou SQL Server Express à prévoir (changement de provider EF Core, pas seulement de chaîne de connexion)
- Migration des données Excel existantes : non retenue, on repart de zéro
- Migrations EF Core appliquées automatiquement au démarrage de l'application (aucune intervention manuelle sur les postes clients)

---

## 2. Glossaire métier

| Terme | Signification |
|---|---|
| IFU | Identifiant Financier Unique (immatriculation fiscale, Burkina Faso) |
| RCCM | Registre du Commerce et du Crédit Mobilier |
| Régime d'imposition | Régime fiscal du contribuable (ex. RSI, DCI) |
| HT | Montant Hors Taxes |
| TTC | Montant Toutes Taxes Comprises |
| TVA | Taxe sur la Valeur Ajoutée (0% ou 18% dans le référentiel actuel) |
| Remise | Réduction commerciale appliquée avant taxes |
| Retenue | Retenue à la source, appliquée côté achats (Bon de commande) |
| Devis | Proposition commerciale non engageante, durée de validité limitée |
| Proforma | Facture "simulée" pré-paiement, souvent utilisée pour ordre de virement |
| Facture | Document de vente définitif, seul document à valeur fiscale/comptable pleine |
| Bordereau de livraison | Document logistique sans montant, atteste la remise physique de marchandise |
| Tiers | Terme générique regroupant Client et Fournisseur dans le référentiel |

---

## 3. Vue d'ensemble des modules

- **Référentiels** — Entreprise, Tiers (clients/fournisseurs), Produits, Paramètres système
- **Documents commerciaux** — Devis, Proforma, Bon de commande, Livraison, Facture
- **Suivi & paiements** — registres, règlements, relances
- **Pilotage** — tableau de bord, rapports
- **Administration** — utilisateurs, droits, sauvegarde, audit (transversal aux blocs ci-dessus)

---

## 4. Référentiels

### 4.1 Entreprise
Une seule fiche par installation, saisie via un assistant de premier lancement : nom, adresse, téléphone, email, site web, localisation, régime d'imposition, RCCM, IFU, jusqu'à 2 comptes bancaires, logo, nom des responsables (facturation, comptabilité).

### 4.2 Tiers (clients et fournisseurs)
Fichier unique servant à la fois de fichier clients et fournisseurs : nom, adresse, téléphone, localisation, IFU, RCCM, régime d'imposition, type (Client / Fournisseur / les deux).

### 4.3 Produits et services
Catalogue unique (contrairement à l'Excel qui avait deux tables séparées Produits/Produits4) : désignation, référence (clé unique), prix, sens d'utilisation (Vente / Achat / les deux).

### 4.4 Paramètres système
Table clé-valeur configurable : taux de TVA disponibles (0%, 18%), grille de remise (0/5/10/15%), grille de retenue (0/5/10/15%), formats de numérotation, dossiers d'export par défaut.

---

## 5. Documents commerciaux

### 5.1 Règles communes

**Numérotation** — format `{Préfixe}{AAAA}-{MM}-{####}`, ex. `D2026-05-0004`. Préfixes : D (Devis), P (Proforma), C (Bon de commande), L (Livraison), F (Facture). Compteur par type + année + mois, incrémentation atomique en transaction pour éviter toute collision en accès concurrent.

**Calculs** :
```
Montant HT       = Σ (prix ligne × quantité)
Montant Remise   = Montant HT × % Remise
Base TVA         = Montant HT − Montant Remise
Montant TVA      = Base TVA × % TVA
Montant Retenue  = Base TVA × % Retenue   (documents d'achat uniquement)
Montant TTC      = Base TVA + Montant TVA (− Montant Retenue si achat)
```

**Lignes** : aucune limite de nombre de lignes (contrairement à l'Excel, contraint par la mise en page des feuilles — 56 lignes pour Devis/Facture, 16 pour Proforma/Bon de commande). Le modèle de données n'impose plus ce plafond ; le générateur de PDF doit gérer la pagination automatique au-delà d'une page.

### 5.2 Devis (D-...)
Client, lignes produits, calcul HT/Remise/TVA/TTC, statut de workflow (voir section 6). Convertible en Facture.

### 5.3 Proforma (P-...)
Structure identique au Devis. Convertible en Facture.

### 5.4 Bon de commande (C-...)
Le tiers est un Fournisseur. Utilise le catalogue produits en sens "Achat". Retenue applicable.

### 5.5 Bordereau de livraison (L-...)
Pas de montant. Colonnes par ligne : référence, désignation, quantité commandée, quantité livrée, observations. Zones de visa client/fournisseur et dates de réception/livraison (à conserver comme champs texte ou signature électronique simple selon la V1 retenue).

### 5.6 Facture (F-...)
Structure identique au Devis. Document définitif, seul à porter un état de paiement.

---

## 6. Logique de statuts et de gestion des modifications de saisie

C'est le point central à bien câbler dès la structure du projet.

### 6.1 Statuts par type de document

| Type | Cycle de statuts |
|---|---|
| Devis / Proforma | Brouillon → Validé → Envoyé → Converti (en Facture) — ou → Annulé à tout moment avant conversion |
| Bon de commande | Brouillon → Validé → Envoyé → Réceptionné (partiel/complet) — ou → Annulé |
| Livraison | Brouillon → Validé → Livré (partiel/complet) — ou → Annulé |
| Facture | Brouillon → Validé → Envoyé — ou → Annulé. L'état de paiement (Impayée / Partielle / Payée) est un **axe séparé**, calculé à partir des règlements, indépendant du statut de workflow |

### 6.2 Règles d'édition selon le statut

- **Brouillon** : entièrement modifiable (tiers, lignes, quantités, remise, TVA, retenue). Recalcul automatique des totaux à chaque changement de ligne. Aucun numéro définitif attribué.
- **Validé** : le numéro est attribué de façon définitive à ce moment précis (jamais avant, pour éviter les trous de séquence dus à des brouillons abandonnés). Lignes et montants figés. Seuls les champs administratifs restent modifiables (état de paiement, mode de paiement, observations).
- **Envoyé** : comme Validé, avec horodatage de la date d'envoi.
- **Correction d'un document Validé/Envoyé** : jamais d'édition directe des lignes ou montants.
  - Pour une **Facture** : émission d'un avoir/note de crédit lié (nouveau document référençant l'original), pour rester conforme aux usages comptables — les factures ne se modifient pas rétroactivement.
  - Pour un **Devis/Proforma** (pas de valeur fiscale figée) : duplication en nouveau document avec référence croisée vers l'original, qui passe en statut Annulé.
- **Annulé** : lecture seule totale. Numéro conservé et jamais réutilisé, visible dans les registres avec mention explicite.

### 6.3 Suppression

- Suppression physique autorisée uniquement sur les **Brouillons** jamais validés.
- Aucune suppression physique au-delà de Brouillon — contrainte volontaire pour préserver l'intégrité de la séquence de numérotation (exigence de traçabilité fiscale).
- Toute annulation au-delà de Brouillon est une suppression logique (changement de statut vers Annulé).

### 6.4 Historique et audit

- Toute transition de statut est journalisée : utilisateur, date/heure, ancien statut, nouveau statut.
- Table d'audit dédiée, append-only (jamais modifiée ni purgée automatiquement).
- Granularité minimale : qui a créé, qui a validé, qui a annulé, et quand — pas nécessairement un audit ligne par ligne de chaque frappe en mode Brouillon.

### 6.5 Gestion de la concurrence (plusieurs utilisateurs)

- Verrouillage optimiste : chaque document porte un numéro de version (`RowVersion`). Toute sauvegarde vérifie que la version n'a pas changé depuis le chargement ; en cas de conflit, message explicite et proposition de recharger avant de retenter.
- Pas de verrouillage pessimiste ("document en cours d'édition par X") en V1 — à évaluer si l'usage multi-poste révèle un besoin réel.

### 6.6 Snapshot des données tierces

- À la création d'un document, les informations du Tiers (nom, adresse, IFU, RCCM, régime fiscal) et des lignes Produit (désignation, prix) sont **copiées** dans le document, pas seulement référencées.
- Une modification ultérieure de la fiche Client ou du catalogue Produits n'affecte **jamais** un document déjà créé. Seuls les nouveaux documents utilisent les valeurs à jour.
- Le lien vers le Tiers/Produit d'origine reste conservé pour la navigation, mais l'affichage utilise toujours le snapshot.

### 6.7 Conversion entre documents

- **Devis/Proforma → Facture** : copie figée des lignes dans un document Facture indépendant. Le document source passe en statut Converti (variante non éditable de Validé) et référence le numéro de la Facture créée.
- **Bon de commande → réception** : ne crée pas de nouveau document. Les quantités livrées sont mises à jour ligne par ligne directement sur le Bon de commande, avec un statut de réception par ligne (Non reçu / Partiel / Complet), ce qui permet des livraisons fournisseur en plusieurs fois.

---

## 7. Suivi & paiements

- Une Facture peut recevoir plusieurs règlements (paiements partiels échelonnés).
- État de paiement recalculé automatiquement : Impayée (aucun règlement), Partielle (0 < payé < TTC), Payée (payé ≥ TTC).
- Contrôle applicatif : un règlement ne peut pas porter le total payé au-delà du montant TTC de la facture.
- Relances : liste des factures Envoyées, non Payées, avec échéance dépassée — consultable et exportable en V1 ; envoi automatique par email en évolution future.

---

## 8. Pilotage

Les indicateurs actuels du tableau de bord Excel (CA HT/TVA/TTC, total payé/impayé, répartition par état et mode de paiement, tendance de ventes, top clients) sont recalculés en direct par requête sur les tables Document/LigneDocument/Règlement — plus de tableau intermédiaire à rafraîchir manuellement.

---

## 9. Administration

### 9.1 Rôles et permissions (matrice proposée, à ajuster)

| Action | Admin | Comptable | Commercial | Consultation |
|---|---|---|---|---|
| Créer/modifier un brouillon (Devis, Proforma, BC, Livraison) | ✓ | ✓ | ✓ | — |
| Créer/modifier un brouillon Facture | ✓ | ✓ | — | — |
| Valider un document | ✓ | ✓ | — | — |
| Annuler un document validé | ✓ | ✓ | — | — |
| Saisir un règlement | ✓ | ✓ | — | — |
| Gérer les référentiels (clients, produits) | ✓ | ✓ | ajout seul | — |
| Gérer les utilisateurs et droits | ✓ | — | — | — |
| Consulter tableau de bord / registres | ✓ | ✓ | ✓ | ✓ |
| Sauvegarder / exporter la base | ✓ | — | — | — |

### 9.2 Sauvegarde

Le fichier SQLite étant unique, la sauvegarde se réduit à une copie de fichier horodatée, déclenchable depuis l'application, stockée dans un dossier configurable. Pas de sauvegarde cloud automatique en V1 (fonctionnement local).

---

## 10. Modèle de données

| Entité | Champs clés | Notes |
|---|---|---|
| **Entreprise** | Id, Nom, Adresse, Telephone, Email, SiteWeb, Localisation, RegimeImposition, RCCM, IFU, Banque1, Banque2, LogoPath, ResponsableFacturation, ResponsableComptabilite | Une seule ligne par installation |
| **Utilisateur** | Id, NomComplet, Identifiant (unique), MotDePasseHache, Role (enum), Actif, DateCreation | |
| **Tiers** | Id, Nom, Adresse, Telephone, Localisation, IFU, RCCM, RegimeImposition, Type (Client/Fournisseur/Les deux), Actif | |
| **Produit** | Id, Designation, Reference (unique), Prix, Sens (Vente/Achat/Les deux), Actif | |
| **Document** | Id, Type (enum), Numero (unique par type), Date, TiersId, TiersNomSnapshot, TiersAdresseSnapshot, TiersIfuSnapshot, TiersRccmSnapshot, TiersRegimeSnapshot, Statut (enum), Libelle, MontantHT, MontantRemise, MontantBaseTVA, MontantTVA, MontantRetenue, MontantTTC, PourcentageRemise, PourcentageTVA, PourcentageRetenue, EtatPaiement (enum, Facture uniquement), ModePaiement, CheminPdf, DocumentConvertiId (auto-référence), RowVersion, DateCreation, DateValidation, DateEnvoi, UtilisateurCreationId, UtilisateurValidationId | Statut et EtatPaiement sont deux axes indépendants |
| **LigneDocument** | Id, DocumentId, ProduitId (nullable), DesignationSnapshot, PrixSnapshot, Quantite, QuantiteLivree (Livraison/BC), Total, Observations, Ordre | |
| **Reglement** | Id, DocumentId (→ Facture), Date, Montant, ModePaiement, Reference, UtilisateurId | |
| **CompteurNumerotation** | Id, TypeDocument, Annee, Mois, DernierNumero | Contrainte unique (TypeDocument, Annee, Mois), incrémentation transactionnelle |
| **ParametreSysteme** | Cle (PK), Valeur (jsonb) | Listes TVA/Remise/Retenue, formats, dossiers |
| **AuditLog** | Id, Entite, EntiteId, Action, AncienneValeur (jsonb), NouvelleValeur (jsonb), UtilisateurId, DateAction | Append-only |

---

## 11. Génération des documents PDF

- Un seul moteur de génération, paramétré par un template par type de document (pas cinq moteurs séparés comme dans l'Excel actuel).
- Champs obligatoires : en-tête entreprise (logo, coordonnées, RCCM/IFU), identification du tiers, tableau des lignes, totaux, montant en lettres, pied de page (comptes bancaires, signataires).
- Le PDF n'est régénérable qu'à partir d'un document Validé (jamais depuis un Brouillon modifiable), pour garantir que le PDF stocké correspond toujours exactement à l'état affiché.
- Pagination automatique si le nombre de lignes dépasse une page.

---

## 12. Exigences non fonctionnelles

- Windows 10/11 64 bits, fonctionnement 100% local
- Interface et documents en français uniquement (V1)
- Devise FCFA uniquement (V1, pas de multi-devise)
- Migrations EF Core appliquées automatiquement au démarrage

---

## 13. Hors périmètre V1 (évolutions futures)

- Gestion de stock réelle (entrées/sorties, valorisation)
- Envoi automatique par email
- Signature électronique
- Export comptable SYSCOHADA (journal des ventes, état de TVA collectée)
- Multi-devise
