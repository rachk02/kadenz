# Outil de sauvegarde de la base du portail : sqlite3, sans autre programme
# de Kadenz. Lancé par le crochet de sauvegarde sous l'uid du portail
# (10001), sans réseau, racine en lecture seule (deploiement/dell/compose.yaml).
#
# Même image de base que le portail, épinglée par empreinte
# (crates/portail/Dockerfile).

FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2
RUN apt-get update \
    && apt-get install -y --no-install-recommends sqlite3 \
    && rm -rf /var/lib/apt/lists/*
USER 10001:10001
ENTRYPOINT ["sqlite3"]
