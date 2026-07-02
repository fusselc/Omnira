# fetch-llama-server.ps1
#
# Downloads the pinned llama.cpp llama-server Windows builds (Vulkan + CPU),
# verifies them against pinned SHA-256 checksums, and extracts them into
# apps/desktop/src-tauri/binaries/ for bundling as Tauri resources.
#
# SECURITY: verification fails closed. On any checksum mismatch this script
# deletes the unverified download and exits nonzero. The build must never
# bundle or distribute an unverified runtime.
#
# Usage:  powershell -ExecutionPolicy Bypass -File scripts/packaging/fetch-llama-server.ps1

$ErrorActionPreference = "Stop"

# ---- Pinned release -------------------------------------------------------
# When updating: change the tag, commit, and checksums together, and update
# THIRD_PARTY_LICENSES and docs/packaging-process-model.md in the same commit.
$PinnedTag    = "b9859"
$PinnedCommit = "4fc4ec5541b243957ae5099edb67372f8f3b550e"

$Artifacts = @(
    @{
        Name    = "llama-$PinnedTag-bin-win-vulkan-x64.zip"
        Sha256  = "5e7794aa22ba34c8e223934b0b3e14cd441612f26e9f06a4a0e5f47b9e7f577b"
        Variant = "vulkan"
    },
    @{
        Name    = "llama-$PinnedTag-bin-win-cpu-x64.zip"
        Sha256  = "c9aa80f233a7d1749341860f11723b912d4cfd6eec19434c3d00bba0abc9f85c"
        Variant = "cpu"
    }
)

$BaseUrl = "https://github.com/ggml-org/llama.cpp/releases/download/$PinnedTag"

# ---- Paths ----------------------------------------------------------------
$RepoRoot   = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$BinDir     = Join-Path $RepoRoot "apps\desktop\src-tauri\binaries"
$CacheDir   = Join-Path $RepoRoot "scripts\packaging\.cache"

New-Item -ItemType Directory -Force -Path $BinDir, $CacheDir | Out-Null

Write-Host "Pinned llama.cpp release: $PinnedTag ($PinnedCommit)"

foreach ($artifact in $Artifacts) {
    $zipPath    = Join-Path $CacheDir $artifact.Name
    $extractDir = Join-Path $BinDir $artifact.Variant

    # Download (reuse cache if the cached file already verifies)
    $needDownload = $true
    if (Test-Path $zipPath) {
        $existing = (Get-FileHash -Algorithm SHA256 -Path $zipPath).Hash.ToLowerInvariant()
        if ($existing -eq $artifact.Sha256) {
            Write-Host "[cache ] $($artifact.Name) already downloaded and verified."
            $needDownload = $false
        } else {
            Write-Host "[cache ] $($artifact.Name) cached copy fails verification; re-downloading."
            Remove-Item -Force $zipPath
        }
    }

    if ($needDownload) {
        $url = "$BaseUrl/$($artifact.Name)"
        Write-Host "[fetch ] $url"
        Invoke-WebRequest -Uri $url -OutFile $zipPath -UseBasicParsing
    }

    # Verify -- FAIL CLOSED on mismatch
    $actual = (Get-FileHash -Algorithm SHA256 -Path $zipPath).Hash.ToLowerInvariant()
    if ($actual -ne $artifact.Sha256) {
        Remove-Item -Force $zipPath -ErrorAction SilentlyContinue
        Write-Error @"
CHECKSUM MISMATCH for $($artifact.Name)
  expected: $($artifact.Sha256)
  actual:   $actual
The unverified download has been deleted. Packaging must not proceed until a
valid checksum matches the pinned artifact. Refusing to continue.
"@
        exit 1
    }
    Write-Host "[verify] $($artifact.Name) sha256 OK."

    # Extract
    if (Test-Path $extractDir) { Remove-Item -Recurse -Force $extractDir }
    Expand-Archive -Path $zipPath -DestinationPath $extractDir -Force
    Write-Host "[unpack] -> $extractDir"

    # Locate llama-server.exe (release zips may nest binaries in a subfolder)
    $serverExe = Get-ChildItem -Recurse -Path $extractDir -Filter "llama-server.exe" | Select-Object -First 1
    if (-not $serverExe) {
        Write-Error "llama-server.exe not found inside $($artifact.Name); refusing to continue."
        exit 1
    }
    # Flatten so the binary and its DLLs sit at $extractDir root
    if ($serverExe.DirectoryName -ne $extractDir) {
        Get-ChildItem -Path $serverExe.DirectoryName | Move-Item -Destination $extractDir -Force
    }

    # Prune everything except llama-server.exe and its DLLs (release zips
    # include many other tools that Omnira does not ship).
    Get-ChildItem -Path $extractDir -Recurse -File |
        Where-Object { $_.Name -ne "llama-server.exe" -and $_.Extension -ne ".dll" } |
        Remove-Item -Force
    Get-ChildItem -Path $extractDir -Directory | Remove-Item -Recurse -Force

    Write-Host "[ready ] $($artifact.Variant): $(Join-Path $extractDir 'llama-server.exe')"
}

Write-Host ""
Write-Host "All runtimes fetched and verified for release $PinnedTag."
Write-Host "Binaries directory: $BinDir (gitignored -- never commit binaries)."
