# Releasing

GitHub Actions builds a release whenever a tag matching `v*` is pushed.
Releases should be created from a clean, tested `main` branch.

## Release outputs

- macOS Apple silicon DMG and ZIP
- macOS Intel DMG and ZIP
- Windows x64 installer
- Windows x64 portable ZIP
- FFmpeg build and license information
- Python dependency manifest
- `SHA256SUMS.txt`

Every application package contains Python, Qt, FFmpeg, and FFprobe.

## Versioning

The version is declared in both:

- `pyproject.toml`
- `src/cliprelay/__init__.py`

Update both, update `CHANGELOG.md`, and commit before tagging.

```bash
git tag -s v0.2.0 -m "ClipRelay 0.2.0"
git push origin v0.2.0
```

Unsigned beta tags can use a suffix such as `v0.2.0-beta.1`. The release
workflow accepts a suffix when its numeric prefix matches the application
version and marks the resulting GitHub release as a pre-release.

## macOS signing and notarization

Public stable releases should use a Developer ID Application certificate and
Apple notarization. Configure these GitHub Actions repository secrets:

| Secret | Value |
| --- | --- |
| `MACOS_CERTIFICATE_P12` | Base64-encoded Developer ID Application `.p12` |
| `MACOS_CERTIFICATE_PASSWORD` | Password protecting the `.p12` |
| `MACOS_SIGN_IDENTITY` | Full Developer ID Application identity |
| `APPLE_ID` | Apple account used for notarization |
| `APPLE_APP_PASSWORD` | App-specific password |
| `APPLE_TEAM_ID` | Apple Developer team ID |

The workflow imports the certificate into a temporary keychain, enables the
hardened runtime, submits both the application and final disk image with
`notarytool`, staples their tickets, and removes the temporary keychain.

If these secrets are absent, the workflow creates an ad-hoc-signed macOS
artifact suitable for pre-release testing. Do not describe that artifact as a
notarized stable build.

## Windows signing

Configure these repository secrets:

| Secret | Value |
| --- | --- |
| `WINDOWS_CERTIFICATE_PFX` | Base64-encoded Authenticode `.pfx` |
| `WINDOWS_CERTIFICATE_PASSWORD` | Password protecting the `.pfx` |

The workflow signs `ClipRelay.exe` before building the installer, then signs
the installer. Without these secrets, the installer is suitable for
pre-release testing but can trigger SmartScreen.

## FFmpeg redistribution

Release jobs record `ffmpeg -version` and `ffmpeg -L` for every platform.
Review those files before promoting a release. Builds with GPL-enabled codecs
must retain the appropriate GPL notices and corresponding-source obligations.
See `THIRD_PARTY_NOTICES.md`.

## Local packaging

macOS:

```bash
CLIPRELAY_REQUIRE_FFMPEG=1 packaging/build-macos.sh
```

Windows PowerShell:

```powershell
$env:CLIPRELAY_REQUIRE_FFMPEG = "1"
.\packaging\build-windows.ps1
.\packaging\build-windows-installer.ps1
```

## Verification

Before publishing:

1. Run the complete test suite.
2. Confirm both media binaries are inside every application bundle.
3. Verify macOS code signatures and notarization tickets.
4. Verify Windows Authenticode signatures.
5. Install on clean target machines without Python or FFmpeg.
6. Test nested library discovery, editing, Telegram delivery, and X handoff.
7. Compare all release files with `SHA256SUMS.txt`.
