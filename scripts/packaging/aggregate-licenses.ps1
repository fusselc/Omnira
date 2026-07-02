# aggregate-licenses.ps1
#
# Collects Rust crate and npm dependency license metadata for packaging.
# The hand-maintained llama.cpp section stays in repo-root THIRD_PARTY_LICENSES;
# this script writes a generated appendix for dependency licenses.
#
# Usage:
#   powershell -ExecutionPolicy Bypass -File scripts/packaging/aggregate-licenses.ps1
#
# Output:
#   THIRD_PARTY_LICENSES.dependencies  (repo root)
#
# Requirements:
#   - Rust toolchain + cargo
#   - Node.js + npm (for frontend licenses)
#   - cargo-license:  cargo install cargo-license
#   - license-checker (via npx)

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$OutFile  = Join-Path $RepoRoot "THIRD_PARTY_LICENSES.dependencies"
$Desktop  = Join-Path $RepoRoot "apps\desktop"
$SrcTauri = Join-Path $Desktop "src-tauri"

Write-Host "Aggregating dependency licenses -> $OutFile"

$lines = New-Object System.Collections.Generic.List[string]
$lines.Add("DEPENDENCY LICENSE APPENDIX (GENERATED)")
$lines.Add("=======================================")
$lines.Add("")
$lines.Add("Generated: $(Get-Date -Format o)")
$lines.Add("Generator: scripts/packaging/aggregate-licenses.ps1")
$lines.Add("")
$lines.Add("Do not edit by hand. Re-run the script after dependency changes.")
$lines.Add("The llama.cpp MIT attribution lives in THIRD_PARTY_LICENSES (section 1).")
$lines.Add("")

# ---- Rust (cargo-license) ---------------------------------------------------
$lines.Add("--------------------------------------------------------------------------")
$lines.Add("Rust crate dependencies (Omnira desktop core)")
$lines.Add("--------------------------------------------------------------------------")
$lines.Add("")

$cargoLicense = Get-Command cargo-license -ErrorAction SilentlyContinue
if (-not $cargoLicense) {
    $lines.Add("SKIPPED: cargo-license not installed.")
    $lines.Add("Install: cargo install cargo-license")
    $lines.Add("Then re-run this script.")
    Write-Warning "cargo-license not found; Rust section will be marked SKIPPED."
} else {
    Push-Location $SrcTauri
    try {
        $rustReport = & cargo license --json 2>&1
        if ($LASTEXITCODE -ne 0) {
            $lines.Add("ERROR: cargo license failed:")
            $lines.Add($rustReport | Out-String)
        } else {
            $lines.Add($rustReport | Out-String)
        }
    } finally {
        Pop-Location
    }
}

$lines.Add("")

# ---- npm (license-checker) --------------------------------------------------
$lines.Add("--------------------------------------------------------------------------")
$lines.Add("JavaScript dependencies (Omnira desktop frontend)")
$lines.Add("--------------------------------------------------------------------------")
$lines.Add("")

Push-Location $Desktop
try {
    if (-not (Test-Path "node_modules")) {
        $lines.Add("SKIPPED: node_modules missing. Run: npm install")
    } else {
        $npmReport = & npx --yes license-checker --production --json 2>&1
        if ($LASTEXITCODE -ne 0) {
            $lines.Add("ERROR: license-checker failed:")
            $lines.Add($npmReport | Out-String)
        } else {
            $lines.Add($npmReport | Out-String)
        }
    }
} finally {
    Pop-Location
}

$lines.Add("")
$lines.Add("--------------------------------------------------------------------------")
$lines.Add("End of generated appendix")
$lines.Add("--------------------------------------------------------------------------")

[System.IO.File]::WriteAllText($OutFile, ($lines -join "`n"), [System.Text.UTF8Encoding]::new($false))
Write-Host "Wrote $OutFile"
Write-Host ""
Write-Host "For release packaging:"
Write-Host "  1. Ensure THIRD_PARTY_LICENSES (llama.cpp + project licenses) is current."
Write-Host "  2. Run this script and attach or merge THIRD_PARTY_LICENSES.dependencies."
Write-Host "  3. Optionally add both files to tauri bundle resources in tauri.conf.json."
