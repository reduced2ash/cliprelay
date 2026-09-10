# Third-party notices

ClipRelay source is licensed under MIT. Packaged applications also contain
third-party software that remains under its own license. This summary is
informational and does not replace those terms.

## GPUI

ClipRelay uses GPUI 0.2.2, Zed's GPU-accelerated UI framework, under the Apache
License 2.0. A patched copy is kept in `vendor/gpui`; the retained upstream
license is `vendor/gpui/LICENSE-APACHE`.

- Project and source: [Zed](https://github.com/zed-industries/zed)
- Patch summary: [vendor/gpui/PATCHES.md](vendor/gpui/PATCHES.md)

## FFmpeg and FFprobe

ClipRelay invokes FFmpeg and FFprobe as separate command-line programs. Native
release packages may bundle them.

- Project and source: [FFmpeg](https://ffmpeg.org/)
- License details: [FFmpeg legal information](https://ffmpeg.org/legal.html)

FFmpeg licensing depends on its build configuration. Builds that enable GPL
components require the corresponding GPL compliance, notices, and source
availability. Release metadata must record the exact `ffmpeg -version` and
`ffmpeg -L` output for bundled binaries.

## GStreamer

ClipRelay uses GStreamer for in-process video playback. macOS packages embed a
private copy of the official framework, its plugin scanner, and selected
runtime plugins.

- Project and source: [GStreamer](https://gstreamer.freedesktop.org/)
- Core license: GNU LGPL version 2.1 or later

The framework remains dynamically linked. Redistributors must review the
license of every included plugin and codec; plugin terms can differ from the
GStreamer core.

## Rust dependencies

The remaining Rust dependencies and exact versions are recorded in
`Cargo.lock`. Each crate retains its own copyright and license. Important
runtime components include SQLite through `rusqlite`, HTTP through `reqwest`,
OS credential storage through `keyring`, Trash integration, image decoding,
and async/concurrency utilities.

Copies of GNU GPL version 3 and GNU LGPL version 3 are retained in `LICENSES`
for packaged media components. Anyone redistributing a binary should audit the
exact build and provide all notices and corresponding source required by its
dependency and codec configuration.

## README screenshot

The screenshot shows the running app with short demonstration clips from
*Big Buck Bunny*. Film: (c) copyright 2008, Blender Foundation /
[www.bigbuckbunny.org](https://www.bigbuckbunny.org/), licensed under
[Creative Commons Attribution 3.0](https://creativecommons.org/licenses/by/3.0/).
The clips were excerpted, renamed, and muted for the demonstration. See the
[project's license information](https://peach.blender.org/about/).
