# Kadenz phase 2 - Portail des licences (jalon F9)

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-15 | Forme proposée par kadenz-34 et kadenz-8e après les décisions 60 et 61 de l'utilisateur : programme, données, rôles, émission, journal, sécurité, essais. Rien n'est codé. |

Statut : **proposé**. Les choix marqués « proposé » restent à confirmer par l'utilisateur ; les décisions citées sont celles du cahier de la phase 2.

## 1. Périmètre (décisions 21, 27, 29, 60)

- Le portail remplace l'outil manuel `kadenz-editeur` pour les licences d'abonnement d'entreprise (format du jalon F3, `KADENZ-E-...`). Les licences V1 perpétuelles n'y passent pas.
- F9 est local et sans carte (décision 60) : l'éditeur ou un revendeur enregistre un paiement reçu (virement, chèque ou espèces) avec son montant et le nombre de postes payés, puis émet la licence. Aucune grille de prix ni aucun nombre de postes inclus n'est écrit dans le portail : ils viendront de l'utilisateur, avec le paiement par carte et son prestataire.
- Mise en service sur le VPS : plus tard, à la décision de l'utilisateur, comme le relais.

## 2. Programme (décision 61)

- Un seul programme Rust, `kadenz-portail` (crate `crates/portail`), qui sert des pages HTML simples rendues sur le serveur : formulaires, aucun framework JavaScript. Bibliothèques : axum et tokio pour le HTTP, maud pour les pages (gabarits vérifiés à la compilation, échappement automatique), argon2 pour les mots de passe (choix de kadenz-34 et kadenz-8e).
- Il dépend de `kadenz-licence` (et de `kadenz-domaine` pour les modules), jamais de `kadenz-stockage` : le chiffrement de la base et des clés de `kadenz-stockage` repose sur la protection des données de Windows, et le portail tourne sous Linux. Il se construira dans un conteneur comme le relais ; cette construction sous Linux n'est pas encore essayée (SQLite de l'espace de travail remplacé par SQLite3 Multiple Ciphers, favicon repris de `docs/` malgré le `.dockerignore`), elle fait partie de F9c.
- Lancement : `kadenz-portail --dossier <dossier> --cle <fichier> [--ecoute 127.0.0.1:47382]`. Le fichier de clé est la clé privée du portail en hexadécimal, au format de `kadenz-editeur`. Le programme refuse de démarrer sans `--cle` ; il ne prend jamais une clé d'essai à la place.
- `kadenz-portail --dossier <dossier> creer-editeur <identifiant>` crée un compte éditeur, mot de passe lu sur l'entrée standard. C'est la seule façon de créer un compte éditeur.
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

## 6. Sécurité

- Mots de passe : argon2id, 10 caractères au moins (proposé).
- Session : jeton aléatoire de 32 octets, dans un cookie de session du navigateur `HttpOnly`, `Secure`, `SameSite=Strict`, gardé côté portail ; fin après 8 heures sans activité (proposé), à la déconnexion et à l'arrêt du portail.
- Chaque formulaire porte un jeton anti-falsification de la session, vérifié avant toute écriture.
- Connexions freinées : après 5 échecs par identifiant ou par adresse, attente de 5 secondes doublée à chaque échec jusqu'à 15 minutes, comme les sessions du serveur ; chaque échec est noté au journal.
- En-têtes : `Content-Security-Policy: default-src 'self'; form-action 'self'; frame-ancestors 'none'`, `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`, `X-Robots-Tag: noindex`.
- Console privée : `noindex` sur chaque page et `robots.txt` qui interdit tout (hors de la liste SEO des sites publics).

## 7. Découpage et essai

- **F9a** (kadenz-34) : crate `kadenz-portail` (données, rôles, émission, journal, sessions, freinage), table des routes axum, programme et exemple `portail_essai` ; essais en processus, dont un revendeur refusé sur le client d'un autre revendeur et une licence émise acceptée par `abonnement::enregistrer_avec`.
- **F9b** (kadenz-8e) : pages dans `crates/portail/src/pages/` (un module par page), signatures fixées par F9a ; style des écrans (angles de 4 px au plus, palette et police de la marque, aucune italique), textes en français.
- **F9c** : construction de l'image du portail sous Linux (conteneur), puis essai à l'écran, portail d'essai lancé en local : l'éditeur crée un revendeur ; le revendeur crée une entreprise (identifiant d'un banc d'essai), enregistre un paiement et émet la licence ; la licence est acceptée par le banc d'essai depuis la rubrique « Abonnement », émetteur visible ; journal complet.
