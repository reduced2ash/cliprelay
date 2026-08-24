#!/usr/bin/env bash
set -Eeuo pipefail

window_id="$1"

# ADOPT: replace or extend this with an accessibility, IPC, GPUI-test, or other
# semantic readiness assertion. Coordinate-based pointer/keyboard input does not
# belong here.
xprop -id "${window_id}" WM_NAME >/dev/null

