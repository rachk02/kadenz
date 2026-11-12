<#
.SYNOPSIS
    Publie une release de Kadenz : vérifications, compilation optimisée, tag Git, release GitHub.

.DESCRIPTION
    Tout se fait sur ce poste, sans consommer de minutes GitHub Actions.
    Étapes :
      1. vérifie que l'arbre de travail est propre, sur la branche main, à jour avec origin ;
      2. vérifie que la version demandée correspond à [workspace.package] dans Cargo.toml
         et qu'une section existe dans CHANGELOG.md ;
      3. lance format, Clippy et tests sur tout l'espace de travail ;
      4. compile kadenz.exe en mode release, puis l'installateur KadenzSetup-X.Y.Z.exe
         (installateur\construire.ps1, WiX 5), tous deux signés avec -Certificat ;
      5. crée le tag annoté vX.Y.Z, le pousse, et crée la release GitHub avec
         l'installateur KadenzSetup-X.Y.Z.exe, le paquet Kadenz-X.Y.Z.msi et
         leurs empreintes SHA-256 (.sha256) ; pas de kadenz.exe seul (décision
         de l'utilisateur du 2026-11-12).

.PARAMETER Version
    Version à publier, sans le "v" (ex. 0.1.0).

.PARAMETER Simulation
    Exécute les vérifications et la compilation, sans créer de tag ni de release.

.PARAMETER Certificat
    Empreinte SHA-1 du certificat de signature de code, transmise à
    installateur\construire.ps1. Sans elle, une publication réelle est refusée :
    un fichier non signé est pris pour un virus par certains antivirus
    (Kaspersky, PDM:Trojan.Win32.Generic, essai du 2026-11-11).

.EXAMPLE
    .\scripts\release.ps1 -Version 0.1.0 -Simulation
    .\scripts\release.ps1 -Version 0.1.0
#>
param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^\d+\.\d+\.\d+$')]
    [string]$Version,

    [switch]$Simulation,

    [string]$Certificat = ''
)

$ErrorActionPreference = 'Stop'
$racine = Split-Path -Parent $PSScriptRoot
Set-Location $racine

$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
if (-not (Test-Path $cargo)) { $cargo = 'cargo' }
$tag = "v$Version"

function Etape([string]$texte) { Write-Host "==> $texte" }

function Executer([string]$exe, [string[]]$arguments) {
    & $exe @arguments
    if ($LASTEXITCODE -ne 0) { throw "échec : $exe $($arguments -join ' ')" }
}

Etape "Contrôle du dépôt"
if (git status --porcelain) { throw "l'arbre de travail n'est pas propre" }
$branche = git rev-parse --abbrev-ref HEAD
if ($branche -ne 'main') { throw "la release se fait depuis main (branche actuelle : $branche)" }
Executer git @('fetch', 'origin', 'main', '--tags')
if ((git rev-parse HEAD) -ne (git rev-parse origin/main)) { throw "main n'est pas à jour avec origin/main" }
if (git tag --list $tag) { throw "le tag $tag existe déjà" }

Etape "Contrôle de la version"
$cargoToml = Get-Content -Raw (Join-Path $racine 'Cargo.toml')
if ($cargoToml -notmatch '(?ms)^\[workspace\.package\].*?^version\s*=\s*"([^"]+)"') {
    throw "version introuvable dans [workspace.package] de Cargo.toml"
}
if ($Matches[1] -ne $Version) {
    throw "Cargo.toml indique $($Matches[1]), la release demandée est $Version"
}
$changelog = Get-Content -Raw (Join-Path $racine 'CHANGELOG.md')
if ($changelog -notmatch "(?m)^## \[$([regex]::Escape($Version))\]") {
    throw "aucune section ## [$Version] dans CHANGELOG.md"
}

Etape "Format, Clippy, tests"
Executer $cargo @('fmt', '--all', '--check')
Executer $cargo @('clippy', '--workspace', '--all-targets', '--', '-D', 'warnings')
Executer $cargo @('test', '--workspace')

Etape "Compilation release"
Executer $cargo @('build', '--release', '-p', 'kadenz')
Etape "Installateur"
if (-not $Certificat -and -not $Simulation) {
    throw "publication refusée sans -Certificat : les fichiers livrés doivent être signés"
}
& (Join-Path $PSScriptRoot '..\installateur\construire.ps1') -SansCompilation -Certificat $Certificat
if ($LASTEXITCODE -ne 0) { throw "échec de la construction de l'installateur" }
$installateur = Join-Path $racine "target\installateur\KadenzSetup-$Version.exe"
if (-not (Test-Path $installateur)) { throw "installateur introuvable : $installateur" }

# Paquet MSI seul (signé par construire.ps1), pour un déploiement par GPO ou
# msiexec ; nommé avec la version, comme l'installateur.
$paquet = Join-Path $racine "target\installateur\Kadenz-$Version.msi"
Copy-Item (Join-Path $racine 'target\installateur\Kadenz.msi') $paquet -Force

# Empreinte SHA-256 « <hex>  <nom> ». Celle de l'installateur est vérifiée par
# la mise à jour intégrée (kadenz-maj) avant tout lancement.
function Empreinte([string]$fichier) {
    $hex = (Get-FileHash -Algorithm SHA256 -Path $fichier).Hash.ToLowerInvariant()
    [System.IO.File]::WriteAllText("$fichier.sha256", "$hex  $(Split-Path -Leaf $fichier)`n", [System.Text.UTF8Encoding]::new($false))
    "$fichier.sha256"
}
$empreinte = Empreinte $installateur
$empreintePaquet = Empreinte $paquet

if ($Simulation) {
    Etape "Simulation terminée : $installateur, $paquet et leurs empreintes prêts, aucun tag ni release créés"
    exit 0
}

Etape "Tag $tag et release GitHub"
Executer git @('tag', '-a', $tag, '-m', "Kadenz $Version")
Executer git @('push', 'origin', $tag)

# Notes de version : la section du CHANGELOG correspondant à la version.
$notes = [regex]::Match($changelog, "(?ms)^## \[$([regex]::Escape($Version))\][^\n]*\n(.*?)(?=^## \[|\z)").Groups[1].Value.Trim()
$fichierNotes = Join-Path ([System.IO.Path]::GetTempPath()) "kadenz-notes-$Version.md"
Set-Content -Path $fichierNotes -Value $notes -Encoding utf8
Executer gh @('release', 'create', $tag, $installateur, $empreinte, $paquet, $empreintePaquet, '--title', "Kadenz $Version", '--notes-file', $fichierNotes)
Remove-Item $fichierNotes

Etape "Release $tag publiée"
