#!/usr/bin/env bash
# Rebuild libaurorality_core and regenerate Swift UniFFI bindings into generated/.
# Run this after changing aurorality-core's exported API or when you see:
#   Fatal error: UniFFI API checksum mismatch: try cleaning and rebuilding your project
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
cargo build -p aurorality-core --features js
cargo run -p aurorality-core --features js --bin uniffi-bindgen generate \
  --library target/debug/libaurorality_core.dylib \
  --language swift \
  --out-dir generated
cp -f generated/aurorality_coreFFI.modulemap generated/module.modulemap
