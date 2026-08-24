#!/usr/bin/env bash
set -Eeuo pipefail

ui_test_root() {
    cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd
}

require_docker() {
    if ! command -v docker >/dev/null 2>&1; then
        echo "FAIL isolated GUI tests require Docker Engine and Docker Compose v2." >&2
        return 127
    fi
    if ! docker info >/dev/null 2>&1; then
        echo "FAIL Docker Engine is unavailable or this user cannot access it." >&2
        return 126
    fi
    if ! docker compose version >/dev/null 2>&1; then
        echo "FAIL Docker Compose v2 is unavailable." >&2
        return 127
    fi
}

compose_base() {
    local repository_root="$1"
    shift
    docker compose \
        --project-directory "${repository_root}" \
        -f "${repository_root}/tools/ui-test/compose.yaml" \
        "$@"
}
