# Changelog

All notable user-facing changes are documented here. ClipRelay follows
[Semantic Versioning](https://semver.org/) while it approaches a stable 1.0
release.

## [Unreleased] — Rust/GPUI rewrite

### Changed

- The entire application was rebuilt in Rust on top of GPUI (Zed's UI
  framework), replacing the Python/Qt implementation. The same design
  system, workflows, database schema, and delivery state machines were
  ported 1:1: workspace tabs, large-library scanning with paged virtualized
  tiles, random picking with avoid-repeats pools, the Prepare workspace
  (trim/crop/masks/compression via FFmpeg), Telegram bot and personal
  (MTProto) delivery, manual X handoff, relay history, and the three
  themes.
- Muted hover previews and Prepare playback are rendered from extracted
  frames (GPUI does not embed a video decoder), keeping the same
  uncropped-frame presentation.

### Added

- `crates/core`: database, settings, secrets, media pipeline, Telegram,
  X handoff, and cleanup services with unit and ffmpeg-backed integration
  tests (`cargo test`).
- `crates/app`: the GPUI application shell.
- Command center (⌘K): scope chips, quick actions, media/folder search
  suggestions with keyboard navigation; typing also filters the library.
- Command palette (⌘K/⌘⇧P): the full 30-action catalog with categories,
  shortcut hints, live enabled states, and fuzzy matching — library
  actions (pick random, reset shuffle, rescan, stop scan), navigation
  (back/forward), view (folder explorer, density), all 13 sort modes,
  workspaces (new/close/reopen), and Prepare (fullscreen, edit/publish
  focus, reset cut, reveal).
- Explorer sidebar keyboard navigation: Up/Down move between folders,
  Left collapses or goes up, Right expands or descends, Space/Enter
  opens a folder.
- Random-source popup keyboard navigation: Up/Down move through the
  tree, Space toggles a folder, Left/Right expand/collapse branches.
- Window shortcuts: ⌘F focuses search (library command center or history
  search), ⌘M minimizes, ⌃⌘F/F11 toggle fullscreen (Esc exits), ⌘⇧W
  quits, ⌘T opens the new-workspace folder picker.
- Tile badges: posted-count indicator (green check + count) and an
  activity icon for unchecked videos; tooltips show the full folder path
  on hover.
- History rows highlight on hover.
- Live diagnostics: the Settings page refreshes the diagnostics grid
  every two seconds while visible.
- Draft persistence is now debounced (400 ms) and written to the
  database independently instead of piggybacking on other commands.
- The app quits when its window closes (GPUI does not exit by default).
- The library grid is scroll-window virtualized: only tiles near the
  viewport are rendered inside a full-height spacer, and the next page
  loads only when the user scrolls to the end — large libraries no
  longer render or paginate eagerly.
- Prepare source strip shows the full source path (with tooltip)
  alongside size, duration, and resolution.
- The full-screen Prepare editor's inspector width is adjustable by
  dragging the divider (360–620 px); double-click resets to 460 px.
- Workspace tab bar scrolls horizontally when many tabs are open.
- The Prepare dock header shows an "Edited" pill when the video has
  active crop/mask/cut edits.
- Grid scroll position resets when switching folders or searches.
- Per-render string leaks in history rows and mask rows are gone
  (widget ids now take owned strings).
- Escape closes every open popup in order (random sources, command
  center, sort menu, workspace menu, activity popup, combos, fields,
  studio, fullscreen).
- Keyboard selection keeps the selected tile inside the grid viewport.
- Header back/forward buttons now reflect the workspace navigation
  history (previously they mirrored tile-neighbor availability).
- Tiles without thumbnails request them lazily as they become visible
  (matching MediaTile.requestThumbnail), so libraries indexed without
  thumbnails fill in as you scroll.
- With nothing selected, the left arrow picks the last row of the
  current sort and the right arrow the first (mirroring the original).
- The workspace tab bar "+" button opens the folder picker (it
  previously sent a command that silently did nothing).
- The history list is scroll-window virtualized like the library grid:
  only rows near the viewport render, and the next page loads when the
  user scrolls to the end.
- Closing the selected video from the full-screen editor also exits the
  studio (the mode flag previously persisted and re-opened the studio on
  the next selection).
- Library-only menus (random sources, sort, workspace, activity) close
  automatically when switching to History or Settings.
- Grid and history scroll positions reset when a fresh page arrives
  (folder/search/sort changes), preventing a blank viewport after the
  list shrinks.
- Trim handles show IN/OUT time tooltips while dragging, matching
  VideoTimeline.qml.
- The header search field shows a clear (✕) button while a query is
  active.
- The workspace tab bar keeps the active tab scrolled into view when
  cycling with Ctrl+Tab across many workspaces.
- Hover-preview generation is deduplicated: repeated hovers on the same
  tile no longer spawn concurrent ffmpeg jobs or re-extract preview
  frames (matching the original's in-flight job tracking).
- Fixed a crash when typing non-ASCII characters (emoji, accented
  letters, non-Latin layouts) into any text field: the caret was a byte
  index advanced by character counts, so the next render or edit could
  slice mid-character. Carets are now character-based everywhere, with
  unit tests covering emoji input, paste, delete, and backspace.
- Trim time fields reject non-finite values ("nan"/"inf"), which could
  otherwise poison the trim state and panic the clamp math.
- Closing all other workspaces with a stale tab id can no longer empty
  the workspace list (the never-empty invariant is restored).
- The app accepts the original's CLI arguments: `--data-dir PATH`,
  `--library PATH`, `--window-width N`, and `--window-height N`
  (minimums 940×660 enforced).
- Personal Telegram sign-in now persists the API hash at login start, so
  the chat picker and video delivery work after sign-in (previously the
  hash was never stored and every personal-mode call failed with "not
  signed in").
- Login code and 2FA password attempts can be retried: mistyped codes
  and wrong passwords no longer destroy the login tokens.
- The Telegram service worker shuts down on sign-out and the app's exit
  wait is bounded (5 s), so quitting never hangs on an in-flight
  network command.
- The hardware encoder profile now matches the original exactly:
  size-targeted presets compute the bitrate from the size budget,
  bitrate clamps (350–1600/700–6500/1200–24000 kbps), and the scale
  height follows the profile (1080/720/480) instead of always 1080.
- `ensure_metadata` stats the file first (mirroring the original):
  missing files are marked invalid immediately, and in-place changes
  re-probe instead of serving stale metadata.
- Stage frame extraction no longer floods stderr when the source is
  unreachable (ffmpeg output suppressed), and repeated extraction
  failures pause playback requests until the next seek or selection.
- The window title reflects the active library ("ClipRelay — <name>")
  and updates when the root changes.
- Library grid geometry fixed: tiles now size to `cell_width − gap` (the
  scrollbar reserve and dock width are included in the column math), so
  exactly `columns` tiles fit per row — the virtualization scroll extent
  previously assumed one more column than fit, leaving the bottom ~25%
  of large libraries unreachable and tiles jumping on window reslices.
- History rows match the original's layout (actions as a full-width
  bottom row) and the list renders all loaded rows with measured
  scroll-end pagination, removing the estimate-driven virtualization
  that could misplace rows and hide the tail.
- Signing out of the personal Telegram account no longer kills the
  service: the worker loop only exits on a dedicated shutdown command,
  so signing back in (or loading chats) works immediately after a
  sign-out.
- The secret store switches to its file fallback permanently once the OS
  keychain fails (instead of retrying every call), and the unit tests no
  longer touch the real keychain at all.
- The command palette now covers the original's full action list
  including: Choose library root, Go to Library/History/Settings
  (⌘1/⌘2/⌘,), the three theme switches, and the activity-sidebar
  toggle — all 33 registry labels are present with matching details
  and shortcuts.
- String parity pass: the output-size estimate now computes real sizes
  for original/balanced/smallest (the original's formula), the action
  dock's idle status includes "· X manual · <estimate>", a "Drag
  video" button joins the X-handoff dock (clipboard copy — no GPUI
  drag-out API), the Telegram status pill reads "configured/not
  configured", the random popup header shows INDEXING/NODES with
  collapse/expand buttons, the command center has the full empty-state
  variant set, the FILES section gained "Video library"/"Generated
  video folder" labels, a LIVE DIAGNOSTICS grid was added to
  Performance, the mask list shows "No masks"/"Clear all masks", the
  activity popup lists Random selection, workspace tabs have
  scanning/stopping/no-root tooltips, and the publish gate verifies
  unreadable media with the original's toast.
- Revealing a video now matches the original's gates and messages:
  wrong-library ("Open this video's library folder before viewing it in
  Prepare."), unavailable ("This source video is not available in the
  current library."), and not-found-in-page ("The selected video could
  not be shown in the library.").
- The command palette footer shows the live search state: SEARCHING
  (warning) while results load and "N RESULT(S)" once they arrive.
- Caption counters use thousands separators (1,024) like the original.
- Narrow windows (under 1080 px) auto-collapse the sidebar with the
  toggle disabled and the "Widen the window to expand the sidebar"
  tooltip, matching the original's narrow-window behavior.
- The window size is persisted and restored on launch (a port addition;
  explicit --window-width/--window-height arguments take precedence).
- Fixed a session-blocking state leak: after verifying an unverified
  selection, the controller's checking flag stayed set forever, so every
  later publish was rejected with "Wait for the selected video check to
  finish." The flag now resets when the check completes.
- The sidebar-toggle palette action is disabled (with the widen-window
  detail) in narrow windows, "Clear all masks" only shows with more
  than one mask, trim-handle tooltips use the precise time format, and
  the SEARCHING indicator/empty state only apply to library scopes with
  an in-flight query.
- Test coverage extended: command-center search scoping/ranking,
  the CLIPRELAY_DATA_DIR override, empty-value secret deletion, the
  caption-counter thousands grouping, the history date format, and the
  media cache key's exact payload contract (sha256 of
  `resolved|size|mtime.6f`, truncated — shared with the original's
  cache).
- History search escapes LIKE wildcards (`%`/`_`) so literal searches
  for "50%" or "discount_5" match only what they spell, with a
  regression test; keyboard-navigation neighbors are covered by a new
  test as well.
- Navigation history now records origins: browsing folders, searching,
  selecting (via tiles, keyboard, reveal, or random picks) pushes the
  previous state onto the back stack and clears the forward stack, so
  ⌘[ / the back button works as in the original; restores are guarded
  from re-recording.
- Bulk-closed workspaces (Close others / Close to right) land on the
  closed stack for reopening; the closed stack persists in append order
  so the most recently closed reopens first after a restart; a failed
  selection check clears the controller's stale selection; and the
  random-source tree toggles whole subtrees (a parent click enables or
  disables every direct-video folder beneath it, with the mode flipping
  to "all" only when every direct folder is selected).
- Random picks are guarded against concurrent runs, a new scan cancels
  a superseded one (with generation-tagged completions so a stale
  thread can't clear the newer scan), back/forward restores validate
  the media before selecting (dead rows and foreign-root media are
  skipped), and the no-selection backward arrow picks the true last row
  for every sort mode.
- The initial workspace (when no tabs are saved) now seeds its sort,
  folder-sort, and random-source settings from the legacy top-level
  settings exactly like the original; ⌃⌘F reaches the fullscreen
  shortcut before ⌘F (arm-order fix).
- Wave 37 (UI parity round, diff-driven): workspace tabs moved to the
  bottom of the window like the original; enabling crop now starts from
  a 0.84 free crop instead of a degenerate zero-width frame; the
  Telegram destination/API-ID/phone inputs and the prepare destination
  prefill from settings at launch; the X handoff row only appears when
  X was an actual destination of the last publish; R and Left/Right
  jump to the library before acting; ⌘⇧P opens the command scope and a
  leading ">" switches the command center into command mode (with the
  needle stripped and the library search left untouched); ⌘F always
  opens the global command center; history rows use the original's
  fixed 166px action column instead of a full-width bottom row; the
  library counts footer is removed; the verify helper text toggles
  with the setting; toasts replace each other instead of stacking;
  the checking strip shows an indeterminate progress bar; tile
  tooltips only appear when the name would truncate; middle-click
  closes a workspace tab; the timeline shows 0/25/50/75/100% tick
  labels; the docked prepare panel no longer duplicates the toolbar's
  header row; tile posters floor at 96px with pixel rounding; and the
  diagnostics grid gains a Refresh control alongside the real
  renderer/decoder/encoder values.
- Wave 38 (functional parity round): the Fit Telegram bot / Fit X /
  Fit both presets now actually enforce their size targets (the
  resolver was dead code and the custom-size field — usually 0 — went
  straight to the encoder, so a large clip could exceed the 50 MB bot
  limit; personal mode honors its 1950 MB ceiling and fit-both takes
  the tighter destination, exactly like the original); connecting a
  bot or signing in personally mid-session now unlocks publishing
  immediately (the configured flags round-trip instead of waiting for
  a restart); resetting the shuffle history refreshes the random-source
  counts (option building extracted into a shared helper); the history
  menu's "Show video in folder" no longer writes a junk empty-key
  setting; the X-limit field prefills from settings; and the size
  resolution logic is covered by a new unit test.
- Wave 39: prepare-state unit tests added (crop free-crop default,
  existing-crop preservation, aspect centering, caption toggle,
  time formatting, cut minimums — app suite now 13 tests), and the
  settings diagnostics section label matches the original's
  "LIVE DIAGNOSTICS".
- Wave 40: clippy cleanup (identical verify branches, clamp patterns,
  dead `retry_random` parameter removed) — build stays warning-free.
- Wave 41: history search now debounces 180 ms like the original's
  timer (stale keystrokes are dropped by a generation + text guard),
  so typing no longer queries the database per keypress.
- Wave 42 (App-shell parity round, diff-driven): the Command key on
  macOS maps to `modifiers.platform` in GPUI 0.2.2 (not `control`,
  which is the physical Control key) — every ⌘ shortcut (⌘1/⌘2/⌘,/
  ⌘F/⌘K/⌘M/⌘T/⌘W/⌘⇧W/⌘⇧P/⌘[/⌘]) now actually works; the Space key
  is named "space" (its char is " ") so play/pause and the explorer
  open action fire; caption fields finally commit — typing flows into
  the caption state, the character counter, the draft, and the publish
  payload, and fields seed from the committed value on focus (IN/OUT
  too); ⌘C/⌘X clipboard support added to text fields; every popup
  (sort, workspace, activity, random-source, history menu) closes on
  press-outside with the 180 ms reopen guard; closing the command
  center after a commands-scope session resets to the library scope
  and restores the media query; Escape cancels a pending tab rename;
  the random-source popup toggle gained the reopen guard. The timeline
  handle arrow-key nudge (0.05 s / Shift = 1 s) remains a documented
  compromise — IN/OUT fields and drag handles cover the same edits.
- Wave 43: tile posted-count badges now use thousands separators like
  the original's toLocaleString; a new random-tree test exposed two
  real bugs — an infinite loop in the random-source option builder when
  the library root contains direct videos (the root row listed itself
  as its own child) and tri-state miscalculation for folders that have
  both direct videos and children (their subtree was ignored). Both
  fixed; the tri-state now matches the original's selected-count math
  exactly (app suite now 14 tests).
- Wave 44: string-sweep parity — the random-source popup gained the
  clear-filter button ("Clear folder search" tooltip), the
  "Building source tree…" loading state with an indeterminate bar, the
  empty-tree hints ("Source folders appear as videos are indexed." /
  "Rescan the library to build the source tree."), and the
  "Parent checks include every nested folder" footer note; the settings
  theme cards gained the "Color theme" label and the
  "Theme changes apply immediately…" helper; the toolbar button is
  "Choose root" like the original; the toast's dismiss control now
  reads "Dismiss"; and the toolbar's frame-edits icon has the
  "Frame edits active" tooltip.
- Wave 45 (verification round): re-verified the remaining functional
  surfaces against the original — startup scan (incl. auto-index
  semantics), workspace activation draft restore, per-workspace draft
  save on tab switch, retry/prepare-again/mark-posted flows, scan
  message strings ("Scanning {title} for video filenames",
  "Stopping scan for …", "Scan failed", "Scan stopped. Videos found so
  far are still available."), the \x1e exact-folder scope tokens, the
  random-pick folder scoping, the settings control inventory (7
  checkboxes, Choose/Reveal, bot + personal login + chat picker +
  x-limit), the history empty-state "Open library" action, and the
  startup scan timing.
- Wave 46 (media pipeline verification): byte-compared the export
  pipeline against the original — output naming (`{stem}_{edited|prepared}_{timestamp}.mp4` with the `-N` collision counter and `.partial.mp4` atomicity), the preview extraction (cache-key filename, 12% start, 1–8s window, 640×360 veryfast crf-29 faststart, 120s timeout), the thumbnail crop (160×90), and the timeline filmstrip (per-frame extraction + `tile={n}x1` mosaic at q:v 4) all match the Python exactly; cleanup trash guards and error strings ("Source videos are protected…", "Only files inside the configured exports folder…", "The generated file is no longer available.") match with tests.
- Wave 47: verified the DB schemas are column-identical across all five
  tables (settings/media_files/exports/posts/delivery_attempts) and all
  32 settings defaults match the original's; hover-preview encoding is
  now bounded like the original's semaphore (1 concurrent encode, 2 in
  maximum-performance mode) instead of spawning an unbounded ffmpeg per
  hovered tile (capped-out requests release their pending mark so a
  re-hover retries); scan progress verbs (Processing/Checking/Creating
  thumbnail for/Adding) byte-match the Python.
- Wave 48: verified the palette action wiring (all 38 ids routed) and
  the library-root change flow (random reset, nav clear, workspace
  reset, custom-title preservation) match the original's setSetting
  handler; a release-binary smoke run on a fresh data dir boots
  cleanly, initializes the database, renders its first frame (window
  bounds persisted), and idles without errors.
- Wave 49: maximum-performance parity — selecting a video preloads its
  hover preview, and the selection neighbors preload their thumbnails
  (and previews when hover previews are on), mirroring the original's
  _select_media and _preload_selection_neighbors; both were missing
  from the Rust port.
- Wave 50: mapped every Python @Slot surface (67 methods) to its Rust
  equivalent — all present (native drag start remains the documented
  clipboard-copy compromise); the output-size estimate formula matches
  the original exactly for every preset (fit_bot 49 MB, fit_x 98% of
  the limit, fit_both min(49, 0.98·limit), custom, smallest 0.28×,
  balanced 0.62× with the same bitrate caps).
- Wave 51: formatting utilities verified byte-for-byte — duration
  (H:MM:SS / MM:SS), byte sizes (B / one-decimal KB–TB with the 1024
  ladder), and the media cache key (sha256 of resolved|size|mtime,
  first 24 hex chars) all match the original's utils.py; sort clauses
  (name COLLATE NOCASE ASC, mtime ASC/DESC with id tiebreakers) match
  the Python's database.py exactly.
- Wave 52 (defect-review round, 10 findings fixed): back/forward
  navigation now actually pops its stacks (previously the same top
  state re-applied forever and the stacks grew with copies), with the
  restore flag cleared on the empty-stack path; hover-preview pending
  marks are released when extraction completes so previews work on
  every hover; full library/history pages are guarded against stale
  generations; caption field states are invalidated when the model
  caption changes (workspace switch, media change), so stale edits
  can't overwrite restored drafts; the theme setting is no longer
  force-reset to "relay" on every launch; the X-limit margin is applied
  once (0.98² bug); pagination now reserves distinct offsets per
  load-more with an in-flight guard and offset-matched application
  (previously the offset was never advanced, so scrolling duplicated
  the first page); timeline extraction dedupes in-flight requests
  (26 ffmpeg processes per selection → 13); and neighbor preview
  preloads route through the concurrency-capped path.
- Wave 53: the load-more completion marker now always clears its
  in-flight flag (a refresh mid-flight could previously stall
  pagination forever), and the page-append decision was extracted into
  a pure, tested helper (generation + exact-next-offset) with the
  full-page replace guard (app suite now 15 tests).
- Wave 54 (second defect-review round, 6 findings fixed): the initial
  workspace now actually activates when the saved active id is missing
  or stale (previously `activate_root` was skipped, so multi-root
  libraries merged at startup and no tab showed active); the
  controller-side has-more flags are now fed from every query result,
  which makes scroll pagination and the reveal chase actually work
  (they had been silently no-ops — libraries never loaded past the
  first 240 rows); a freshly served page seeds the next expected
  offset; a personal sign-in display name now counts as configured, so
  publishing via the personal account works immediately instead of
  after a restart; the no-selection right arrow no longer flips the
  sort (it picked the oldest under "newest"); hover-preview frame
  directories are keyed by the media cache key so repeated hovers
  overwrite instead of leaking six files + six map entries per hover;
  and the trash containment check resolves symlinks like the original
  (with a parent-based fallback for missing files).
- Wave 55: added a pagination-stability test (25 rows paged at 7/10 per
  page under newest and name sorts — every row appears exactly once,
  no duplicates or gaps — core suite now 39 tests), and a fresh-data
  smoke run confirms the initial workspace now activates and persists
  its id (previously the active id stayed empty).
- Wave 56 (third defect-review round, 9 findings fixed): the playback
  frame clock resyncs on seek and loop wrap (backward seeks no longer
  freeze the preview, forward seeks no longer burst ffmpeg spawns);
  frame extraction is throttled to one spawn per 80ms with a cached-key
  fast path, and the drag release force-extracts the final position;
  precise time formatting carries hundredths overflow (65.999 renders
  as 01:06.00 — no more malformed 3-digit fractions that silently
  shifted trims on re-parse, now pinned by a test); the timeline
  pending mark clears when the job finishes so filmstrips can retry
  after failures; the crop/mask drag origin now matches the real
  layout (the formula still counted the tabs that moved to the bottom
  in wave 37, offsetting drags by 34-76px); settings static fields and
  explorer/random tree rows got collision-free element ids (folder
  names containing underscores no longer share a gpui state slot);
  a stale in-flight selection check can no longer overwrite a newer
  selection; and "Close workspaces to the right" no longer truncates
  every tab but the first when its target id went stale.
- Wave 57 (fourth defect-review round, 8 findings fixed): studio-mode
  pointer math accounts for the sidebar (timeline seeks, trim, and
  crop/mask drags were offset by the sidebar width in the full-screen
  editor); the Telegram destination is a single source of truth (the
  settings field and the prepare pane sync both ways, so a Settings
  edit can't silently send to the old channel); history timestamps
  render in LOCAL time (were UTC); trim drags force-extract their final
  frame; a failed selection check no longer clobbers a newer selection;
  the x-limit field seeds from the numeric setting (was silently reset
  to 512 on commit); folder chevron clicks no longer navigate
  (occluded hitboxes); and preview frame extraction releases its lock
  even on partial/failed runs (a stuck mark previously disabled
  re-extraction for the media).
- Wave 58: a real-library smoke drive confirmed the full root-change
  flow end-to-end — switching the library root on a data dir with a
  previous root deactivated the old root's rows (active=0, the wave-54
  activation fix working live), indexed and verified the new root's
  files (duration probed), and updated the workspace tab + library_root
  setting; the destination now stays a single source of truth through
  draft restores as well (a workspace draft updates the setting like
  the original's bound field).
- Wave 59 (visual verification unblocked): diagnosed and fixed the
  session's rendering failure at the gpui level — gpui 0.2 only starts
  its CVDisplayLink when the window reports NSWindowOcclusionStateVisible,
  which never happens when the display pipeline is broken; a dev-only,
  env-gated patch (`GPUI_FORCE_TIMER_DISPLAY=1`) starts the display
  link at window creation and falls back to a 16ms timer when the link
  can't run, so the app renders continuously (probe: 24 frames/4s).
  With the omp host's granted screen-capture permission, real visual
  sweeps are now possible and immediately caught two real bugs:
  (1) finishing a scan never refreshed the library grid, counts, or
  explorer — the UI stayed at "No videos found" with 3 indexed rows;
  (2) the active-root reconciliation compared the unresolved root
  (/tmp) against the scanner's canonical root (/private/tmp), so every
  row was deactivated and the library always appeared empty on macOS;
  normalize_path now canonicalizes and the reveal/navigation root
  gates compare resolved paths. Verified visually: unchecked tiles
  (name/size/Unchecked badge), verified tiles (real thumbnails,
  duration badges, resolution metadata), explorer tree counts, dynamic
  window title, and all three themes (relay warm-dark+orange, pitch
  black near-black+blue, full white light).
- Wave 60 (window-focused captures): screenshots are now cropped to the
  app window's exact bounds (CGWindowList on-screen frame, x2 retina
  scale, ffmpeg crop) instead of full-desktop grabs — the earlier
  "whole window" captures had actually been showing a STALE instance of
  an old /Applications/ClipRelay.app build that had been running since
  00:50 (killall didn't match the capitalized name); with it gone and
  the app brought to the active space via yabai, clean window-only
  captures of the current build were verified: populated grid with
  thumbnails/duration badges/resolutions, explorer counts, dynamic
  title "ClipRelay — cr-lib".
- Wave 61 (narrow-window adaptivity): the header and context toolbar
  previously used fixed-width children that overflowed the window edge
  at narrow sizes (tiling WMs routinely give the app less than the
  940px minimum). The header now shrinks the search field and drops the
  random/pick labels to icon-only below 820px; the toolbar collapses
  the Hide-folders/Choose-root/Rescan labels to icons below 1060px and
  hides the "Video library /" breadcrumb text plus the explorer band
  below 880px, with the prepare band's name gated on the dock width
  like the original; studio mode squeezes the inspector before the
  stage overflows; and the window minimum was lowered to 700x520 (with
  the restore clamp matched) so half-screen layouts are actually
  usable. Verified visually at 740px: no right-edge clipping, all
  controls reachable, grid + explorer intact.
- The expanded Prepare dock now follows the original's width formula
  exactly (widens toward 680 px when space allows, never squeezing the
  grid below 460 px), and the Settings content width adapts to the
  window (min(820, page − 48)) instead of overflowing on narrow windows.
- The command palette gained the "Commands" scope chip (actions only)
  and the ↑↓/↵/ESC hint footer, matching the original.
- Workspace tabs now show the scanning/stopping indicator live (the
  scan lifecycle previously never propagated to the tab bar).
- The library empty state distinguishes auto-index on/off exactly like
  the original ("Rescan the library, or enable deep format detection…"
  vs "Background indexing is off…").
- The window enforces a 940×660 minimum size (matching the original).
- Selections that live beyond the loaded library page are now chased:
  restoring a workspace, picking a command-center media, or navigating
  loads pages until the tile's page is available, then scrolls it into
  view (contain-style, matching GridView.Contain).
- Explorer sidebar: expandable folder tree with per-folder video counts
  and a folder sort menu (name/size/count/newest).
- Random source popup: folder hierarchy with tri-state selection,
  search filter, "Selected only" mode, and a Clear button.
- Workspace tab context menu (rename in place, duplicate, close, close
  others/right, reopen last closed) and per-tab close buttons.
- Context toolbar with sort controls (library and folder) and the
  activity popup (scan/check/timeline/publish progress with Stop scan).
- Prepare autoplay on selection with real-time position advancement,
  lazy frame extraction at the playhead rate, and trim loop-back.
- Diagnostics page section with an auto-requesting Refresh button.
- Draft round-trip: Prepare state (trim, crop, masks, captions,
  compression, scroll positions) is saved per workspace and restored
  when the workspace and its selection are re-activated.

### Fixed

- Workspace restore now activates the saved workspace at startup instead
  of being skipped by the activation guard (selection, folder, search,
  and draft were never re-applied).
- Playback position advanced by the whole frame rate instead of one
  frame per tick, immediately wrapping to the trim start.
- Stage frame extraction used a thread-local media path that extraction
  threads never saw; frames now travel with the request.
- The UI render loop could stall when no event notified gpui; a heartbeat
  task now keeps the view draining queued messages.
- Selected media's thumbnail/timeline assets now update in place when
  their background jobs complete.
- Prepare source strip shows resolution alongside size and duration;
  "Pick random" reflects the actual picking state.

## [0.2.0] - 2026-08-04

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

- Moved Prepare's Edit and Publish switcher to an equal-width bottom tab rail,
  removed the redundant edit footer, and condensed source details to one row.
- Made the Prepare player follow each video's real aspect ratio and allocate
  more stage height when the window can support it.
- Reorganized Prepare so mode and panel-size changes preserve playback,
  trimming, frame edits, publishing fields, and active operation state.
- Moved startup root activation, filtered library and history queries, folder
  aggregation, count refreshes, and thumbnail asset bookkeeping off the UI
  thread.

### Fixed

- Kept full-screen source details and inspector tabs fully inside the window,
  including compact heights and resized inspector widths.
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

[Unreleased]: https://github.com/reduced2ash/cliprelay/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/reduced2ash/cliprelay/compare/v0.1.0-beta.2...v0.2.0
[0.1.0-beta.2]: https://github.com/reduced2ash/cliprelay/compare/v0.1.0-beta.1...v0.1.0-beta.2
[0.1.0-beta.1]: https://github.com/reduced2ash/cliprelay/releases/tag/v0.1.0-beta.1

- Visual polish wave (custom title bar + UI pass): the window now uses
  a custom FLUSH title bar — the native macOS title bar is hidden
  (appears_transparent) and the app's own 40px header row becomes the
  title bar, with the traffic lights parked over it (empirically
  centered at y=27) and a drag region (gpui patched with
  performWindowDragWithEvent) plus double-click-to-zoom on the empty
  header area; the search field gained a magnifier glyph and clears the
  traffic lights (96px left inset); the context toolbar and header are
  fully adaptive (icon-only buttons and collapsed breadcrumbs below
  their width thresholds); tiles hover with an accent border, the
  active workspace tab gets a 2px accent top edge and a raised fill,
  popups/menus share a softer 10px radius, command-center section
  headers are uppercase with better contrast and rows have hover
  states, the popup width matches the search field, explorer rows
  hover, and the prepare source strip no longer overflows into the
  Reveal button (flexible parts, capped path, icon-only reveal on
  narrow docks); a hardcoded relay-theme leak in tile posters/badges
  now uses the active theme. Dev-only helpers for screenshot sweeps:
  CLIPRELAY_PAGE=history|settings and CLIPRELAY_OPEN_COMMAND=1.

- Polish iteration 2 (popup sweep): every remaining popup was captured
  and audited via the window-only screenshot loop — random-source popup
  (fixed a duplicated empty-state message and the boot-open env now
  triggers the folder-options load), sort menu (VIDEOS + EXPLORER
  FOLDERS sections verified), studio mode (header band, stage, timeline
  ticks, inspector, splitter), publish inspector (OUTPUT estimate,
  DESTINATIONS with the Needs-setup pill, action dock), activity popup
  (idle state), and the workspace context menu (items, dividers,
  disabled states). The action-dock buttons now flex to share the dock
  width and their labels ellipsize instead of clipping (the button
  widget dropped its hardcoded flex_none and wraps labels in a
  min-width-zero ellipsis container); the source-strip path contrast
  was raised. New boot-open envs for sweeps: CLIPRELAY_OPEN_RANDOM,
  CLIPRELAY_OPEN_SORT, CLIPRELAY_OPEN_ACTIVITY,
  CLIPRELAY_OPEN_WORKSPACE_MENU. All findings were pixel-verified (the
  vision model's edge-clipping claims were measured and dismissed as
  crop artifacts).

- Polish iteration 3 (theme + interaction sweep): pitch-black and
  full-white themes re-captured with the populated library after the
  tile-color fix — both verified (near-black+blue, light+blue, the
  amber warning colors are the intentional warning palette matching the
  Python). The full-white sweep found a real contrast bug: disabled
  buttons used 46% opacity, washing colored fills out (white on light
  blue); they now render as neutral raised fills with muted text on
  every theme. Fields gained a hover border to match the other
  controls. The duration badges' dark overlay is intentionally
  video-agnostic (the Python does the same).
- Polish iteration 4 (settings/telegram/toast/command-center sweep): the
  settings page's lower sections (FILES, TELEGRAM bot + personal, X HANDOFF,
  LIVE DIAGNOSTICS) were captured and audited for the first time; the
  personal-account grid was restructured to the Python's 2-column layout
  (API ID + API hash / phone + Send login code / Login code + password).
  The error toast was captured (boot-toast env) and verified: error-soft
  fill, error border, glyph, Dismiss action, bottom-centered pill. The
  command center was captured with a live query: video results now render
  at boot with CLIPRELAY_QUERY (command_query is seeded and the real
  search runs after restore).

- Polish iteration 5 (history + commands-scope sweep): the history page
  was captured populated for the first time (seeded posts) — rows verified:
  thumbnail, title, caption, platform status pills with the Python's
  visibility rules (a platform pill is hidden when it was not requested),
  local-format dates ("Aug 12, 2026 · 09:11 PM"), 166px action column with
  View + More actions. The commands-scope palette (">" prefix) exposed a
  real bug: the action filter matched the raw query including the ">"
  prefix, so ">scan" never matched anything; the filter now strips the
  prefix (the needle) and searches label + detail + category + keywords
  like the original, and matched commands are grouped under their real
  category headers (LIBRARY/NAVIGATION/…) instead of a single "ACTIONS"
  header.

- Clippy: the app and core crates are now warning-free (excluding the
  transitive future-incompat notes from `block` and `proc-macro-error2`).
  Cleaned ~30 lints: collapsed ifs, needless borrows, map/inspect_err
  idioms, unneeded muts, a clamp pattern, an identical-branch label, and
  unused error bindings; added a `ProgressFn` type alias for the ffmpeg
  progress callback, boxed the large `UiMessage::Event` variant, and
  scoped too-many-arguments allows to the 8 ported pipeline functions.

- Vendored the two local gpui patches into `vendor/gpui` (a full copy of
  gpui 0.2.2 + `[patch.crates-io]` in the root Cargo.toml), so the custom
  title-bar drag and the display-link timer fallback build on any machine
  and survive a cargo re-download. PATCHES.md documents both edits.

- Motion: the indeterminate progress bars (library scan, publish, check)
  now sweep with a looping 1.4 s animation (gpui with_animation, no
  patching needed), and the five popups (command center, random-source,
  sort menu, activity, workspace menu) fade in over 120 ms with an
  ease-in-out. Also cleaned an unnecessary unsafe block in the vendored
  display-link timer fallback.

- Interaction states: trim handles now brighten (accent_pressed) on hover
  and while dragging, and checkbox indicators fill with the accent on
  hover exactly like the original AppCheckBox.

- Settings combo triggers gain a hover border (border_strong), matching
  the field hover treatment. Typography scale re-audited against the
  QML (9/10/11/12/13/15/16/20/21 all map to their Python roles).

- History "More actions" menu now anchors to the clicked row (the click
  position is recorded and the menu opens at that y), mirroring the
  original's button-anchored Menu popup; previously it floated at a
  fixed y, which landed far from rows 2+.

- Workspace menu now opens upward from the bottom tab bar (its
  trigger), matching the original's tabContextMenu popup; it previously
  rendered at the top of the window.

- Dock icon: the bare binary now applies the original's relay icon
  (assets/cliprelay.svg, embedded via include_bytes) to the dock after
  the gpui platform initializes, mirroring the Python's setWindowIcon.

- Offscreen surface capture (dev-only): CLIPRELAY_CAPTURE=/path.png
  writes the rendered window surface as a PNG after N frames
  (CLIPRELAY_CAPTURE_AFTER, default 40) — reads the Metal drawable back
  after the GPU finishes, so sweeps work while the physical display is
  asleep/locked. Also fixed the occlusion gate so GPUI_FORCE_TIMER_DISPLAY
  actually starts the display link even when the window is never
  reported visible (the env escape was only in the occlusion-change
  handler, not in start_display_link).
  With this, the two-workspace tab bar (both tabs, active indicator,
  restored folder/search) and the full final sweep (history, settings
  scrolled, commands-scope palette, error toast) were captured and
  audited — the previously environment-blocked verifications are done.
