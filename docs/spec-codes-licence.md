# Kadenz - Codes de licence courts (spécification)

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-18 | Premier jet par kadenz-c1, d'après les décisions de l'utilisateur du 2026-11-18 (décision 74 du cahier de la phase 2). À valider avec kadenz-6d avant tout code. |
| 0.2 | 2026-11-18 | Relecture de kadenz-6d : séparateurs retirés sans rien dire et autres caractères signalés, résumé dans la réponse du portail, champ sur un `TextInput` avec curseur en octets, lot L5 réparti, question ouverte sur la lecture des postes dans le code. Lot L1 livré. |
| 0.3 | 2026-11-18 | Lots L2a et L3 livrés. L2a : schéma du portail en version 2 avec chemin de migration, code tiré depuis un paiement, route `/api/activation`. Règle des dates alignée sur l'émission existante. Codes à vendre reportés en L2b. Essai de bout en bout revu : l'exécutable du portail n'accepte que la clé embarquée. |
| 0.4 | 2026-11-18 | Lot L2b livré : schéma en version 3, rubrique « Codes à vendre » du portail (tirage de 1 à 50 codes, liste, codes en clair affichés une seule fois), annulation d'un code libre par l'éditeur ou par celui qui l'a tiré (le paiement d'un code annulé redevient libre). |
| 0.5 | 2026-11-18 | Lot L5 livré : étape Licence de KadenzAdmin avec `ChampCode`, activation au passage de l'étape Entreprise (global `ActivationUi`, module partagé `activation_ui.rs`), traductions. Variable `KADENZ_PORTAIL_CLE` des versions de développement pour l'essai de bout en bout avec les exécutables ; essai fait à l'écran (section 8). |
| 0.6 | 2026-11-18 | Section 7 bis : décision 75 (licences de la ligne V1 par codes, abonnement daté, longues clés émises gardées, multiposte V1 sur données partagées) et proposition technique (licence V1 en version 2, gamme du code, branche `release/0.2.x`). |

## 1. Décisions de l'utilisateur (2026-11-18)

- Les licences se saisissent par un code court, comme chez VMware ou Kaspersky, dans un champ qui met tout en majuscules, place les séparations tout seul et accepte le collage.
- Format : 25 caractères en 5 groupes, `XXXXX-XXXXX-XXXXX-XXXXX-XXXXX`, sans O, 0, I ni 1, le dernier caractère servant de contrôle.
- Activation en ligne seulement : le code part au portail, qui renvoie la licence signée ; Internet est obligatoire pour activer et pour renouveler.
- Toutes les licences passent à ce format, y compris les sortes existantes ; le nombre de postes est porté par le code, comme les gammes Kaspersky.

**Question ouverte (posée à l'utilisateur le 2026-11-18)** : le nombre de postes doit-il se lire dans le code lui-même, ou suffit-il que le portail le connaisse ?
- (a) jeton pur, retenu en attendant : le nombre de postes est libre et connu du portail seul ; l'écran affiche « Ce code ouvre N postes » avant de confirmer ;
- (b) le premier caractère code un palier (1, 3, 5, 10...), lisible hors ligne, avec 23 caractères de hasard (115 bits) ; les paliers seraient alors à fixer par l'utilisateur.
Passer de (a) à (b) ne change que le tirage et le contrôle du code.
- Ordre de mise en œuvre laissé à la session ; le monoposte vient en dernier.

## 2. Principe

Une signature Ed25519 ne tient pas dans 25 caractères. Le code court est donc un **code d'activation** : un jeton tiré au hasard, enregistré au portail avec ce qu'il ouvre (postes, formule, modules, émetteur). La licence d'abonnement signée `KADENZ-E-...` (jalon F3) reste ce que le serveur vérifie ; seul le chemin pour l'obtenir change. Vérification, lecture seule à l'échéance, nombre de postes et leurs essais ne bougent pas.

Personne ne voit ni ne colle plus la longue clé : elle voyage du portail au programme qui a saisi le code, puis au serveur par le chemin d'installation actuel.

## 3. Format du code (crate `kadenz-licence`, module `code_court`)

- Alphabet de 32 signes : `ABCDEFGHJKLMNPQRSTUVWXYZ23456789` (A à Z sans I ni O, 2 à 9).
- 24 caractères tirés par un générateur cryptographique (120 bits), plus 1 caractère de contrôle calculé par l'algorithme de Luhn en base 32 : il détecte toute substitution d'un caractère et toute inversion de deux caractères voisins, sauf celle de A et 9 (rangs 0 et 31 de l'alphabet, limite connue de l'algorithme, vérifiée par essai).
- Normalisation de la saisie : majuscules ; tout caractère hors de l'alphabet est retiré ; 25 caractères au plus. Les séparateurs (tiret, tiret bas, espace, tabulation, retour à la ligne, espace insécable, tirets typographiques) le sont sans rien dire ; un O, un 0, un I ou un 1 est retiré et signalé (« Ambigu »), tout autre caractère aussi (« Autre »).
- Écriture : groupes de 5 séparés par un tiret.
- Contrôle local avant tout appel réseau : 25 caractères et caractère de contrôle juste, sinon un message clair (« Code incomplet », « Code erroné : vérifiez la saisie »).
- O, 0, I et 1 tapés par erreur : jamais remplacés en silence ; le champ affiche un message qui le dit.

## 4. Portail (crate `kadenz-portail`)

### 4.1 Données (schéma en version suivante, `PRAGMA user_version`)

Table `code_activation` :
- `empreinte` : SHA-256 du code normalisé, unique ; le code en clair n'est jamais gardé, comme les codes d'inscription des postes ;
- `fin` : 5 derniers caractères, pour le reconnaître dans les listes ;
- `postes`, `formule` (mensuelle, annuelle), `modules`, `emetteur` (l'éditeur ou un revendeur) ;
- `paiement` : le paiement d'où il vient, facultatif ;
- `entreprise` : vide tant qu'il n'est pas activé, sauf code émis pour une entreprise connue ;
- `etat` : libre, activé ou annulé ; `cree_par`, `cree_le`, `active_le` ;
- `licence` : la licence émise à l'activation.

La table `licence_emise` reçoit un lien vers le code ; son lien au paiement devient facultatif (reconstruction de la table, données gardées).

### 4.2 Émission des codes

- Depuis un paiement (chemin actuel) : la page d'émission produit un code à la place du texte de la licence ; postes et formule repris du paiement, code lié d'avance à l'entreprise du paiement.
- Codes à vendre (nouveau) : l'éditeur ou un revendeur crée un ou plusieurs codes pour un nombre de postes et une formule, sans entreprise ; elle se lie à l'activation. Le nombre de postes est un nombre libre saisi à l'émission (1 à 1000, comme les paiements) : aucune gamme n'est figée tant que l'utilisateur ne l'a pas donnée.
- Le code en clair ne s'affiche qu'une fois, à sa création, pour être copié ou imprimé ; ensuite seule sa fin reste visible.
- Annulation d'un code libre ; chaque création, activation et annulation est notée au journal.

### 4.3 Activation (route JSON)

`POST /api/activation`, hors des routes de la console (ni cookie ni jeton de formulaire), réponse JSON.

Demande : `{ "code", "entreprise", "client", "version" }` (identifiant de l'entreprise, raison sociale, version du logiciel).

Règles :
- code inconnu ou annulé : refus ;
- code libre : il se lie à l'entreprise (créée au portail si elle n'y est pas encore, avec la raison sociale reçue), la licence est émise et signée, le code passe à « activé » ;
- code déjà activé par la même entreprise : la même licence est renvoyée, sans nouvelle émission (réinstallation) ;
- code activé ou lié d'avance à une autre entreprise : refus ;
- début : le jour de l'activation ; échéance : celle de la dernière licence de l'entreprise si elle court encore, sinon la veille, plus un mois ou un an selon la formule (même règle que l'émission depuis un paiement, section 5 de la spécification du portail ; un serveur remplace la licence installée dès qu'il en reçoit une nouvelle) ;
- modules : ceux du code ;
- frein : 5 échecs par adresse ou par entreprise, puis attente doublée jusqu'à 15 minutes, avec `frein.rs` ; chaque échec noté au journal. Risque accepté : qui connaît l'identifiant d'une entreprise (non public) peut retarder ses activations par de faux codes ;
- un code tiré par un revendeur désactivé ensuite s'active quand même, avec ce revendeur pour émetteur : le client a payé (proposé, l'utilisateur peut préférer un refus) ;
- toute demande mal formée (JSON illisible, identifiant d'entreprise ou raison sociale invalide) reçoit le modèle « demande d'activation invalide » : le client ne reçoit que des modèles qu'il sait traduire.

Réponse : `{ "licence": "KADENZ-E-...", "postes", "formule", "debut", "echeance" }` (le résumé sert à l'écran sans relire la licence), ou `{ "erreur": "<modèle>" }` avec un statut 4xx ; le programme traduit le modèle.

## 5. Côté client

- L'appel au portail part du programme où le code est tapé : la fenêtre d'administration du serveur (`KadenzAdmin`, étape Licence de l'assistant) et la rubrique « Abonnement » d'un poste Admin (renouvellement). Le service Windows du serveur, qui tourne sous SYSTEM, n'ouvre aucune connexion vers Internet.
- Client HTTPS : `ureq` avec les certificats de Windows, comme les mises à jour (`crates/maj`) ; délai de 20 secondes ; aucune donnée de l'entreprise n'est envoyée en dehors de la demande ci-dessus.
- Adresse du portail : fichier embarqué (`crates/maj/portail-url.txt`), vide tant que le nom de domaine n'est pas choisi (décision 72) ; vide, l'activation répond « Activation indisponible : adresse du portail non configurée ». Variable `KADENZ_PORTAIL` pour les essais, en version de développement seulement, comme `KADENZ_MAJ_CLE`.
- La licence reçue suit ensuite le chemin actuel : installation dans l'assistant, ou `RenouvelerAbonnement` vers le serveur. Elle n'est enregistrée qu'après `entreprise::verifier` avec `cles_portail()` et l'identifiant de l'entreprise de la base, comme une licence collée : une réponse d'un faux portail n'installe rien (le client d'activation ne contrôle que le préfixe `KADENZ-E-`).
- Assistant de KadenzAdmin (L5) : l'étape Licence ne fait que contrôler le code sur place ; l'appel au portail part au « Suivant » de l'étape Entreprise, car il a besoin de la raison sociale. Pendant l'appel, le bouton affiche « Activation de la licence… » et les deux boutons sont grisés. Réussite : la licence vérifiée prend la place de l'ancienne clé collée, le récapitulatif montre client, postes et échéance, et l'installation suit le chemin existant. Refus ou portail injoignable : retour à l'étape Licence avec le message, code gardé ; on peut corriger, réessayer, ou vider le champ et installer sans licence. Un code modifié annule la licence déjà reçue.
- L'appel bloquant (`activation_ui::activer`) tourne dans un fil à part ; le refus est traduit à son retour dans la boucle de l'interface (`activation_ui::message`), la langue étant propre à ce fil. La rubrique « Abonnement » (L5 bis) reprend le même global et le même module.
- Erreurs de transport : un certificat refusé a son propre message (date et heure du poste, mandataire qui intercepte), le reste donne « portail d'activation injoignable ».
- L'identifiant de l'entreprise n'a plus à être transmis à la main : le programme l'envoie avec le code. Le texte « Transmettez cet identifiant pour recevoir la licence » disparaît ; l'identifiant reste affiché pour le support.

## 6. Champ de saisie (`composants.slint`, kadenz-6d)

- Un composant `ChampCode` sur un `TextInput` aux couleurs du thème (le `LineEdit` standard ne donne pas la position du curseur) : une ligne, police à chasse fixe, indice `XXXXX-XXXXX-XXXXX-XXXXX-XXXXX`. Il sert à l'application et à KadenzAdmin, qui importent tous deux `composants.slint`.
- À chaque modification (`edited`), un rappel global `CodeCourtUi.formater` appelle `code_court::mettre_en_forme(precedent, nouveau)` : texte précédent déjà mis en forme, nouveau texte brut. La fin de la partie tapée, collée ou effacée se trouve par le suffixe commun ; le curseur revient après le même caractère utile, en octets comme Slint ; le signal donne le message à afficher. Aucune logique en double côté Slint.
- Collage : un code collé avec ou sans tirets, en minuscules, avec des espaces ou un retour à la ligne, donne le même code.
- Le bouton d'activation s'active quand le code est complet et juste ; sinon le message de la section 3.

## 7. Licences existantes (proposé, l'utilisateur peut revenir dessus)

- Une licence d'abonnement déjà installée reste valable jusqu'à son échéance ; son renouvellement passe par un code. Les écrans n'ont plus de champ pour une longue clé.
- La licence d'essai « dev-kadenz » (9999 postes, sans échéance) est remplacée par un code émis au portail d'essai.
- Licences V1 : voir la section 7 bis (décision 75).

## 7 bis. Ligne V1 (0.2.x) : décision 75 et proposition

Décision de l'utilisateur (2026-11-18, cahier de la phase 2, décision 75) :
- un code activé sur la ligne V1 donne un abonnement daté, renouvelé par un nouveau code ; le perpétuel ne vaut plus que pour les licences V1 déjà émises (décision 7) ;
- les longues clés V1 déjà remises restent valables, sans rien faire chez les clients ;
- l'édition multiposte de la ligne V1 veut dire plusieurs postes sur les mêmes données : elle réutilise le serveur et les postes de la phase 2 (1.x), présentés comme la gamme des petits commerces ; la ligne 0.2.x reste monoposte (précision du même jour).

Constats vérifiés sur le tag `v0.2.2` (`crates/licence/src/lib.rs`) :
- la 0.2.2 accepte une licence V1 signée par la clé suivante (`cle-publique-suivante.hex`, identique dans `v0.2.2` et sur main), qui est la clé du portail (décision 29). Le portail peut donc signer des licences V1 sans la clé V1, qui reste hors ligne chez l'éditeur ;
- la 0.2.2 refuse toute licence dont le champ `version` diffère du sien (1). Une licence V1 datée en version 2 n'est donc jamais acceptée par une 0.2.2, où elle deviendrait perpétuelle faute d'y lire l'échéance.

Proposition (kadenz-3a, à relire avec kadenz-74) :
- Format : licence V1 en version 2, même préfixe `KADENZ-`, mêmes champs plus `echeance` et `postes` (1 en monoposte), signée par la clé du portail avec son identifiant `cle`. Une 0.2.3 lit les versions 1 (perpétuelles, longues clés déjà émises) et 2 (datées).
- Échéance : même règle que la phase 2 (avertissements à 30 jours, 7 jours et la veille, puis lecture seule ; décisions 25 et la « lecture seule à l'échéance »), les données restant gardées.
- Portail : un code porte sa gamme (`entreprise` ou `v1`), fixée au tirage. Pour un code V1, la demande d'activation envoie l'identifiant du poste (`XXXX-XXXX-XXXX-XXXX`) à la place de l'identifiant d'entreprise ; la licence renvoyée est une licence V1 version 2 liée à ce poste. Un code d'une gamme présenté par un logiciel de l'autre est refusé avec un message à lui.
- Client 0.2.3 : branche `release/0.2.x` tirée de `v0.2.2` (décision 39), qui reprend `code_court`, le client d'activation et `ChampCode` ; l'écran de licence perd le champ de la longue clé pour les nouvelles licences et garde la lecture de celle qui est installée.
- Ligne 0.2.x : monoposte seulement, un code de 1 poste. Le multiposte « petits commerces » est un code de gamme `entreprise` (serveur et postes 1.x) ; ce qui le distingue (prix, modules) relève de la grille de prix, encore ouverte (décision 73).

## 8. Essais

- `code_court` : aller-retour normalisation et écriture ; collages variés ; tout code tiré passe le contrôle ; une substitution est toujours détectée, une inversion de deux voisins aussi, sauf A et 9.
- Portail, dans le processus : code inconnu, code libre activé, réactivation par la même entreprise (même licence), autre entreprise refusée, code annulé, frein après 5 échecs, début calé sur l'échéance précédente.
- Bout en bout, en local seulement. Dans les essais automatiques, le portail tourne dans le processus d'essai (`web::lancer` sur 127.0.0.1, clé d'essai), avec le client d'activation pointé dessus (`client_d_activation_contre_le_portail`, `activation_par_la_route_json`). À l'écran, une version de développement lit `KADENZ_PORTAIL_CLE` (clé publique d'essai en hexadécimal) à la place de la clé embarquée, dans `cles_portail()` : le portail, le serveur et KadenzAdmin acceptent alors la clé d'essai ; une version optimisée l'ignore. Jamais la clé de production, jamais le VPS : la mise en ligne reste une décision de l'utilisateur.
- Essai à l'écran de L5 (2026-11-18, dossiers d'essai, portail d'essai sur 127.0.0.1, KadenzAdmin sans élévation, `ProgramData` détourné pour le dossier des sauvegardes) : code tapé en minuscules sans tirets remis en forme ; caractère de contrôle faux signalé et « Suivant » refusé ; code juste activé à l'étape Entreprise, récapitulatif « 3 postes, jusqu'au 17/11/2027 », installation terminée avec la licence ; même code renvoyé par la même entreprise, même licence ; même code depuis une autre base refusé (« Ce code d'activation appartient à une autre entreprise. »), retour à l'étape Licence, code gardé ; portail arrêté : « Portail d'activation injoignable : vérifiez la connexion à Internet. »
- Chaque texte nouveau dans les six fichiers de traduction.

## 9. Ce qui bloque la publication

Avec l'activation en ligne seulement, une version qui contient ces codes ne s'active chez aucun client tant que le portail n'est pas en ligne avec cette route et un nom de domaine. La mise en ligne (VPS, domaine) est une décision de l'utilisateur ; les sessions la préparent.

## 10. Découpage proposé

| Lot | Contenu | Session |
|---|---|---|
| L1 | Module `code_court` (format, contrôle, normalisation, mise en forme du champ) et ses essais | kadenz-c1, livré |
| L2a | Portail : schéma en version 2 et migration, code tiré depuis un paiement, route d'activation, pages, essais | kadenz-c1, livré |
| L2b | Portail : codes à vendre (lien au paiement facultatif, reconstruction de `licence_emise`), annulation d'un code | kadenz-c1, livré |
| L3 | Client d'activation (`kadenz_maj::activation`, qui a déjà le client HTTPS), adresse du portail (`crates/maj/portail-url.txt`) | kadenz-c1, livré |
| L4 | `ChampCode` dans `composants.slint` | kadenz-6d, livré |
| L5 | Étape Licence de l'assistant de KadenzAdmin, avec ses traductions | kadenz-c1, livré |
| L5 bis | Rubrique « Abonnement » du poste, avec ses traductions | kadenz-6d |
| L6 | Essai de bout en bout local, captures | kadenz-c1 ; fait pour KadenzAdmin, reste la rubrique « Abonnement » après L5 bis |
