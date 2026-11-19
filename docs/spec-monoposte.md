# Édition monoposte de la phase 2

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-19 | Premier jet de kadenz-3a, après l'état des lieux et les ajouts de kadenz-cc (une seule instance, authentification gardée, clés au niveau du compte, arrêt propre). Rien n'est codé. |

Statut : **proposé**. Les points marqués « à trancher par l'utilisateur » lui sont posés groupés par kadenz-cc. Le reste se décide entre sessions (décision du 2026-11-16).

## 1. Cadre

- Décisions de l'utilisateur :
  - 2026-11-17 : chaque release de la phase 2 sort aussi en édition monoposte (journal du 2026-11-18 00:32:32) ;
  - 2026-11-18 : la release 1.0.1 attend cette édition (journal, 10:31:34) ;
  - 2026-11-18 : approche retenue, « serveur intégré au poste (127.0.0.1) » (journal, 11:06:01) ;
  - 2026-11-18 : bouton final de la présentation « Commencer l'installation » en monoposte, « Rejoindre l'entreprise » en multiposte (journal, 01:57:58).
- Ces décisions remplacent la décision 31 du cahier (« Toujours un serveur ») par la décision 76.
- Principe : le monoposte n'est pas un second mode de l'application. C'est le multiposte avec son serveur dans le même processus que la fenêtre, un seul poste inscrit, et rien d'ouvert au réseau. L'application garde un seul chemin : poste relié, copie du serveur, synchroniseur. Toutes les fonctions de la phase 2 restent donc disponibles sans code en double.

## 2. Serveur intégré

### 2.1 Processus et adresse

- `kadenz_serveur::Serveur::demarrer` est appelé par l'application au démarrage, dans son processus. Le serveur s'arrête avec la fenêtre. Pas de service Windows, pas de droits d'administrateur, rien ne tourne fenêtre fermée (accord de kadenz-cc).
- Adresse d'écoute `127.0.0.1` seulement, jamais `0.0.0.0`.
- Pas de découverte UDP (`decouverte: false`).
- Port : celui noté dans la base du serveur (`admin::port`). Au premier lancement, ou si ce port est pris par un autre programme, le serveur écoute sur un port choisi par le système (`127.0.0.1:0`), et l'application réécrit l'adresse du poste relié (`PosteRelie.adresse`) **avant** de lancer le synchroniseur. Les deux sont dans le même processus : aucun poste extérieur n'a besoin de connaître le nouveau port. Sans cette réécriture, le port noté à l'inscription ne servirait plus au lancement suivant.

### 2.2 Authentification gardée

- `127.0.0.1` est joignable par tout processus et par tout compte Windows de la machine. Rien n'est relâché parce que l'adresse est locale (kadenz-cc) : TLS avec empreinte épinglée, certificat du poste, sessions, comptes, droits et validation restent ceux du multiposte.

### 2.3 Dossier, clés et droits

- Dossier du serveur : `%LOCALAPPDATA%\Kadenz\Serveur`, dans le profil du compte Windows, et non `%ProgramData%\Kadenz\Serveur` (service du multiposte).
- Clés. Aujourd'hui, toutes les clés du serveur sont protégées par DPAPI à la portée **machine** : clé de la base (`kadenz_stockage::ouvrir_serveur`), clé TLS (`identite`), mot de passe des sauvegardes (`sauvegardes.rs`), restauration (`restauration.rs`). À cette portée, n'importe quel compte de la machine qui lit le fichier peut le déchiffrer ; la protection repose sur les droits du dossier. En monoposte, ces quatre chemins passent à la portée **compte** (`Portee::Compte`, comme la base V1 et le poste relié). Une portée est ajoutée à la configuration du serveur et à l'ouverture de sa base ; sa valeur par défaut reste `Machine`, et le multiposte ne change pas.
- Droits du dossier. `droits::restreindre` réserve aujourd'hui le dossier à SYSTEM et aux Administrateurs. Dans un profil, cela bloquerait le compte lui-même. Un mode « compte courant » est ajouté : propriétaire le compte courant, accès au compte courant et à SYSTEM, sans héritage. Il est posé à la création du dossier et vérifié à chaque démarrage, comme le mode actuel. Couper le contrôle (`controle_droits: false`) est écarté : ce réglage reste réservé aux essais.

### 2.4 Une seule instance

- L'application prend déjà un verrou par base (`instance::prendre`, mutex nommé d'après le chemin) : un second lancement ramène la fenêtre ouverte au premier plan.
- En monoposte, un verrou exclusif est aussi pris sur le dossier du serveur, par le serveur lui-même, sur un fichier `serveur.verrou` ouvert sans partage et gardé tant que le serveur tourne. Sans ce verrou, deux fenêtres pourraient lancer deux serveurs sur la même base (kadenz-cc). Un second démarrage sur ce dossier est refusé avec un message, si jamais le verrou de l'application ne l'a pas déjà arrêté.

### 2.5 Un autre compte Windows

- Chaque compte Windows a son propre dossier du serveur et ses propres clés, illisibles pour les autres comptes. Un autre compte qui lance Kadenz arrive donc sur sa propre installation vide : présentation, puis création d'une autre entreprise. C'est le comportement de la V1, où chaque compte a sa base.
- Licence : la nouvelle entreprise a son propre identifiant. Si elle active le même code que la première, le portail refuse avec « ce code d'activation appartient à une autre entreprise » (`MODELES_ACTIVATION`, `kadenz-licence`). Elle ne peut pas prendre la licence de la première.

### 2.6 Accès à distance et relais

- `Serveur::demarrer` lance toujours le fil `distance::veiller`. Ce fil reste au repos tant qu'aucun relais n'est réglé et actif : il relit le réglage, puis attend. Aucun réglage du relais n'est proposé en monoposte, donc le fil ne se connecte jamais.
- La rubrique « Accès à distance » et la recherche d'un serveur sur le réseau sont cachées en monoposte.

## 3. Premier lancement

Ordre, tout dans l'application :

1. Présentation, avec `monoposte` vrai : le bouton final est « Commencer l'installation ».
2. Le serveur intégré démarre sur un dossier neuf. L'identifiant de l'entreprise est écrit dès la création de sa base (`synchro`, migration), donc avant l'installation : le serveur peut écouter avant que l'entreprise existe.
3. Assistant de création : entreprise, premier compte Admin, code d'activation. Il reprend les étapes de l'assistant de `KadenzAdmin`, y compris l'appel au portail au « Suivant » de l'étape Entreprise (spécification des codes, L5). L'installation passe par `admin::installer` sur une connexion à la base du serveur : tout ou rien, et une licence refusée n'installe rien.
4. Inscription du poste, sans code saisi :
   - l'application crée un code d'inscription (`inscription::creer_code`, nom du poste = nom de la machine) ;
   - elle appelle `Client::inscrire` directement sur `127.0.0.1:<port>` avec l'empreinte lue dans le dossier du serveur. Elle ne passe pas par `admin::code_complet`, qui met le nom de la machine et l'adresse du réseau local dans le code ;
   - en cas d'échec, le code est annulé aussitôt : aucun code d'inscription ne reste ouvert.
5. Création de la copie du poste relié (`poste_relie::creer_copie`), puis écran de connexion avec le compte Admin créé à l'étape 3.

En cas d'échec entre les étapes 3 et 5, un relancement reprend là où l'installation s'est arrêtée : entreprise installée mais poste non inscrit, on reprend à l'étape 4. Il n'y a jamais de seconde entreprise.

### 3.1 Sans licence

- Décision 40 : un serveur sans licence est en lecture seule, comme à l'échéance. L'assistant permet d'installer sans code, et l'édition monoposte est alors en lecture seule jusqu'à l'activation d'un code depuis la rubrique « Abonnement ».
- Décision 41 : un seul poste peut s'inscrire sans licence ; c'est le cas du monoposte.
- **À trancher par l'utilisateur** : une période d'essai, ou un socle gratuit comme celui de la ligne V1 à l'échéance (décision 75 d), plutôt que la lecture seule.

## 4. Fermeture

La décision 47 coupe la sauvegarde à la fermeture sur un poste relié. Le monoposte n'en aurait donc aucune. En monoposte, la fermeture de la fenêtre suit cet ordre, avec un délai total borné (proposé : 30 secondes, avec un message d'attente) :

1. Les nouvelles écritures sont bloquées dans l'interface.
2. La file d'envoi du synchroniseur se vide vers le serveur, avec une attente bornée. La file est gardée dans la copie et renvoyée au démarrage suivant avec le même identifiant, sans double application (`kadenz-poste`). Un délai dépassé ne perd donc rien : l'écriture part au lancement suivant.
3. Le synchroniseur s'arrête.
4. Sauvegarde du serveur (`kadenz_serveur::sauvegardes::sauvegarder`), comme la sauvegarde de la V1 à la fermeture.
5. Arrêt du serveur (`Serveur::arreter`) : écoute, planification et relais s'arrêtent, puis le verrou du dossier est rendu.

Les sauvegardes quotidiennes planifiées du serveur ne tournent que fenêtre ouverte.

## 5. Ce que fait la fenêtre d'administration

`KadenzAdmin` n'est pas installé en monoposte. Ce qu'il fait en multiposte passe dans l'application :

| Fonction de `KadenzAdmin` | En monoposte |
|---|---|
| État du service | Sans objet : le serveur vit dans la fenêtre. |
| Empreinte du certificat | Sans objet : aucun autre poste ne s'inscrit. |
| Premier compte Admin et licence (assistant) | Assistant du premier lancement (section 3). |
| Codes d'inscription des postes | Sans objet : seule l'inscription automatique, une fois. |
| Réglage des sauvegardes (heure, dossier) | Rubrique « Sauvegardes » des Paramètres, montrée de nouveau en monoposte (exception à la décision 47), pour l'Admin. |
| Restauration d'une sauvegarde | Rubrique « Sauvegardes » et écran « Démarrage impossible ». Le serveur intégré s'arrête d'abord, la restauration se fait sur le dossier fermé, puis le serveur redémarre et la copie du poste est recréée. |
| Relais | Sans objet (section 2.6). |
| Mises à jour | Celles de l'application, comme aujourd'hui. |

## 6. Ce qui fixe l'édition

**À trancher par l'utilisateur.** Deux contraintes viennent de ses décisions : « une édition monoposte et une édition multiposte à chaque release », et le bouton final de la présentation, qui dépend de l'édition et suppose donc qu'elle soit connue dès le premier écran.

- **Recommandation** : un seul programme, et l'installateur de l'édition monoposte écrit une marque d'édition à côté de l'exécutable. Deux installateurs sont publiés à chaque release : `KadenzSetup` (multiposte : poste, et serveur à part) et une édition monoposte au nom à choisir. Cela répond aux deux contraintes avec un seul chemin de code, et l'édition est connue avant la présentation.
- Écarté : un choix au premier lancement. Il fixe bien le bouton, mais ne fait pas deux éditions au sens de la décision.
- Écarté : déduire l'édition du code d'activation. Le code n'arrive qu'à l'étape 3, après la présentation.

## 7. Licence et noms

**À trancher par l'utilisateur.**

- Licence de l'édition monoposte. Techniquement, un code de gamme `entreprise` à 1 poste suffit déjà : l'abonnement du serveur intégré porte `postes = 1`. Le portail n'a rien à changer. Restent à fixer le prix, la formule et les modules (grille de prix ouverte, décision 73).
- Noms commerciaux des deux éditions.
- Section 3.1 : essai ou socle gratuit, plutôt que la lecture seule sans licence.

## 8. Passage au multiposte (plus tard, non spécifié)

Le dossier du monoposte ne peut pas servir tel quel au service : un service sous un compte machine ne lit ni un profil ni des clés protégées au niveau du compte. Le passage demandera trois choses :

1. déplacer le dossier vers `%ProgramData%\Kadenz\Serveur` ;
2. reprotéger les clés à la portée machine ;
3. reposer les droits du multiposte.

Ensuite, le poste devient un poste relié ordinaire. Ce passage n'est pas dans la 1.0.1.

## 9. Essais

- Chaque essai donne son propre dossier du serveur, comme `dossier_essai` de `KadenzAdmin`. Jamais `%APPDATA%\Kadenz` ni `%LOCALAPPDATA%\Kadenz` (base réelle de l'utilisateur).
- Essais automatiques prévus :
  - démarrage sur `127.0.0.1` sans découverte ;
  - port repris, puis port pris et réécrit ;
  - clés à la portée compte ;
  - mode de droits « compte courant » ;
  - verrou du dossier (second démarrage refusé) ;
  - inscription automatique, avec le code annulé en cas d'échec ;
  - reprise d'une installation interrompue ;
  - fermeture avec une file non vide, puis lancement suivant sans perte ;
  - sauvegarde à la fermeture.
- L'écran est essayé par la session (pilotage de l'écran, décision du 2026-11-17).

## 10. Lots

Chaque lot est compilé et essayé dans son propre tour.

| Lot | Contenu | Crates |
|---|---|---|
| M1 | Serveur configurable : portée des clés, mode de droits « compte courant », verrou du dossier, port repris ou réécrit | `kadenz-stockage`, `kadenz-serveur` |
| M2 | Démarrage de l'application en monoposte, assistant, inscription automatique, reprise d'une installation interrompue | `kadenz-app` (`.po` communs) |
| M3 | Fermeture et sauvegarde | `kadenz-app`, `kadenz-poste` |
| M4 | Rubrique « Sauvegardes » et restauration en monoposte, rubriques cachées | `kadenz-app` (`.po` communs) |
| M5 | Marque d'édition et installateur monoposte, après la décision de la section 6 | `installateur/` |

Avant de toucher `crates/serveur` ou `crates/app`, demander à kadenz-cc qui y travaille. Avant d'ajouter des textes à l'interface, prévenir kadenz-74 (fichiers `.po` communs).
