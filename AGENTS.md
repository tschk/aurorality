# AGENTS.md — Aurorality

Context for AI agents working on this codebase.

## Project

Aurorality — SwiftUI + Rust shell for `.crepus` templates. Write Tailwind-inspired templates, render to SwiftUI views.

## Workspace Layout

```
aurorality/
├── Cargo.toml                 # Workspace root
├── crates/
│   ├── aurorality-core/       # UniFFI bridge, render API, plugins
│   │   └── src/
│   │       ├── lib.rs        # UniFFI exports: renderTemplate, pluginInvoke
│   │       ├── render.rs    # Wrapper around crepuscularity_native
│   │       ├── bridge.rs   # Plugin registry
│   │       └── plugins.rs # Built-in plugins (core, app, stats)
│   ├── aurorality-cli/      # Dev server, build commands
│   │   └── src/
│   │       ├── main.rs    # CLI entry
│   │       ├── dev_server.rs
│   │       └── scaffold.rs
│   └── aurorality-macros/
├── swift/Sources/Aurorality/
│   ├── IR.swift             # ViewIr, ViewNode, ViewStyle (Codable mirrors)
│   ├── AurorView.swift     # SwiftUI renderer for IR
│   ├── AurorState.swift     # @Observable state container
│   ├── AurorBridge.swift    # Swift plugin invoke → Rust
│   ├── AurorApp.swift       # App entry point
│   └── HotReloadClient.swift
└── examples/
    ├── basic/
    ├── counter/
    └── textanalyzer/
```

## Key Dependencies

- `crepuscularity-native` — IR types + template parsing (external)
- `crepuscularity-core` — context + evaluation (external)
- `uniffi` — FFI scaffolding

## IR Schema (Source of Truth)

The canonical IR lives in `crepuscularity-native/src/ir.rs`:

- `ViewIr` — `{ version: 3, root: [ViewNode] }`
- `ViewNode` — tagged enum: `text`, `stack`, `button`, `image`, `scroll`, `slotRotate`
- `ViewStyle` — 40+ CSS properties (padding, margin, sizing, typography, color, border, flex)

Swift `IR.swift` mirrors this with Codable.

## Style Parser

`crepuscularity-native/src/style.rs` parses Tailwind classes → ViewStyle.

Coverage targets Tailwind v3. Skipped: breakpoints, pseudo-classes, gradients, shadows, grid, animations.

## Renderer

`swift/Sources/Aurorality/AurorView.swift` — recursive SwiftUI renderer.

- `AurorRootView` — renders root nodes
- `AurorNodeView` — dispatches on `node.kind`
- Style modifiers: `AurorTextStyleModifier`, `AurorContainerStyleModifier`, `AurorLayoutModifier`

## How to Add New ViewNode Types

1. **Rust**: Add variant to `ViewNode` enum in `crepuscularity-native/src/ir.rs`
2. **Parser**: Add parsing in `crepuscularity-native/src/render.rs`
3. **Swift IR**: Add case to `ViewNode.Kind` in `swift/Sources/Aurorality/IR.swift`
4. **Renderer**: Add view builder in `AurorNodeView` (AurorView.swift)

## How to Add New Style Properties

1. **Rust IR**: Add field to `ViewStyle` in crepuscularity-native
2. **Parser**: Add class handler in `crepuscularity-native/src/style.rs`
3. **Swift IR**: Add field to `ViewStyle` in `swift/Sources/Aurorality/IR.swift`
4. **Renderer**: Add computed property in `ViewStyle` extension + use in modifiers

## Commands

```bash
# Build
cargo build -p aurorality-core

# Test
cargo test -p aurorality-core

# Dev server
cargo run -p aurorality-cli -- dev examples/basic
```

## Current Coverage Gaps

See ViewStyle in ir.rs for all implemented properties.

Missing common CSS properties need implementation:
- flex-direction (not axis)
- position (static/relative/absolute)
- z-index
- transform (translate, scale, rotate)
- transition
- animation
- box-shadow
- gradient backgrounds
- object-fit / object-position
- text-overflow, white-space
- cursor
- user-select

<claude-mem-context>
# Memory Context

# claude-mem status

This project has no memory yet. The current session will seed it; subsequent sessions will receive auto-injected context for relevant past work.

Memory injection starts on your second session in a project.

`/learn-codebase` is available if the user wants to front-load the entire repo into memory in a single pass (~5 minutes on a typical repo, optional). Otherwise memory builds passively as work happens.

Live activity: http://localhost:37701
How it works: `/how-it-works`

This message disappears once the first observation lands.
</claude-mem-context>
