# Kadenz

Logiciel desktop Windows de gestion commerciale (devis, proforma, bons de commande, bordereaux de livraison, factures, paiements), mono-poste, installé indépendamment chez chaque entreprise cliente.

[![Made with Slint](https://raw.githubusercontent.com/slint-ui/slint/master/logo/MadeWithSlint-logo-whitebg.png)](https://slint.dev)

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

Logiciel propriétaire, tous droits réservés, édité par Mcarre. Le badge en tête de cette page est l'attribution exigée par la licence Royalty-free 2.0 de la bibliothèque d'interface : il doit rester sur cette page publique, d'où se téléchargent les versions.
