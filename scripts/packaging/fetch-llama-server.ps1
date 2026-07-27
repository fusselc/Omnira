# fetch-llama-server.ps1
#
# Downloads the pinned llama.cpp llama-server Windows builds (Vulkan + CPU +
# CUDA 12.4), verifies them against pinned SHA-256 checksums, and extracts
# them into apps/desktop/src-tauri/binaries/ for bundling as Tauri resources.
#
# CUDA also unpacks the matching cudart redistributable so NVIDIA users do not
# need a full CUDA Toolkit install. Pass -SkipCuda to fetch only Vulkan + CPU
# (smaller; Phase 5 alpha-compatible).
#
# SECURITY: verification fails closed. On any checksum mismatch this script
# deletes the unverified download and exits nonzero. The build must never
# bundle or distribute an unverified runtime.
#
# Usage:
#   powershell -ExecutionPolicy Bypass -File scripts/packaging/fetch-llama-server.ps1
#   powershell -ExecutionPolicy Bypass -File scripts/packaging/fetch-llama-server.ps1 -SkipCuda

param(
    [switch]$SkipCuda
)

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

if (-not $SkipCuda) {
    $Artifacts += @{
        Name    = "llama-$PinnedTag-bin-win-cuda-12.4-x64.zip"
        Sha256  = "05ae4f4f0b141a11c72dd18b58af28356725be99f2bdd1867e3787601b3de9ec"
        Variant = "cuda"
    }
    $CudartArtifact = @{
        Name   = "cudart-llama-bin-win-cuda-12.4-x64.zip"
        Sha256 = "8c79a9b226de4b3cacfd1f83d24f962d0773be79f1e7b75c6af4ded7e32ae1d6"
    }
}

$BaseUrl = "https://github.com/ggml-org/llama.cpp/releases/download/$PinnedTag"

# ---- Paths ----------------------------------------------------------------
$RepoRoot   = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$BinDir     = Join-Path $RepoRoot "apps\desktop\src-tauri\binaries"
$CacheDir   = Join-Path $RepoRoot "scripts\packaging\.cache"

New-Item -ItemType Directory -Force -Path $BinDir, $CacheDir | Out-Null

function Get-VerifiedZip([string]$Name, [string]$ExpectedSha) {
    $zipPath = Join-Path $CacheDir $Name
    $needDownload = $true
    if (Test-Path $zipPath) {
        $existing = (Get-FileHash -Algorithm SHA256 -Path $zipPath).Hash.ToLowerInvariant()
        if ($existing -eq $ExpectedSha) {
            Write-Host "[cache ] $Name already downloaded and verified."
            $needDownload = $false
        } else {
            Write-Host "[cache ] $Name cached copy fails verification; re-downloading."
            Remove-Item -Force $zipPath
        }
    }

    if ($needDownload) {
        $url = "$BaseUrl/$Name"
        Write-Host "[fetch ] $url"
        Invoke-WebRequest -Uri $url -OutFile $zipPath -UseBasicParsing
    }

    $actual = (Get-FileHash -Algorithm SHA256 -Path $zipPath).Hash.ToLowerInvariant()
    if ($actual -ne $ExpectedSha) {
        Remove-Item -Force $zipPath -ErrorAction SilentlyContinue
        Write-Error @"
CHECKSUM MISMATCH for $Name
  expected: $ExpectedSha
  actual:   $actual
The unverified download has been deleted. Packaging must not proceed until a
valid checksum matches the pinned artifact. Refusing to continue.
"@
        exit 1
    }
    Write-Host "[verify] $Name sha256 OK."
    return $zipPath
}

function Expand-RuntimeVariant([string]$ZipPath, [string]$Variant) {
    $extractDir = Join-Path $BinDir $Variant
    if (Test-Path $extractDir) { Remove-Item -Recurse -Force $extractDir }
    Expand-Archive -Path $ZipPath -DestinationPath $extractDir -Force
    Write-Host "[unpack] -> $extractDir"

    $serverExe = Get-ChildItem -Recurse -Path $extractDir -Filter "llama-server.exe" | Select-Object -First 1
    if (-not $serverExe) {
        Write-Error "llama-server.exe not found inside $ZipPath; refusing to continue."
        exit 1
    }
    if ($serverExe.DirectoryName -ne $extractDir) {
        Get-ChildItem -Path $serverExe.DirectoryName | Move-Item -Destination $extractDir -Force
    }

    Get-ChildItem -Path $extractDir -Recurse -File |
        Where-Object { $_.Name -ne "llama-server.exe" -and $_.Extension -ne ".dll" } |
        Remove-Item -Force
    Get-ChildItem -Path $extractDir -Directory | Remove-Item -Recurse -Force

    Write-Host "[ready ] ${Variant}: $(Join-Path $extractDir 'llama-server.exe')"
    return $extractDir
}

Write-Host "Pinned llama.cpp release: $PinnedTag ($PinnedCommit)"
if ($SkipCuda) {
    Write-Host "Skipping CUDA variant (-SkipCuda)."
} else {
    Write-Host "Including CUDA 12.4 variant + cudart redistributable."
}

foreach ($artifact in $Artifacts) {
    $zipPath = Get-VerifiedZip $artifact.Name $artifact.Sha256
    Expand-RuntimeVariant $zipPath $artifact.Variant | Out-Null
}

if (-not $SkipCuda) {
    $cudartZip = Get-VerifiedZip $CudartArtifact.Name $CudartArtifact.Sha256
    $cudaDir = Join-Path $BinDir "cuda"
    $cudartTmp = Join-Path $CacheDir "cudart-extract"
    if (Test-Path $cudartTmp) { Remove-Item -Recurse -Force $cudartTmp }
    Expand-Archive -Path $cudartZip -DestinationPath $cudartTmp -Force
    Get-ChildItem -Recurse -Path $cudartTmp -Filter "*.dll" | ForEach-Object {
        Copy-Item $_.FullName -Destination $cudaDir -Force
    }
    Remove-Item -Recurse -Force $cudartTmp
    Write-Host "[ready ] cuda: cudart DLLs merged into $cudaDir"
}

Write-Host ""
Write-Host "All requested runtimes fetched and verified for release $PinnedTag."
Write-Host "Binaries directory: $BinDir (gitignored -- never commit binaries)."
