# Kadenz - Codes de licence courts (spécification)

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-18 | Premier jet par kadenz-c1, d'après les décisions de l'utilisateur du 2026-11-18 (décision 74 du cahier de la phase 2). À valider avec kadenz-6d avant tout code. |
| 0.2 | 2026-11-18 | Relecture de kadenz-6d : séparateurs retirés sans rien dire et autres caractères signalés, résumé dans la réponse du portail, champ sur un `TextInput` avec curseur en octets, lot L5 réparti, question ouverte sur la lecture des postes dans le code. Lot L1 livré. |
| 0.3 | 2026-11-18 | Lots L2a et L3 livrés. L2a : schéma du portail en version 2 avec chemin de migration, code tiré depuis un paiement, route `/api/activation`. Règle des dates alignée sur l'émission existante. Codes à vendre reportés en L2b. Essai de bout en bout revu : l'exécutable du portail n'accepte que la clé embarquée. |
| 0.4 | 2026-11-18 | Lot L2b livré : schéma en version 3, rubrique « Codes à vendre » du portail (tirage de 1 à 50 codes, liste, codes en clair affichés une seule fois), annulation d'un code libre par l'éditeur ou par celui qui l'a tiré (le paiement d'un code annulé redevient libre). |
| 0.5 | 2026-11-18 | Lot L5 livré : étape Licence de KadenzAdmin avec `ChampCode`, activation au passage de l'étape Entreprise (global `ActivationUi`, module partagé `activation_ui.rs`), traductions. Variable `KADENZ_PORTAIL_CLE` des versions de développement pour l'essai de bout en bout avec les exécutables ; essai fait à l'écran (section 8). |
| 0.6 | 2026-11-18 | Section 7 bis : décision 75 (licences de la ligne V1 par codes, abonnement daté, longues clés émises gardées, multiposte V1 sur données partagées) et proposition technique (licence V1 en version 2, gamme du code, branche `release/0.2.x`). |
| 0.7 | 2026-11-18 | Section 7 bis complétée après la relecture de kadenz-74 : multiposte V1 sur le serveur de la phase 2 (ligne 0.2.x monoposte), retour au socle à l'échéance (proposé), longue clé encore saisissable, écrans de la 0.2.3, changement de poste (à trancher), répartition du travail. |
| 0.8 | 2026-11-18 | Décisions de l'utilisateur sur la ligne V1 : retour au socle à l'échéance, changement de poste par « Libérer le poste » au portail. |
| 0.9 | 2026-11-18 | Contrat de la ligne V1 figé : champ `poste` de la demande, licence V1 version 2, règle de clé par version, modèles de refus [5] et [6]. |
| 0.10 | 2026-11-18 | Correction : la 0.2.2 n'active pas les modules d'une licence signée par la clé du portail (son stockage ne vérifie qu'avec la clé V1) ; seule la vérification de son assistant accepte les deux clés. |
| 0.11 | 2026-11-18 | Plan du portail pour la ligne V1 (lot L8) ; lots L7, L7 bis et L8 au tableau. |
| 0.12 | 2026-11-19 | Lot L8 codé : schéma 5 du portail (gamme du code, table `licence_v1`), activation par poste, « Libérer le poste ». |

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
- la 0.2.2 embarque la clé suivante (`cle-publique-suivante.hex`, identique dans `v0.2.2` et sur main), qui est la clé du portail (décision 29), mais ne s'en sert que dans la vérification de l'assistant (`kadenz_licence::verifier`, `crates/app/src/licence.rs`) : le chemin qui active les modules (`kadenz_stockage::licence`, `etat` et `enregistrer`) vérifie avec la clé V1 seule et refuse donc une licence signée par la clé du portail (correction du 2026-11-18 ; la version 0.6 de ce document disait le contraire). Une 0.2.3 est donc nécessaire pour qu'une licence émise par le portail serve ; le portail signe sans la clé V1, qui reste hors ligne chez l'éditeur ;
- la 0.2.2 refuse toute licence dont le champ `version` diffère du sien (1). Une licence V1 datée en version 2 n'est donc jamais acceptée par une 0.2.2, où elle deviendrait perpétuelle faute d'y lire l'échéance.

Proposition (kadenz-3a, à relire avec kadenz-74) :
- Format : licence V1 en version 2, même préfixe `KADENZ-`, mêmes champs plus `echeance` et `postes` (1 en monoposte), signée par la clé du portail avec son identifiant `cle`. Une 0.2.3 lit les versions 1 (perpétuelles, longues clés déjà émises) et 2 (datées).
- Échéance (décidé par l'utilisateur le 2026-11-18, sur proposition de kadenz-74) : avertissements à 30 jours, 7 jours et la veille, puis retour au socle seul (tiers, produits, factures, avoirs, administration), exactement comme une clé invalide dans la 0.2.2 ; les données des modules sont gardées. La lecture seule de la phase 2 bloquerait aussi le socle, gratuit sans licence en V1, et demanderait un mode lecture seule dans tous les écrans de la 0.2.x.
- Portail : un code porte sa gamme (`entreprise` ou `v1`), fixée au tirage. Pour un code V1, la demande d'activation envoie l'identifiant du poste (`XXXX-XXXX-XXXX-XXXX`) à la place de l'identifiant d'entreprise ; la licence renvoyée est une licence V1 version 2 liée à ce poste. Un code d'une gamme présenté par un logiciel de l'autre est refusé avec un message à lui.
- Client 0.2.3 : branche `release/0.2.x` tirée de `v0.2.2` (décision 39), qui reprend `code_court`, `code_saisie.rs`, `traduction.rs` et le client d'activation tels quels ; `ChampCode` est recâblé sur les couleurs de la 0.2.2, qui n'a pas les jetons `Theme.champ` ni `Theme.focus` (portage des écrans : kadenz-74 ; portail, lecture de la version 2 et client d'activation : kadenz-3a).
- Écrans de la 0.2.3 (relecture de kadenz-74) :
  - Assistant, étape Licence : code facultatif, socle seul sans code ; `ChampCode` en premier, lien discret « J'ai une clé longue » qui ouvre l'ancienne `SaisieCle`, pour un client qui réinstalle avec une longue clé déjà émise (elles restent valables). Le texte « Transmettez l'identifiant » disparaît ; l'identifiant reste affiché pour le support.
  - Paramètres, Licence : lignes Échéance et Postes, bandeau avant l'échéance, bouton Renouveler qui ouvre `ChampCode` dans la page ; même lien vers la longue clé. Licence version 1 installée : « Perpétuelle », sans bandeau.
- Changement de poste (décidé par l'utilisateur le 2026-11-18 ; l'identifiant du poste suit le matériel) : un code V1 reste lié au poste qui l'a activé. L'éditeur ou le revendeur libère le poste sur le portail (« Libérer le poste »), puis le même code s'active sur le nouveau poste ; pas de réactivation libre, qui laisserait un code de 1 poste servir sur plusieurs PC jusqu'à l'échéance (une licence ne se retire pas d'un poste hors ligne). Message quand le poste est pris : « ce code est déjà activé sur un autre poste : demandez à votre revendeur de le libérer ». Risque formellement accepté par l'utilisateur le 2026-11-18 : entre la libération et l'échéance, l'ancien poste garde une licence signée valable, sans révocation possible hors ligne. Le limiter par des échéances courtes ou une liste de révocation embarquée demanderait une nouvelle version des postes.
Contrat figé (kadenz-3a, 2026-11-18), à ne plus changer sans monter les deux côtés : une 0.2.3 installée chez un client se met rarement à jour, et un modèle qu'elle ne connaît pas devient « réponse inattendue du portail d'activation ».
- Demande V1 (`POST /api/activation`, JSON) : `{"code", "poste", "client", "version"}`, `poste` au format `XXXX-XXXX-XXXX-XXXX` à la place de `entreprise`. Le portail reconnaît la gamme par le champ présent et la confronte à celle du code.
- Réponse : la même que pour la phase 2 (`licence`, `postes`, `formule`, `debut`, `echeance`), `licence` étant alors une licence V1 version 2.
- Licence V1 version 2 : `KADENZ-<données>.<signature>`, données JSON `{"version": 2, "cle", "client", "poste", "modules", "emise_le", "echeance", "postes"}`, `echeance` au format AAAA-MM-JJ (dernier jour valable), `postes` à 1. Émission et vérification dans `kadenz_licence` (une seule définition), vecteur d'essai fixe partagé par main et `release/0.2.x`.
- Règle de clé par version, dans la 0.2.3 : version 1 acceptée avec la clé V1 seulement (licences perpétuelles déjà émises) ; version 2 avec la clé du portail seulement. C'est déjà ce que fait le stockage de la 0.2.2 pour la version 1 ; la 0.2.3 aligne la vérification de l'assistant, qui accepte aujourd'hui les deux clés, sur cette règle, pour qu'un portail compromis ne puisse jamais signer du perpétuel. Vérifié au journal du dépôt : la seule licence V1 émise (2026-11-11) précède la clé suivante (2026-11-12).
- Deux modèles de refus ajoutés à `MODELES_ACTIVATION`, à la suite des cinq premiers : [5] « ce code ne sert pas à ce logiciel : vérifiez la gamme indiquée sur votre abonnement » (code `v1` présenté par une 1.x, ou l'inverse), [6] « ce code est déjà activé sur un autre poste : demandez à votre revendeur de le libérer ».
- Échéance dans la 0.2.3 : état d'abonnement calculé comme dans `kadenz_stockage::abonnement` (date du poste qui ne recule jamais) ; à l'échéance, retour au socle.

Portail, lot L8 (codé par kadenz-3a ; le schéma 4 est pris par le frein du lot D1) :
- Schéma en version 5 : `code_activation` gagne `gamme` (`entreprise` par défaut, ou `v1`) ; nouvelle table `licence_v1` (code, poste, client, texte signé, début, échéance, émise le, libérée le, libérée par), une ligne par poste qui a activé le code. La ligne en cours est celle sans date de libération, une au plus par code (index unique partiel) ; le poste lié se lit dans cette ligne, sans colonne `poste` sur le code.
- Tirage : la rubrique « Codes à vendre » demande la gamme ; un code V1 vaut 1 poste (refus sinon). Un code tiré depuis un paiement reste de gamme `entreprise`.
- Activation : une demande porte `poste` ou `entreprise`, jamais les deux ni aucun (sinon [4]) ; avec `poste`, elle ne vaut que pour un code `v1`, avec `entreprise` que pour un code `entreprise` (sinon [5]). Le poste est validé (quatre groupes de quatre caractères en base 32 de Crockford, majuscules) et entre dans les clés du frein comme l'entreprise. Code V1 libre : licence V1 datée signée par la clé du portail, début et échéance calculés comme pour la phase 2, poste lié. Même poste : même licence. Autre poste pendant que le code est lié : [6].
- « Libérer le poste » : bouton de la fiche du code, pour l'éditeur ou celui qui a tiré le code, journalisé. Le code redevient activable ; une nouvelle activation garde l'échéance de la première (le changement de PC ne prolonge rien).
- Essais : activation V1, réactivation, autre poste refusé, libération puis nouveau poste avec la même échéance, gamme croisée refusée, licence reçue acceptée par `kadenz_licence::verifier_selon` avec la clé du portail.

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
| L7 | Ligne V1 sur `release/0.2.x` : licence datée, échéance, `code_court`, client d'activation (champ `poste`) | kadenz-3a, en cours |
| L7 bis | Écrans de la 0.2.3 (assistant, page Licence) | kadenz-74 |
| L8 | Portail : codes V1, `licence_v1`, « Libérer le poste » | kadenz-3a, livré le 2026-11-19 |
| L6 | Essai de bout en bout local, captures | kadenz-c1 ; fait pour KadenzAdmin, reste la rubrique « Abonnement » après L5 bis |
