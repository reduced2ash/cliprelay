#!/usr/bin/env bash
set -Eeuo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=common.sh
source "${script_dir}/common.sh"

repository_root="$(ui_test_root)"
require_docker
export COMPOSE_PROJECT_NAME=cliprelay-ui-inspect
export GUI_TEST_ARTIFACT_DIR="${repository_root}/artifacts/ui-test/inspect"
compose_base "${repository_root}" --profile inspect down --remove-orphans
echo "PASS removed ClipRelay GUI inspection containers and network; artifacts were preserved."

