# Contributing to ClipRelay

Thank you for helping improve ClipRelay. The current application is the Rust
GPUI workspace in `crates/`; `src/`, the QML files, and the Python packaging
scripts are retained only as the legacy implementation.

## Before opening an issue

- Search existing issues for the same behavior.
- Reproduce the problem with the latest Rust build.
- Remove private filenames, Telegram identifiers, tokens, phone numbers, and
  captions from screenshots or logs.
- For security-sensitive problems, follow [SECURITY.md](SECURITY.md) instead of
  opening a public issue.

## Development setup

Install Rust 1.96 or newer, FFmpeg/FFprobe, and the GStreamer runtime and
development files for your platform. On Ubuntu or Debian:

```bash
sudo apt-get install ffmpeg libgstreamer1.0-dev \
  libgstreamer-plugins-base1.0-dev gstreamer1.0-plugins-base \
  gstreamer1.0-plugins-good gstreamer1.0-libav
```

Then build and run the GPUI app:

```bash
git clone https://github.com/reduced2ash/cliprelay.git
cd cliprelay
cargo run -p cliprelay
```

Run the same gates as CI:

```bash
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

## Pull requests

1. Create a focused branch from the current Rust branch.
2. Keep unrelated formatting or legacy Python/QML cleanup out of the change.
3. Add or update tests for behavioral changes.
4. Preserve source-video safety and generated-file boundaries.
5. Test the affected workflow on the relevant operating system.
6. Explain user impact, validation, and known limitations in the pull request.

## GPUI conventions

- Keep filesystem, FFmpeg, database, GStreamer setup, and network work off the
  GPUI application thread.
- Retain async tasks that own important work; dropping a task cancels it.
- Virtualize unbounded media and history collections and use stable element IDs.
- Use semantic theme tokens and shared controls.
- Preserve keyboard alternatives for pointer and hover interactions.
- Keep interactive targets at least 44 logical pixels where practical.
- Use explicit delivery states and actionable error text.

## Product boundaries

ClipRelay is a relay tool, not a general-purpose nonlinear editor. Changes
should support quick selection, preparation, delivery, history, safety, or
large-library responsiveness. Avoid destructive source operations, silent
posting, decorative motion, and features that require uploading a user's
library to ClipRelay infrastructure.

## Release changes

Do not commit certificates, signing keys, API credentials, generated
installers, application databases, logs, or real user media. The legacy
Python packaging workflow must not be used to publish the Rust application;
native GPUI packaging and GStreamer deployment need platform validation.
