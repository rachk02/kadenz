# Relais de l'accès à distance de Kadenz (jalon F8), pour Linux.
#
# Construit depuis la racine du dépôt (l'espace de travail Cargo entier est
# nécessaire à la résolution, seul kadenz-relais est compilé) :
#   docker build -f crates/relais/Dockerfile -t kadenz-relais .
#
# Deux programmes dans l'image :
# - /usr/local/bin/kadenz-relais : le relais, licences vérifiées avec la clé
#   du portail (programme par défaut) ;
# - /usr/local/bin/relais_essai : le relais d'essai, qui accepte la clé du
#   banc d'essai (exemple de développement, pour l'essai F8c seulement).
#
# Données (identité TLS, traces) dans /donnees, à garder dans un volume.
#
# Images épinglées par empreinte (Docker Hub, vérifiées le 2026-11-15) :
# rust:1.99.1-slim-trixie et debian:trixie-slim.

FROM rust:1.99.1-slim-trixie@sha256:f6c4a35d21dbb3c89d7a8d056c193e94fc09037f2b58dd858b09c39a47dd02d7 AS construction
WORKDIR /source
# Deux compilations en parallèle au plus, comme sur le poste de développement
# (docker build --build-arg CARGO_BUILD_JOBS=N pour changer).
ARG CARGO_BUILD_JOBS=2
COPY . .
RUN cargo build --locked --release -p kadenz-relais --bin kadenz-relais --example relais_essai

FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2
RUN useradd --system --home-dir /donnees --shell /usr/sbin/nologin kadenz \
    && mkdir /donnees && chown kadenz /donnees
COPY --from=construction /source/target/release/kadenz-relais /usr/local/bin/kadenz-relais
COPY --from=construction /source/target/release/examples/relais_essai /usr/local/bin/relais_essai
USER kadenz
VOLUME /donnees
EXPOSE 47381
ENTRYPOINT ["/usr/local/bin/kadenz-relais", "--dossier", "/donnees", "--ecoute", "0.0.0.0:47381"]
