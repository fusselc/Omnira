# offline-smoke-test.ps1
#
# Defines how to verify Omnira works with no network after installation.
# This is a guided smoke test, not a fully automated end-to-end runner.
#
# Prerequisites (manual):
#   - Omnira installed from a release build (NSIS installer or equivalent)
#   - Bundled llama-server runtimes present (from installer)
#   - At least one valid local .gguf registered in Omnira
#
# Usage:
#   powershell -ExecutionPolicy Bypass -File scripts/diagnostics/offline-smoke-test.ps1
#   powershell -ExecutionPolicy Bypass -File scripts/diagnostics/offline-smoke-test.ps1 -InstallDir "C:\Program Files\Omnira"

param(
    [string]$InstallDir = "",
    [switch]$SkipNetworkAdapterCheck
)

$ErrorActionPreference = "Stop"

function Write-Step($n, $text) {
    Write-Host ""
    Write-Host "=== Step $n : $text ===" -ForegroundColor Cyan
}

function Test-AdapterConnected {
    $up = Get-NetAdapter -ErrorAction SilentlyContinue | Where-Object { $_.Status -eq "Up" }
    return ($null -ne $up -and @($up).Count -gt 0)
}

Write-Host "Omnira offline-after-install smoke test"
Write-Host "Date: $(Get-Date -Format o)"
Write-Host ""

# ---------------------------------------------------------------------------
# Step 0: Preconditions
# ---------------------------------------------------------------------------
Write-Step 0 "Preconditions"

Write-Host @"
Confirm before continuing:
  [ ] Omnira is installed (not just 'tauri dev')
  [ ] A local .gguf is registered and was used successfully at least once online
  [ ] You can locate the Omnira executable (Start Menu shortcut or -InstallDir)
"@

if ($InstallDir -ne "") {
    $exe = Join-Path $InstallDir "Omnira.exe"
    if (-not (Test-Path $exe)) {
        Write-Warning "Omnira.exe not found at $exe -- adjust -InstallDir or launch manually."
    } else {
        Write-Host "Found: $exe" -ForegroundColor Green
    }
}

$dataDir = Join-Path $env:LOCALAPPDATA "Omnira"
Write-Host "Expected data directory: $dataDir"
if (Test-Path $dataDir) {
    Write-Host "  Data directory exists." -ForegroundColor Green
} else {
    Write-Warning "  Data directory missing -- run Omnira once before this test."
}

# ---------------------------------------------------------------------------
# Step 1: Disconnect networking
# ---------------------------------------------------------------------------
Write-Step 1 "Disconnect networking"

if (-not $SkipNetworkAdapterCheck) {
    if (Test-AdapterConnected) {
        Write-Warning @"
Network adapters appear UP. For a valid offline test, disconnect Wi-Fi/Ethernet
or enable airplane mode, then re-run from Step 2.

To continue anyway (not a true offline test), pass -SkipNetworkAdapterCheck.
"@
        exit 2
    }
    Write-Host "No UP network adapters detected." -ForegroundColor Green
} else {
    Write-Warning "Skipping adapter check (-SkipNetworkAdapterCheck)."
}

# ---------------------------------------------------------------------------
# Step 2: Launch Omnira
# ---------------------------------------------------------------------------
Write-Step 2 "Launch Omnira (manual)"

Write-Host @"
Launch Omnira from the Start Menu or:
  Start-Process -FilePath '<path-to>\Omnira.exe'

Verify:
  [ ] App opens without network error dialogs
  [ ] Settings still shows local-first / offline copy
"@

Read-Host "Press Enter after Omnira is running"

# ---------------------------------------------------------------------------
# Step 3: Model and chat (manual)
# ---------------------------------------------------------------------------
Write-Step 3 "Model + chat workflow (manual)"

Write-Host @"
In the UI (network still off):
  [ ] Models screen shows your registered GGUF (not 'missing')
  [ ] Start/use the model -- runtime reaches 'Running locally'
  [ ] Open Chat, send a short message, receive a streamed reply
  [ ] Stop generation mid-stream (optional)
  [ ] Quit Omnira completely
"@

Read-Host "Press Enter after chat workflow succeeded"

# ---------------------------------------------------------------------------
# Step 4: Relaunch persistence (manual)
# ---------------------------------------------------------------------------
Write-Step 4 "Relaunch persistence (manual)"

Write-Host @"
Relaunch Omnira (still offline):
  [ ] Previous conversation(s) still listed
  [ ] Message history intact
  [ ] Chat works again without reconnecting to the internet
"@

Read-Host "Press Enter after relaunch check succeeded"

# ---------------------------------------------------------------------------
# Step 5: Optional process/network observation
# ---------------------------------------------------------------------------
Write-Step 5 "Optional: runtime process check"

$omnira = Get-Process -Name "Omnira" -ErrorAction SilentlyContinue
$llama = Get-Process -Name "llama-server" -ErrorAction SilentlyContinue
Write-Host "Omnira processes: $(@($omnira).Count)"
Write-Host "llama-server processes: $(@($llama).Count)"
Write-Host @"
For stronger evidence, use Resource Monitor or Wireshark during Step 3 and
confirm Omnira/llama-server do not initiate outbound connections.

Note: this script does not automate UI interaction; see docs/alpha-readiness-checklist.md.
"@

Write-Host ""
Write-Host "Offline smoke test steps completed." -ForegroundColor Green
Write-Host "Record pass/fail in docs/alpha-readiness-checklist.md before alpha release."
