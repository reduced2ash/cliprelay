#!/usr/bin/env bash

start_x_session() {
    Xvfb "${DISPLAY}" \
        -screen 0 "${GUI_TEST_SCREEN:-1440x900x24}" \
        -dpi "${GUI_TEST_DPI:-96}" \
        -nolisten tcp \
        -noreset \
        -ac \
        > /artifacts/logs/xvfb.log 2>&1 &
    xvfb_pid=$!

    local attempt
    for attempt in $(seq 1 100); do
        if xdpyinfo -display "${DISPLAY}" >/dev/null 2>&1; then
            break
        fi
        if ! kill -0 "${xvfb_pid}" >/dev/null 2>&1; then
            echo "Xvfb exited before the display became ready." >&2
            return 1
        fi
        sleep 0.1
    done
    if ! xdpyinfo -display "${DISPLAY}" >/dev/null 2>&1; then
        echo "Xvfb did not become ready." >&2
        return 1
    fi

    openbox > /artifacts/logs/openbox.log 2>&1 &
    openbox_pid=$!
    sleep 0.3
    xsetroot -solid '#20242b'
}

stop_x_session() {
    if [[ -n "${openbox_pid:-}" ]]; then
        kill -TERM "${openbox_pid}" >/dev/null 2>&1 || true
    fi
    if [[ -n "${xvfb_pid:-}" ]]; then
        kill -TERM "${xvfb_pid}" >/dev/null 2>&1 || true
    fi
}

