# Packaging ClipRelay

ClipRelay uses native PyInstaller builds; it cannot cross-compile between
macOS and Windows. Tagged releases are built on Apple-silicon macOS, Intel
macOS, and Windows x64 by `.github/workflows/release.yml`.

Every public package must contain:

- The ClipRelay application and Python runtime
- Qt for Python and the locked runtime dependencies
- FFmpeg and FFprobe
- The MIT license, third-party notices, and bundled GPL/LGPL license texts

Users should not need Python, Qt, FFmpeg, Homebrew, Chocolatey, or another
developer tool after downloading a release.

## Local macOS package

```bash
export CLIPRELAY_FFMPEG_DIR="/absolute/path/to/ffmpeg/bin"
export CLIPRELAY_REQUIRE_FFMPEG=1
packaging/build-macos.sh
```

The script produces a native DMG, ZIP, and FFmpeg build-information file.
Without `CLIPRELAY_CODESIGN_IDENTITY`, the application receives an ad-hoc
development signature. Developer ID and notarization environment variables
are documented in [Releasing](docs/RELEASING.md).

## Local Windows package

```powershell
$env:CLIPRELAY_FFMPEG_DIR = "C:\absolute\path\to\ffmpeg\bin"
$env:CLIPRELAY_REQUIRE_FFMPEG = "1"
.\packaging\build-windows.ps1
.\packaging\build-windows-installer.ps1
```

The first script creates the self-contained application directory. The second
requires Inno Setup 6 and creates a per-user installer that does not require
administrator access.

## Release and legal checks

Release artifacts include `ffmpeg -version`, `ffmpeg -L`, a locked Python
dependency manifest, and SHA-256 checksums. Review the exact FFmpeg build's
license and corresponding-source obligations before promoting a beta to a
stable release. See [Third-party notices](THIRD_PARTY_NOTICES.md) and the
complete [release guide](docs/RELEASING.md).
