#!/usr/bin/env bash
set -Eeuo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=common.sh
source "${script_dir}/common.sh"

root="$(project_root)"
require_docker
stamp="$(date -u +%Y%m%dT%H%M%SZ)-$$"
artifact_dir="${root}/artifacts/gui-test/${stamp}"
mkdir -p "${artifact_dir}"
chmod 0700 "${artifact_dir}"
export GUI_TEST_ARTIFACT_DIR="${artifact_dir}"
export GUI_TEST_UID="$(id -u)"
export GUI_TEST_GID="$(id -g)"
export COMPOSE_PROJECT_NAME="gui-test-${GUI_TEST_UID}-${stamp,,}"
unset GUI_TEST_VNC_PASSWORD

cleanup() {
    compose_template "${root}" down --remove-orphans >/dev/null 2>&1 || true
}
trap cleanup EXIT

if ! compose_template "${root}" config >"${artifact_dir}/compose-config.yaml" 2>"${artifact_dir}/compose-config.log"; then
    echo "FAIL invalid GUI-test Compose model. Artifacts: ${artifact_dir}"
    exit 2
fi
if ! compose_template "${root}" build gui-test >"${artifact_dir}/build.log" 2>&1; then
    echo "FAIL GUI-test image build. Artifacts: ${artifact_dir}"
    exit 3
fi
if compose_template "${root}" run --rm --no-deps gui-test >"${artifact_dir}/container.log" 2>&1; then
    echo "PASS isolated GUI test. Artifacts: ${artifact_dir}" | tee "${artifact_dir}/summary.txt"
else
    status=$?
    echo "FAIL isolated GUI test (exit ${status}). Artifacts: ${artifact_dir}" | tee "${artifact_dir}/summary.txt"
    exit "${status}"
fi
