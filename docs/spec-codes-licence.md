# Kadenz - Codes de licence courts (spécification)

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-18 | Premier jet par kadenz-c1, d'après les décisions de l'utilisateur du 2026-11-18 (décision 74 du cahier de la phase 2). À valider avec kadenz-6d avant tout code. |
| 0.2 | 2026-11-18 | Relecture de kadenz-6d : séparateurs retirés sans rien dire et autres caractères signalés, résumé dans la réponse du portail, champ sur un `TextInput` avec curseur en octets, lot L5 réparti, question ouverte sur la lecture des postes dans le code. Lot L1 livré. |
| 0.3 | 2026-11-18 | Lots L2a et L3 livrés. L2a : schéma du portail en version 2 avec chemin de migration, code tiré depuis un paiement, route `/api/activation`. Règle des dates alignée sur l'émission existante. Codes à vendre reportés en L2b. Essai de bout en bout revu : l'exécutable du portail n'accepte que la clé embarquée. |

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
- La licence reçue suit ensuite le chemin actuel : installation dans l'assistant, ou `RenouvelerAbonnement` vers le serveur.
- L'identifiant de l'entreprise n'a plus à être transmis à la main : le programme l'envoie avec le code. Le texte « Transmettez cet identifiant pour recevoir la licence » disparaît ; l'identifiant reste affiché pour le support.

## 6. Champ de saisie (`composants.slint`, kadenz-6d)

- Un composant `ChampCode` sur un `TextInput` aux couleurs du thème (le `LineEdit` standard ne donne pas la position du curseur) : une ligne, police à chasse fixe, indice `XXXXX-XXXXX-XXXXX-XXXXX-XXXXX`. Il sert à l'application et à KadenzAdmin, qui importent tous deux `composants.slint`.
- À chaque modification (`edited`), un rappel global `CodeCourtUi.formater` appelle `code_court::mettre_en_forme(precedent, nouveau)` : texte précédent déjà mis en forme, nouveau texte brut. La fin de la partie tapée, collée ou effacée se trouve par le suffixe commun ; le curseur revient après le même caractère utile, en octets comme Slint ; le signal donne le message à afficher. Aucune logique en double côté Slint.
- Collage : un code collé avec ou sans tirets, en minuscules, avec des espaces ou un retour à la ligne, donne le même code.
- Le bouton d'activation s'active quand le code est complet et juste ; sinon le message de la section 3.

## 7. Licences existantes (proposé, l'utilisateur peut revenir dessus)

- Une licence d'abonnement déjà installée reste valable jusqu'à son échéance ; son renouvellement passe par un code. Les écrans n'ont plus de champ pour une longue clé.
- La licence d'essai « dev-kadenz » (9999 postes, sans échéance) est remplacée par un code émis au portail d'essai.
- Licences V1 perpétuelles (postes 0.2.2, vérifiées hors ligne) : leur passage aux codes se fait avec la mise à jour de la ligne 0.2.2, à la fin, avec le monoposte.

## 8. Essais

- `code_court` : aller-retour normalisation et écriture ; collages variés ; tout code tiré passe le contrôle ; une substitution est toujours détectée, une inversion de deux voisins aussi, sauf A et 9.
- Portail, dans le processus : code inconnu, code libre activé, réactivation par la même entreprise (même licence), autre entreprise refusée, code annulé, frein après 5 échecs, début calé sur l'échéance précédente.
- Bout en bout, en local seulement. L'exécutable `kadenz-portail` refuse toute clé autre que la clé embarquée, et les programmes du client ne reconnaissent que les licences signées par elle. L'essai passe donc par le portail lancé dans le processus d'essai (`web::lancer` sur 127.0.0.1, clé d'essai), avec le client d'activation pointé dessus (`activation_par_la_route_json` pour la route). Jamais la clé de production, jamais le VPS : la mise en ligne reste une décision de l'utilisateur.
- Chaque texte nouveau dans les six fichiers de traduction.

## 9. Ce qui bloque la publication

Avec l'activation en ligne seulement, une version qui contient ces codes ne s'active chez aucun client tant que le portail n'est pas en ligne avec cette route et un nom de domaine. La mise en ligne (VPS, domaine) est une décision de l'utilisateur ; les sessions la préparent.

## 10. Découpage proposé

| Lot | Contenu | Session |
|---|---|---|
| L1 | Module `code_court` (format, contrôle, normalisation, mise en forme du champ) et ses essais | kadenz-c1, livré |
| L2a | Portail : schéma en version 2 et migration, code tiré depuis un paiement, route d'activation, pages, essais | kadenz-c1, livré |
| L2b | Portail : codes à vendre (lien au paiement facultatif, reconstruction de `licence_emise`), annulation d'un code | kadenz-c1 |
| L3 | Client d'activation (`kadenz_maj::activation`, qui a déjà le client HTTPS), adresse du portail (`crates/maj/portail-url.txt`) | kadenz-c1, livré |
| L4 | `ChampCode` dans `composants.slint` | kadenz-6d |
| L5 | Étape Licence de l'assistant de KadenzAdmin, avec ses traductions | kadenz-c1 |
| L5 bis | Rubrique « Abonnement » du poste, avec ses traductions | kadenz-6d |
| L6 | Essai de bout en bout local, captures | kadenz-c1 |
