# Aurorality — Full Framework Roadmap

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
