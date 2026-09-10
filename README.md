# ClipRelay

[![CI](https://github.com/reduced2ash/cliprelay/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/reduced2ash/cliprelay/actions/workflows/ci.yml?query=branch%3Amain)

A Rust/GPUI desktop app for browsing video libraries, preparing clips, and
sending them to Telegram or handing them off to X.

![ClipRelay library and Prepare panel](docs/screenshot.png)

- Browse, search, preview, and randomly pick videos across multiple folders.
- Trim, rotate, crop, mask, and compress without modifying the source.
- Send through a Telegram bot or personal account; open X for manual posting.
- Keep workspaces, settings, and delivery history locally in SQLite.

## Run from source

Install Rust 1.96+, FFmpeg/FFprobe, and the GStreamer runtime and development
packages with the base, good, and libav plugins. Platform setup and checks are
in [Contributing](CONTRIBUTING.md).

```sh
git clone https://github.com/reduced2ash/cliprelay.git
cd cliprelay
cargo run --release --locked -p cliprelay
```

Use `--library PATH` to open a folder or `--data-dir PATH` for separate app data.
A graphical desktop session is required.

Development happens on `main`. The retired Python/Qt version is preserved on
[`legacy-python`](https://github.com/reduced2ash/cliprelay/tree/legacy-python).
Existing release downloads are for that version; build from source for the
current app while native packaging is being validated.

## Documentation

[User guide](docs/USER_GUIDE.md) · [Development](CONTRIBUTING.md) ·
[Architecture](docs/ARCHITECTURE.md) · [GUI testing](docs/GUI_TESTING.md) ·
[Security](SECURITY.md)

[MIT License](LICENSE). See [third-party notices](THIRD_PARTY_NOTICES.md) for
dependencies and screenshot media credits.
