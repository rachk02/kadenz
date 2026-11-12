# Kadenz - Écrans multi-postes (premier jet)

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-12 | Premier jet (kadenz-8e) : écrans du poste pour le serveur de l'entreprise, l'accès à distance, l'état de la connexion, les brouillons hors connexion, les conflits, les opérations en attente, l'abonnement et la reprise d'une base V1, tirés des décisions 5 à 29 du cahier de la phase 2. |

Statut : **à l'étude**. Ce document décrit ce que voit et fait la personne sur un poste ; le protocole, le stockage et les clés sont dans `docs/spec-synchronisation.md` (kadenz-34). Il ne répète pas les décisions, il y renvoie (cahier de la phase 2, section 2, « décision n »). Rien n'est codé. Les points qui demandent un choix de l'utilisateur sont en section 10.

Règles communes, reprises de la V1 : tous les textes en sept langues (français, anglais, espagnol, turc, arabe de droite à gauche, japonais, chinois) ; mêmes composants et mêmes couleurs que l'application actuelle ; angles de 4 px au plus ; aucun liseré de couleur sur un seul côté d'un bloc ; les erreurs s'affichent dans la zone concernée, les succès dans le bandeau de retour.

## 1. Mode du poste

Au premier lancement d'une version de la phase 2, l'assistant demande comment ce poste travaille :

- **Poste seul** : comme la V1, base locale, aucune connexion. Reste possible pour une entreprise d'un seul poste.
- **Poste relié au serveur de l'entreprise** : le poste se connecte au serveur installé sur une machine de l'entreprise (décision 16).

Le serveur lui-même s'installe à part, comme service Windows (décision 11) ; son administration est traitée en section 10, question 1.

Un poste seul peut passer plus tard en poste relié (Paramètres, rubrique « Serveur ») ; s'il a des données, c'est la reprise de la section 8.

## 2. Connexion au serveur (Paramètres, rubrique « Serveur »)

- **Trouver le serveur** : recherche automatique sur le réseau local, avec la liste des serveurs Kadenz trouvés (nom de la machine, nom de l'entreprise) ; ou saisie de l'adresse à la main.
- **Confiance** : à la première connexion, le poste affiche l'empreinte du certificat du serveur ; l'Admin la compare à celle affichée sur le serveur et confirme. Ensuite, un certificat différent bloque la connexion avec un message clair, jamais une acceptation silencieuse.
- **Rattachement** : un poste rejoint l'entreprise avec un code à usage unique créé par l'Admin sur le serveur ; le nombre de postes est limité par l'abonnement (décision 21). Si la limite est atteinte, le message le dit et renvoie vers l'abonnement (section 7).
- **Comptes** : une fois relié, la connexion se fait avec les comptes communs de l'entreprise, gérés sur le serveur ; la rubrique « Utilisateurs » agit sur ces comptes.

## 3. État de la connexion

Une indication permanente et discrète dans la barre de l'application, avec texte et icône (jamais la couleur seule) :

| État | Texte |
|---|---|
| Relié en réseau local | « Connecté au serveur » |
| Relié par l'accès à distance | « Connecté à distance » |
| Synchronisation en cours | « Synchronisation... » avec le nombre d'opérations restantes |
| Hors connexion | « Hors connexion : n opérations à envoyer » |
| Serveur refusé | « Serveur non reconnu » (certificat changé), avec un lien vers la rubrique « Serveur » |

Un clic ouvre le détail : dernière synchronisation réussie, opérations en file, conflits à trancher (section 5) et opérations en attente (section 6), s'il y en a.

## 4. Travail hors connexion

- Tout reste consultable et modifiable dans la copie locale ; les créations et modifications vont dans la file d'envoi (décision 5).
- **Validation** : hors connexion, le bouton « Valider » d'un document reste visible mais inactif, avec le texte « La validation attend le serveur : le numéro est attribué par le serveur » (décision 9). Le document reste en brouillon.
- **Documents à valider au retour** : une liste, ouverte depuis l'état de la connexion, des brouillons dont la validation a été demandée hors connexion ; au retour du serveur, l'application propose de les valider, document par document ou tous ensemble, chacun recevant alors son numéro.
- Les PDF d'un brouillon portent la mention « Brouillon » comme en V1 ; un document sans numéro définitif n'est jamais présenté comme remis au client.

## 5. Conflits à trancher (décision 17)

- **Qui** : l'Admin, ou la personne qui synchronise si elle a le droit de modifier la fiche concernée (à confirmer, section 10, question 2).
- **Liste** : fiche concernée (client, fournisseur, produit), les deux postes et comptes, les deux dates.
- **Comparaison** : les deux versions côte à côte, les champs qui diffèrent mis en évidence (texte et fond, pas un liseré latéral) ; boutons « Garder cette version » sous chacune ; option « Choisir champ par champ » (à confirmer, section 10, question 3).
- Tant qu'un conflit n'est pas tranché, la fiche reste utilisable dans sa version du serveur, avec un signe « Conflit à trancher ».
- Chaque choix est inscrit au journal d'audit (qui, quand, version gardée).

## 6. Opérations en attente (décision 18)

- Quand un compte a été désactivé ou ses droits réduits pendant une coupure, ses opérations hors connexion postérieures au changement arrivent au serveur sans être appliquées.
- **Admin seulement** : liste par compte, avec la date, le poste, le type d'opération et le document ou la fiche concernés ; détail de chaque opération.
- Boutons « Accepter » et « Rejeter », à l'unité ou pour une sélection ; chaque décision va au journal d'audit.
- Le compte concerné voit, à sa prochaine connexion s'il est encore actif, que ses opérations ont été acceptées ou rejetées.

## 7. Abonnement (Paramètres, rubrique « Abonnement », remplace « Licence » pour un poste relié)

- **État** : entreprise, formule (mensuelle ou annuelle), échéance, postes inclus et postes utilisés, émetteur de la licence (éditeur ou revendeur, décision 29).
- **Renouvellement** : saisie d'une nouvelle licence, ou récupération automatique depuis le portail quand le serveur est relié à Internet (détail dans la spécification de 4.1).
- **Avertissements** (décisions 25 et 26) : bandeau à 30 jours, à 7 jours et la veille de l'échéance ; l'Admin seul au début, tous les comptes dans les derniers jours. Le bandeau dit la date, la conséquence (« Kadenz passera en lecture seule ») et, pour l'Admin, le chemin pour renouveler.
- **Lecture seule** (décisions 6, 20 et 21, sans délai de grâce) : bandeau permanent « Abonnement échu : lecture seule ». Consultation, recherche, impression et exports PDF et Excel restent possibles ; boutons de création, de modification et de validation inactifs, avec la raison en info-bulle. La rubrique « Abonnement » reste accessible à l'Admin pour renouveler.
- Les postes sous licence V1 perpétuelle (décision 7) gardent la rubrique « Licence » actuelle.

## 8. Reprise d'une base V1 (décision 19)

- Assistant réservé à l'Admin, depuis la rubrique « Serveur » : choisir la base V1 (le poste actuel, ou une sauvegarde `.kzs` avec son mot de passe), voir un résumé (entreprise, nombre de clients, de produits, de documents, dernier numéro par type), confirmer.
- **Une seule base** par serveur : si le serveur a déjà des données, l'assistant refuse et l'explique ; les autres installations V1 restent consultables à part.
- La numérotation continue après les numéros repris ; la base V1 d'origine n'est jamais effacée.

## 9. Accès à distance (Paramètres, rubrique « Accès à distance », Admin)

- **Activer ou désactiver** l'accès à distance pour l'entreprise (décision 16) ; désactivé par défaut.
- **État du relais** : joignable ou non depuis le serveur, dernière connexion distante.
- **Postes autorisés à distance** : liste, avec ajout par code à usage unique et retrait immédiat (un poste retiré ne peut plus passer par le relais).
- Texte d'explication court : les échanges passent par le relais de l'éditeur, chiffrés de bout en bout, sans que l'éditeur puisse les lire (décisions 10 et 28) ; si le serveur de l'entreprise est éteint, un poste distant travaille hors connexion.

## 10. Questions ouvertes

1. **Administration du serveur** : le service serveur a-t-il sa propre petite fenêtre d'administration sur sa machine (empreinte du certificat, codes de rattachement, sauvegardes), ou tout se fait-il depuis un poste connecté avec un compte Admin ?
2. **Qui tranche un conflit** : l'Admin seulement, ou aussi la personne qui synchronise si elle a le droit de modifier la fiche ?
3. **Granularité** : choix d'une version entière seulement, ou aussi champ par champ ?
4. **Poste seul** : faut-il garder ce mode dans la phase 2, ou tout poste de la phase 2 est-il relié à un serveur (le serveur pouvant tourner sur la même machine pour une entreprise d'un seul poste) ?
