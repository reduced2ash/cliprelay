# Changelog

All notable user-facing changes are documented here. ClipRelay follows
[Semantic Versioning](https://semver.org/) while it approaches a stable 1.0
release.

## [Unreleased]

### Added

- A production-ready Prepare workspace with persistent Edit and Publish
  inspectors, a visible black-mask object list, and pinned delivery actions.
- A resizable full-screen Prepare layout with one shared player and editor,
  per-tab inspector scroll positions, and a compact 38-pixel context bar.
- Command-center actions for opening Prepare full screen, focusing either
  inspector, resetting the current cut, and revealing the selected video.
- Persistent, full-width workspace tabs with independent library roots,
  Prepare drafts, random-source filters, and browser-style navigation history.
- Workspace context actions for rename, duplicate, reveal, close variants, and
  reopening recently closed tabs.
- Theme-matched frameless window chrome with platform-aware controls,
  keyboard-accessible window actions, and a compact native-scale title bar.
- System window movement with a direct geometry fallback, plus all eight edge
  and corner resize zones on macOS and Windows.
- macOS window shadows, rounded windowed corners, double-click zoom, standard
  green-button full screen, and Option-click zoom behavior.
- Optional Maximum performance mode with persistent graphics resources,
  adjacent-media preloading, and higher bounded thumbnail concurrency.
- Live renderer, GPU, refresh-rate, decoder, export-encoder, and frame-pacing
  diagnostics.
- Hardware H.264 export through VideoToolbox on macOS and compatible Windows
  encoders, with automatic libx264 fallback.

### Changed

- Reorganized Prepare so mode and panel-size changes preserve playback,
  trimming, frame edits, publishing fields, and active operation state.
- Moved startup root activation, filtered library and history queries, folder
  aggregation, count refreshes, and thumbnail asset bookkeeping off the UI
  thread.

### Fixed

- Made title-bar dragging and frameless resizing dependable even when the Qt
  platform backend does not provide a native operation.
- Made shared dropdown triggers close an open menu instead of immediately
  reopening it.
- Added faster eased mouse-wheel momentum throughout the app while preserving
  precise native trackpad scrolling.

## [0.1.0-beta.2] - 2026-07-26

### Added

- Public macOS and Windows release automation.
- Self-contained DMG, ZIP, Windows installer, and portable Windows packages.
- Optional Developer ID, notarization, and Authenticode release signing.
- Release checksums and dependency build information.
- Previous-random navigation and previous/next video controls.
- Keyboard shortcuts for playback, selection, random picks, search, and pages.

### Changed

- Rebuilt the random-source picker for large folder trees with search,
  breadcrumbs, counts, selected-only filtering, and scroll-preserving
  incremental selection.
- Made adjacent-video navigation follow the visible library sort and filter
  without blocking the interface.

## [0.1.0-beta.1] - 2026-07-26

### Added

- Recursive large-library browsing and fast random selection.
- Folder-scoped random selection with repeat avoidance.
- Thumbnail and hover-preview generation.
- Trimming, compression, crop, and black shape masks.
- Telegram bot and personal-account delivery.
- Manual X browser handoff with caption prefilling and file paste or drag.
- Local post history, retry actions, and safe generated-file cleanup.
- Relay, pitch-black, and full-white themes.
- Collapsible navigation and expanded or full-screen Prepare workspaces.

### Fixed

- Large-library startup and thumbnail queue responsiveness.
- Full-frame thumbnails and Prepare previews.
- TypeScript `.ts` and `.mts` files being mistaken for transport-stream video.

[Unreleased]: https://github.com/reduced2ash/cliprelay/compare/v0.1.0-beta.2...HEAD
[0.1.0-beta.2]: https://github.com/reduced2ash/cliprelay/compare/v0.1.0-beta.1...v0.1.0-beta.2
[0.1.0-beta.1]: https://github.com/reduced2ash/cliprelay/releases/tag/v0.1.0-beta.1
