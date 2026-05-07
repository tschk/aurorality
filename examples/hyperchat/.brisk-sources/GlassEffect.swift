import SwiftUI

#if canImport(AppKit)
import AppKit

/// Glass effect for top functional surfaces.
///
/// macOS 26+ uses native Liquid Glass. macOS 14/15 falls back to
/// `NSVisualEffectView` with `.hudWindow` material.
///
/// Apple guidance: Liquid Glass is for functional/navigation layers,
/// not content cards. Use `.regularMaterial` for message cards.
struct GlassEffectModifier: ViewModifier {
    func body(content: Content) -> some View {
        if #available(macOS 26, *) {
            content
                .glassEffect(in: RoundedRectangle(cornerRadius: 12))
        } else {
            content
                .background(GlassFallback())
        }
    }
}

struct GlassFallback: NSViewRepresentable {
    func makeNSView(context: Context) -> NSVisualEffectView {
        let view = NSVisualEffectView()
        view.material = .hudWindow
        view.blendingMode = .behindWindow
        view.state = .active
        view.isEmphasized = true
        return view
    }

    func updateNSView(_ nsView: NSVisualEffectView, context: Context) {}
}

public extension View {
    /// Apply Liquid Glass (macOS 26) or frosted material fallback (macOS 14/15).
    ///
    /// Use for functional surfaces: toolbar, compose bar, sidebar headers.
    /// Do NOT use for message content cards — use `.background(.regularMaterial)` instead.
    func glassEffect() -> some View {
        modifier(GlassEffectModifier())
    }
}

#endif
