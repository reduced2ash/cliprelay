# ClipRelay

[![CI](https://github.com/reduced2ash/cliprelay/actions/workflows/ci.yml/badge.svg?branch=rustgpui)](https://github.com/reduced2ash/cliprelay/actions/workflows/ci.yml?query=branch%3Arustgpui)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

ClipRelay is a local-first desktop workspace for selecting videos from large
folder trees, preparing safe derivatives, sending them to Telegram, and handing
them off to X for a deliberate manual post.

![ClipRelay library workspace](docs/screenshot.png)

> [!IMPORTANT]
> The current application is the native Rust/GPUI rewrite. Historical release
> downloads contain the retired Python/Qt build and are not produced from this
> source tree. Native release packaging is still being validated.

## Features

- Persistent workspaces for multiple video-library roots
- Recursive discovery, search, sorting, thumbnails, previews, and random picks
- Non-destructive trimming, cropping, masking, and compression
- Telegram bot and personal-account delivery
- Manual X composer handoff with copy, drag, and reveal actions
- Local SQLite history with independent platform delivery states
- Generated-file cleanup constrained to the configured export directory
- No telemetry, advertising, hosted account, or ClipRelay cloud service

See the [user guide](docs/USER_GUIDE.md) for the complete workflow.

## Build from source

ClipRelay requires Rust 1.96 or newer, FFmpeg/FFprobe, and GStreamer runtime
and development packages including the base, good, and libav plugins. Linux
development also needs the window-system libraries installed by
[CI](.github/workflows/ci.yml).

On macOS, install the official GStreamer runtime and development packages and
configure the SDK before running Cargo:

```bash
export GST_ROOT=/Library/Frameworks/GStreamer.framework/Versions/1.0
export PATH="$GST_ROOT/bin:$PATH"
export PKG_CONFIG_PATH="$GST_ROOT/lib/pkgconfig"
```

Then build and test the workspace:

```bash
cargo build --release --locked
cargo test --workspace --locked
cargo run -p cliprelay
```

The app accepts `--data-dir PATH`, `--library PATH`, `--window-width`, and
`--window-height`. `CLIPRELAY_DATA_DIR` is the environment-variable equivalent
for an isolated data directory.

## Repository layout

```text
crates/app/       GPUI desktop application
crates/core/      database, media, delivery, settings, and safety logic
packaging/        native packaging assets and the macOS bundle builder
tools/ui-test/    isolated Docker/Xvfb GUI test harness
vendor/gpui/      patched GPUI 0.2.2 source required by this workspace
```

The GPUI fork is intentionally vendored because ClipRelay relies on narrow
display-link, title-bar, and surface-capture fixes documented in
[`vendor/gpui/PATCHES.md`](vendor/gpui/PATCHES.md).

## Packaging status

`packaging/build-macos.sh` creates a self-contained app, ZIP, and DMG with
GStreamer, FFmpeg, and FFprobe embedded. Public native releases remain gated on
signed, notarized clean-machine validation and an equivalent Windows runtime
deployment. The release workflow therefore validates source only and publishes
no artifacts.

## Safety and privacy

Source videos are never opened for writing. Exports are written through a
temporary partial file, and deletion is restricted to generated files inside
the configured export directory. Library metadata, settings, and history stay
local. See [Privacy](PRIVACY.md) and [Security](SECURITY.md).

## Contributing and license

Development conventions and verification commands are in
[CONTRIBUTING.md](CONTRIBUTING.md). ClipRelay is licensed under the
[MIT License](LICENSE); bundled components retain their own licenses as listed
in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
