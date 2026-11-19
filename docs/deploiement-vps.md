# Kadenz phase 2 - Déploiement sur le VPS (proposition)

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-18 | Brouillon de kadenz-74 à la demande de kadenz-cc : Caddyfile, compose.yaml, pare-feu et sauvegardes proposés, d'après la réponse 4 d'OpenCode (`docs/securite/echanges-opencode.md`), la section 7 de `docs/spec-synchronisation.md` et les Dockerfile du dépôt. Rien n'est en service. |
| 0.2 | 2026-11-18 | Relecture d'OpenCode (réponse 6) intégrée : filtrage par la chaîne DOCKER-USER, Caddy sans root sur les ports internes 8080 et 8443, apex, journaux d'accès, rotation des journaux Docker, healthchecks, ulimits et pids du relais, pages d'erreur par code, pièges de la sauvegarde SQLite, défauts du relais à corriger dans le code. |
| 0.3 | 2026-11-18 | Lot R1 du relais écrit (pas encore compilé ni commité) : les quatre défauts du relais passent en « corrigé dans R1 » ; sonde de santé `kadenz-relais sante` ; propriétaire de `/donnees` ; adresses vues par le relais derrière docker-proxy, à vérifier avant la mise en ligne. |
| 0.4 | 2026-11-19 | Section 7 : variante sur le Dell avec Tailscale, sans nom de domaine (décision de l'utilisateur du 2026-11-19). Funnel vérifié dans sa documentation : le portail y passe, le relais non dans son protocole actuel ; trois pistes pour le relais, à trancher. |

Statut : **proposé**. Le VPS, son prestataire, le nom de domaine et la mise en service restent à la décision de l'utilisateur (décisions 16 et 69 du cahier de la phase 2). Le relais et le portail y iront une fois leur implémentation finie (décision de l'utilisateur du 2026-11-18). Pour l'instant, ni domaine ni VPS : hébergement sur le Dell avec Tailscale, et Vercel au besoin (décision du 2026-11-19, section 7). Rien ici n'a été essayé sur un VPS ; chaque fichier est à relire et à essayer d'abord sur une machine d'essai.

## 1. Ce qui tourne sur le VPS

| Service | Programme | Port vu d'Internet | Données |
|---|---|---|---|
| Entrée HTTPS | Caddy (image officielle) | 80 et 443 (TCP), 443 (UDP, HTTP/3) | certificats dans le volume `caddy-donnees` |
| Console des licences | `kadenz-portail` (`crates/portail`) | aucun : joint par Caddy seulement | `portail.sqlite` dans le volume `portail-donnees` |
| Site public | fichiers statiques (Zola, piste A de `docs/spec-site.md`, pas encore codé) | aucun : servis par Caddy | dossier en lecture seule |
| Relais d'accès à distance | `kadenz-relais` (`crates/relais`) | 47381 (TCP) | identité TLS et traces dans `relais-donnees` |

- Le relais termine lui-même son TLS 1.3 et les postes épinglent son empreinte (`docs/spec-synchronisation.md`, section 7) : Caddy ne peut pas le précéder, il est publié directement. Le port 47381 est celui par défaut ; le port définitif « reste à décider » avec la mise en service (même section).
- Le site public est servi directement par Caddy depuis un volume en lecture seule, ce qui évite un conteneur de plus (réponse 4 d'OpenCode). Un conteneur à part reste possible si l'utilisateur le préfère (section 3, variante).
- Deux noms d'hôte, sur la recommandation d'OpenCode (réponse 3) : un pour le site, un pour la console. Le cookie de session de la console ne part ainsi jamais vers le site. Ce choix, comme la forme du front de la console, est porté à l'utilisateur.

Paramètres, à remplir par l'utilisateur, jamais inventés :

| Paramètre | Rôle |
|---|---|
| `DOMAINE_SITE` | nom d'hôte du site public |
| `DOMAINE_CONSOLE` | nom d'hôte de la console des licences |
| `DOMAINE_APEX` | domaine nu, redirigé vers le site (bloc à retirer si le DNS ne le publie pas) |
| `EMAIL_ACME` | adresse donnée à l'autorité de certification (facultative) |
| `PORT_RELAIS` | port public du relais, 47381 par défaut |

## 2. Caddyfile proposé

Les valeurs `{$NOM}` sont lues dans l'environnement du conteneur Caddy (fichier `.env` à côté du compose, hors dépôt).

```caddyfile
{
	email {$EMAIL_ACME}
	# API d'administration coupée : la configuration se recharge par un redémarrage.
	admin off
	# Ports internes : Caddy tourne sans root (section 3) et ne peut pas lier 80 et 443.
	# Ces réglages ne changent pas les ports vus des clients : l'hôte publie 80 vers 8080
	# et 443 vers 8443.
	http_port 8080
	https_port 8443
	servers {
		max_header_size 16KB
		timeouts {
			read_header 10s
			read_body 30s
			write 30s
			idle 120s
		}
	}
}

# En-têtes communs aux deux hôtes.
(entetes) {
	header {
		# HSTS progressif : un jour au début, puis deux ans une fois le domaine
		# stable (max-age=63072000; includeSubDomains). Pas de preload sans décision.
		Strict-Transport-Security "max-age=86400"
		X-Content-Type-Options "nosniff"
		Referrer-Policy "no-referrer"
		Permissions-Policy "camera=(), microphone=(), geolocation=(), payment=()"
		Cross-Origin-Opener-Policy "same-origin"
		Cross-Origin-Resource-Policy "same-origin"
		X-Frame-Options "DENY"
		-Server
	}
	# Journal d'accès en JSON sur la sortie (rotation par Docker, section 3), pour
	# fail2ban et l'analyse d'un incident ; log_credentials reste désactivé.
	log {
		output stdout
		format json
	}
}

# Apex : servi seulement pour rediriger vers le site, avec son propre certificat.
# Si le DNS ne publie pas l'apex, supprimer ce bloc.
{$DOMAINE_APEX} {
	import entetes
	redir https://{$DOMAINE_SITE}{uri} permanent
}

{$DOMAINE_SITE} {
	import entetes
	header Content-Security-Policy "default-src 'none'; style-src 'self'; img-src 'self'; font-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'"
	request_body {
		max_size 64KB
	}
	root * /srv/site
	# Cache : no-cache par défaut ; les fichiers à empreinte recevront
	# « public, max-age=31536000, immutable » quand leur nommage par Zola sera connu (W1).
	header ?Cache-Control "no-cache"
	file_server
	# Page d'erreur selon le code (404.html, 500.html...), repli sur erreur.html :
	# une panne ne s'annonce pas comme une page introuvable.
	handle_errors {
		try_files /{err.status_code}.html /erreur.html
		file_server
	}
}

{$DOMAINE_CONSOLE} {
	import entetes
	# La CSP de la console est posée par le portail lui-même (crates/portail/src/web.rs) :
	# une seule source, deux CSP s'appliqueraient en intersection.
	request_body {
		max_size 1MB
	}
	reverse_proxy portail:47382
}
```

- TLS : Caddy obtient et renouvelle les certificats et redirige HTTP vers HTTPS par défaut ; TLS 1.2 et 1.3 par défaut, à garder tant que des postes Windows anciens ou des mandataires d'entreprise sont dans le public (jugement d'OpenCode, réponse 6). TLS 1.3 seul reste possible (`tls { protocols tls1.3 }`).
- HSTS : montée par paliers, en attendant la durée complète de chaque palier avant de passer au suivant : 86400, puis 604800, 2592000 et enfin 63072000 secondes, avec `includeSubDomains` (paliers de hstspreload.org). La valeur 0 est le seul moyen de retirer la politique. `includeSubDomains` sur www ne couvre pas la console, qui est un hôte frère : chacun reçoit sa politique par `import entetes`. L'apex, s'il envoyait `includeSubDomains`, couvrirait les deux. Pas de préinscription (preload) sans décision : hstspreload.org elle-même ne la recommande pas. Vérifier après la mise en service que l'en-tête est aussi sur les erreurs et les redirections (`curl -I` sur une 404).
- CSP du site : `style-src 'self'` interdit aussi les attributs `style=` et les blocs `<style>`. À contrôler sur le HTML produit par Zola au jalon W1 (coloration syntaxique notamment) ; le cas échéant, sortir les styles dans une feuille plutôt qu'ouvrir `'unsafe-inline'`. `Permissions-Policy payment=()` serait à revoir si une page de paiement arrivait sur ce domaine.
- `request_body` est une directive expérimentale de Caddy (réponse 4 d'OpenCode) : le portail doit garder sa propre limite de taille (défaut 3 de la réponse 5).
- Caddy n'a pas de limite de débit native ; le frein du portail reste la barrière (défauts 3 à 5 de la réponse 5, à corriger avant la mise en ligne).
- Formulaire de contact et espace client : routes du portail à venir (`docs/spec-site.md`, sections 5 et 6) ; on ajoutera alors un `handle /contact* { reverse_proxy portail:47382 }` sur l'hôte du site.

## 3. compose.yaml proposé

Construit depuis la racine du dépôt tant que le portail n'en est pas sorti (les Dockerfile l'exigent). Les images de base sont déjà épinglées par empreinte dans les Dockerfile ; celle de Caddy doit l'être aussi, c'est le conteneur qui termine TLS et voit tout le trafic.

```yaml
name: kadenz

# Rotation des journaux : sans elle, le pilote json-file ne limite rien et un disque
# plein arrête le portail et le relais.
x-journaux: &journaux
  logging:
    driver: json-file
    options:
      max-size: "10m"
      max-file: "3"

services:
  caddy:
    <<: *journaux
    image: caddy:2@sha256:<empreinte à relever à la mise en service>
    restart: unless-stopped
    # Sans root : l'image officielle tourne en root par défaut ; ses dossiers /data/caddy
    # et /config/caddy sont en 1777, l'utilisateur nobody peut y écrire. Caddy écoute
    # en interne sur 8080 et 8443 (http_port et https_port du Caddyfile).
    user: "65534:65534"
    ports:
      - "80:8080"
      - "443:8443"
      - "443:8443/udp"
    env_file: .env
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile:ro
      - ./site:/srv/site:ro
      - caddy-donnees:/data
      - caddy-config:/config
    read_only: true
    tmpfs:
      - /tmp:rw,noexec,nosuid,size=16m
    # Aucune capacité, aucun cap_add : un processus non root sur des ports hauts n'en a
    # pas besoin.
    cap_drop: [ALL]
    security_opt: [no-new-privileges:true]
    pids_limit: 256
    mem_limit: 256m
    networks: [entree, console]

  portail:
    <<: *journaux
    build:
      context: .
      dockerfile: crates/portail/Dockerfile
    restart: unless-stopped
    # Le portail ne traite pas SIGTERM (crates/portail/Dockerfile) : init obligatoire.
    init: true
    command: ["--cle", "/run/secrets/cle_portail", "--mandataire"]
    secrets: [cle_portail]
    volumes:
      - portail-donnees:/donnees
    read_only: true
    tmpfs:
      - /tmp:rw,noexec,nosuid,size=64m
    cap_drop: [ALL]
    security_opt: [no-new-privileges:true]
    pids_limit: 256
    mem_limit: 512m
    cpus: 1
    # Aucun port publié : seul Caddy, sur le réseau « console », le joint.
    networks: [console]

  relais:
    <<: *journaux
    build:
      context: .
      dockerfile: crates/relais/Dockerfile
    restart: unless-stopped
    init: true
    ports:
      - "${PORT_RELAIS:-47381}:47381"
    volumes:
      - relais-donnees:/donnees
    read_only: true
    tmpfs:
      - /tmp:rw,noexec,nosuid,size=16m
    cap_drop: [ALL]
    security_opt: [no-new-privileges:true]
    # Un fil par connexion et un second par session (crates/relais/src/relais.rs) : le
    # cgroup compte les fils comme des PID. Avec le plafond de 4096 connexions, environ
    # 6100 fils à saturation : 256 bloquerait le relais dès la 256e connexion.
    pids_limit: 6400
    ulimits:
      nofile:
        soft: 16384
        hard: 16384
    # Valeur plausible, non mesurée : à trancher par un essai de charge à 1000 sessions.
    mem_limit: 512m
    cpus: 1
    networks: [relais]

networks:
  entree: {}
  console:
    internal: true
  # Le relais n'a besoin d'aucun accès sortant ; internal: true est à essayer, en
  # vérifiant que la publication du port fonctionne toujours.
  relais: {}

volumes:
  caddy-donnees: {}
  caddy-config: {}
  portail-donnees: {}
  relais-donnees: {}

secrets:
  cle_portail:
    file: ./secrets/cle-portail.hex
```

- `ENTRYPOINT` des Dockerfile : `--dossier /donnees --ecoute 0.0.0.0:...` ; `command` ne fait qu'ajouter les options. Le portail refuse de démarrer sans `--cle`, et refuse une clé que Kadenz ne reconnaît pas (`crates/portail/src/main.rs`).
- `--mandataire` : le portail lit alors le dernier `X-Forwarded-For`. Il le croit aujourd'hui sans vérifier que le pair est Caddy (défaut 4 de la réponse 5) : c'est pourquoi le réseau `console` est interne et ne contient que Caddy et le portail. Le défaut reste à corriger avant la mise en ligne.
- Caddy sans root (réponse 6 d'OpenCode, point b) : avec `cap_drop: ALL`, la capacité de fichier que porte le binaire de l'image est masquée par l'ensemble limite et n'apporte rien ; un Caddy root aurait besoin de `cap_add: NET_BIND_SERVICE`, et un Caddy non root ne peut pas compter sur cette capacité de fichier sous `no-new-privileges`. D'où les ports internes 8080 et 8443. À vérifier à l'essai : `caddy validate`, et l'écriture des certificats dans `/data/caddy` avec l'uid 65534.
- Clé du portail : fichier hors dépôt, jamais dans l'image, exclu des sauvegardes (le dossier `secrets/` est exclu du dépôt restic). En compose sans Swarm, un secret de type fichier est un montage du fichier de l'hôte, avec ses droits : le fichier doit être lisible par l'utilisateur `kadenz` du conteneur seul (mode 400). Cet utilisateur est créé par `useradd --system` sans uid fixe (Dockerfile) : fixer `--uid 10001` dans les deux Dockerfile rend le propriétaire stable. Une copie hors ligne de la clé reste chez l'éditeur ; procédure de compromission : révocation et remplacement par une mise à jour (décision 29).
- Santé : les images du portail et du relais (debian-slim) n'ont ni curl ni wget. Une sonde demande soit une sous-commande de santé dans les deux programmes, soit une sonde extérieure (section 6). Sans elle, la panne silencieuse du relais décrite plus bas ne se voit pas.
  - Relais (lot R1) : `kadenz-relais sante --dossier /donnees` fait la poignée de main TLS avec le certificat du dossier épinglé, attend le défi du relais, puis sort avec le code 0 ou 1, sans rien créer. Le `--dossier` est obligatoire dans la sonde : le défaut est le chemin relatif `donnees-relais`. Dans le Dockerfile (lot D1 de kadenz-3a) : `HEALTHCHECK CMD ["kadenz-relais", "sante", "--dossier", "/donnees"]`.
  - Portail : sous-commande de santé à prévoir dans les lots du portail.
- Propriétaire de `/donnees` du relais : l'utilisateur du conteneur (uid 10001, une fois fixé dans le Dockerfile). Le relais met le dossier en 700 au démarrage et refuse de démarrer s'il ne peut pas le faire (lot R1) ; un volume créé avec un autre propriétaire l'empêche donc de démarrer, ce qui se voit tout de suite au lieu de passer inaperçu.
- Variante à trois conteneurs : un service `site` sur l'image Caddy, avec `auto_https off`, `admin off`, racine en lecture seule et `file_server`, sur le réseau `console` ; le Caddy d'entrée fait alors `reverse_proxy site:80` pour l'hôte du site.
- Démon Docker : mode rootless ou `userns-remap` à étudier, à décider avant la mise en service, car `userns-remap` change le propriétaire des volumes et du secret ; jamais de `docker.sock` monté, jamais `--privileged` ni `network_mode: host`.

### Corrections à faire dans le code avant la mise en ligne

Relevées par OpenCode (réponses 5 et 6), à porter dans les lots du portail et du relais :

| Programme | Correction |
|---|---|
| relais | **corrigé dans le lot R1** (écrit, compilation et commit en attente) : un échec de création de fil paniquait et tuait le fil d'écoute. Désormais `thread::Builder::spawn` ; la connexion est refusée, comptée dans les traces, et l'écoute continue ; une session dont le second fil ne part pas est coupée proprement. Essai automatique avec une limite de fils simulée |
| relais | **corrigé dans le lot R1** : frein par adresse (`crates/relais/src/frein.rs`), 256 connexions ouvertes au plus par adresse (pire cas d'une entreprise entière derrière une adresse : 161), rafale de 160 puis 16 nouvelles connexions par seconde, préfixe /64 en IPv6, table bornée à 16384 adresses. Le plafond par source dans `DOCKER-USER` (section 4) reste utile en amont |
| relais | **corrigé dans le lot R1** : un certificat ou une clé présent seul fait refuser le démarrage avec un message clair, au lieu de créer une nouvelle identité. Essai automatique dans les deux sens. Reste à écrire : la procédure de compromission de l'identité |
| relais | **corrigé dans le lot R1** : le relais met son dossier de données en 700 au démarrage (propriétaire du volume : voir plus haut) |
| portail et relais | binaires d'essai et clé d'essai dans les images de production ; uid de l'utilisateur `kadenz` à fixer |
| portail | défauts 1 à 9 de la réponse 5 (décision de kadenz-cc du 2026-11-18) ; journal structuré sur la sortie, avec l'adresse du client, pour fail2ban |

## 4. Pare-feu de l'hôte

- ufw garde SSH (port à choisir, par clé seulement, root refusé). Les règles ufw sur 80, 443 et le port du relais ne filtrent **pas** les ports publiés par Docker : ces paquets passent par la table nat et les chaînes FORWARD, avant les chaînes INPUT que ufw utilise (documentation de Docker, citée en réponse 6). Les garder pour la lisibilité seulement.
- Le filtrage tenu est la chaîne `DOCKER-USER`, traversée avant les règles de Docker. Les paquets y arrivent après la traduction d'adresse : le port public se lit par conntrack (`--ctorigdstport`), pas par `--dport`, qui désignerait le port du conteneur (8080 et 8443 pour Caddy). La règle de retour des connexions établies vient en premier. Squelette, à adapter à l'interface externe :

```sh
iptables -I DOCKER-USER 1 -m conntrack --ctstate ESTABLISHED,RELATED -j RETURN
iptables -I DOCKER-USER 2 -p tcp -m conntrack --ctorigdstport 80 -j RETURN
iptables -I DOCKER-USER 3 -p tcp -m conntrack --ctorigdstport 443 -j RETURN
iptables -I DOCKER-USER 4 -p udp -m conntrack --ctorigdstport 443 -j RETURN
iptables -I DOCKER-USER 5 -p tcp -m conntrack --ctorigdstport 47381 -j RETURN
iptables -I DOCKER-USER 6 -j DROP
```

- Mêmes règles en ip6tables, Docker publiant aussi sur IPv6 ; si IPv6 ne sert pas, le couper dans `daemon.json` est plus simple.
- Persistance : rien ne garantit que `DOCKER-USER` survive à un redémarrage du démon. Recharger les règles par une unité systemd `After=docker.service`, et revérifier après chaque mise à jour de Docker.
- Le `DROP` final change un oubli en panne visible : un service ajouté plus tard avec `ports:` reste fermé au lieu d'ouvrir une porte.
- Relais : plafond de nouvelles connexions par source dans `DOCKER-USER` (`-m hashlimit`), `net.ipv4.tcp_syncookies=1` (valeur par défaut), `nf_conntrack_max` dimensionné.
- Pare-feu du prestataire, s'il en propose un : seconde barrière, pas un remplacement.
- fail2ban sur SSH, puis sur les refus de la console une fois le journal structuré en place (section 3) et le journal d'accès de Caddy actif (section 2).
- Mises à jour de sécurité automatiques de l'hôte, heure en UTC. Vérification finale par un balayage des ports depuis l'extérieur.

## 5. Sauvegardes

À mettre en place et à **essayer en restauration avant la mise en ligne**, pas après (réponse 4 d'OpenCode).

| Quoi | Où | Pourquoi |
|---|---|---|
| `portail.sqlite` | volume `portail-donnees` | comptes, paiements, codes (empreintes), licences émises, journal |
| identité TLS du relais (certificat **et** clé) | volume `relais-donnees` | sans elle, l'empreinte change et chaque serveur d'entreprise doit réapprendre celle du relais |
| certificats de Caddy | volume `caddy-donnees` | évite de redemander les certificats après une restauration |
| Caddyfile, compose.yaml, `.env`, règles `DOCKER-USER` | dossier du déploiement | reconstruire le service à l'identique |

- Base du portail : copie cohérente par l'API de sauvegarde de SQLite (`.backup`), pas par une copie de fichier pendant que le portail écrit (mode WAL, `crates/portail/src/base.rs`). Deux pièges relevés par OpenCode : lancée en root sur le volume, la commande crée des fichiers `-wal` et `-shm` appartenant à root, que le portail ne pourra plus ouvrir ; la lancer avec l'uid du conteneur, ou conteneur arrêté. Et restic ne sauvegarde pas le volume en direct, seulement le fichier produit par `.backup`, après un `PRAGMA integrity_check` sur la copie.
- Le portail n'a pas de `busy_timeout` (réponse 5, défaut 13) : une sauvegarde qui tombe pendant une écriture peut échouer et doit être relancée par le script, en attendant la correction.
- La base n'est pas chiffrée au repos : la copie part chiffrée, hors du VPS. L'utilisateur emploie déjà restic vers OneDrive pour son serveur maison ; la même méthode convient, avec un dépôt restic propre au VPS et un mot de passe gardé hors du VPS. La destination reste à son choix.
- Relais : après chaque restauration, comparer l'empreinte affichée au démarrage à celle réglée sur les serveurs d'entreprise (une restauration qui oublie le certificat ou la clé en crée une nouvelle sans prévenir).
- La clé du portail n'entre pas dans ces sauvegardes : elle a sa copie hors ligne chez l'éditeur.
- Durée de conservation du journal du portail (adresses IP et identifiants) : à décider (réponse 5, points légers).

## 6. Avant la mise en ligne

1. Corrections de la section 3 faites dans le code (portail et relais), binaires d'essai retirés des images, image de Caddy épinglée par empreinte.
2. Nom de domaine et VPS choisis par l'utilisateur ; paramètres de la section 1 remplis.
3. Adresses vues par le relais : son frein par adresse (lot R1) ne sépare les clients que s'il voit leur vraie adresse. Une connexion qui passe par `docker-proxy` (IPv6 publié sans ip6tables, trafic en épingle depuis le VPS lui-même, `userland-proxy` actif) arrive avec l'adresse de la passerelle du pont Docker : tous ces clients partagent alors un seul seau de 256 connexions. Vérifier depuis deux adresses extérieures, en IPv4 et en IPv6, que les traces du relais et ses refus distinguent bien les adresses ; sinon, désactiver `userland-proxy` dans `daemon.json` avec ip6tables actif, ou couper IPv6.
4. Essai complet sur une machine d'essai : démarrage, certificats, en-têtes vérifiés (`curl -I`, erreurs et redirections comprises), règles `DOCKER-USER` après un redémarrage de Docker, balayage des ports depuis l'extérieur, essai de charge du relais, activation d'un code depuis un KadenzAdmin d'essai, enregistrement d'un serveur d'essai sur le relais, sauvegarde puis restauration de la base.
5. Supervision : sonde HTTP sur la console et poignée TLS sur le relais toutes les minutes, alerte sur l'expiration des certificats, le disque, les redémarrages. L'utilisateur a déjà ntfy et healthchecks.io pour son serveur maison ; même méthode possible.

## 7. Variante : Dell et Tailscale, sans nom de domaine

Décision de l'utilisateur du 2026-11-19, transmise par kadenz-cc : pas de nom de domaine ni de VPS pour l'instant ; hébergement sur le Dell (serveur maison, déjà relié par Tailscale) et, au besoin, sur Vercel ; Docker accordé sur le Dell. Les sections 2 à 6 restent la cible pour un VPS ; cette section dit ce qui change sur le Dell. Rien n'y a été essayé.

### Ce que permet Tailscale Funnel (documentation vérifiée le 2026-11-19)

| Point | Ce que dit Tailscale | Source |
|---|---|---|
| Ports publics | 443, 8443 et 10000 seulement | https://tailscale.com/kb/1223/funnel |
| Nom public | un nom dans le domaine du tailnet, `<machine>.<tailnet>.ts.net` ; pas de domaine personnel | même page |
| Chiffrement | « Funnel only works over TLS-encrypted connections » | même page |
| Aiguillage | les serveurs relais de Funnel lisent le nom SNI du ClientHello TLS et transmettent la connexion chiffrée à la machine, sans terminer TLS | https://tailscale.com/blog/introducing-tailscale-funnel |
| Modes | `--https=<port>` (Tailscale termine TLS avec un certificat ts.net et passe à un service HTTP local), `--tcp=<port>` (TCP brut), `--tls-terminated-tcp=<port>` ; `--proxy-protocol=1|2` pour transmettre l'adresse du client en TCP | https://tailscale.com/kb/1311/tailscale-funnel |
| Débit | « subject to non-configurable bandwidth limits », valeur non publiée sur la page | https://tailscale.com/kb/1223/funnel |
| En-têtes | le trafic Funnel ne porte aucun en-tête d'identité Tailscale ; la page de Serve ne dit rien de `X-Forwarded-For` | https://tailscale.com/kb/1312/serve |
| Prérequis | MagicDNS, certificats HTTPS du tailnet, attribut `funnel` dans la politique du tailnet, Tailscale 1.38.3 ou plus | https://tailscale.com/kb/1223/funnel |

### Conséquences pour chaque service

- **Console des licences (portail)** : compatible. `tailscale funnel --https=443` vers le port local du portail ; Tailscale fournit le certificat du nom ts.net, Caddy n'est plus indispensable devant. Deux points à vérifier à l'essai, faute de documentation : l'adresse du client que voit le portail (Funnel n'annonce pas de `X-Forwarded-For` ; sans elle, le frein de connexion du portail et le journal compteraient toutes les requêtes comme une seule adresse), et la tenue de l'option `--mandataire`, qui ne doit pas croire un en-tête qu'un client aurait fourni lui-même.
- **Site public** : un seul nom ts.net par machine, et les cookies ne sont pas isolés par port : site et console sur le même nom (443 et 8443) partageraient le cookie de session de la console, ce que la recommandation des deux noms d'hôte (section 1) voulait éviter. Héberger le site statique sur Vercel, sous son adresse par défaut, garde les deux origines séparées ; les conditions de Vercel sont à vérifier dans sa documentation avant de l'adopter.
- **Relais d'accès à distance** : **incompatible avec Funnel dans son protocole actuel**, pour deux raisons tirées du code :
  1. un poste se présente par des trames en clair (premier octet au plus 0x10, `crates/relais/src/relais.rs`, fonction `connexion`) ; Funnel n'accepte que des connexions TLS ;
  2. la connexion de contrôle d'un serveur est bien en TLS, mais avec le nom SNI `kadenz-relais` (`NOM_TLS`, `crates/relais/src/tls.rs`), et non le nom ts.net de la machine ; Funnel ne peut pas l'aiguiller.

  De plus, derrière Funnel, le relais verrait toutes les connexions venir de Tailscale : son frein par adresse (lot R1) n'aurait plus d'effet sans `--proxy-protocol` et sans lecture du protocole PROXY dans le relais, qui ne la fait pas aujourd'hui.

### Pistes pour le relais, à trancher

| Piste | Ce qu'elle demande | Limites |
|---|---|---|
| A. Redirection d'un port de la box vers le Dell | une adresse IPv4 publique (pas de CGNAT) et une redirection de port sur la box ; adresse du relais donnée en IP | l'adresse publique d'un abonnement domestique peut changer, et chaque serveur d'entreprise devrait alors être réglé de nouveau ; à vérifier chez le fournisseur d'accès de l'utilisateur |
| B. Protocole du relais rendu compatible avec Funnel | toutes les connexions en TLS avec le nom SNI ts.net (le relais garde son certificat épinglé, Funnel ne terminant pas TLS), trames du poste passées dans TLS, lecture du protocole PROXY ; nouvelle version des trames, donc changement dans le relais, le serveur et le poste | changement de spécification (section 7 de `docs/spec-synchronisation.md`) à décider entre sessions ; débit de Funnel limité et non publié ; nom public lié au tailnet de l'utilisateur |
| C. Relais joignable dans le tailnet seulement | aucune modification | les serveurs et postes des entreprises clientes devraient rejoindre le tailnet de l'utilisateur ; ne convient qu'aux essais internes (jugement) |

Pour les essais internes, la piste C suffit et ne demande rien. Pour des clients réels, A dépend de la ligne de l'utilisateur, B d'une décision de conception ; le VPS des sections 2 à 6 reste la solution sans ces limites.

### Ce qui ne change pas

- Docker sur le Dell : mêmes réglages de conteneurs que la section 3 (sans root, `cap_drop: ALL`, lecture seule, `pids_limit` et `ulimits` du relais, rotation des journaux) ; les règles `DOCKER-USER` de la section 4 ne s'appliquent qu'aux ports publiés, et rien n'est publié vers Internet si tout passe par Funnel.
- Sauvegardes de la section 5 : le Dell a déjà restic vers OneDrive ; la base du portail et l'identité du relais y entrent avec les mêmes précautions (`.backup` avec l'uid du conteneur, certificat **et** clé du relais).
- Sondes : `kadenz-relais sante --dossier /donnees` dans le conteneur ; ntfy et healthchecks.io déjà en place sur le Dell.
