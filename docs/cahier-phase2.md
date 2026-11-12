# Kadenz - Cahier de la phase 2 (brouillon)

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-12 | Premier brouillon : candidats repris de la section 17 du cahier V1 et de la visée annoncée le 2026-11-11 (serveur, site web, suite comparable à Sage), dépendances et questions ouvertes. Rien n'est décidé ni codé. |
| 0.2 | 2026-11-12 | Premières décisions de l'utilisateur : les sept candidats entrent dans la phase 2, plus les chantiers jugés nécessaires ; serveur installable chez le client et hébergé ; comptabilité comme produit suivant de la suite ; licence par abonnement. Chantiers de fondation ajoutés (proposés, à valider), ordre proposé, questions mises à jour. |
| 0.3 | 2026-11-12 | Décisions suivantes : travail hors connexion, lecture seule à l'échéance, licences V1 perpétuelles, chantiers et ordre de la 0.2 validés. Relecture de kadenz-34 (stockage, licence, installateur) : étape 0 « pont V1 » proposée, points techniques de la section 6, nouvelles questions dont la numérotation hors connexion. |
| 0.4 | 2026-11-12 | Décisions : validation au retour du serveur pour numéroter, données hébergées jamais lisibles par l'éditeur, serveur chez le client sous Windows seulement, pont V1 validé. Conséquences notées (chiffrement côté client face aux traitements du serveur, règles de conflit de la synchronisation). |
| 0.5 | 2026-11-12 | Décisions 13 à 15 : serveur aveugle (il ne lit jamais les données), versions de la phase 2 publiées dans le même dépôt sans jamais le drapeau Latest, cadrage du pont V1. Rédaction de kadenz-34 : règles du pont V1 (4.0), protocole de synchronisation et de numérotation à la place d'une API métier (4.3), mise en oeuvre du serveur aveugle et ses conséquences (4.4) ; question 8.2 réglée. |

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

1. **Périmètre** : les sept candidats de la section 3 entrent dans la phase 2 (multi-poste et serveur, export SYSCOHADA, stock, email, multi-devise, signature électronique, site web), plus « tout ce que tu vas juger nécessaire pour la suite » : ces ajouts sont en section 4.
2. **Serveur** : les deux formes, installable sur le réseau local de l'entreprise et hébergé sur Internet.
3. **Suite** : la comptabilité est le produit qui suit la gestion commerciale.
4. **Licence** : abonnement.
5. **Hors connexion** : un poste doit pouvoir travailler quand le serveur ou Internet est coupé, puis resynchroniser. La numérotation hors connexion est réglée par la décision 9.
6. **Échéance de l'abonnement** : à la fin d'un abonnement non renouvelé, Kadenz passe en lecture seule.
7. **Licences V1 déjà émises** : elles restent perpétuelles ; l'abonnement concerne le serveur et les nouvelles licences.
8. **Plan** : les chantiers ajoutés 4.1 à 4.5 et l'ordre de la section 5 sont validés tels quels.
9. **Numérotation hors connexion** : validation au retour. Hors connexion, on saisit et on garde les documents en brouillon ; la validation, qui attribue le numéro, se fait quand le serveur répond. Les numéros restent continus et sans doublon ; un document à numéro définitif ne peut donc pas être remis au client avant ce retour. L'utilisateur a évoqué un numéro fondé sur le temps, comme les codes des applications de double authentification : un tel code garantit l'unicité à un instant donné, mais pas une suite continue sans trou par type, année et mois, règle de la V1 ; il n'est pas retenu.
10. **Données hébergées** : jamais lisibles par l'éditeur ; le client seul garde la clé. Une clé perdue rend les données perdues (voir 4.4).
11. **Système du serveur installé chez le client** : Windows seulement (service Windows, un seul installateur).
12. **Pont V1** : l'étape 0 (4.0) est validée ; elle passe avant toute release de la phase 2.
13. **Serveur aveugle** : pour tenir la décision 10, le serveur ne lit jamais les données. Il relaie la synchronisation chiffrée, attribue les numéros et tient un journal en ajout seul ; les traitements restent sur les postes (4.3 et 4.4).
14. **Publication de la phase 2** : dans le même dépôt GitHub, sans jamais le drapeau Latest, réservé à la V1 ; `scripts/release.ps1` fera respecter cette règle (4.0).
15. **Cadrage du pont V1** : les règles a à e de 4.0 sont validées, y compris la génération des deux nouvelles clés par l'utilisateur avec `kadenz-editeur` au moment de coder.

## 3. Candidats

Chaque candidat décrit ce qu'il recouvrirait, ce qu'il touche dans la V1 et ce qu'il faut savoir avant de le spécifier.

### 3.1 Multi-poste et serveur (retenu : réseau local et hébergé, travail hors connexion)

- Recouvre : plusieurs postes qui travaillent sur les mêmes données en même temps ; comptes communs ; numérotation des documents sans doublon entre postes ; travail hors connexion puis resynchronisation (décision 5).
- Touche : le stockage entier (aujourd'hui un fichier SQLite local), la clé de chiffrement liée au poste, la sauvegarde, la licence liée au poste, la mise à jour (serveur et postes à la même version du schéma).
- Conséquence de la décision 2 : un seul serveur, livré sous deux formes (installé chez le client, ou exploité par l'éditeur), et non deux produits. La forme hébergée ajoute l'exploitation par l'éditeur (hébergement, sécurité réseau, disponibilité, sauvegardes côté serveur, séparation des données entre entreprises clientes).
- Conséquence de la décision 5 : chaque poste garde une copie locale des données dont il a besoin et une file des opérations faites hors connexion ; au retour du réseau, le serveur les applique et signale les conflits. Selon kadenz-34, la base SQLite chiffrée de la V1 peut servir de copie locale.
- Conséquence de la décision 9 : hors connexion, les documents restent en brouillon ; la validation et la numérotation se font sur le serveur.
- Conflits à régler (kadenz-34) : fiches tiers et produits modifiées sur deux postes, droits changés ou compte désactivé pendant une coupure, et plus tard le stock (deux postes hors connexion qui vendent le même dernier article). Qui l'emporte : question 8.1.
- Reste ouvert : stockage côté serveur. Avec la décision 13, il ne conserve que des données chiffrées, des compteurs de numérotation et un journal (4.4).

### 3.2 Export comptable SYSCOHADA

- Recouvre, selon la section 17 du cahier V1 : journal des ventes et état de la TVA collectée.
- Touche : factures, avoirs et règlements validés (déjà figés à la validation), paramètres de TVA.
- Lien avec la décision 3 : la comptabilité devenant le produit suivant, l'export doit être pensé comme la première brique de ce produit (mêmes écritures, même plan de comptes), pas comme un fichier isolé.
- À savoir avant de spécifier : le plan de comptes à utiliser (comptes de ventes, de TVA, de clients), le format attendu, le traitement des avoirs et des retenues. Ces règles doivent venir d'une source vérifiée (texte de référence ou comptable), pas de ce brouillon.

### 3.3 Gestion de stock réelle

- Recouvre, selon la section 17 du cahier V1 : entrées et sorties de stock, valorisation.
- Touche : le référentiel Produits, la réception des bons de commande (entrée) et la confirmation des bordereaux de livraison (sortie), déjà présentes en V1 ; le tableau de bord.
- À savoir : méthode de valorisation, gestion de plusieurs dépôts ou non, inventaire et ajustements, stock négatif permis ou refusé, produits qui ne sont pas stockés (services). Avec le travail hors connexion, deux postes peuvent sortir le même article sans se voir : le stock négatif est alors possible de fait.

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
- Conséquence de la décision 13 : un espace client ne peut montrer les données de l'entreprise qu'en les déchiffrant dans le navigateur, avec la clé du client ; sinon il se limite à l'abonnement et aux téléchargements.

## 4. Chantiers ajoutés

Les chantiers 4.1 à 4.5 sont validés (décision 8), le chantier 4.0, proposé par kadenz-34, aussi (décisions 12 et 15).

### 4.0 Pont V1 (validé, décisions 12, 14 et 15)

Une petite version V1, publiée avant toute version de la phase 2, pour que les postes déjà installés ne soient pas mis en difficulté par celle-ci. Constats vérifiés dans le code le 2026-11-12 :

- **Mises à jour** : `kadenz-maj` interroge `releases/latest` (`crates/maj/src/lib.rs`, `DERNIERE_RELEASE`) et propose toute version plus grande. Une version de la phase 2 publiée en Latest serait proposée à tous les postes V1 sous licence perpétuelle (décision 7). Il faut d'abord livrer une règle de compatibilité : même version majeure, champ « mise à jour possible depuis » dans la release, ou canal distinct.
- **Clé de licence** : une seule clé publique embarquée, sans identifiant (`crates/licence/src/lib.rs`, `CLE_PUBLIQUE`). Le portail d'abonnement (4.1) mettra une clé de signature en ligne ; les postes installés doivent déjà accepter une liste de clés avec un identifiant, pour permettre une rotation.
- **Intégrité des mises à jour** : elle repose sur le fichier `.sha256` publié dans la même release ; un compte GitHub compromis suffirait à piéger la mise à jour de tous les postes. Une signature Ed25519 du manifeste de mise à jour, indépendante du certificat de code, ferme ce risque.

Règles retenues (décisions 14 et 15) :

- **a. Drapeau Latest réservé à la V1.** Les postes en v0.2.1 et avant interrogent `releases/latest` et le feront tant qu'ils n'auront pas installé le pont. Aucune version de la phase 2 ne porte donc jamais ce drapeau, bien que publiée dans le même dépôt. `scripts/release.ps1` le fait respecter : une version d'une autre génération que la V1 est publiée sans le drapeau Latest, et le script refuse de la marquer ainsi.
- **b. Manifeste de mise à jour signé.** Chaque release joint un fichier `kadenz-maj.json` : version, génération (`v1` pour la V1), version minimale depuis laquelle la mise à jour est permise, nom et empreinte SHA-256 de l'installateur. Il est signé en Ed25519 par une clé de mise à jour, distincte de celle des licences. Un poste qui a installé le pont parcourt la liste des releases, et non plus seulement `releases/latest`. Il ne retient que les manifestes à signature valide, de sa génération, et dont la version minimale ne dépasse pas la sienne. L'empreinte de l'installateur reste vérifiée avant tout lancement, comme aujourd'hui.
- **c. Clés de licence avec identifiant.** Le format de la licence reçoit un identifiant de clé, et l'application une liste de clés publiques acceptées. Une licence déjà émise, sans identifiant, reste vérifiée par la clé actuelle : les licences perpétuelles (décision 7) restent valables sans rien faire. Le pont embarque dès maintenant la clé publique suivante, générée au moment de coder et gardée hors ligne. Le portail d'abonnement (4.1) pourra ainsi changer de clé de signature sans nouvelle mise à jour des postes V1.
- **d. Clés générées par l'utilisateur.** La clé de mise à jour et la clé de licence suivante sont générées par l'utilisateur avec `kadenz-editeur`, au moment de coder. Leurs parties privées restent hors du dépôt, comme la clé de licence actuelle ; seules les clés publiques sont embarquées.
- **e. Écrans.** Presque rien ne change : deux ou trois messages traduits dans les sept langues (mise à jour refusée pour signature invalide ; existence d'une nouvelle génération de Kadenz, qui s'installe séparément, si on choisit de l'annoncer).

### 4.1 Abonnement et portail de licences

- Pourquoi : la licence par abonnement (décision 4) remplace, pour les nouvelles licences, une clé perpétuelle vérifiée hors ligne par une licence datée qu'il faut renouveler.
- Recouvre : émission et renouvellement des licences (aujourd'hui l'outil `kadenz-editeur`, manuel) ; suivi des abonnements par entreprise ; passage en lecture seule à l'échéance (décision 6) ; paiement.
- Touche : la licence V1 (section 15 du cahier V1 : Ed25519, liaison au poste, modules), l'assistant de premier lancement, le site (3.7).
- Points techniques (kadenz-34) : une licence datée vérifiée hors connexion (décision 5) dépend de l'horloge du poste, l'échéance y compris (décision 6), il faut un garde-fou contre le retour en arrière (dernière date vue, conservée) et l'heure du serveur quand il est joignable ; le lien au poste ne convient plus, la licence se lie au serveur en réseau local ou à l'entreprise cliente en hébergé, selon l'unité choisie (question 8.5).
- Lecture seule : aucune échéance n'empêche de consulter, d'imprimer ni d'exporter les données déjà saisies (les factures émises doivent rester consultables) ; à confirmer (question 8.4).

### 4.2 Reprise des installations V1

- Pourquoi : une base V1 doit pouvoir être reprise sans perte (section 7) ; avec un serveur, il faut aussi pouvoir verser une base mono-poste dans le serveur.
- Recouvre : migration du schéma, import d'une base V1 (ou d'une sauvegarde `.kzs`) dans le serveur, numérotation continue après la reprise.
- Point ouvert (kadenz-34) : une base V1 correspond à une entreprise. Fusionner plusieurs installations V1 indépendantes d'une même entreprise fait entrer des numéros en collision (deux factures F2026-05-0001, par exemple) ; à exclure ou à spécifier (question 8.3). Importer un `.kzs` suppose que le serveur lise le format chiffré de la V1.

### 4.3 Socle commun de la suite

- Pourquoi : la comptabilité vient ensuite (décision 3) ; ce qui est commun aux deux produits doit être séparé avant de bâtir le serveur, pour ne pas le refaire.
- Recouvre : entreprise, comptes et droits, tiers (clients et fournisseurs), paramètres fiscaux, licence et modules, journal d'audit, partagés par les produits de la suite.
- Frontière entre postes et serveur (kadenz-34, revue avec la décision 13) : ce n'est pas une API métier côté serveur, mais un protocole de synchronisation et de numérotation. Chaque poste garde sa base locale chiffrée et la logique métier : saisie, calculs, droits, validation, PDF, exports, comme en V1. Le serveur ne connaît que des opérations chiffrées à relayer entre les postes, les compteurs de numérotation et le journal. Ce protocole est à fixer ici, avant tout écran multi-poste ; le stockage de la V1 en est la base côté poste.

### 4.4 Sécurité et exploitation du serveur

- Pourquoi : la forme hébergée (décision 2) met des données d'entreprises clientes sur un serveur exploité par l'éditeur.
- Recouvre : authentification des postes et des comptes, chiffrement des échanges, séparation des données entre entreprises, sauvegardes côté serveur, journal des accès, mises à jour du serveur.
- Points techniques (kadenz-34) : une base par entreprise (séparation naturelle, sauvegarde et restauration par client, réutilisation du format `.kzs`) ou une base partagée entre clients, choix structurant ; la clé de chiffrement actuelle, protégée par un compte Windows, ne se transpose pas à un service ; côté serveur, les sauvegardes deviennent planifiées, avec rotation et restauration par l'administrateur, le `.kzs` restant le format d'échange.
- Mise en oeuvre de la décision 10 : serveur aveugle (décision 13). L'autre voie examinée, un serveur qui déchiffre en mémoire avec une clé fournie par le client, a été écartée : l'exploitant aurait la clé en mémoire pendant les sessions et pourrait lire les données. Cette voie ne protège que les données au repos et les sauvegardes. Le serveur aveugle prolonge le travail hors connexion (décision 5), qui impose déjà aux postes une copie locale et les traitements. Le même serveur sert l'installation chez le client et la forme hébergée.
- **Numérotation** : à la validation, le poste n'envoie au serveur que le type de document, l'année et le mois ; le serveur rend le rang suivant de façon atomique (règle de `prochain_rang` en V1) et le poste scelle le document. Ce que le serveur voit, à écrire tel quel dans la spécification : le type, le mois et le rythme des validations, jamais les montants, les lignes ni les tiers.
- **Clés** : une clé de données par entreprise, tirée de l'aléa du système. Elle est enveloppée pour chaque compte (clé dérivée du mot de passe par Argon2, déjà utilisé en V1 pour les mots de passe), pour chaque poste (protection Windows, comme `kadenz.cle` en V1) et par une clé de récupération imprimée sur papier, remise au client. L'éditeur n'en détient aucune. Tout perdre, comptes, postes et clé de récupération, rend les données perdues, comme en V1 sans sauvegarde : à dire au client.
- **Droits et preuve** : les droits sont appliqués par les postes, comme en V1 ; le serveur ne peut pas les imposer. Pour l'attribution, chaque opération est signée par le compte qui la fait (Ed25519). Le journal du serveur est chaîné par empreintes : il reste vérifiable et infalsifiable, bien que chiffré.
- **Conséquences sur les autres chantiers** : l'envoi par email (3.4) et l'export SYSCOHADA (3.2) partent des postes ; l'espace client du site (3.7) ne montre les données qu'en les déchiffrant dans le navigateur ; les sauvegardes côté serveur sont des blocs chiffrés, que seul le client peut restaurer avec sa clé ; la reprise d'une base V1 (4.2) se fait par un poste, qui lit le `.kzs` et verse les données chiffrées dans le serveur.

### 4.5 Signature du code

- Pourquoi : déjà spécifiée en V1 (section 16 du cahier V1) mais en attente du certificat ; un service serveur non signé avec une exception de pare-feu est bloqué bien plus souvent qu'une application (kadenz-34), et les mises à jour des postes et du serveur en dépendent.
- Placée dans les fondations. Reste une décision et un achat de l'utilisateur (question 8 du cahier V1).

## 5. Ordre (validé, décision 8)

0. **Pont V1** (4.0, décision 12) : avant toute release de la phase 2.
1. **Fondations** : socle commun (4.3), serveur sous ses deux formes (3.1), sécurité (4.4), reprise des bases V1 (4.2), abonnement (4.1), signature du code (4.5). Ces chantiers décident de l'architecture ; les faire après les fonctions obligerait à refaire celles-ci.
2. **Fonctions métier sur le nouveau socle** : stock (3.3), puis export SYSCOHADA (3.2) comme première brique de la comptabilité.
3. **Chantiers indépendants**, intercalables selon la demande des clients : email (3.4), signature électronique (3.6), multi-devise (3.5, à faire avant l'export SYSCOHADA s'il doit en tenir compte).
4. **Site web** (3.7) : site de présentation possible dès maintenant ; espace client et abonnement après 4.1 et 3.1.

## 6. Points techniques et dépendances

### 6.1 Dépendances entre chantiers

- 4.0 (pont V1) précède toute release de la phase 2.
- 3.1 (serveur) conditionne un espace client sur le site (3.7) et change la façon de faire la sauvegarde, la licence et les mises à jour.
- 4.3 (protocole de synchronisation et de numérotation) précède les écrans multi-poste et 4.2 (reprise par un poste) ; 4.4 (clés de l'entreprise et de récupération) précède toute synchronisation réelle.
- 4.1 (abonnement) dépend du garde-fou d'horloge et du lien au serveur ou à l'entreprise (4.1, points techniques).
- 3.2 (export comptable) est plus simple si 3.5 (multi-devise) n'est pas encore là, et doit être revu quand 3.5 arrive.
- 3.3 (stock) s'appuie sur les réceptions et livraisons de la V1 ; un stock partagé entre postes dépend du serveur.
- 3.4 (email) et 3.6 (signature) s'appuient sur les PDF existants et sont indépendants des autres.

### 6.2 Points relevés dans le code (kadenz-34, 2026-11-12)

- **Numérotation** : le numéro d'un document est attribué à la validation, sans trou, par type, année et mois (`prochain_rang`, `crates/stockage/src/lib.rs`). Un poste coupé du serveur ne peut pas l'attribuer seul sans risque de doublon, et réserver des blocs de numéros par poste casse la continuité ; d'où la décision 9.
- **Horodatage** : `maintenant()` (`crates/stockage/src/lib.rs`) écrit l'heure locale sans fuseau. Avec plusieurs postes et un serveur hébergé, peut-être dans un autre fuseau, les horodatages techniques (audit, création, validation) doivent venir du serveur, avec le fuseau ; les dates métier (date de la facture) restent saisies.
- **Installateur du serveur** (réseau local sous Windows) : service Windows, règle de pare-feu, certificat TLS sur le réseau local (auto-signé, donc confiance ou épinglage par les postes), moyen pour les postes de trouver le serveur. Windows seulement (décision 11).
- **Désinstallation** : à l'inverse de la V1, désinstaller le serveur ne doit jamais effacer les données de l'entreprise ; sur un poste client en multi-poste, les données locales ne sont qu'une copie de travail, à condition que les opérations faites hors connexion aient été envoyées au serveur.

## 7. Contraintes déjà connues

- Ne rien casser pour les installations V1 : une base V1 doit pouvoir être reprise (migration du schéma, jamais de perte), et ses licences restent perpétuelles (décision 7).
- Les documents remis aux clients ne portent pas l'identité Kadenz.
- Les décisions de la V1 restent valables tant qu'elles ne sont pas revues explicitement (cahier V1, sections 18 et 19). La décision 4 revoit la licence perpétuelle de la section 15 du cahier V1 pour les nouvelles licences seulement.

## 8. Questions ouvertes pour l'utilisateur

### Réglées le 2026-11-12

- Périmètre, forme du serveur, produit suivant, modèle de licence, hors connexion, échéance, licences V1, plan, numérotation hors connexion, données hébergées, système du serveur, pont V1 (décisions 1 à 12) ; serveur aveugle, publication de la phase 2, cadrage du pont V1 (décisions 13 à 15).

### Nouvelles

1. **Conflits de synchronisation** : quand une fiche (client, fournisseur, produit) a été modifiée sur deux postes pendant une coupure, qui l'emporte : la dernière modification, le serveur, ou une personne qui choisit ? Et un compte désactivé ou des droits changés pendant la coupure prennent-ils effet dès le retour, en annulant les opérations faites hors connexion par ce compte ?
2. **Données hébergées, mise en oeuvre** : réglée le 2026-11-12, serveur aveugle (décision 13, conséquences en 4.4).
3. **Fusion d'installations V1** : une entreprise qui a plusieurs postes V1 indépendants peut-elle les fusionner dans un même serveur (avec renumérotation ou séries distinctes), ou la reprise se limite-t-elle à une base V1 par entreprise ?
4. **Lecture seule** : confirmer qu'à l'échéance la consultation, l'impression et l'export des données déjà saisies restent toujours possibles.
5. **Abonnement, forme** : période (mensuelle, annuelle), unité (par entreprise, par poste, par utilisateur), moyen de paiement envisagé, délai de grâce.

### Toujours ouvertes

6. **Clients visés** : la V1 vise des PME, bureaux d'études et coopératives au Burkina Faso et dans l'UEMOA, en FCFA (cahier V1, sections 1 et 6). La phase 2 garde-t-elle ce cadre ? Combien de postes par entreprise ?
7. **SYSCOHADA** : quel format de sortie, pour quel logiciel comptable en attendant la comptabilité Kadenz, et quelle source fait foi pour le plan de comptes (comptable, texte de référence) ?
8. **Stock** : méthode de valorisation, un ou plusieurs dépôts, stock négatif permis ?
9. **Email** : messagerie de l'entreprise ou service tiers ; relances envoyées automatiquement ou à la main ?
10. **Multi-devise** : quelles devises, et comment le taux est-il fixé ?
11. **Signature électronique** : image ou cachet, certificat numérique, ou signature du client à distance ?
12. **Site web** : objet (présentation de Kadenz, espace client, abonnement), langues, nom de domaine, contenus disponibles ?
13. **Hébergement** : pour la forme hébergée, où (pays, prestataire) et qui l'exploite ?

## 9. Hors de ce document

- La distribution de la V1 (recherche d'antériorité de la marque) : question ouverte 8 du cahier V1, traitée à part ; la signature du code est reprise en 4.5.
- L'essai complet de la V1 par l'utilisateur.
