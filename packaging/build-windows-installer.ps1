param(
    [string]$Version = $(if ($env:CLIPRELAY_VERSION) { $env:CLIPRELAY_VERSION } else { "0.1.0" })
)

$ErrorActionPreference = "Stop"

Set-Location (Join-Path $PSScriptRoot "..")

$isccCandidates = @(
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
)
$iscc = $isccCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) {
    throw "Inno Setup 6 is required to build the Windows installer."
}
if (-not (Test-Path "dist\ClipRelay\ClipRelay.exe")) {
    throw "Build the Windows application before creating the installer."
}

& $iscc "/DAppVersion=$Version" "packaging\windows\ClipRelay.iss"
if ($LASTEXITCODE -ne 0) {
    throw "Inno Setup failed with exit code $LASTEXITCODE."
}

Write-Host "Built dist/ClipRelay-Setup-Windows-x64.exe"
