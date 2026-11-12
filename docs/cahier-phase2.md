# Kadenz - Cahier de la phase 2 (brouillon)

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-12 | Premier brouillon : candidats repris de la section 17 du cahier V1 et de la visée annoncée le 2026-11-11 (serveur, site web, suite comparable à Sage), dépendances et questions ouvertes. Rien n'est décidé ni codé. |

Statut : **à l'étude** pour tout le document. Aucun bloc n'est spécifié tant que l'utilisateur n'a pas répondu aux questions de la section 5 ; l'ordre des chantiers est sa décision. Aucun chiffre, aucune règle comptable ou fiscale et aucun choix technique n'est retenu ici sans source vérifiée : là où il en faut, une question est posée.

## 1. Point de départ

D'après le cahier V1 (`docs/cahier-des-charges.md`) :

- Kadenz V1 est une application Windows mono-poste, en Rust et Slint ; jalons J1 à J10 et J8b livrés (section 19).
- Données dans une base SQLite locale unique, chiffrée (clé du poste protégée par Windows), sauvegardes chiffrées par mot de passe (sections 10.3 et 13).
- Comptes et rôles sur le poste (Admin, Comptable, Commercial, Consultation), matrice des droits modifiable (section 10.1).
- Modules activés par clé de licence perpétuelle liée au poste (sections 3.2 et 15).
- Documents : Devis, Proforma, Bon de commande, Livraison, Facture, Avoir ; règlements, relances, tableau de bord ; PDF et exports Excel ; interface en sept langues, documents clients en français (sections 6 à 9, 12 et 13).
- Hors périmètre V1 (section 17) : gestion de stock réelle, envoi par email, signature électronique, export comptable SYSCOHADA, multi-devise, multi-poste en réseau.

Visée annoncée par l'utilisateur le 2026-11-11 : une phase suivante plus grande, avec un serveur, de nouvelles fonctions et un site web ; à terme, une suite de logiciels comparable à Sage.

## 2. Candidats

Chaque candidat décrit ce qu'il recouvrirait, ce qu'il touche dans la V1 et ce qu'il faut savoir avant de le spécifier.

### 2.1 Multi-poste et serveur

- Recouvre : plusieurs postes qui travaillent sur les mêmes données en même temps ; comptes communs ; numérotation des documents sans doublon entre postes.
- Touche : le stockage entier (aujourd'hui un fichier SQLite local), la clé de chiffrement liée au poste, la sauvegarde, la licence liée au poste, la mise à jour (serveur et postes à la même version du schéma).
- Formes possibles, à départager : serveur sur le réseau local de l'entreprise ; serveur hébergé sur Internet ; les deux. Le choix décide de l'hébergement, de la sécurité réseau, du fonctionnement hors connexion et du coût pour le client.
- Prérequis probable de 2.7 (site web avec espace client) si un tel espace est voulu.

### 2.2 Export comptable SYSCOHADA

- Recouvre, selon la section 17 : journal des ventes et état de la TVA collectée.
- Touche : factures, avoirs et règlements validés (déjà figés à la validation), paramètres de TVA.
- À savoir avant de spécifier : le plan de comptes à utiliser (comptes de ventes, de TVA, de clients), le format attendu (fichier pour un logiciel comptable précis, ou état imprimable), le logiciel comptable des clients visés, et le traitement des avoirs et des retenues. Ces règles doivent venir d'une source vérifiée (texte de référence ou comptable), pas de ce brouillon.

### 2.3 Gestion de stock réelle

- Recouvre, selon la section 17 : entrées et sorties de stock, valorisation.
- Touche : le référentiel Produits, la réception des bons de commande (entrée) et la confirmation des bordereaux de livraison (sortie), déjà présentes en V1 ; le tableau de bord.
- À savoir : méthode de valorisation, gestion de plusieurs dépôts ou non, inventaire et ajustements, stock négatif permis ou refusé, produits qui ne sont pas stockés (services).

### 2.4 Envoi par email

- Recouvre : envoyer un document PDF (devis, facture, relance) au client depuis Kadenz.
- Touche : les PDF et les lettres de relance existants, la fiche client (adresse email), le journal d'audit.
- À savoir : envoi par le compte de messagerie de l'entreprise (réglage du serveur d'envoi) ou par un service tiers ; envoi manuel seulement, ou aussi automatique (relances à date) ; copie à l'expéditeur.

### 2.5 Multi-devise

- Recouvre : documents dans une autre devise que le FCFA (seule devise de la V1, section 13).
- Touche : montants et arrondi (aujourd'hui au franc), montant en lettres, PDF, règlements, tableau de bord, export comptable.
- À savoir : quelles devises ; taux saisi par document ou table de taux ; devise de tenue des comptes ; montant en lettres dans les autres devises.

### 2.6 Signature électronique

- Le terme recouvre des besoins très différents : image de signature ou cachet sur le PDF ; signature numérique du PDF (certificat) ; signature du client à distance sur un devis.
- À savoir : lequel est voulu, et s'il répond à une exigence légale ou à un usage commercial.

### 2.7 Site web

- Recouvre au minimum un site public. Son objet reste à préciser : site de présentation de Kadenz (le produit), site d'une entreprise cliente, ou espace client relié au serveur (2.1).
- Pour tout site public, la liste de contrôle SEO de l'utilisateur s'applique (plan du site, robots, balises canoniques et langues, titres et descriptions uniques, données structurées tirées du contenu réel, images, performances mobiles, Search Console préparée pour l'utilisateur).
- À savoir : objet, langues, nom de domaine, hébergement, contenus disponibles (textes, captures, tarifs) ; rien ne sera inventé pour remplir une page.

## 3. Dépendances entre candidats

- 2.1 (serveur) conditionne un espace client sur le site (2.7) et change la façon de faire la sauvegarde, la licence et les mises à jour.
- 2.2 (export comptable) est plus simple si 2.5 (multi-devise) n'est pas encore là, et doit être revu quand 2.5 arrive.
- 2.3 (stock) s'appuie sur les réceptions et livraisons de la V1 ; il ne dépend pas du serveur, mais un stock partagé entre postes en dépend.
- 2.4 (email) et 2.6 (signature) s'appuient sur les PDF existants et sont indépendants des autres.

## 4. Contraintes déjà connues

- Ne rien casser pour les installations V1 : une base V1 doit pouvoir être reprise (migration du schéma, jamais de perte).
- Les documents remis aux clients ne portent pas l'identité Kadenz.
- Les décisions de la V1 restent valables tant qu'elles ne sont pas revues explicitement (cahier V1, sections 18 et 19).

## 5. Questions ouvertes pour l'utilisateur

1. **Ordre** : quels candidats de la section 2 entrent dans la phase 2, et dans quel ordre ?
2. **Serveur** : réseau local de l'entreprise, hébergé sur Internet, ou les deux ? Fonctionnement hors connexion nécessaire ?
3. **Clients visés** : la V1 vise des PME, bureaux d'études et coopératives au Burkina Faso et dans l'UEMOA, en FCFA (cahier V1, sections 1 et 6). La phase 2 garde-t-elle ce cadre ? Combien de postes par entreprise ?
4. **SYSCOHADA** : quel format de sortie, pour quel logiciel comptable, et quelle source fait foi pour le plan de comptes (comptable, texte de référence) ?
5. **Stock** : méthode de valorisation, un ou plusieurs dépôts, stock négatif permis ?
6. **Email** : messagerie de l'entreprise ou service tiers ; relances envoyées automatiquement ou à la main ?
7. **Multi-devise** : quelles devises, et comment le taux est-il fixé ?
8. **Signature électronique** : image ou cachet, certificat numérique, ou signature du client à distance ?
9. **Site web** : objet (présentation de Kadenz, site d'un client, espace client), langues, nom de domaine, contenus disponibles ?
10. **« Suite comparable à Sage »** : quels produits de la suite viennent après la gestion commerciale (comptabilité, paie, autre) ? Cela décide de l'architecture commune.
11. **Licence** : la licence perpétuelle par poste reste-t-elle avec un serveur, ou un autre modèle (par entreprise, par abonnement) est-il envisagé ?

## 6. Hors de ce document

- La distribution de la V1 (signature du code, recherche d'antériorité de la marque) : question ouverte 8 du cahier V1, traitée à part.
- L'essai complet de la V1 par l'utilisateur.
