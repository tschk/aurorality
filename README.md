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

## Quick Start

```bash
# Build the Rust core with JavaScript plugin support
cargo build -p aurorality-core --features js

# Generate eqswift/UniFFI Swift bindings
cargo run -p aurorality-core --features js --bin uniffi-bindgen generate \
  --library target/debug/libaurorality_core.dylib \
  --language swift --out-dir generated
cp generated/aurorality_coreFFI.modulemap generated/module.modulemap

# Run dev server
cargo run -p aurorality-cli -- dev examples/basic

# Or open in Xcode
open examples/basic/Aurorality.xcodeproj
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

## Hot Reload

`aurorality dev` watches `.crepus` files and pushes updates over WebSocket:

1. **`DevHello` on connect** — session id, roots, optional swiftgen paths.
2. **IR reload / patches** — unless `--no-ir`: full IR plus incremental `IrMutation` ops (`replaceRoot`, `replaceNode`, …).
3. **`SwiftgenStatus`** — when `--swiftgen-view` / `--swiftgen-out` are set, each save re-runs `swiftgen` and reports ok/errors + output path.

Swift hosts use **`HotReloadClient`** → **`HotReloadBus`** and optional **`AurorDevOverlay`** (`AURORALITY_DEV=1` or `.environment(\.aurorDevEnabled, true)`) for a corner HUD + live IR toggle.

## Persistence (`aurorStore`)

`aurorality-core` exposes **`aurorStoreGet` / `aurorStoreSet` / `aurorStorePath`** (JSON blob under `~/Library/Application Support/<bundle_id>/aurorality-store.json`) for lightweight app state — used by examples like HyperChat for transcripts/settings snapshots.

## Dependencies

- `crepuscularity-native` — IR + template parsing
- `crepuscularity-core` — context + evaluation
- `eqswift` / `uniffi` — Rust ↔ Swift FFI

## License

MPL-2.0
