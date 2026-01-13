$ErrorActionPreference = "Stop"

# One-key dev starter:
# - Starts Vite dev server if not running (127.0.0.1:1420)
# - Reuses it via TAURI_SKIP_DEVSERVER, then launches Tauri

$repoRoot = Split-Path -Parent $PSScriptRoot
$devUrl = "http://127.0.0.1:1420"
$devHost = "127.0.0.1"
$devPort = 1420

function Test-PortReady {
  param([int]$Retries = 60, [int]$DelayMs = 500)
  for ($i = 0; $i -lt $Retries; $i++) {
    $check = Test-NetConnection -ComputerName $devHost -Port $devPort -WarningAction SilentlyContinue
    if ($check.TcpTestSucceeded) { return $true }
    Start-Sleep -Milliseconds $DelayMs
  }
  return $false
}

function Wait-DevServerReady {
  param([int]$Retries = 60, [int]$DelayMs = 500)
  for ($i = 0; $i -lt $Retries; $i++) {
    try {
      $resp = Invoke-WebRequest -Uri $devUrl -UseBasicParsing -TimeoutSec 2
      if ($resp.StatusCode -eq 200) { return $true }
    } catch {
      # ignore and retry
    }
    Start-Sleep -Milliseconds $DelayMs
  }
  return $false
}

function Start-ViteIfNeeded {
  $check = Test-NetConnection -ComputerName $devHost -Port $devPort -WarningAction SilentlyContinue
  if ($check.TcpTestSucceeded) {
    Write-Host "Vite dev server already listening on $devUrl" -ForegroundColor Green
    return $null
  }

  $cmd = Get-Command npm.cmd -ErrorAction SilentlyContinue
  $npmPath = if ($cmd) { $cmd.Source } else { $null }
  if (-not $npmPath) {
    throw "npm not found. Please install Node.js and ensure npm is in PATH."
  }

  Write-Host "Starting Vite dev server on $devUrl ..." -ForegroundColor Yellow
  $proc = Start-Process -FilePath $npmPath `
    -ArgumentList @("run","dev","--","--host",$devHost,"--port",$devPort) `
    -WorkingDirectory $repoRoot `
    -WindowStyle Minimized `
    -PassThru

  if (-not (Test-PortReady)) {
    throw "Vite dev server did not open port $devPort within timeout."
  }
  if (-not (Wait-DevServerReady)) {
    throw "Vite dev server did not respond with HTTP 200 at $devUrl within timeout."
  }

  Write-Host "Vite dev server is ready." -ForegroundColor Green
  return $proc
}

# 1) Ensure Vite running
$viteProc = Start-ViteIfNeeded

# 2) Launch Tauri reusing existing dev server
$env:TAURI_SKIP_DEVSERVER = "1"
$env:TAURI_DEV_URL = $devUrl
Write-Host "Launching Tauri with TAURI_SKIP_DEVSERVER=1 ..." -ForegroundColor Cyan

Push-Location $repoRoot
try {
  npm run tauri -- dev
} finally {
  Pop-Location
}

# 3) Keep Vite alive for next runs; do not stop the process
if ($viteProc -ne $null) {
  Write-Host "Vite continues running (PID=$($viteProc.Id)). Stop it manually when finished." -ForegroundColor Yellow
}
