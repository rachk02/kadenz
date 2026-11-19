# Kadenz phase 2 - Relais par WebSocket (Cloudflare Tunnel)

| Version | Date | Changement |
|---|---|---|
| 0.1 | 2026-11-19 | Conception proposée par kadenz-e6, à la demande de kadenz-dd, après la décision de l'utilisateur (« on y va ») : faire passer le relais par le Cloudflare Tunnel, aucun port ne pouvant être ouvert sur la box. Limites de Cloudflare vérifiées par OpenCode dans la documentation officielle. Rien n'est codé. |

Statut : **proposé**. Complète la section 7 de `docs/spec-synchronisation.md` (accès à distance par le relais) sans en changer le protocole.

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

- **Sous les trames et sous le TLS du relais**, dans `tcp_vers`. Une adresse `hôte:port` garde le TCP direct, inchangé. Une adresse `wss://hôte[:port][/chemin]` ouvre la WebSocket, puis rend un `TcpStream` local relié à elle (voir ci-dessous). `poste::ouvrir`, `ouvrir_session` et `canal_epingle` continuent de recevoir un `TcpStream` et ne changent pas, pas plus que leurs appelants : délais, `try_clone`, `shutdown`, épinglage.
- **Pont local côté client** : `tcp_vers` ouvre une écoute sur `127.0.0.1:0`, se connecte à elle, et confie l'autre bout à un fil qui fait la copie dans les deux sens entre cette connexion locale et la WebSocket. Le pont accepte une seule connexion, celle qu'il vient d'ouvrir (adresse et port vérifiés), puis ferme son écoute. Fermeture d'un côté : fermeture de l'autre.
- **Passerelle côté relais**, dans le même programme `kadenz-relais` : pour chaque WebSocket acceptée, une connexion TCP vers le port 47381 du relais lui-même (`127.0.0.1`), et la même copie. Le relais ne voit qu'une connexion de plus, avec son frein, ses limites et ses délais ; son code de mise en relation ne change pas.
- Alternative écartée : rendre le cœur du relais générique sur le type de flux (aujourd'hui `TcpStream` partout, avec `peek`, `try_clone` et des délais de lecture) ; plus de code touché et un risque de régression sur le chemin TCP.

## 4. Bibliothèques

- Aucune bibliothèque WebSocket dans l'arbre aujourd'hui. `tokio` y est déjà (1.54.1, par le portail et axum).
- **Proposé** : `tokio-tungstenite` (et `tungstenite`) avec `tokio` en exécuteur à un fil, sur les deux côtés. La raison est la copie en duplex intégral : un flux TLS ou WebSocket bloquant ne se partage pas entre deux fils, alors qu'un flux asynchrone se sépare en lecture et écriture.
- TLS vers Cloudflare côté client : `tokio-rustls` avec les racines du magasin de Windows (`rustls-native-certs`), pour suivre les autorités ajoutées par une entreprise (proxy d'inspection TLS) ; l'épinglage du relais, à l'intérieur, reste la vraie protection. À trancher : racines de Mozilla intégrées (`webpki-roots`), plus prévisibles mais aveugles à ces proxys.
- Versions exactes et licences relevées au moment du codage ; ajout au `Cargo.lock` vérifié par `cargo deny` s'il est en place.
- Taille ajoutée aux programmes Windows (poste, service) : à mesurer à la compilation.

## 5. Adresse du client et frein

- Toutes les WebSocket arrivent de cloudflared : sans précaution, tous les postes auraient la même adresse pour le frein du relais.
- La passerelle lit `CF-Connecting-IP` sur la requête d'ouverture (101), et seulement si la connexion vient de l'adresse fixe de cloudflared (option `--mandataire <adresse>`, comme pour le portail). Sinon, adresse du pair.
- Transmission au cœur du relais : la passerelle lie sa connexion locale à `127.0.0.1:0` avant de se connecter (`tokio::net::TcpSocket`), note « port local vers adresse du client » dans une table partagée, puis se connecte. À l'acceptation, une connexion venue de `127.0.0.1` dont le port est dans la table prend l'adresse notée pour le frein (et la retire de la table) ; sans entrée (sonde de santé), `127.0.0.1`. La table ne sert qu'à l'intérieur du processus : aucun en-tête ni protocole PROXY à croire sur le réseau.

## 6. Délais et limites (figés avec le retour d'OpenCode)

- **Ping WebSocket toutes les 30 secondes**, des deux côtés (pont du client et passerelle du relais), pour l'inactivité de Cloudflare, dont la valeur n'est pas publiée pour le plan gratuit. Les pings ne passent pas dans le flux : le délai d'inactivité des sessions du relais (600 secondes) garde son sens. Pas de pong pendant 90 secondes : connexion fermée.
- Le battement de la connexion de contrôle (15 secondes, silence de 45 secondes) reste ; il suffit déjà pour elle.
- **Messages binaires de 16 Kio au plus** à l'envoi, comme le tampon de copie du relais ; à la réception, message refusé au-delà de 64 Kio et trame au-delà de 64 Kio (réglages de `tungstenite`). Les messages texte sont refusés.
- Ouverture de la WebSocket : 10 secondes pour la poignée de main HTTP ; requête d'ouverture limitée au chemin réglé et à la méthode GET.
- Limites du relais inchangées (4096 connexions, 256 attentes, 64 sessions reliées par entreprise) : chaque WebSocket compte comme une connexion. La passerelle refuse au-delà de 4096 WebSocket ouvertes.

## 7. Reconnexion

- **Connexion de contrôle** : la reprise existe déjà (`distance.rs`, attente de 5 secondes doublée jusqu'à 5 minutes). À ajouter : un tirage aléatoire de plus ou moins 20 % sur chaque attente, pour que les serveurs coupés ensemble par un déploiement de Cloudflare ne reviennent pas tous à la même seconde. La reprise d'état est faite par le protocole lui-même : réenregistrement avec la licence et la liste des postes autorisés.
- Après une coupure, l'ancienne connexion de contrôle peut rester vue par le relais jusqu'à 45 secondes (silence) : le refus `deja_enregistree` doit être traité comme passager, avec la même attente, et non comme une erreur de réglage. À vérifier dans `distance.rs` et à couvrir par un essai.
- **Sessions d'un poste** : une coupure ferme la session, comme une coupure du réseau aujourd'hui ; le poste rouvre une session au prochain échange, et la synchronisation reprend où elle en était (opérations gardées et rejouées par le poste). Le même tirage aléatoire s'applique à ses attentes. À vérifier par un essai de coupure au milieu d'une synchronisation.

## 8. Compatibilité

- Le TCP direct reste : adresse `hôte:port`, pour un relais joignable sans tunnel (VPS, réseau local, essais).
- Format de l'adresse : `regler_relais` (KadenzAdmin) accepte en plus `wss://hôte[:port][/chemin]`, sans espace, port de 1 à 65535, 443 par défaut. `ws://` sans chiffrement est refusé, sauf vers `127.0.0.1` et `localhost` pour les essais et le banc.
- Postes déjà installés (1.0.0) : ils reçoivent l'adresse dans `Bienvenue` et n'ont que le TCP. Une adresse `wss://` leur donne « relais injoignable », sans autre effet ; le bureau reste essayé en premier. L'accès à distance n'est pas en service aujourd'hui (port fermé), aucun usage n'est donc cassé. Il faut la nouvelle version du poste et du serveur pour passer par le tunnel.
- Le protocole du relais garde sa version (`VERSION = 1`) : le transport ne change pas les trames.

## 9. Trousse du Dell (kadenz-dd)

- `compose.public.yaml` : le relais rejoint un réseau partagé avec cloudflared ; réseau propre au relais proposé (`tunnel-relais`), distinct de `console`, pour que le portail et le relais ne se voient pas ; adresse fixe de cloudflared sur ce réseau, donnée à `--mandataire`.
- Commande du relais : `--ecoute-ws 0.0.0.0:47383 --mandataire <adresse de cloudflared>` ; la publication `127.0.0.1:47381` reste (sonde, administration).
- Compte Cloudflare de l'utilisateur (à lui de le faire, préparé par kadenz-dd) : route `relais.mcarre.dev` vers `http://relais:47383` ; Bot Fight Mode désactivé ou, avec Super Bot Fight Mode, « Definitely Automated » sur Allow pour ce nom (nos clients ne sont pas des navigateurs) ; pas de règle de limitation de débit qui coupe les reconnexions d'un poste.
- Sonde de santé inchangée (TCP local). À ajouter à `kadenz-sante` : une ouverture WebSocket de bout en bout par `wss://`, avec le contrôle épinglé.
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
- Limites : message texte refusé, message de plus de 64 Kio refusé.
- De bout en bout sur le Dell, après la route du tunnel : poste hors du réseau, par `wss://relais.mcarre.dev`.
