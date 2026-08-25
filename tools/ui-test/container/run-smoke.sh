#!/usr/bin/env bash
set -uo pipefail

# shellcheck source=x-session.sh
source /opt/gui-test/bin/x-session.sh

failure_reason=""
app_pid=""

record_failure() {
    failure_reason="$1"
    echo "$1" >&2
    return 1
}

stop_app() {
    if [[ -n "${app_pid}" ]] && kill -0 "${app_pid}" >/dev/null 2>&1; then
        kill -TERM "${app_pid}" >/dev/null 2>&1 || true
        for _ in $(seq 1 30); do
            kill -0 "${app_pid}" >/dev/null 2>&1 || break
            sleep 0.1
        done
        kill -KILL "${app_pid}" >/dev/null 2>&1 || true
        wait "${app_pid}" >/dev/null 2>&1 || true
    fi
    app_pid=""
    for _ in $(seq 1 30); do
        if ! xdotool search --onlyvisible --name '^ClipRelay' >/dev/null 2>&1; then
            break
        fi
        sleep 0.1
    done
}

cleanup() {
    stop_app
    stop_x_session
}
trap cleanup EXIT

wait_for_window() {
    local attempt window_id
    for attempt in $(seq 1 200); do
        if ! kill -0 "${app_pid}" >/dev/null 2>&1; then
            return 1
        fi
        window_id="$(xdotool search --onlyvisible --name '^ClipRelay' 2>/dev/null | head -n 1 || true)"
        if [[ -n "${window_id}" ]]; then
            printf '%s\n' "${window_id}"
            return 0
        fi
        sleep 0.1
    done
    return 1
}

wait_for_capture() {
    local path="$1" attempt
    for attempt in $(seq 1 200); do
        if [[ -s "${path}" ]]; then
            return 0
        fi
        if ! kill -0 "${app_pid}" >/dev/null 2>&1; then
            return 1
        fi
        sleep 0.1
    done
    return 1
}

assert_image() {
    local path="$1" dimensions mean
    dimensions="$(identify -format '%w %h' "${path}" 2>/dev/null)" || return 1
    mean="$(identify -format '%[fx:mean]' "${path}" 2>/dev/null)" || return 1
    awk -v dims="${dimensions}" -v mean="${mean}" '
        BEGIN {
            split(dims, size, " ");
            if (size[1] < 700 || size[2] < 520 || mean <= 0.005 || mean >= 0.995) exit 1;
        }
    '
}

compare_baseline() {
    local case_name="$1" screenshot="$2"
    local baseline="/opt/gui-test/baselines/${case_name}.png"
    local diff="/artifacts/diffs/${case_name}.png"
    local metric max_pixels
    max_pixels="${GUI_TEST_MAX_DIFF_PIXELS:-0}"
    if [[ ! -f "${baseline}" ]]; then
        printf '%s\t%s\t%s\n' "${case_name}" "not-configured" "-" >> /artifacts/metrics.tsv
        return 0
    fi
    metric="$(compare -fuzz 2% -metric AE "${baseline}" "${screenshot}" "${diff}" 2>&1 >/dev/null || true)"
    metric="${metric%% *}"
    [[ "${metric}" =~ ^[0-9]+$ ]] || return 1
    printf '%s\t%s\t%s\n' "${case_name}" "${metric}" "${max_pixels}" >> /artifacts/metrics.tsv
    (( metric <= max_pixels ))
}

assert_distinct_states() {
    local first="$1" second="$2"
    local first_path="/artifacts/screenshots/${first}.png"
    local second_path="/artifacts/screenshots/${second}.png"
    local diff="/artifacts/diffs/state-${first}-${second}.png"
    local metric
    metric="$(compare -fuzz 2% -metric AE "${first_path}" "${second_path}" "${diff}" 2>&1 >/dev/null || true)"
    metric="${metric%% *}"
    [[ "${metric}" =~ ^[0-9]+$ ]] || return 1
    printf '%s-vs-%s\t%s\t%s\n' "${first}" "${second}" "${metric}" ">1000" >> /artifacts/metrics.tsv
    (( metric > 1000 ))
}

capture_case() {
    local case_name="$1"
    shift
    local case_data="/tmp/cases/${case_name}"
    local screenshot="/artifacts/screenshots/${case_name}.png"
    local app_log="/artifacts/logs/app-${case_name}.log"
    local window_id geometry width height

    mkdir -p "${case_data}" /tmp/library
    rm -f "${screenshot}"
    env \
        CLIPRELAY_DATA_DIR="${case_data}" \
        CLIPRELAY_CAPTURE="${screenshot}" \
        CLIPRELAY_CAPTURE_AFTER="${GUI_TEST_CAPTURE_AFTER:-35}" \
        RUST_LOG=info \
        "$@" \
        /usr/local/bin/cliprelay \
        --data-dir "${case_data}" \
        --library /tmp/library \
        --window-width 1180 \
        --window-height 760 \
        >"${app_log}" 2>&1 &
    app_pid=$!

    window_id="$(wait_for_window)" || {
        stop_app
        record_failure "${case_name}: ClipRelay did not create a visible X11 window."
        return 1
    }
    sleep "${GUI_TEST_SETTLE_SECONDS:-3}"
    kill -0 "${app_pid}" >/dev/null 2>&1 || {
        stop_app
        record_failure "${case_name}: ClipRelay exited before capture."
        return 1
    }

    geometry="$(xdotool getwindowgeometry --shell "${window_id}" 2>/dev/null)" || {
        stop_app
        record_failure "${case_name}: could not read the ClipRelay window geometry."
        return 1
    }
    width="$(awk -F= '$1 == "WIDTH" { print $2 }' <<<"${geometry}")"
    height="$(awk -F= '$1 == "HEIGHT" { print $2 }' <<<"${geometry}")"
    if (( width < 700 || height < 520 )); then
        stop_app
        record_failure "${case_name}: window geometry ${width}x${height} is below the app minimum."
        return 1
    fi

    if ! wait_for_capture "${screenshot}"; then
        if kill -0 "${app_pid}" >/dev/null 2>&1; then
            record_failure "${case_name}: GPUI did not produce a renderer capture within 20 seconds."
        else
            record_failure "${case_name}: ClipRelay exited before renderer capture completed."
        fi
        stop_app
        return 1
    fi
    if ! assert_image "${screenshot}"; then
        stop_app
        record_failure "${case_name}: screenshot is missing, blank, or has invalid bounds."
        return 1
    fi
    if ! compare_baseline "${case_name}" "${screenshot}"; then
        stop_app
        record_failure "${case_name}: visual diff exceeds GUI_TEST_MAX_DIFF_PIXELS."
        return 1
    fi

    stop_app
}

run_suite() {
    start_x_session || return 1
    rm -rf /tmp/library
    mkdir -p /tmp/library/nested
    if ! ffmpeg -hide_banner -loglevel error -y \
        -f lavfi -i 'color=c=red:s=562x1024:r=24:d=1.2' \
        -vf 'setsar=1408/1405' \
        -an -c:v libx264 -pix_fmt yuv420p /tmp/library/nested/clip-000.mp4; then
        record_failure "fixture: ffmpeg could not create the workflow video."
        return 1
    fi
    for index in $(seq -w 1 280); do
        cp /tmp/library/nested/clip-000.mp4 "/tmp/library/nested/clip-${index}.mp4"
    done
    {
        echo "date=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
        echo "display=${DISPLAY}"
        echo "screen=${GUI_TEST_SCREEN:-1440x900x24}"
        echo "dpi=${GUI_TEST_DPI:-96}"
        echo "locale=${LC_ALL}"
        echo "timezone=${TZ}"
        echo "cliprelay_sha256=$(sha256sum /usr/local/bin/cliprelay | awk '{print $1}')"
        echo "font_set_sha256=$(fc-list | LC_ALL=C sort | sha256sum | awk '{print $1}')"
        ffmpeg -version | head -n 1
        gst-launch-1.0 --version | head -n 1
    } > /artifacts/environment.txt
    printf 'case\tdiff_pixels\tallowed_pixels\n' > /artifacts/metrics.tsv

    capture_case library || return 1
    capture_case history CLIPRELAY_PAGE=history || return 1
    capture_case settings CLIPRELAY_PAGE=settings CLIPRELAY_SETTINGS_SCROLL=0 || return 1
    capture_case command CLIPRELAY_OPEN_COMMAND=1 CLIPRELAY_QUERY=library || return 1
    capture_case workflow CLIPRELAY_EXERCISE_WORKFLOW=1 CLIPRELAY_CAPTURE_AFTER=110 || return 1
    if [[ "$(convert /artifacts/screenshots/workflow.png -crop 1x1+1025+270 \
        -format '%[fx:r>b+0.2]' info:)" != "1" ]]; then
        record_failure "workflow: Prepare rendered a known red video with swapped color channels."
        return 1
    fi
    if ! grep -q 'gpui-video-player ready' /artifacts/logs/app-workflow.log \
        || ! grep -q 'ui-test workflow played a real hover preview' /artifacts/logs/app-workflow.log \
        || ! grep -q 'ui-test workflow observed Prepare autoplay' /artifacts/logs/app-workflow.log \
        || ! grep -q 'ui-test workflow retained user scroll after Library reveal' /artifacts/logs/app-workflow.log \
        || ! grep -q 'library reveal resolved media' /artifacts/logs/app-workflow.log; then
        record_failure "workflow: scroll recovery, hover preview, Random, Prepare autoplay, or source reveal did not complete."
        return 1
    fi
    assert_distinct_states library history || {
        record_failure "library/history: semantic states produced no meaningful visual change."
        return 1
    }
    assert_distinct_states library settings || {
        record_failure "library/settings: semantic states produced no meaningful visual change."
        return 1
    }
    assert_distinct_states library command || {
        record_failure "library/command: semantic states produced no meaningful visual change."
        return 1
    }
    assert_distinct_states library workflow || {
        record_failure "library/workflow: the exercised playback state produced no meaningful visual change."
        return 1
    }
    return 0
}

if run_suite; then
    echo "PASS isolated ClipRelay GUI smoke test (5 semantic states, including Random/playback/reveal). Artifacts: /artifacts" > /artifacts/summary.txt
    exit 0
fi

if [[ -z "${failure_reason}" ]]; then
    failure_reason="GUI harness setup failed; inspect container.log and logs/."
fi
echo "FAIL ${failure_reason} Artifacts: /artifacts" > /artifacts/summary.txt
exit 1
