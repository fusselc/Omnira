# Fixture-free validation test for the packaging guard. Uses temporary
# zero-byte payload markers; no runtime downloads or repository binaries.
#
# Usage:
#   powershell -ExecutionPolicy Bypass -File scripts/packaging/test-runtime-payload-check.ps1

$ErrorActionPreference = "Stop"
$FetchScript = Join-Path $PSScriptRoot "fetch-llama-server.ps1"
$ProbeRoot = Join-Path ([IO.Path]::GetTempPath()) ("omnira-runtime-check-" + [guid]::NewGuid())
$StdoutPath = Join-Path ([IO.Path]::GetTempPath()) ("omnira-runtime-check-out-" + [guid]::NewGuid())
$StderrPath = Join-Path ([IO.Path]::GetTempPath()) ("omnira-runtime-check-err-" + [guid]::NewGuid())

function Invoke-Validation {
    $process = Start-Process -FilePath "powershell.exe" -ArgumentList @(
        "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "`"$FetchScript`"",
        "-ValidateOnly", "-BinaryDirectory", "`"$ProbeRoot`""
    ) -Wait -PassThru -NoNewWindow -RedirectStandardOutput $StdoutPath -RedirectStandardError $StderrPath
    $output = (Get-Content -LiteralPath $StdoutPath -Raw -ErrorAction SilentlyContinue) + (Get-Content -LiteralPath $StderrPath -Raw -ErrorAction SilentlyContinue)
    return @{ ExitCode = $process.ExitCode; Output = $output }
}

try {
    New-Item -ItemType Directory -Force -Path (Join-Path $ProbeRoot "cuda") | Out-Null

    $result = Invoke-Validation
    if ($result.ExitCode -eq 0 -or $result.Output -notmatch "fetch-llama-server\.ps1: required vulkan runtime payload") {
        throw "Missing Vulkan payload did not fail clearly. Exit $($result.ExitCode): $($result.Output)"
    }

    foreach ($variant in @("vulkan", "cpu")) {
        $variantDir = Join-Path $ProbeRoot $variant
        New-Item -ItemType Directory -Force -Path $variantDir | Out-Null
        [IO.File]::WriteAllBytes((Join-Path $variantDir "llama-server.exe"), [byte[]]@())
    }

    $result = Invoke-Validation
    if ($result.ExitCode -ne 0) {
        throw "Present Vulkan and CPU payloads should pass without CUDA. Exit $($result.ExitCode): $($result.Output)"
    }

    Remove-Item -LiteralPath (Join-Path $ProbeRoot "cpu\llama-server.exe")
    $result = Invoke-Validation
    if ($result.ExitCode -eq 0 -or $result.Output -notmatch "fetch-llama-server\.ps1: required cpu runtime payload") {
        throw "Missing CPU payload did not fail clearly. Exit $($result.ExitCode): $($result.Output)"
    }

    Write-Host "PASS: Vulkan and CPU are required; CUDA is optional."
} finally {
    if (Test-Path -LiteralPath $ProbeRoot) { Remove-Item -LiteralPath $ProbeRoot -Recurse -Force }
    Remove-Item -LiteralPath $StdoutPath, $StderrPath -Force -ErrorAction SilentlyContinue
}
