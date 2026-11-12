# Kadenz - Cahier de la phase 2 (brouillon)

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-12 | Premier brouillon : candidats repris de la section 17 du cahier V1 et de la visée annoncée le 2026-11-11 (serveur, site web, suite comparable à Sage), dépendances et questions ouvertes. Rien n'est décidé ni codé. |
| 0.2 | 2026-11-12 | Premières décisions de l'utilisateur : les sept candidats entrent dans la phase 2, plus les chantiers jugés nécessaires ; serveur installable chez le client et hébergé ; comptabilité comme produit suivant de la suite ; licence par abonnement. Chantiers de fondation ajoutés (proposés, à valider), ordre proposé, questions mises à jour. |

Statut : **à l'étude** pour tout le document. Les décisions de la section 2 sont prises ; aucun bloc n'est encore spécifié ni codé. Aucun chiffre, aucune règle comptable ou fiscale et aucun choix technique n'est retenu ici sans source vérifiée : là où il en faut, une question est posée.

## 1. Point de départ

D'après le cahier V1 (`docs/cahier-des-charges.md`) :

- Kadenz V1 est une application Windows mono-poste, en Rust et Slint ; jalons J1 à J10 et J8b livrés (section 19) ; dernière version publiée : v0.2.1 (2026-11-12).
- Données dans une base SQLite locale unique, chiffrée (clé du poste protégée par Windows), sauvegardes chiffrées par mot de passe (sections 10.3 et 13).
- Comptes et rôles sur le poste (Admin, Comptable, Commercial, Consultation), matrice des droits modifiable (section 10.1).
- Modules activés par clé de licence perpétuelle liée au poste, vérifiée hors ligne (sections 3.2 et 15).
- Documents : Devis, Proforma, Bon de commande, Livraison, Facture, Avoir ; règlements, relances, tableau de bord ; PDF et exports Excel ; interface en sept langues, documents clients en français (sections 6 à 9, 12 et 13).
- Hors périmètre V1 (section 17) : gestion de stock réelle, envoi par email, signature électronique, export comptable SYSCOHADA, multi-devise, multi-poste en réseau.

Visée annoncée par l'utilisateur le 2026-11-11 : une phase suivante plus grande, avec un serveur, de nouvelles fonctions et un site web ; à terme, une suite de logiciels comparable à Sage.

## 2. Décisions de l'utilisateur (2026-11-12)

1. **Périmètre** : les sept candidats de la section 3 entrent dans la phase 2 (multi-poste et serveur, export SYSCOHADA, stock, email, multi-devise, signature électronique, site web), plus « tout ce que tu vas juger nécessaire pour la suite » : ces ajouts sont en section 4, à valider un par un.
2. **Serveur** : les deux formes, installable sur le réseau local de l'entreprise et hébergé sur Internet.
3. **Suite** : la comptabilité est le produit qui suit la gestion commerciale.
4. **Licence** : abonnement.

## 3. Candidats

Chaque candidat décrit ce qu'il recouvrirait, ce qu'il touche dans la V1 et ce qu'il faut savoir avant de le spécifier.

### 3.1 Multi-poste et serveur (retenu : réseau local et hébergé)

- Recouvre : plusieurs postes qui travaillent sur les mêmes données en même temps ; comptes communs ; numérotation des documents sans doublon entre postes.
- Touche : le stockage entier (aujourd'hui un fichier SQLite local), la clé de chiffrement liée au poste, la sauvegarde, la licence liée au poste, la mise à jour (serveur et postes à la même version du schéma).
- Conséquence de la décision « les deux » : un seul serveur, livré sous deux formes (installé chez le client, ou exploité par l'éditeur), et non deux produits. La forme hébergée ajoute l'exploitation par l'éditeur (hébergement, sécurité réseau, disponibilité, sauvegardes côté serveur, séparation des données entre entreprises clientes).
- Reste ouvert : fonctionnement hors connexion des postes (question 5.1), système d'exploitation du serveur installé chez le client, choix de la base de données côté serveur.

### 3.2 Export comptable SYSCOHADA

- Recouvre, selon la section 17 du cahier V1 : journal des ventes et état de la TVA collectée.
- Touche : factures, avoirs et règlements validés (déjà figés à la validation), paramètres de TVA.
- Lien avec la décision 3 : la comptabilité devenant le produit suivant, l'export doit être pensé comme la première brique de ce produit (mêmes écritures, même plan de comptes), pas comme un fichier isolé.
- À savoir avant de spécifier : le plan de comptes à utiliser (comptes de ventes, de TVA, de clients), le format attendu, le traitement des avoirs et des retenues. Ces règles doivent venir d'une source vérifiée (texte de référence ou comptable), pas de ce brouillon.

### 3.3 Gestion de stock réelle

- Recouvre, selon la section 17 du cahier V1 : entrées et sorties de stock, valorisation.
- Touche : le référentiel Produits, la réception des bons de commande (entrée) et la confirmation des bordereaux de livraison (sortie), déjà présentes en V1 ; le tableau de bord.
- À savoir : méthode de valorisation, gestion de plusieurs dépôts ou non, inventaire et ajustements, stock négatif permis ou refusé, produits qui ne sont pas stockés (services).

### 3.4 Envoi par email

- Recouvre : envoyer un document PDF (devis, facture, relance) au client depuis Kadenz.
- Touche : les PDF et les lettres de relance existants, la fiche client (adresse email), le journal d'audit.
- À savoir : envoi par le compte de messagerie de l'entreprise (réglage du serveur d'envoi) ou par un service tiers ; envoi manuel seulement, ou aussi automatique (relances à date) ; copie à l'expéditeur.

### 3.5 Multi-devise

- Recouvre : documents dans une autre devise que le FCFA (seule devise de la V1, section 13).
- Touche : montants et arrondi (aujourd'hui au franc), montant en lettres, PDF, règlements, tableau de bord, export comptable.
- À savoir : quelles devises ; taux saisi par document ou table de taux ; devise de tenue des comptes ; montant en lettres dans les autres devises.

### 3.6 Signature électronique

- Le terme recouvre des besoins très différents : image de signature ou cachet sur le PDF ; signature numérique du PDF (certificat) ; signature du client à distance sur un devis.
- À savoir : lequel est voulu, et s'il répond à une exigence légale ou à un usage commercial.

### 3.7 Site web

- Recouvre au minimum un site public. Son objet reste à préciser : site de présentation de Kadenz (le produit), espace client relié au serveur (3.1), espace d'abonnement (4.1).
- Pour tout site public, la liste de contrôle SEO de l'utilisateur s'applique (plan du site, robots, balises canoniques et langues, titres et descriptions uniques, données structurées tirées du contenu réel, images, performances mobiles, Search Console préparée pour l'utilisateur).
- À savoir : objet, langues, nom de domaine, hébergement, contenus disponibles (textes, captures, tarifs) ; rien ne sera inventé pour remplir une page.

## 4. Chantiers ajoutés (proposés, à valider)

Proposés en réponse à la décision 1 ; chacun découle d'une décision de la section 2 et reste à valider par l'utilisateur.

### 4.1 Abonnement et portail de licences

- Pourquoi : la licence par abonnement (décision 4) remplace une clé perpétuelle vérifiée hors ligne par une licence datée qu'il faut renouveler.
- Recouvre : émission et renouvellement des licences (aujourd'hui l'outil `kadenz-editeur`, manuel) ; suivi des abonnements par entreprise ; comportement à l'échéance ; paiement.
- Touche : la licence V1 (section 15 du cahier V1 : Ed25519, liaison au poste, modules), l'assistant de premier lancement, le site (3.7).
- À savoir : questions 5.2 à 5.4.

### 4.2 Reprise des installations V1

- Pourquoi : une base V1 doit pouvoir être reprise sans perte (section 5) ; avec un serveur, il faut aussi pouvoir verser une base mono-poste dans le serveur.
- Recouvre : migration du schéma, import d'une base V1 (ou d'une sauvegarde `.kzs`) dans le serveur, numérotation continue après la reprise.

### 4.3 Socle commun de la suite

- Pourquoi : la comptabilité vient ensuite (décision 3) ; ce qui est commun aux deux produits doit être séparé avant de bâtir le serveur, pour ne pas le refaire.
- Recouvre : entreprise, comptes et droits, tiers (clients et fournisseurs), paramètres fiscaux, licence et modules, journal d'audit, partagés par les produits de la suite.

### 4.4 Sécurité et exploitation du serveur

- Pourquoi : la forme hébergée (décision 2) met des données d'entreprises clientes sur un serveur exploité par l'éditeur.
- Recouvre : authentification des postes et des comptes, chiffrement des échanges, séparation des données entre entreprises, sauvegardes côté serveur, journal des accès, mises à jour du serveur.

### 4.5 Signature du code

- Pourquoi : déjà spécifiée en V1 (section 16 du cahier V1) mais en attente du certificat ; avec des postes et un serveur qui se mettent à jour, l'installation non signée devient un frein plus fort (antivirus, confiance du client).
- Reste une décision et un achat de l'utilisateur (question 8 du cahier V1).

## 5. Ordre proposé (à valider)

1. **Fondations** : socle commun (4.3), serveur sous ses deux formes (3.1), sécurité (4.4), reprise des bases V1 (4.2), abonnement (4.1). Ces chantiers décident de l'architecture ; les faire après les fonctions obligerait à refaire celles-ci.
2. **Fonctions métier sur le nouveau socle** : stock (3.3), puis export SYSCOHADA (3.2) comme première brique de la comptabilité.
3. **Chantiers indépendants**, intercalables selon la demande des clients : email (3.4), signature électronique (3.6), multi-devise (3.5, à faire avant l'export SYSCOHADA s'il doit en tenir compte).
4. **Site web** (3.7) : site de présentation possible dès maintenant ; espace client et abonnement après 4.1 et 3.1.

## 6. Dépendances entre chantiers

- 3.1 (serveur) conditionne un espace client sur le site (3.7) et change la façon de faire la sauvegarde, la licence et les mises à jour.
- 4.1 (abonnement) dépend d'un moyen de vérifier ou de renouveler la licence ; sa forme dépend de la réponse 5.1 (hors connexion).
- 3.2 (export comptable) est plus simple si 3.5 (multi-devise) n'est pas encore là, et doit être revu quand 3.5 arrive.
- 3.3 (stock) s'appuie sur les réceptions et livraisons de la V1 ; un stock partagé entre postes dépend du serveur.
- 3.4 (email) et 3.6 (signature) s'appuient sur les PDF existants et sont indépendants des autres.

## 7. Contraintes déjà connues

- Ne rien casser pour les installations V1 : une base V1 doit pouvoir être reprise (migration du schéma, jamais de perte).
- Les documents remis aux clients ne portent pas l'identité Kadenz.
- Les décisions de la V1 restent valables tant qu'elles ne sont pas revues explicitement (cahier V1, sections 18 et 19). La décision 4 revoit la licence perpétuelle de la section 15 du cahier V1 pour les nouvelles licences ; le sort des licences déjà émises est la question 5.4.

## 8. Questions ouvertes pour l'utilisateur

### Réglées le 2026-11-12

- Périmètre (décision 1), forme du serveur (décision 2), produit suivant de la suite (décision 3), modèle de licence (décision 4).

### Nouvelles, nées de ces décisions

1. **Hors connexion** : un poste doit-il pouvoir travailler quand le serveur ou Internet est coupé (et resynchroniser ensuite), ou le serveur est-il obligatoire pour travailler ?
2. **Abonnement, échéance** : que se passe-t-il à la fin d'un abonnement non renouvelé : lecture seule, socle seul comme une V1 sans clé, ou blocage ? Délai de grâce ?
3. **Abonnement, forme** : période (mensuelle, annuelle), unité (par entreprise, par poste, par utilisateur), et moyen de paiement envisagé.
4. **Licences V1 déjà émises** : restent-elles perpétuelles pour la version mono-poste, ou passent-elles à l'abonnement ?
5. **Chantiers ajoutés** : la section 4 est-elle validée telle quelle ?
6. **Ordre** : l'ordre de la section 5 convient-il ?

### Toujours ouvertes

7. **Clients visés** : la V1 vise des PME, bureaux d'études et coopératives au Burkina Faso et dans l'UEMOA, en FCFA (cahier V1, sections 1 et 6). La phase 2 garde-t-elle ce cadre ? Combien de postes par entreprise ?
8. **SYSCOHADA** : quel format de sortie, pour quel logiciel comptable en attendant la comptabilité Kadenz, et quelle source fait foi pour le plan de comptes (comptable, texte de référence) ?
9. **Stock** : méthode de valorisation, un ou plusieurs dépôts, stock négatif permis ?
10. **Email** : messagerie de l'entreprise ou service tiers ; relances envoyées automatiquement ou à la main ?
11. **Multi-devise** : quelles devises, et comment le taux est-il fixé ?
12. **Signature électronique** : image ou cachet, certificat numérique, ou signature du client à distance ?
13. **Site web** : objet (présentation de Kadenz, espace client, abonnement), langues, nom de domaine, contenus disponibles ?
14. **Hébergement** : pour la forme hébergée, où (pays, prestataire) et qui l'exploite ?

## 9. Hors de ce document

- La distribution de la V1 (recherche d'antériorité de la marque) : question ouverte 8 du cahier V1, traitée à part ; la signature du code est reprise en 4.5.
- L'essai complet de la V1 par l'utilisateur.
