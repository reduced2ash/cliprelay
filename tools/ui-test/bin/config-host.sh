#!/usr/bin/env bash
set -Eeuo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=common.sh
source "${script_dir}/common.sh"

repository_root="$(ui_test_root)"
require_docker
mkdir -p "${repository_root}/artifacts/ui-test/latest"
export GUI_TEST_ARTIFACT_DIR="${repository_root}/artifacts/ui-test/latest"
export GUI_TEST_UID="$(id -u)"
export GUI_TEST_GID="$(id -g)"
unset GUI_TEST_VNC_PASSWORD
compose_base "${repository_root}" config >/dev/null
echo "PASS ClipRelay GUI Compose configuration is valid."
