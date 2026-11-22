<#
.SYNOPSIS
    Construit le paquet du serveur Kadenz : target\installateur\KadenzServeur-<version>.msi.

.DESCRIPTION
    1. compile kadenz-serveur.exe et KadenzAdmin.exe en mode release (sauf -SansCompilation) ;
    2. construit le paquet MSI (KadenzServeur.wxs) et le soumet aux contrôles ICE.
    Avec -Certificat, signe les deux exécutables puis le paquet (signtool, SHA-256,
    horodatage). Sans certificat, rien n'est signé. Aucune publication : les
    releases de la phase 2 se décident à part.

    Prérequis : WiX 5 (`dotnet tool install --global wix --version 5.0.1`) et ses
    extensions `wix extension add -g WixToolset.Util.wixext/5.0.1` (relance du
    service après une panne) et `wix extension add -g WixToolset.Firewall.wixext/5.0.1`
    (règle du pare-feu).

.PARAMETER SansCompilation
    Reprend target\release\kadenz-serveur.exe et KadenzAdmin.exe tels quels.

.PARAMETER Editeur
    Éditeur affiché par Windows (programmes installés, propriétés du fichier).

.PARAMETER Certificat
    Empreinte SHA-1 du certificat de signature de code, dans le magasin de
    certificats de Windows (CurrentUser\My ou LocalMachine\My).

.PARAMETER Horodatage
    Serveur d'horodatage RFC 3161.

.PARAMETER CertificatEssai
    Fichier .pfx d'un certificat d'essai auto-signé (jalon F10, essai en
    simulation), à la place de -Certificat : signature sans horodatage (pas
    d'appel extérieur), vérifiée par l'empreinte du signataire, le certificat
    n'étant reconnu par aucune autorité. Jamais pour une publication :
    scripts\release.ps1 le refuse hors de -Simulation.

.PARAMETER MotDePasseEssai
    Mot de passe du fichier .pfx d'essai.

.EXAMPLE
    .\installateur\construire-serveur.ps1
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

function Signer([string]$fichier) {
    if (-not $signtool) { return }
    if ($CertificatEssai) {
        # Essai : sans horodatage ; signataire vérifié par son empreinte.
        Executer $signtool @('sign', '/f', $CertificatEssai, '/p', $MotDePasseEssai, '/fd', 'sha256', '/d', 'Kadenz Serveur', $fichier)
        $signature = Get-AuthenticodeSignature -FilePath $fichier
        if (-not $signature.SignerCertificate -or $signature.SignerCertificate.Thumbprint -ne $empreinteEssai) {
            throw "signature d'essai absente ou d'un autre signataire : $fichier"
        }
        return
    }
    Executer $signtool @('sign', '/sha1', $Certificat, '/fd', 'sha256', '/tr', $Horodatage, '/td', 'sha256', '/d', 'Kadenz Serveur', $fichier)
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
    Executer $cargo @('build', '--release', '-p', 'kadenz-serveur', '-p', 'kadenz-admin')
}
# Garde-fou repris de construire.ps1 : un exécutable plus ancien que le dernier
# commit du code Rust n'est pas celui du code livré.
$dernierCommit = [DateTime]::Parse((git -C $racine log -1 --format=%cI -- crates Cargo.toml Cargo.lock))
foreach ($nom in @('kadenz-serveur.exe', 'KadenzAdmin.exe')) {
    $chemin = Join-Path $racine "target\release\$nom"
    if (-not (Test-Path $chemin)) { throw "$nom introuvable : $chemin" }
    if ((Get-Item $chemin).LastWriteTime -lt $dernierCommit) {
        throw "$nom ($((Get-Item $chemin).LastWriteTime)) est plus ancien que le dernier commit ($dernierCommit) : relancer sans -SansCompilation"
    }
}

$sortie = Join-Path $racine 'target\installateur'
New-Item -ItemType Directory -Force $sortie | Out-Null
$exe = Join-Path $sortie 'kadenz-serveur.exe'
Copy-Item (Join-Path $racine 'target\release\kadenz-serveur.exe') $exe -Force
Signer $exe
$admin = Join-Path $sortie 'KadenzAdmin.exe'
Copy-Item (Join-Path $racine 'target\release\KadenzAdmin.exe') $admin -Force
Signer $admin
$icone = Join-Path $racine 'crates\app\kadenz.ico'
$msi = Join-Path $sortie "KadenzServeur-$version.msi"

Executer wix @('build', (Join-Path $PSScriptRoot 'KadenzServeur.wxs'),
    '-ext', 'WixToolset.Util.wixext', '-ext', 'WixToolset.Firewall.wixext',
    '-arch', 'x64', '-d', "Version=$version", '-d', "Editeur=$Editeur", '-d', "Icone=$icone",
    '-d', "Exe=$exe", '-d', "Admin=$admin", '-o', $msi)
# Contrôles ICE de Windows Installer. ICE43 et ICE57 sont écartés, comme pour
# le paquet de l'application : ils supposent une installation par compte
# (raccourci du menu Démarrer dans le profil), alors que le paquet est installé
# pour toute la machine, où ce dossier est celui de tous les comptes.
Executer wix @('msi', 'validate', '-sice', 'ICE43', '-sice', 'ICE57', $msi)
Signer $msi
if ($signtool) {
    Write-Host "Paquet signé : $msi"
} else {
    Write-Host "Paquet prêt, non signé : $msi"
}
