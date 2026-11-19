# Relais de l'accès à distance de Kadenz (jalon F8), pour Linux.
#
# Construit depuis la racine du dépôt (l'espace de travail Cargo entier est
# nécessaire à la résolution, seul kadenz-relais est compilé) :
#   docker build -f crates/relais/Dockerfile -t kadenz-relais .
#
# Un seul programme dans l'image, /usr/local/bin/kadenz-relais, qui vérifie
# les licences avec la clé du portail. Ni le relais d'essai ni la clé du banc
# d'essai n'y entrent (lot D1, docs/spec-portail.md, section 9).
#
# Données (identité TLS, traces) dans /donnees, à garder dans un volume. Le
# dossier appartient à l'utilisateur kadenz d'uid et de gid fixes, 10001 : le
# relais le met en 700 au démarrage et refuse de démarrer s'il ne peut pas
# (lot R1). Santé : « kadenz-relais sante --dossier /donnees ».
#
# Lancer avec --init : le relais ne traite pas SIGTERM, et sans processus
# d'initialisation « docker stop » attend 10 secondes avant de le tuer
# (constaté à l'essai F8c). Le relais ne garde rien en mémoire qui doive être
# écrit à l'arrêt :
#   docker run -d --init -p 47381:47381 -v kadenz-relais:/donnees kadenz-relais
#
# Images épinglées par empreinte (Docker Hub, vérifiées le 2026-11-15) :
# rust:1.99.1-slim-trixie et debian:trixie-slim.

FROM rust:1.99.1-slim-trixie@sha256:f6c4a35d21dbb3c89d7a8d056c193e94fc09037f2b58dd858b09c39a47dd02d7 AS construction
WORKDIR /source
# Deux compilations en parallèle au plus, comme sur le poste de développement
# (docker build --build-arg CARGO_BUILD_JOBS=N pour changer).
ARG CARGO_BUILD_JOBS=2
COPY . .
RUN cargo build --locked --release -p kadenz-relais --bin kadenz-relais

FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2
RUN groupadd --system --gid 10001 kadenz \
    && useradd --system --uid 10001 --gid 10001 --home-dir /donnees --shell /usr/sbin/nologin kadenz \
    && mkdir /donnees && chown 10001:10001 /donnees && chmod 700 /donnees
COPY --from=construction /source/target/release/kadenz-relais /usr/local/bin/kadenz-relais
USER 10001:10001
VOLUME /donnees
EXPOSE 47381
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD ["/usr/local/bin/kadenz-relais", "sante", "--dossier", "/donnees"]
ENTRYPOINT ["/usr/local/bin/kadenz-relais", "--dossier", "/donnees", "--ecoute", "0.0.0.0:47381"]
