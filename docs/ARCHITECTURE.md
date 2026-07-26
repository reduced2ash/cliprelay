# Architecture

ClipRelay is a Python desktop application using Qt Quick through PySide6.

## Main components

| Component | Responsibility |
| --- | --- |
| `app.py` | Application lifecycle, Qt/QML setup, command-line entry point |
| `controller.py` | UI-facing orchestration, background tasks, delivery state |
| `database.py` | SQLite schema, media manifest, history, delivery attempts |
| `media.py` | Discovery, FFprobe validation, thumbnails, previews, FFmpeg exports |
| `telegram.py` | Telegram Bot API and Telethon personal-account workflows |
| `x_assist.py` | Official X composer, clipboard, drag, and reveal handoff |
| `cleanup.py` | Generated-file boundary checks and Trash operations |
| `secrets.py` | Operating-system credential store and restricted fallback |
| `qt_models.py` | Paged and virtualized Qt list models |
| `qml/` | Application shell, library, Prepare, history, settings, shared controls |

## Large-library model

Filename discovery, metadata verification, thumbnail generation, and hover
preview generation are separate stages.

1. A recursive `os.scandir` pass builds a lightweight persistent manifest.
2. SQLite provides paged media rows and folder summaries.
3. Visible tiles request thumbnails through a bounded worker queue.
4. Random selection reads from SQLite and validates only the selected clip.
5. Optional comprehensive verification and thumbnail generation run outside
   the UI thread.

This separation keeps application startup and Random responsive even when the
library contains thousands of files.

## Media safety

- Source videos are never opened for writing.
- FFmpeg writes to a `.partial.mp4` path first.
- A completed partial file is atomically moved to its final export path.
- Cancellation and errors remove the partial output.
- Cleanup requires an export record marked as generated.
- Cleanup also verifies that the target resolves inside the configured export
  directory before moving it to Trash.

## Delivery state

Posts and platform-specific attempts are stored separately. Telegram can be
sent while X remains prepared, and either platform can retain its completed
state if the other fails.

## Packaging

PyInstaller freezes the Python runtime, Qt libraries, QML, assets, application
code, and bundled media tools. GitHub Actions builds natively for each target
architecture. macOS packages are ZIP and DMG files; Windows packages are an
Inno Setup installer and a portable ZIP.
