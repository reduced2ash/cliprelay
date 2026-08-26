# ClipRelay — Relay Splice identity

Relay Splice is a new logo system for ClipRelay. Its continuous folded ribbon
suggests a selected clip moving through a precise handoff, while the open
silhouette resolves into a compact `C` without relying on a play button,
clapperboard, chain link, or generic forwarding arrow.

## Core palette

- Signal coral: `#F26A4F`
- Relay ink: `#101114`
- Warm off-white: `#F4F1ED`

## Package contents

- `logos/`: horizontal lockups for light and dark surfaces in PNG and traced SVG
- `marks/`: standalone color, ink, and off-white marks in PNG and traced SVG
- `app-icons/cliprelay-app-icon-1024.png`: cross-platform app-icon master
- `app-icons/png/`: 16–512 px application icon exports
- `app-icons/macos.iconset/`: complete macOS iconset source folder
- `web/`: multi-size favicon, touch icon, PWA icons, and compact mark
- `preview/`: identity board and finished pack preview
- `masters/raw/`: untouched model outputs retained for provenance
- `PROMPTS.md`: the final image-generation prompt set

## Usage

Use `cliprelay-lockup-light` on light surfaces and `cliprelay-lockup-dark` on
dark surfaces. Use the standalone mark where the ClipRelay name is already
visible or where horizontal space is constrained.

Keep clear space around the mark equal to at least one quarter of its height.
Use the supplied raster exports below 32 px; they were reduced from the icon
master and are easier to review than an arbitrary browser or toolkit resize.

The SVG files are clean traced outlines intended for normal product and brand
use. The untouched generated masters remain available if the identity is later
redrawn as hand-authored Bézier geometry.

## Packaging notes

- Windows: use `web/favicon.ico` for web/favicon contexts or build an app ICO
  from the supplied multi-size PNGs.
- macOS: run `iconutil -c icns app-icons/macos.iconset` on macOS.
- Linux: use the appropriate size from `app-icons/png/`.

