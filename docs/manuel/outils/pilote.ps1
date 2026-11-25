<#
.SYNOPSIS
    Pilote la fenêtre Kadenz lancée par lancer.ps1 et en fait des captures.

.DESCRIPTION
    Actions, séparées par « ; » :
      z:L,H         place la fenêtre en (40,40) à la taille L x H (client compris)
      c:X,Y         clic gauche à (X,Y) relatifs au coin haut gauche de la fenêtre
      d:X,Y         double clic
      m:X,Y,N       molette en (X,Y) : N crans vers le bas (N négatif : vers le haut)
      t:texte       frappe le texte (SendKeys : + ^ % ~ ( ) { } [ ] sont à écrire entre accolades)
      k:{TAB}       touche ou combinaison (SendKeys)
      w:ms          pause
      s:nom         capture de la fenêtre dans <Sortie>\nom.png
      r:nom,X,Y,L,H capture d'une région (relative à la fenêtre)
    Les captures sont celles de l'écran : la fenêtre doit rester au premier plan et
    visible ; ne pas toucher à la souris ni au clavier pendant l'exécution.
#>
param(
    [Parameter(Mandatory = $true)][string]$Actions,
    [string]$Sortie = (Join-Path (Split-Path -Parent $PSScriptRoot) 'captures')
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices;
public class PK {
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f,uint dx,uint dy,uint d,UIntPtr e);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int n);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h,int x,int y,int w,int hh,bool r);
  [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h, int attr, out RECT r, int size);
  public struct RECT { public int L,T,R,B; }
  public static void Clic(int x,int y){ SetCursorPos(x,y); System.Threading.Thread.Sleep(150); mouse_event(2,0,0,0,UIntPtr.Zero); mouse_event(4,0,0,0,UIntPtr.Zero);}
  public static void Molette(int x,int y,int d){ SetCursorPos(x,y); System.Threading.Thread.Sleep(150); mouse_event(0x800,0,0,(uint)d,UIntPtr.Zero);}
  public static void Double(int x,int y){ Clic(x,y); System.Threading.Thread.Sleep(80); Clic(x,y);}
}
"@
New-Item -ItemType Directory -Force $Sortie | Out-Null
$racine = Join-Path $env:TEMP 'kadenz-manuel'
$proc = Get-Process kadenz -ErrorAction SilentlyContinue | Where-Object { $_.Path -like "$racine*" -and $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $proc) { throw "aucune fenêtre Kadenz lancée par lancer.ps1" }
$h = $proc.MainWindowHandle
[void][PK]::ShowWindow($h, 9)
[void][PK]::SetForegroundWindow($h)
# Bornes visibles (sans l'ombre invisible de la fenêtre) : DWMWA_EXTENDED_FRAME_BOUNDS = 9.
function Rect {
    $r = New-Object PK+RECT
    if ([PK]::DwmGetWindowAttribute($h, 9, [ref]$r, 16) -ne 0) { [void][PK]::GetWindowRect($h, [ref]$r) }
    $r
}
# Garde : aucune frappe, aucun clic ni capture si Kadenz n'est pas la fenêtre active
# (sinon l'action tomberait sur une autre application de l'utilisateur).
function Premier-Plan {
    for ($i = 0; $i -lt 5; $i++) {
        if ([PK]::GetForegroundWindow() -eq $h) { return }
        [PK]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero)
        [void][PK]::SetForegroundWindow($h)
        [PK]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
        Start-Sleep -Milliseconds 400
    }
    throw "Kadenz n'est pas la fenêtre active : action refusée (rien n'a été cliqué, tapé ni capturé)"
}
foreach ($a in $Actions -split ';') {
    $a = $a.Trim(); if (-not $a) { continue }
    $k = $a.Substring(0, 1); $v = $a.Substring(2)
    switch ($k) {
        'z' { $d = $v -split ','; [void][PK]::MoveWindow($h, 40, 40, [int]$d[0], [int]$d[1], $true); Start-Sleep -Milliseconds 600 }
        'c' { Premier-Plan; $xy = $v -split ','; $r = Rect; [PK]::Clic([int]$xy[0] + $r.L, [int]$xy[1] + $r.T); Start-Sleep -Milliseconds 450 }
        'm' { Premier-Plan; $p = $v -split ','; $r = Rect; [PK]::Molette([int]$p[0] + $r.L, [int]$p[1] + $r.T, -120 * [int]$p[2]); Start-Sleep -Milliseconds 500 }
        'd' { Premier-Plan; $xy = $v -split ','; $r = Rect; [PK]::Double([int]$xy[0] + $r.L, [int]$xy[1] + $r.T); Start-Sleep -Milliseconds 450 }
        't' { Premier-Plan; [System.Windows.Forms.SendKeys]::SendWait($v); Start-Sleep -Milliseconds 300 }
        'k' { Premier-Plan; [System.Windows.Forms.SendKeys]::SendWait($v); Start-Sleep -Milliseconds 300 }
        'w' { Start-Sleep -Milliseconds ([int]$v) }
        's' {
            Premier-Plan
            $r = Rect; $r.L += 1; $r.R -= 1; $r.B -= 2; $w = $r.R - $r.L; $hh = $r.B - $r.T
            $img = New-Object System.Drawing.Bitmap $w, $hh
            $g = [System.Drawing.Graphics]::FromImage($img)
            $g.CopyFromScreen($r.L, $r.T, 0, 0, (New-Object System.Drawing.Size $w, $hh))
            $img.Save((Join-Path $Sortie "$v.png")); $g.Dispose(); $img.Dispose()
            "capture $v (${w}x$hh)"
        }
        'r' {
            Premier-Plan
            $p = $v -split ','; $r = Rect
            $img = New-Object System.Drawing.Bitmap ([int]$p[3]), ([int]$p[4])
            $g = [System.Drawing.Graphics]::FromImage($img)
            $g.CopyFromScreen($r.L + [int]$p[1], $r.T + [int]$p[2], 0, 0, (New-Object System.Drawing.Size ([int]$p[3]), ([int]$p[4])))
            $img.Save((Join-Path $Sortie "$($p[0]).png")); $g.Dispose(); $img.Dispose()
            "capture $($p[0])"
        }
    }
}
