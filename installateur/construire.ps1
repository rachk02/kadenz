<#
.SYNOPSIS
    Construit l'installateur Windows de Kadenz : target\installateur\KadenzSetup-<version>.exe.

.DESCRIPTION
    1. compile kadenz.exe en mode release (sauf -SansCompilation) ;
    2. construit le paquet MSI (Kadenz.wxs) et le soumet aux contrôles ICE ;
    3. l'enveloppe dans l'installateur .exe (KadenzSetup.wxs).
    Rien n'est signé ni publié : la signature demande un certificat et l'accord de
    l'utilisateur, la publication passe par scripts\release.ps1.

    Prérequis : WiX 5 (`dotnet tool install --global wix --version 5.0.1`) et son
    extension `wix extension add -g WixToolset.BootstrapperApplications.wixext/5.0.1`.

.PARAMETER SansCompilation
    Reprend target\release\kadenz.exe tel quel.

.PARAMETER Editeur
    Éditeur affiché par Windows (programmes installés, propriétés du fichier).

.EXAMPLE
    .\installateur\construire.ps1
#>
param(
    [switch]$SansCompilation,
    [string]$Editeur = 'Mcarre'
)

$ErrorActionPreference = 'Stop'
$racine = Split-Path -Parent $PSScriptRoot
Set-Location $racine

function Executer([string]$exe, [string[]]$arguments) {
    & $exe @arguments
    if ($LASTEXITCODE -ne 0) { throw "échec : $exe $($arguments -join ' ')" }
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
$exe = Join-Path $racine 'target\release\kadenz.exe'
if (-not (Test-Path $exe)) { throw "kadenz.exe introuvable : $exe" }
# Garde-fou : un kadenz.exe plus ancien que le dernier commit n'est pas celui du
# code livré (un installateur a déjà embarqué par erreur le socle du premier jour).
$dernierCommit = [DateTime]::Parse((git -C $racine log -1 --format=%cI))
if ((Get-Item $exe).LastWriteTime -lt $dernierCommit) {
    throw "kadenz.exe ($((Get-Item $exe).LastWriteTime)) est plus ancien que le dernier commit ($dernierCommit) : relancer sans -SansCompilation"
}

$sortie = Join-Path $racine 'target\installateur'
New-Item -ItemType Directory -Force $sortie | Out-Null
$icone = Join-Path $racine 'crates\app\kadenz.ico'
$msi = Join-Path $sortie 'Kadenz.msi'
$setup = Join-Path $sortie "KadenzSetup-$version.exe"
$communes = @('-arch', 'x64', '-d', "Version=$version", '-d', "Editeur=$Editeur", '-d', "Icone=$icone")

Executer wix (@('build', (Join-Path $PSScriptRoot 'Kadenz.wxs'), '-d', "Exe=$exe", '-o', $msi) + $communes)
# Contrôles ICE de Windows Installer. ICE43 et ICE57 sont écartés : ils supposent
# une installation par compte (raccourcis du menu Démarrer et du bureau dans le
# profil), alors que le paquet est installé pour tout le poste (ALLUSERS=1), où
# ces dossiers sont ceux de tous les comptes.
Executer wix @('msi', 'validate', '-sice', 'ICE43', '-sice', 'ICE57', $msi)
Executer wix (@('build', (Join-Path $PSScriptRoot 'KadenzSetup.wxs'), '-ext', 'WixToolset.BootstrapperApplications.wixext', '-d', "Msi=$msi", '-o', $setup) + $communes)

Write-Host "Installateur prêt : $setup"
