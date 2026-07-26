param(
    [string]$Version = $(if ($env:CLIPRELAY_VERSION) { $env:CLIPRELAY_VERSION } else { "0.1.0" })
)

$ErrorActionPreference = "Stop"

Set-Location (Join-Path $PSScriptRoot "..")

uv sync --frozen --extra dev
$env:PYTHONPATH = "src"
$env:CLIPRELAY_VERSION = $Version

if (-not $env:CLIPRELAY_FFMPEG_DIR) {
    $ffmpegCommand = Get-Command ffmpeg -ErrorAction SilentlyContinue
    if ($ffmpegCommand) {
        $env:CLIPRELAY_FFMPEG_DIR = Split-Path $ffmpegCommand.Source
    }
}
if ($env:CLIPRELAY_REQUIRE_FFMPEG -eq "1") {
    $ffmpeg = Join-Path $env:CLIPRELAY_FFMPEG_DIR "ffmpeg.exe"
    $ffprobe = Join-Path $env:CLIPRELAY_FFMPEG_DIR "ffprobe.exe"
    if (-not (Test-Path $ffmpeg) -or -not (Test-Path $ffprobe)) {
        throw "FFmpeg and FFprobe are required for a distributable build."
    }
}

uv run python packaging/make_icons.py
uv run pyinstaller --noconfirm --clean packaging/ClipRelay.spec

if ($env:CLIPRELAY_REQUIRE_FFMPEG -eq "1") {
    $bundledFfmpeg = Get-ChildItem "dist\ClipRelay" -Recurse -Filter "ffmpeg.exe"
    $bundledFfprobe = Get-ChildItem "dist\ClipRelay" -Recurse -Filter "ffprobe.exe"
    if (-not $bundledFfmpeg -or -not $bundledFfprobe) {
        throw "The Windows bundle is missing FFmpeg or FFprobe."
    }
}

Write-Host "Built dist/ClipRelay/ClipRelay.exe"
