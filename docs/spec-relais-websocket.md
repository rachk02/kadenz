# Kadenz phase 2 - Relais par WebSocket (Cloudflare Tunnel)

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-19 | Conception proposée par kadenz-e6, à la demande de kadenz-dd, après la décision de l'utilisateur (« on y va ») : faire passer le relais par le Cloudflare Tunnel, aucun port ne pouvant être ouvert sur la box. Limites de Cloudflare vérifiées par OpenCode dans la documentation officielle. Rien n'est codé. |
| 0.2 | 2026-11-20 | Lot 1 codé dans `kadenz-relais` (kadenz-e6, accord de kadenz-dd) : `ws.rs` (adresse, pont local, passerelle), `tcp_vers`, options `--ecoute-ws` et `--mandataire`, essais. Adresse sans chemin (`wss://hôte[:port]`). Racines du magasin du système retenues. Versions des bibliothèques relevées. |
| 0.3 | 2026-11-20 | Lot 2 (kadenz-e6) : `regler_relais` accepte `wss://`, tirage de plus ou moins 20 % sur l'attente de reprise du serveur, essai de bout en bout par `ws://` dans `crates/serveur/tests/distance.rs`. Corrections du lot 1 après relecture : TLS 1.2 et 1.3 sur le tronçon Cloudflare, en-tête `Host` sans le port par défaut, pont local bloquant. Section 7 vérifiée ; risques et essais restants listés. Libellé et aide de KadenzAdmin. Ouverture TLS vers Cloudflare essayée une fois (`licences.mcarre.dev`) : réponse HTTP du portail, aucune erreur TLS. |
| 0.4 | 2026-11-20 | Quatre essais restants faits (kadenz-e6, à la demande de kadenz-dd) : coupure de toutes les WebSocket en pleine série d'écritures, message de plus de 64 Kio des deux côtés, frein par poste derrière le mandataire (avec contre-épreuve), ping et silence. Délais des WebSocket réglables (`DelaisWs`, `Config.delais_ws`) pour les essais. Code 303 de l'essai vers Cloudflare relevé par kadenz-dd (curl). |
| 0.5 | 2026-11-20 | Relais en service sur le Dell derrière `wss://relais.mcarre.dev` (kadenz-dd, accord de l'utilisateur). Essai en ligne `relais_en_service_par_cloudflare` (`#[ignore]`, empreinte passée par `KADENZ_EMPREINTE_RELAIS`) : défi du relais reçu en 1,07 s par la poignée de main TLS épinglée à travers Cloudflare, et autre empreinte refusée (certificat différent). Rien d'enregistré. Un premier lancement avait échoué sur une réponse DNS négative gardée en cache par la box. |

Statut : **en service** depuis le 2026-11-20 (relais du Dell, `wss://relais.mcarre.dev`) ; **lots 1 et 2 codés** (relais, pont, passerelle, réglage dans KadenzAdmin, reconnexion) ; trousse du Dell faite par kadenz-dd (cc6ce9a), rien n'est déployé. Complète la section 7 de `docs/spec-synchronisation.md` (accès à distance par le relais) sans en changer le protocole.

## 1. Principe

- Le relais accepte, en plus de son port TCP 47381, des connexions WebSocket sur un second port (47383 proposé), servi en HTTP simple sur le réseau Docker. cloudflared y envoie les connexions de `wss://relais.mcarre.dev` (route « Published application » du tunnel, service `http://relais:47383`).
- Chaque connexion WebSocket porte exactement le flux d'octets d'une connexion TCP au relais, découpé en messages binaires. Rien ne change au-dessus : trames en clair du poste, connexion de contrôle en TLS 1.3 épinglée (SNI `kadenz-relais`), sessions TLS de bout en bout entre poste et serveur.
- Cloudflare termine le TLS de `wss://` mais ne voit, dans la WebSocket, que ce que voit aujourd'hui un observateur du réseau : les trames en clair d'ouverture d'un poste (identifiant d'entreprise, certificat du poste, signature du défi), puis du chiffré. La connexion de contrôle, qui porte la licence, reste chiffrée de bout en bout avec le relais.

## 2. Qui se connecte au relais

Recensé dans le code : toutes les connexions au relais passent par une seule fonction, `kadenz_relais::tcp_vers(adresse, delai)` (`crates/relais/src/lib.rs`).

| Côté | Fonction | Programme | Connexion |
|---|---|---|---|
| Serveur de l'entreprise | `controle::canal_epingle` (contrôle), `controle::ouvrir_session` (une par session) | service du serveur (`crates/serveur/src/distance.rs`) | sortante, durable pour le contrôle |
| Serveur | `controle::verifier_sante` | sonde `kadenz-relais sante` (dans le conteneur, en TCP local) | courte |
| Poste distant | `poste::ouvrir` | poste (`crates/client`, `Client::connecter_par_relais`) | une par session |

KadenzAdmin ne se connecte pas au relais : il ne fait que régler l'adresse et l'empreinte dans la base du serveur (`crates/serveur/src/admin.rs`, `regler_relais`). Le poste apprend l'adresse du serveur dans `Bienvenue`.

## 3. Où se branche le transport

- **Sous les trames et sous le TLS du relais**, dans `tcp_vers`. Une adresse `hôte:port` garde le TCP direct, inchangé. Une adresse `wss://hôte[:port]` (sans chemin : la passerelle ne sert que `/`) ouvre la WebSocket, puis rend un `TcpStream` local relié à elle (voir ci-dessous). `poste::ouvrir`, `ouvrir_session` et `canal_epingle` continuent de recevoir un `TcpStream` et ne changent pas, pas plus que leurs appelants : délais, `try_clone`, `shutdown`, épinglage.
- **Pont local côté client** : `tcp_vers` ouvre une écoute sur `127.0.0.1:0`, se connecte à elle, et confie l'autre bout à un fil qui fait la copie dans les deux sens entre cette connexion locale et la WebSocket. Le pont accepte une seule connexion, celle qu'il vient d'ouvrir (adresse et port vérifiés), puis ferme son écoute. Fermeture d'un côté : fermeture de l'autre.
- **Passerelle côté relais**, dans le même programme `kadenz-relais` : pour chaque WebSocket acceptée, une connexion TCP vers le port 47381 du relais lui-même (`127.0.0.1`), et la même copie. Le relais ne voit qu'une connexion de plus, avec son frein, ses limites et ses délais ; son code de mise en relation ne change pas.
- Alternative écartée : rendre le cœur du relais générique sur le type de flux (aujourd'hui `TcpStream` partout, avec `peek`, `try_clone` et des délais de lecture) ; plus de code touché et un risque de régression sur le chemin TCP.

## 4. Bibliothèques

- Aucune bibliothèque WebSocket dans l'arbre aujourd'hui. `tokio` y est déjà (1.54.1, par le portail et axum).
- **Retenu** : `tokio-tungstenite` 0.30.0 (fonction `handshake` seule) avec `tokio` 1.54.1 : exécuteur à un fil pour chaque pont du client, deux fils pour la passerelle du relais. La raison est la copie en duplex intégral : un flux TLS ou WebSocket bloquant ne se partage pas entre deux fils, alors qu'un flux asynchrone se sépare en lecture et écriture.
- TLS vers Cloudflare côté client : `tokio-rustls` 0.26.6 (fournisseur ring, comme le reste de l'arbre, sans aws-lc), versions par défaut de rustls (TLS 1.2 et 1.3 : un proxy d'inspection d'entreprise ne parle souvent que 1.2 ; la protection reste le TLS 1.3 épinglé du relais, à l'intérieur), avec les racines du magasin du système (`rustls-native-certs` 0.8.5), pour suivre les autorités ajoutées par une entreprise (proxy d'inspection TLS) ; l'épinglage du relais, à l'intérieur, reste la vraie protection. Retenu au codage du lot 1 plutôt que les racines de Mozilla intégrées (`webpki-roots`).
- `futures-util` 0.3.34 (`sink`, `std`) pour séparer la WebSocket en envoi et réception.
- Taille ajoutée aux programmes Windows (poste, service) : à mesurer à la compilation.

## 5. Adresse du client et frein

- Toutes les WebSocket arrivent de cloudflared : sans précaution, tous les postes auraient la même adresse pour le frein du relais.
- La passerelle lit `CF-Connecting-IP` sur la requête d'ouverture (101), et seulement si la connexion vient de l'adresse fixe de cloudflared (option `--mandataire <adresse>`, comme pour le portail). Sinon, adresse du pair.
- Transmission au cœur du relais : la passerelle lie sa connexion locale à `127.0.0.1:0` avant de se connecter (`tokio::net::TcpSocket`), note « port local vers adresse du client » dans une table partagée, puis se connecte. À l'acceptation, une connexion venue de `127.0.0.1` dont le port est dans la table prend l'adresse notée pour le frein (et la retire de la table) ; sans entrée (sonde de santé), `127.0.0.1`. La table ne sert qu'à l'intérieur du processus : aucun en-tête ni protocole PROXY à croire sur le réseau.

## 6. Délais et limites (figés avec le retour d'OpenCode)

- **Ping WebSocket toutes les 30 secondes**, des deux côtés (pont du client et passerelle du relais), pour l'inactivité de Cloudflare, dont la valeur n'est pas publiée pour le plan gratuit. Les pings ne passent pas dans le flux : le délai d'inactivité des sessions du relais (600 secondes) garde son sens. Pas de pong pendant 90 secondes : connexion fermée.
- Le battement de la connexion de contrôle (15 secondes, silence de 45 secondes) reste ; il suffit déjà pour elle.
- **Messages binaires de 16 Kio au plus** à l'envoi, comme le tampon de copie du relais ; à la réception, message refusé au-delà de 64 Kio et trame au-delà de 64 Kio (réglages de `tungstenite`). Les messages texte sont refusés.
- Ouverture de la WebSocket : 10 secondes pour la poignée de main HTTP ; requête d'ouverture limitée au chemin `/` (404 sinon) et à la méthode GET.
- Limites du relais inchangées (4096 connexions, 256 attentes, 64 sessions reliées par entreprise) : chaque WebSocket compte comme une connexion. La passerelle refuse au-delà de 4096 WebSocket ouvertes.
- **Poignées de main en cours** : 512 au plus, places distinctes des 4096 sessions (`OUVERTURES_EN_COURS`). Une inondation de clients qui ouvrent le TCP sans envoyer leur requête HTTP épuise ces 512 places, puis se fait couper à 10 secondes (`OUVERTURE`), sans priver les sessions établies. Les refus de rythme et de poignée de main sont comptés et résumés en une ligne de journal par minute au plus (`passerelle refus_connexions ...`).
- **Première trame** : côté passerelle, la première trame binaire du pair doit arriver dans les 20 secondes qui suivent la poignée de main (`PREMIERE_TRAME`), sinon la connexion est coupée ; le silence ordinaire ne coupe qu'à 90 secondes. Le relais parle le premier (défi), donc une WebSocket qui n'envoie rien n'est pas un poste : elle n'occupe pas une place deux minutes. Le pont local (poste et serveur) ne subit pas ce délai, le relais y parlant aussi le premier.

## 7. Reconnexion

- **Connexion de contrôle** : la reprise existe déjà (`distance.rs`, attente de 5 secondes doublée jusqu'à 5 minutes). À ajouter : un tirage aléatoire de plus ou moins 20 % sur chaque attente, pour que les serveurs coupés ensemble par un déploiement de Cloudflare ne reviennent pas tous à la même seconde. La reprise d'état est faite par le protocole lui-même : réenregistrement avec la licence et la liste des postes autorisés.
- Après une coupure, l'ancienne connexion de contrôle peut rester vue par le relais jusqu'à 45 secondes (silence). **Vérifié** : un enregistrement avec le même certificat de serveur remplace l'ancien et coupe sa connexion (`relais.rs`, `controle`) ; `deja_enregistree` ne vise qu'un autre certificat et ne gêne donc pas la reprise. L'essai `poste_distant_par_websocket` coupe et rallume le serveur.
- **Sessions d'un poste** : une coupure ferme la session, comme une coupure du réseau aujourd'hui ; le poste rouvre une session au prochain échange, et la synchronisation reprend où elle en était (opérations gardées et rejouées par le poste). Le même tirage aléatoire s'applique à ses attentes. À vérifier par un essai de coupure au milieu d'une synchronisation.

## 8. Compatibilité

- Le TCP direct reste : adresse `hôte:port`, pour un relais joignable sans tunnel (VPS, réseau local, essais).
- Format de l'adresse : `regler_relais` (KadenzAdmin) accepte en plus `wss://hôte[:port]`, sans chemin ni espace, port de 1 à 65535, 443 par défaut. `ws://` sans chiffrement est refusé, sauf vers `127.0.0.1` et `localhost` pour les essais et le banc.
- Postes déjà installés (1.0.0) : ils reçoivent l'adresse dans `Bienvenue` et n'ont que le TCP. Une adresse `wss://` leur donne « relais injoignable », sans autre effet ; le bureau reste essayé en premier. L'accès à distance n'est pas en service aujourd'hui (port fermé), aucun usage n'est donc cassé. Il faut la nouvelle version du poste et du serveur pour passer par le tunnel.
- Le protocole du relais garde sa version (`VERSION = 1`) : le transport ne change pas les trames.
- Demi-fermeture : une fin de lecture d'un côté du pont ferme toute la WebSocket. Seul `fermer` du serveur (`crates/serveur/src/lib.rs`) ferme en écriture puis lit encore ; ce qu'il lit alors est jeté, rien n'est donc perdu par ce chemin.
- KadenzAdmin : libellé « Adresse du relais (wss://hôte ou hôte:port) », et l'aide rappelle que `ws://` n'est permis que vers la machine elle-même (décision de kadenz-dd et kadenz-e6, six langues).

## 9. Trousse du Dell (kadenz-dd)

- `compose.public.yaml` : le relais rejoint un réseau partagé avec cloudflared ; réseau propre au relais proposé (`tunnel-relais`), distinct de `console`, pour que le portail et le relais ne se voient pas ; adresse fixe de cloudflared sur ce réseau, donnée à `--mandataire`.
- Commande du relais : `--ecoute-ws 0.0.0.0:47383 --mandataire <adresse de cloudflared>` ; la publication `127.0.0.1:47381` reste (sonde, administration).
- Compte Cloudflare de l'utilisateur (à lui de le faire, préparé par kadenz-dd) : route `relais.mcarre.dev` vers `http://relais:47383` ; Bot Fight Mode désactivé ou, avec Super Bot Fight Mode, « Definitely Automated » sur Allow pour ce nom (nos clients ne sont pas des navigateurs) ; pas de règle de limitation de débit qui coupe les reconnexions d'un poste. La passerelle borne elle-même les ouvertures à 1024 immédiates puis 256 par seconde, et à 512 poignées de main en cours, avant de consommer une des 4096 places ; une limitation complémentaire côté Cloudflare reste recommandée pour les flots répartis. L'adresse du client reste un en-tête cru sur la seule adresse IP du mandataire : tout processus qui obtiendrait cette IP sur le réseau `tunnel-relais` pourrait contourner le frein par adresse.
- Sonde de santé, trois niveaux. Dans le conteneur, `HEALTHCHECK` appelle `kadenz-relais sante --dossier /donnees`, qui ne voit que le port TCP du relais. La surcouche publique demande en plus `kadenz-relais sante --ws 127.0.0.1:47383` (`deploiement/dell/compose.public.yaml`), qui ouvre une WebSocket locale et vérifie la poignée de main (`crates/relais/src/ws.rs`, `verifier_sante`). Enfin `kadenz-sante` sonde les noms publics quand le tunnel est en service : `https://licences.mcarre.dev/` attendu en 303, `https://relais.mcarre.dev/` attendu en 101 sur une ouverture WebSocket, alerte au changement d'état. Vérifié à la main le 2026-11-22 05:32 : 303, 101, et 502 sur un GET simple du relais.
- Conditions de Cloudflare (OpenCode) : rien n'interdit ce protocole ; la clause 2.2.1(j) (pas de VPN ni de proxy général offert à des tiers) est respectée, le relais ne porte que le protocole Kadenz pour nos clients.

## 10. Release

- Changements dans `kadenz-relais` (bibliothèque partagée) : programme du relais (image Docker du Dell), service du serveur et poste. KadenzAdmin seulement pour la validation de l'adresse.
- Une nouvelle version du serveur et du poste est nécessaire ; son numéro et sa date restent à l'utilisateur (la 1.0.1 attend aujourd'hui le monoposte).
- L'image du relais peut être déployée avant : sans route du tunnel, la passerelle n'est pas joignable et le TCP reste comme aujourd'hui.

## 11. Essais prévus

- `regler_relais` : adresses `wss://` admises et refusées, `ws://` limité au local.
- Pont et passerelle : relais d'essai avec écoute WebSocket locale ; enregistrement du serveur, session d'un poste et synchronisation complète par `ws://127.0.0.1`, comme `crates/serveur/tests/distance.rs` en TCP.
- Frein : deux postes derrière la passerelle avec deux `CF-Connecting-IP` différents, comptés séparément ; en-tête ignoré quand la connexion ne vient pas du mandataire.
- Coupure de la WebSocket au milieu d'une session et de la connexion de contrôle : reprise, `deja_enregistree` passager.
- Ping : pas de coupure sur une session muette de plus de 2 minutes.
- Limites : message texte refusé, message de plus de 64 Kio refusé, poignée de main sans première trame coupée au délai de 20 secondes, poignées de main en cours bornées sans prendre les places durables (`premiere_trame_absente_coupee`, `limites_coherentes`).
- De bout en bout sur le Dell, après la route du tunnel : poste hors du réseau, par `wss://relais.mcarre.dev`.

Faits (lots 1 et 2) : adresses admises et refusées (relais et `regler_relais`) ; pont local ; passerelle (en-tête cru seulement du mandataire, chemin `/` seul, message texte qui ferme) ; santé, enregistrement et sessions par `ws://127.0.0.1` avec 300 Kio dans les deux sens ; serveur réel coupé puis rallumé par `ws://` ; tirage de l'attente ; essai `#[ignore]` d'ouverture TLS vers Cloudflare (`ws::essais_cloudflare`, à lancer à la main).

Faits ensuite (0.4) :
- coupure de toutes les WebSocket (contrôle du serveur et session du poste) pendant la onzième de trente écritures d'un poste distant, par un mandataire coupable devant la passerelle : serveur réenregistré, poste revenu par le relais, file vidée, chacune des trente écritures présente une fois et une seule au serveur (`crates/poste/tests/synchroniseur.rs`, `coupure_des_websocket_en_pleine_session`) ;
- message de 70 Kio : la passerelle ferme sans rien transmettre au relais et sert les suivantes ; le pont du client ferme sa connexion locale sans rien rendre ; aucune panique ;
- frein : 220 connexions d'un poste en une seconde environ, une partie refusée ; un second poste derrière le même mandataire, ouvert aussitôt, passe. Contre-épreuve : sans `--mandataire`, le second est refusé avec le premier ;
- ping et silence, délais raccourcis (100 ms et 400 ms) : un pair qui répond aux pings garde une session muette ouverte 1,5 seconde ; un pair muet est coupé entre 400 ms et 3 secondes.
- durcissement du 2026-11-22 (OpenCode, à la demande de l'utilisateur) : 512 poignées de main en cours au plus, séparées des 4096 places durables, et première trame binaire exigée en 20 secondes côté passerelle. Deux essais ajoutés : `premiere_trame_absente_coupee` (WebSocket muette coupée au délai) et `limites_coherentes` (constantes compatibles). Le refus du relais est désormais un `ErreurClient::RelaisRefuse` muni du motif, vérifié par `poste_distant_par_le_relais` (`crates/serveur/tests/distance.rs`) au lieu d'être confondu avec une adresse injoignable.

Reste : la taille ajoutée aux programmes Windows, à mesurer avec une construction optimisée (avec l'image du Dell, décision de kadenz-dd).

## 12. Risques

- Racines TLS sous Windows : `rustls-native-certs` lit les autorités déjà présentes dans le magasin ; Windows n'y ajoute une racine manquante qu'à la demande d'un programme qui passe par CryptoAPI. Un poste neuf qui n'a jamais vu la racine de Cloudflare pourrait la refuser. Parade si le cas se présente : `rustls-platform-verifier` (vérification par Windows lui-même), à décider avec kadenz-dd.
