# Architecture

ClipRelay is a native Rust desktop application. The workspace separates the
GPUI interface from reusable domain and service code while keeping local
SQLite storage as the source of truth.

## Workspace

| Area | Responsibility |
| --- | --- |
| `crates/app` | GPUI startup, views, interaction state, playback, and platform integration |
| `crates/core` | database, media, settings, secrets, Telegram, X handoff, and cleanup |
| `vendor/gpui` | patched GPUI 0.2.2 source pinned by the root Cargo patch |
| `tools/ui-test` | isolated launch, workflow, and render-capture checks |

The app owns presentation state and sends long-running work to background
workers. The core crate remains independent of GPUI so database, media, safety,
and delivery behavior can be tested directly.

## Library model

Filename discovery, metadata verification, thumbnails, and hover previews are
separate stages:

1. A recursive scan builds a lightweight persistent manifest.
2. SQLite serves paged media rows, folder summaries, search, and random picks.
3. Bounded workers validate media and generate visual derivatives.
4. The GPUI layer applies results only when they still belong to the active
   workspace and generation.

This keeps startup, navigation, and random selection responsive for large
libraries.

## Prepare and playback

GStreamer provides in-process playback. FFmpeg and FFprobe run as external
processes for probing, thumbnails, previews, and exports. Editing state stays
non-destructive until an export is requested.

The docked Prepare panel and full Studio view share the same live state:
selection, playback position, trim, crop, masks, captions, compression,
destinations, progress, and errors.

## Persistence and delivery

SQLite stores library roots, media metadata, workspaces, settings, post
history, and per-platform delivery attempts. Telegram delivery and X handoff
have independent states, so one platform can succeed without hiding a failure
or incomplete action on the other.

Secrets use the operating-system credential store where available, with a
permission-restricted local fallback. The fallback and database are local
application data and must never be attached to public reports.

## Safety boundaries

- Source videos are opened read-only.
- FFmpeg writes to a partial output before the final atomic move.
- Cancellation and failure remove incomplete generated output.
- Cleanup requires a database record marked as generated.
- Cleanup resolves the target and verifies it is inside the configured export
  directory before moving it to Trash.

## Platform and packaging boundaries

Shared behavior lives in the workspace crates. Platform-specific window,
clipboard, and packaging work is kept behind target-specific Rust modules or
scripts. The macOS packager embeds GStreamer and media tools; Windows native
runtime deployment is not yet considered release-ready.
