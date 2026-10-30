# Aurorality

A SwiftUI + Rust shell for `.crepus` templates — build native iOS/macOS apps with a Tailwind-inspired template syntax.

## Overview

Aurorality combines:
- **`.crepus` templates** — Tailwind-inspired declarative syntax that compiles to SwiftUI. From [crepuscularity](https://github.com/semitechnological/crepuscularity).
- **Rust rendering backend** — Parses templates + manages plugin logic via UniFFI
- **SwiftUI renderer** — Reactive view tree from the IR with hot reload support

```
div flex flex-col gap-6 p-8
  span text-3xl font-bold
    "Hello from aurorality"
  span text-base
    "Edit this file and watch SwiftUI update live."
```

## Project Structure

```
aurorality/
├── crates/
│   ├── aurorality-core/    # UniFFI bridge, render API, plugins
│   ├── aurorality-cli/  # Dev server, build, scaffold
│   └── aurorality-macros/
├── swift/Sources/Aurorality/
│   ├── IR.swift          # ViewIr, ViewNode, ViewStyle
│   ├── AurorView.swift    # SwiftUI renderer
│   ├── AurorState.swift   # State management
│   ├── AurorBridge.swift # Plugin calls
│   ├── HotReloadClient.swift
│   ├── HotReloadBus.swift / HotReloadHUD.swift / AurorDevOverlay.swift
│   └── TransportTypes.swift
└── examples/
    ├── basic/
    ├── counter/
    ├── hyperchat/
    └── textanalyzer/
```

## Install

```bash
# Install the CLI
cargo install aurorality-cli --git https://github.com/your-org/aurorality

# Or use via the crepus CLI (install both)
cargo install crepuscularity-cli --git https://github.com/semitechnological/crepuscularity
# Then: crepus aurora dev --watch views

# Verify
aurorality --version
```

## Quick Start

```bash
# Build the Rust core with JavaScript plugin support
cargo build -p aurorality-core --features js

# Generate eqswift/UniFFI Swift bindings
cargo run -p aurorality-core --features js --bin uniffi-bindgen generate \
  --library target/debug/libaurorality_core.dylib \
  --language swift --out-dir generated
cp generated/aurorality_coreFFI.modulemap generated/module.modulemap

# Run an example
cd examples/counter && ./run.sh

# Or start the dev server (from the project directory)
cd examples/basic && cargo run --manifest-path ../../Cargo.toml -p aurorality-cli -- dev
```

## Template Syntax

`.crepus` files use a Lisp-like indentation syntax with Tailwind classes:

```
div.flex.flex-col.gap-4.p-8
  span.text-2xl.font-bold
    "Hello, {name}!"
  button.rounded.bg-blue-500
    "Click me"
```

### Supported Elements

| Element | Description |
|--------|------------|
| `div` | Stack container (VStack/HStack based on axis) |
| `span` | Text node |
| `button` | Tappable button |
| `image` | AsyncImage or bundled asset |
| `scroll` | ScrollView wrapper |
| `slotRotate` | Timed phrase rotation |

### Supported Styles (ViewStyle IR)

See `crepuscularity-native/src/ir.rs` for the full ViewStyle schema.

#### Spacing (`ir.rs` line 29-59)
- `p-[1-96]`, `px-`, `py-`, `pt-`, `pb-`, `pl-`, `pr-`
- `m-[1-96]`, `mx-`, `my-`, `mt-`, `mb-`, `ml-`, `mr-`

#### Sizing (line 61-83)
- `w-[1-96]`, `w-full`, `w-screen`, `w-fit`, `w-1/2`, `w-1/3`
- `h-[1-96]`, `h-full`, `h-screen`, `h-fit`
- `min-w-[ ]`, `max-w-[ ]`, `min-h-[ ]`, `max-h-[ ]`
- `size-[ ]`, `aspect-square`, `aspect-video`
- `widthFraction`, `heightFraction`

#### Typography (line 85-109)
- `text-[xs|sm|base|lg...|9xl]`
- `font-[thin|extralight|light|normal|medium|semibold|bold|extrabold|black]`
- `font-[sans|serif|mono]`
- `text-align`, `leading-[ ]`, `tracking-[ ]`
- `uppercase`, `lowercase`, `capitalize`
- `italic`, `underline`, `line-through`

#### Color (line 111-123)
- `text-[named|hex]`, `bg-[named|hex]`
- Tailwind palette: `text-red-500`, `bg-blue-300/50`

#### Border (line 117-123)
- `border-[0|2|4|8]`, `rounded-[none|sm|md|lg|xl|2xl|3xl|full]`

#### Visibility (line 125-132)
- `opacity-[0-100]`, `hidden`, `invisible`, `overflow-hidden`

#### Flex (line 134-143)
- `flex-[1|auto|none]`, `flex-wrap`, `flex-nowrap`
- `grow`, `shrink`, `items-[start|center|end|stretch|baseline]`
- `justify-[start|center|end|between|around|evenly]`

## Plugins

Built-in Rust plugins accessible via `AurorBridge`:

### `core` Plugin
- `ping` → `{ pong: true }`
- `echo(payload)` → echo back
- `timestamp` → `{ unixMs, unixNs }`
- `randomU32(max?)` → `{ value }`

### `app` Plugin
- `version` → `{ aurority }`
- `platform` → `{ os, arch }`

### `stats` Plugin
- `analyze(text)` → `{ wordCount, charCount, lineCount, topWord, topWordCount }`
- `tokenize(text)` → `{ tokens: [] }`

## Examples

Each example demonstrates the three backend styles together:

- **Rust**: built-in framework plugins exposed through eqswift/UniFFI (`app`, `core`, `stats`), plus example-local Rust crates where the app needs domain logic
- **JavaScript**: JavaScriptCore plugins loaded from each example's `scripts/backend.js`
- **Swift**: app-local plugins for UI state and persistence

`examples/hyperchat` is the larger chat prototype. Its chat routing lives in `examples/hyperchat/rust-backend`, an eqswift-backed Rust crate, while Swift owns local chat state and JavaScript scores draft routing. It combines Bitchat-style nearby mesh, Matrix federation, `../stalwart-lite` self-hosted archive semantics, and direct P2P routing into one optimized local chat surface.

## New projects

`aurorality new <name>` scaffolds a SwiftPM package (`Package.swift`, `Sources/App.swift`, `views/main.crepus`, `.brisk.toml`) that depends on an aurorality checkout by absolute path — by default the checkout the CLI was built from, or `--aurorality <path>`. `cd <name> && aurorality run` builds the Rust core, generates bindings, and launches the app.

## iOS Simulator

`aurorality run --ios` builds `aurorality-core` for `aarch64-apple-ios-sim` (install it with `rustup target add aarch64-apple-ios-sim`), links the Swift build against that static library via `AURORALITY_RUST_LIB_DIR`, wraps the binary in an iOS `.app`, and installs and launches it with `simctl`. Any failed step exits non-zero.

The Rust library is looked up in `CARGO_TARGET_DIR` when that variable is set, otherwise in the checkout's `target/`. A relative `CARGO_TARGET_DIR` resolves against the project directory for both host and simulator builds. A `build.target-dir` set in `.cargo/config.toml` is **not** read; if you use one, also export `CARGO_TARGET_DIR` with the same path. Projects created before this change need their generated pre-build commands updated to use `CARGO_TARGET_DIR` for the host library.

## Hot Reload

`aurorality dev` builds and launches the app, then watches `views/` and `Sources/`. **By default it opens no port.** It launches the app with `AURORALITY_VIEWS_DIR` set to the project's `views/` directory, and the app re-renders its own templates when they are saved:

```swift
try state.load(template: AurorTemplates.source("main", bundle: .module), context: context)
state.watchLiveTemplate("main")   // no-op outside `aurorality dev` and in release builds
```

`AurorTemplates.source` reads the live file under `aurorality dev` (DEBUG builds) and the bundled copy otherwise. `watchLiveTemplate` re-renders on each save with the context the app last passed to `load`, so `{title}` and other variables still resolve. A broken save sets `state.error`, and the next good save clears it. Projects created by `aurorality new` and `examples/basic` are already wired up this way.

A template save needs no rebuild while such an app is running. Swift changes, and template saves for apps that don't call `watchLiveTemplate`, kill the app, rebuild it, and relaunch it.

**Limits:**
- **macOS and the iOS Simulator only.** The app has to read the template from the host filesystem. Real devices need `--serve`.
- **Each save re-renders the whole template.** There is no incremental patch.

### `dev --serve`: WebSocket server

`aurorality dev --serve` also serves hot reload on `ws://127.0.0.1:47832` (`--port`), and passes `AURORALITY_DEV_PORT` to the launched app. Use it for `HotReloadClient` consumers: the Runner preview app and `AurorDevOverlay`.

For a real device, run `aurorality dev --serve --host 0.0.0.0` and connect the client to the Mac's LAN address and the selected port. The server has no authentication or encryption and exposes template contents and project paths to clients. Use a trusted network and firewall, and stop the server when finished. The default remains loopback-only.

1. **`DevHello` on connect**: session id, watch dir, optional swiftgen paths. The current template follows as a `fullReload`.
2. **IR reload / patches**, unless `--no-ir`: each `.crepus` save sends an `IrMutation` patch (`replaceRoot`, `replaceNode`, …), or a full reload when the template's shape changed. **The server renders with an empty context**, so templates that use app context variables show them unresolved. In-process reload doesn't have this problem.
3. **`SwiftgenStatus`**: when `--swiftgen-view` / `--swiftgen-out` are set, each save of that view re-runs `swiftgen` and reports ok/errors plus the output path.

Browser connections (handshakes with an `Origin` header) are rejected. Swift hosts use **`HotReloadClient`** → **`HotReloadBus`**, and optionally **`AurorDevOverlay`** (`AURORALITY_DEV=1` or `.environment(\.aurorDevEnabled, true)`) for a corner HUD and a live IR toggle.

## Persistence (`aurorStore`)

`aurorality-core` exposes **`aurorStoreGet` / `aurorStoreSet` / `aurorStorePath`** (JSON blob under `~/Library/Application Support/<bundle_id>/aurorality-store.json`) for lightweight app state — used by examples like HyperChat for transcripts/settings snapshots.

## Dependencies

- `crepuscularity-native` — IR + template parsing
- `crepuscularity-core` — context + evaluation
- `eqswift` / `uniffi` — Rust ↔ Swift FFI

## License

ISC
