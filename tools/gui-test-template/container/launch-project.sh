#!/usr/bin/env bash
set -Eeuo pipefail

# ADOPT: set isolated fixture/data environment and deterministic app arguments
# here. Keep this as an exec so the harness observes the real app process.
exec /opt/gui-test/app

