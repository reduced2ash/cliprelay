#!/usr/bin/env bash
set -Eeuo pipefail

mkdir -p \
    /artifacts/diffs \
    /artifacts/logs \
    /artifacts/screenshots \
    "${HOME}/.cache" \
    "${HOME}/.config" \
    "${HOME}/.local/share"

exec "$@"

