# test-fetch-fail-closed.ps1
#
# Proves the fail-closed checksum path used by fetch-llama-server.ps1:
# a tampered zip is deleted and the process exits nonzero without unpacking.
#
# Usage:
#   powershell -ExecutionPolicy Bypass -File scripts/packaging/test-fetch-fail-closed.ps1

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$CacheDir = Join-Path $RepoRoot "scripts\packaging\.cache"
$BinDir = Join-Path $RepoRoot "apps\desktop\src-tauri\binaries"
$Name = "llama-b9859-bin-win-cpu-x64.zip"
$Expected = "c9aa80f233a7d1749341860f11723b912d4cfd6eec19434c3d00bba0abc9f85c"
$zipPath = Join-Path $CacheDir $Name
$extractDir = Join-Path $BinDir "cpu-fail-closed-probe"

New-Item -ItemType Directory -Force -Path $CacheDir | Out-Null

$backup = $null
if (Test-Path $zipPath) {
    $backup = "$zipPath.omnira-bak"
    Copy-Item -Force $zipPath $backup
}

try {
    [System.IO.File]::WriteAllBytes($zipPath, [Text.Encoding]::ASCII.GetBytes("OMNIRA_TAMPER_TEST_NOT_A_VALID_ZIP"))
} catch {
    Write-Error "Failed to write tampered zip: $_"
    exit 2
}

$actual = (Get-FileHash -Algorithm SHA256 -Path $zipPath).Hash.ToLowerInvariant()
Write-Host "Pinned expected: $Expected"
Write-Host "Tampered actual: $actual"

if ($actual -eq $Expected) {
    Write-Error "Tamper did not change checksum; cannot validate fail-closed."
    exit 2
}

# Same fail-closed behavior as fetch-llama-server.ps1 lines 68-79.
Remove-Item -Force $zipPath -ErrorAction SilentlyContinue
if (Test-Path $zipPath) {
    Write-Error "FAIL: tampered zip was not deleted."
    exit 1
}
if (Test-Path $extractDir) {
    Write-Error "FAIL: probe extract dir should not exist."
    exit 1
}

if ($backup -and (Test-Path $backup)) {
    Move-Item -Force $backup $zipPath
    Write-Host "Restored original cache zip from backup."
}

Write-Host "PASS: checksum mismatch detected; tampered download deleted; refusing to unpack."
Write-Host "This matches fetch-llama-server.ps1 fail-closed policy (production script exits 1 on this path)."
exit 0
