# Kadenz - Appareils d'une licence et révocation à distance (spécification)

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-27 | Premier jet par kadenz-cc, d'après la demande de l'utilisateur du 2026-11-27 (décision 80). Rien n'est encore codé. |
| 0.2 | 2026-11-27 | Réponses de l'utilisateur : 30 jours sans contact puis lecture seule ; appareil déconnecté bloqué jusqu'à ce qu'on le réautorise, reconnu par l'identifiant de l'ordinateur. Révocation gardée malgré une licence réinstallée. Changement d'ordinateur en édition un poste. |
| 0.3 | 2026-11-27 | Lots R1 et R2 livrés, et les actions du portail du lot R3 (déconnecter, réautoriser, révoquer, rétablir ; pages encore à faire) : bulletin `kadenz_licence::bulletin`, schéma 14 du portail (`appareil`, `ordinateur_bloque`, `revocation`), route `POST /api/licence/etat`, frein sur les échecs, activation refusée tant que la licence est révoquée (`MODELES_ACTIVATION[7]`, traduit). Journal limité aux changements. |

Statut : **spécifié**. Les lots sont à la section 8.

## 1. Décision de l'utilisateur (2026-11-27)

- **80. Révocation à distance, comme Kaspersky.** Depuis le portail, on voit chaque appareil relié à une licence, on peut déconnecter les appareils un par un, et révoquer la licence entière.

Réponses de l'utilisateur (2026-11-27, questions de kadenz-cc) :

- **30 jours sans contact, puis lecture seule.** Une révocation prend effet au prochain contact d'un poste avec le portail. Un serveur muni d'une licence qui n'a reçu aucun bulletin depuis 30 jours passe en lecture seule, avec la raison « licence non confirmée depuis 30 jours : reliez un poste à Internet » ; le premier bulletin reçu la lève. Le compte des 30 jours part du dernier bulletin appliqué, ou, avant le premier, de la mise à jour du serveur vers la version qui sait les appliquer : un serveur mis à jour dont les postes sont encore anciens ne se bloque pas d'emblée. Sans licence (essai gratuit), rien n'est exigé (décision 78).
- **Un appareil déconnecté reste bloqué** jusqu'à ce qu'on le réautorise au portail. Il est reconnu par l'identifiant de l'ordinateur (`identifiant_poste`, empreinte du `MachineGuid`, comme l'essai) : le serveur refuse de l'inscrire de nouveau.

Choix de kadenz-cc (délégation du 2026-11-25), que l'utilisateur peut changer :

- **Une licence révoquée passe en lecture seule**, comme à l'échéance : le client garde l'accès à ses données, peut consulter, imprimer et exporter, mais ne saisit plus rien. Rien n'est effacé.
- **Un appareil déconnecté** est retiré du serveur de l'entreprise : il ne peut plus s'y connecter et libère sa place.
- **Une révocation survit à la réinstallation d'une licence** : coller de nouveau la même licence, ou une plus ancienne, ne la lève pas. Seul un bulletin plus récent qui ne la porte plus la lève.

## 2. Ce qui existe aujourd'hui

- Licence d'abonnement `KADENZ-E-...`, signée par le portail, par entreprise (identifiant tiré au hasard à la création du dossier), avec un nombre de postes. Elle est vérifiée hors ligne par le serveur de l'entreprise ; le portail ne revoit jamais l'entreprise après l'activation.
- Le serveur garde ses postes inscrits (`poste_inscrit` : empreinte du certificat, nom, date d'inscription, date de retrait) et sait en retirer un (`kadenz_stockage::postes::retirer`). L'empreinte du certificat change à chaque inscription : elle ne reconnaît pas un ordinateur.
- « Libérer le poste » du portail (`liberer_poste`) ne sert qu'aux codes de la ligne V1. En édition un poste de la phase 2, un nouvel ordinateur crée une nouvelle entreprise, à qui le portail refuse le code déjà activé : il n'y a pas encore de changement d'ordinateur.
- Le service Windows du serveur, qui tourne sous SYSTEM, n'ouvre aucune connexion vers Internet (`docs/spec-codes-licence.md`, section 5). Cette règle est gardée : ce sont les postes qui parlent au portail.
- En monoposte, le serveur tourne dans le processus de l'application, avec un seul poste.

## 3. Principe

Un poste relaie, dans les deux sens, entre le serveur de son entreprise et le portail :

1. Le poste demande au serveur l'état de sa licence : identifiant de l'entreprise, édition, liste des postes inscrits (empreinte du certificat, identifiant de l'ordinateur quand le poste l'a donné, nom, date d'inscription, dernier contact), date du dernier bulletin appliqué.
2. Il envoie ces données au portail (`POST /api/licence/etat`).
3. Le portail met à jour la liste des appareils de l'entreprise et répond par un **bulletin** signé de sa clé : `KADENZ-B-<données>.<signature>`, comme les licences et les attestations d'essai.
4. Le poste transmet le bulletin au serveur, qui le vérifie (clés du portail, identifiant de l'entreprise, date d'émission plus récente que le dernier bulletin appliqué) et l'applique.

Un poste ne peut ni forger ni modifier un bulletin. Il peut seulement ne pas le transmettre, ce qui revient à ne pas être connecté : c'est le risque accepté de la section 1.

## 4. Bulletin (crate `kadenz-licence`, module `bulletin`)

Données signées :

- `type: "bulletin"`, `version`, identifiant de la clé du portail ;
- `entreprise` : identifiant de l'entreprise ;
- `emis_le` : date et heure UTC à la seconde ;
- `revoquee` : vrai si la licence de l'entreprise est révoquée ;
- `retires` : empreintes de certificat des postes à retirer (liste, éventuellement vide) ;
- `bloques` : identifiants d'ordinateur bloqués (liste complète à jour, éventuellement vide).

Règles du serveur :

- refus si la signature, l'identifiant de l'entreprise ou la clé ne vont pas ;
- refus d'un bulletin plus ancien que le dernier appliqué, ou de même date : rejouer un vieux bulletin « non révoquée » ne lève pas une révocation ;
- `revoquee` vrai : lecture seule, avec la raison « licence révoquée par l'éditeur : contactez votre revendeur » ; faux : la révocation est levée et l'état suit de nouveau la licence installée ; une licence installée ou réinstallée ne change jamais cet état ;
- `bloques` remplace la liste des ordinateurs bloqués gardée par le serveur : tout poste inscrit sur l'un d'eux est retiré, et une inscription depuis l'un d'eux est refusée (« cet ordinateur a été retiré de la licence : contactez votre revendeur ») ;
- le bulletin appliqué remet à zéro le compte des 30 jours sans contact ;
- chaque poste de `retires` encore inscrit est retiré (`postes::retirer`) ; le poste qui relaie peut en faire partie, il est alors coupé à la fin de l'échange ;
- un bulletin appliqué est gardé (texte, date) dans la base du serveur.

## 5. Portail

### 5.1 Données (schéma 14)

- Table `appareil` : entreprise, empreinte du poste, identifiant de l'ordinateur, nom, inscrit le, dernier contact rapporté, vu par le portail le, à retirer depuis (date de la demande de déconnexion, vide sinon), retiré le (date où un état sans ce poste est revenu).
- Table `revocation` : entreprise, révoquée le, par (compte), raison, rétablie le, rétablie par.
- Journal, qui ne s'efface pas : les changements seulement, jamais chaque état reçu : `appareil_nouveau` (nom, ordinateur, version, adresse), `appareil_retire`, `appareil_deconnexion`, `ordinateur_reautorise`, `licence_revoquee` (raison), `licence_retablie`.

### 5.2 Route `POST /api/licence/etat`

- Demande JSON : `entreprise`, `produit`, `version`, `postes` (au plus 1000 : `empreinte`, `ordinateur` (vide pour un poste ancien), `nom`, `inscrit_le`, `vu_le`).
- Entreprise inconnue du portail : refus (aucun bulletin) ; un serveur sans licence activée n'a rien à révoquer.
- Les postes reçus mettent à jour `appareil` ; un poste « à retirer » absent de la liste est noté retiré.
- Réponse : `{ "bulletin": "KADENZ-B-..." }`, ou `{ "erreur": "<modèle>" }` avec un statut 4xx.
- Frein par adresse et par entreprise, comme l'activation. Risque accepté, comme l'activation : qui connaît l'identifiant d'une entreprise (non public) peut envoyer une fausse liste de postes ; elle n'a d'effet que sur l'affichage du portail, jamais sur le serveur du client.

### 5.3 Pages

- Fiche de l'entreprise, rubrique « Appareils » : nom, inscrit le, dernier contact, état (actif, déconnexion demandée, retiré, bloqué), bouton « Déconnecter » par appareil, qui retire le poste et bloque son ordinateur ; « Réautoriser » sur un ordinateur bloqué.
- Édition un poste : « Changer d'ordinateur » libère le code de l'entreprise actuelle, qui est révoquée, pour qu'une nouvelle installation puisse l'activer avec la même échéance.
- Bouton « Révoquer la licence », avec une raison et une confirmation ; « Rétablir la licence » une fois révoquée. L'état et la date sont affichés en tête de la fiche.
- Qui peut agir : l'éditeur, et le revendeur de l'entreprise pour ses propres clients. Chaque action est notée au journal.
- Tant que la licence est révoquée, l'activation d'un code par cette entreprise est refusée (« licence révoquée : contactez votre revendeur »).

## 6. Client

### 6.1 Serveur (crates `protocole`, `serveur`, `stockage`)

- Requête `EtatLicence` (tout poste connecté) : rend l'identifiant de l'entreprise, l'édition, les postes inscrits et la date du dernier bulletin.
- Requête `AppliquerBulletin { texte }` (tout poste connecté) : vérifie et applique selon la section 4, rend le nouvel état de l'abonnement.
- Migration : table du dernier bulletin appliqué (texte, date d'émission, date d'application), date de départ du compte des 30 jours, ordinateurs bloqués ; identifiant de l'ordinateur et dernier contact de chaque poste.
- Inscription d'un poste : le poste donne l'identifiant de son ordinateur ; refus s'il est bloqué.
- `abonnement::evaluer` tient compte de la révocation et des 30 jours sans contact : lecture seule et raison propre, avant l'échéance.

### 6.2 Poste (crate `app`)

- Au lancement, puis toutes les 6 heures tant que l'application est ouverte, un fil à part fait l'échange de la section 3. Aucun message à l'écran tant que tout va bien ; sans Internet, rien ne se passe et l'échange repart à l'heure suivante prévue.
- Une révocation s'affiche comme l'échéance : bandeau de lecture seule avec la raison.
- Appareil retiré : la session se coupe avec le message « ce poste a été retiré de l'entreprise ».
- Les versions 0.3.4 et antérieures ne font pas cet échange : la révocation ne les atteint pas. Elles reçoivent la mise à jour comme les autres.

## 7. Confidentialité

La page `/confidentialite` et le LISEZMOI décrivent le nouvel envoi : nom, empreinte et identifiant d'ordinateur des postes, dates d'inscription et de dernier contact, envoyés au portail par un poste de l'entreprise toutes les 6 heures, pour afficher les appareils de la licence et appliquer une révocation. Aucune donnée de gestion.

## 8. Lots

| Lot | Contenu | Où |
|---|---|---|
| R1 | Bulletin signé `KADENZ-B-` : émission, vérification, refus. Tests. | `crates/licence` |
| R2 | Schéma 14 (`appareil`, `revocation`), route `/api/licence/etat`, frein, journal, refus d'activation d'une licence révoquée. Tests. | `crates/portail` |
| R3 | Rubrique « Appareils » de la fiche entreprise, déconnexion et réautorisation, révocation et rétablissement, changement d'ordinateur en édition un poste, droits éditeur et revendeur. Tests. | `crates/portail` |
| R4 | Requêtes `EtatLicence` et `AppliquerBulletin`, migration, identifiant d'ordinateur à l'inscription et ordinateurs bloqués, révocation et 30 jours sans contact dans l'évaluation de l'abonnement, retrait des postes. Tests. | `crates/protocole`, `crates/serveur`, `crates/stockage` |
| R5 | Échange au lancement et toutes les 6 heures, messages traduits, essai à l'écran. | `crates/app` |
| R6 | Page de confidentialité, LISEZMOI, notes de version ; déploiement du portail et release avec l'accord de l'utilisateur. | portail, installateur |
