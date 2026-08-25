# Contributing to ClipRelay

ClipRelay is a Rust workspace built on GPUI. Please keep changes focused,
protect user media and credentials, and include evidence for behavior that
cannot be proven by compilation alone.

## Development setup

Install Rust 1.96 or newer, FFmpeg/FFprobe, and the GStreamer runtime and
development files for your platform. Ubuntu and Debian development systems
need at least:

```bash
sudo apt-get install ffmpeg libgstreamer1.0-dev \
  libgstreamer-plugins-base1.0-dev gstreamer1.0-plugins-base \
  gstreamer1.0-plugins-good gstreamer1.0-libav \
  libfontconfig1-dev libxkbcommon-x11-dev libwayland-dev \
  libx11-xcb-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
```

On macOS, install the official GStreamer runtime and development packages,
then configure the SDK:

```bash
export GST_ROOT=/Library/Frameworks/GStreamer.framework/Versions/1.0
export PATH="$GST_ROOT/bin:$PATH"
export PKG_CONFIG_PATH="$GST_ROOT/lib/pkgconfig"
pkg-config --modversion gstreamer-1.0
pkg-config --modversion gstreamer-app-1.0
```

Build and run with `cargo run -p cliprelay`.

## Required checks

Run the same static and behavioral gates as CI:

```bash
cargo fmt --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
git diff --check
```

GUI changes must also pass the isolated Docker/Xvfb harness:

```bash
make ui-test
```

The harness never connects to the host display or input devices. See
[docs/GUI_TESTING.md](docs/GUI_TESTING.md) for its security boundary and
artifacts.

## Pull requests

1. Branch from `main` and keep the change narrowly scoped.
2. Add focused tests for changed domain or interaction behavior.
3. Verify UI changes at compact and default window sizes.
4. Preserve source-video immutability and generated-file boundaries.
5. Remove private filenames, captions, account identifiers, credentials, and
   media from screenshots or logs.
6. Describe user impact, exact verification, and untested platforms.

## GPUI conventions

- Keep filesystem, database, FFmpeg, GStreamer, and network work off the GPUI
  application thread.
- Retain tasks that own important async work; dropping a GPUI task cancels it.
- Virtualize unbounded media and history collections and use stable element IDs.
- Use semantic theme tokens, keyboard alternatives, visible focus, and clear
  disabled states.
- Keep platform-specific behavior behind narrow boundaries and preserve
  fallbacks on other supported targets.

The workspace patches GPUI 0.2.2 through `vendor/gpui`. Do not replace that
path dependency or refresh the vendored source without reviewing and
reapplying [its documented patches](vendor/gpui/PATCHES.md).

## Product and release boundaries

ClipRelay is a fast relay workflow, not a general-purpose nonlinear editor.
Avoid destructive source operations, silent social posting, and features that
upload a user's library to ClipRelay infrastructure.

Do not commit certificates, signing keys, service credentials, generated
installers, databases, logs, or user media. Native release artifacts are not
published until platform packaging, signing, and clean-machine launch checks
are complete.
