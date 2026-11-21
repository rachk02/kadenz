# Kadenz phase 2 - Déploiement sur le VPS (proposition)

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-18 | Brouillon de kadenz-74 à la demande de kadenz-cc : Caddyfile, compose.yaml, pare-feu et sauvegardes proposés, d'après la réponse 4 d'OpenCode (`docs/securite/echanges-opencode.md`), la section 7 de `docs/spec-synchronisation.md` et les Dockerfile du dépôt. Rien n'est en service. |
| 0.2 | 2026-11-18 | Relecture d'OpenCode (réponse 6) intégrée : filtrage par la chaîne DOCKER-USER, Caddy sans root sur les ports internes 8080 et 8443, apex, journaux d'accès, rotation des journaux Docker, healthchecks, ulimits et pids du relais, pages d'erreur par code, pièges de la sauvegarde SQLite, défauts du relais à corriger dans le code. |
| 0.3 | 2026-11-18 | Lot R1 du relais écrit (pas encore compilé ni commité) : les quatre défauts du relais passent en « corrigé dans R1 » ; sonde de santé `kadenz-relais sante` ; propriétaire de `/donnees` ; adresses vues par le relais derrière docker-proxy, à vérifier avant la mise en ligne. |
| 0.4 | 2026-11-19 | Section 7 : variante sur le Dell avec Tailscale, sans nom de domaine (décision de l'utilisateur du 2026-11-19). Funnel vérifié dans sa documentation : le portail y passe, le relais non dans son protocole actuel ; trois pistes pour le relais, à trancher. |
| 0.5 | 2026-11-19 | `--mandataire` prend l'adresse du mandataire de confiance (lot D1 de kadenz-3a) : Caddy à adresse fixe sur le réseau « console » (paramètres `RESEAU_CONSOLE` et `IP_CADDY_CONSOLE`), adresse de tailscaled à relever sur le Dell pour Funnel ; sonde de santé du portail. |
| 0.6 | 2026-11-19 | Section 7 : trousse du Dell écrite par kadenz-3a à la demande de kadenz-cc (`deploiement/dell/`) : compose durci sur 127.0.0.1, image d'essai du portail, crochet de sauvegarde, sonde de santé et alerte, procédure ; étape Funnel sur accord de l'utilisateur. Rien n'est lancé. Section 5 : le portail a désormais un `busy_timeout`. |
| 0.7 | 2026-11-19 | Trousse du Dell : forme d'appel de `/usr/local/sbin/alerte` et place du crochet dans `/usr/local/sbin/sauvegarde`, relevées sur le Dell par kadenz-cc. |
| 0.8 | 2026-11-19 | Essai de la trousse sur le Dell par kadenz-cc (603d3da, scripts de 374be61), tout nettoyé ensuite : construction, clé d'essai, santé, écoute sur 127.0.0.1, cookies sur `http://localhost`, adresse vue par le portail (la passerelle), sauvegarde, restauration et alertes conformes. Écarts corrigés : copie de la base en 600, entrée standard fermée pour les appels docker, conteneur arrêté vu comme arrêté, `journal.jsonl` mis de côté après une restauration. |
| 0.9 | 2026-11-19 | Section 7 : ouverture de la console par Cloudflare Tunnel, domaine acheté par l'utilisateur chez Cloudflare (transmis par kadenz-51). Surcouche `compose.public.yaml` (cloudflared durci, sans port publié, jeton lu depuis un fichier hors dépôt, `--mandataire` à l'adresse de cloudflared), sonde de santé par les étiquettes de Compose, étapes du compte Cloudflare marquées pour l'utilisateur, option Cloudflare Access en décision de l'utilisateur. Rien n'est lancé ; l'en-tête de l'adresse du client attend la vérification d'OpenCode. |
| 0.10 | 2026-11-19 | Cloudflare Tunnel : image cloudflared relevée sur le Dell par kadenz-51 (2026.10.7, empreinte, utilisateur 65532, `--token-file` présent) ; adresse du client lue dans `CF-Connecting-IP` (vérification d'OpenCode, option `--entete-client` du portail) ; relais : piste de la redirection de port sur la box retenue pour plus tard. |
| 0.11 | 2026-11-19 | Cloudflare Tunnel : tunnel de l'utilisateur relié, conteneur provisoire `kadenz-tunnel` sur le Dell (kadenz-51), à retirer avant le démarrage par la surcouche ; jeton à nettoyer de sa fin de ligne. |
| 0.12 | 2026-11-19 | Portail en service sur le Dell (kadenz-51, accord de l'utilisateur). Point d'entrée unique `kadenz-compose` : la surcouche publique est gardée à chaque appel dès que `KADENZ_PUBLIC=1`, garde-fou contre la recréation du portail sans elle ; `kadenz-sauvegarde` passe par lui. |
| 0.13 | 2026-11-19 | Secrets de double authentification chiffrés avec une clé dérivée de celle du portail (spécification du portail 0.11) : changer la clé du portail demande `retirer-double-authentification` pour chaque éditeur. |
| 0.14 | 2026-11-19 | Archivage du journal du portail (spécification du portail 0.13) : étape 4 de `kadenz-sauvegarde`, seulement si `KADENZ_JOURNAL_JOURS` est réglé (vide par défaut, durée à décider par l'utilisateur) ; restauration avec un journal archivé. |
| 0.15 | 2026-11-20 | Relais en WebSocket par Cloudflare Tunnel (`docs/spec-relais-websocket.md`, lot 1 de kadenz-e6, 453c3f5) : surcouche publique avec la passerelle `--ecoute-ws 0.0.0.0:47383 --mandataire <IP de cloudflared>`, réseau interne `tunnel-relais` (`RESEAU_TUNNEL_RELAIS`, `IP_TUNNEL_RELAIS`), route relais.mcarre.dev vers `http://relais:47383` à créer par l'utilisateur. Rien n'est déployé. |
| 0.16 | 2026-11-20 | Relais en WebSocket en service sur le Dell (kadenz-dd, accord de l'utilisateur) : images d69e97da7171 (portail, relais, outil-sqlite), `RESEAU_TUNNEL_RELAIS=172.30.49.0/24` et `IP_TUNNEL_RELAIS=172.30.49.10` posés, route relais.mcarre.dev vers `http://relais:47383` créée par l'utilisateur ; ouverture WebSocket par Cloudflare vérifiée (101), GET simple en 502. Six extinctions brutales du Dell dans la nuit (délestage) : services revenus seuls, construction relancée. |
| 0.17 | 2026-11-21 | Portail 8c96a66b4799 en service sur le Dell (kadenz-dd, accord direct de l'utilisateur) : thème noir et sauge des pages du compte, titres de page en « \| », relais et outil-sqlite inchangés (images rebaptisées). Sauvegarde avant bascule ; retour arrière : `KADENZ_REV=d69e97da7171` puis `kadenz-compose up -d portail relais`. |
| 0.18 | 2026-11-21 | Portail 5db1c8c23961 (feuille de style à empreinte) puis 15be84d7a6e2 (coque animée du gabarit : GSAP, ScrollTrigger, SplitText, Lenis officiels, fichiers à empreinte sous /gabarit/) en service sur le Dell. Sauvegarde avant chaque bascule ; retour arrière : remettre KADENZ_REV à la révision précédente puis `kadenz-compose up -d portail relais`. |

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
| `RESEAU_CONSOLE` | sous-réseau privé du réseau Docker « console », libre sur l'hôte |
| `IP_CADDY_CONSOLE` | adresse de Caddy dans ce sous-réseau, donnée au portail par `--mandataire` |

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
    # Adresse fixe sur le réseau « console » : c'est elle que le portail reçoit par
    # --mandataire, la seule dont il croit l'en-tête X-Forwarded-For.
    networks:
      entree: {}
      console:
        ipv4_address: ${IP_CADDY_CONSOLE}

  portail:
    <<: *journaux
    build:
      context: .
      dockerfile: crates/portail/Dockerfile
    restart: unless-stopped
    # Le portail ne traite pas SIGTERM (crates/portail/Dockerfile) : init obligatoire.
    init: true
    command: ["--cle", "/run/secrets/cle_portail", "--mandataire", "${IP_CADDY_CONSOLE}"]
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
    # Sous-réseau fixé pour que l'adresse de Caddy (IP_CADDY_CONSOLE) le soit aussi.
    ipam:
      config:
        - subnet: ${RESEAU_CONSOLE}
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
- `--mandataire <adresse>` : adresse IP du mandataire de confiance, vue par le conteneur du portail. Pour les seules connexions venues de cette adresse, le portail lit l'adresse du client dans la dernière valeur de `X-Forwarded-For` ; pour toute autre, il garde l'adresse du pair (`crates/portail/src/web.rs`, `adresse_selon`, et son essai). Le défaut 4 de la réponse 5 (en-tête cru sans vérifier le pair) est ainsi corrigé dans le lot D1 de kadenz-3a, encore en cours au 2026-11-19. Ici, l'adresse est celle de Caddy sur le réseau `console`, fixée par `IP_CADDY_CONSOLE` dans le sous-réseau `RESEAU_CONSOLE`. Le réseau `console` reste interne et ne contient que Caddy et le portail.
- Caddy sans root (réponse 6 d'OpenCode, point b) : avec `cap_drop: ALL`, la capacité de fichier que porte le binaire de l'image est masquée par l'ensemble limite et n'apporte rien ; un Caddy root aurait besoin de `cap_add: NET_BIND_SERVICE`, et un Caddy non root ne peut pas compter sur cette capacité de fichier sous `no-new-privileges`. D'où les ports internes 8080 et 8443. À vérifier à l'essai : `caddy validate`, et l'écriture des certificats dans `/data/caddy` avec l'uid 65534.
- Clé du portail : fichier hors dépôt, jamais dans l'image, exclu des sauvegardes (le dossier `secrets/` est exclu du dépôt restic). En compose sans Swarm, un secret de type fichier est un montage du fichier de l'hôte, avec ses droits : le fichier doit être lisible par l'utilisateur `kadenz` du conteneur seul (mode 400). Cet utilisateur est créé par `useradd --system` sans uid fixe (Dockerfile) : fixer `--uid 10001` dans les deux Dockerfile rend le propriétaire stable. Une copie hors ligne de la clé reste chez l'éditeur ; procédure de compromission : révocation et remplacement par une mise à jour (décision 29).
- Santé : les images du portail et du relais (debian-slim) n'ont ni curl ni wget. Une sonde demande soit une sous-commande de santé dans les deux programmes, soit une sonde extérieure (section 6). Sans elle, la panne silencieuse du relais décrite plus bas ne se voit pas.
  - Relais (lot R1) : `kadenz-relais sante --dossier /donnees` fait la poignée de main TLS avec le certificat du dossier épinglé, attend le défi du relais, puis sort avec le code 0 ou 1, sans rien créer. Le `--dossier` est obligatoire dans la sonde : le défaut est le chemin relatif `donnees-relais`. Dans le Dockerfile (lot D1 de kadenz-3a) : `HEALTHCHECK CMD ["kadenz-relais", "sante", "--dossier", "/donnees"]`.
  - Portail (lot D1 de kadenz-3a, en cours) : `kadenz-portail sante [--ecoute <adresse:port>]` interroge `/sante` du portail local et sort avec le code 0 si la réponse est 200 (`crates/portail/src/main.rs`).
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
- Le portail attend désormais 5 secondes une écriture en cours (`PRAGMA busy_timeout = 5000`, `crates/portail/src/base.rs`, défaut 13 de la réponse 5 corrigé) ; l'outil de sauvegarde attend de son côté jusqu'à 10 secondes (`.timeout`, trousse du Dell, section 7).
- La base n'est pas chiffrée au repos : la copie part chiffrée, hors du VPS. L'utilisateur emploie déjà restic vers OneDrive pour son serveur maison ; la même méthode convient, avec un dépôt restic propre au VPS et un mot de passe gardé hors du VPS. La destination reste à son choix.
- Relais : après chaque restauration, comparer l'empreinte affichée au démarrage à celle réglée sur les serveurs d'entreprise (une restauration qui oublie le certificat ou la clé en crée une nouvelle sans prévenir).
- La clé du portail n'entre pas dans ces sauvegardes : elle a sa copie hors ligne chez l'éditeur.
- Les secrets de double authentification sont chiffrés dans la base avec une clé dérivée de celle du portail (spécification du portail, section 9). Restaurer une base avec une autre clé du portail, ou changer la clé, les rend illisibles : pour chaque éditeur, `kadenz-portail --dossier <dossier> retirer-double-authentification <identifiant>`, puis nouvelle mise en place à sa connexion. La première ouverture par le programme du schéma 10 chiffre les secrets encore en clair ; essayer cette ouverture sur une copie de la base avant de remplacer le portail en service.
- Durée de conservation du journal du portail (adresses IP et identifiants) : à décider (réponse 5, points légers). `KADENZ_JOURNAL_JOURS` (section 7) borne seulement la base en service : l'archivage déplace les entrées dans `journal.jsonl`, sans rien effacer, et elles restent aussi dans chaque instantané restic. La conservation réelle des adresses IP et identifiants dépend donc de `journal.jsonl` et de la rétention de restic, à décider avec la durée ; vide, rien n'est archivé.

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

- **Console des licences (portail)** : compatible. `tailscale funnel --https=443` vers le port local du portail ; Tailscale fournit le certificat du nom ts.net, Caddy n'est plus indispensable devant. Deux points à vérifier à l'essai, faute de documentation : l'adresse du client que voit le portail (Funnel n'annonce pas de `X-Forwarded-For` ; sans elle, le frein de connexion du portail et le journal compteraient toutes les requêtes comme une seule adresse), et l'adresse à donner à `--mandataire` : celle de tailscaled vue par le conteneur du portail, sans doute la passerelle du réseau Docker si tailscaled tourne sur l'hôte (remarque de kadenz-3a), à relever à l'essai sur le Dell. Le portail ne croit `X-Forwarded-For` que des connexions venues de cette adresse.
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

### Trousse du Dell (`deploiement/dell/`)

Écrite le 2026-11-19 à la demande de kadenz-cc, **rien n'est lancé** : ni construction, ni démarrage, ni Funnel. L'essai sur le Dell est conduit par kadenz-cc, avec une clé d'essai et tout sur 127.0.0.1. Aucun secret dans le dépôt, qui est public : ni sujet ntfy, ni adresse de veille, ni mot de passe restic, ni adresse Tailscale, ni nom ts.net.

| Fichier | Rôle |
|---|---|
| `compose.yaml` | portail et relais : images marquées par le commit (`KADENZ_REV`, obligatoire), racine en lecture seule, `cap_drop: ALL`, `no-new-privileges`, `init`, volumes nommés, ports publiés sur `127.0.0.1` seulement, sondes de santé, journaux bornés, `restart: unless-stopped` ; clé du portail montée en lecture seule depuis un fichier de l'hôte ; service `outil-sqlite` (profil `outils`) pour les sauvegardes |
| `compose.essai.yaml` | essai seulement : image du portail construite avec le profil `dev`, étiquette `essai-<commit>`, `KADENZ_PORTAIL_CLE` (seule une version de développement lit cette variable) |
| `kadenz.env.exemple` | modèle de `/etc/kadenz/kadenz.env`, sans valeur secrète |
| `outil-sqlite/Dockerfile` | sqlite3 sur la même image de base épinglée, sous l'uid 10001 |
| `kadenz-sauvegarde` | crochet appelé par `/usr/local/sbin/sauvegarde` avant restic |
| `kadenz-sante`, `kadenz-sante.service`, `kadenz-sante.timer` | santé des conteneurs toutes les 5 minutes, alerte au changement d'état ; conteneurs trouvés par les étiquettes de Compose, `tunnel` surveillé seulement s'il existe |
| `compose.public.yaml` | ouverture de la console par Cloudflare Tunnel, sur accord de l'utilisateur (sous-section « Cloudflare Tunnel ») |
| `kadenz-compose` | point d'entrée unique de Docker Compose : `compose.yaml`, plus `compose.public.yaml` si `KADENZ_PUBLIC=1` dans les réglages, ou `compose.essai.yaml` si `KADENZ_ESSAI=1` à l'appel ; garde-fou (choix et limites) |

Le Dockerfile du portail prend un argument `PROFIL` (`release` par défaut) ; celui du relais ne change pas : le relais reste en version optimisée, y compris à l'essai.

#### Choix et limites

- Clé du portail : fichier de l'hôte, propriétaire `10001:10001`, mode 400, hors des dossiers sauvegardés par restic (`/etc`, `/home`, `/root`, `/usr/local`...), d'où `/srv/kadenz/cle/` dans le modèle. Le montage est en lecture seule ; un fichier absent fait refuser le démarrage au lieu de créer un dossier vide à sa place.
- `--mandataire` reçoit la passerelle du réseau `portail`, à sous-réseau fixé (`RESEAU_PORTAIL`, `PASSERELLE_PORTAIL`) : une connexion venue de l'hôte, par docker-proxy ou par tailscaled, arrive avec cette adresse. Conséquence : tout programme local du Dell peut aussi fournir un `X-Forwarded-For` cru par le portail ; accepté sur le Dell, à revoir si d'autres services y tournent. Confirmé à l'essai du 2026-11-19 : le portail voit la passerelle, sans Funnel. Avec Cloudflare Tunnel, `compose.public.yaml` remplace cette valeur par l'adresse de cloudflared (plus bas).
- Sauvegarde de la base : jamais de copie à chaud du fichier. `VACUUM INTO` par l'outil sqlite3, sous l'uid du portail (les fichiers `-wal` et `-shm` restent les siens), sans réseau ; puis `PRAGMA integrity_check` sur la copie, qui doit répondre `ok`.
- Journal : `journal-exporter --depuis <dernier id exporté>` dans le conteneur en service, ajouté à `journal.jsonl` ; puis `journal-verifier`, dont l'échec déclenche une alerte. Après une restauration, la base a perdu les entrées postérieures à sa sauvegarde, et les entrées suivantes reprendront les mêmes numéros avec un autre contenu : `journal.jsonl` est alors mis de côté (étape 9), jamais complété tel quel.
- Archivage du journal (étape 4 du crochet, spécification du portail, section 9), seulement si `KADENZ_JOURNAL_JOURS` est réglé dans `kadenz.env` ; vide par défaut, rien n'est archivé. Après les copies : `journal-verifier --archive -` sur `journal.jsonl` complet, puis `journal-archiver` jusqu'à la dernière ligne d'avant la limite. Un désaccord ou un refus déclenche une alerte et ne retire rien, sans arrêter le crochet ni la sauvegarde restic qui le suit. Les entrées archivées ne vivent plus que dans `journal.jsonl` et ses sauvegardes restic : ce fichier n'est jamais tronqué ni effacé, sinon elles sont perdues.
- Limite de la restauration avec un journal archivé : la base restaurée repart de son point d'archivage, pas de l'entrée 0. Repartir d'un `journal.jsonl` vide ne suffit plus : la vérification avec l'export échouerait chaque nuit et l'archivage serait refusé. Il faut repartir de l'ancien export, coupé à la dernière entrée de la base restaurée (étape 9).
- Relais : certificat **et** clé copiés (`relais.crt`, `relais.cle`).
- Santé : `restart: unless-stopped` ne redémarre pas un conteneur « unhealthy » ; l'alerte de `kadenz-sante` est la seule réaction. Une alerte à la panne, une au retour, pas à chaque passage ; un conteneur absent ou arrêté compte comme une panne.
- `/usr/local/sbin/alerte` prend `alerte "titre" "message" [priorité 1 à 5]`, priorité 3 par défaut, titre préfixé par le nom de la machine (relevé sur le Dell par kadenz-cc). Titres « Kadenz, sauvegarde » et « Kadenz, <service> » ; priorité 4 pour un échec ou une panne, 3 pour un retour. Variables `ALERTE`, `KADENZ_SOURCE`, `KADENZ_ENV`, `KADENZ_SAUVEGARDES` pour adapter les chemins sans modifier les scripts.
- Construction sur le Dell : 3,2 Go de mémoire, d'où `CARGO_BUILD_JOBS=1`.
- Tous les appels passent par `kadenz-compose`, jamais par `docker compose -f compose.yaml` seul. Une fois le portail ouvert, un appel sans la surcouche signale le tunnel comme « orphan container », et un `up` recréerait le portail avec l'ancien `--mandataire` et sans `--entete-client` : le frein par adresse cesserait de distinguer les clients, sans rien signaler ; un `--remove-orphans` couperait le tunnel. `kadenz-compose` ajoute la surcouche dès que `KADENZ_PUBLIC=1` figure dans `/etc/kadenz/kadenz.env` (ligne lue, fichier jamais exécuté). Garde-fou : sans `KADENZ_PUBLIC=1` alors qu'un conteneur du service `tunnel` existe, `up`, `create`, `down` et `rm` sont refusés ; `ps`, `logs`, `config`, `exec`, `cp`, `run` et `restart` passent. `KADENZ_ESSAI=1` est refusé quand le portail est ouvert. `COMPOSE_FILE` n'est pas employé : la documentation de Compose ne le dit lu que depuis le fichier `.env` du dossier de travail (https://docs.docker.com/compose/how-tos/environment-variables/envvars/), et un `-f` explicite l'emporte. Le nom du projet vient de `name: kadenz` dans `compose.yaml`.

#### Procédure

Tout se fait en root (`sudo -i`), depuis `/opt/kadenz/source` : les chemins `deploiement/dell/...` y sont relatifs. Raccourcis utilisés plus bas :

```sh
kz()  { /usr/local/sbin/kadenz-compose "$@"; }
kze() { KADENZ_ESSAI=1 /usr/local/sbin/kadenz-compose "$@"; }
```

1. **Relire l'existant** : `/usr/local/sbin/alerte` (forme d'appel ci-dessus) et `/usr/local/sbin/sauvegarde` (fonction `echec`, commentaire « Données à sauvegarder », ligne `restic backup --one-file-system --tag auto` sur deux lignes, au 2026-11-19). Si l'un a changé, ajuster les scripts de la trousse.
2. **Source** dans `/opt/kadenz/source`, au commit voulu. La façon dont le Dell obtient le code reste à la décision de l'utilisateur (plan de découpage des dépôts). Vérifier que l'arbre est propre (`git status --porcelain` vide) et noter `git rev-parse --short=12 HEAD`.
3. **Réglages** : `install -m 755 deploiement/dell/kadenz-compose /usr/local/sbin/kadenz-compose` ; copier `kadenz.env.exemple` dans `/etc/kadenz/kadenz.env` (root, 600), `KADENZ_PUBLIC` vide ; `KADENZ_REV` = le commit noté à l'étape 2, sinon l'étiquette ment ; choisir un sous-réseau libre (`docker network ls`, `docker network inspect`, `ip route`) pour `RESEAU_PORTAIL` et sa première adresse pour `PASSERELLE_PORTAIL`.
4. **Clé d'essai** (jamais la clé de production) : paire Ed25519 neuve, la clé privée dans un fichier réservé à l'uid 10001, la clé publique affichée pour `CLE_PUBLIQUE_ESSAI` ; `CLE_PORTAIL=/srv/kadenz/essai/cle-portail.hex`.

   ```sh
   install -d -m 700 -o 10001 -g 10001 /srv/kadenz/essai
   ( umask 077; d=$(mktemp -d)
     openssl genpkey -algorithm ed25519 -out "$d/essai.pem"
     openssl pkey -in "$d/essai.pem" -outform DER | tail -c 32 | od -An -tx1 | tr -d ' \n' > /srv/kadenz/essai/cle-portail.hex
     openssl pkey -in "$d/essai.pem" -pubout -outform DER | tail -c 32 | od -An -tx1 | tr -d ' \n'; echo
     rm -rf "$d" )
   chown 10001:10001 /srv/kadenz/essai/cle-portail.hex && chmod 400 /srv/kadenz/essai/cle-portail.hex
   ```

   Une dérivation fausse se voit tout de suite : le portail refuse de démarrer avec une clé dont la moitié publique n'est pas celle attendue.
5. **Construction** : `kze build --build-arg CARGO_BUILD_JOBS=1 portail relais`, puis `kz --profile outils build outil-sqlite`. Vérifier les étiquettes : `docker image ls 'kadenz-*'`.
6. **Démarrage** : `kze up -d portail relais` ; `kz ps` doit montrer les deux services `healthy` en moins d'une minute, ports sur `127.0.0.1` seulement (`ss -ltnp | grep -E '4738[12]'`).
7. **Compte éditeur d'essai** : `kz exec -it portail kadenz-portail --dossier /donnees creer-editeur essai-dell "Essai Dell"`, mot de passe de 15 caractères au moins choisi par la personne qui conduit l'essai, jamais noté dans le dépôt.
8. **Console par tunnel** : depuis le PC, `ssh -L 47382:127.0.0.1:47382 dell`, puis `http://localhost:47382` dans le navigateur. Les cookies de la console sont `Secure` avec le préfixe `__Host-` : en HTTP, seul un nom `localhost` peut les recevoir, à vérifier à l'essai. Se connecter, puis relever dans le journal de la console l'adresse notée pour cette connexion : c'est l'adresse vue par le portail (attendue : `PASSERELLE_PORTAIL`).
9. **Sauvegarde** :
   - `install -m 755 deploiement/dell/kadenz-sauvegarde /usr/local/sbin/kadenz-sauvegarde` ;
   - le lancer à la main, puis vérifier `/var/backups/kadenz/portail/portail.sqlite`, `journal.jsonl` (une ligne JSON par entrée) et `/var/backups/kadenz/relais/relais.crt` et `relais.cle` ; un second lancement n'ajoute que les entrées nouvelles du journal ;
   - dans `/usr/local/sbin/sauvegarde`, juste avant la ligne `restic backup`, ajouter `/usr/local/sbin/kadenz-sauvegarde || echec "kadenz-sauvegarde a échoué"`, puis ajouter `/var/backups/kadenz` aux dossiers de la ligne `restic backup` ;
   - essai de restauration de la base : `kz stop portail` ; copier `portail.sqlite` de la sauvegarde dans `/var/backups/kadenz/depot/`, propriétaire 10001 ; `kz run --rm -T --entrypoint sh outil-sqlite -c 'rm -f /donnees/portail.sqlite-wal /donnees/portail.sqlite-shm && cp /depot/portail.sqlite /donnees/portail.sqlite'` ; `kze up -d portail` ; se reconnecter et vérifier la chaîne du journal (`kz exec -T portail kadenz-portail --dossier /donnees journal-verifier`) ;
   - après toute restauration, mettre l'export du journal de côté avant la sauvegarde suivante : `mv /var/backups/kadenz/portail/journal.jsonl /var/backups/kadenz/portail/journal-avant-restauration-$(date +%Y%m%d-%H%M%S).jsonl` ; le crochet repart alors de l'entrée 0 de la base restaurée.
   - si la base restaurée a des entrées archivées (`journal-verifier` affiche « Entrées archivées jusqu'à ... »), repartir de l'export mis de côté, coupé à la dernière entrée de la base restaurée, au lieu d'un export vide : `dernier=$(kz run --rm -T outil-sqlite /donnees/portail.sqlite "SELECT max(id) FROM journal;")`, puis `awk -v m="$dernier" 'match($0, /"id":[0-9]+/) && substr($0, RSTART + 5, RLENGTH - 5) + 0 > m { exit } { print }' <export mis de côté> > /var/backups/kadenz/portail/journal.jsonl` et `chmod 600` ; enfin `kz exec -T portail kadenz-portail --dossier /donnees journal-verifier --archive - < /var/backups/kadenz/portail/journal.jsonl` doit répondre « intacte » (à essayer sur le Dell).
10. **Alerte** :
    - `install -m 755 deploiement/dell/kadenz-sante /usr/local/sbin/kadenz-sante` ; `install -m 644 deploiement/dell/kadenz-sante.service deploiement/dell/kadenz-sante.timer /etc/systemd/system/` ; `systemctl daemon-reload && systemctl enable --now kadenz-sante.timer` ;
    - essai : `kz stop relais`, une alerte « relais en panne » dans les 5 minutes ; `kz start relais`, une alerte de retour ; aucune alerte entre les deux.
11. **Fin de l'essai** : `kze down` (les volumes restent ; `kze down -v` les efface), retirer `/srv/kadenz/essai` et l'image `kadenz-portail:essai-*`.

#### Cloudflare Tunnel : sur accord de l'utilisateur

Domaine acheté par l'utilisateur chez Cloudflare le 2026-11-19 (zone DNS chez Cloudflare), transmis par kadenz-51. Nom public proposé pour la console : `licences.mcarre.dev` ; `relais.mcarre.dev` réservé pour plus tard. Rien n'est lancé ; c'est la voie retenue à la place de Funnel.

Ce que fait la surcouche `compose.public.yaml` :

- Service `tunnel` : image officielle `cloudflare/cloudflared:2026.10.7`, épinglée par empreinte dans `kadenz.env.exemple` (relevée sur le Dell par kadenz-51 le 2026-11-19 : publiée le 2026-10-30, utilisateur 65532:65532, point d'entrée `cloudflared`, `--token-file` présent dans `tunnel run --help`), même durcissement que les autres conteneurs, **aucun port publié**. Il ne fait que sortir vers Cloudflare ; rien ne touche le réseau de l'hôte, ni Tailscale, ni ufw, ni sshd.
- Tunnel géré à distance : la route du nom public vers `http://portail:47382` est réglée dans le compte Cloudflare, pas dans le dépôt. cloudflared lit son jeton dans un fichier (`--token-file`, variable `TUNNEL_TOKEN_FILE`), monté en lecture seule depuis `JETON_TUNNEL` ; `--no-autoupdate` ; métriques sur `127.0.0.1:20241` dans le conteneur seulement. Source : page « Tunnel run parameters », https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/configure-tunnels/run-parameters/ (lue le 2026-11-19). En conteneur, le serveur de métriques écoute par défaut sur toutes les adresses (page « Tunnel metrics », https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/monitor-tunnels/metrics/), d'où l'adresse imposée.
- Réseaux : `console` (interne, sous-réseau `RESEAU_CONSOLE`) entre le portail et cloudflared, à adresse fixe `IP_TUNNEL` ; `sortie` (pont ordinaire) pour la sortie de cloudflared. Le portail garde sa publication sur `127.0.0.1:47382` pour l'administration par `ssh -L` à travers Tailscale.
- `--mandataire` reçoit `IP_TUNNEL`, et `--entete-client cf-connecting-ip` : derrière Cloudflare, l'adresse du client est dans `CF-Connecting-IP` et non dans `X-Forwarded-For` (vérifié par OpenCode dans la documentation de Cloudflare le 2026-11-19, `docs/securite/echanges-opencode.md`). Le portail ne lit cet en-tête que pour les connexions venues de cloudflared. Une connexion par `127.0.0.1` arrive de la passerelle du réseau `portail`, qui n'est pas le mandataire : son adresse reste celle du pair.
- Le relais en TCP brut n'y passe pas : aucune offre gratuite de Cloudflare ne porte son protocole sans logiciel côté client (Spectrum en transmission directe relève de l'offre Enterprise, prix non publié ; relevé d'OpenCode). Aucune ouverture de port n'étant possible sur la box (utilisateur, 2026-11-19), le relais passe en WebSocket (`docs/spec-relais-websocket.md`) : la surcouche donne au relais `--ecoute-ws 0.0.0.0:47383 --mandataire IP_TUNNEL_RELAIS`, et le place avec cloudflared sur le réseau interne `tunnel-relais` (sous-réseau `RESEAU_TUNNEL_RELAIS`, distinct de `console`, pour que le portail et le relais ne se voient pas). L'écoute TCP 47381 reste publiée sur `127.0.0.1` seulement. Exige une image du relais construite depuis 453c3f5 ou plus récent : une image plus ancienne refuse `--ecoute-ws` et le relais ne démarre pas, d'où l'ordre : construire, poser `RESEAU_TUNNEL_RELAIS` et `IP_TUNNEL_RELAIS`, puis seulement installer la nouvelle surcouche et relancer.
- Route du relais, dans le compte de l'utilisateur (sur son accord, quand les postes sauront s'y connecter) : onglet **Routes** du tunnel, **Published application**, nom `relais.mcarre.dev`, Service URL `http://relais:47383`. Dans **Security > Bots**, si Super Bot Fight Mode est actif, « Definitely Automated » doit rester sur Allow, sinon l'ouverture WebSocket échoue (`websocket: bad handshake`, relevé d'OpenCode : https://developers.cloudflare.com/waf/feature-interoperability/).
- Sonde : aucune dans le conteneur, l'image n'ayant ni shell ni curl et aucune commande de santé n'étant vérifiée dans la documentation. `kadenz-sante` surveille l'état du conteneur, et l'état du tunnel se lit dans le tableau de bord de l'utilisateur.
- Sauvegarde : rien pour cloudflared. Le jeton est révocable et se régénère dans le compte ; il n'entre pas dans restic.

À relever à l'essai : la fusion des réseaux du portail (`docker compose ... config` doit montrer `portail` et `console`).

Étapes de **l'utilisateur, dans son compte Cloudflare** (libellés de la page « Create a tunnel (dashboard) », https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/get-started/create-remote-tunnel/, relevés par kadenz-51 le 2026-11-19) :

1. **Networking > Tunnels** : créer un tunnel de type cloudflared, nommé par exemple `kadenz-dell`.
2. Copier le jeton du tunnel et le déposer lui-même sur le Dell dans `/srv/kadenz/tunnel/jeton` ; le jeton collé depuis la commande du tableau de bord arrive avec une fin de ligne, à retirer avant de changer le propriétaire (relevé par kadenz-51) : `tr -d '\r\n' < jeton > jeton.net && mv jeton.net jeton`, puis propriétaire 65532 (l'utilisateur de l'image) et mode 400 ; jamais dans un message, jamais dans le dépôt. Ignorer la commande d'installation proposée par la page : le conteneur de la trousse en tient lieu.
3. Onglet **Routes**, **Published application** : nom `licences.mcarre.dev`, service `http://portail:47382`.

Étapes sur le Dell, en root depuis la source (fonctions `kz`, `kze` de la procédure ci-dessus) :

1. Réglages : `IMAGE_CLOUDFLARED` (valeur de `kadenz.env.exemple` ; pour une version plus récente, `docker pull cloudflare/cloudflared:<version>` puis `docker image inspect --format '{{index .RepoDigests 0}}'`, une version publiée depuis au moins deux semaines), `RESEAU_CONSOLE` (sous-réseau libre), `IP_TUNNEL` (dans ce sous-réseau, pas la première adresse), `JETON_TUNNEL`.
2. Poser `KADENZ_PUBLIC=1` dans `/etc/kadenz/kadenz.env`, puis vérifier la configuration fusionnée : `kz config` ne doit montrer aucun `ports:` pour `tunnel`, et les réseaux `portail` et `console` pour le portail.
3. Démarrer, **sur accord de l'utilisateur** : d'abord retirer le conteneur provisoire posé par kadenz-51 le 2026-11-19 (`docker run` hors Compose, même image et même jeton, aucune route publiée) : `docker rm -f kadenz-tunnel` ; puis `kz up -d portail tunnel`. Le portail est recréé avec le nouveau `--mandataire`. Le conteneur provisoire n'a pas les étiquettes de Compose : `kadenz-sante` ne le surveille pas.
4. Vérifier : tunnel « Healthy » dans le tableau de bord de l'utilisateur ; `https://licences.mcarre.dev/` ouvre la page de connexion ; les cookies `__Host-` et `Secure` passent (HTTPS terminé chez Cloudflare) ; l'adresse notée au journal de la console pour cette connexion doit être l'adresse publique du poste qui se connecte, et non `IP_TUNNEL`.
5. Retour en arrière : `kz stop tunnel` suffit à couper l'accès (ou révocation du jeton dans le compte), `KADENZ_PUBLIC=1` restant posé. Pour revenir tout à fait à la console locale : `kz rm -s -f tunnel`, puis `KADENZ_PUBLIC` remis à vide dans `/etc/kadenz/kadenz.env`, puis `kz up -d portail` (retour au `--mandataire` de la passerelle). Dans l'autre ordre, le garde-fou refuse le `up`.
6. Dell déjà ouvert avant `kadenz-compose` (2026-11-19) : installer `kadenz-compose` et le nouveau `kadenz-sauvegarde` dans `/usr/local/sbin`, ajouter `KADENZ_PUBLIC=1` aux réglages ; contrôle sans rien recréer : `kz config --services` liste `tunnel`, `kz ps` ne signale plus d'orphelin, et `docker inspect -f '{{index .Config.Labels "com.docker.compose.project"}}' kadenz-portail-1` rend `kadenz`.

Jamais avec `compose.essai.yaml` (image d'essai, clé d'essai), sauf essai décidé par l'utilisateur. Aucune étape ne modifie l'accès au Dell ; une étape qui le ferait serait à faire **seulement avec l'utilisateur sur place**.

##### Option : Cloudflare Access devant la console (décision de l'utilisateur)

Une porte de plus avant la page de connexion du portail, proposée par kadenz-51. Ce que dit la documentation, lue le 2026-11-19 :

| Point | Ce que dit Cloudflare | Source |
|---|---|---|
| Plan gratuit | « Best for teams under 50 users », « 50 user limit » | https://www.cloudflare.com/plans/zero-trust-services/ |
| Code par courriel | Access envoie un code à usage unique aux adresses autorisées par une politique ; il expire 10 minutes après la demande, sert une fois, et n'est envoyé que si l'adresse est autorisée | https://developers.cloudflare.com/cloudflare-one/integrations/identity-providers/one-time-pin/ |
| Exception d'un chemin | « Bypass is typically used to enable applications that require specific endpoints to be public » ; Cloudflare ne recommande pas Bypass pour un accès permanent direct à une application interne | https://developers.cloudflare.com/cloudflare-one/access-controls/policies/ |
| Chemin le plus précis | « the more specific rule takes precedence », sans héritage de la règle du chemin parent | https://developers.cloudflare.com/cloudflare-one/access-controls/policies/app-paths/ |

Montage proposé, entièrement dans le compte de l'utilisateur :

1. Application Access sur `licences.mcarre.dev` (chemin vide), politique Allow limitée à l'adresse ou aux adresses de l'éditeur, connexion par code à usage unique.
2. Seconde application sur `licences.mcarre.dev/api/activation`, politique Bypass « Everyone » : les postes clients appellent cette route sans session ni navigateur (`crates/portail/src/web.rs`) et seraient refusés sans elle.
3. Vérifier que la page de connexion demande le code d'Access, puis qu'une activation depuis un KadenzAdmin d'essai passe toujours.

Limites : l'activation reste exposée comme aujourd'hui, protégée par le frein et le journal du portail ; Access ajoute une dépendance à Cloudflare pour se connecter à la console (l'administration par `ssh -L` à travers Tailscale reste possible). Rien dans la trousse ne change avec cette option.

#### Ouverture par Funnel : sur accord de l'utilisateur

Voie remplacée par Cloudflare Tunnel (ci-dessus) depuis l'achat du domaine ; gardée pour mémoire.


Étape écrite, **à ne faire que sur accord explicite de l'utilisateur** : elle ouvre la console à Internet et demande un changement dans son compte Tailscale. Jamais avec l'image d'essai ni la clé d'essai.

1. Prérequis côté compte (utilisateur) : MagicDNS, certificats HTTPS du tailnet, attribut `funnel` pour le Dell dans la politique du tailnet.
2. Portail en service avec la clé de production (fichier mis en place par l'utilisateur), image release.
3. `tailscale funnel --bg --https=443 http://127.0.0.1:47382` ; vérifier avec `tailscale funnel status`. Syntaxe à confirmer par `tailscale funnel --help` sur le Dell.
4. Se connecter depuis une adresse extérieure et relever dans le journal l'adresse notée : l'adresse réelle du client si tailscaled transmet `X-Forwarded-For`, sinon la passerelle, et alors le frein de connexion compte toutes les requêtes comme une seule adresse.
5. Retour en arrière : `tailscale funnel --https=443 off`.
