# Kadenz - Cahier de la phase 2 (brouillon)

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-12 | Premier brouillon : candidats repris de la section 17 du cahier V1 et de la visée annoncée le 2026-11-11 (serveur, site web, suite comparable à Sage), dépendances et questions ouvertes. Rien n'est décidé ni codé. |
| 0.2 | 2026-11-12 | Premières décisions de l'utilisateur : les sept candidats entrent dans la phase 2, plus les chantiers jugés nécessaires ; serveur installable chez le client et hébergé ; comptabilité comme produit suivant de la suite ; licence par abonnement. Chantiers de fondation ajoutés (proposés, à valider), ordre proposé, questions mises à jour. |
| 0.3 | 2026-11-12 | Décisions suivantes : travail hors connexion, lecture seule à l'échéance, licences V1 perpétuelles, chantiers et ordre de la 0.2 validés. Relecture de kadenz-34 (stockage, licence, installateur) : étape 0 « pont V1 » proposée, points techniques de la section 6, nouvelles questions dont la numérotation hors connexion. |
| 0.4 | 2026-11-12 | Décisions : validation au retour du serveur pour numéroter, données hébergées jamais lisibles par l'éditeur, serveur chez le client sous Windows seulement, pont V1 validé. Conséquences notées (chiffrement côté client face aux traitements du serveur, règles de conflit de la synchronisation). |
| 0.5 | 2026-11-12 | Décisions 13 à 15 : serveur aveugle (il ne lit jamais les données), versions de la phase 2 publiées dans le même dépôt sans jamais le drapeau Latest, cadrage du pont V1. Rédaction de kadenz-34 : règles du pont V1 (4.0), protocole de synchronisation et de numérotation à la place d'une API métier (4.3), mise en oeuvre du serveur aveugle et ses conséquences (4.4) ; question 8.2 réglée. |
| 0.6 | 2026-11-12 | Pont V1 publié en v0.2.2. Décisions 16 à 24 pour les fondations : serveur sur une machine de l'entreprise, accès à distance optionnel par un relais sur le VPS de l'éditeur ; conflits tranchés par une personne ; opérations d'un compte désactivé hors connexion mises en attente ; une seule base V1 reprise ; lecture seule avec consultation, impression et export ; forme de l'abonnement ; zones OHADA ; 5 à 20 postes ; certificat plus tard. Questions mises à jour. |
| 0.7 | 2026-11-12 | Décisions 25 à 27 : avertissements d'échéance à 30 jours, 7 jours et la veille, à l'Admin puis à tous les comptes dans les derniers jours ; revendeurs avec un compte sur le portail. |
| 0.8 | 2026-11-12 | Décision 28 : serveur de confiance chez l'entreprise, base chiffrée au repos, à la place du serveur aveugle (décision 13), le relais restant aveugle par un chiffrement de bout en bout. Décision 29 : licences signées par une clé de portail en ligne, avec un émetteur dans chaque licence, aucune clé chez les revendeurs. Sections 4.3 et 4.4 à reprendre par kadenz-34 dans `docs/spec-synchronisation.md`. |
| 0.9 | 2026-11-12 | Sections 4.3 et 4.4 reprises pour le serveur de confiance (décision 28) ; le protocole de synchronisation, la numérotation et les clés sont spécifiés à part, dans `docs/spec-synchronisation.md` (premier jet de kadenz-34, à l'étude). |
| 0.10 | 2026-11-12 | Décisions 30 à 34 : petite fenêtre d'administration sur le serveur, toujours un serveur (pas de poste seul en phase 2), toutes les opérations hors connexion refusées en attente, conflits tranchés par l'Admin ou un ayant droit, version entière ou champ par champ. |
| 0.11 | 2026-11-12 | Décisions 35 à 37 (recommandations de kadenz-34, validées) : sauvegardes quotidiennes et à l'arrêt du serveur, dix gardées ; code d'inscription valable 24 heures ; fuseau unique de l'entreprise. |

Statut : **à l'étude** pour tout le document. Les décisions de la section 2 sont prises ; aucun bloc n'est encore spécifié ni codé. Aucun chiffre, aucune règle comptable ou fiscale et aucun choix technique n'est retenu ici sans source vérifiée : là où il en faut, une question est posée.

## 1. Point de départ

D'après le cahier V1 (`docs/cahier-des-charges.md`) :

- Kadenz V1 est une application Windows mono-poste, en Rust et Slint ; jalons J1 à J10 et J8b livrés (section 19) ; dernière version publiée : v0.2.2, pont V1 (2026-11-12).
- Données dans une base SQLite locale unique, chiffrée (clé du poste protégée par Windows), sauvegardes chiffrées par mot de passe (sections 10.3 et 13).
- Comptes et rôles sur le poste (Admin, Comptable, Commercial, Consultation), matrice des droits modifiable (section 10.1).
- Modules activés par clé de licence perpétuelle liée au poste, vérifiée hors ligne (sections 3.2 et 15).
- Documents : Devis, Proforma, Bon de commande, Livraison, Facture, Avoir ; règlements, relances, tableau de bord ; PDF et exports Excel ; interface en sept langues, documents clients en français (sections 6 à 9, 12 et 13).
- Hors périmètre V1 (section 17) : gestion de stock réelle, envoi par email, signature électronique, export comptable SYSCOHADA, multi-devise, multi-poste en réseau.

Visée annoncée par l'utilisateur le 2026-11-11 : une phase suivante plus grande, avec un serveur, de nouvelles fonctions et un site web ; à terme, une suite de logiciels comparable à Sage.

## 2. Décisions de l'utilisateur (2026-11-12)

1. **Périmètre** : les sept candidats de la section 3 entrent dans la phase 2 (multi-poste et serveur, export SYSCOHADA, stock, email, multi-devise, signature électronique, site web), plus « tout ce que tu vas juger nécessaire pour la suite » : ces ajouts sont en section 4.
2. **Serveur** : les deux formes, installable sur le réseau local de l'entreprise et hébergé sur Internet ; précisé par la décision 16.
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
13. **Serveur aveugle** (remplacée par la décision 28) : pour tenir la décision 10, le serveur ne lit jamais les données. Il relaie la synchronisation chiffrée, attribue les numéros et tient un journal en ajout seul ; les traitements restent sur les postes (4.3 et 4.4).
14. **Publication de la phase 2** : dans le même dépôt GitHub, sans jamais le drapeau Latest, réservé à la V1 ; `scripts/release.ps1` fera respecter cette règle (4.0).
15. **Cadrage du pont V1** : les règles a à e de 4.0 sont validées, y compris la génération des deux nouvelles clés par l'utilisateur avec `kadenz-editeur` au moment de coder. Pont publié en v0.2.2 le 2026-11-12.
16. **Serveur et accès à distance** (précise la décision 2) : le serveur Kadenz est installé sur une machine de l'entreprise ; les postes du bureau s'y connectent en réseau local. Un accès à distance peut être activé en option : les postes hors du bureau passent alors par un relais sur le VPS de l'éditeur, que l'utilisateur va prendre, et qui portera aussi le site web, dans des conteneurs. Le relais ne fait que transmettre les échanges chiffrés entre le poste distant et le serveur de l'entreprise, sans rien conserver ; si ce serveur est éteint, le poste distant travaille hors connexion (décision 5). Les données de l'entreprise ne sont donc jamais stockées chez l'éditeur.
17. **Conflits de synchronisation** : quand une fiche (client, fournisseur, produit) a été modifiée sur deux postes pendant une coupure, le conflit est signalé et une personne (l'Admin, ou la personne qui synchronise) compare les deux versions et choisit. Rien n'est écarté en silence.
18. **Compte désactivé ou droits réduits pendant une coupure** : les opérations faites hors connexion par ce compte après le changement sont mises en attente ; l'Admin les voit et les accepte ou les rejette.
19. **Reprise V1** : une seule base V1 est reprise dans le serveur d'une entreprise ; les autres installations V1 restent consultables à part. Pas de fusion.
20. **Lecture seule** : à l'échéance, la consultation, l'impression et l'export (PDF, Excel) des données déjà saisies restent toujours possibles ; seules la saisie et la validation sont bloquées.
21. **Forme de l'abonnement** : mensuel ou annuel, au choix du client ; compté par entreprise, avec un nombre de postes inclus et des postes en plus ; paiement par virement ou chèque, par carte bancaire en ligne, ou en espèces auprès de l'éditeur ou d'un revendeur (Mobile Money non retenu pour l'instant) ; aucun délai de grâce : lecture seule dès le lendemain de l'échéance, avec des avertissements avant.
22. **Zones visées** : plus large que la V1 : UEMOA (franc CFA de la BCEAO), CEMAC (franc CFA de la BEAC) et autres pays OHADA (Guinée, Comores, RD Congo). Les devises, la TVA et les règles de facturation de chaque pays sont à vérifier sur des sources (question 8.8).
23. **Taille des entreprises clientes** : 5 à 20 postes en général.
24. **Certificat de signature du code** : plus tard ; il faudra l'avoir avant la première mise en service du serveur chez un client.
25. **Avertissements d'échéance** : Kadenz prévient 30 jours avant la fin de l'abonnement, puis 7 jours avant, puis la veille.
26. **Destinataires des avertissements** : l'Admin seulement au début, puis tous les comptes dans les derniers jours (à partir du rappel des 7 jours, précision à confirmer à la spécification de 4.1).
27. **Revendeurs** : un revendeur a un compte sur le portail ; il enregistre lui-même les paiements de ses clients et renouvelle leurs licences. L'éditeur garde la vue sur tout.
28. **Serveur de confiance** (remplace la décision 13, à la suite de la décision 16) : le serveur étant chez l'entreprise et non chez l'éditeur, il lit ses données, chiffrées au repos sur son disque ; il impose les droits, l'heure, la validation et la numérotation, aide à trancher les conflits et fait des sauvegardes lisibles par l'entreprise. Le relais du VPS reste aveugle : les échanges entre un poste distant et le serveur sont chiffrés de bout en bout, l'éditeur ne lit rien (décision 10 tenue). Risque accepté : une personne qui vole la machine serveur en marche, ou qui l'administre dans l'entreprise, peut lire les données.
29. **Clé du portail et revendeurs** : le portail signe toutes les licences d'abonnement avec une seule clé de portail, en ligne sur le VPS ; chaque licence porte, dans ses données signées, son émetteur (l'éditeur ou l'identifiant du revendeur). Les revendeurs n'ont jamais de clé, ils agissent par leur compte sur le portail, qui garde la trace. La clé de portail est la clé suivante embarquée par le pont V1 (`crates/licence/cle-publique-suivante.hex`) ; la clé actuelle reste hors ligne chez l'éditeur pour les licences V1. Si le VPS est compromis, la clé du portail est révoquée et remplacée par une mise à jour (recommandation de kadenz-34, validée).
30. **Administration du serveur** : une petite fenêtre sur la machine du serveur, limitée à ce qui doit marcher quand aucun poste ne peut se connecter : état du service, empreinte du certificat, premier compte Admin, codes d'inscription des postes, restauration d'une sauvegarde. Tout le reste (comptes, accès à distance, abonnement, sauvegardes planifiées) se fait depuis un poste connecté avec un compte Admin.
31. **Toujours un serveur** : tout poste de la phase 2 est relié à un serveur, qui peut tourner sur la même machine pour une entreprise d'un seul poste. Pas de mode « poste seul » dans la phase 2 ; la V1 reste disponible pour cet usage, avec ses licences perpétuelles.
32. **Opérations en attente** (précise la décision 18) : toute opération faite hors connexion et refusée pour un compte ou un droit va en attente, qu'elle soit antérieure ou postérieure au changement ; l'heure du poste n'est affichée qu'à titre indicatif, l'Admin décide.
33. **Qui tranche un conflit** (précise la décision 17) : l'Admin, ou la personne qui synchronise si elle a le droit de modifier la fiche concernée.
34. **Granularité d'un conflit** : on garde une version entière en un clic, ou on choisit champ par champ.
35. **Sauvegardes du serveur** : une sauvegarde automatique par jour, à une heure réglable, plus une à l'arrêt du service ; les dix dernières gardées, comme en V1 ; dossier par défaut sur la machine serveur, modifiable dans la fenêtre d'administration ; avertissement permanent, dans cette fenêtre et sur les postes Admin, tant que le dossier est sur le même disque que la base (copie hors de la machine conseillée, seul recours en cas de vol ou de panne).
36. **Code d'inscription d'un poste** : à usage unique, valable 24 heures, annulable par l'Admin ; un nouveau code annule le précédent pour le même poste.
37. **Fuseau** : un seul fuseau par entreprise, celui de Windows sur la machine serveur ; les heures techniques sont gardées en UTC avec décalage et affichées dans ce fuseau sur tous les postes, distants compris ; les dates métier restent celles saisies.

## 3. Candidats

Chaque candidat décrit ce qu'il recouvrirait, ce qu'il touche dans la V1 et ce qu'il faut savoir avant de le spécifier.

### 3.1 Multi-poste et serveur (retenu : serveur chez le client, relais d'accès à distance, travail hors connexion)

- Recouvre : plusieurs postes qui travaillent sur les mêmes données en même temps ; comptes communs ; numérotation des documents sans doublon entre postes ; travail hors connexion puis resynchronisation (décision 5).
- Touche : le stockage entier (aujourd'hui un fichier SQLite local), la clé de chiffrement liée au poste, la sauvegarde, la licence liée au poste, la mise à jour (serveur et postes à la même version du schéma).
- Conséquence des décisions 2 et 16 : un seul serveur, installé sur une machine de l'entreprise (une entreprise par serveur), et un relais d'accès à distance exploité par l'éditeur sur son VPS. Le relais n'héberge aucune donnée ; il authentifie les connexions, les sépare par entreprise et doit rester disponible pour les postes distants. Dimensionnement : 5 à 20 postes par entreprise (décision 23).
- Conséquence de la décision 5 : chaque poste garde une copie locale des données dont il a besoin et une file des opérations faites hors connexion ; au retour du réseau, le serveur les applique et signale les conflits. Selon kadenz-34, la base SQLite chiffrée de la V1 peut servir de copie locale.
- Conséquence de la décision 9 : hors connexion, les documents restent en brouillon ; la validation et la numérotation se font sur le serveur.
- Conflits à régler (kadenz-34) : fiches tiers et produits modifiées sur deux postes, droits changés ou compte désactivé pendant une coupure, et plus tard le stock (deux postes hors connexion qui vendent le même dernier article). Règles retenues : décisions 17 et 18 ; le stock reste à traiter avec 3.3.
- Stockage côté serveur : base de l'entreprise chiffrée au repos, lue par le serveur (décision 28) ; détail dans `docs/spec-synchronisation.md` (kadenz-34, à l'étude).

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
- Avec la décision 22, la devise de tenue des comptes dépend du pays du client : franc CFA de la BCEAO, franc CFA de la BEAC, franc guinéen, franc comorien, franc congolais. La devise de l'entreprise devient un paramètre, distinct des documents en devise étrangère.

### 3.6 Signature électronique

- Le terme recouvre des besoins très différents : image de signature ou cachet sur le PDF ; signature numérique du PDF (certificat) ; signature du client à distance sur un devis.
- À savoir : lequel est voulu, et s'il répond à une exigence légale ou à un usage commercial.

### 3.7 Site web

- Recouvre au minimum un site public. Son objet reste à préciser : site de présentation de Kadenz (le produit), espace client relié au serveur (3.1), espace d'abonnement (4.1).
- Pour tout site public, la liste de contrôle SEO de l'utilisateur s'applique (plan du site, robots, balises canoniques et langues, titres et descriptions uniques, données structurées tirées du contenu réel, images, performances mobiles, Search Console préparée pour l'utilisateur).
- À savoir : objet, langues, nom de domaine, hébergement, contenus disponibles (textes, captures, tarifs) ; rien ne sera inventé pour remplir une page.
- Hébergement : sur le VPS de l'éditeur, dans des conteneurs, à côté du relais d'accès à distance (décision 16).
- Espace client : les données de l'entreprise ne sont jamais sur le VPS (décisions 16 et 28) ; un espace client sur le site se limite donc à l'abonnement, aux licences et aux téléchargements, sauf à passer par le relais vers le serveur de l'entreprise, chiffré de bout en bout.

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
- Recouvre : émission et renouvellement des licences (aujourd'hui l'outil `kadenz-editeur`, manuel) ; suivi des abonnements par entreprise ; passage en lecture seule à l'échéance (décisions 6 et 20) ; paiement.
- Émission (décision 29) : clé de portail en ligne, émetteur dans chaque licence, comptes revendeurs sans clé ; identifiant d'entreprise commun avec le protocole de synchronisation (kadenz-34).
- Forme retenue (décision 21) : mensuel ou annuel ; par entreprise avec postes inclus et postes en plus ; virement ou chèque, carte en ligne, espèces auprès de l'éditeur ou d'un revendeur ; pas de délai de grâce. Les paiements hors ligne supposent qu'une personne (l'éditeur, ou un revendeur) enregistre le paiement et renouvelle la licence ; la carte en ligne suppose un prestataire de paiement sur le site.
- Touche : la licence V1 (section 15 du cahier V1 : Ed25519, liaison au poste, modules), l'assistant de premier lancement, le site (3.7).
- Points techniques (kadenz-34) : une licence datée vérifiée hors connexion (décision 5) dépend de l'horloge du poste, l'échéance y compris (décision 6), il faut un garde-fou contre le retour en arrière (dernière date vue, conservée) et l'heure du serveur quand il est joignable ; le lien au poste ne convient plus : la licence se lie à l'entreprise, donc à son serveur, avec le nombre de postes permis (décision 21).
- Lecture seule : aucune échéance n'empêche de consulter, d'imprimer ni d'exporter les données déjà saisies (décision 20).

### 4.2 Reprise des installations V1

- Pourquoi : une base V1 doit pouvoir être reprise sans perte (section 7) ; avec un serveur, il faut aussi pouvoir verser une base mono-poste dans le serveur.
- Recouvre : migration du schéma, import d'une base V1 (ou d'une sauvegarde `.kzs`) dans le serveur, numérotation continue après la reprise.
- Fusion exclue (décision 19) : une base V1 correspond à une entreprise, et fusionner plusieurs installations V1 ferait entrer des numéros en collision (deux factures F2026-05-0001, par exemple) ; une seule base est reprise, les autres restent consultables à part. Importer un `.kzs` suppose que le serveur lise le format chiffré de la V1.

### 4.3 Socle commun de la suite

- Pourquoi : la comptabilité vient ensuite (décision 3) ; ce qui est commun aux deux produits doit être séparé avant de bâtir le serveur, pour ne pas le refaire.
- Recouvre : entreprise, comptes et droits, tiers (clients et fournisseurs), paramètres fiscaux, licence et modules, journal d'audit, partagés par les produits de la suite.
- Frontière entre postes et serveur (kadenz-34, revue avec la décision 28) : le serveur détient la base de référence et applique les règles métier en réutilisant le crate `kadenz-stockage` de la V1 (droits, transitions, validation, numérotation, audit). Chaque poste garde une copie locale chiffrée, au format de la V1, et une file des opérations faites hors connexion. Entre les deux, un protocole d'opérations idempotentes et de changements numérotés, spécifié dans `docs/spec-synchronisation.md`, avant tout écran multi-poste (`docs/spec-ecrans-multiposte.md`).

### 4.4 Sécurité et exploitation du serveur

- Pourquoi : le serveur garde les données de toute l'entreprise, et l'accès à distance (décision 16) fait passer ses échanges par un relais exploité par l'éditeur.
- Recouvre : authentification des postes et des comptes, chiffrement des échanges, séparation des données entre entreprises, sauvegardes côté serveur, journal des accès, mises à jour du serveur.
- Points techniques (kadenz-34) : avec la décision 16, un serveur sert une seule entreprise, donc une base par entreprise (sauvegarde et restauration par client, réutilisation du format `.kzs`) ; le relais ne garde aucune base ; la clé de chiffrement actuelle, protégée par un compte Windows, ne se transpose pas à un service ; côté serveur, les sauvegardes deviennent planifiées, avec rotation et restauration par l'administrateur, le `.kzs` restant le format d'échange.
- Mise en oeuvre des décisions 10 et 28 : serveur de confiance chez l'entreprise, base chiffrée au repos, relais aveugle grâce à une session TLS de bout en bout entre le poste distant et le serveur, à travers le relais. Le serveur aveugle (décision 13) est abandonné, et avec lui la clé de données enveloppée par compte et par poste, la clé de récupération sur papier et les opérations signées par compte. Détail : `docs/spec-synchronisation.md`, sections 7 et 8.
- **Numérotation** : la validation est appliquée sur le serveur par la fonction `valider` de la V1, qui prend le rang dans la même transaction ; une opération renvoyée après une coupure garde son identifiant et reçoit le même numéro, jamais un second. Les numéros restent continus et sans doublon (décision 9 ; `docs/spec-synchronisation.md`, section 5).
- **Clés** : base du serveur chiffrée par une clé protégée par Windows au niveau de la machine, pour le service ; copie locale de chaque poste chiffrée comme en V1 ; certificat propre au serveur, épinglé par les postes ; paire de clés par poste, enregistrée à l'inscription ; sauvegardes du serveur au format `.kzs`, chiffrées par mot de passe, restaurables sur une nouvelle machine. L'éditeur ne détient aucune clé des données. Perdre la machine serveur et toutes ses sauvegardes rend les données perdues : à dire au client.
- **Droits et preuve** : les droits sont imposés par le serveur, avec l'état présent de chaque compte ; une opération faite hors connexion et refusée pour un compte ou un droit est mise en attente (décision 18). Le journal d'audit de la V1, en ajout seul, tenu par le serveur, suffit à l'attribution des opérations.
- **Conséquences sur les autres chantiers** : l'envoi par email (3.4) et l'export SYSCOHADA (3.2) peuvent partir du serveur ou des postes, à choisir à leur spécification ; un espace client du site (3.7) ne peut pas lire les données, qui ne quittent jamais l'entreprise ; la reprise d'une base V1 (4.2) peut se faire sur le serveur, qui lit le `.kzs` avec son mot de passe.

### 4.5 Signature du code

- Pourquoi : déjà spécifiée en V1 (section 16 du cahier V1) mais en attente du certificat ; un service serveur non signé avec une exception de pare-feu est bloqué bien plus souvent qu'une application (kadenz-34), et les mises à jour des postes et du serveur en dépendent.
- Placée dans les fondations. Achat repoussé (décision 24) : le certificat doit être là avant la première mise en service du serveur chez un client.

## 5. Ordre (validé, décision 8)

0. **Pont V1** (4.0, décision 12) : avant toute release de la phase 2.
1. **Fondations** : socle commun (4.3), serveur chez le client et relais d'accès à distance (3.1), sécurité (4.4), reprise des bases V1 (4.2), abonnement (4.1), signature du code (4.5). Ces chantiers décident de l'architecture ; les faire après les fonctions obligerait à refaire celles-ci.
2. **Fonctions métier sur le nouveau socle** : stock (3.3), puis export SYSCOHADA (3.2) comme première brique de la comptabilité.
3. **Chantiers indépendants**, intercalables selon la demande des clients : email (3.4), signature électronique (3.6), multi-devise (3.5, à faire avant l'export SYSCOHADA s'il doit en tenir compte).
4. **Site web** (3.7) : site de présentation possible dès maintenant ; espace client et abonnement après 4.1 et 3.1.

## 6. Points techniques et dépendances

### 6.1 Dépendances entre chantiers

- 4.0 (pont V1) précède toute release de la phase 2.
- 3.1 (serveur) conditionne un espace client sur le site (3.7) et change la façon de faire la sauvegarde, la licence et les mises à jour.
- 4.3 (protocole de synchronisation et de numérotation) précède les écrans multi-poste et 4.2 (reprise d'une base V1 dans le serveur) ; 4.4 (certificat du serveur, inscription des postes, clé de la base du serveur) précède toute synchronisation réelle.
- 4.1 (abonnement) dépend du garde-fou d'horloge et du lien au serveur ou à l'entreprise (4.1, points techniques).
- 3.2 (export comptable) est plus simple si 3.5 (multi-devise) n'est pas encore là, et doit être revu quand 3.5 arrive.
- 3.3 (stock) s'appuie sur les réceptions et livraisons de la V1 ; un stock partagé entre postes dépend du serveur.
- 3.4 (email) et 3.6 (signature) s'appuient sur les PDF existants et sont indépendants des autres.

### 6.2 Points relevés dans le code (kadenz-34, 2026-11-12)

- **Numérotation** : le numéro d'un document est attribué à la validation, sans trou, par type, année et mois (`prochain_rang`, `crates/stockage/src/lib.rs`). Un poste coupé du serveur ne peut pas l'attribuer seul sans risque de doublon, et réserver des blocs de numéros par poste casse la continuité ; d'où la décision 9.
- **Horodatage** : `maintenant()` (`crates/stockage/src/lib.rs`) écrit l'heure locale sans fuseau. Avec plusieurs postes, dont des postes distants peut-être dans un autre fuseau, les horodatages techniques (audit, création, validation) doivent venir du serveur, avec le fuseau ; les dates métier (date de la facture) restent saisies.
- **Installateur du serveur** (réseau local sous Windows) : service Windows, règle de pare-feu, certificat TLS sur le réseau local (auto-signé, donc confiance ou épinglage par les postes), moyen pour les postes de trouver le serveur. Windows seulement (décision 11).
- **Désinstallation** : à l'inverse de la V1, désinstaller le serveur ne doit jamais effacer les données de l'entreprise ; sur un poste client en multi-poste, les données locales ne sont qu'une copie de travail, à condition que les opérations faites hors connexion aient été envoyées au serveur.

## 7. Contraintes déjà connues

- Ne rien casser pour les installations V1 : une base V1 doit pouvoir être reprise (migration du schéma, jamais de perte), et ses licences restent perpétuelles (décision 7).
- Les documents remis aux clients ne portent pas l'identité Kadenz.
- Les décisions de la V1 restent valables tant qu'elles ne sont pas revues explicitement (cahier V1, sections 18 et 19). La décision 4 revoit la licence perpétuelle de la section 15 du cahier V1 pour les nouvelles licences seulement.

## 8. Questions ouvertes pour l'utilisateur

### Réglées le 2026-11-12

- Décisions 1 à 37 (section 2) ; avertissements d'échéance et revendeurs réglés par les décisions 25 à 27. Les questions 1 à 6 et 13 de la version 0.5 sont réglées : conflits (décisions 17 et 18), données hébergées (13), fusion V1 (19), lecture seule (20), abonnement (21), clients visés (22 et 23), hébergement (16).

### Ouvertes pour les fondations

1. **VPS** : prestataire et pays, à préciser au moment de la souscription (le relais et le site en dépendent).
2. **Postes inclus** : combien de postes dans l'abonnement de base, et le prix des postes en plus ; rien n'est fixé ici sans vos chiffres.

### Ouvertes pour les chantiers métier

3. **SYSCOHADA** : quel format de sortie, pour quel logiciel comptable en attendant la comptabilité Kadenz, et quelle source fait foi pour le plan de comptes (comptable, texte de référence) ?
4. **Stock** : méthode de valorisation, un ou plusieurs dépôts, stock négatif permis ?
5. **Email** : messagerie de l'entreprise ou service tiers ; relances envoyées automatiquement ou à la main ?
6. **Multi-devise** : en plus de la devise de l'entreprise (décision 22), quelles devises étrangères, et comment le taux est-il fixé ?
7. **Signature électronique** : image ou cachet, certificat numérique, ou signature du client à distance ?
8. **Fiscalité par pays** : pour chaque pays visé (décision 22), quelle source fait foi pour la TVA, les mentions obligatoires des factures et la numérotation ?
9. **Site web** : objet (présentation de Kadenz, espace client, abonnement), langues, nom de domaine, contenus disponibles ?

## 9. Hors de ce document

- La distribution de la V1 (recherche d'antériorité de la marque) : question ouverte 8 du cahier V1, traitée à part ; la signature du code est reprise en 4.5.
- L'essai complet de la V1 par l'utilisateur.
