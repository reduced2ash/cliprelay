# ClipRelay

[![CI](https://github.com/reduced2ash/cliprelay/actions/workflows/ci.yml/badge.svg)](https://github.com/reduced2ash/cliprelay/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/reduced2ash/cliprelay?include_prereleases)](https://github.com/reduced2ash/cliprelay/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

ClipRelay is a local-first desktop workspace for choosing videos from a large
folder tree, preparing a safe derivative, sending it to Telegram, and handing
it off to X for a deliberate manual post.


![ClipRelay library workspace](docs/screenshot.png)

## Download

Get the newest build from [GitHub Releases](https://github.com/reduced2ash/cliprelay/releases).

| Platform | Recommended download |
| --- | --- |
| Apple-silicon Mac | `ClipRelay-macOS-arm64.dmg` |
| Intel Mac | `ClipRelay-macOS-x86_64.dmg` |
| Windows 10/11 | `ClipRelay-Setup-Windows-x64.exe` |
| Portable Windows | `ClipRelay-Windows-x64.zip` |

The downloads currently published by the legacy release workflow are for the
older Python/Qt implementation. The native macOS packager now creates a
self-contained GPUI app with FFmpeg and GStreamer; a public native release
remains gated on signed, notarized clean-machine validation and equivalent
Windows runtime deployment.

## What it does

- Finds videos recursively without freezing large libraries.
- Keeps multiple independent library roots open as persistent workspace tabs.
- Browses videos in flat or folder views with uncropped thumbnails and previews.
- Picks random videos from the whole library or selected folder subtrees.
- Avoids repeats until the current random pool has been exhausted.
- Trims, crops, masks, and compresses a generated copy without modifying the source.
- Supports Telegram bots and personal Telegram accounts.
- Opens X's official composer with the caption prefilled and the video ready to paste or drag.
- Keeps a local history with delivery state, retry actions, and safe generated-file cleanup.
- Offers Relay, pitch-black, and full-white themes plus compact workspace scaling.

## Quick start

1. Install ClipRelay from the latest release.
2. Choose the top-level folder containing your video archive.
   Use the bottom `+` button when you want another root open at the same time.
3. Select a video or use **Pick random**.
4. Adjust the cut, crop, masks, and compression in Prepare.
5. Choose Telegram, X, or both, then complete the relevant action.

The X workflow remains manual by design. ClipRelay opens the official browser
composer and prepares the local file, but you review and press **Post**.

The complete setup and posting instructions are in the
[User guide](docs/USER_GUIDE.md).

## Safety and privacy

ClipRelay does not modify source videos. Generated media is written to a
separate export directory through a temporary partial file, and cleanup is
restricted to generated files inside that directory.

There is no telemetry, advertising SDK, analytics service, or ClipRelay
account. Library metadata and history stay in a local SQLite database.
Telegram and X receive data only when you explicitly use their workflows.
Read the full [Privacy statement](PRIVACY.md) and
[Security policy](SECURITY.md).

## Development

### Rust (current — GPUI rewrite)

The current application is a native Rust app on top of
[GPUI](https://gpui.rs) (Zed's UI framework). Requirements:

- Rust 1.96 or newer
- FFmpeg and FFprobe on `PATH` (or `CLIPRELAY_FFMPEG_DIR` pointing at a
  directory containing both binaries)
- GStreamer runtime and development files, including the base, good, and
  libav plugins used for common video formats

On macOS, local compilation still needs an SDK, but packaged users do not.
Install both official GStreamer runtime and development packages, then use:

```bash
export GST_ROOT=/Library/Frameworks/GStreamer.framework/Versions/1.0
export PATH="$GST_ROOT/bin:$PATH"
export PKG_CONFIG_PATH="$GST_ROOT/lib/pkgconfig"
```

`packaging/build-macos.sh` builds the Rust application and embeds the complete
GStreamer framework, plugin scanner, plugins, FFmpeg, and FFprobe inside
`ClipRelay.app`. The resulting ZIP and DMG do not require a separate
GStreamer or FFmpeg installation. Set `CLIPRELAY_FFMPEG_DIR` to a directory
containing distributable, self-contained `ffmpeg` and `ffprobe` binaries.

```bash
cargo build --release
cargo test --workspace --locked
./target/release/cliprelay # or: cargo run
```

The app accepts the original's command-line arguments:
`--data-dir PATH` (application-data directory), `--library PATH`
(override the library root on launch), and `--window-width` /
`--window-height` (initial window size, minimum 700×520).

On machines where the macOS display pipeline never reports the window
visible (headless sessions, broken display state), gpui's display link
does not start and the UI freezes after the first frame. Setting
`GPUI_FORCE_TIMER_DISPLAY=1` switches to a fixed-rate render timer
(dev patch in the local gpui crate); healthy machines don't need it.

Notes:

- On macOS, the full Xcode toolchain is required to build the default Metal
  renderer; when only Command Line Tools are installed the app builds
  with the `macos-blade` renderer (see the `gpui` dependency in
  `Cargo.toml`).
- `CLIPRELAY_DATA_DIR` overrides the application-data directory (useful
  for testing); the database lives at
  `~/Library/Application Support/ClipRelay/cliprelay.sqlite3` by default.
- Media integration tests generate real videos with ffmpeg and skip
  silently when it is unavailable.
- Linux development also needs the GPUI X11/Wayland development packages;
  see [Contributing](CONTRIBUTING.md) for the CI-tested Ubuntu command.

### Python (legacy Qt implementation)

Requirements:

- macOS 12+ or Windows 10/11
- Python 3.11 through 3.13
- [uv](https://docs.astral.sh/uv/)
- FFmpeg and FFprobe for media integration tests

```bash
git clone https://github.com/reduced2ash/cliprelay.git
cd cliprelay
uv sync --frozen --extra dev
PYTHONPATH=src uv run pytest
uv run cliprelay
See [Contributing](CONTRIBUTING.md) for development conventions and
[Architecture](docs/ARCHITECTURE.md) for the major components.

## Releases

The checked-in tag workflow still packages the legacy Python/Qt branch. Do not
use it for a GPUI release. Rust CI now checks, lints, tests, and builds the
current workspace; native installers still need platform-specific GStreamer
deployment and signing validation.

The intended native release targets remain:

- Apple-silicon macOS
- Intel macOS
- Windows x64

See [Releasing](docs/RELEASING.md) for the legacy process and outstanding
native packaging work.

## License

ClipRelay source code is available under the [MIT License](LICENSE).
Packaged releases include separately licensed components such as Qt for Python
and FFmpeg. See [Third-party notices](THIRD_PARTY_NOTICES.md).
