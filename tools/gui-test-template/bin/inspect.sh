#!/usr/bin/env bash
set -Eeuo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=common.sh
source "${script_dir}/common.sh"

root="$(project_root)"
require_docker
artifact_dir="${root}/artifacts/gui-test/inspect"
mkdir -p "${artifact_dir}"
chmod 0700 "${artifact_dir}"
export GUI_TEST_ARTIFACT_DIR="${artifact_dir}"
export GUI_TEST_UID="$(id -u)"
export GUI_TEST_GID="$(id -g)"
export COMPOSE_PROJECT_NAME=gui-test-inspect

if [[ "${1:-}" == "down" ]]; then
    compose_template "${root}" --profile inspect down --remove-orphans
    echo "PASS stopped the localhost-only GUI inspector."
    exit 0
fi

if [[ -z "${GUI_TEST_VNC_PASSWORD:-}" ]]; then
    GUI_TEST_VNC_PASSWORD="$(od -An -N4 -tx1 /dev/urandom | tr -d ' \n')"
    export GUI_TEST_VNC_PASSWORD
fi
echo "Inspector: http://127.0.0.1:${GUI_TEST_NOVNC_PORT:-6080}/vnc.html"
echo "Ephemeral VNC password: ${GUI_TEST_VNC_PASSWORD}"
compose_template "${root}" --profile inspect up --build --abort-on-container-exit gui-inspect

