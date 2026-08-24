#!/usr/bin/env bash
set -Eeuo pipefail

project_root() {
    cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd
}

require_docker() {
    command -v docker >/dev/null 2>&1 || {
        echo "FAIL GUI tests require Docker Engine and Docker Compose v2." >&2
        return 127
    }
    docker info >/dev/null 2>&1 || {
        echo "FAIL Docker Engine is unavailable or inaccessible." >&2
        return 126
    }
    docker compose version >/dev/null 2>&1 || {
        echo "FAIL Docker Compose v2 is unavailable." >&2
        return 127
    }
}

compose_template() {
    local root="$1"
    shift
    docker compose --project-directory "${root}" \
        -f "${root}/tools/gui-test-template/compose.yaml" "$@"
}

