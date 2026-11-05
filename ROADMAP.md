# Aurorality — Full Framework Roadmap

## Product and reliability audit

The checklists below track fields and features, not full CSS compatibility. A
class being parsed into IR does not mean that SwiftUI renders every combination
with web-equivalent behavior. Priorities for making templates useful as native
applications:

1. **Make context loss visible.** `crates/aurorality-core/src/render.rs` rejects
   unsupported nested objects and non-object array elements with their paths.
   Investigate extending the upstream template value model to support them.
2. **Make IR controls interactive.** The IR renderer's input and picker use
   constant preview bindings. Design observable state and event bindings before
   presenting these as application controls; generated Swift is a separate path.
3. **Test layout semantics, not just decoded fields.** Resolve width and height
   independently, implement axis-aware flex and parent-aware positioning, and
   verify box styling on text and buttons with rendered SwiftUI tests. Current
   frames, positioning offsets, and stack behavior approximate web layout.
4. **Make hot reload recoverable.** Surface failed mutations, recover with a
   full reload, and clear errors on successful updates. Exercise the real Swift
   client and Rust producer together in macOS CI.
5. **Build an inspectable playground.** A development overlay could show each
   node's source classes, decoded IR, effective native styling, and warnings
   for unsupported or approximate behavior. Editable state and native controls
   would make this a creative prototyping tool rather than a static preview.

### Playground product plan

This section is a proposal, not a description of implemented playground
features. The first target is a macOS development experience using the existing
SwiftUI IR renderer and `aurorality dev`. Generated Swift is a separate render
path and is not covered by the first playground.

The playground pairs a native preview with a node tree, an inspector, and a
context editor. Selecting a preview node or tree entry selects the same node in
the inspector. The inspector shows source classes, decoded IR, and the SwiftUI
modifiers and values actually applied by the renderer. It also distinguishes
unsupported classes, values lost before rendering, and known approximations.
For example, flex growth currently expands width without parent-axis context,
and absolute positioning uses offsets rather than web positioning. Width and
height resolution and box styling on text and buttons also need verification.
A warning must name the affected node and property, explain the difference,
and point to the source when a location is available. Unknown behavior must
not be labeled web-equivalent.

Source classes, source locations, stable node identities, and effective native
styles are not established inspector data. Add development metadata through
the render and hot-reload paths rather than inferring source from the IR.
When metadata is unavailable, show that it is unavailable. Reconcile selection
after a patch or full reload using stable identity; if a node disappears, clear
the selection rather than selecting a different node by its old tree index.
Keep this metadata and the overlay out of production output.

The context editor starts with a JSON object and offers typed controls for
supported values. Applying an edit rerenders the current template through the
same Rust render path as `AurorState.load`. Invalid JSON or render errors leave
the last successful preview and edited text available for correction. Report
paths for values the context conversion rejects, including nested objects and
non-object array elements. Reset restores the example's initial context.
Context edits are local preview state, not writes to an example or backend.

Native inputs and pickers need observable bindings and explicit event routing
before the playground presents them as interactive. Today the IR renderer uses
constant bindings for both. Define which context key each control edits, how a
change rerenders the template, and how button events reach a preview handler.
Show unbound controls as preview-only, not as working application controls.
The playground must not invoke production plugins or persist changes without
an explicit development configuration.

Example workflows to support:

- Open `examples/counter`, change `count` in the context editor, and inspect
  the resulting text and layout. Once event routing exists, use the increment
  and decrement buttons and observe the context and preview update together.
- Open `examples/textanalyzer` with sample statistics and a `history` list of
  objects. Edit a value and inspect the repeated nodes. Enter an unsupported
  nested object or primitive array and see the rejected path before applying it.
- Select a flex child or an absolutely positioned node in a small fixture.
  Compare its source class, IR value, native modifier, and approximation warning.
  Edit the template and verify that selection and warnings reflect the new IR.

Deliver the playground in stages. Each stage has an observable acceptance gate:

1. **Preview and diagnostics.** Open a template and context in the native
   preview. Show render errors without losing the last successful preview.
   A macOS integration test covers an invalid edit followed by a valid edit.
   The dev connection shows disconnected and error states. A failed patch
   triggers a full reload, and a successful update clears the error.
2. **Inspector.** Select nodes in the preview and tree. For a fixture with flex
   growth and absolute positioning, verify source classes, IR, applied native
   values, and the matching warnings. Test selection across patch, full reload,
   and node deletion. Missing metadata is visibly marked unavailable.
3. **Editable context.** Change a scalar and a list of objects, then verify
   the rendered output changes. Invalid JSON preserves the preview. Unsupported
   values produce path-specific warnings, and reset restores the initial data.
4. **Interactive controls.** Bind a text input and picker to context and route
   a counter button to a development handler. Verify edits, events, rerenders,
   and reset in a real SwiftUI interaction test. Unbound controls are labeled.

The playground is a native preview, not a browser or a CSS conformance oracle.
Warnings cover known cases only and cannot prove equivalence. It does not
promise production state persistence, arbitrary plugin execution, generated
Swift parity, or support for context shapes the template engine cannot express.
The existing hot-reload client applies full and patch messages, but recovery
from failed mutations and error clearing are prerequisites, not assumptions.

**Requires hands-on Mac verification:** the native playground's rendered
layout, pointer and keyboard interactions, real-device networking, and the
iOS Simulator build and launch have not been verified in this Linux orb.
GitHub's macOS CI can compile and run automated tests but does not replace
inspection of those experiences on a Mac or device.

## Priority 1: CSS Property Coverage (Critical)

### Spacing ✓ (Complete)
- p-, px-, py-, pt-, pb-, pl-, pr-: done
- m-, mx-, my-, mt-, mb-, ml-, mr-: done

### Sizing ✓ (Complete)
- w-, h-, min-w-, max-w-, min-h-, max-h-: done
- widthFraction, heightFraction: done
- aspect-ratio: done
- size-: done

### Typography ✓ (Very Strong)
- text-size, font-weight, font-family: done
- text-align, leading, tracking: done
- text-transform: done
- italic, underline, line-through: done
- line-height: done

### Colors ✓ (Strong)
- foreground/background colors: done
- border-color: done

### Border ✓ (Complete)
- border-width, corner-radius: done

### Flex ✓ (Strong)
- flex-grow, flex_shrink: done
- flex-wrap: done
- align-self: done

### Visibility ✓ (Basic)
- opacity, hidden, overflow_hidden: done

---

## Priority 2: Missing High-Value CSS Properties

### Layout (flex_direction)
- [ ] Add flex_direction field: "row" | "column" 
- [ ] Add parser: flex-row, flex-col
- [ ] Add Swift mapping in layout modifier

### Position & Layering
- [ ] Add position: "static" | "relative" | "absolute" | "fixed"
- [ ] Add top, right, bottom, left (for absolute positioning)
- [ ] Add z_index: i32

### Transforms
- [ ] Add transform field with translate/scale/rotate
- [ ] Add parsers: translate-x-N, translate-y-N, scale-N, rotate-N
- [ ] Swift: Apply via .transformEffect()

### Box Shadow
- [x] Add shadow fields: shadow_color, shadow_radius, shadow_offset
- [x] Add parsers: shadow-sm, shadow-md, shadow-lg, shadow-xl

### Gradients
- [x] Add background gradient direction + from/to colors
- [x] Add parsers: bg-gradient-to-r, from-blue-500, to-red-500

### Text Overflow
- [x] Add text_overflow: "clip" | "ellipsis" | "truncate"
- [x] Add white_space: "normal" | "nowrap" | "pre"
- [x] Add line_clamp-N for truncation

### Cursor & Selection
- [x] Add cursor: "auto" | "default" | "pointer" | "text"
- [x] Add user_select: "auto" | "none" | "text" | "all"

---

## Priority 3: Missing ViewNode Types

### Input Controls
- [ ] Input (TextField) node
- [ ] Toggle / Switch node
- [ ] Checkbox node
- [ ] Radio node
- [ ] Slider node

### Display Elements
- [ ] Progress (determinate/indeterminate)
- [ ] Spinner
- [ ] Divider
- [ ] Badge / Chip

### Navigation & Modal
- [ ] NavigationLink
- [ ] Sheet / Modal
- [ ] TabView
- [ ] Drawer

### Rich Media
- [ ] Video (AVPlayer)
- [ ] WebView
- [ ] SVG

---

## Priority 4: ViewNode Enhancements

### Stack
- [ ] Gap parsing: gap-2, gap-4, gap-N
- [ ] justifyContent full mapping

### Button
- [ ] onClick handler with payload
- [ ] Button variants: filled, outlined, text

### Image
- [x] object_fit / object_position
- [x] placeholder for loading

### Scroll
- [ ] always bounce option
- [ ] indicator style (auto, always, never)

---

## Priority 5: Framework Infrastructure

### Dev Experience
- [ ] Error boundary with template validation
- [ ] VSCode extension for .crepus

### Testing  
- [ ] Rust tests for IR generation
- [ ] Swift snapshot tests

### Build
- [ ] XcodeGen full integration
- [ ] SPM package support

### Documentation
- [ ] Style class reference table
- [ ] Example gallery

---

## Completed ✓

- Padding/margin (14 props)
- Sizing (w/h, min/max, aspect)
- Typography (size, weight, spacing, transform)
- Colors (fg, bg, border)
- Borders (radius, width)
- Visibility (opacity, hidden, overflow)
- Flex (grow, shrink, wrap, align-self)
- Text decorations (italic, underline, strike)

- ViewNodes: text, stack, button, image, scroll, slotRotate
- Hot reload (full + patch)
- Plugin system (core, app, stats)
- Dev server with WebSocket
