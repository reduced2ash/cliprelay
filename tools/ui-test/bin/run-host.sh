#!/usr/bin/env bash
set -Eeuo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=common.sh
source "${script_dir}/common.sh"

repository_root="$(ui_test_root)"
require_docker

run_stamp="$(date -u +%Y%m%dT%H%M%SZ)-$$"
artifact_dir="${repository_root}/artifacts/ui-test/${run_stamp}"
mkdir -p "${artifact_dir}"
chmod 0700 "${artifact_dir}"

export GUI_TEST_ARTIFACT_DIR="${artifact_dir}"
export GUI_TEST_UID="$(id -u)"
export GUI_TEST_GID="$(id -g)"
export COMPOSE_PROJECT_NAME="cliprelay-ui-test-${GUI_TEST_UID}-${run_stamp,,}"
unset GUI_TEST_VNC_PASSWORD

cleanup() {
    compose_base "${repository_root}" down --remove-orphans >/dev/null 2>&1 || true
}
trap cleanup EXIT

if ! compose_base "${repository_root}" config >"${artifact_dir}/compose-config.yaml" 2>"${artifact_dir}/compose-config.log"; then
    printf 'FAIL Compose configuration is invalid. Artifacts: %s\n' "${artifact_dir}" | tee "${artifact_dir}/summary.txt"
    exit 2
fi

if ! compose_base "${repository_root}" build ui-test >"${artifact_dir}/build.log" 2>&1; then
    printf 'FAIL isolated GUI image build. Artifacts: %s\n' "${artifact_dir}" | tee "${artifact_dir}/summary.txt"
    exit 3
fi

if compose_base "${repository_root}" run --rm --no-deps ui-test >"${artifact_dir}/container.log" 2>&1; then
    if [[ -s "${artifact_dir}/summary.txt" ]]; then
        mv "${artifact_dir}/summary.txt" "${artifact_dir}/summary.container.txt"
        sed "s#Artifacts: /artifacts#Artifacts: ${artifact_dir}#" "${artifact_dir}/summary.container.txt" \
            | tee "${artifact_dir}/summary.txt"
    else
        printf 'PASS isolated ClipRelay GUI smoke test. Artifacts: %s\n' "${artifact_dir}" | tee "${artifact_dir}/summary.txt"
    fi
else
    status=$?
    if [[ -s "${artifact_dir}/summary.txt" ]]; then
        mv "${artifact_dir}/summary.txt" "${artifact_dir}/summary.container.txt"
        sed "s#Artifacts: /artifacts#Artifacts: ${artifact_dir}#" "${artifact_dir}/summary.container.txt" \
            | tee "${artifact_dir}/summary.txt"
    else
        printf 'FAIL isolated ClipRelay GUI smoke test (exit %s). Artifacts: %s\n' "${status}" "${artifact_dir}" | tee "${artifact_dir}/summary.txt"
    fi
    exit "${status}"
fi
