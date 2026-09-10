# User guide

## Library

Choose one top-level folder. ClipRelay searches every nested folder for video
files while omitting hidden folders, source-control metadata, exports, and
known non-video files.

The library can display:

- folders and their direct contents
- every video at one level

Search matches filenames and relative folder paths. Sort by newest, oldest,
name, duration, or size.

The title-bar command center searches videos and folders without rescanning the
library. Start a query with `>` to browse app commands, or press
`Cmd+Shift+P` / `Ctrl+Shift+P` to open command mode directly.

## Workspaces

The compact rail at the bottom keeps multiple library roots open at once.
Each workspace remembers its own open folder, search, sort, selected video,
random-source filter, Prepare draft, and back/forward navigation history.

Use the `+` button or `Cmd+T` / `Ctrl+T` to open another root. Right-click a
tab to rename, duplicate, reveal, close, or reopen workspaces. Middle-click
also closes a tab. The title-bar back and forward buttons always operate on
the active tab only.

## Random selection

**Pick random** uses the local filename manifest instead of walking the folder
tree for every click. The **From** control can limit random selection to one or
more folders, including their descendants.

When repeat avoidance is enabled, a selected video is removed from the current
random pool. The pool resets after all eligible videos have been seen. Use
the title-bar back button to return to the prior random pick, folder, or video,
or **Reset shuffle** to reset repeat avoidance.

The **From** picker supports search and a selected-only view. Selecting a
folder updates it in place, so long folder lists keep their current scroll
position.

A newly discovered video is validated after selection. Preparation controls
remain unavailable if the file is unreadable or while that single-file check
is still running.

## Prepare and edit

Prepare always works from the source video and writes a generated copy only
when required.

Available adjustments:

- exact cut start and end
- compression preset or target size
- free crop and common aspect-ratio crops
- multiple movable and resizable black rectangle or square masks

The regular Prepare panel can be widened. Full-screen Prepare keeps the same
live playback and editing state while providing a larger editor.

## Keyboard shortcuts

Application shortcuts work from Library, Prepare, Studio, and other ordinary
surfaces without focusing a special panel. Text entry and IME composition keep
their editing keys; menus and focused controls keep their local arrows and
activation keys. Explorer folder rows use arrows to navigate the tree, while
`R` and `S` remain available. After dismissal or leaving a field, application
shortcuts resume. Search/command shortcuts in Studio return to the main search
surface.

| Shortcut | Action |
| --- | --- |
| `Space` | Play or pause the selected video |
| `←` / `→` | Select the previous or next video |
| `R` | Pick a random video |
| `S` | Open the current valid selection in Studio |
| `I` / `O` | Set the cut In or Out point at the playhead |
| `Cmd+K` / `Ctrl+K` | Focus the command center |
| `Cmd+F` / `Ctrl+F` | Focus video and folder search |
| `Cmd+Shift+P` / `Ctrl+Shift+P` | Open command mode |
| `Cmd+1` / `Ctrl+1` | Open Library |
| `Cmd+2` / `Ctrl+2` | Open History |
| `Cmd+,` / `Ctrl+,` | Open Settings |
| `Cmd+T` / `Ctrl+T` | Open a new workspace |
| `Cmd+W` / `Ctrl+W` | Close the active workspace |
| `Cmd+Shift+T` / `Ctrl+Shift+T` | Reopen the last closed workspace |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | Cycle workspace tabs |
| `Cmd+[` / `Ctrl+[` / `Alt+←` | Go back in the active workspace |
| `Cmd+]` / `Ctrl+]` / `Alt+→` | Go forward in the active workspace |
| `Cmd+M` / `Ctrl+M` | Minimize the window |
| `Control+Cmd+F` / `F11` | Enter or exit full screen |
| `Escape` | Dismiss the active menu or text field, then Studio or full screen |

The themed title bar behaves like the platform window frame. Drag its empty
area to move the window, drag any edge or corner to resize, and double-click
the title bar to maximize or restore. On macOS, the green button enters full
screen; Option-click it to zoom the window instead.

## Telegram

### Bot mode

Bot mode is the simplest option for a channel:

1. Create a bot with `@BotFather`.
2. Add the bot to the channel and grant permission to post.
3. Paste the token under Settings and validate it.
4. Enter the channel username or numeric chat ID and validate it.

### Personal-account mode

1. Create an API application at
   [my.telegram.org](https://my.telegram.org/).
2. Enter the API ID, API hash, and phone number.
3. Request and enter the Telegram sign-in code.
4. Enter the two-step-verification password if required.
5. Load chats or enter a destination directly.

Telegram credentials and sessions are stored locally.

## X handoff

ClipRelay does not silently post to X.

When preparing for X, ClipRelay:

1. opens X's official web composer with the caption prefilled
2. copies the prepared video as a local file where the operating system
   supports it
3. keeps Copy video, Drag video, and Show in folder actions available
4. records the handoff in History

After posting in X, use **Mark X posted** in History. A post URL can be stored
when available.

## History

History records Telegram and X independently. A combined action can therefore
show Telegram as sent while X is still prepared.

Available actions include:

- reopen the video in Prepare and reveal it in the library
- retry a failed Telegram delivery
- prepare the X handoff again
- open a recorded Telegram or X post
- reveal the source or generated file
- move an eligible generated copy to Trash

## Performance settings

Fast Random is on by default. Full media verification, deep detection, and
eager thumbnail generation are optional because each can be expensive for a
large or remote library.

Visible thumbnails use a bounded background queue. Hover previews are also
generated in the background and may be disabled.

**Settings → Interface → Interface scale** changes text, icons, controls,
menus, and media tools together. Choose 80%, 90%, 100% (default), 110%, 125%,
or 150%. Changes apply immediately and are saved for the next launch; select
100% to reset. This multiplies your operating system's display scaling. Focus
the selector and use the arrow keys to adjust it, or open it with Enter.
Library density remains an independent grid preference.

Layouts respond to the space remaining at your chosen scale. Navigation
collapses before the workspace gets crowded. When Library and Prepare no
longer fit together, Prepare uses a full-window view: **Back to Library** and
**Return to Prepare** switch between them without discarding the selection or
edit draft. **Inspector** switches between the preview and editing/delivery
controls. Short windows scroll the preview's lower controls; Settings uses
compact section tabs and scrollable content.

The Interface settings also include **Fit the whole video inside Library
thumbnails**. Enable it to keep every edge of portrait, square, or ultrawide
videos visible; the thumbnail uses letterboxing instead of cropping to fill.

**Maximum performance** keeps VSync enabled at the active display refresh
rate, preloads adjacent clips, and raises safe thumbnail and hover-preview
concurrency. A restart applies the renderer and decoder preference; preloading
changes apply immediately.

The export-encoder setting offers Automatic, Prefer hardware, and Software
only. Automatic uses hardware in Maximum mode and the quality-focused software
path otherwise. Hardware export uses VideoToolbox on supported Macs, or NVENC,
Quick Sync, or AMF on supported Windows systems. Every hardware job has an
automatic software fallback.

Settings also shows live renderer, GPU, display refresh, decoder, export
encoder, frame-pacing, and frame-spike diagnostics. Frame sampling only runs
while the diagnostics page is visible.
