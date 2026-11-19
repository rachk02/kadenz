# Portail des licences de Kadenz (jalon F9), pour Linux.
#
# Construit depuis la racine du dépôt (l'espace de travail Cargo entier est
# nécessaire à la résolution, seul kadenz-portail est compilé) :
#   docker build -f crates/portail/Dockerfile -t kadenz-portail .
#
# Un seul programme dans l'image, /usr/local/bin/kadenz-portail, qui exige le
# fichier de la clé du portail et refuse une clé que Kadenz ne reconnaît pas.
# Ni le portail d'essai ni la clé du banc d'essai n'y entrent (lot D1,
# docs/spec-portail.md, section 9) : ils restent des exemples de
# développement, construits hors de l'image.
#
# Utilisateur kadenz d'uid et de gid fixes, 10001 : le fichier de la clé monté
# lui appartient, en mode 400 (docs/deploiement-vps.md) ; le portail refuse un
# fichier de clé lisible par d'autres.
#
# Données (base du portail) dans /donnees, à garder dans un volume. La clé du
# portail n'est jamais dans l'image : elle se monte en lecture seule et se
# passe par --cle. Lancer avec --init (le portail ne traite pas SIGTERM) :
#   docker run -d --init -p 127.0.0.1:47382:47382 -v kadenz-portail:/donnees \
#     -v /chemin/cle-portail.hex:/cle/portail.hex:ro kadenz-portail --cle /cle/portail.hex
# Derrière un mandataire (Caddy, ou tailscaled pour Funnel), ajouter
# --mandataire <adresse IP du mandataire vue par le conteneur> pour que les
# connexions soient freinées par adresse du client.
# Santé : « kadenz-portail sante » interroge /sante en local (l'image n'a pas
# curl).
#
# Profil de compilation : release par défaut. --build-arg PROFIL=dev construit
# une image d'essai, qui lit KADENZ_PORTAIL_CLE (clé d'essai) ; jamais en
# service, toujours sous une étiquette à part (deploiement/dell/compose.essai.yaml).
#
# Images épinglées par empreinte, les mêmes que pour le relais
# (crates/relais/Dockerfile, vérifiées le 2026-11-15).

FROM rust:1.99.1-slim-trixie@sha256:f6c4a35d21dbb3c89d7a8d056c193e94fc09037f2b58dd858b09c39a47dd02d7 AS construction
WORKDIR /source
# Deux compilations en parallèle au plus, comme sur le poste de développement
# (docker build --build-arg CARGO_BUILD_JOBS=N pour changer).
ARG CARGO_BUILD_JOBS=2
ARG PROFIL=release
COPY . .
# Le profil dev écrit dans target/debug, les autres dans target/<profil>.
RUN cargo build --locked --profile "$PROFIL" -p kadenz-portail --bin kadenz-portail \
    && case "$PROFIL" in dev) sortie=debug ;; *) sortie="$PROFIL" ;; esac \
    && cp "target/$sortie/kadenz-portail" /kadenz-portail

FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2
RUN groupadd --system --gid 10001 kadenz \
    && useradd --system --uid 10001 --gid 10001 --home-dir /donnees --shell /usr/sbin/nologin kadenz \
    && mkdir /donnees && chown 10001:10001 /donnees && chmod 700 /donnees
COPY --from=construction /kadenz-portail /usr/local/bin/kadenz-portail
USER 10001:10001
VOLUME /donnees
EXPOSE 47382
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD ["/usr/local/bin/kadenz-portail", "sante"]
ENTRYPOINT ["/usr/local/bin/kadenz-portail", "--dossier", "/donnees", "--ecoute", "0.0.0.0:47382"]
