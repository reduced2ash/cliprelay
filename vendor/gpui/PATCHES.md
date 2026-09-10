# gpui 0.2.2 fork — local platform patches

This is a vendored copy of `gpui 0.2.2` with local platform and rendering patches.
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

## 3. Dev surface capture (`src/platform/blade/blade_renderer.rs`)

The Blade renderer accepts an optional output path and reads the rendered
surface back to a PNG after GPU completion. The macOS and X11 windows pass a
one-shot pending capture request; Wayland still passes `None`. Linux Blade
surfaces include copy usage so Vulkan swapchain images are valid transfer
sources for readback.

## 4. Native macOS backdrop material (`src/platform/mac/window.rs`)

`WindowBackgroundAppearance::Blurred` uses AppKit's semantic
`UnderWindowBackground` material in behind-window mode. The visual-effect view
keeps the system-owned tint and saturation filters so the frosted window color
comes from the actual desktop behind it rather than an application-supplied
hue.

To refresh from a registry download, copy only the standalone manifest,
license, build script, platform resources, and `src/` tree. Reapply these
patches, remove registry metadata and upstream examples, then validate the
entire ClipRelay workspace.

## 5. Popup backdrop blur (`window.rs`, `scene.rs`, Blade renderer/shader)

`Window::paint_backdrop_blur` inserts a sampling dependency and an isolated
quad batch. Blade snapshots the painted surface into a reusable texture before
that batch, then samples a Gaussian kernel inside the popup's rounded bounds.
The surface requires copy usage on both Vulkan and Metal. Opaque menus skip
this work in ClipRelay's shared overlay widget.

The Quad layout is kept in sync with WGSL and HLSL (the native Metal shader
uses generated headers). Non-Blade renderers draw the opaque fallback instead.
The Linux Blade path is covered by live Agent Workspace checks; macOS Blade
and Windows fallback require verification on those platforms.

## Application zoom (`window.rs`, `interactive.rs`, `platform.rs`)

`Window::set_ui_scale` composes application zoom with native display DPI.
Call it between frames (ClipRelay schedules it with `on_next_frame`). The
layout viewport shrinks as zoom increases; native window bounds and persisted
window dimensions retain OS logical units. Paint and text rasterization use
the composed scale. Native pointer, pixel scroll and file-drop coordinates
are converted once at `on_input`; synthetic events already use layout units.
Line scroll units stay unchanged. IME rectangles, point lookup, native window
menus and client insets convert at their platform boundaries. The default is
1.0, preserving upstream behavior for other windows.

Regression coverage lives in `crates/app/src/responsive.rs`, including native
bounds stability, display/zoom composition, pointer/drop/scroll conversion,
and compatibility with the old stored scale presets.
