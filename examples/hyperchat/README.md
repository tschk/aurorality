# HyperChat

HyperChat is the Aurorality protocol-messaging example. The **default UI** is authored in **`views/main.crepus`** and compiled by **`aurorality swiftgen`** into native SwiftUI at **`Generated/HyperChatGeneratedView.swift`**. Transports are implemented in **`rust-backend`** (`hyperchat-backend`) using **[crates.io `eqswift` 0.1.1](https://crates.io/crates/eqswift)** and UniFFI, linked as `libhyperchat_backend.dylib`.

`views/main.json` is still produced for tooling parity. The app itself runs the generated SwiftUI view, so `.crepus` remains the source of truth while controls such as `List`, `Picker`, `TextField`, and `Button` are native macOS SwiftUI widgets.

`App.swift` / `HyperChatModel.swift` use **`#if canImport(Aurorality)`** so SwiftPM still imports the **Aurorality** product; Brisk’s flat `swiftc` build does not need that import.

## Run (Brisk)

Brisk **0.1.7+** (sibling checkout **`brisk`** next to this repo, or `cargo install --path` that tree) runs **`[pre_build]`**, builds, then **`embedded_dylibs`** — copies `libaurorality_core.dylib` into `AurorHyperChat.app/Contents/Frameworks`, repoints the main binary to `@rpath`, re-signs. No extra shell script.

Install the patched CLI, then from `examples/hyperchat`:

```bash
cargo install --path ../../../brisk --force   # when `aurorality` and `brisk` share the same parent directory
cd examples/hyperchat
brisk run
```

Optional: **`./run.sh`** is a one-line `brisk run` wrapper.

Older Brisk / no `embedded_dylibs`: launch still dies in dyld when `target/debug/...` path missing (looks like “check with the developer” / macOS compatibility). Upgrade Brisk or run `cargo build -p aurorality-core --features js` before `brisk run`.

### Manual swiftgen (if pre_build did not run)

```bash
../../target/debug/aurorality swiftgen \
  --view views/main.crepus --out Generated \
  --view-name HyperChatGeneratedView --context-type HyperChatContext
```

**UniFFI checksum mismatch:** `../../scripts/sync-uniffi.sh` from the Aurorality repo root, then `brisk run` again.

## Run (SwiftPM, no Brisk)

```bash
cd examples/hyperchat
swift build
swift run HyperChat
```

## Template workflow

Brisk builds from `.brisk-sources`; keep it refreshed from `Sources/`, `Generated/`, `FFI/`, and the shared Aurorality Swift files. The configured `[pre_build]` documents those copy/codegen steps, but if your installed Brisk does not run it, regenerate manually:

```bash
cargo run --manifest-path ../../Cargo.toml -p aurorality-cli -- swiftgen \
  --view views/main.crepus \
  --out Generated \
  --view-name HyperChatGeneratedView \
  --context-type HyperChatContext
```

Then run `brisk build` or `brisk run`.

## Dev overlay + hybrid reload

Set `AURORALITY_DEV=1` and run from the Aurorality repo:

```bash
cargo run -p aurorality-cli -- dev examples/hyperchat/views \
  --swiftgen-view examples/hyperchat/views/main.crepus \
  --swiftgen-out examples/hyperchat/Generated \
  --swiftgen-name HyperChatGeneratedView \
  --swiftgen-context-type HyperChatContext
```

Brisk’s rebuild loop picks up regenerated `HyperChatGeneratedView.swift`; the overlay shows swiftgen/IR status immediately.

## Persistence

Conversation transcripts and UI prefs are snapshotted via **`aurorStore*`** (`HyperChatModel`) under the app’s bundle id.

## Protocols

Matrix uses the Matrix Client-Server API:

```bash
export MATRIX_HOMESERVER=https://matrix.example
export MATRIX_USER_ID=@user:matrix.example
export MATRIX_ACCESS_TOKEN=...
export MATRIX_ROOM_ID='!room:matrix.example'
```

Stalwart targets a local Stalwart/JMAP service, typically the sibling `../stalwart-lite` checkout:

```bash
export STALWART_BASE_URL=http://localhost:8080
export STALWART_USERNAME=...
export STALWART_PASSWORD=...
```

HyperChat reports each service’s configuration using the **`hyperchat-backend`** health JSON exports (same env vars as above).

Bitchat is a mesh fallback in the demo UI. There is no linkable Swift library for real sends; the model surfaces that limitation in status text.

## Files

- `views/main.crepus` — primary UI (Tailwind-style classes + native tags like `list`; mapped by `swiftgen` to SwiftUI).
- `Generated/HyperChatGeneratedView.swift` — generated; do not edit by hand.
- `Sources/HyperChatDevConnectView.swift` — connect sheet for `aurorality dev` (host/port + status).
- `Sources/App.swift` — **`HyperChatGeneratedView`** + **`HyperChatGeneratedViewCommands`** (from `.crepus` `menubar`), **`AurorDevOverlay`** for optional HUD against `aurorality dev`, sheet-based Settings, focus hooks for polling/notifications.
- `rust-backend/` — **`hyperchat-backend`** Rust crate: [eqswift **0.1.1** on crates.io](https://crates.io/crates/eqswift) + UniFFI exports (`matrix_health_json`, `stalwart_health_json`, etc.). Build: `cargo build -p hyperchat-backend` from the repo root.
- `Generated/hyperchat_backend.swift`, `FFI/` — UniFFI output for Swift (regenerated in `[pre_build]` via `uniffi-bindgen` on `libhyperchat_backend.dylib`; optional [`cargo-eqswift`](https://crates.io/crates/cargo-eqswift): `cargo eqswift swift` from `rust-backend`).
- `.brisk.toml` — `pre_build` (Rust, UniFFI, swiftgen, copy sources), `app.embedded_dylibs` (Rust dylib inside `.app`).
