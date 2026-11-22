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
         (installateur\construire.ps1, WiX 5) et le paquet du serveur
         KadenzServeur-X.Y.Z.msi (installateur\construire-serveur.ps1), tous
         signés avec -Certificat ;
      5. écrit le manifeste de mise à jour kadenz-maj.json et sa signature
         (kadenz-editeur manifeste, clé privée -CleMaj), après avoir vérifié les
         clés embarquées et que la clé privée correspond à la clé de mise à jour
         embarquée (cahier de la phase 2, pont V1, règle b). À partir de la
         génération v2 (phase 2, jalon F10), le manifeste désigne aussi le
         paquet du serveur ;
      6. génération v2 : refuse la release si elle porte à plus de 25 le nombre
         de releases publiées depuis la dernière release de la V1 (versions
         0.x, décision 62) : les postes du pont ne lisent que les 30 dernières
         (spécification de la synchronisation, point 6) ;
      7. crée le tag annoté vX.Y.Z, le pousse, et crée la release GitHub avec
         l'installateur KadenzSetup-X.Y.Z.exe, le paquet Kadenz-X.Y.Z.msi, le
         paquet du serveur KadenzServeur-X.Y.Z.msi, leurs empreintes SHA-256
         (.sha256), le manifeste et sa signature ; pas de kadenz.exe seul
         (décision de l'utilisateur du 2026-11-12).
         Drapeau Latest (règle a, décision 14 de la phase 2) : posé pour la
         génération v1, toujours retiré pour une autre génération ; il est passé
         explicitement dans les deux cas, GitHub le posant sinon par défaut.

.PARAMETER Version
    Version à publier, sans le "v" (ex. 0.1.0).

.PARAMETER Simulation
    Exécute les vérifications et la compilation, sans créer de tag ni de release.

.PARAMETER CleMaj
    Fichier de la clé privée de mise à jour (kadenz-editeur generer), hors du
    dépôt. Sa clé publique doit être celle de crates\maj\cle-maj.hex.

.PARAMETER Depuis
    Version minimale depuis laquelle la mise à jour automatique est permise
    (0.0.0 par défaut : depuis toute version).

.PARAMETER Certificat
    Empreinte SHA-1 du certificat de signature de code, transmise à
    installateur\construire.ps1 et construire-serveur.ps1. Sans elle, une
    publication réelle est refusée : un fichier non signé est pris pour un virus
    par certains antivirus (Kaspersky, PDM:Trojan.Win32.Generic, essai du
    2026-11-11).

.PARAMETER Essai
    Release d'essai (jalon F10), avec -Simulation seulement : clé de mise à
    jour d'essai (-CleMaj, dossier créé par kadenz-editeur generer, dont la clé
    publique est lue à côté), sans contrôle du dépôt, du tag, du CHANGELOG, du
    format, de Clippy ni des tests (lancés à part), manifeste écrit par
    kadenz-editeur en version de développement (KADENZ_MAJ_CLE). Jamais publié.

.PARAMETER CertificatEssai
    Fichier .pfx d'un certificat d'essai auto-signé, avec -Essai seulement,
    transmis aux scripts de l'installateur.

.PARAMETER MotDePasseEssai
    Mot de passe du fichier .pfx d'essai.

.EXAMPLE
    .\scripts\release.ps1 -Version 0.3.0 -CleMaj D:\cles\maj\kadenz-cle-privee.hex -Simulation
    .\scripts\release.ps1 -Version 0.3.0 -CleMaj D:\cles\maj\kadenz-cle-privee.hex -Certificat <empreinte>
    .\scripts\release.ps1 -Version 1.0.0 -CleMaj <essai>\kadenz-cle-privee.hex -Simulation -Essai -CertificatEssai <essai>\essai.pfx -MotDePasseEssai <mot de passe>
#>
param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^\d+\.\d+\.\d+$')]
    [string]$Version,

    [Parameter(Mandatory = $true)]
    [string]$CleMaj,

    [ValidatePattern('^\d+\.\d+\.\d+$')]
    [string]$Depuis = '0.0.0',

    [switch]$Simulation,

    [string]$Certificat = '',

    [switch]$Essai,

    # Publication non signée, explicitement décidée par l'utilisateur le
    # 2026-11-21 sans certificat de signature de code disponible. Sans ce
    # commutateur, une publication réelle refuse tout fichier non signé.
    [switch]$SansCertificat,

    [string]$CertificatEssai = '',

    [string]$MotDePasseEssai = ''
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

# Mode essai : jamais publié, jamais avec le vrai certificat.
if ($Essai -and -not $Simulation) { throw "-Essai n'existe qu'avec -Simulation : une release d'essai n'est jamais publiée" }
if ($CertificatEssai -and -not $Essai) { throw "-CertificatEssai n'existe qu'avec -Essai" }
if ($Essai -and $Certificat) { throw "-Essai n'utilise jamais le vrai certificat : passer -CertificatEssai" }
if ($SansCertificat -and $Certificat) { throw "-SansCertificat et -Certificat s'excluent" }
if ($SansCertificat -and $CertificatEssai) { throw "-SansCertificat et -CertificatEssai s'excluent" }

Etape "Contrôle du dépôt"
if (-not (Test-Path -PathType Leaf $CleMaj)) { throw "clé privée de mise à jour introuvable : $CleMaj" }
if ($Essai) {
    Write-Host "    Essai : dépôt, tag et CHANGELOG non contrôlés"
} else {
    if (git status --porcelain) { throw "l'arbre de travail n'est pas propre" }
    $branche = git rev-parse --abbrev-ref HEAD
    if ($branche -ne 'main') { throw "la release se fait depuis main (branche actuelle : $branche)" }
    Executer git @('fetch', 'origin', 'main', '--tags')
    if ((git rev-parse HEAD) -ne (git rev-parse origin/main)) { throw "main n'est pas à jour avec origin/main" }
    if (git tag --list $tag) { throw "le tag $tag existe déjà" }
}

Etape "Contrôle de la version"
$cargoToml = Get-Content -Raw (Join-Path $racine 'Cargo.toml')
if ($cargoToml -notmatch '(?ms)^\[workspace\.package\].*?^version\s*=\s*"([^"]+)"') {
    throw "version introuvable dans [workspace.package] de Cargo.toml"
}
if ($Matches[1] -ne $Version) {
    throw "Cargo.toml indique $($Matches[1]), la release demandée est $Version"
}
$changelog = Get-Content -Raw (Join-Path $racine 'CHANGELOG.md')
if (-not $Essai -and $changelog -notmatch "(?m)^## \[$([regex]::Escape($Version))\]") {
    throw "aucune section ## [$Version] dans CHANGELOG.md"
}

# Sans adresse, l'activation répond « indisponible » sur tous les postes :
# crates/maj/src/activation.rs:66-74. Refuser une release plutôt que de livrer
# des postes incapables d'activer un code.
Etape "Contrôle de l'adresse du portail"
$urlPortail = (Get-Content -Raw (Join-Path $racine 'crates\maj\portail-url.txt')).Trim()
if ($urlPortail -notmatch '^https://[A-Za-z0-9.-]+(?::[0-9]+)?/?$') {
    throw "crates/maj/portail-url.txt doit contenir une adresse https, trouvé : « $urlPortail »"
}

if ($Essai) {
    Etape "Format, Clippy, tests : non lancés (essai)"
} else {
    Etape "Format, Clippy, tests"
    Executer $cargo @('fmt', '--all', '--check')
    Executer $cargo @('clippy', '--workspace', '--all-targets', '--', '-D', 'warnings')
    Executer $cargo @('test', '--workspace')
}

Etape "Compilation release"
Executer $cargo @('build', '--release', '-p', 'kadenz')
Etape "Installateur"
if (-not $Certificat -and -not $Simulation -and -not $SansCertificat) {
    throw "publication refusée sans -Certificat : les fichiers livrés doivent être signés"
}
if ($SansCertificat) {
    Write-Host "    ATTENTION : publication non signée, décision de l'utilisateur du 2026-11-21."
    Write-Host "    Les antivirus peuvent classer l'installateur comme dangereux (décision du 2026-11-11)."
}
$signature = if ($CertificatEssai) {
    @{ CertificatEssai = $CertificatEssai; MotDePasseEssai = $MotDePasseEssai }
} else {
    @{ Certificat = $Certificat }
}
& (Join-Path $PSScriptRoot '..\installateur\construire.ps1') -SansCompilation @signature
if ($LASTEXITCODE -ne 0) { throw "échec de la construction de l'installateur" }
$installateur = Join-Path $racine "target\installateur\KadenzSetup-$Version.exe"
if (-not (Test-Path $installateur)) { throw "installateur introuvable : $installateur" }

Etape "Paquet du serveur"
& (Join-Path $PSScriptRoot '..\installateur\construire-serveur.ps1') @signature
if ($LASTEXITCODE -ne 0) { throw "échec de la construction du paquet du serveur" }
$serveur = Join-Path $racine "target\installateur\KadenzServeur-$Version.msi"
if (-not (Test-Path $serveur)) { throw "paquet du serveur introuvable : $serveur" }

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
$empreinteServeur = Empreinte $serveur

Etape "Manifeste de mise à jour signé"
if ($Essai) {
    # Clé d'essai : kadenz-editeur en version de développement, qui lit la clé
    # publique d'essai dans KADENZ_MAJ_CLE au lieu de la clé embarquée.
    $publique = Join-Path (Split-Path -Parent $CleMaj) 'kadenz-cle-publique.hex'
    if (-not (Test-Path -PathType Leaf $publique)) { throw "clé publique d'essai introuvable : $publique" }
    $env:KADENZ_MAJ_CLE = (Get-Content -Raw $publique).Trim()
    try {
        $sortieManifeste = & $cargo run --quiet -p kadenz-editeur -- manifeste $CleMaj $installateur $Depuis $serveur
    } finally {
        Remove-Item Env:\KADENZ_MAJ_CLE
    }
} else {
    $sortieManifeste = & $cargo run --release --quiet -p kadenz-editeur -- manifeste $CleMaj $installateur $Depuis $serveur
}
if ($LASTEXITCODE -ne 0) { throw "manifeste de mise à jour refusé : aucune release sans manifeste valide" }
$generation = ($sortieManifeste | Select-Object -Last 1).Trim()
$manifeste = Join-Path $racine 'target\installateur\kadenz-maj.json'
$signatureManifeste = Join-Path $racine 'target\installateur\kadenz-maj.json.sig'
foreach ($f in @($manifeste, $signatureManifeste)) {
    if (-not (Test-Path $f)) { throw "fichier du manifeste introuvable : $f" }
}
# Règle a : Latest réservé à la génération v1, explicite dans les deux cas.
$drapeauLatest = if ($generation -eq 'v1') { '--latest' } else { '--latest=false' }

# Règle des 25 releases (génération v2) : les postes du pont lisent les 30
# dernières releases ; au plus 25 de la phase 2 après la dernière de la V1
# (versions 0.x), pour qu'une éventuelle mise à jour V1 reste visible. Lecture
# seule de la liste des releases.
if ($generation -ne 'v1') {
    Etape "Règle des 25 releases"
    $liste = gh release list --limit 100 --json tagName,createdAt,isDraft | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw "liste des releases illisible (gh release list)" }
    $publiees = @($liste | Where-Object { -not $_.isDraft } | Sort-Object { [DateTime]$_.createdAt } -Descending)
    $depuisV1 = 0
    foreach ($r in $publiees) {
        if ($r.tagName -match '^v0\.') { break }
        $depuisV1++
    }
    Write-Host "    $depuisV1 release(s) de la phase 2 depuis la dernière release de la V1"
    if ($depuisV1 + 1 -gt 25) {
        throw "release refusée : elle porterait à $($depuisV1 + 1) les releases publiées depuis la dernière release de la V1 (25 au plus ; la prochaine release V1 doit lire toutes les pages)"
    }
}

$fichiersRelease = @($installateur, $empreinte, $paquet, $empreintePaquet, $serveur, $empreinteServeur, $manifeste, $signatureManifeste)

if ($Simulation) {
    Etape "Simulation terminée : génération $generation, fichiers prêts ($($fichiersRelease -join ', ')), aucun tag ni release créés"
    Write-Host "    Drapeau pour gh release create : $drapeauLatest"
    exit 0
}

Etape "Tag $tag et release GitHub"
Executer git @('tag', '-a', $tag, '-m', "Kadenz $Version")
Executer git @('push', 'origin', $tag)

# Notes de version : la section du CHANGELOG correspondant à la version.
$notes = [regex]::Match($changelog, "(?ms)^## \[$([regex]::Escape($Version))\][^\n]*\n(.*?)(?=^## \[|\z)").Groups[1].Value.Trim()
$fichierNotes = Join-Path ([System.IO.Path]::GetTempPath()) "kadenz-notes-$Version.md"
Set-Content -Path $fichierNotes -Value $notes -Encoding utf8
Executer gh (@('release', 'create', $tag) + $fichiersRelease + @('--title', "Kadenz $Version", '--notes-file', $fichierNotes, $drapeauLatest))
Remove-Item $fichierNotes

Etape "Release $tag publiée"
