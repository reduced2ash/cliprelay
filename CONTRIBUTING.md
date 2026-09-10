# Contributing

Branch from `main`, which contains the Rust/GPUI app. The `legacy-python`
branch and its pre-release downloads are retained for reference.

## Setup

Install Rust 1.96 or newer, FFmpeg/FFprobe, and GStreamer with its development
headers and base, good, and libav plugins.

On Debian/Ubuntu:

```sh
sudo apt-get install build-essential pkg-config clang cmake libssl-dev \
  ffmpeg libfontconfig1-dev libgstreamer1.0-dev \
  libgstreamer-plugins-base1.0-dev gstreamer1.0-plugins-base \
  gstreamer1.0-plugins-good gstreamer1.0-libav libxkbcommon-x11-dev \
  libwayland-dev libx11-xcb-dev libxcb-render0-dev libxcb-shape0-dev \
  libxcb-xfixes0-dev
```

On macOS, install the official GStreamer runtime and development packages, then
point Cargo at the SDK:

```sh
export GST_ROOT=/Library/Frameworks/GStreamer.framework/Versions/1.0
export PATH="$GST_ROOT/bin:$PATH"
export PKG_CONFIG_PATH="$GST_ROOT/lib/pkgconfig"
```

On Windows, use the MSVC Rust toolchain and matching GStreamer MSVC runtime
and development packages. Put the GStreamer `bin` directory and FFmpeg/FFprobe
on `PATH`, and its `lib/pkgconfig` directory on `PKG_CONFIG_PATH`.

```sh
cargo run --locked -p cliprelay
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Run tests for the behavior you changed, for example
`cargo test -p cliprelay responsive::` or `cargo test -p cliprelay-core settings`.
Use `cargo test --workspace --locked` for broad changes and release checkpoints.
For UI changes, inspect the affected states in a running app at normal and
compact window sizes. [GUI testing](docs/GUI_TESTING.md) covers the isolated
harness. Do not include personal media, credentials, or app databases in reports.

## Code layout

- `crates/app`: GPUI views, controls, playback, and application coordination.
- `crates/core`: SQLite, media processing, settings, delivery, and cleanup.
- `vendor/gpui`: required GPUI 0.2.2 fork; document changes in its
  [patch notes](vendor/gpui/PATCHES.md).
- `packaging`: app icons and the macOS bundle builder.

Keep blocking I/O off the UI thread, preserve keyboard access, and use the
existing controls and theme tokens. Source videos must remain read-only;
cleanup must stay within the generated-file boundary.

## Packaging

On macOS, `packaging/build-macos.sh` builds an app bundle, ZIP, and DMG with
GStreamer, FFmpeg, and FFprobe embedded. Public native packages still need
signing, notarization, and clean-machine validation; Windows runtime packaging
is also unfinished. The release-readiness workflow validates source and does
not publish binaries.
