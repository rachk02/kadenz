param([string]$Pdf,[string]$Out,[string]$Pages,[double]$Scale=1.5)
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$null=[Windows.Data.Pdf.PdfDocument,Windows.Data.Pdf,ContentType=WindowsRuntime]
$null=[Windows.Storage.StorageFile,Windows.Storage,ContentType=WindowsRuntime]
$null=[Windows.Storage.Streams.InMemoryRandomAccessStream,Windows.Storage.Streams,ContentType=WindowsRuntime]
$asTask=([System.WindowsRuntimeSystemExtensions].GetMethods()|?{$_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1'})[0]
function Await($op,$type){$t=$asTask.MakeGenericMethod($type).Invoke($null,@($op));$t.Wait()|Out-Null;$t.Result}
$file=Await ([Windows.Storage.StorageFile]::GetFileFromPathAsync($Pdf)) ([Windows.Storage.StorageFile])
$doc=Await ([Windows.Data.Pdf.PdfDocument]::LoadFromFileAsync($file)) ([Windows.Data.Pdf.PdfDocument])
"pages: " + $doc.PageCount
New-Item -ItemType Directory -Force $Out | Out-Null
foreach($n in ($Pages -split ',')){
  $page=$doc.GetPage([uint32]($n-1))
  $ms=New-Object Windows.Storage.Streams.InMemoryRandomAccessStream
  $opt=New-Object Windows.Data.Pdf.PdfPageRenderOptions
  $opt.DestinationWidth=[uint32]($page.Size.Width*$Scale)
  $opt.DestinationHeight=[uint32]($page.Size.Height*$Scale)
  $a=$page.RenderToStreamAsync($ms,$opt); $t=$asTask2=([System.WindowsRuntimeSystemExtensions].GetMethods()|?{$_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncAction'})[0].Invoke($null,@($a)); $t.Wait()|Out-Null
  $fs=[System.IO.File]::Create((Join-Path $Out ("p{0:000}.png" -f [int]$n)))
  $ms.Seek(0)|Out-Null
  $in=[System.IO.WindowsRuntimeStreamExtensions]::AsStreamForRead($ms)
  $in.CopyTo($fs); $fs.Close(); $page.Dispose()
}
