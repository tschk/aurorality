#!/usr/bin/env bash
# Thin wrapper; needs Brisk ≥0.1.7 (embedded_dylibs) or install from sibling ../brisk:
#   cargo install --path ../../../brisk --force
set -euo pipefail
cd "$(dirname "$0")"
exec brisk run "$@"
