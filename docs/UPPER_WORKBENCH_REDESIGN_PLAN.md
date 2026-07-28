# Upper Workbench Redesign

## Direction

Direction C is the approved north star: a dense, native-feeling media
workbench with VS Code-like structure, Raycast-like command search, and
Linear-like interaction polish.

The redesign removes the duplicated brand row, the oversized Library banner,
and the separate search toolbar. The relay mark in the titlebar owns product
identity. Search, navigation, random selection, and background status share
one compact titlebar, while folder and library context share one compact
toolbar beneath it.

## Scope

This pass covers:

- the custom frameless titlebar
- titlebar window controls and draggable regions
- the compact activity rail header
- the Library context toolbar
- video and folder search
- command search
- video navigation and random-selection controls
- background activity status
- responsive states, keyboard access, focus, and long-text handling

This pass does not redesign:

- History or Settings content
- the lower activity rail
- folder-tree rows
- media tiles
- Prepare content
- application themes
- random-selection rules
- the release workflow

## Layout

The upper workbench uses two bands:

1. A 40-pixel titlebar:
   window controls, relay mark, back and forward selection, command center,
   activity status, random-source scope, shuffle reset, and Pick random.
2. A 42-pixel Library context toolbar:
   Library activity context, Explorer count and collapse, library breadcrumb,
   folder visibility, root-folder selection, sort, and Rescan.

At wide widths, controls include concise labels. At standard widths, secondary
actions become icon-first. At the 940-pixel minimum width, the command center,
random scope, window controls, and Pick random remain available; lower-priority
actions move to compact or command-only access.

## Command Center

The command center placeholder is:

`Search videos, folders, and commands`

- `Command/Ctrl+K` focuses the command center.
- `Command/Ctrl+F` focuses media search.
- `Command/Ctrl+Shift+P` opens command mode.
- Normal text filters the visible library and queries capped SQLite-backed
  video and folder suggestions.
- A leading `>` enters command mode without changing the media filter.
- Arrow keys move through results, Enter activates, and Escape dismisses.
- Search is debounced, stale queries are discarded, and no search triggers a
  filesystem scan.

Commands cover random selection, previous random, shuffle reset, folder
visibility, root selection, rescan, sort modes, page navigation, themes, and
sidebar collapse.

## Visual System

- flat tonal surfaces with one-pixel dividers
- restrained accent use for focus, selection, and the primary action
- system UI type at 12 to 13 pixels for workbench labels
- native monospaced figures for counts and status where appropriate
- 28 to 30-pixel visual controls inside desktop-sized hit regions
- three to four-pixel radii for technical toolbar controls
- no gradients, glass, glow, nested cards, or decorative motion

The implementation must work in Relay, Pitch Black, and Full White themes.

## Interaction and Accessibility

- every action has an accessible name and tooltip when its label is hidden
- keyboard focus is visible
- disabled actions explain their state through tooltip copy
- popup triggers toggle closed when pressed again
- popups render in the window overlay and remain inside window bounds
- long names and paths use middle elision
- active work is communicated with text or icon state, never color alone
- motion is limited to short color and opacity state feedback

## Performance

- media suggestions query SQLite rather than the filesystem
- input debounce targets 120 milliseconds
- suggestions are capped to a small bounded result set
- stale asynchronous results are ignored
- opening the titlebar does not regenerate thumbnails or interrupt playback
- no blur, continuous animation, or layout animation is introduced

## Acceptance Criteria

- the Library upper chrome is exactly two compact bands
- the titlebar remains draggable and window controls retain native behavior
- duplicated branding, the oversized Library header, and the separate search
  row are removed
- every existing upper-workbench action remains available
- command search and media search work from the same field
- random-source, activity, combo, and command popups toggle and stay in bounds
- long library names and folder paths never escape their controls
- the layout remains usable at 940 pixels and under all scale presets
- all three themes remain legible and coherent
- large-library startup and browsing performance do not regress
- automated tests pass and the packaged Apple-silicon build launches
