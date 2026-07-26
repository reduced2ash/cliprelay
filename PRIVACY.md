# Privacy

ClipRelay is local-first software. It has no ClipRelay-hosted account system,
telemetry, analytics, advertising SDK, or background data collection.

## Data stored locally

ClipRelay stores:

- the selected library and export-directory settings
- a SQLite index of video paths and metadata
- generated thumbnails and muted hover previews
- post history and delivery states
- Telegram credentials or sessions in the operating-system credential store
  when available

This data remains on the device unless the user backs it up or shares it.

## Data sent to third parties

ClipRelay communicates with a third party only after a user requests the
associated workflow:

- Telegram receives credentials during validation and receives the selected
  video and caption during delivery.
- X receives the prefilled caption through its official web composer. The
  selected local video is attached by the user through paste or drag.

ClipRelay does not proxy either service through its own server.

## Local fallback credentials

If the operating-system credential store fails, ClipRelay can use a
permission-restricted `secrets.json` file in its local configuration directory.
This file is not uploaded by ClipRelay, but it is less protective than the
system credential store. Do not share it.

## Source media

Source videos are read-only from ClipRelay's perspective. Editing and
compression create a separate derivative in the configured export directory.
Cleanup operations are restricted to generated files within that directory.

## Removing data

Uninstalling the app may not remove local settings, history, thumbnails,
previews, exports, or credential-store entries. Users can remove those
separately through their operating system. Generated files can also be moved
to Trash from ClipRelay where the history entry permits it.
