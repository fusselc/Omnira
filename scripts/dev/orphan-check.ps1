# orphan-check.ps1
#
# Phase 3 exit criterion: no orphaned llama-server.exe after the parent
# process is force-killed (Job Object KILL_ON_JOB_CLOSE verification).
#
# Starts the ignored `orphan_check_holds_runtime_for_kill` test (which spawns
# a supervised llama-server and then sleeps), force-kills the test process,
# and verifies every llama-server.exe it spawned dies with it.
#
# Prerequisites:
#   - Run from a development checkout on Windows.
#   - Fetched llama-server runtimes must exist under apps/desktop/src-tauri/binaries.
#   - A test GGUF path/config expected by the ignored runtime test must be available.
#
# Usage:
#   powershell -ExecutionPolicy Bypass -File scripts/dev/orphan-check.ps1
#
# If the harness cannot run in the current environment, validate manually by
# starting Omnira from an installed build, loading a local GGUF, confirming
# llama-server.exe appears, force-closing Omnira, and confirming llama-server.exe
# exits with it.

$ErrorActionPreference = "Stop"
$srcTauri = Resolve-Path (Join-Path $PSScriptRoot "..\..\apps\desktop\src-tauri")

$before = @(Get-Process -Name "llama-server" -ErrorAction SilentlyContinue | ForEach-Object { $_.Id })

Write-Host "Starting harness (spawns supervised llama-server, then sleeps)..."
Write-Host "Prerequisites: fetched runtimes and the ignored runtime test environment must be available."
$logOut = Join-Path $env:TEMP "omnira-orphan-check.out.log"
$logErr = Join-Path $env:TEMP "omnira-orphan-check.err.log"
$proc = Start-Process -FilePath "cargo" -ArgumentList @(
    "test", "--test", "runtime_spikes", "--",
    "--ignored", "orphan_check_holds_runtime_for_kill", "--nocapture"
) -WorkingDirectory $srcTauri -PassThru -WindowStyle Hidden `
    -RedirectStandardOutput $logOut -RedirectStandardError $logErr

# Wait for a NEW llama-server.exe to appear (runtime is healthy once spawned
# and past health gating; allow generous compile+load time).
$deadline = (Get-Date).AddMinutes(5)
$newServer = $null
while ((Get-Date) -lt $deadline) {
    Start-Sleep -Seconds 2
    if ($proc.HasExited) {
        Get-Content $logOut, $logErr -ErrorAction SilentlyContinue | Select-Object -Last 20
        Write-Error "Harness exited early."
        exit 1
    }
    $now = @(Get-Process -Name "llama-server" -ErrorAction SilentlyContinue | ForEach-Object { $_.Id })
    $newServer = $now | Where-Object { $before -notcontains $_ } | Select-Object -First 1
    if ($newServer) { break }
}
if (-not $newServer) { Write-Error "llama-server never appeared."; exit 1 }
Write-Host "llama-server.exe running with PID $newServer under harness PID $($proc.Id)."

# Let the server settle, then FORCE-KILL the parent (no graceful shutdown path).
Start-Sleep -Seconds 3
Write-Host "Force-killing harness process tree root (taskkill /F, no /T)..."
taskkill /F /PID $proc.Id | Out-Null

Start-Sleep -Seconds 3
$orphan = Get-Process -Id $newServer -ErrorAction SilentlyContinue
if ($orphan) {
    Write-Error "FAIL: llama-server.exe (PID $newServer) survived the parent kill. Job Object is not working."
    Stop-Process -Id $newServer -Force
    exit 1
}
Write-Host "PASS: llama-server.exe died with its parent. No orphaned processes."
