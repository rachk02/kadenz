# Portail des licences de Kadenz (jalon F9), pour Linux.
#
# Construit depuis la racine du dépôt (l'espace de travail Cargo entier est
# nécessaire à la résolution, seul kadenz-portail est compilé) :
#   docker build -f crates/portail/Dockerfile -t kadenz-portail .
#
# Deux programmes dans l'image :
# - /usr/local/bin/kadenz-portail : le portail (programme par défaut), qui
#   exige le fichier de la clé du portail et refuse une clé que Kadenz ne
#   reconnaît pas ;
# - /usr/local/bin/portail_essai : le portail d'essai, avec la clé du banc
#   d'essai (exemple de développement, pour l'essai F9c seulement).
#
# Données (base du portail) dans /donnees, à garder dans un volume. La clé du
# portail n'est jamais dans l'image : elle se monte en lecture seule et se
# passe par --cle. Lancer avec --init (le portail ne traite pas SIGTERM) :
#   docker run -d --init -p 127.0.0.1:47382:47382 -v kadenz-portail:/donnees \
#     -v /chemin/cle-portail.hex:/cle/portail.hex:ro kadenz-portail --cle /cle/portail.hex
# Derrière un mandataire (Caddy sur le VPS), ajouter --mandataire pour que les
# connexions soient freinées par adresse du client.
#
# Images épinglées par empreinte, les mêmes que pour le relais
# (crates/relais/Dockerfile, vérifiées le 2026-11-15).

FROM rust:1.99.1-slim-trixie@sha256:f6c4a35d21dbb3c89d7a8d056c193e94fc09037f2b58dd858b09c39a47dd02d7 AS construction
WORKDIR /source
# Deux compilations en parallèle au plus, comme sur le poste de développement
# (docker build --build-arg CARGO_BUILD_JOBS=N pour changer).
ARG CARGO_BUILD_JOBS=2
COPY . .
RUN cargo build --locked --release -p kadenz-portail --bin kadenz-portail --example portail_essai

FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2
RUN useradd --system --home-dir /donnees --shell /usr/sbin/nologin kadenz \
    && mkdir /donnees && chown kadenz /donnees && chmod 700 /donnees
COPY --from=construction /source/target/release/kadenz-portail /usr/local/bin/kadenz-portail
COPY --from=construction /source/target/release/examples/portail_essai /usr/local/bin/portail_essai
USER kadenz
VOLUME /donnees
EXPOSE 47382
ENTRYPOINT ["/usr/local/bin/kadenz-portail", "--dossier", "/donnees", "--ecoute", "0.0.0.0:47382"]
