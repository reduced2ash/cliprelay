#!/usr/bin/env bash
set -Eeuo pipefail

# shellcheck source=x-session.sh
source /opt/gui-test/bin/x-session.sh
[[ -n "${GUI_TEST_VNC_PASSWORD:-}" ]] || {
    echo "GUI_TEST_VNC_PASSWORD is required." >&2
    exit 2
}

app_pid=""
vnc_pid=""
web_pid=""
cleanup() {
    for pid in "${web_pid}" "${vnc_pid}" "${app_pid}"; do
        [[ -n "${pid}" ]] && kill -TERM "${pid}" >/dev/null 2>&1 || true
    done
    stop_x_session
}
trap cleanup EXIT INT TERM

start_x_session
mkdir -p /tmp/vnc
x11vnc -storepasswd "${GUI_TEST_VNC_PASSWORD}" /tmp/vnc/passwd >/dev/null
/opt/gui-test/bin/launch-project.sh > /artifacts/logs/app-inspect.log 2>&1 &
app_pid=$!
x11vnc -display "${DISPLAY}" -localhost -forever -shared \
    -rfbauth /tmp/vnc/passwd -rfbport 5900 > /artifacts/logs/x11vnc.log 2>&1 &
vnc_pid=$!
websockify --web=/usr/share/novnc/ 6080 localhost:5900 \
    > /artifacts/logs/novnc.log 2>&1 &
web_pid=$!

while kill -0 "${app_pid}" && kill -0 "${vnc_pid}" && kill -0 "${web_pid}"; do
    sleep 1
done
exit 1

