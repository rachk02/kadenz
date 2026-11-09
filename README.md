# Kadenz

Logiciel desktop Windows de gestion commerciale (devis, proforma, bons de commande, bordereaux de livraison, factures, paiements), mono-poste, installé indépendamment chez chaque entreprise cliente.

La spécification de référence est le [cahier des charges](docs/cahier-des-charges.md). Les sources d'origine (ancien cahier, captures de l'outil Excel) sont dans `docs_of_interest/`.

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

## Licence

Logiciel propriétaire, tous droits réservés. Slint est utilisé sous la licence Slint Royalty-free 2.0, qui impose l'affichage du composant `AboutSlint` dans l'écran "À propos" de l'application.
