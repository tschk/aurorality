#!/usr/bin/env bash
set -euo pipefail

# Pre-build: Rust core + UniFFI bindings
(cd "$(dirname "$0")/../.." && cargo build -p aurorality-core --features js)
(cd "$(dirname "$0")/../.." && cargo run -p aurorality-core --features js --bin uniffi-bindgen generate \
  --library target/debug/libaurorality_core.dylib \
  --language swift --out-dir generated)
cp "$(dirname "$0")/../../generated/aurorality_coreFFI.modulemap" "$(dirname "$0")/../../generated/module.modulemap"

exec swift run "$@"
