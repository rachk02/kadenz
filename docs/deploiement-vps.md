# Kadenz phase 2 - Déploiement sur le VPS (proposition)

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-18 | Brouillon de kadenz-74 à la demande de kadenz-cc : Caddyfile, compose.yaml, pare-feu et sauvegardes proposés, d'après la réponse 4 d'OpenCode (`docs/securite/echanges-opencode.md`), la section 7 de `docs/spec-synchronisation.md` et les Dockerfile du dépôt. Rien n'est en service. |
| 0.2 | 2026-11-18 | Relecture d'OpenCode (réponse 6) intégrée : filtrage par la chaîne DOCKER-USER, Caddy sans root sur les ports internes 8080 et 8443, apex, journaux d'accès, rotation des journaux Docker, healthchecks, ulimits et pids du relais, pages d'erreur par code, pièges de la sauvegarde SQLite, défauts du relais à corriger dans le code. |

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
- Variante à trois conteneurs : un service `site` sur l'image Caddy, avec `auto_https off`, `admin off`, racine en lecture seule et `file_server`, sur le réseau `console` ; le Caddy d'entrée fait alors `reverse_proxy site:80` pour l'hôte du site.
- Démon Docker : mode rootless ou `userns-remap` à étudier, à décider avant la mise en service, car `userns-remap` change le propriétaire des volumes et du secret ; jamais de `docker.sock` monté, jamais `--privileged` ni `network_mode: host`.

### Corrections à faire dans le code avant la mise en ligne

Relevées par OpenCode (réponses 5 et 6), à porter dans les lots du portail et du relais :

| Programme | Correction |
|---|---|
| relais | un échec de création de fil (`thread::spawn`) panique et tue le fil d'écoute ; le processus reste vivant sans plus rien accepter (`crates/relais/src/relais.rs`, `main.rs`) : passer par `thread::Builder::spawn` et refuser la connexion |
| relais | aucun frein par adresse sur les nouvelles connexions ; en attendant, plafond par source dans `DOCKER-USER` (section 4) |
| relais | un certificat manquant est régénéré même si la clé est là (`crates/relais/src/tls.rs`) : une restauration partielle change l'empreinte sans prévenir ; procédure de compromission de l'identité à écrire |
| relais | `/donnees` créé sans `chmod 700` (`crates/relais/Dockerfile`), contrairement au portail |
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
3. Essai complet sur une machine d'essai : démarrage, certificats, en-têtes vérifiés (`curl -I`, erreurs et redirections comprises), règles `DOCKER-USER` après un redémarrage de Docker, balayage des ports depuis l'extérieur, essai de charge du relais, activation d'un code depuis un KadenzAdmin d'essai, enregistrement d'un serveur d'essai sur le relais, sauvegarde puis restauration de la base.
4. Supervision : sonde HTTP sur la console et poignée TLS sur le relais toutes les minutes, alerte sur l'expiration des certificats, le disque, les redémarrages. L'utilisateur a déjà ntfy et healthchecks.io pour son serveur maison ; même méthode possible.
