<#
.SYNOPSIS
    Lance Kadenz sur des données d'exemple isolées, pour les captures du manuel.

.DESCRIPTION
    Rien n'est lu ni écrit dans %APPDATA%\Kadenz ni %LOCALAPPDATA%\Kadenz réels :
    KADENZ_BASE, LOCALAPPDATA et APPDATA pointent vers un dossier de travail à part
    (par défaut %TEMP%\kadenz-manuel\<nom>). Le programme est copié à côté de ce
    dossier pour pouvoir y poser la marque de l'édition monoposte sans toucher au
    dossier target.

.PARAMETER Edition
    monoposte (marque edition-monoposte posée) ou multiposte.

.PARAMETER Nom
    Nom du jeu de données ; un nom neuf donne une installation vierge (assistant).

.PARAMETER Langue
    Code de langue de l'interface avant connexion (fr par défaut).

.PARAMETER Remise
    Supprime le jeu de données du même nom avant de lancer.
#>
param(
    [ValidateSet('monoposte', 'multiposte')]
    [string]$Edition = 'monoposte',
    [string]$Nom = 'vierge',
    [string]$Langue = 'fr',
    [switch]$Remise,
    [string]$Exe = (Join-Path (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot))) 'target\release\kadenz.exe')
)
$ErrorActionPreference = 'Stop'
if (-not (Test-Path $Exe)) { throw "kadenz.exe introuvable : $Exe" }
$racine = Join-Path $env:TEMP 'kadenz-manuel'
$dossier = Join-Path $racine $Nom
if ($Remise -and (Test-Path $dossier)) {
    Get-Process kadenz -ErrorAction SilentlyContinue | Where-Object { $_.Path -like "$racine*" } | Stop-Process -Force
    Start-Sleep -Milliseconds 500
    Remove-Item -Recurse -Force $dossier
}
$prog = Join-Path $dossier 'programme'
$donnees = Join-Path $dossier 'donnees'
New-Item -ItemType Directory -Force $prog, $donnees, (Join-Path $dossier 'local'), (Join-Path $dossier 'roaming') | Out-Null
$copie = Join-Path $prog 'kadenz.exe'
if (-not (Test-Path $copie) -or (Get-Item $copie).LastWriteTime -lt (Get-Item $Exe).LastWriteTime) {
    Copy-Item $Exe $copie -Force
}
$marque = Join-Path $prog 'edition-monoposte'
if ($Edition -eq 'monoposte') { if (-not (Test-Path $marque)) { New-Item -ItemType File $marque | Out-Null } }
elseif (Test-Path $marque) { Remove-Item $marque }

$env:KADENZ_BASE = Join-Path $donnees 'kadenz.db'
$env:KADENZ_LANGUE = $Langue
$env:LOCALAPPDATA = Join-Path $dossier 'local'
$env:APPDATA = Join-Path $dossier 'roaming'
Start-Process -FilePath $copie -WorkingDirectory $prog
Write-Host "Kadenz lancé ($Edition), données : $dossier"
