<#
.SYNOPSIS
    Construit l'installateur Windows de Kadenz : target\installateur\KadenzSetup-<version>.exe.

.DESCRIPTION
    1. compile kadenz.exe en mode release (sauf -SansCompilation) ;
    2. construit le paquet MSI (Kadenz.wxs) et le soumet aux contrôles ICE ;
    3. l'enveloppe dans l'installateur .exe (KadenzSetup.wxs).
    Avec -Certificat, signe chaque fichier exécutable livré (signtool, SHA-256,
    horodatage) : kadenz.exe, l'extension de l'assistant, le paquet MSI, le moteur
    Burn de l'installateur puis l'installateur lui-même. Sans certificat, rien
    n'est signé. La publication passe par scripts\release.ps1.

    Prérequis : WiX 5 (`dotnet tool install --global wix --version 5.0.1`) et ses
    extensions `wix extension add -g WixToolset.BootstrapperApplications.wixext/5.0.1`
    et `wix extension add -g WixToolset.Util.wixext/5.0.1` (suppression des données à la
    désinstallation).

.PARAMETER SansCompilation
    Reprend target\release\kadenz.exe tel quel.

.PARAMETER Editeur
    Éditeur affiché par Windows (programmes installés, propriétés du fichier).

.PARAMETER Certificat
    Empreinte SHA-1 du certificat de signature de code, dans le magasin de
    certificats de Windows (CurrentUser\My ou LocalMachine\My).

.PARAMETER Horodatage
    Serveur d'horodatage RFC 3161 : la signature reste valide après
    l'expiration du certificat.

.PARAMETER CertificatEssai
    Fichier .pfx d'un certificat d'essai auto-signé (jalon F10, essai en
    simulation), à la place de -Certificat : signature sans horodatage (pas
    d'appel extérieur), vérifiée par l'empreinte du signataire, le certificat
    n'étant reconnu par aucune autorité. Jamais pour une publication :
    scripts\release.ps1 le refuse hors de -Simulation.

.PARAMETER MotDePasseEssai
    Mot de passe du fichier .pfx d'essai.

.EXAMPLE
    .\installateur\construire.ps1
#>
param(
    [switch]$SansCompilation,
    [string]$Editeur = 'Mcarre',
    [string]$Certificat = '',
    [string]$Horodatage = 'http://time.certum.pl',
    [string]$CertificatEssai = '',
    [string]$MotDePasseEssai = ''
)

$ErrorActionPreference = 'Stop'
$racine = Split-Path -Parent $PSScriptRoot
Set-Location $racine

function Executer([string]$exe, [string[]]$arguments) {
    # cargo et wix écrivent sur stderr même en réussite ; Windows PowerShell
    # traite cette sortie comme une erreur quand $ErrorActionPreference vaut Stop.
    $precedent = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        & $exe @arguments
        if ($LASTEXITCODE -ne 0) { throw "échec : $exe $($arguments -join ' ')" }
    } finally {
        $ErrorActionPreference = $precedent
    }
}

if ($Certificat -and $CertificatEssai) { throw "-Certificat et -CertificatEssai s'excluent" }
$signtool = $null
$empreinteEssai = $null
if ($CertificatEssai) {
    if (-not (Test-Path -PathType Leaf $CertificatEssai)) { throw "certificat d'essai introuvable : $CertificatEssai" }
    # Lu depuis le fichier, sans passer par le magasin de certificats de Windows.
    $empreinteEssai = [System.Security.Cryptography.X509Certificates.X509Certificate2]::new(
        (Resolve-Path $CertificatEssai).Path, $MotDePasseEssai).Thumbprint
}
if ($Certificat -or $CertificatEssai) {
    $signtool = Get-ChildItem (Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin\*\x64\signtool.exe') -ErrorAction SilentlyContinue |
        Sort-Object FullName | Select-Object -Last 1 -ExpandProperty FullName
    if (-not $signtool) { throw "signtool.exe introuvable (Windows SDK) : il signe les fichiers livrés" }
}

# Signe un fichier puis vérifie sa signature (stratégie Authenticode).
function Signer([string]$fichier) {
    if (-not $signtool) { return }
    if ($CertificatEssai) {
        # Essai : sans horodatage ; signataire vérifié par son empreinte.
        Executer $signtool @('sign', '/f', $CertificatEssai, '/p', $MotDePasseEssai, '/fd', 'sha256', '/d', 'Kadenz', $fichier)
        $signature = Get-AuthenticodeSignature -FilePath $fichier
        if (-not $signature.SignerCertificate -or $signature.SignerCertificate.Thumbprint -ne $empreinteEssai) {
            throw "signature d'essai absente ou d'un autre signataire : $fichier"
        }
        return
    }
    Executer $signtool @('sign', '/sha1', $Certificat, '/fd', 'sha256', '/tr', $Horodatage, '/td', 'sha256', '/d', 'Kadenz', $fichier)
    Executer $signtool @('verify', '/pa', '/q', $fichier)
}

$cargoToml = Get-Content -Raw (Join-Path $racine 'Cargo.toml')
if ($cargoToml -notmatch '(?ms)^\[workspace\.package\].*?^version\s*=\s*"([^"]+)"') {
    throw "version introuvable dans [workspace.package] de Cargo.toml"
}
$version = $Matches[1]

if (-not $SansCompilation) {
    $cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
    if (-not (Test-Path $cargo)) { $cargo = 'cargo' }
    Executer $cargo @('build', '--release', '-p', 'kadenz')
}
$compile = Join-Path $racine 'target\release\kadenz.exe'
if (-not (Test-Path $compile)) { throw "kadenz.exe introuvable : $compile" }
# Garde-fou : un kadenz.exe plus ancien que le dernier commit du code Rust n'est
# pas celui du code livré (un installateur a déjà embarqué par erreur le socle du
# premier jour). Les commits de l'installateur seul ne comptent pas.
$dernierCommit = [DateTime]::Parse((git -C $racine log -1 --format=%cI -- crates Cargo.toml Cargo.lock))
if ((Get-Item $compile).LastWriteTime -lt $dernierCommit) {
    throw "kadenz.exe ($((Get-Item $compile).LastWriteTime)) est plus ancien que le dernier commit ($dernierCommit) : relancer sans -SansCompilation"
}

$sortie = Join-Path $racine 'target\installateur'
New-Item -ItemType Directory -Force $sortie | Out-Null
# Copie livrée (signée avec -Certificat) : celle du paquet MSI et de la release.
$exe = Join-Path $sortie 'kadenz.exe'
Copy-Item $compile $exe -Force
Signer $exe
$icone = Join-Path $racine 'crates\app\kadenz.ico'
$msi = Join-Path $sortie 'Kadenz.msi'
$setup = Join-Path $sortie "KadenzSetup-$version.exe"
$communes = @('-arch', 'x64', '-d', "Version=$version", '-d', "Editeur=$Editeur", '-d', "Icone=$icone")

# Extension de l'assistant (extension\kadenzbaf.cpp) : bouton Suivant et case
# « Lancer Kadenz », que WixStdBA ne sait pas faire seul. Compilée avec MSVC,
# contre les en-têtes et bibliothèques des paquets NuGet WiX 5.0.1, téléchargés
# une fois dans target\installateur\wix-natif.
$natif = Join-Path $sortie 'wix-natif'
$paquets = @('wixtoolset.dutil', 'wixtoolset.bootstrapperapplicationapi', 'wixtoolset.wixstandardbootstrapperapplicationfunctionapi')
foreach ($paquet in $paquets) {
    $dossier = Join-Path $natif $paquet
    if (-not (Test-Path $dossier)) {
        $nupkg = Join-Path $natif "$paquet.zip"
        New-Item -ItemType Directory -Force $natif | Out-Null
        Invoke-WebRequest -UseBasicParsing -Uri "https://api.nuget.org/v3-flatcontainer/$paquet/5.0.1/$paquet.5.0.1.nupkg" -OutFile $nupkg
        Expand-Archive -Path $nupkg -DestinationPath $dossier
        Remove-Item $nupkg
    }
}
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$vs = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $vs) { throw "MSVC (outils C++ x64) introuvable : il compile l'extension de l'assistant" }
$vcvars = Join-Path $vs 'VC\Auxiliary\Build\vcvars64.bat'
$extension = Join-Path $PSScriptRoot 'extension'
$baf = Join-Path $sortie 'kadenzbaf.dll'
$inclusions = @(
    "$natif\wixtoolset.dutil\build\native\include",
    "$natif\wixtoolset.bootstrapperapplicationapi\build\native\include",
    "$natif\wixtoolset.wixstandardbootstrapperapplicationfunctionapi\lib\native\include"
) | ForEach-Object { "/I`"$_`"" }
$bibliotheques = @(
    "$natif\wixtoolset.wixstandardbootstrapperapplicationfunctionapi\lib\native\v14\x64\wixstdfn.lib",
    "$natif\wixtoolset.bootstrapperapplicationapi\build\native\v14\x64\balutil.lib",
    "$natif\wixtoolset.dutil\build\native\v14\x64\dutil.lib"
) | ForEach-Object { "`"$_`"" }
$systeme = 'user32.lib advapi32.lib ole32.lib oleaut32.lib shell32.lib shlwapi.lib comctl32.lib gdiplus.lib msimg32.lib wininet.lib version.lib crypt32.lib rpcrt4.lib msi.lib'
# Passage par un fichier .cmd : les guillemets d'une ligne cmd /c ne survivent
# pas à Windows PowerShell 5.1.
$script = Join-Path $sortie 'compiler-extension.cmd'
@(
    '@echo off',
    "set `"PATH=%PATH%;$(Split-Path $vswhere)`"",
    "call `"$vcvars`" >nul || exit /b 1",
    "cl /nologo /O2 /EHsc /MT /W3 /DUNICODE /D_UNICODE /LD $($inclusions -join ' ') `"$extension\kadenzbaf.cpp`" /Fo`"$sortie\\`" /Fe`"$baf`" /link /DEF:`"$extension\kadenzbaf.def`" $($bibliotheques -join ' ') $systeme"
) | Set-Content -Encoding ascii $script
& cmd /c $script
if ($LASTEXITCODE -ne 0) { throw "échec de la compilation de l'extension de l'assistant" }
Signer $baf
$communes += @('-d', "Extension=$baf")

# Fichiers livrés à côté de kadenz.exe : LISEZMOI, modèles d'import (installateur\contenu),
# historique des versions (CHANGELOG.md) et composants libres utilisés (cargo metadata).
$contenu = Join-Path $sortie 'contenu'
if (Test-Path $contenu) { Remove-Item -Recurse -Force $contenu }
Copy-Item (Join-Path $PSScriptRoot 'contenu') $contenu -Recurse
Copy-Item (Join-Path $racine 'CHANGELOG.md') (Join-Path $contenu 'Notes de version.txt')
# cargo tree plutôt que cargo metadata : ConvertFrom-Json de Windows PowerShell 5.1 refuse un JSON de plus de 2 Mo.
$arbre = & cargo tree --workspace --locked -e normal,build --target all --prefix none --format '{p}|{l}|{r}' --manifest-path (Join-Path $racine 'Cargo.toml') 2>$null
$composants = $arbre | Where-Object { $_.Trim() -and $_ -notmatch ' \(\w:|^\S+ v\S+ \(/' } | ForEach-Object { $_ -replace ' \(\*\)$', '' } | Sort-Object -Unique
$licences = @(
    'Composants libres utilisés par Kadenz',
    'Liste établie à la compilation depuis Cargo.lock : nom, version, licence déclarée',
    'par l''auteur, dépôt. Le texte de chaque licence est publié dans le dépôt du composant.',
    ''
)
$licences += $composants | ForEach-Object {
    $p = $_ -split '\|', 3
    $lic = if ($p[1]) { $p[1] } else { 'voir le dépôt' }
    $depot = if ($p[2]) { $p[2] } else { 'https://crates.io' }
    "$($p[0]) : $lic ($depot)"
}
[System.IO.File]::WriteAllLines((Join-Path $contenu 'Licences-tierces.txt'), $licences, [System.Text.UTF8Encoding]::new($true))

# Deux éditions, un seul programme (docs/spec-monoposte.md, section 6) : le
# monoposte ajoute la marque « edition-monoposte » à côté de kadenz.exe. Leurs
# paquets partagent le code de mise à niveau (l'une remplace l'autre), leurs
# installateurs ont chacun leur nom et leur code. Noms commerciaux (décision du
# 2026-11-25) : « Kadenz » pour le monoposte, « Kadenz Entreprise » pour le
# multiposte ; les noms de fichiers ne changent pas.
$editions = @(
    @{ Nom = 'multiposte'; Msi = 'Kadenz.msi'; Setup = "KadenzSetup-$version.exe"; Bundle = 'Kadenz Entreprise'; Code = '39f20fc0-e088-4792-85bb-29b2cc47d5ef' },
    @{ Nom = 'monoposte'; Msi = 'KadenzMonoposte.msi'; Setup = "KadenzMonoposteSetup-$version.exe"; Bundle = 'Kadenz'; Code = '5d7c3a1e-8b24-4f6a-9c0d-2e1f7a3b4c58' }
)
$contenuBase = $contenu
foreach ($edition in $editions) {
    $contenu = Join-Path $sortie "contenu-$($edition.Nom)"
    if (Test-Path $contenu) { Remove-Item -Recurse -Force $contenu }
    Copy-Item $contenuBase $contenu -Recurse
    if ($edition.Nom -eq 'monoposte') {
        New-Item -ItemType File (Join-Path $contenu 'edition-monoposte') | Out-Null
    }
    $msi = Join-Path $sortie $edition.Msi
    $setup = Join-Path $sortie $edition.Setup
    $parEdition = @('-d', "Contenu=$contenu", '-d', "Edition=$($edition.Nom)")
    Executer wix (@('build', (Join-Path $PSScriptRoot 'Kadenz.wxs'), '-ext', 'WixToolset.Util.wixext', '-d', "Exe=$exe", '-o', $msi) + $communes + $parEdition)
    # Contrôles ICE de Windows Installer. ICE43 et ICE57 sont écartés : ils supposent
    # une installation par compte (raccourcis du menu Démarrer et du bureau dans le
    # profil), alors que le paquet est installé pour tout le poste (ALLUSERS=1), où
    # ces dossiers sont ceux de tous les comptes.
    Executer wix @('msi', 'validate', '-sice', 'ICE43', '-sice', 'ICE57', $msi)
    Signer $msi
    Executer wix (@('build', (Join-Path $PSScriptRoot 'KadenzSetup.wxs'), '-ext', 'WixToolset.BootstrapperApplications.wixext', '-d', "Msi=$msi", '-d', "NomBundle=$($edition.Bundle)", '-d', "CodeBundle=$($edition.Code)", '-o', $setup) + $communes)
    if ($signtool) {
        # Le moteur Burn, qui lit la suite de l'installateur, se signe à part :
        # détaché, signé, rattaché, puis l'installateur entier est signé.
        $moteur = Join-Path $sortie 'moteur.exe'
        Executer wix @('burn', 'detach', $setup, '-engine', $moteur)
        Signer $moteur
        Executer wix @('burn', 'reattach', $setup, '-engine', $moteur, '-o', $setup)
        Remove-Item $moteur
        Signer $setup
        Write-Host "Installateur signé : $setup"
    } else {
        Write-Host "Installateur prêt, non signé : $setup"
    }
}
