# run-alpha-human-qa.ps1
#
# Interactive helper for the maintainer human-QA session.
# It asks simple yes/no questions, writes dated evidence files under
# docs/evidence/, and prints what to do next in the checklist.
#
# You must still: turn on Airplane Mode, click through Omnira, and watch
# Resource Monitor. This script records your answers - it cannot drive the UI.
#
# Usage (from repo root, in a normal PowerShell window - not while Cursor is
# the only thing that needs network if you go fully offline):
#   powershell -ExecutionPolicy Bypass -File scripts/diagnostics/run-alpha-human-qa.ps1

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$EvidenceDir = Join-Path $RepoRoot "docs\evidence"
$Date = Get-Date -Format "yyyy-MM-dd"
$Maintainer = "Chris Fussel"
$Machine = "$env:COMPUTERNAME / $($PSVersionTable.PSVersion) / $([System.Environment]::OSVersion.VersionString)"

New-Item -ItemType Directory -Force -Path $EvidenceDir | Out-Null

function Ask-YesNo([string]$Prompt) {
    while ($true) {
        $a = Read-Host "$Prompt [y/n]"
        if ($a -match '^[yY]') { return $true }
        if ($a -match '^[nN]') { return $false }
        Write-Host "Please answer y or n."
    }
}

function Write-Evidence([string]$Slug, [string]$Body) {
    $path = Join-Path $EvidenceDir "$Date-$Slug.md"
    $nl = [Environment]::NewLine
    $header = "# Evidence: $Slug" + $nl + $nl +
        "- **Date:** $Date" + $nl +
        "- **Maintainer:** $Maintainer" + $nl +
        "- **Machine:** $Machine" + $nl +
        "- **Recorded by:** scripts/diagnostics/run-alpha-human-qa.ps1" + $nl + $nl
    Set-Content -Path $path -Value ($header + $Body) -Encoding utf8
    Write-Host "Wrote $path" -ForegroundColor Green
    return $path
}

$nl = [Environment]::NewLine

Write-Host ""
Write-Host "Omnira alpha human QA helper" -ForegroundColor Cyan
Write-Host "Date: $Date"
Write-Host "Evidence folder: $EvidenceDir"
Write-Host ""
Write-Host "Before you start, you need:"
Write-Host "  1. A built installer (or already-installed Omnira under Program Files)"
Write-Host "  2. A local .gguf file on disk"
Write-Host "  3. Willingness to turn Airplane Mode ON for the offline parts"
Write-Host ""

if (-not (Ask-YesNo "Do you have Omnira installed (or an NSIS setup.exe ready) and a .gguf available?")) {
    Write-Host "Build first (needs internet):"
    Write-Host "  cd $RepoRoot\apps\desktop"
    Write-Host "  npm.cmd run tauri build"
    Write-Host ""
    Write-Host "Installer lands under:"
    Write-Host "  apps\desktop\src-tauri\target\release\bundle\nsis\"
    Write-Host ""
    Write-Host "Install it, then re-run this script."
    exit 1
}

# --- Fresh install / relaunch ---
Write-Host ""
Write-Host "=== Gate: Fresh install / relaunch ===" -ForegroundColor Cyan
Write-Host "Install Omnira if needed. Launch it. Add a .gguf. Start the model. Send a chat. Quit. Relaunch. Confirm history is there."
$fresh = Ask-YesNo "Did fresh install + chat + quit/relaunch persistence all pass?"
$freshNotes = Read-Host "Optional notes (or Enter to skip)"
$resFresh = if ($fresh) { 'Pass' } else { 'Fail' }
$b1 = "## Steps taken" + $nl +
    "1. Installed or used existing NSIS install under Program Files\Omnira" + $nl +
    "2. Launched Omnira, selected local GGUF, started runtime, sent a chat message" + $nl +
    "3. Quit and relaunched; checked conversation persistence" + $nl + $nl +
    "## Result" + $nl +
    "- Pass / Fail: $resFresh" + $nl +
    "- Notes: $freshNotes" + $nl
$null = Write-Evidence "fresh-install-relaunch" $b1

# --- Offline ---
Write-Host ""
Write-Host "=== Gate: Offline-after-install ===" -ForegroundColor Cyan
Write-Host "1. Turn ON Airplane Mode (or disable Wi-Fi and Ethernet)."
Write-Host "2. Confirm the taskbar shows no network."
Write-Host "3. Launch Omnira, chat again, quit, relaunch - still offline."
$null = Read-Host "Press Enter when Airplane Mode is ON"
$offline = Ask-YesNo "Did offline chat + relaunch succeed with network still off?"
$offlineNotes = Read-Host "Optional notes (or Enter to skip)"
$resOffline = if ($offline) { 'Pass' } else { 'Fail' }
$b2 = "## Steps taken" + $nl +
    "1. Enabled Airplane Mode / disabled network adapters before launching Omnira" + $nl +
    "2. Ran local GGUF chat while offline" + $nl +
    "3. Quit and relaunched while still offline; history present" + $nl + $nl +
    "## Result" + $nl +
    "- Pass / Fail: $resOffline" + $nl +
    "- Notes: $offlineNotes" + $nl +
    "- Networking disconnected: yes (maintainer attestation)" + $nl
$null = Write-Evidence "offline-after-install" $b2

# --- Network monitor ---
Write-Host ""
Write-Host "=== Gate: No external network calls ===" -ForegroundColor Cyan
Write-Host "1. Open Resource Monitor (resmon) -> Network tab, OR Wireshark."
Write-Host "2. With Omnira chatting (still offline is fine), watch omnira.exe and llama-server.exe."
Write-Host "3. You should see loopback/local only - no outbound internet connections."
$tool = Read-Host "Which tool did you use? (Resource Monitor / Wireshark / other)"
$net = Ask-YesNo "Confirm: no outbound internet connections from Omnira or llama-server during chat?"
$netNotes = Read-Host "Optional notes (or Enter to skip)"
$resNet = if ($net) { 'Pass' } else { 'Fail' }
$obsNet = if ($net) { 'none' } else { 'see notes' }
$b3 = "## Steps taken" + $nl +
    "1. Monitored processes during chat generation" + $nl +
    "2. Tool used: $tool" + $nl + $nl +
    "## Result" + $nl +
    "- Pass / Fail: $resNet" + $nl +
    "- Outbound connections observed: $obsNet" + $nl +
    "- Notes: $netNotes" + $nl
$null = Write-Evidence "network-monitor" $b3

# --- Prompt-free logs ---
Write-Host ""
Write-Host "=== Gate: Prompt-free logs ===" -ForegroundColor Cyan
$logDir = Join-Path $env:LOCALAPPDATA "Omnira\logs"
$latestLog = $null
if (Test-Path $logDir) {
    $latestLog = Get-ChildItem $logDir -Filter "omnira-*.log" -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
}
if ($latestLog) {
    Write-Host "Latest log: $($latestLog.FullName)"
    $excerpt = Get-Content $latestLog.FullName -Tail 40 -ErrorAction SilentlyContinue
    Write-Host "--- last 40 lines ---"
    $excerpt | ForEach-Object { Write-Host $_ }
    Write-Host "---------------------"
} else {
    Write-Host "No log file found yet under $logDir - chat once, then continue."
}
$logsOk = Ask-YesNo "Do the logs contain only lifecycle/runtime lines (no your chat text / model replies)?"
$logNotes = Read-Host "Optional notes (or Enter to skip)"
$excerptText = if ($excerpt) { ($excerpt -join $nl) } else { "(no excerpt captured)" }
$resLogs = if ($logsOk) { 'Pass' } else { 'Fail' }
$latestLogPath = if ($latestLog) { $latestLog.FullName } else { "" }
$codeFence = '```'
$b4 = "## Steps taken" + $nl +
    "1. Exercised chat in Omnira" + $nl +
    "2. Inspected log under %LOCALAPPDATA%\Omnira\logs\" + $nl + $nl +
    "## Log path" + $nl +
    "$latestLogPath" + $nl + $nl +
    "## Excerpt (tail)" + $nl +
    $codeFence + $nl +
    "$excerptText" + $nl +
    $codeFence + $nl + $nl +
    "## Result" + $nl +
    "- Pass / Fail: $resLogs" + $nl +
    "- Notes: $logNotes" + $nl
$null = Write-Evidence "prompt-free-logs" $b4

# --- Devtools ---
Write-Host ""
Write-Host "=== Gate: Devtools smoke-check ===" -ForegroundColor Cyan
Write-Host "In the installed Omnira window: right-click the UI. There should be no Inspect / DevTools."
Write-Host "Also: no unexpected remote-debugging browser attachment."
$devtools = Ask-YesNo "Confirm Inspect/devtools is NOT available on the release install?"
$devtoolsNotes = Read-Host "Optional notes (or Enter to skip)"
$resDev = if ($devtools) { 'Pass' } else { 'Fail' }
$b5 = "## Steps taken" + $nl +
    "1. Opened release/installed Omnira" + $nl +
    "2. Right-click UI; checked for Inspect / DevTools" + $nl +
    "3. Confirmed Cargo.toml does not enable tauri 'devtools' feature (already documented)" + $nl + $nl +
    "## Result" + $nl +
    "- Pass / Fail: $resDev" + $nl +
    "- Notes: $devtoolsNotes" + $nl
$null = Write-Evidence "devtools-smoke" $b5

# --- Uninstall ---
Write-Host ""
Write-Host "=== Gate: Uninstall ===" -ForegroundColor Cyan
Write-Host "1. Close Omnira."
Write-Host '2. Run C:\Program Files\Omnira\uninstall.exe (or Apps & Features).'
Write-Host "3. Confirm Program Files\Omnira is gone."
Write-Host '4. Confirm %LOCALAPPDATA%\Omnira\ still has your data.'
Write-Host "5. Confirm your .gguf file on disk still exists."
$uninstall = Ask-YesNo "Did uninstall remove Program Files but keep LOCALAPPDATA and GGUF intact?"
$uninstallNotes = Read-Host "Optional notes (or Enter to skip)"
$resUninst = if ($uninstall) { 'Pass' } else { 'Fail' }
$b6 = "## Steps taken" + $nl +
    "1. Closed Omnira" + $nl +
    "2. Ran uninstaller" + $nl +
    "3. Checked Program Files\Omnira removed" + $nl +
    "4. Checked %LOCALAPPDATA%\Omnira preserved" + $nl +
    "5. Checked GGUF file still on disk" + $nl +
    "6. Job Object orphan-check previously PASS (see 2026-07-19-orphan-check.txt)" + $nl + $nl +
    "## Result" + $nl +
    "- Pass / Fail: $resUninst" + $nl +
    "- Notes: $uninstallNotes" + $nl
$null = Write-Evidence "uninstall-orphan" $b6

# --- 13 criteria ---
Write-Host ""
Write-Host "=== Gate: 13 MVP acceptance criteria ===" -ForegroundColor Cyan
Write-Host "Based on everything you just did, answer once:"
$c13 = Ask-YesNo "Do all 13 criteria in docs/alpha-manual-verification.md pass for this build?"
$c13Notes = Read-Host "Any exceptions? (or Enter for none)"
$criteria = @(
    "1. Install/launch without terminal/Python/Docker",
    "2. Select local .gguf in place",
    "3. Managed llama-server starts",
    "4. Streaming local chat",
    "5. Stop generation works",
    "6. Quit/relaunch persistence",
    "7. No telemetry/accounts/cloud sync",
    "8. Works offline / no default external network",
    "9. Loopback + session api-key",
    "10. Main UI says Running locally",
    "11. Remove model does not delete GGUF",
    "12. Diagnostics redaction / prompt-free export",
    "13. No orphaned llama-server on force-kill (Job Object)"
)
$lines = foreach ($c in $criteria) {
    if ($c13) { "- [x] Pass: $c" } else { "- [ ] Review: $c" }
}
$criteriaJoined = $lines -join $nl
$resC13 = if ($c13) { 'Pass' } else { 'Fail / needs review' }
$b7 = "## Result" + $nl +
    "- Overall Pass / Fail: $resC13" + $nl +
    "- Exceptions: $c13Notes" + $nl + $nl +
    "## Criteria" + $nl +
    "$criteriaJoined" + $nl
$null = Write-Evidence "mvp-acceptance-13" $b7

# --- Summary ---
Write-Host ""
Write-Host "=== Done collecting your answers ===" -ForegroundColor Green
Write-Host "Evidence files written under: $EvidenceDir"
Write-Host ""
Write-Host "Tell Cursor (Agent mode):"
Write-Host "  Human QA done - update the checklist and Sign-Off from the new evidence files."
Write-Host ""
Write-Host "You can turn Airplane Mode OFF again now."
Write-Host ""

$allPass = $fresh -and $offline -and $net -and $logsOk -and $devtools -and $uninstall -and $c13
if ($allPass) {
    Write-Host "All gates you answered were PASS. After the agent updates the checklist, date Sign-Off." -ForegroundColor Green
} else {
    Write-Host "One or more gates were FAIL. Fix those before Sign-Off." -ForegroundColor Yellow
}
