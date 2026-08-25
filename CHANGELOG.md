# Changelog

Notable user-facing changes are recorded here. ClipRelay has not yet published
a release from the native Rust/GPUI source tree.

## Unreleased

### Native application

- Replaced the retired Python/Qt implementation with a Rust workspace and a
  native GPUI interface.
- Added persistent multi-root workspaces, paged library browsing, folder and
  media search, repeat-aware random selection, and local navigation history.
- Added GStreamer playback with FFmpeg-based probing, thumbnails, previews,
  trimming, cropping, masks, compression, and safe atomic exports.
- Added Telegram bot and personal-account delivery plus a deliberate manual X
  composer handoff.
- Added independent delivery history, retry actions, generated-file cleanup,
  local settings, and credential-store integration.
- Added Relay, pitch-black, and full-white themes, compact density settings,
  command search, keyboard navigation, and a focused Prepare Studio.
- Added Rust CI, isolated Docker/Xvfb GUI checks, a self-contained macOS bundle
  builder, and an explicitly gated release-readiness workflow.

### Repository

- Removed the retired Python/QML application, its dependency lockfile, tests,
  and obsolete Python packaging scripts.
- Removed internal migration notes, design plans, generic templates, and
  upstream example baggage from the vendored GPUI patch.
- Consolidated public documentation around the current native application.

## Historical builds

Tags through `v0.2.0-beta.2` refer to the retired Python/Qt implementation.
Those artifacts remain historical pre-releases and are not reproducible from
the current `main` branch.
