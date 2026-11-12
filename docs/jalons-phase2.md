# Kadenz phase 2 - Jalons des fondations (proposition)

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-12 | Proposition de kadenz-8e, à relire par kadenz-34 puis à valider par l'utilisateur : découpage des fondations (étape 1 du cahier de la phase 2, section 5) en jalons vérifiables, d'après `docs/spec-synchronisation.md` 0.3 et `docs/spec-ecrans-multiposte.md` 0.3. |

Statut : **proposé**. Rien ne démarre avant l'accord de l'utilisateur sur ce découpage. Chaque jalon se termine par une vérification visible (tests, essai à l'écran sur des bases d'essai, jamais sur les données réelles), un commit poussé sur `main` et une ligne au journal, comme les jalons J1 à J10 de la V1. Aucune version de la phase 2 n'est publiée avant le jalon F9, et jamais avec le drapeau Latest (décision 14).

Rôles : **kadenz-34** pour le serveur, le stockage, le protocole, la licence et l'installateur ; **kadenz-8e** pour les écrans du poste et de la fenêtre d'administration, et les traductions. Une seule session compile à la fois, comme aujourd'hui.

## Jalons

| Jalon | Contenu | Qui | Vérification | Dépend de |
|---|---|---|---|---|
| F1 | Identifiants et opérations : `uid` (UUID version 7) sur les lignes synchronisées, identifiant d'entreprise, table des opérations idempotentes, application par `kadenz-stockage`, version par ligne, suite numérotée des changements (spec. synchronisation, sections 3, 4 et 6) | kadenz-34 | tests sans écran : opération renvoyée appliquée une seule fois, numéro rendu identique après une coupure, ordre respecté ; migration d'une base V1 d'essai sans perte | rien |
| F2 | Serveur minimal : programme serveur en service Windows, base chiffrée avec une clé protégée au niveau de la machine, TLS 1.3 avec certificat créé à l'installation, comptes vérifiés sur le serveur, horodatage en UTC dans le fuseau de l'entreprise (sections 4, 7 et 8) ; installateur du serveur (service, règle de pare-feu, désinstallation qui n'efface jamais les données) | kadenz-34 | service installé et démarré sur le poste de développement, connexion d'un client d'essai en TLS avec épinglage | F1 |
| F3 | Fenêtre d'administration et inscription : fenêtre du serveur (état, empreinte, premier compte Admin, codes d'inscription de 24 heures) ; sur le poste, assistant de premier lancement par code d'inscription, découverte sur le réseau local, épinglage, rubrique « Serveur » (écrans, sections 1 et 2) | kadenz-8e, avec kadenz-34 pour l'appel au serveur | un poste d'essai s'inscrit avec un code ; un code expiré, déjà servi ou annulé est refusé ; un autre certificat bloque la connexion | F2 |
| F4 | Poste relié en ligne : l'écran lit la copie locale, chaque opération part au serveur et la copie suit la réponse ; comptes communs ; validation et numérotation par le serveur ; état de la connexion (écrans, section 3) | kadenz-8e et kadenz-34 | essai à deux postes sur la même machine (deux bases d'essai) : un client créé sur l'un apparaît sur l'autre ; deux validations simultanées donnent deux numéros qui se suivent | F3 |
| F5 | Hors connexion : file d'envoi, brouillons, bouton Valider inactif et liste « à valider au retour », conflits avec comparaison et choix champ par champ, opérations en attente pour l'Admin (écrans, sections 4 à 6 ; spec., section 6) | kadenz-8e et kadenz-34 | essai en coupant le serveur : saisie hors ligne, retour, conflit créé exprès puis tranché, compte désactivé pendant la coupure et ses opérations en attente | F4 |
| F6 | Reprise V1 et sauvegardes du serveur : assistant de reprise d'une base V1 ou d'un `.kzs` (une seule base), sauvegardes chaque jour et à l'arrêt, dix gardées, restauration depuis la fenêtre, avertissement si même disque (écrans, sections 1 et 8 ; spec., section 8) | kadenz-34 et kadenz-8e | reprise d'une copie d'essai d'une base V1, numérotation continue ; restauration sur une machine « neuve » simulée | F4 |
| F7 | Relais et accès à distance : relais en conteneur, connexion sortante du serveur, TLS de bout en bout à travers le relais, rubrique « Accès à distance » (écrans, section 9 ; spec., section 7) | kadenz-34 pour le relais, kadenz-8e pour les écrans | relais lancé en local dans un conteneur, poste distant simulé ; mise en service sur le VPS quand il existe | F5 ; VPS pour la mise en service |
| F8 | Abonnement et portail : licence d'entreprise datée, signée par la clé du portail, émetteur, nombre de postes ; garde-fou contre le retour en arrière de l'horloge ; avertissements à 30 jours, 7 jours et la veille ; lecture seule ; rubrique « Abonnement » ; portail web avec comptes éditeur et revendeur (décisions 21, 25 à 27, 29) | kadenz-34 pour la licence et le portail, kadenz-8e pour les écrans | licence d'essai à échéance proche puis dépassée : avertissements, lecture seule, consultation, impression et export toujours possibles | F4 ; VPS, nombre de postes inclus et prix, prestataire de paiement par carte |
| F9 | Signature du code et première version de la phase 2 : signature du serveur, du poste et des installateurs ; règle des 25 releases dans `scripts/release.ps1` ; publication sans le drapeau Latest | kadenz-34 | release d'essai en simulation, signée ; drapeau Latest refusé | F1 à F8 ; certificat de signature (décision 24) |

## Ce qui reste hors de ces jalons

- Le chantier multi-devise (3.5) : la devise de l'entreprise devient un paramètre avec la décision 22, mais elle est traitée avec ce chantier, à l'étape 3.
- Le stock (3.3) et l'export SYSCOHADA (3.2) : étape 2, après les fondations.
- Le site de présentation (3.7) : possible en parallèle dès que le VPS existe, sans dépendre de ces jalons.

## Ce que l'utilisateur fournit, et quand

- **VPS** (prestataire et pays) : avant la mise en service de F7 et le portail de F8.
- **Nombre de postes inclus et prix**, **prestataire de paiement par carte** : avant F8.
- **Certificat de signature du code** : avant F9 (décision 24).
