# Kadenz

Logiciel desktop Windows de gestion commerciale (devis, proforma, bons de commande, bordereaux de livraison, factures, paiements), mono-poste, installé indépendamment chez chaque entreprise cliente.

La spécification de référence est le [cahier des charges](docs/cahier-des-charges.md). Les sources d'origine (ancien cahier, captures de l'outil Excel) contiennent des données d'entreprise : elles sont conservées hors dépôt, dans `docs_of_interest/` (ignoré par Git).

## Technologies

- Rust, édition 2024
- Interface : [Slint](https://slint.dev) 1.18
- Base de données : SQLite embarqué, via `rusqlite`

## Structure

| Dossier | Crate | Rôle |
|---|---|---|
| `crates/domaine` | `kadenz-domaine` | Règles métier pures : statuts, calculs, numérotation, paiements, droits |
| `crates/stockage` | `kadenz-stockage` | SQLite : ouverture, migrations, compteurs, sauvegarde |
| `crates/app` | `kadenz` | Application Slint |

## Développement

Prérequis : Rust stable (MSVC) sous Windows.

```
cargo run -p kadenz            # lancer l'application
cargo test --workspace         # tests
cargo fmt --all                # formatage
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release -p kadenz
```

La base est créée au premier lancement dans `%APPDATA%\Kadenz\kadenz.sqlite` ; les migrations s'appliquent automatiquement.

L'intégration continue GitHub (`.github/workflows/ci.yml`) vérifie format, Clippy et tests sous Windows à chaque push sur `main` et sur chaque pull request.

## Versions, tags et releases

- Numérotation SemVer, version unique dans `[workspace.package]` de `Cargo.toml`, tag Git `vX.Y.Z`.
- Chaque version a sa section dans `CHANGELOG.md`, qui sert de notes de release.
- Publication depuis ce poste, sans minutes GitHub Actions :

```
.\scripts\release.ps1 -Version 0.1.0 -Simulation   # vérifications et compilation, sans publier
.\scripts\release.ps1 -Version 0.1.0               # tag vX.Y.Z, push du tag, release GitHub avec kadenz.exe
```

## Licence

Logiciel propriétaire, tous droits réservés. Slint est utilisé sous la licence Slint Royalty-free 2.0, qui impose l'affichage du composant `AboutSlint` dans l'écran "À propos" de l'application.
