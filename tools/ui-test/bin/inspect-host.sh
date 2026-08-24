#!/usr/bin/env bash
set -Eeuo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=common.sh
source "${script_dir}/common.sh"

repository_root="$(ui_test_root)"
require_docker
artifact_dir="${repository_root}/artifacts/ui-test/inspect"
mkdir -p "${artifact_dir}"
chmod 0700 "${artifact_dir}"

export COMPOSE_PROJECT_NAME=cliprelay-ui-inspect
export GUI_TEST_ARTIFACT_DIR="${artifact_dir}"
export GUI_TEST_UID="$(id -u)"
export GUI_TEST_GID="$(id -g)"

if [[ "${1:-}" == "down" ]]; then
    compose_base "${repository_root}" --profile inspect down --remove-orphans
    echo "PASS stopped the localhost-only ClipRelay GUI inspector."
    exit 0
fi

if [[ -z "${GUI_TEST_VNC_PASSWORD:-}" ]]; then
    GUI_TEST_VNC_PASSWORD="$(od -An -N4 -tx1 /dev/urandom | tr -d ' \n')"
    export GUI_TEST_VNC_PASSWORD
fi

novnc_port="${GUI_TEST_NOVNC_PORT:-6080}"
echo "ClipRelay inspector: http://127.0.0.1:${novnc_port}/vnc.html"
echo "Ephemeral VNC password: ${GUI_TEST_VNC_PASSWORD}"
echo "Press Ctrl-C to stop; no host display or input device is mounted."
compose_base "${repository_root}" --profile inspect up --build --abort-on-container-exit ui-inspect

