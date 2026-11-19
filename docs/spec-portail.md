# Kadenz phase 2 - Portail des licences (jalon F9)

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-15 | Forme proposée par kadenz-34 et kadenz-8e après les décisions 60 et 61 de l'utilisateur : programme, données, rôles, émission, journal, sécurité, essais. Rien n'est codé. |
| 0.2 | 2026-11-19 | Décisions de l'utilisateur du 2026-11-19 (section 8) : pages maud gardées, dépôt à part privé, hébergement sur le Dell. Relecture d'OpenCode et ordre des lots de durcissement D1 et D2 (section 9). |
| 0.3 | 2026-11-19 | Lot D1 livré (section 9) ; mots de passe de 15 caractères et liste de mots de passe compromis ; option `--mandataire <adresse>` ; sous-commande `sante`. |
| 0.4 | 2026-11-19 | Construction de l'image du portail sous Linux vérifiée sur le Dell (F9c), ainsi que l'image du relais ; lot L8 livré. |
| 0.5 | 2026-11-19 | Lot D2 livré (section 9) : jeton de la page de connexion, cookies `__Host-`, rotation de la session à la connexion, CSP sans script ni objet. |

Statut : **proposé**. Les choix marqués « proposé » restent à confirmer par l'utilisateur ; les décisions citées sont celles du cahier de la phase 2.

## 1. Périmètre (décisions 21, 27, 29, 60)

- Le portail remplace l'outil manuel `kadenz-editeur` pour les licences d'abonnement d'entreprise (format du jalon F3, `KADENZ-E-...`). Les licences V1 perpétuelles n'y passent pas.
- F9 est local et sans carte (décision 60) : l'éditeur ou un revendeur enregistre un paiement reçu (virement, chèque ou espèces) avec son montant et le nombre de postes payés, puis émet la licence. Aucune grille de prix ni aucun nombre de postes inclus n'est écrit dans le portail : ils viendront de l'utilisateur, avec le paiement par carte et son prestataire.
- Mise en service : plus tard, à la décision de l'utilisateur ; pour l'instant sur le serveur maison (section 8), pas sur un VPS.

## 2. Programme (décision 61)

- Un seul programme Rust, `kadenz-portail` (crate `crates/portail`), qui sert des pages HTML simples rendues sur le serveur : formulaires, aucun framework JavaScript. Bibliothèques : axum et tokio pour le HTTP, maud pour les pages (gabarits vérifiés à la compilation, échappement automatique), argon2 pour les mots de passe (choix de kadenz-34 et kadenz-8e).
- Il dépend de `kadenz-licence` (et de `kadenz-domaine` pour les modules), jamais de `kadenz-stockage` : le chiffrement de la base et des clés de `kadenz-stockage` repose sur la protection des données de Windows, et le portail tourne sous Linux. Il se construit dans un conteneur comme le relais (SQLite de l'espace de travail remplacé par SQLite3 Multiple Ciphers, favicon repris de `docs/` malgré le `.dockerignore`) ; cette construction sous Linux est vérifiée sur le Dell le 2026-11-19 (F9c), depuis 9510d00.
- Lancement : `kadenz-portail --dossier <dossier> --cle <fichier> [--ecoute 127.0.0.1:47382]`. Le fichier de clé est la clé privée du portail en hexadécimal, au format de `kadenz-editeur`. Le programme refuse de démarrer sans `--cle` ; il ne prend jamais une clé d'essai à la place.
- `kadenz-portail --dossier <dossier> creer-editeur <identifiant> <nom>` crée un compte éditeur, mot de passe saisi sans écho (ou lu sur l'entrée standard quand elle n'est pas un terminal). C'est la seule façon de créer un compte éditeur.
- `kadenz-portail sante [--ecoute <adresse:port>]` interroge `/sante` du portail local : code de sortie 0 si le portail répond 200 (vérification de santé de Docker, l'image n'a pas curl).
- `--mandataire <adresse>` : adresse IP du mandataire de confiance ; pour ses requêtes seulement, l'adresse du client est lue dans la dernière valeur de `X-Forwarded-For` (section 9).
- Écoute en HTTP sur `127.0.0.1` par défaut. Sur le VPS, le TLS est assuré par un mandataire devant le portail (Caddy, certificats automatiques ; proposé).
- Exemple de développement `portail_essai` : le même programme avec la clé d'essai du banc (`banc_essai`), pour les essais ; jamais livré.

## 3. Données

Base SQLite dans le dossier du portail (proposé : protégée par les droits du compte de service, fichiers en mode 0600 comme l'identité du relais, et par le chiffrement du disque du VPS ; pas de chiffrement propre à la base en F9).

- **Compte** : identifiant, nom, rôle (éditeur ou revendeur), mot de passe (argon2id), actif ou non. Un revendeur porte un identifiant public, repris comme émetteur dans les licences qu'il émet (décision 29).
- **Entreprise** : identifiant d'entreprise (celui que l'Admin lit dans Kadenz, « Identifiant de l'entreprise » : rubrique « Serveur » d'un poste relié et assistant d'installation du serveur ; ajouté à la rubrique « Abonnement » par F9b), nom du client, revendeur rattaché (aucun : client direct de l'éditeur).
- **Paiement** : entreprise, date de réception, mode (virement, chèque, espèces), montant entier et devise saisis, référence (numéro de chèque ou de virement, facultative), nombre de postes payés, formule (mensuelle ou annuelle), compte qui l'a saisi. Un paiement sert à une seule licence.
- **Licence émise** : entreprise, paiement, texte complet de la licence, début, échéance, postes, modules, formule, émetteur, date d'émission, compte.
- **Journal** : chaque action, avec l'heure UTC, le compte, l'action, l'entreprise ou le compte visé et les valeurs utiles (jamais un mot de passe). Ajout seul : la base refuse la modification et l'effacement d'une ligne.

## 4. Rôles (décision 27)

- **Éditeur** : voit et fait tout ; crée, désactive et réactive les comptes revendeurs ; rattache une entreprise à un revendeur.
- **Revendeur** : ne voit et ne touche que les entreprises qui lui sont rattachées ; crée ses entreprises (rattachées à lui), enregistre leurs paiements et émet leurs licences. Le contrôle est fait dans le crate, à chaque fonction, pas seulement dans les pages.
- Un compte désactivé ne se connecte plus ; ses sessions finissent.

## 5. Émission

- À partir d'un paiement non encore utilisé : formule et postes repris du paiement ; modules repris de la dernière licence de l'entreprise (tous les modules pour une première licence ; proposé), modifiables ; échéance saisie par l'opérateur, comme dans `kadenz-editeur`.
- Échéance pré-remplie (proposé, à confirmer par l'utilisateur) : l'échéance de la dernière licence, ou la date du jour si elle est passée ou s'il n'y en a pas, plus un mois ou un an selon la formule. La décision 21 exclut un délai de grâce et ne fixe pas d'autre règle.
- Début : date du jour (UTC). Date d'émission : date du jour. Émetteur : l'éditeur, ou le revendeur (identifiant et nom).
- La licence est signée par la clé du portail et vérifiée aussitôt avec la clé publique avant d'être enregistrée. Le texte s'affiche pour être copié dans la rubrique « Abonnement » d'un poste Admin. Un serveur accepte une seconde licence émise le même jour : la licence installée est toujours remplacée, seule la date retenue contre le retour en arrière dépend de la date d'émission (`abonnement::enregistrer_avec`).

- Depuis la décision 74 (codes de licence courts, `docs/spec-codes-licence.md`), la page d'émission tire un code d'activation à la place du texte : postes et formule du paiement, modules choisis, code lié d'avance à l'entreprise du paiement, affiché une seule fois. La licence est émise quand le programme du client envoie le code (`POST /api/activation`), avec l'échéance calculée ce jour-là par la règle ci-dessus.

## 6. Sécurité

- Mots de passe : argon2id, 15 caractères au moins, et refus des mots de passe compromis de la liste embarquée (lot D1, section 9).
- Session : jeton aléatoire de 32 octets, dans le cookie de session du navigateur `__Host-kadenz_portail` (`HttpOnly`, `Secure`, `SameSite=Strict`, `Path=/`, sans `Domain` ni `Max-Age`), gardé côté portail ; fin après 8 heures sans activité, 12 heures après l'ouverture quoi qu'il arrive, à la déconnexion, au changement de mot de passe et à l'arrêt du portail. Une connexion réussie ferme la session que portait le navigateur et en ouvre une neuve.
- Chaque formulaire porte un jeton anti-falsification de la session, vérifié avant toute écriture.
- Connexions freinées : après 5 échecs par identifiant ou par adresse, attente de 5 secondes doublée à chaque échec jusqu'à 15 minutes, comme les sessions du serveur ; chaque échec est noté au journal.
- En-têtes : `Content-Security-Policy: default-src 'self'; form-action 'self'; frame-ancestors 'none'`, `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`, `X-Robots-Tag: noindex`.
- Console privée : `noindex` sur chaque page et `robots.txt` qui interdit tout (hors de la liste SEO des sites publics).

## 7. Découpage et essai

- **F9a** (kadenz-34) : crate `kadenz-portail` (données, rôles, émission, journal, sessions, freinage), table des routes axum, programme et exemple `portail_essai` ; essais en processus, dont un revendeur refusé sur le client d'un autre revendeur et une licence émise acceptée par `abonnement::enregistrer_avec`.
- **F9b** (kadenz-8e) : pages dans `crates/portail/src/pages/` (un module par page), signatures fixées par F9a ; style des écrans (angles de 4 px au plus, palette et police de la marque, aucune italique), textes en français.
- **F9c** : construction de l'image du portail sous Linux (conteneur), puis essai à l'écran, portail d'essai lancé en local : l'éditeur crée un revendeur ; le revendeur crée une entreprise (identifiant d'un banc d'essai), enregistre un paiement et émet la licence ; la licence est acceptée par le banc d'essai depuis la rubrique « Abonnement », émetteur visible ; journal complet.

## 8. Décisions de l'utilisateur du 2026-11-19 (relayées par kadenz-cc)

- Les pages restent produites par le serveur (maud), sans front séparé : la proposition 2 du découpage (dépôt avec une API et un front séparés) est abandonnée. OpenCode le recommandait aussi (échappement automatique, aucune surface JavaScript).
- Le portail sort dans un dépôt à part, privé, sur le compte rachk02, avec ce dont il dépend ; le découpage se prépare après le lot D1.
- Pas de nom de domaine ni de VPS pour l'instant : le portail sera hébergé sur le serveur maison (Dell, Tailscale), le site public seul pouvant aller sur Vercel. Vercel ne convient pas au portail, qui a une base SQLite sur disque, une clé de signature en fichier, ainsi que des sessions et un frein en mémoire. Le site ne partage pas le nom `ts.net` du portail, à cause des cookies.
- L'adresse embarquée dans les clients (`crates/maj/portail-url.txt`) reste vide tant qu'aucune adresse n'est choisie. Une adresse Tailscale Funnel (`*.ts.net`) dépend du nom du tailnet : la question d'un nom de domaine est posée à l'utilisateur.
- Derrière Funnel, l'adresse du client arrive dans un en-tête posé par Tailscale. Son nom est à vérifier dans la documentation et à l'essai sur le Dell avant d'être lu (option `--mandataire`, lot D1).
- Docker est installé sur le Dell : la construction Linux du portail s'y essaiera après D1, avec une clé d'essai seulement.

## 9. Durcissement (relecture d'OpenCode, docs/securite/echanges-opencode.md)

Ordre décidé entre sessions le 2026-11-19 : D1 (livré le 2026-11-19), puis L8 (codes de la ligne V1, docs/spec-codes-licence.md ; livré le 2026-11-19), puis D2, puis la chaîne de hachage du journal et la double authentification des éditeurs, avant toute mise en service.

- D1, indépendant des pages :
  - verrou de la base relâché pendant Argon2 ;
  - Argon2 factice pour un compte inconnu ou désactivé ;
  - frein borné, purgé, persistant (schéma 4), clés construites après validation ;
  - adresse du client lue seulement depuis un pair de confiance ;
  - plus de verrouillage d'un compte par un tiers ;
  - erreur interne ni affichée ni détaillée dans la page ;
  - mot de passe de 15 caractères au moins, avec une liste embarquée de mots de passe compromis (SecLists, licence et provenance à vérifier), et mot de passe lu sans écho par `creer-editeur` ;
  - sessions fermées au changement de mot de passe, `busy_timeout` ;
  - fichier de clé : droits vérifiés au démarrage, effacement de la mémoire ;
  - sous-commande de santé ;
  - images du portail et du relais avec l'utilisateur 10001, sans binaire ni clé d'essai.
- D1, ce qui est en place :
  - Argon2 passe par une porte de 2 places, hors du verrou de la base ; sous le verrou, avant d'ouvrir la session, le compte doit être encore actif avec la même empreinte. Un compte inconnu est vérifié contre une empreinte factice.
  - Frein en base (table `frein`, schéma 4), avec 3 clés pour une connexion : adresse, identifiant depuis l'adresse (5 échecs, attente doublée jusqu'à 15 minutes), identifiant seul (50 échecs, 1 minute au plus). Les clés d'une activation suivent le même modèle, avec l'entreprise seulement si son identifiant est valide. Les lignes sans échec depuis un jour sont purgées, et 10 000 lignes sans attente sont gardées au plus ; une attente en cours n'est jamais retirée.
  - `X-Forwarded-For` n'est lu que pour une connexion venue de l'adresse donnée par `--mandataire`, et sa dernière valeur doit être une adresse IP. Tailscale remplace cet en-tête par l'adresse source réelle (`addProxyForwardedHeaders`, `ipn/ipnlocal/serve.go`), et Caddy y ajoute la sienne en dernier. Reste à relever à l'essai sur le Dell l'adresse du mandataire vue par le conteneur (sans doute la passerelle du réseau Docker).
  - Mots de passe : 15 caractères, 72 mots de passe compromis refusés (ceux de 15 caractères ou plus parmi les 100 000 de la liste xato de SecLists, licence MIT ; provenance dans `crates/portail/donnees/mots-de-passe-compromis-SOURCE.txt`). Un changement de mot de passe ferme toutes les sessions du compte et en ouvre une nouvelle. Une session dure 12 heures au plus.
  - Fichier de clé refusé sous Unix s'il est lisible par le groupe ou par tous ; texte de la clé effacé de la mémoire après lecture. `busy_timeout` de 5 secondes.
  - Images du portail et du relais : utilisateur 10001, sans binaire ni clé d'essai, avec `HEALTHCHECK`. Construites et essayées sur le Dell le 2026-11-19 par kadenz-cc, depuis 9510d00 :
    - portail (125 Mo) : seul binaire `kadenz-portail`, uid 10001 ; sans `--cle`, « clé du portail manquante » et code 1 ; clé d'essai en 644, refus qui demande `chmod 400` ; en 400, « pas une clé du portail reconnue » ;
    - relais (121 Mo) : démarre en lecture seule, `cap-drop ALL`, `no-new-privileges`, `/donnees` en 700 à l'uid 10001 ; `kadenz-relais sante` rend 0 et le `HEALTHCHECK` passe à « healthy ».
    - Reste l'essai d'un portail en marche (`portail_essai`, clé du banc) derrière Tailscale, pour relever l'adresse du mandataire vue par le conteneur.
- D2, sur les pages maud (livré le 2026-11-19) :
  - `GET /connexion` pose un jeton aléatoire dans le cookie `__Host-kadenz_connexion` (une heure) et dans le formulaire ; `POST /connexion` exige les deux, égaux (comparaison en temps constant), et refuse une demande marquée `Sec-Fetch-Site: cross-site` ou `same-site`. Un autre site ne peut donc pas connecter le navigateur à un compte de son choix. Le cookie de connexion est effacé à la connexion réussie.
  - Cookie de session renommé `__Host-kadenz_portail` : le navigateur ne l'accepte qu'avec `Secure`, `Path=/` et sans `Domain`, si bien qu'un sous-domaine ne peut ni le poser ni le remplacer. La durée absolue de 12 heures est tenue par le portail (D1) ; le cookie reste un cookie de session du navigateur.
  - Rotation : une connexion réussie ferme la session portée par le navigateur ; le changement de mot de passe ferme déjà toutes les sessions du compte (D1). Le jeton anti-falsification des formulaires appartient à la session et change avec elle.
  - CSP : `default-src 'self'; script-src 'none'; object-src 'none'; form-action 'self'; frame-ancestors 'none'; base-uri 'none'`.
- À garder : échappement maud, SQL paramétré, jeton anti-falsification en temps constant sur les autres `POST`, empreinte des codes, clé inconnue refusée.
