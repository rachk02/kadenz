# Kadenz

[![tests](https://img.shields.io/github/actions/workflow/status/rachk02/kadenz/ci.yml?branch=main&label=tests&logo=github)](https://github.com/rachk02/kadenz/actions/workflows/ci.yml)
[![version](https://img.shields.io/github/v/release/rachk02/kadenz?label=version)](https://github.com/rachk02/kadenz/releases/latest)
[![publiée](https://img.shields.io/github/release-date/rachk02/kadenz?label=publi%C3%A9e)](https://github.com/rachk02/kadenz/releases/latest)
[![téléchargements](https://img.shields.io/github/downloads/rachk02/kadenz/total?label=t%C3%A9l%C3%A9chargements)](https://github.com/rachk02/kadenz/releases)
[![dernier commit](https://img.shields.io/github/last-commit/rachk02/kadenz/main?label=dernier%20commit)](https://github.com/rachk02/kadenz/commits/main)
![plateforme](https://img.shields.io/badge/plateforme-Windows%2010%20%7C%2011%2C%2064%20bits-0078d6)
![Rust](https://img.shields.io/badge/Rust-%C3%A9dition%202024-b7410e?logo=rust)
![base](https://img.shields.io/badge/base-SQLite%20chiffr%C3%A9e-003b57?logo=sqlite)
![langues](https://img.shields.io/badge/langues-7-1f4fd8)
[![licence](https://img.shields.io/badge/licence-propri%C3%A9taire-555555)](#licence)

Logiciel desktop Windows de gestion commerciale (devis, proforma, bons de commande, bordereaux de livraison, factures, paiements), mono-poste, installé indépendamment chez chaque entreprise cliente.

La spécification de référence est le [cahier des charges](docs/cahier-des-charges.md). Les sources d'origine (ancien cahier, captures de l'outil Excel) contiennent des données d'entreprise : elles sont conservées hors dépôt, dans `docs_of_interest/` (ignoré par Git).

## Technologies

- Rust, édition 2024
- Base de données : SQLite embarqué, via `rusqlite`
- PDF : `krilla` ; export Excel : `rust_xlsxwriter`

## Structure

| Dossier | Crate | Rôle |
|---|---|---|
| `crates/domaine` | `kadenz-domaine` | Règles métier pures : statuts, calculs, numérotation, paiements, droits |
| `crates/stockage` | `kadenz-stockage` | SQLite : ouverture, migrations, compteurs, sauvegarde |
| `crates/import` | `kadenz-import` | Lecture des fichiers CSV et Excel à importer, aperçu ligne par ligne |
| `crates/pdf` | `kadenz-pdf` | Documents, listes et lettres en PDF |
| `crates/licence` | `kadenz-licence` | Clé de licence signée, identifiant du poste |
| `crates/editeur` | `kadenz-editeur` | Outil de l'éditeur (clés de licence), jamais distribué |
| `crates/export` | `kadenz-export` | Export Excel des registres et des documents |
| `crates/maj` | `kadenz-maj` | Recherche et installation des mises à jour |
| `crates/app` | `kadenz` | Application : écrans et orchestration |

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
.\scripts\release.ps1 -Version 0.1.0               # tag vX.Y.Z, push du tag, release GitHub avec l'installateur .exe, le paquet .msi et leurs empreintes
```

## Installateur Windows

`.\installateur\construire.ps1` produit `target\installateur\KadenzSetup-X.Y.Z.exe`, éditeur Mcarre : assistant aux couleurs de Kadenz, langue choisie à la première page parmi les sept de l'application, dossier d'installation au choix (`Program Files\Kadenz` par défaut), raccourcis du bureau et du menu Démarrer au choix, lancement de Kadenz à la fin, désinstallation depuis les paramètres de Windows. Le thème de l'assistant est généré par `installateur\theme\generer.py`, les icônes par `docs\marque\generer_icones.py` (Pillow requis). La base dans `%APPDATA%\Kadenz` n'est pas touchée à l'installation ni aux mises à jour ; la désinstallation la supprime, avec sa clé. Prérequis : WiX 5 (`dotnet tool install --global wix --version 5.0.1`) et les extensions `WixToolset.BootstrapperApplications.wixext/5.0.1` et `WixToolset.Util.wixext/5.0.1` (`wix extension add -g ...`). Avec `-Certificat <empreinte>`, le script signe kadenz.exe, l'extension de l'assistant, le paquet MSI et l'installateur (signtool du Windows SDK) ; `scripts
elease.ps1` refuse de publier sans certificat.

## Licence

Logiciel propriétaire, tous droits réservés, édité par Mcarre.
