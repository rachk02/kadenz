# Plan du découpage des dépôts

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-19 | Premier jet de kadenz-3a à la demande de kadenz-cc. Rien n'est créé ni réécrit. |
| 0.2 | 2026-11-19 | Relecture de kadenz-cc. Les mises à jour des postes dépendent de la visibilité publique de Kadenz. Bilan de l'historique (pas de secret, des détails d'infrastructure au journal). Les essais croisés vont dans un crate hors de l'espace de travail. Rien n'est créé avant la réponse de l'utilisateur sur la visibilité. |

Statut : **proposé, relu par kadenz-cc, en attente de l'utilisateur**. Rien ne se crée, ne se réécrit ni ne se pousse avant sa réponse sur la visibilité de Kadenz (section 1), qui peut changer l'architecture. L'avis de kadenz-74 est demandé sur le relais, la construction sur le Dell et `AGENTS.md`.

## 1. Point de départ

- Décision de l'utilisateur du 2026-11-19, transmise par kadenz-cc. Le portail des licences reste en pages produites par le serveur (maud). Le « découpage en dépôt à part reste ». Les dépôts découpés vont sur le compte GitHub `rachk02`, tous privés, et leur création est autorisée.
- Fait relevé le 2026-11-19 par `gh repo view` : `rachk02/kadenz` est **public**. Le journal du 2026-11-09 le disait privé. Conséquence : le code du portail et tout son historique sont déjà publics dans Kadenz. Le découpage ne les retire pas de l'historique public ; seule une réécriture de l'historique de Kadenz le ferait, et ce plan ne la propose pas. **Question pour l'utilisateur** : Kadenz doit-il rester public ?
  - Conséquence relevée par kadenz-cc : `crates/maj/src/lib.rs`, ligne 49, lit les mises à jour sur `https://api.github.com/repos/rachk02/kadenz/releases`, sans identifiant. Passer Kadenz en privé couperait les mises à jour de tous les postes installés. Pour l'éviter, il faudrait d'abord publier une version qui lit ses mises à jour ailleurs, par exemple dans un dépôt public qui ne contient que les versions.
  - Bilan de l'historique par kadenz-cc : aucun mot de passe, aucun sujet ntfy, aucune adresse de veille, aucune clé privée. Mais le journal suivi (`.claude/journal.md`) contient des détails d'infrastructure : nom du réseau wifi, adresses locales et adresses Tailscale. Ils sont publics tant que Kadenz l'est.
  - Si Kadenz devient privé, le découpage lui-même est à reconsidérer : le portail pourrait rester dans Kadenz. Ce plan ne s'applique donc que si Kadenz reste public, ou si l'utilisateur maintient le découpage dans tous les cas.
- Liens actuels entre crates :
  - `kadenz-portail` dépend de `kadenz-licence` et de `kadenz-domaine` (`Module`, `MessageCompose`). `kadenz-domaine` n'a aucune dépendance, et `kadenz-licence` n'utilise pas rusqlite.
  - Deux essais du portail utilisent les crates de l'application, en dépendance d'essai : `licence_acceptee_par_un_serveur` (`kadenz-stockage`) et `client_d_activation_contre_le_portail` (`kadenz-maj`).
  - Le portail inclut à la compilation les polices `crates/app/ui/polices/{Inter-Regular,Inter-SemiBold,SpaceGrotesk-Bold}.ttf` et `docs/marque/web/favicon.svg`. Il inclut aussi `crates/portail/donnees/mots-de-passe-compromis.txt`, déjà dans le crate.
  - `kadenz-relais` est une dépendance normale de `kadenz-client`, `kadenz-poste` et `kadenz-serveur`.
  - Le correctif `[patch.crates-io] libsqlite3-sys = vendor/...` de la racine s'applique aujourd'hui au portail : sa base est compilée avec SQLite3 Multiple Ciphers, sans être chiffrée.

## 2. Dépôts

| Dépôt | Contenu | Statut |
|---|---|---|
| `rachk02/kadenz` (existant) | Application, serveur, poste, relais, licence, domaine, specs générales | Garde tout sauf le portail. |
| `rachk02/kadenz-portail` (nouveau, privé) | Le crate du portail, sa spec, son Dockerfile, ses polices et le favicon | Proposé. |
| Relais | Reste dans Kadenz | Proposé, en attente de kadenz-74. Le poste et le serveur l'appellent en bibliothèque : un dépôt à part imposerait une dépendance git croisée à chaque changement du protocole. Seule son image Docker se construit à part, comme aujourd'hui. |
| Site (Vercel) | Aucun code n'existe encore | Hors de ce découpage ; son dépôt privé se créera avec lui. |

## 3. Ce qui part et ce qui reste

Part dans `kadenz-portail`, avec son historique :

| Dans Kadenz | Dans kadenz-portail |
|---|---|
| `crates/portail/` (sources, essais, exemple, `donnees/`, `Dockerfile`) | racine du dépôt |
| `docs/spec-portail.md` | `docs/spec-portail.md` |
| `crates/app/ui/polices/Inter-Regular.ttf`, `Inter-SemiBold.ttf`, `SpaceGrotesk-Bold.ttf` | `ressources/polices/` |
| `crates/app/ui/polices/OFL-Inter.txt`, `OFL-SpaceGrotesk.txt` (la licence OFL doit accompagner les polices) | `ressources/polices/` |
| `docs/marque/web/favicon.svg` | `ressources/favicon.svg` |

Les polices et le favicon sont **copiés** : ils restent aussi dans Kadenz, où l'application et la marque s'en servent.

Reste dans Kadenz :
- `docs/spec-codes-licence.md`, qui couvre aussi `kadenz-licence` et l'activation côté application. Le dépôt du portail y renvoie.
- `docs/deploiement-vps.md`, qui décrit aussi le relais et Caddy. Sa ligne « construit depuis la racine du dépôt tant que le portail n'en est pas sorti » est mise à jour.
- `docs/spec-portail.md` devient une courte page de renvoi vers le nouveau dépôt, pour ne casser aucun lien des autres documents.

## 4. Crates communes

Proposé : `kadenz-portail` dépend de `kadenz-licence` et de `kadenz-domaine` **par dépendance git sur `rachk02/kadenz`, épinglée sur un commit précis** (`rev = "<empreinte>"`).

- Kadenz étant public, la construction n'a besoin d'aucun identifiant. Si Kadenz redevient privé, la construction sur le Dell demandera une clé de déploiement, que l'utilisateur décidera.
- Écarté : un dépôt commun pour `kadenz-licence`. Il faudrait y mettre aussi `kadenz-domaine`, qui est le domaine entier de l'application (calcul, comptabilité, stock), pour deux types.
- Changer la licence se fait en deux temps : un commit dans Kadenz, puis un commit dans le portail qui avance le `rev`. C'est voulu : le portail ne suit jamais une licence non relue.
- Le correctif de `libsqlite3-sys` ne s'applique qu'à la racine de Kadenz. Le dépôt du portail compilera rusqlite avec le SQLite ordinaire (`bundled`), ce qui suffit pour sa base non chiffrée. À vérifier pendant l'essai : une base `portail.sqlite` créée avant le découpage s'ouvre et passe `journal-verifier`.

## 5. Essais qui traversent la frontière

`licence_acceptee_par_un_serveur` et `client_d_activation_contre_le_portail` partent dans un **crate d'essai hors de l'espace de travail de Kadenz**, `essais/croises/`.
- Ce crate est ajouté à `exclude` à la racine de Kadenz. Il a son propre `Cargo.lock` et se lance à la main (`cargo test --manifest-path essais/croises/Cargo.toml`), avant un changement de la licence ou du portail.
- Dépendances : `kadenz-maj`, `kadenz-stockage`, `kadenz-licence` et `kadenz-domaine` par chemin ; `kadenz-portail` en dépendance git, épinglée par `rev`.

Raisons (relecture de kadenz-cc) :
- **Pas de dépendance au dépôt privé dans l'espace de travail.** Une dépendance git vers un dépôt privé, même d'essai, entre dans la résolution du `Cargo.lock` de tout l'espace de travail. Toute construction de Kadenz exigerait alors l'accès au dépôt privé : `release.ps1`, l'image du relais sur le Dell, un tiers qui clone. Hors de l'espace de travail, seul ce crate le demande.
- **Pas de `kadenz-stockage` dans le portail.** Le portail ne le prend pas en dépendance. Le crate d'essai étant sa propre racine, il porte son propre `[patch.crates-io] libsqlite3-sys = { path = "../../vendor/libsqlite3-sys" }` : `kadenz-stockage` y est compilé avec le chiffrement, comme dans Kadenz.
- **Pas de doublon de types.** `kadenz-portail` (git) tirerait sa propre copie de `kadenz-licence`, et un type de licence passé d'un côté à l'autre ne compilerait plus (deux copies, deux types). La racine du crate d'essai ajoute donc :
  ```toml
  [patch."https://github.com/rachk02/kadenz"]
  kadenz-licence = { path = "../../crates/licence" }
  kadenz-domaine = { path = "../../crates/domaine" }
  ```
  Ces essais vérifient alors le portail épinglé contre la licence du moment, ce qui détecte une rupture de compatibilité.
- Écarté : les porter dans `kadenz-portail` lui-même. Il faudrait y prendre `kadenz-stockage` par git, sans le correctif de chiffrement à sa racine.
- Écarté : remplacer ces essais par des essais de contrat de chaque côté. On perdrait l'essai de bout en bout de l'activation et de l'acceptation de la licence.

## 6. Cargo.lock et construction

- Dans la copie filtrée, on reprend le `Cargo.lock` actuel de Kadenz, puis on construit **sans** `--locked`. Cargo retire les paquets inutiles et garde les versions épinglées. On vérifie que la différence ne contient que des retraits, plus l'entrée git de `kadenz-licence` et de `kadenz-domaine`. Jamais de `cargo generate-lockfile`, qui ferait monter des versions.
- Nouveau `Cargo.toml` à la racine du portail, recopié de Kadenz :
  - `version = "1.0.0"`, `edition = "2024"`, `publish = false` ;
  - les versions et fonctions des dépendances héritées : rusqlite 0.41.1 avec `bundled` et `backup`, argon2 0.6.0, chrono 0.4.45 sans fonctions par défaut avec `clock` et `std`, ed25519-dalek 2.2.0, getrandom 0.3.4, sha2 0.10.9, serde_json 1.0.154 ;
  - les profils `[profile.dev.package."*"] opt-level = 3` et `[profile.release] lto = true, codegen-units = 1, strip = true`.
  - Aucune section `[workspace.lints]` ni aucun fichier `rust-toolchain` n'existe dans Kadenz : rien à recopier.
- Les `include_bytes!` passent à `ressources/...`, dans un commit ordinaire après le filtrage.
- Docker :
  - le `Dockerfile` arrive à la racine du portail ;
  - `COPY . .` et `cargo build --locked --release` restent ;
  - les images de base restent épinglées par empreinte ;
  - le portail reçoit son propre `.dockerignore` ;
  - l'étape de construction télécharge `kadenz-licence` depuis GitHub, sans identifiant tant que Kadenz est public.
- Construction sur le Dell, en attente de kadenz-74 et de l'utilisateur pour la façon de faire venir le code privé :
  - clone avec une clé de déploiement en lecture seule (décision de l'utilisateur) ;
  - ou archive `git archive` envoyée depuis le PC par Tailscale.
  - Critère : même essai qu'au F9c, image d'environ 125 Mo, sonde de santé verte, essai de connexion.
- Kadenz après le départ :
  - `scripts/release.ps1` lance `clippy --workspace` et `test --workspace`, qui ne couvriront plus le portail ;
  - le portail aura ses propres vérifications (fmt, clippy `--all-targets -D warnings`, test), lancées à la main avant chaque commit ;
  - l'intégration continue n'existe pas aujourd'hui et n'est pas ajoutée ici.

## 7. Historique

- Outil : `git filter-repo`, absent aujourd'hui. Il s'installe par `py -3.13 -m pip install git-filter-repo`, en version épinglée. C'est un outil nouveau sur le poste, signalé ici. `git subtree split` est écarté : il ne suit qu'un dossier et perdrait l'historique de la spec et des polices.
- Toujours sur un clone à part, dans le dossier temporaire : `git clone --no-local` du dépôt, **jamais dans l'arbre partagé**.
- Chemins gardés : `crates/portail/`, `docs/spec-portail.md`, les 5 fichiers de polices et `docs/marque/web/favicon.svg`. Ils sont renommés comme en section 3 par `--path-rename` (`crates/portail/:` vers la racine).
- L'historique de Kadenz **n'est jamais réécrit**. Le portail en sort par un seul commit ordinaire de retrait (section 9).
- Contrôles sur la copie filtrée, avant toute poussée :
  - recherche de secrets dans tout l'historique : `*.pem`, `*.key`, `kadenz-cle-privee.hex`, `essai-double-authentification.txt`, `*.sqlite`, et toute chaîne de 64 caractères hexadécimaux ou de base64 longue. La clé `cle_essai()` des essais, `[5u8; 32]`, est une clé d'essai connue et attendue ;
  - aucune ligne d'attribution d'outil dans les messages : `git log --format=%B | grep -ci "co-authored-by\|generated with"` = 0 ;
  - le nombre de commits gardés est comparé à `git log -- crates/portail` (16 aujourd'hui).

## 8. Fichiers de session et journal

- `.claude/CLAUDE.md`, `.claude/settings.json` et `scripts/sessions.ps1` ne sont jamais commités : ils restent ignorés par `.git/info/exclude` dans Kadenz, et le nouveau dépôt reçoit le même `.git/info/exclude`.
- `AGENTS.md` est suivi dans Kadenz : **question à kadenz-74**. Est-ce un fichier OpenCode à retirer, ou un fichier commun voulu ? Il ne part pas dans le portail tant que ce n'est pas tranché.
- Journal : celui de Kadenz reste l'historique du projet et n'est pas copié. Le nouveau dépôt ouvre son propre `.claude/journal.md`, dont la première entrée renvoie au commit de départ de Kadenz.

## 9. Ordre des opérations

1. Relecture de ce plan par kadenz-cc (faite, 0.2), réponses de kadenz-74, et **réponse de l'utilisateur sur la visibilité de Kadenz**. Rien ne commence avant cette réponse.
2. Installation de `git-filter-repo`, clone à part, filtrage, nouveau `Cargo.toml`, `.dockerignore`, chemins des `include_bytes!`, `Cargo.lock` élagué. Rien n'est poussé.
3. Tour de compilation demandé à kadenz-cc. Sur la copie : fmt, clippy `--all-targets -D warnings`, essais (les deux essais de la section 5 en moins), construction de `portail_essai`.
4. Recherche de secrets et contrôles de la section 7.
5. Création du dépôt privé `rachk02/kadenz-portail` et première poussée. La création est autorisée par l'utilisateur (2026-11-19) ; je la fais seulement après le feu de kadenz-cc, et je la signale dans le journal.
6. Construction de l'image sur le Dell depuis le nouveau dépôt et comparaison avec l'image actuelle, avec kadenz-74.
7. Seulement ensuite, dans Kadenz :
   - une étiquette `avant-decoupage-portail` sur le commit du moment ;
   - puis un seul commit de retrait, coordonné avec kadenz-74 parce qu'il touche le `Cargo.toml` et le `Cargo.lock` communs :
     - membre `crates/portail` et ligne `kadenz-portail` retirés ;
     - `crates/portail/` supprimé ;
     - exception du favicon retirée du `.dockerignore` ;
     - crate `essais/croises/` ajouté et mis dans `exclude` (section 5) ;
     - `docs/spec-portail.md` en page de renvoi ;
     - ligne de `deploiement-vps.md` mise à jour ;
     - `Cargo.lock` mis à jour et indexé par moi.
   - Vérification du commit de retrait : `clippy --workspace --all-targets` et `test --workspace`, en un tour.

## 10. Retour arrière

- Avant l'étape 7 : rien n'a changé dans Kadenz. On abandonne la copie, et au besoin on supprime le nouveau dépôt, avec l'accord de l'utilisateur.
- Après l'étape 7 : `git revert` du commit de retrait remet le portail dans Kadenz, et l'étiquette `avant-decoupage-portail` donne l'état exact. Le Dell reconstruit depuis Kadenz comme au F9c. Le dépôt `kadenz-portail` reste, inutilisé, jusqu'à décision de l'utilisateur.
- Les modifications faites entre-temps dans `kadenz-portail` se reportent à la main, avec `git format-patch` puis `git am` et `--directory=crates/portail`.

## 11. Questions ouvertes

1. Utilisateur : Kadenz doit-il rester public (section 1) ? S'il devient privé : publier d'abord une version qui lit ses mises à jour ailleurs, et décider si le portail reste dans Kadenz.
2. Utilisateur : comment le Dell obtient-il le code privé (clé de déploiement ou archive envoyée) ?
3. kadenz-74 : relais dans Kadenz (proposé) ?
4. kadenz-74 : `AGENTS.md` ?
5. Tranché par kadenz-cc (0.2) : essais croisés dans un crate hors de l'espace de travail (section 5).
