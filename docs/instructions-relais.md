# Relais : ce qu'il reste à faire sur le Dell et chez Cloudflare

Rédigé le 2026-11-25. Rien de ceci n'a été appliqué en production : ce sont des actions
sur le Dell (Docker) et dans ton compte Cloudflare, donc à toi, ou à moi avec ton accord
explicite.

## Où en est le problème

Le relais et le portail sont sains sur le Dell (aucun redémarrage de conteneur). Les
pannes viennent du tunnel cloudflared : délais dépassés du transport QUIC les 23 et 24
novembre, et coupures du Dell lui-même. Deux actions, indépendantes.

## Action 1 : passer cloudflared en HTTP/2 (faite le 2026-11-25)

Appliquée sur le Dell le 2026-11-25 avec l'accord de l'utilisateur : tunnel recréé seul, 4 connexions en http2, portail en 303, relais en 101. Retour arrière : copier `/opt/kadenz/source/deploiement/dell/compose.public.yaml.avant-http2` sur `compose.public.yaml`, puis `kadenz-compose up -d tunnel`. Les étapes suivantes sont gardées pour mémoire.

Le fichier `deploiement/dell/compose.public.yaml` du dépôt est déjà corrigé (options
`--protocol http2` du service `tunnel`). Pour l'appliquer sur le Dell :

1. Se connecter : `ssh dell`.
2. Mettre à jour la source du dépôt sur le Dell, comme pour les déploiements précédents
   (procédure de `docs/deploiement-vps.md`), puis relancer seulement le tunnel :
   `kadenz-compose up -d tunnel` (le script `kadenz-compose` du dossier `deploiement/dell`
   reprend les fichiers de base et la surcouche publique).
3. Vérifier : `docker logs --tail 20 kadenz-tunnel-1` doit montrer des connexions
   enregistrées avec `protocol=http2`, puis `kadenz-sante` doit répondre (portail en 303,
   relais en 101).

Coupure attendue : quelques secondes, le temps du redémarrage du tunnel.

## Action 2 : secret de la passerelle `--secret-ws` (faite le 2026-11-25)

Active sur le Dell le 2026-11-25 avec l'accord de l'utilisateur. Il a fallu reconstruire l'image du relais (l'ancienne ne connaissait pas l'option) : `kadenz-relais:43a411971ab1`, révision propre au relais (`KADENZ_REV_RELAIS`), portail inchangé. Vérifié : accès public en 101 par Cloudflare, 403 sans le secret depuis un pair non local. Retour arrière : copier les fichiers `.avant-secret` de `/opt/kadenz/source/deploiement/dell` et `/etc/kadenz/kadenz.env`, retirer `KADENZ_REV_RELAIS`, `kadenz-compose up -d relais`. Les étapes suivantes sont gardées pour mémoire.

Elle empêche de joindre le relais autrement que par Cloudflare. **L'ordre compte** : une fois
l'option posée, une règle Cloudflare absente ou fausse coupe toutes les sessions distantes.

1. Dans Cloudflare : Rules > Overview > Create rule > Request Header Transform Rule, nom
   `Kadenz relais secret`, expression `http.host eq "relais.mcarre.dev"`, Modify request
   header, Set static, nom `x-kadenz-secret`, valeur un secret choisi (long, aléatoire),
   Deploy. Détail et sources : `docs/deploiement-vps.md`.
2. Vérifier qu'une ouverture WebSocket aboutit encore.
3. Déposer le même secret sur le Dell dans un fichier (hors dépôt, mode 400), ajouter les
   deux éléments décrits en commentaire dans `compose.public.yaml` (service `relais`),
   relancer le relais.

Si quelque chose casse : retirer les deux éléments et relancer le relais, tout revient.

## Ce que je peux faire pour toi

Sur ton feu vert explicite, je peux faire l'Action 1 moi-même par `ssh dell` et vérifier le
résultat. L'Action 2 demande de toute façon ton compte Cloudflare (étape 1) et le choix du
secret : je te guide, tu cliques et tu choisis le secret.
