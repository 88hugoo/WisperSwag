<#
.SYNOPSIS
  Mide el consumo real de Wispr Swag en Windows: memoria y CPU del proceso (y de
  sus procesos hijos de WebView2) en reposo, con la ventana principal abierta y
  después de cerrarla.

.DESCRIPTION
  Usa un perfil portátil temporal (WISPR_SWAG_HOME) para no tocar los datos del
  usuario: onboarding completado, inicio minimizado, sin comprobar
  actualizaciones y sin inicio automático. No graba audio ni usa la red.

.EXAMPLE
  pwsh tools/measure.ps1 -Exe target\release\wispr-swag.exe -Out medicion.json
#>
param(
  [Parameter(Mandatory = $true)] [string] $Exe,
  [int] $WarmupSeconds = 20,
  [int] $IdleSeconds = 60,
  [int] $WindowSeconds = 20,
  [string] $Out = "medicion.json"
)

$ErrorActionPreference = "Stop"
$Exe = (Resolve-Path $Exe).Path
$cores = [Environment]::ProcessorCount

$profileDir = Join-Path ([IO.Path]::GetTempPath()) ("wispr-swag-medicion-" + [Guid]::NewGuid().ToString("N").Substring(0, 8))
New-Item -ItemType Directory -Force (Join-Path $profileDir "config") | Out-Null
$settings = @{
  onboarding_done = $true
  general         = @{ start_minimized = $true; check_updates = $false; launch_at_startup = $false; auto_backup = $false }
}
$settings | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 (Join-Path $profileDir "config\settings.json")
$env:WISPR_SWAG_HOME = $profileDir

function Get-Descendants([int] $RootId) {
  $all = Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId, Name
  $ids = [System.Collections.Generic.List[int]]::new()
  $ids.Add($RootId)
  $i = 0
  while ($i -lt $ids.Count) {
    $parent = $ids[$i]
    foreach ($p in $all) {
      if ($p.ParentProcessId -eq $parent -and -not $ids.Contains([int]$p.ProcessId)) { $ids.Add([int]$p.ProcessId) }
    }
    $i++
  }
  return $ids
}

function Get-Snapshot([int] $RootId) {
  $ids = Get-Descendants $RootId
  $procs = @(Get-Process -Id $ids -ErrorAction SilentlyContinue)
  $main = $procs | Where-Object Id -eq $RootId
  [pscustomobject]@{
    Time        = Get-Date
    MainWs      = [int64]$main.WorkingSet64
    MainPrivate = [int64]$main.PrivateMemorySize64
    MainCpuMs   = [double]$main.TotalProcessorTime.TotalMilliseconds
    TreeWs      = [int64](($procs | Measure-Object WorkingSet64 -Sum).Sum)
    TreePrivate = [int64](($procs | Measure-Object PrivateMemorySize64 -Sum).Sum)
    Processes   = $procs.Count
    WebView     = @($procs | Where-Object { $_.ProcessName -like "msedgewebview2*" }).Count
    Threads     = [int]$main.Threads.Count
  }
}

function Measure-Phase([string] $Name, [int] $RootId, [int] $Seconds) {
  Write-Host "Midiendo «$Name» durante $Seconds s…"
  $first = Get-Snapshot $RootId
  $samples = @()
  for ($s = 0; $s -lt $Seconds; $s++) {
    Start-Sleep -Seconds 1
    $samples += Get-Snapshot $RootId
  }
  $last = $samples[-1]
  $elapsedMs = ($last.Time - $first.Time).TotalMilliseconds
  $cpu = if ($elapsedMs -gt 0) { ($last.MainCpuMs - $first.MainCpuMs) / ($elapsedMs * $cores) * 100 } else { 0 }
  $mb = { param($b) [math]::Round($b / 1MB, 1) }
  [pscustomobject]@{
    fase                       = $Name
    segundos                   = $Seconds
    memoria_proceso_mb_media   = & $mb (($samples | Measure-Object MainWs -Average).Average)
    memoria_proceso_mb_max     = & $mb (($samples | Measure-Object MainWs -Maximum).Maximum)
    memoria_privada_mb_media   = & $mb (($samples | Measure-Object MainPrivate -Average).Average)
    memoria_total_mb_media     = & $mb (($samples | Measure-Object TreeWs -Average).Average)
    memoria_total_privada_mb   = & $mb (($samples | Measure-Object TreePrivate -Average).Average)
    cpu_proceso_pct            = [math]::Round($cpu, 3)
    cpu_ms_en_la_fase          = [math]::Round($last.MainCpuMs - $first.MainCpuMs, 1)
    procesos                   = $last.Processes
    procesos_webview2          = $last.WebView
    hilos                      = $last.Threads
  }
}

Write-Host "Iniciando $Exe (perfil temporal en $profileDir)"
$app = Start-Process -FilePath $Exe -ArgumentList "--minimized" -PassThru
Start-Sleep -Seconds $WarmupSeconds
if ($app.HasExited) { throw "La aplicación se cerró al iniciar (código $($app.ExitCode))." }

$results = @()
$results += Measure-Phase "reposo (solo bandeja y barra)" $app.Id $IdleSeconds

# Segunda ejecución: la instancia abierta muestra la ventana principal.
Start-Process -FilePath $Exe -Wait
Start-Sleep -Seconds 8
$results += Measure-Phase "ventana principal abierta" $app.Id $WindowSeconds

Start-Process -FilePath $Exe -ArgumentList "--close-window" -Wait
Start-Sleep -Seconds 10
$results += Measure-Phase "tras cerrar la ventana" $app.Id $IdleSeconds

Start-Process -FilePath $Exe -ArgumentList "--quit" -Wait
$closed = $app.WaitForExit(15000)
if (-not $closed) { Stop-Process -Id $app.Id -Force }

$report = [pscustomobject]@{
  fecha            = (Get-Date).ToString("s")
  equipo           = "$([Environment]::OSVersion.VersionString) · $cores núcleos lógicos"
  ejecutable       = $Exe
  cierre_ordenado  = $closed
  fases            = $results
}
$report | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 $Out

$table = @("| Fase | Memoria del proceso (media) | Memoria privada | Memoria total con WebView2 | CPU del proceso | Procesos WebView2 |", "|---|---|---|---|---|---|")
foreach ($r in $results) {
  $table += "| $($r.fase) | $($r.memoria_proceso_mb_media) MB | $($r.memoria_privada_mb_media) MB | $($r.memoria_total_mb_media) MB | $($r.cpu_proceso_pct) % | $($r.procesos_webview2) |"
}
$table += ""
$table += "Cierre ordenado con ``--quit``: $closed. Equipo: $($report.equipo)."
$table | ForEach-Object { Write-Host $_ }
if ($env:GITHUB_STEP_SUMMARY) {
  "## Consumo medido de Wispr Swag" | Add-Content $env:GITHUB_STEP_SUMMARY
  $table | Add-Content $env:GITHUB_STEP_SUMMARY
}
Remove-Item -Recurse -Force $profileDir -ErrorAction SilentlyContinue
if (-not $closed) { exit 1 }
