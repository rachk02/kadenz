# Édition monoposte de la phase 2

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-19 | Premier jet de kadenz-3a, après l'état des lieux et les ajouts de kadenz-cc (une seule instance, authentification gardée, clés au niveau du compte, arrêt propre). Rien n'est codé. |
| 0.2 | 2026-11-19 | Relecture de kadenz-74 : passage des clients de la V1, assistant de l'application, voile de fermeture, sauvegardes sous Documents, causes du démarrage impossible, serveur démarré avant la copie, partage des lots. M1 livré (606aa2e) ; erreur de démarrage typée et vidage borné de la file. |
| 0.3 | 2026-11-19 | M2 livré par kadenz-74 (f8c8908). `Serveur::arreter` rend la main base fermée (remarque de kadenz-74). Fermeture : bornes fixées par kadenz-cc, sauvegarde d'arrêt faite par `Serveur::arreter` lui-même, fin de session Windows vérifiée dans les sources de winit par kadenz-74. |

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
- Le serveur démarre donc avant l'ouverture de la copie et avant `relie::demarrer`. Le démarrage (TLS, ouverture de la base) se fait hors du fil de l'interface, derrière un court écran d'attente (kadenz-74).
- Livré au lot M1 : `Config::monoposte(dossier)`, `dossier_monoposte()`, champs `portee` et `reprendre_port`, `admin::port_note`. `Serveur::adresse()` rend le port effectif.

### 2.2 Authentification gardée

- `127.0.0.1` est joignable par tout processus et par tout compte Windows de la machine. Rien n'est relâché parce que l'adresse est locale (kadenz-cc) : TLS avec empreinte épinglée, certificat du poste, sessions, comptes, droits et validation restent ceux du multiposte.

### 2.3 Dossier, clés et droits

- Dossier du serveur : `%LOCALAPPDATA%\Kadenz\Serveur`, dans le profil du compte Windows, et non `%ProgramData%\Kadenz\Serveur` (service du multiposte).
- Clés. Aujourd'hui, toutes les clés du serveur sont protégées par DPAPI à la portée **machine** : clé de la base (`kadenz_stockage::ouvrir_serveur`), clé TLS (`identite`), mot de passe des sauvegardes (`sauvegardes.rs`), restauration (`restauration.rs`). À cette portée, n'importe quel compte de la machine qui lit le fichier peut le déchiffrer ; la protection repose sur les droits du dossier. En monoposte, ces quatre chemins passent à la portée **compte** (`Portee::Compte`, comme la base V1 et le poste relié). Une portée est ajoutée à la configuration du serveur et à l'ouverture de sa base ; sa valeur par défaut reste `Machine`, et le multiposte ne change pas.
- Droits du dossier. `droits::restreindre` réserve aujourd'hui le dossier à SYSTEM et aux Administrateurs. Dans un profil, cela bloquerait le compte lui-même. Un mode « compte courant » est ajouté : propriétaire le compte courant, accès au compte courant et à SYSTEM, sans héritage. Il est posé à la création du dossier et vérifié à chaque démarrage, comme le mode actuel. Couper le contrôle (`controle_droits: false`) est écarté : ce réglage reste réservé aux essais.

### 2.4 Une seule instance

- L'application prend déjà un verrou par base (`instance::prendre`, mutex nommé d'après le chemin de la base V1) : un second lancement ramène la fenêtre ouverte au premier plan. Ce chemin est propre au compte Windows : en monoposte, ce verrou suffit pour l'application, sans nouveau verrou de son côté (kadenz-74).
- En monoposte, un verrou exclusif est aussi pris sur le dossier du serveur, par le serveur lui-même, sur un fichier `serveur.verrou` ouvert sans partage et gardé tant que le serveur tourne. Sans ce verrou, deux fenêtres pourraient lancer deux serveurs sur la même base (kadenz-cc). Un second démarrage sur ce dossier est refusé avec un message, si jamais le verrou de l'application ne l'a pas déjà arrêté.

### 2.5 Un autre compte Windows

- Chaque compte Windows a son propre dossier du serveur et ses propres clés, illisibles pour les autres comptes. Un autre compte qui lance Kadenz arrive donc sur sa propre installation vide : présentation, puis création d'une autre entreprise. C'est le comportement de la V1, où chaque compte a sa base.
- Licence : la nouvelle entreprise a son propre identifiant. Si elle active le même code que la première, le portail refuse avec « ce code d'activation appartient à une autre entreprise » (`MODELES_ACTIVATION`, `kadenz-licence`). Elle ne peut pas prendre la licence de la première.

### 2.6 Accès à distance et relais

- `Serveur::demarrer` lance toujours le fil `distance::veiller`. Ce fil reste au repos tant qu'aucun relais n'est réglé et actif : il relit le réglage, puis attend. Aucun réglage du relais n'est proposé en monoposte, donc le fil ne se connecte jamais.
- La rubrique « Accès à distance » et la recherche d'un serveur sur le réseau sont cachées en monoposte.

### 2.7 Démarrage impossible

`Serveur::demarrer` rend une erreur typée, `ErreurDemarrage`, dont la forme a été convenue avec kadenz-74. Chaque cause mène à l'écran « Démarrage impossible » de l'application, avec son message traduit. L'application construit le texte ; `Display` sert à `erreurs.log`.

| Cause | Exemple |
|---|---|
| `Verrouille` | Un autre serveur tient le verrou du dossier. |
| `Droits(texte)` | Droits du dossier impossibles à poser, ou différents de ceux voulus. |
| `AutreEdition` | Dossier d'un service ouvert par le serveur intégré, ou l'inverse. |
| `Base(kadenz_stockage::Erreur)` | Clé introuvable, clé d'un autre compte Windows, clé endommagée, base indéchiffrable, base d'une version plus récente : messages existants de `sauvegardes::message_demarrage`. |
| `Identite(texte)` | Certificat ou clé TLS du serveur illisible. |
| `Ecoute(texte)` | Écoute impossible, même sur un port libre. |
| `Autre(texte)` | Dossier non créé, port non noté. |

Boutons de l'écran : « Restaurer » arrête le serveur, restaure, puis redémarre ; « Ouvrir le dossier des données » ouvre le dossier du serveur. Les textes vont dans les `.po`.

## 3. Premier lancement

Ordre, tout dans l'application :

1. Présentation, avec `monoposte` vrai : le bouton final est « Commencer l'installation ».
2. Le serveur intégré démarre sur un dossier neuf. L'identifiant de l'entreprise est écrit dès la création de sa base (`synchro`, migration), donc avant l'installation : le serveur peut écouter avant que l'entreprise existe.
3. Assistant de création : c'est l'assistant de l'application, déjà connu des clients de la V1 (Licence, Entreprise, Administrateur, Taux et paiements, Récapitulatif), avec l'activation par code au « Suivant » de l'étape Entreprise (L5 bis). Celui de `KadenzAdmin` n'a pas l'étape Taux et paiements (kadenz-74). L'installation passe par `admin::installer` sur une connexion à la base du serveur (`admin::ouvrir_base`), et non plus sur la base V1 : tout ou rien, et une licence refusée n'installe rien.
4. Inscription du poste, sans code saisi :
   - l'application crée un code d'inscription (`inscription::creer_code`, nom du poste = nom de la machine) ;
   - elle appelle `Client::inscrire` directement sur `127.0.0.1:<port>` avec l'empreinte lue dans le dossier du serveur. Elle ne passe pas par `admin::code_complet`, qui met le nom de la machine et l'adresse du réseau local dans le code ;
   - en cas d'échec, le code est annulé aussitôt : aucun code d'inscription ne reste ouvert.
5. Création de la copie du poste relié (`poste_relie::creer_copie`), puis écran de connexion avec le compte Admin créé à l'étape 3.

En cas d'échec entre les étapes 3 et 5, un relancement reprend là où l'installation s'est arrêtée : entreprise installée mais poste non inscrit, on reprend à l'étape 4. Il n'y a jamais de seconde entreprise.

### 3.0 Clients de la V1

Un client de la ligne 0.2.x qui installe l'édition monoposte a déjà sa base V1 (`%APPDATA%\Kadenz\kadenz.sqlite`) ; aujourd'hui, l'application l'ouvre quand il n'y a pas de copie de poste relié (kadenz-74).

- Proposé : au premier lancement en monoposte, si une base V1 existe pour ce compte Windows, l'application propose de la reprendre juste après l'inscription automatique. La reprise existe déjà (`reprise.rs`, décisions 19 et 53 à 55) ; elle demande un serveur neuf, au sens de `stockage::reprise::serveur_neuf`, qui ne compte que les tiers, produits, documents et règlements. Elle reste donc possible après l'assistant. Sans base V1, l'installation reste vide.
- La base V1 reste intacte (décision 45).
- Le texte de la présentation (« Commencer l'installation ») s'adresse à un client neuf : à revoir pour un client qui reprend sa base.
- **À trancher par l'utilisateur** : reprendre aussi l'entreprise et le compte Admin depuis la base V1, sans repasser par l'assistant.

### 3.1 Sans licence

- Décision 40 : un serveur sans licence est en lecture seule, comme à l'échéance. L'assistant permet d'installer sans code, et l'édition monoposte est alors en lecture seule jusqu'à l'activation d'un code depuis la rubrique « Abonnement ».
- Décision 41 : un seul poste peut s'inscrire sans licence ; c'est le cas du monoposte.
- **À trancher par l'utilisateur** : une période d'essai, ou un socle gratuit comme celui de la ligne V1 à l'échéance (décision 75 d), plutôt que la lecture seule. La lecture seule d'un poste relié sans abonnement est déjà gérée (`relie::appliquer_modules`, `abonnement.rs`). Un socle gratuit reprendrait tel quel celui de la 0.2.3 : tiers, produits, factures, avoirs, administration.

## 4. Fermeture

La décision 47 coupe la sauvegarde à la fermeture sur un poste relié. Le monoposte n'en aurait donc aucune. En monoposte, la fermeture de la fenêtre suit cet ordre, avec un délai total de 30 secondes, sauvegarde d'arrêt comprise (kadenz-cc) : à 10 secondes, le voile dit que l'opération prend plus de temps que prévu ; à 30 secondes, la fenêtre se ferme même si le fil n'a pas fini.

Aujourd'hui, le travail de fermeture se fait après la fin de `fenetre.run()`, donc fenêtre disparue : aucun message ne peut s'afficher, et un second lancement pendant ce temps ramènerait au premier plan une fenêtre qui n'existe plus (kadenz-74). La demande de fermeture est donc interceptée : la fenêtre reste affichée sous un voile « Enregistrement et sauvegarde en cours… », qui bloque aussi les écritures (étape 1). Les étapes 2 à 4 se font dans un fil à part, puis la fenêtre se ferme. `Serveur` et `Synchroniseur` sont `Send`, vérifié à la compilation.

1. Les nouvelles écritures sont bloquées dans l'interface.
2. La file d'envoi du synchroniseur se vide vers le serveur, avec une attente bornée (`Synchroniseur::vider(délai)`, vrai si la file est vide). La file est gardée dans la copie et renvoyée au démarrage suivant avec le même identifiant, sans double application (`kadenz-poste`). Un délai dépassé ne perd donc rien : l'écriture part au lancement suivant.
3. Le synchroniseur s'arrête.
4. Arrêt du serveur (`Serveur::arreter`) : écoute, planification et relais s'arrêtent ; puis la sauvegarde d'arrêt (décision 35), faite par `arreter` lui-même quand un mot de passe des sauvegardes est choisi, comme la sauvegarde de la V1 à la fermeture : l'application n'en lance pas une seconde ; puis la connexion à la base se ferme et le fil de la base est joint ; enfin le verrou du dossier est rendu. Au retour, le fichier de la base n'est plus ouvert.

Les sauvegardes quotidiennes planifiées du serveur ne tournent que fenêtre ouverte.

Fermeture de la session Windows ou arrêt de la machine : winit (0.30.13, `platform_impl/windows/event_loop.rs`) ne traduit que `WM_CLOSE` en demande de fermeture ; `WM_QUERYENDSESSION` et `WM_ENDSESSION` passent à `DefWindowProc`, et Windows termine le processus après `WM_ENDSESSION` (vérifié dans les sources par kadenz-74). Il n'y a donc ni voile, ni vidage, ni sauvegarde d'arrêt. Les données restent intactes : la base du serveur est en mode WAL, et la file du synchroniseur, gardée dans la copie, part au lancement suivant. C'est accepté.

### 4.1 Dossier des sauvegardes

- En monoposte, le dossier par défaut est `Documents\Kadenz\Sauvegardes\Serveur` (`sauvegardes::dossier_par_defaut_de`) : sous le dossier que les clients de la V1 connaissent, et hors du dossier des données. Il est à part de `Documents\Kadenz\Sauvegardes` lui-même, parce que les sauvegardes de la V1 portent les mêmes noms (`kadenz-<genre>-AAAAMMJJ-HHMMSS.kzs`) : dans le même dossier, la rotation des sauvegardes automatiques du serveur supprimerait celles de la V1.
- Le mot de passe des sauvegardes est protégé au niveau du compte (lot M1). Sans mot de passe choisi, `sauvegardes::sauvegarder` refuse : l'assistant ou la rubrique « Sauvegardes » doit en faire choisir un.
- Documents synchronisé par OneDrive : c'est un avantage, puisque les sauvegardes, chiffrées, ont alors une copie hors du poste. Une sauvegarde s'écrit dans le sous-dossier `en-cours`, puis est renommée dans le dossier : le fichier n'apparaît complet qu'au renommage. Un reste partiel peut être téléversé depuis `en-cours`, sans gravité (kadenz-74).

## 5. Ce que fait la fenêtre d'administration

`KadenzAdmin` n'est pas installé en monoposte. Ce qu'il fait en multiposte passe dans l'application :

| Fonction de `KadenzAdmin` | En monoposte |
|---|---|
| État du service | Sans objet : le serveur vit dans la fenêtre. |
| Empreinte du certificat | Sans objet : aucun autre poste ne s'inscrit. |
| Premier compte Admin et licence (assistant) | Assistant du premier lancement (section 3). |
| Codes d'inscription des postes | Sans objet : seule l'inscription automatique, une fois. |
| Réglage des sauvegardes (heure, dossier) | Rubrique « Sauvegardes » des Paramètres, montrée de nouveau en monoposte (exception à la décision 47), pour l'Admin. |
| Restauration d'une sauvegarde | Rubrique « Sauvegardes » et écran « Démarrage impossible ». Le serveur intégré s'arrête d'abord, la restauration se fait sur le dossier fermé, puis le serveur redémarre et la copie du poste est recréée. `Serveur::arreter` rend la main une fois la connexion à la base fermée (remarque de kadenz-74) : le fichier peut être remplacé aussitôt. |
| Relais | Sans objet (section 2.6). |
| Rubrique « Serveur » d'un poste relié, recherche de serveur | Cachées, sauf l'accès à la reprise V1 (section 3.0). Liste exacte des rubriques au lot M4 (kadenz-74). |
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
| M1 | Serveur configurable : portée des clés, mode de droits « compte courant », verrou du dossier, port repris ou réécrit. Livré (606aa2e). | `kadenz-stockage`, `kadenz-serveur` |
| M2 | Démarrage de l'application en monoposte, assistant, inscription automatique, reprise d'une installation interrompue | `kadenz-app` (`.po` communs) |
| M3 | Fermeture et sauvegarde | `kadenz-app`, `kadenz-poste` |
| M4 | Rubrique « Sauvegardes » et restauration en monoposte, rubriques cachées | `kadenz-app` (`.po` communs) |
| M5 | Marque d'édition et installateur monoposte, après la décision de la section 6 | `installateur/` |

Partage arrêté entre kadenz-74 et kadenz-3a (2026-11-19), sans fichier touché par les deux :

- kadenz-3a :
  - M1 ;
  - la part de M3 hors de l'application : `ErreurDemarrage`, `Synchroniseur::vider`, assertions `Send`, dossier des sauvegardes selon la portée ;
  - M5, avec la lecture de la marque d'édition dans le crate qui la porte.
- kadenz-74 : M2, la part de M3 dans l'application (voile, ordre des étapes), M4, et tous les `.po`. Ces trois lots passent l'un après l'autre.
- kadenz-3a ne touche ni `crates/app` ni les `.po`. kadenz-74 ne touche ni `crates/serveur`, ni `crates/poste`, ni `crates/stockage`, ni `installateur/`. Les points de contact se fixent par message avant le code.
