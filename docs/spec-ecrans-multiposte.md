# Kadenz - Écrans multi-postes (premier jet)

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-12 | Premier jet (kadenz-8e) : écrans du poste pour le serveur de l'entreprise, l'accès à distance, l'état de la connexion, les brouillons hors connexion, les conflits, les opérations en attente, l'abonnement et la reprise d'une base V1, tirés des décisions 5 à 29 du cahier de la phase 2. |
| 0.2 | 2026-11-12 | Décisions 30 à 34 appliquées : fenêtre d'administration du serveur (nouvelle section 1), pas de poste seul, inscription par un code qui porte l'empreinte du certificat (relecture de kadenz-34), toutes les opérations refusées en attente, conflits tranchés par l'Admin ou un ayant droit, version entière ou champ par champ. Questions de la section 10 réglées. |

Statut : **à l'étude**. Ce document décrit ce que voit et fait la personne sur un poste ; le protocole, le stockage et les clés sont dans `docs/spec-synchronisation.md` (kadenz-34). Il ne répète pas les décisions, il y renvoie (cahier de la phase 2, section 2, « décision n »). Rien n'est codé. Les points qui demandent un choix de l'utilisateur sont en section 10.

Règles communes, reprises de la V1 : tous les textes en sept langues (français, anglais, espagnol, turc, arabe de droite à gauche, japonais, chinois) ; mêmes composants et mêmes couleurs que l'application actuelle ; angles de 4 px au plus ; aucun liseré de couleur sur un seul côté d'un bloc ; les erreurs s'affichent dans la zone concernée, les succès dans le bandeau de retour.

## 1. Serveur et fenêtre d'administration (décisions 30 et 31)

- Tout poste de la phase 2 est relié à un serveur ; il n'y a pas de mode « poste seul ». Pour une entreprise d'un seul poste, le serveur s'installe sur la même machine. Un poste seul reste possible avec la V1.
- Le serveur s'installe comme service Windows (décision 11) et a une petite fenêtre d'administration sur sa machine, limitée au dépannage :
  - état du service (démarré, arrêté, dernière erreur) ;
  - empreinte du certificat du serveur ;
  - création du premier compte Admin de l'entreprise ;
  - création des codes d'inscription des postes ;
  - restauration d'une sauvegarde (base abîmée ou nouvelle machine).
- Tout le reste se fait depuis un poste connecté avec un compte Admin : comptes, accès à distance (section 9), abonnement (section 7), sauvegardes planifiées.
- Au premier lancement d'un poste de la phase 2, l'assistant demande le code d'inscription (section 2) au lieu de créer une base locale.

## 2. Connexion au serveur (Paramètres, rubrique « Serveur »)

- **Inscription** : un poste rejoint l'entreprise avec un code d'inscription à usage unique, créé dans la fenêtre du serveur ou par un Admin depuis un poste. Le code porte l'empreinte du certificat du serveur : le poste vérifie seul qu'il parle au bon serveur, sans comparaison à l'oeil.
- **Trouver le serveur** : recherche automatique sur le réseau local, avec la liste des serveurs Kadenz trouvés (nom de la machine, nom de l'entreprise), ou saisie de l'adresse à la main. La recherche sert seulement à trouver l'adresse ; elle ne donne aucune confiance.
- **Recours** : si le code est saisi sans l'empreinte, le poste affiche l'empreinte du certificat reçu, que l'Admin compare à celle de la fenêtre du serveur avant de confirmer. Ensuite, un certificat différent bloque la connexion avec un message clair, jamais une acceptation silencieuse.
- **Nombre de postes** : limité par l'abonnement (décision 21). Si la limite est atteinte, le message le dit et renvoie vers l'abonnement (section 7).
- **Comptes** : la connexion se fait avec les comptes communs de l'entreprise, gérés sur le serveur ; la rubrique « Utilisateurs » agit sur ces comptes.

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

- **Qui** (décision 33) : l'Admin, ou la personne qui synchronise si elle a le droit de modifier la fiche concernée.
- **Liste** : fiche concernée (client, fournisseur, produit), les deux postes et comptes, les deux dates.
- **Comparaison** : les deux versions côte à côte, les champs qui diffèrent mis en évidence (texte et fond, pas un liseré latéral) ; boutons « Garder cette version » sous chacune, et « Choisir champ par champ », qui met un choix sur chaque champ différent (décision 34).
- Tant qu'un conflit n'est pas tranché, la fiche reste utilisable dans sa version du serveur, avec un signe « Conflit à trancher ».
- Chaque choix est inscrit au journal d'audit (qui, quand, version gardée).

## 6. Opérations en attente (décisions 18 et 32)

- Toute opération faite hors connexion et refusée par le serveur pour un compte ou un droit (compte désactivé, droit retiré) arrive en attente, sans être appliquée, qu'elle soit antérieure ou postérieure au changement.
- **Admin seulement** : liste par compte, avec l'heure du poste (indicative seulement, l'horloge d'un poste pouvant être fausse), le poste, le type d'opération et le document ou la fiche concernés ; détail de chaque opération.
- Boutons « Accepter » et « Rejeter », à l'unité ou pour une sélection ; chaque décision va au journal d'audit.
- Le compte concerné voit, à sa prochaine connexion s'il est encore actif, que ses opérations ont été acceptées ou rejetées.

## 7. Abonnement (Paramètres, rubrique « Abonnement », remplace « Licence » pour un poste relié)

- **État** : entreprise, formule (mensuelle ou annuelle), échéance, postes inclus et postes utilisés, émetteur de la licence (éditeur ou revendeur, décision 29).
- **Renouvellement** : saisie d'une nouvelle licence, ou récupération automatique depuis le portail quand le serveur est relié à Internet (détail dans la spécification de 4.1).
- **Avertissements** (décisions 25 et 26) : bandeau à 30 jours, à 7 jours et la veille de l'échéance ; l'Admin seul au début, tous les comptes dans les derniers jours. Le bandeau dit la date, la conséquence (« Kadenz passera en lecture seule ») et, pour l'Admin, le chemin pour renouveler.
- **Lecture seule** (décisions 6, 20 et 21, sans délai de grâce) : bandeau permanent « Abonnement échu : lecture seule ». Consultation, recherche, impression et exports PDF et Excel restent possibles ; boutons de création, de modification et de validation inactifs, avec la raison en info-bulle. La rubrique « Abonnement » reste accessible à l'Admin pour renouveler.
- Les postes sous licence V1 perpétuelle (décision 7) gardent la rubrique « Licence » actuelle.

## 8. Reprise d'une base V1 (décision 19)

- Assistant réservé à l'Admin, depuis la rubrique « Serveur » d'un poste relié, juste après l'inscription d'un serveur neuf : choisir la base V1 (le poste actuel, ou une sauvegarde `.kzs` avec son mot de passe), voir un résumé (entreprise, nombre de clients, de produits, de documents, dernier numéro par type), confirmer.
- **Une seule base** par serveur : si le serveur a déjà des données, l'assistant refuse et l'explique ; les autres installations V1 restent consultables à part.
- La numérotation continue après les numéros repris ; la base V1 d'origine n'est jamais effacée.

## 9. Accès à distance (Paramètres, rubrique « Accès à distance », Admin)

- **Activer ou désactiver** l'accès à distance pour l'entreprise (décision 16) ; désactivé par défaut.
- **État du relais** : joignable ou non depuis le serveur, dernière connexion distante.
- **Postes autorisés à distance** : liste, avec ajout par code à usage unique et retrait immédiat (un poste retiré ne peut plus passer par le relais).
- Texte d'explication court : les échanges passent par le relais de l'éditeur, chiffrés de bout en bout, sans que l'éditeur puisse les lire (décisions 10 et 28) ; si le serveur de l'entreprise est éteint, un poste distant travaille hors connexion.

## 10. Questions

Réglées le 2026-11-12 : administration du serveur (décision 30), poste seul (31), opérations en attente (32), qui tranche un conflit (33), granularité (34). Aucune question ouverte propre aux écrans ; celles du protocole sont dans `docs/spec-synchronisation.md`, section 9.
