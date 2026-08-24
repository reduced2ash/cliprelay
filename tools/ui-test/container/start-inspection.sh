#!/usr/bin/env bash
set -Eeuo pipefail

# shellcheck source=x-session.sh
source /opt/gui-test/bin/x-session.sh

if [[ -z "${GUI_TEST_VNC_PASSWORD:-}" ]]; then
    echo "GUI_TEST_VNC_PASSWORD is required for the inspection profile." >&2
    exit 2
fi

app_pid=""
vnc_pid=""
web_pid=""

cleanup() {
    for process_id in "${web_pid}" "${vnc_pid}" "${app_pid}"; do
        if [[ -n "${process_id}" ]]; then
            kill -TERM "${process_id}" >/dev/null 2>&1 || true
        fi
    done
    stop_x_session
}
trap cleanup EXIT INT TERM

start_x_session
mkdir -p /tmp/inspect-data /tmp/inspect-library /tmp/vnc
x11vnc -storepasswd "${GUI_TEST_VNC_PASSWORD}" /tmp/vnc/passwd >/dev/null

CLIPRELAY_DATA_DIR=/tmp/inspect-data \
RUST_LOG=info \
/usr/local/bin/cliprelay \
    --data-dir /tmp/inspect-data \
    --library /tmp/inspect-library \
    --window-width 1180 \
    --window-height 760 \
    > /artifacts/logs/app-inspect.log 2>&1 &
app_pid=$!

x11vnc \
    -display "${DISPLAY}" \
    -localhost \
    -forever \
    -shared \
    -rfbauth /tmp/vnc/passwd \
    -rfbport 5900 \
    > /artifacts/logs/x11vnc.log 2>&1 &
vnc_pid=$!

websockify --web=/usr/share/novnc/ 6080 localhost:5900 \
    > /artifacts/logs/novnc.log 2>&1 &
web_pid=$!

echo "noVNC is ready on container port 6080 with VNC authentication."
while kill -0 "${app_pid}" && kill -0 "${vnc_pid}" && kill -0 "${web_pid}"; do
    sleep 1
done
echo "The app, VNC server, or noVNC proxy exited unexpectedly." >&2
exit 1

