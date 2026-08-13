# gpui 0.2.2 fork — local macOS patches

This is a vendored copy of `gpui 0.2.2` with two small macOS patches.
It is wired into the workspace via `[patch.crates-io]` in the root
`Cargo.toml` so the changes survive `cargo clean` and work on any
machine, not just the dev box.

## 1. Display-link timer fallback (`src/platform/mac/display_link.rs`)

`DisplayLink::new` falls back to a 60 Hz timer thread when the
CVDisplayLink cannot start (the machine's display pipeline reports
"Internal failure" in `ioreg`, so the link never fires). The timer is
also used when `GPUI_FORCE_TIMER_DISPLAY=1` is set, which is handy for
headless-ish screenshot sweeps.

## 2. Occlusion + custom title-bar drag (`src/platform/mac/window.rs`)

- The window's display link is started even when the window does not
  report `NSWindowOcclusionStateVisible` (some machines never report
  visible, which previously froze the app after frame 1). Opt out with
  `GPUI_FORCE_TIMER_DISPLAY` unset? No — see the code: the env var
  forces the timer, the occlusion workaround is unconditional.
- `Window::start_window_move` (the facade method used by the custom
  flush title bar) is implemented as `performWindowDragWithEvent` with
  a synthesized `NSLeftMouseDown` + `NSCommandKeyMask` event, matching
  how frameless apps implement drag. Stock gpui 0.2.2 had a no-op.

To regenerate from a fresh registry download: apply the same edits to
`~/.cargo/registry/src/*/gpui-0.2.2/` and copy the directory here, or
`diff -r` this directory against the registry copy.
