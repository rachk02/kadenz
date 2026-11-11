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
$communes += @('-d', "Extension=$baf")

Executer wix (@('build', (Join-Path $PSScriptRoot 'Kadenz.wxs'), '-d', "Exe=$exe", '-o', $msi) + $communes)
# Contrôles ICE de Windows Installer. ICE43 et ICE57 sont écartés : ils supposent
# une installation par compte (raccourcis du menu Démarrer et du bureau dans le
# profil), alors que le paquet est installé pour tout le poste (ALLUSERS=1), où
# ces dossiers sont ceux de tous les comptes.
Executer wix @('msi', 'validate', '-sice', 'ICE43', '-sice', 'ICE57', $msi)
Executer wix (@('build', (Join-Path $PSScriptRoot 'KadenzSetup.wxs'), '-ext', 'WixToolset.BootstrapperApplications.wixext', '-d', "Msi=$msi", '-o', $setup) + $communes)

Write-Host "Installateur prêt : $setup"
