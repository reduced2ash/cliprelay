# Automated agent rules

The active application is the Rust/GPUI workspace in `crates/`. Preserve any
existing staged or unstaged work before changing it.

For GUI validation, automated agents must use `make ui-test`. That command runs
ClipRelay in a Docker container on its own Xvfb display and returns a concise
pass/fail result. Inspect `artifacts/ui-test/` only when the command fails or a
human explicitly requests visual review.

Agents must never automate ClipRelay through the host keyboard or pointer, open
the app on the host display, connect to the host X11 or Wayland socket, mount a
host browser profile, use `/dev/uinput`, or mount the host Docker socket into a
test container. `make ui-inspect` is a human-only, localhost-only debugging
path; it does not authorize host-desktop automation.

See `docs/GUI_TESTING.md` for commands, artifacts, security boundaries, and
baseline review.

