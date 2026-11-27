# Kadenz - Téléchargements, mises à jour et essai gratuit (spécification)

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-26 | Premier jet par kadenz-cc, d'après les décisions de l'utilisateur du 2026-11-26 (décisions 77 à 79 du cahier de la phase 2). Rien n'est encore codé. |
| 0.2 | 2026-11-26 | Lot T1 livré : module `telechargement` du portail (noms de paquets, dernier paquet par édition, liens HMAC de dix minutes, plages), routes `/telecharger/{produit}` et `/fichiers/{nom}`, option `--depot`, frein `TELECHARGEMENT` (vingt liens par adresse, puis jusqu'à 15 minutes), journal `telechargement`. Dépôt à plat. Clé des liens tirée au démarrage. |
| 0.3 | 2026-11-26 | Lot E1 livré : attestation d'essai signée `KADENZ-T-` (`kadenz_licence::essai`), table `essai` du portail (schéma 13, un essai par poste et par édition), route `POST /api/essai` (adresse email vérifiée, frein par adresse, autre adresse pour un même poste notée au journal `essai_autre_adresse`). |
| 0.4 | 2026-11-26 | Lot T2 livré : route `GET /api/releases` (versions publiées du dépôt au format de la liste des releases de GitHub, liens signés absolus), dépôt à plat étendu aux paquets MSI, au manifeste `kadenz-maj-X.Y.Z.json`, à sa signature et aux notes `notes-X.Y.Z.md` ; client de mise à jour qui lit le portail d'abord, GitHub si le portail ne répond pas ou n'a rien publié. |
| 0.5 | 2026-11-26 | Lot E2 livré : adresse email à l'étape Licence de l'assistant (serveur et monoposte, sans code), essai noté « à confirmer » dans la base du serveur, demande au portail hors du fil de l'interface à l'installation puis à chaque lancement tant qu'il n'est pas confirmé, attestation gardée dans `%ProgramData%\Kadenz\essai-<édition>.txt`, lecture seule après sept jours sans confirmation (raison affichée), règle d'adresse partagée (`kadenz_licence::essai::valider_email`). Les essais commencés avant cette règle ne sont pas concernés. |
| 0.6 | 2026-11-26 | Décision de l'utilisateur du 2026-11-26 : ni email ni connexion exigés. L'essai se reconnaît par l'identifiant du poste seul, envoyé sans rien demander ; sans réponse du portail, l'essai continue normalement et la demande repart à chaque lancement (plus de lecture seule après sept jours). Colonne `email` retirée de la table `essai` (schéma 13, jamais déployé). |
| 0.7 | 2026-11-27 | Release 0.3.4 servie par le portail du Dell (`/srv/kadenz/depot`). Lot E3 livré : page publique `/confidentialite` du portail (pas encore de site public), qui décrit d'après le code ce que Kadenz et le portail envoient et gardent (essai, activation, téléchargement, frein, journal, hébergement, cookies) ; lien depuis la page de téléchargement et le LISEZMOI. |

Statut : **spécifié**. Les lots sont au tableau de la section 7.

## 1. Décisions de l'utilisateur (2026-11-26)

- **77. Téléchargements sans exposer les dépôts.** Les installateurs et les mises à jour ne se prennent plus sur un dépôt public : le portail donne un lien signé et de courte durée, et sert le fichier depuis le Dell. Le téléchargement reste simple : aucun code ni compte pour télécharger, une page « Votre téléchargement va démarrer » avec un bouton de secours, comme sur les autres sites. Pas de lien à usage unique ni lié à une adresse : ce serait trop strict.
- **78. Essai gratuit connu du portail.** L'essai de 30 jours (décision du 2026-11-25) est connu du portail par l'identifiant du poste, envoyé sans rien demander à l'utilisateur : ni email ni connexion obligatoire (décision de l'utilisateur du 2026-11-26, qui remplace la première version avec email). Une réinstallation sur le même poste retrouve la date d'origine.
- **79. Rien de caché sur le poste du client.** L'essai se protège par le portail, pas par des traces dissimulées dans le registre ou des services déguisés : ces procédés font classer un logiciel comme indésirable par les antivirus et trahissent la confiance du client. Tout ce que Kadenz pose sur le poste porte son nom et figure dans le LISEZMOI.

## 2. Ce qui existe aujourd'hui

- Mises à jour et installateurs : releases du dépôt GitHub public `rachk02/kadenz-releases` ; le client lit `api.github.com` (`crates/maj/src/lib.rs`, constante `RELEASES`). Le manifeste est signé et le client vérifie la signature et l'empreinte du paquet : un fichier falsifié est refusé, mais le dépôt est ouvert à tous.
- Essai : la date de début est gardée dans la base du serveur (`abonnement.essai_debut`, `crates/stockage/src/abonnement.rs`). En monoposte, la base est dans `%LOCALAPPDATA%\Kadenz`, que la désinstallation supprime (`installateur/Kadenz.wxs`, `RemoveFolderEx`) : désinstaller puis réinstaller redonne 30 jours.
- Identifiant du poste : `kadenz_licence::identifiant_poste`, empreinte du `MachineGuid` de Windows, déjà utilisée par les licences de la ligne V1. Le `MachineGuid` lui-même ne quitte jamais le poste.

## 3. Téléchargement d'un installateur

Parcours :

1. Le site public (ou le portail en attendant le site) affiche « Télécharger Kadenz » et « Télécharger Kadenz Entreprise ».
2. Le bouton mène à `/telecharger/<produit>` : page « Votre téléchargement va démarrer dans quelques secondes. S'il ne démarre pas, cliquez ici. » La page lance le téléchargement toute seule (rafraîchissement ou script court) ; le bouton porte le même lien.
3. Le lien est `/fichiers/<nom>?exp=<horodatage>&sig=<signature>` : HMAC-SHA256 du nom et de l'échéance, clé tenue par le portail, échéance de 10 minutes. Le lien peut servir plusieurs fois dans ce délai (reprise d'un téléchargement coupé).
4. Le portail vérifie la signature et l'échéance, puis sert le fichier en flux, avec `Content-Length`, `Content-Disposition: attachment`, et la reprise (`Range`).

Garde-fous :

- Frein par adresse, comme celui des connexions et des activations (`frein.rs`) : au-delà d'un nombre de liens par heure, attente croissante. Pas de blocage par adresse unique.
- Journal du portail : une entrée par lien donné (produit, version, adresse), sans donnée personnelle de plus.
- Noms de fichiers d'une liste fermée (les paquets publiés) : aucun chemin pris dans la requête.

## 4. Mises à jour

- Le client demande `GET /api/releases` au portail : la liste des versions publiées, au format de la liste des releases de GitHub qu'il lit déjà (`kadenz_maj::choisir`). Chaque fichier (installateurs, manifeste signé `kadenz-maj.json`, sa signature) y figure avec un lien signé absolu, au format de la section 3. Le choix de la version, du produit et de la génération reste celui du client. Le portail a besoin de `--depot` et de `--adresse-publique`.
- La signature du manifeste reste la garantie : un portail compromis ne peut pas faire installer un paquet non signé.
- Repli : tant que des postes en 0.3.3 ou avant existent, leurs releases GitHub restent en place (ces versions ne connaissent que GitHub). Les nouvelles versions publient sur le Dell ; GitHub ne garde que ce qui sert aux anciens postes, puis passe en privé quand plus aucun poste ne l'appelle (date à fixer avec l'utilisateur).
- Le client essaie le portail d'abord, GitHub ensuite, pendant la transition seulement.

## 5. Stockage des fichiers sur le Dell

- Dossier hors du service web, à plat (`KadenzSetup-X.Y.Z.exe`, `KadenzMonoposteSetup-X.Y.Z.exe`), passé au portail par `--depot` ; emplacement sur le Dell à choisir à la mise en place (lot T3). Lecture seule pour le compte du portail, écriture par la publication seule.
- Publication : `scripts/release.ps1` copie les paquets et le manifeste signé vers le Dell (par Tailscale), en plus de la release GitHub pendant la transition. La copie vérifie l'empreinte à l'arrivée.
- Sauvegarde : le dossier entre dans la sauvegarde restic existante (OneDrive).
- Taille : un installateur fait environ 100 Mo. Le fichier passe par le tunnel Cloudflare du portail : les limites et conditions du plan gratuit pour des réponses de cette taille sont à vérifier dans la documentation de Cloudflare, et un essai de bout en bout avec le vrai paquet est obligatoire avant la bascule.

## 6. Essai gratuit

### 6.1 Démarrage

- À l'installation sans code, l'assistant ne demande rien de plus : l'essai démarre aussitôt, avec la date du jour, marqué « à confirmer ».
- Le programme envoie au portail `POST /api/essai`, hors du fil de l'interface : produit, identifiant du poste (`identifiant_poste`, empreinte du `MachineGuid`), version. Aucune donnée personnelle. Le portail répond avec la date de début de l'essai, signée par sa clé.
- Règle du portail :
  - poste inconnu : l'essai démarre aujourd'hui ;
  - poste déjà connu pour cette édition : le portail renvoie la date de début d'origine ; une réinstallation ne relance donc pas l'essai.
- À la réponse, la date du portail remplace celle du poste si elle est plus ancienne, et l'essai n'est plus « à confirmer ».

### 6.2 Sans Internet

- Aucune connexion n'est exigée : sans réponse du portail, l'essai continue normalement. Le programme renvoie la demande à chaque lancement tant qu'elle n'est pas confirmée ; elle aboutit dès que le poste est connecté.

### 6.3 Marque d'essai sur le poste

- La date signée reçue du portail est aussi gardée dans `%ProgramData%\Kadenz\essai-<édition>.txt`, fichier nommé, documenté dans le LISEZMOI, que la désinstallation ne supprime pas. Elle évite un appel au portail à chaque réinstallation et sert hors ligne.
- Le fichier n'est qu'un cache : effacé, il se reconstitue depuis le portail ; modifié, sa signature est refusée.

### 6.4 Ce qui reste accepté

Changer de machine redonne un essai, de même qu'un poste qui ne se connecte jamais pendant ses 30 jours. C'est accepté (décision 79) : le recoupement du portail suffit contre la réinstallation, et la désinstallation efface de toute façon les données saisies.

## 7. Lots

| Lot | Contenu | Où |
|---|---|---|
| T1 (livré) | Liens signés et route `/fichiers/...` (HMAC, échéance, liste fermée, `Range`), page `/telecharger/<produit>`, frein et journal. Tests. | `crates/portail` |
| T2 (livré) | Route `/api/releases` (manifeste et lien signé) ; client qui essaie le portail puis GitHub. Tests. | `crates/portail`, `crates/maj` |
| T3 | Publication vers le Dell dans `release.ps1`, dossier, droits, sauvegarde. Demande l'accord de l'utilisateur (serveur de production). | `scripts/`, Dell |
| E1 (livré) | Table des essais du portail (poste, produit, début), route `/api/essai`, date signée, journal. Tests. | `crates/portail`, `crates/licence` |
| E2 (livré) | Appel au portail sans rien demander, date « à confirmer » tant que le portail n'a pas répondu, cache `%ProgramData%\Kadenz\essai-<édition>.txt`. | `crates/app`, `crates/stockage`, installateur |
| E3 (livré) | Page de confidentialité : usage de l'identifiant du poste. Sur le portail, `/confidentialite`, tant que le site public n'existe pas ; à reprendre sur le site. | `crates/portail` |

Ordre proposé : T1, E1, T2, E2, puis T3 et E3 avec l'utilisateur. Les lots T3 et l'essai sur le portail de production demandent son accord.
