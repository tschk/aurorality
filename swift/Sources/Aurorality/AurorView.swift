/// Recursive SwiftUI renderer for ViewIr / ViewNode trees.

import SwiftUI

// MARK: - Root view

/// Top-level view — renders the root nodes from AurorState.
public struct AurorRootView: View {
    public let state: AurorState
    @Environment(AurorBridge.self) var bridge

    public init(state: AurorState) {
        self.state = state
    }

    public var body: some View {
        if let errorMsg = state.error {
            Text("⚠ \(errorMsg)")
                .foregroundStyle(.red)
                .padding()
        } else if state.ir.root.isEmpty {
            ProgressView()
        } else {
            ForEach(Array(state.ir.root.enumerated()), id: \.offset) { _, node in
                AurorNodeView(node: node)
            }
        }
    }
}

// MARK: - Node renderer

struct AurorNodeView: View {
    let node: ViewNode
    @Environment(AurorBridge.self) var bridge

    var body: some View {
        switch node.kind {
        case .text:       textView
        case .stack:      stackView
        case .button:     buttonView
        case .image:      imageView
        case .scroll:     scrollView
        case .slotRotate: slotRotateView
        }
    }

    // MARK: text

    @ViewBuilder
    private var textView: some View {
        Text(transformedContent)
            .auroraTextStyle(node.style)
            .auroraLayout(node.style)
    }

    private var transformedContent: String {
        let raw = node.content ?? ""
        switch node.style?.textTransform {
        case "uppercase":  return raw.uppercased()
        case "lowercase":  return raw.lowercased()
        case "capitalize": return raw.capitalized
        default:           return raw
        }
    }

    // MARK: stack

    @ViewBuilder
    private var stackView: some View {
        let children = node.children ?? []
        let spacing = node.spacing.map(CGFloat.init)

        if node.axis == "row" {
            HStack(alignment: hAlignment, spacing: spacing) {
                childViews(children)
            }
            .auroraContainerStyle(node.style)
            .auroraLayout(node.style)
        } else {
            VStack(alignment: vAlignment, spacing: spacing) {
                childViews(children)
            }
            .auroraContainerStyle(node.style)
            .auroraLayout(node.style)
        }
    }

    private var hAlignment: VerticalAlignment {
        switch node.alignItems {
        case "start":    return .top
        case "end":      return .bottom
        case "center":   return .center
        case "baseline": return .firstTextBaseline
        default:         return .center
        }
    }

    private var vAlignment: HorizontalAlignment {
        switch node.alignItems {
        case "start":  return .leading
        case "end":    return .trailing
        case "center": return .center
        default:       return .center
        }
    }

    // MARK: button

    @ViewBuilder
    private var buttonView: some View {
        Button(node.label ?? "") {
            if let handler = node.onClick {
                try? bridge.invoke(pluginId: "core", method: "echo", payload: "{\"event\":\"\(handler)\"}")
            }
        }
        .auroraTextStyle(node.style)
        .auroraLayout(node.style)
    }

    // MARK: image

    @ViewBuilder
    private var imageView: some View {
        if let src = node.src {
            if src.hasPrefix("http://") || src.hasPrefix("https://") {
                AsyncImage(url: URL(string: src)) { image in
                    image.resizable().scaledToFit()
                } placeholder: {
                    ProgressView()
                }
                .auroraContainerStyle(node.style)
                .auroraLayout(node.style)
            } else {
                Image(src)
                    .resizable()
                    .scaledToFit()
                    .auroraContainerStyle(node.style)
                    .auroraLayout(node.style)
            }
        }
    }

    // MARK: scroll

    @ViewBuilder
    private var scrollView: some View {
        let children = node.children ?? []
        let axes: Axis.Set = node.axis == "row" ? .horizontal : .vertical
        ScrollView(axes) {
            if axes == .horizontal {
                HStack { childViews(children) }
            } else {
                VStack { childViews(children) }
            }
        }
        .auroraContainerStyle(node.style)
        .auroraLayout(node.style)
    }

    // MARK: slotRotate

    @ViewBuilder
    private var slotRotateView: some View {
        if let phrases = node.phrases, !phrases.isEmpty {
            TimedTextView(phrases: phrases, intervalMs: node.intervalMs ?? 2000)
                .auroraTextStyle(node.style)
                .auroraLayout(node.style)
        }
    }

    // MARK: helpers

    @ViewBuilder
    private func childViews(_ children: [ViewNode]) -> some View {
        ForEach(Array(children.enumerated()), id: \.offset) { _, child in
            AurorNodeView(node: child)
        }
    }
}

// MARK: - TimedTextView

struct TimedTextView: View {
    let phrases: [String]
    let intervalMs: UInt64

    @State private var index = 0

    var body: some View {
        Text(phrases[index])
            .onAppear { startTimer() }
    }

    private func startTimer() {
        let interval = Double(intervalMs) / 1000.0
        Timer.scheduledTimer(withTimeInterval: interval, repeats: true) { _ in
            index = (index + 1) % phrases.count
        }
    }
}

// MARK: - Style modifier extensions

extension View {
    /// Typography + foreground color (for text nodes).
    func auroraTextStyle(_ style: ViewStyle?) -> some View {
        modifier(AurorTextStyleModifier(style: style))
    }

    /// Background, border, clip (for container nodes).
    func auroraContainerStyle(_ style: ViewStyle?) -> some View {
        modifier(AurorContainerStyleModifier(style: style))
    }

    /// Layout: frame, padding, margin, opacity, visibility, flex, aspect ratio.
    /// Applied last — wraps the fully-styled view.
    func auroraLayout(_ style: ViewStyle?) -> some View {
        modifier(AurorLayoutModifier(style: style))
    }
}

// MARK: - Text style modifier

struct AurorTextStyleModifier: ViewModifier {
    let style: ViewStyle?

    func body(content: Content) -> some View {
        content
            .font(style?.swiftFont)
            .foregroundStyle(style?.swiftForegroundColor ?? Color.primary)
            .italic(style?.italic == true)
            .underline(style?.underline == true)
            .strikethrough(style?.strikethrough == true)
            .multilineTextAlignment(style?.swiftTextAlignment ?? .leading)
            .lineSpacing(style?.swiftLineSpacing ?? 0)
            .kerning(style?.letterSpacing.map(CGFloat.init) ?? 0)
    }
}

// MARK: - Container style modifier

struct AurorContainerStyleModifier: ViewModifier {
    let style: ViewStyle?

    func body(content: Content) -> some View {
        let cornerR = CGFloat(style?.cornerRadius ?? 0)
        let bg = style?.swiftBackgroundColor ?? .clear
        let borderW = CGFloat(style?.borderWidth ?? 0)
        let borderC = style?.borderColor.flatMap(Color.init(cssString:)) ?? .clear

        content
            .padding(style?.swiftEdgeInsets ?? .init())
            .background(bg)
            .clipShape(RoundedRectangle(cornerRadius: cornerR))
            .overlay(
                borderW > 0
                    ? AnyView(RoundedRectangle(cornerRadius: cornerR)
                        .stroke(borderC, lineWidth: borderW))
                    : AnyView(EmptyView())
            )
            .clipped(antialiased: false)
            .opacity(style?.overflowHidden == true ? 1 : 1) // clipped above handles clip
            // note: .clipped() called in AurorLayoutModifier when overflowHidden
    }
}

// MARK: - Layout modifier

struct AurorLayoutModifier: ViewModifier {
    let style: ViewStyle?

    func body(content: Content) -> some View {
        content
            .auroraFrame(style)
            .auroraAspectRatio(style)
            .auroraMargin(style)
            .opacity(style?.opacity.map(Double.init) ?? 1.0)
            .opacity(style?.hidden == true ? 0 : 1)
            .allowsHitTesting(style?.hidden != true)
            .auroraOverflowClip(style)
            .auroraFlexGrow(style)
            .auroraAlignSelf(style)
    }
}

// MARK: - Frame helpers

private extension View {
    /// `.frame()` from width/height/min/max sizing fields.
    @ViewBuilder
    func auroraFrame(_ style: ViewStyle?) -> some View {
        let minW  = style?.minWidth.map(CGFloat.init)
        let maxW  = resolvedMaxDim(style?.maxWidth, fill: style?.width == -1.0 || style?.maxWidth == -1.0)
        let minH  = style?.minHeight.map(CGFloat.init)
        let maxH  = resolvedMaxDim(style?.maxHeight, fill: style?.height == -1.0 || style?.maxHeight == -1.0)
        let absW  = absValue(style?.width)
        let absH  = absValue(style?.height)
        let fitW  = style?.width == -2.0
        let fitH  = style?.height == -2.0

        if fitW || fitH {
            self.fixedSize(horizontal: fitW, vertical: fitH)
        } else if absW != nil || absH != nil || minW != nil || maxW != nil || minH != nil || maxH != nil {
            self.frame(
                minWidth: minW, idealWidth: absW, maxWidth: maxW,
                minHeight: minH, idealHeight: absH, maxHeight: maxH,
                alignment: .topLeading
            )
        } else if let wf = style?.widthFraction {
            GeometryReader { geo in
                self.frame(width: geo.size.width * CGFloat(wf),
                           height: style?.heightFraction.map { geo.size.height * CGFloat($0) })
            }
        } else if let hf = style?.heightFraction {
            GeometryReader { geo in
                self.frame(height: geo.size.height * CGFloat(hf))
            }
        } else {
            self
        }
    }

    @ViewBuilder
    func auroraAspectRatio(_ style: ViewStyle?) -> some View {
        if let ar = style?.aspectRatio {
            self.aspectRatio(CGFloat(ar), contentMode: .fit)
        } else {
            self
        }
    }

    @ViewBuilder
    func auroraMargin(_ style: ViewStyle?) -> some View {
        let top    = CGFloat(style?.marginTop    ?? style?.marginVertical   ?? style?.margin ?? 0)
        let bottom = CGFloat(style?.marginBottom ?? style?.marginVertical   ?? style?.margin ?? 0)
        let lead   = CGFloat(style?.marginLeft   ?? style?.marginHorizontal ?? style?.margin ?? 0)
        let trail  = CGFloat(style?.marginRight  ?? style?.marginHorizontal ?? style?.margin ?? 0)
        if top != 0 || bottom != 0 || lead != 0 || trail != 0 {
            self.padding(EdgeInsets(top: top, leading: lead, bottom: bottom, trailing: trail))
        } else {
            self
        }
    }

    @ViewBuilder
    func auroraOverflowClip(_ style: ViewStyle?) -> some View {
        if style?.overflowHidden == true {
            self.clipped()
        } else {
            self
        }
    }

    /// Simulate flex-grow: expand along primary axis (maxWidth; best-effort without axis context).
    @ViewBuilder
    func auroraFlexGrow(_ style: ViewStyle?) -> some View {
        if let grow = style?.flexGrow, grow >= 1.0 {
            self.frame(maxWidth: .infinity)
        } else {
            self
        }
    }

    /// align-self via frame alignment.
    @ViewBuilder
    func auroraAlignSelf(_ style: ViewStyle?) -> some View {
        switch style?.alignSelf {
        case "start":   self.frame(maxWidth: .infinity, alignment: .leading)
        case "end":     self.frame(maxWidth: .infinity, alignment: .trailing)
        case "center":  self.frame(maxWidth: .infinity, alignment: .center)
        case "stretch": self.frame(maxWidth: .infinity)
        default:        self
        }
    }
}

private func absValue(_ v: Float?) -> CGFloat? {
    guard let v, v > 0 else { return nil }
    return CGFloat(v)
}

private func resolvedMaxDim(_ v: Float?, fill: Bool) -> CGFloat? {
    if fill { return .infinity }
    guard let v, v > 0 else { return nil }
    return CGFloat(v)
}

// MARK: - ViewStyle → SwiftUI computed properties

extension ViewStyle {
    var swiftFont: Font? {
        var font: Font
        if let size = fontSize {
            font = .system(size: CGFloat(size))
        } else {
            font = .body
        }
        if let weight = fontWeight {
            font = font.weight(fontWeightValue(weight))
        }
        if let family = fontFamily {
            switch family {
            case "mono":  font = font.monospaced()
            case "serif": font = font.serif()
            default: break  // "sans" = system default
            }
        }
        return font
    }

    var swiftForegroundColor: Color? {
        foregroundColor.flatMap(Color.init(cssString:))
    }

    var swiftBackgroundColor: Color? {
        backgroundColor.flatMap(Color.init(cssString:))
    }

    var swiftTextAlignment: TextAlignment {
        switch textAlign {
        case "center":            return .center
        case "right", "trailing": return .trailing
        default:                  return .leading
        }
    }

    var swiftEdgeInsets: EdgeInsets {
        EdgeInsets(
            top:      CGFloat(paddingTop    ?? paddingVertical   ?? padding ?? 0),
            leading:  CGFloat(paddingLeft   ?? paddingHorizontal ?? padding ?? 0),
            bottom:   CGFloat(paddingBottom ?? paddingVertical   ?? padding ?? 0),
            trailing: CGFloat(paddingRight  ?? paddingHorizontal ?? padding ?? 0)
        )
    }

    /// Line spacing in points from a multiplier. Uses system body line height (~20pt) as base.
    var swiftLineSpacing: CGFloat? {
        guard let lh = lineHeight else { return nil }
        let baseLineHeight: CGFloat = 20
        return CGFloat(lh) * baseLineHeight - baseLineHeight
    }

    private func fontWeightValue(_ w: UInt16) -> Font.Weight {
        switch w {
        case ..<300:    return .ultraLight
        case 300..<400: return .light
        case 400..<500: return .regular
        case 500..<600: return .medium
        case 600..<700: return .semibold
        case 700..<800: return .bold
        case 800..<900: return .heavy
        default:        return .black
        }
    }
}

// MARK: - Color from CSS string

extension Color {
    /// Tailwind hex strings (#rrggbb or #rrggbbaa) + basic named colors.
    init?(cssString: String) {
        switch cssString.lowercased() {
        case "red":               self = .red
        case "blue":              self = .blue
        case "green":             self = .green
        case "white":             self = .white
        case "black":             self = .black
        case "gray", "grey":      self = .gray
        case "clear", "transparent": self = .clear
        case "orange":            self = .orange
        case "yellow":            self = .yellow
        case "purple":            self = .purple
        case "pink":              self = .pink
        case "primary":           self = .primary
        case "secondary":         self = .secondary
        default:
            if cssString.hasPrefix("#") {
                self.init(hex: cssString)
            } else {
                return nil
            }
        }
    }

    init(hex: String) {
        let hex = hex.trimmingCharacters(in: .init(charactersIn: "#"))
        let r, g, b, a: Double
        if hex.count == 6 {
            let n = UInt64(hex, radix: 16) ?? 0
            r = Double((n >> 16) & 0xFF) / 255
            g = Double((n >> 8)  & 0xFF) / 255
            b = Double(n         & 0xFF) / 255
            a = 1
        } else if hex.count == 8 {
            let n = UInt64(hex, radix: 16) ?? 0
            r = Double((n >> 24) & 0xFF) / 255
            g = Double((n >> 16) & 0xFF) / 255
            b = Double((n >> 8)  & 0xFF) / 255
            a = Double(n         & 0xFF) / 255
        } else {
            r = 0; g = 0; b = 0; a = 1
        }
        self.init(red: r, green: g, blue: b, opacity: a)
    }
}
