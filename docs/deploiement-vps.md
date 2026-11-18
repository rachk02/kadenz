# Kadenz phase 2 - Déploiement sur le VPS (proposition)

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-18 | Brouillon de kadenz-74 à la demande de kadenz-cc : Caddyfile, compose.yaml, pare-feu et sauvegardes proposés, d'après la réponse 4 d'OpenCode (`docs/securite/echanges-opencode.md`), la section 7 de `docs/spec-synchronisation.md` et les Dockerfile du dépôt. Rien n'est en service. |

Statut : **proposé**. Le VPS, son prestataire, le nom de domaine et la mise en service restent à la décision de l'utilisateur (décisions 16 et 69 du cahier de la phase 2). Le relais et le portail y iront une fois leur implémentation finie (décision de l'utilisateur du 2026-11-18). Rien ici n'a été essayé sur un VPS ; chaque fichier est à relire et à essayer d'abord sur une machine d'essai.

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
| `EMAIL_ACME` | adresse donnée à l'autorité de certification (facultative) |
| `PORT_RELAIS` | port public du relais, 47381 par défaut |

## 2. Caddyfile proposé

Les valeurs `{$NOM}` sont lues dans l'environnement du conteneur Caddy (fichier `.env` à côté du compose, hors dépôt).

```caddyfile
{
	email {$EMAIL_ACME}
	# API d'administration coupée : la configuration se recharge par un redémarrage.
	admin off
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
}

{$DOMAINE_SITE} {
	import entetes
	header Content-Security-Policy "default-src 'none'; style-src 'self'; img-src 'self'; font-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'"
	request_body {
		max_size 64KB
	}
	root * /srv/site
	file_server
	handle_errors {
		rewrite * /404.html
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

- TLS : Caddy obtient et renouvelle les certificats et redirige HTTP vers HTTPS par défaut ; TLS 1.2 et 1.3 par défaut. TLS 1.3 seul est possible (`tls { protocols tls1.3 }`), à décider.
- `request_body` est une directive expérimentale de Caddy (réponse 4 d'OpenCode) : le portail doit garder sa propre limite de taille (défaut 3 de la réponse 5).
- Caddy n'a pas de limite de débit native ; le frein du portail reste la barrière (défauts 3 à 5 de la réponse 5, à corriger avant la mise en ligne).
- Formulaire de contact et espace client : routes du portail à venir (`docs/spec-site.md`, sections 5 et 6) ; on ajoutera alors un `handle /contact* { reverse_proxy portail:47382 }` sur l'hôte du site.

## 3. compose.yaml proposé

Construit depuis la racine du dépôt tant que le portail n'en est pas sorti (les Dockerfile l'exigent). Les images de base sont déjà épinglées par empreinte dans les Dockerfile.

```yaml
name: kadenz

services:
  caddy:
    image: caddy:2   # à épingler par empreinte au moment de la mise en service
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
      - "443:443/udp"
    env_file: .env
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile:ro
      - ./site:/srv/site:ro
      - caddy-donnees:/data
      - caddy-config:/config
    read_only: true
    tmpfs:
      - /tmp:rw,noexec,nosuid,size=16m
    cap_drop: [ALL]
    # Le binaire de l'image officielle porte la capacité de lier les ports 80 et 443 :
    # sans elle, il peut refuser de démarrer avec cap_drop ALL. À vérifier à l'essai.
    cap_add: [NET_BIND_SERVICE]
    security_opt: [no-new-privileges:true]
    pids_limit: 256
    mem_limit: 256m
    networks: [entree, console]

  portail:
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
    pids_limit: 256
    mem_limit: 512m
    cpus: 1
    networks: [relais]

networks:
  entree: {}
  console:
    internal: true
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
- Clé du portail : fichier hors dépôt, jamais dans l'image. En compose sans Swarm, un secret de type fichier est un montage du fichier de l'hôte, avec ses droits : le fichier doit être lisible par l'utilisateur `kadenz` du conteneur (mode 400, propriétaire l'uid de cet utilisateur), et rien d'autre. Une copie hors ligne de la clé reste chez l'éditeur ; procédure de compromission : révocation et remplacement par une mise à jour (décision 29).
- Les images actuelles embarquent aussi `portail_essai` et `relais_essai`, avec la clé d'essai du banc (Dockerfile) : à retirer des images de production avant la mise en ligne (réponse 4 d'OpenCode).
- Variante à trois conteneurs : un service `site` sur l'image Caddy, avec `auto_https off`, `admin off`, racine en lecture seule et `file_server`, sur le réseau `console` ; le Caddy d'entrée fait alors `reverse_proxy site:80` pour l'hôte du site.
- Démon Docker : mode rootless ou `userns-remap` à étudier (réponse 4 d'OpenCode) ; jamais de `docker.sock` monté, jamais `--privileged` ni `network_mode: host`.

## 4. Pare-feu de l'hôte

- Ouverts : SSH (port à choisir, par clé seulement, root refusé), 80/tcp, 443/tcp, 443/udp si HTTP/3 reste actif, `PORT_RELAIS`/tcp. Rien d'autre.
- Attention : les ports publiés par Docker contournent les règles d'ufw, car Docker écrit ses propres règles iptables avant celles d'ufw. La règle tenue est donc de ne publier que les ports ci-dessus dans le compose (le portail n'en publie aucun), et, si un filtrage plus fin est voulu, de passer par la chaîne `DOCKER-USER` ou par le pare-feu du prestataire. À vérifier sur la machine d'essai avec un balayage de ports depuis l'extérieur.
- fail2ban sur SSH ; sur les refus de connexion de la console quand le portail les journalisera (aujourd'hui, `eprintln!` seulement, réponse 5).
- Mises à jour de sécurité automatiques de l'hôte, heure en UTC.

## 5. Sauvegardes

À mettre en place et à **essayer en restauration avant la mise en ligne**, pas après (réponse 4 d'OpenCode).

| Quoi | Où | Pourquoi |
|---|---|---|
| `portail.sqlite` | volume `portail-donnees` | comptes, paiements, codes (empreintes), licences émises, journal |
| identité TLS du relais | volume `relais-donnees` | sans elle, l'empreinte change et chaque serveur d'entreprise doit réapprendre celle du relais |
| certificats de Caddy | volume `caddy-donnees` | évite de redemander les certificats après une restauration |
| Caddyfile, compose.yaml, `.env` | dossier du déploiement | reconstruire le service à l'identique |

- Base du portail : copie cohérente par l'API de sauvegarde de SQLite, pas par une copie de fichier pendant que le portail écrit (mode WAL, `crates/portail/src/base.rs`). Par exemple, depuis l'hôte, `sqlite3 portail.sqlite ".backup '/sauvegardes/portail-AAAA-MM-JJ.sqlite'"` sur le dossier du volume. Le portail n'a pas de `busy_timeout` aujourd'hui (réponse 5, défaut 13) : une sauvegarde qui tombe pendant une écriture peut échouer et doit être relancée ; à corriger dans le portail ou à gérer dans le script.
- La base n'est pas chiffrée au repos : la copie part chiffrée, hors du VPS. L'utilisateur emploie déjà restic vers OneDrive pour son serveur maison ; la même méthode convient, avec un dépôt restic propre au VPS et un mot de passe gardé hors du VPS. La destination reste à son choix.
- La clé du portail n'entre pas dans ces sauvegardes : elle a sa copie hors ligne chez l'éditeur.
- Durée de conservation du journal du portail (adresses IP et identifiants) : à décider (réponse 5, points légers).

## 6. Avant la mise en ligne

1. Défauts 1 à 9 de la réponse 5 corrigés dans le portail (décision de kadenz-cc du 2026-11-18), et binaires d'essai retirés des images.
2. Nom de domaine et VPS choisis par l'utilisateur ; paramètres de la section 1 remplis.
3. Essai complet sur une machine d'essai : démarrage, certificats, en-têtes vérifiés (`curl -I`), balayage des ports depuis l'extérieur, activation d'un code depuis un KadenzAdmin d'essai, enregistrement d'un serveur d'essai sur le relais, sauvegarde puis restauration de la base.
4. Supervision : sonde sur la console et sur le relais, alerte sur l'expiration des certificats, le disque, les redémarrages. L'utilisateur a déjà ntfy et healthchecks.io pour son serveur maison ; même méthode possible.
