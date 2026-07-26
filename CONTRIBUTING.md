# Contributing to ClipRelay

Thank you for helping improve ClipRelay. Bug reports, focused fixes,
accessibility improvements, performance work, and platform testing are all
welcome.

## Before opening an issue

- Search existing issues for the same behavior.
- Reproduce the problem with the latest release.
- Remove private filenames, Telegram identifiers, tokens, phone numbers, and
  captions from screenshots or logs.
- For security-sensitive problems, follow [SECURITY.md](SECURITY.md) instead of
  opening a public issue.

## Development setup

```bash
git clone https://github.com/reduced2ash/cliprelay.git
cd cliprelay
uv sync --frozen --extra dev
```

Install FFmpeg and FFprobe:

```bash
# macOS
brew install ffmpeg
```

```powershell
# Windows
choco install ffmpeg
```

Run the app:

```bash
uv run cliprelay
```

Run the checks:

```bash
PYTHONPATH=src uv run pytest
PYTHONPATH=src uv run python -m compileall -q src tests
uv run pyside6-qmllint src/cliprelay/qml/Main.qml
uv build
```

## Pull requests

1. Create a focused branch from `main`.
2. Keep unrelated formatting or cleanup out of the change.
3. Add or update tests for behavioral changes.
4. Preserve source-video safety and generated-file boundaries.
5. Test the affected workflow on the relevant operating system.
6. Explain user impact, validation, and known limitations in the pull request.

## Product boundaries

ClipRelay is a relay tool, not a general-purpose nonlinear editor. Changes
should support quick selection, preparation, delivery, history, safety, or
large-library responsiveness. Avoid destructive source operations, silent
posting, decorative motion, and features that require uploading a user's
library to ClipRelay infrastructure.

## QML and Python conventions

- Keep filesystem, FFmpeg, database, and network work off the UI thread.
- Use the existing theme tokens and shared controls.
- Preserve keyboard and screen-reader alternatives for hover interactions.
- Keep interactive targets at least 44 logical pixels where practical.
- Use explicit delivery states and actionable error text.
- Prefer small service methods with tests over adding logic to QML.

## Release changes

Do not commit certificates, signing keys, API credentials, generated
installers, application databases, logs, or real user media. Maintainer release
instructions are in [docs/RELEASING.md](docs/RELEASING.md).
