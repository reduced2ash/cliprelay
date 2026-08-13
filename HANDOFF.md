# ClipRelay Rust/GPUI Port — Handoff Document

**Last updated:** 2026-08-12 (after the custom-title-bar + visual polish wave)
**Branch:** `rustgpui` (pushed to `origin/rustgpui`; `main` intentionally untouched)
**Status:** The port is feature-complete, parity-verified, and mid-way through an
open-ended *visual polish* goal ("make it look better than the Python app").

---

## 1. What this project is

ClipRelay is a video library + relay tool: you point it at a folder of videos,
it indexes them (filename manifest → optional deep verify → thumbnails/previews/
timeline filmstrips), lets you pick random videos avoiding repeats, prepare them
(trim, crop, black masks, captions), and deliver them: send to a Telegram channel
(via bot token or a personal grammers session) and/or hand off to X (composer
URL + clipboard file copy).

The original is a Python/PySide6 + QML app (`src/cliprelay/` — **untouched**).
This branch adds a from-scratch native Rust port on GPUI 0.2.2 (`crates/`).
The user's directive: reach parity with the Python app, then exceed it in visual
polish — and keep iterating until it is "absolutely polished".

**Why Rust/GPUI:** native performance, no Python runtime, and a deliberate
technical bet on Zed's UI framework (gpui 0.2.2, macos-blade renderer).

---

## 2. Repo layout

```
Cargo.toml            # workspace: crates/core + crates/app
Cargo.lock
crates/
  core/               # UI-free library crate, fully unit/integration tested
    src/
      db.rs           # SQLite: settings, media_files, exports, posts, attempts;
                      #   identical schema to the Python database.py
      media.rs        # ffmpeg/ffprobe pipelines: scan, verify, thumbnails,
                      #   previews, timeline filmstrips, export (passthrough,
                      #   hardware+fallback, two-pass size-targeted, CRF)
      telegram.rs     # Bot API (blocking reqwest) + grammers 0.10 personal
                      #   session login (incl. 2FA), session persistence
      secrets.rs      # macOS keychain with 0600 JSON fallback
      settings.rs     # 32 defaults, byte-identical to the Python settings.py
      x.rs            # X composer URL + clipboard file copy + Finder reveal
      cleanup.rs      # trash guards (source-protected / outside-export-dir)
      paths.rs, utils.rs, lib.rs
    tests/media_integration.rs   # 9 ffmpeg-backed integration tests
  app/                # the GPUI binary (single App view + controller thread)
    src/
      main.rs         # App struct: render tree, event pump, key dispatch,
                      #   fields, popups, window setup, CLI args
      controller.rs   # background controller: commands, scan/publish/telegram
                      #   state machines, workspace/nav/random logic, DB writes
      state.rs        # Event + Command enums, shared structs
      render_impls.rs # header, toolbar, workspace tabs, sidebar, all popups,
                      #   command palette, activity, random-source tree
      library.rs      # virtualized grid, tiles, explorer tree, layout math
      history.rs      # history page, rows, actions, pagination
      prepare.rs      # PrepareState: trim/crop/masks/captions/draft model
      prepare_render.rs # stage (frame/transport/timeline), edit+publish
                      #   inspectors, action dock, studio mode
      settings_page.rs# settings UI, theme cards, telegram connection forms
      theme.rs        # relay / pitch_black / full_white palettes (match QML)
      widgets.rs      # field, button, checkbox, combo, text area, tooltip…
src/cliprelay/        # the ORIGINAL Python app — do not modify
CHANGELOG.md          # wave-by-wave history (this port's changes)
README.md             # build notes + CLI args
HANDOFF.md            # this file
```

**~22k lines of Rust** across the two crates.

---

## 3. Architecture decisions (why it's built this way)

- **Two crates, clean split.** `crates/core` has zero GPUI dependency and is the
  tested brain (DB, ffmpeg, Telegram, secrets). `crates/app` is the GPUI shell.
  This is why 48 of the 64 tests run without a window.
- **Controller thread + event pump.** `spawn_controller` runs one thread that
  owns all mutable controller state and processes `Command`s from a flume
  channel. Background jobs (scans, publishes, Telegram sends, thumbnail/preview
  extraction) are `std::thread`s that send `Event`s back. `main.rs` drains the
  queue at the top of every `render()` into `Arc<Mutex<VecDeque<UiMessage>>>`
  via three pump threads (events, frames, 100ms tick). Render-driven work (the
  window-size save, reveal chase) therefore runs on the first frame.
- **Round trips for cross-thread state.** Anything the controller must *decide*
  after a job finishes goes through `Event → Command`: `ScanFinished`,
  `PreviewFinished`, `SelectionVerifyFailed`, `MarkBotConfigured`,
  `LoadMoreFinished`, `TimelineFinished`, `SelectionVerified`, etc. This pattern
  fixed several stale-state races.
- **Single source of truth rules** discovered the hard way: Telegram destination
  (settings field ↔ prepare pane ↔ draft restore all sync), scan
  offset/generation (controller reserves offsets; UI applies only the exact
  next page), nav stacks (pop, don't peek).
- **Theme via thread-local.** `widgets.rs` keeps a `CURRENT_THEME` thread-local
  set at render start; widgets look up colors without plumbing. (A hardcoded
  `Theme::relay()` leak in tile posters was fixed to use the active theme.)
- **CLI/env for testing without input.** `--data-dir`, `--library`,
  `--window-width/height`; dev-only envs for screenshot sweeps:
  `CLIPRELAY_PAGE=history|settings` (applied after the boot restore so the
  seeded-draft case can be swept too), `CLIPRELAY_OPEN_COMMAND=1` +
  `CLIPRELAY_QUERY=clip` (opens the command center and runs the real search),
  `CLIPRELAY_SETTINGS_SCROLL=950` (scrolls the settings page; the offset is
  applied after the page's first frame), `CLIPRELAY_BOOT_TOAST=error:msg`
  (info:/success:/error: kinds), and `CLIPRELAY_QUERY=">scan"` switches the
  command center into commands scope (the '>' prefix mirrors the original).

---

## 4. What's done (wave summary — see CHANGELOG.md for detail)

### Parity (waves 1–36)
- Full UI surface: header, workspace tabs (bottom, like the Python), context
  toolbar, sidebar, virtualized grid (exact tile constants), explorer,
  prepare dock + studio, history, settings, random-source popup, command
  palette (38 actions), sort/workspace/activity menus, toasts, tooltips.
- Full function: scan/verify/thumbnail/preview/timeline/export pipelines
  (byte-identical ffmpeg argv), Telegram bot + personal (2FA, sessions),
  X handoff, history actions, workspace/draft persistence, navigation history,
  random pool with seen-reset, cleanup.
- Byte-level parity verified: DB schema (5 tables), 32 settings defaults,
  every user-visible string, stage labels, sort clauses, formatting helpers.
- All 67 Python `@Slot` methods mapped to Rust equivalents.

### Correctness hardening (waves 43–58, four independent review rounds)
34 real defects found and fixed, including several that had silently broken
core features:
- **Pagination was triple-broken**: offsets never advanced (scroll duplicated
  the first page), `has_more` never set true (load-more was a no-op), and a
  refresh mid-flight could stall the in-flight flag. Fixed with reserved
  offsets + `LoadMoreFinished(generation, has_more, offset)` round trips +
  `page_appendable` (generation + exact-next-offset) on the UI.
- **Back/forward navigation never popped its stacks** (same top state re-applied
  forever) and `nav_restoring` could stick on the empty-stack path.
- **Theme was force-reset to "relay" on every launch** (leftover debug write).
- **Root-path canonicalization mismatch**: scan stores `/private/tmp/…`, the DB
  stored `/tmp/…` → `active_root` reconciliation deactivated every row, so any
  library opened via `/tmp` looked empty on macOS. `normalize_path` now
  canonicalizes; reveal/nav gates compare resolved roots.
- **Scan completion never refreshed the UI** (grid/counts/explorer stayed stale).
- Hover-preview pending marks leaked (previews worked once per media per
  session); preview encoding was unbounded (now 1–2 concurrent slots);
  frame extraction during drags spawned ffmpeg per mouse-move (now throttled
  80ms + force on release).
- Element-id collisions (settings static fields, explorer/random rows),
  caption field staleness across workspace switches, double 0.98 X-limit
  margin, studio pointer math ignoring the sidebar, timeline pending never
  cleared, trash containment skipping symlink resolution, UTC timestamps in
  history, `CloseWorkspacesToRight` fallback deleting all tabs, stale
  in-flight selection checks clobbering newer selections, and more.

### Visual polish (waves 59–61 + the current goal)
- **Display-pipeline unblock**: gpui 0.2.2 only starts its CVDisplayLink when
  the window reports `NSWindowOcclusionStateVisible`; broken/headless sessions
  never do → the app froze after frame 1. A dev-only, env-gated gpui patch
  (`GPUI_FORCE_TIMER_DISPLAY=1`) starts the link at window creation and falls
  back to a 16ms timer. This machine still needs it; healthy Macs don't.
- **Visual verification loop** (see §6) — real window-only screenshots +
  vision-model audits + pixel measurements, used to drive every polish change.
- **Custom FLUSH title bar** (the user's key requirement): native title bar
  hidden (`appears_transparent`), the app's own 40px header becomes the title
  bar, traffic lights parked over it (empirically `traffic_light_position
  (18, 27)`), a drag region (gpui patched with `performWindowDragWithEvent`)
  and double-click-to-zoom on the empty header, search field with magnifier
  glyph and 96px traffic-light clearance.
- **Adaptive narrow-window layout**: header/toolbar collapse to icon-only
  below width thresholds; window minimum lowered 940→700 so half-screen
  layouts work; studio squeezes the inspector before overflowing.
- **Micro-polish**: 10px popup radius, uppercase command-section headers,
  row hover states (tiles, explorer, palette), 2px accent top edge on the
  active workspace tab, source-strip overflow fix, theme-aware tile colors.

---

## 5. The two gpui patches — VENDORED (no dev-machine dependency)

The patches now live in **`vendor/gpui/`** (a full gpui 0.2.2 copy) and are
wired in via `[patch.crates-io]` in the root `Cargo.toml`. They build and
work on any machine; `cargo clean` cannot lose them:

```
vendor/gpui/src/platform/mac/display_link.rs   # timer fallback when
                                               # CVDisplayLink fails or
                                               # GPUI_FORCE_TIMER_DISPLAY=1
vendor/gpui/src/platform/mac/window.rs         # occlusion-visible workaround
                                               # at creation +
                                               # start_window_move
                                               # (performWindowDragWithEvent)
vendor/gpui/PATCHES.md                         # documentation of both edits
```

Note: if you edit the vendored gpui you must `cargo clean -p gpui` (or the
app's target) before rebuilding — Cargo fingerprints path deps differently
but the crate still must be recompiled for the edit to take effect.

`Window::start_window_move` exists in stock gpui 0.2.2 as a no-op; the
vendored copy implements the macOS drag.
- If the machine's display state is healthy, `GPUI_FORCE_TIMER_DISPLAY` is
  unnecessary; it forces the timer even when the real link works, which is
  fine but slightly wasteful (62Hz timer + real link both firing).

---

## 6. How visual verification works on this machine (the loop)

The macOS display pipeline here reports "Internal failure" (ioreg), windows
are flagged not-on-screen, and the app opens on whichever yabai space yabai
picks. This is the proven recipe:

1. Build: `cargo build --release -p cliprelay`
2. Launch (fresh data dir avoids the pre-seed fragility — see below):
   ```
   GPUI_FORCE_TIMER_DISPLAY=1 CLIPRELAY_DATA_DIR=/tmp/cr-x ./target/release/cliprelay \
     --library /tmp/cr-lib --window-width 1460 --window-height 950
   ```
   Optionally `CLIPRELAY_PAGE=history|settings`, `CLIPRELAY_OPEN_COMMAND=1`.
3. Move to a dedicated space so captures are unobstructed:
   ```
   YID=$(yabai -m query --windows | jq -r '.[] | select(.app=="cliprelay") | .id' | head -1)
   yabai -m window $YID --space 5; yabai -m space --focus 5
   ```
   (The app often opens on a *different* space; `--space mouse` also works.)
4. Capture the full desktop with the omp host's `desktop.screenshot()` (it has
   Screen Recording TCC; the bash/terminal does not), then crop to the window:
   ```
   # bounds via CGWindowList (points); crop at 2x retina:
   swift -e 'import CoreGraphics; … CGWindowListCopyWindowInfo …'
   ffmpeg -i shot.png -vf "crop=<W*2>:<H*2>:<X*2>:<Y*2>" window.png
   ```
5. Audit with `inspect_image` — ask for pixel-level specifics, quote all text.
   **Caveat:** the vision model hallucinates image content (it once described
   a test pattern as "Fortnite gameplay") and its edge estimates are ±20px.
   Verify layout claims with pixel measurements (ffmpeg gray-row dumps) and
   ignore content hallucinations (check thumbnails directly instead).

**Pre-seeding the DB is fragile**: if the `settings` table is created by an
external python/sqlite3 before the app's first boot, the app's own writes
silently don't appear (observed once; root cause not chased — the workaround
is: boot once, kill, THEN seed/relaunch). For the prepare view, boot once
(with `--library`), read a media id, then seed
`workspace_tabs[0].selectedMediaId` and relaunch.

---

## 7. Known limitations / deliberate compromises

- **Frame-based playback** (not frame-synced seek): documented, accepted.
- **Clipboard-copy instead of native drag** for the X handoff "Drag video"
  button (gpui 0.2 has no drag-out API).
- **Timeline handle arrow-key nudge** (0.05s / Shift=1s in the Python) absent —
  IN/OUT fields and drags cover it.
- **Diagnostics placeholders** (GPU/display/frame pacing read "—"; gpui can't
  sample them).
- **The two gpui patches are not vendored** (see §5).
- **No input automation**: the omp host has `inputPermission: denied`, so
  popups that need clicks can't be driven in sweeps (the command center has a
  boot-open env; the others were audited via shared styling + the code).
- **History/More-actions menu** renders at a fixed window position (cosmetic).
- **Pre-seeded DB writes can silently fail** (see §6) — always use the
  boot-once-then-seed recipe for screenshot fixtures.
- Screen-capture TCC is granted to the omp host only; the terminal shell still
  gets "could not create image" from `screencapture`.

---

## 8. Current state (verify with these commands)

```
cargo build -p cliprelay            # 0 errors; the only "warning" is the
cargo build --release -p cliprelay  #   transitive future-incompat note
cargo test -p cliprelay             # 16 unit tests
cargo test -p cliprelay-core        # 39 unit tests
cargo test -p cliprelay-core --test media_integration   # 9 ffmpeg tests
```

Git: `rustgpui` at `7e342ce` (pushed). `main` untouched at `d61d58b`.
The Python original lives in `src/cliprelay/` and is deliberately unmodified.

---

## 9. How to continue the polish goal

The user wants the UI to be *better than* the Python app and "absolutely
polished", with the custom title bar as a hard requirement (done). Sensible
next iterations, roughly in impact order:

1. **More pages in the sweep loop**: the prepare studio mode, publish
   inspector, and the sort/workspace/activity/random popups haven't been
   screenshot-audited (they need clicks; consider boot-open envs for each, or
   grant Accessibility to the omp host for synthetic input).
2. **Interaction states**: hover/focus/active for every button and row was
   partially audited; do a focused pass on fields (focus ring), checkboxes,
   combos, and the timeline handles.
3. **Typography rhythm**: the audits flagged description text nearly the same
   size as titles in places; a consistent type scale (titles 13, meta 11,
   captions 10) would sharpen everything.
4. **Motion**: gpui 0.2 has no transitions API in the element DSL, so hover
   fades/animations would require patching; only do this if it's clearly worth
   it — instant states are already acceptable.
5. **Full-white and pitch-black theme passes**: they were captured once; re-run
   the sweep after any color change (the tile theme bug fix already touched
   both).
6. **Vendor the gpui patches** (§5) so drag + the display-timer work on any
   machine, and so the registry edits can't be silently lost by a cargo
   re-download.
7. **Release hygiene**: `cargo clippy` pedantic cleanups remain (mostly
   too-many-arguments and type-complexity); not user-visible.

### When is it "done"?
The goal says don't stop until it's better than the Python and polished.
There is no objective completion bar; the practical gate is: every page and
popup screenshot-audited with zero actionable findings, all three themes
clean, interactions consistent, tests green, and the branch pushed. The
current wave already satisfies the title-bar requirement and the first
audit sweep; keep iterating page by page until the audit lists are empty.

---

## 10. Quick reference

- App entry: `crates/app/src/main.rs` (`main()` → `App::new` → `render`).
- Commands/events: `crates/app/src/state.rs` (keep both enums in sync;
  events flow controller→UI, commands UI→controller).
- Add a cross-thread flow: emit `Event` from the worker → route in
  `main.rs` `on_event` → `Command` → handle in `controller.rs` (search for
  `TimelineFinished` as the canonical example).
- DB schema + defaults must stay byte-identical to
  `src/cliprelay/database.py` + `settings.py` (cross-compat is a hard rule).
- Theme colors must stay byte-identical to `src/cliprelay/qml/Theme.qml`.
- User-visible strings must match the Python exactly (toast/status/stage text).
