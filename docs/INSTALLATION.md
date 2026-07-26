# Installation

Release packages include the Python runtime, Qt, FFmpeg, FFprobe, and ClipRelay
itself. Installing Python or a package manager is not required.

## macOS

Choose the build matching the Mac:

- `arm64` for Apple silicon
- `x86_64` for Intel

### DMG

1. Download the matching `.dmg` from
   [GitHub Releases](https://github.com/reduced2ash/cliprelay/releases).
2. Open the disk image.
3. Drag **ClipRelay** into **Applications**.
4. Open ClipRelay from Applications.

### ZIP

1. Download and expand the matching macOS ZIP.
2. Move `ClipRelay.app` into Applications.
3. Open ClipRelay.

A release built with a Developer ID certificate and notarized by Apple should
open normally. An unsigned or ad-hoc-signed pre-release may show a Gatekeeper
warning. Use a signed release for routine use. Do not disable Gatekeeper
system-wide.

ClipRelay supports macOS 12 and newer.

## Windows

### Installer

1. Download `ClipRelay-Setup-Windows-x64.exe`.
2. Run the installer.
3. Choose whether to add a desktop shortcut.
4. Launch ClipRelay from the Start menu.

The installer uses the current user's local application directory and does not
require administrator access.

### Portable package

1. Download `ClipRelay-Windows-x64.zip`.
2. Extract the complete `ClipRelay` folder.
3. Run `ClipRelay.exe` inside that folder.

Do not move only the executable out of the portable folder. Its bundled
runtime and media tools must remain beside it.

An Authenticode-signed release should identify its publisher in Windows. An
unsigned pre-release can trigger Microsoft Defender SmartScreen. Prefer a
signed release and verify its checksum.

## Verify a download

Every release includes `SHA256SUMS.txt`.

On macOS:

```bash
shasum -a 256 -c SHA256SUMS.txt
```

On Windows PowerShell:

```powershell
Get-FileHash .\ClipRelay-Setup-Windows-x64.exe -Algorithm SHA256
```

Compare the printed value with the corresponding line in
`SHA256SUMS.txt`.

## First launch

1. Choose a library folder.
2. Wait for the lightweight filename manifest or use Random immediately after
   the first manifest batch is available.
3. Configure Telegram only if you intend to send through Telegram.
4. Confirm the generated-file folder under Settings.

The selected source library is never used as the generated-file destination.
