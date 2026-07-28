# Library Canvas Redesign

## Direction

The Library canvas extends the approved upper workbench into a technical media
index. The titlebar and Library context toolbar already own navigation, search,
sorting, root selection, random scope, scanning, and commands. The canvas does
not duplicate them.

The redesign makes media easier to scan, gives every tile state a precise
visual language, and keeps the grid stable with very large libraries.

## Scope

This pass covers:

- media-tile composition
- responsive grid measurements
- Default and Compact density
- selection, keyboard focus, hover, posted, and validation states
- thumbnail loading and failure states
- hover-preview resource ownership
- the 40-pixel Library footer
- large-library scrolling performance

This pass does not redesign:

- either upper-workbench band
- the activity rail
- Explorer structure or folder rows
- Prepare content
- History or Settings structure
- themes or publishing behavior

## Layout

The Library content has two regions:

1. A virtualized media grid.
2. A 40-pixel footer aligned with the Explorer footer.

Default density prioritizes readable filenames and complete metadata. Compact
density fits more media into the same viewport without changing application
scale. Columns derive from available content width, not window-size labels or
fixed counts.

Tiles use stable 16:9 media wells. Every source frame uses
`PreserveAspectFit`, so portrait and unusual aspect ratios are letterboxed
rather than cropped. Loading state changes never alter row geometry.

## Media Tiles

Each tile contains:

1. An uncropped thumbnail or muted hover preview.
2. A duration badge when media has been checked.
3. A middle-elided filename.
4. One compact metadata and state row.

The tile has no enclosing card surface. A one-pixel outline defines the media
well; selection and keyboard focus use the two-pixel accent treatment.
Selection also receives a check glyph so focus and selection are distinguishable.

Unchecked media states move out of the thumbnail and into metadata. Thumbnail
states use precise language: pending, queued, creating, loading, and
unavailable.

## Density

- **Default:** 12-pixel grid gaps and a 236-pixel target minimum tile width.
- **Compact:** 8-pixel grid gaps and a 176-pixel target minimum tile width.

Density is persistent, available in Settings, and callable from the command
center. Switching density never triggers a scan, metadata check, thumbnail
generation, or model refresh. The selected tile is kept visible.

## Footer

The Library footer reports:

- visible model rows
- whether a video is selected
- Compact density when active
- indexing activity
- active thumbnail work
- thumbnail idle state

Labels accompany every colored state. The footer remains flat and shares the
same divider, height, typography, and baseline as the Explorer footer.

## Interaction

- Click selects and prepares a video.
- Space controls playback.
- Left and Right retain previous and next selection behavior.
- Hover preview starts after 350 milliseconds.
- Only one hover-preview player may exist at a time.
- Leaving or recycling a tile releases its preview immediately.
- Long filenames and metadata expose their complete value through a tooltip.

## Performance

- Only visible delegates plus bounded overscan are instantiated.
- Delegate reuse remains enabled.
- Thumbnail decoding requests the rendered pixel size.
- Tile geometry stays constant through every background state.
- Only one hover-preview player decodes at a time.
- Filename filtering and paging remain SQLite-backed.
- Scrolling does not touch the filesystem.
- Density changes do not rescan or regenerate assets.
- Layout properties do not animate during scroll or resize.

## Acceptance Criteria

- The upper workbench and Prepare remain visually and functionally unchanged.
- Tiles never overlap at any supported width or scale.
- Thumbnails and previews are never cropped.
- Loading thumbnails never move surrounding tiles.
- Default and Compact density persist and preserve selection context.
- Long filenames cannot escape their tile.
- Only one hover preview runs at once.
- The Library footer aligns continuously with the Explorer footer.
- Every state is understandable without relying on color.
- Existing tests pass and the packaged Apple-silicon build launches.
