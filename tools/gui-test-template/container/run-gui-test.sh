#!/usr/bin/env bash
set -uo pipefail

# shellcheck source=x-session.sh
source /opt/gui-test/bin/x-session.sh

app_pid=""
cleanup() {
    if [[ -n "${app_pid}" ]]; then
        kill -TERM "${app_pid}" >/dev/null 2>&1 || true
        wait "${app_pid}" >/dev/null 2>&1 || true
    fi
    stop_x_session
}
trap cleanup EXIT

fail() {
    echo "FAIL $1 Artifacts: /artifacts" > /artifacts/summary.container.txt
    echo "$1" >&2
    exit 1
}

start_x_session || fail "Xvfb session did not become ready."
/opt/gui-test/bin/launch-project.sh > /artifacts/logs/app.log 2>&1 &
app_pid=$!

window_id=""
for _ in $(seq 1 200); do
    kill -0 "${app_pid}" >/dev/null 2>&1 || fail "The application exited before creating a window."
    window_id="$(xdotool search --onlyvisible --name "${GUI_TEST_WINDOW_NAME}" 2>/dev/null | head -n 1 || true)"
    [[ -n "${window_id}" ]] && break
    sleep 0.1
done
[[ -n "${window_id}" ]] || fail "No visible application window matched GUI_TEST_WINDOW_NAME."

/opt/gui-test/bin/project-assert.sh "${window_id}" || fail "The project semantic assertion failed."
sleep "${GUI_TEST_SETTLE_SECONDS:-2}"
kill -0 "${app_pid}" >/dev/null 2>&1 || fail "The application exited before capture."
import -display "${DISPLAY}" -window "${window_id}" /artifacts/screenshots/smoke.png \
    2>> /artifacts/logs/app.log || fail "X11 window capture failed."

dimensions="$(identify -format '%w %h' /artifacts/screenshots/smoke.png 2>/dev/null)" \
    || fail "The screenshot is unreadable."
mean="$(identify -format '%[fx:mean]' /artifacts/screenshots/smoke.png 2>/dev/null)" \
    || fail "The screenshot is unreadable."
awk -v dims="${dimensions}" -v mean="${mean}" '
    BEGIN { split(dims, s, " "); if (s[1] < 1 || s[2] < 1 || mean <= 0.005 || mean >= 0.995) exit 1 }
' || fail "The screenshot is blank or has invalid bounds."

baseline=/opt/gui-test/baselines/smoke.png
if [[ -f "${baseline}" ]]; then
    metric="$(compare -fuzz 2% -metric AE "${baseline}" /artifacts/screenshots/smoke.png \
        /artifacts/diffs/smoke.png 2>&1 >/dev/null || true)"
    metric="${metric%% *}"
    [[ "${metric}" =~ ^[0-9]+$ ]] || fail "Visual comparison could not be evaluated."
    (( metric <= ${GUI_TEST_MAX_DIFF_PIXELS:-0} )) \
        || fail "Visual diff exceeds GUI_TEST_MAX_DIFF_PIXELS."
fi

echo "PASS isolated GUI test. Artifacts: /artifacts" > /artifacts/summary.container.txt

