/// Swift mirror of the Rust `crepuscularity_native` IR types.
/// All types are `Codable` so they round-trip through the JSON returned by `renderTemplate`.

import Foundation

// MARK: - ViewIr

public struct ViewIr: Codable, Equatable {
    public let version: Int
    public let root: [ViewNode]

    public static let empty = ViewIr(version: 2, root: [])
}

// MARK: - ViewNode

/// Flat Codable struct matching the serde `#[serde(tag = "kind")]` enum.
public struct ViewNode: Codable, Equatable {
    public enum Kind: String, Codable {
        case text, stack, button, image, scroll, slotRotate
    }

    public let kind: Kind

    // text
    public var content: String?

    // stack / scroll
    public var axis: String?
    public var spacing: Float?
    public var alignItems: String?
    public var justifyContent: String?
    public var children: [ViewNode]?

    // button
    public var label: String?
    public var onClick: String?

    // image
    public var src: String?
    public var alt: String?

    // slotRotate
    public var phrases: [String]?
    public var intervalMs: UInt64?

    // shared
    public var style: ViewStyle?
}

// MARK: - ViewStyle

public struct ViewStyle: Codable, Equatable {
    // ── Spacing ────────────────────────────────────────────
    public var padding: Float?
    public var paddingHorizontal: Float?
    public var paddingVertical: Float?
    public var paddingTop: Float?
    public var paddingBottom: Float?
    public var paddingLeft: Float?
    public var paddingRight: Float?
    public var margin: Float?
    public var marginHorizontal: Float?
    public var marginVertical: Float?
    public var marginTop: Float?
    public var marginBottom: Float?
    public var marginLeft: Float?
    public var marginRight: Float?

    // ── Sizing ─────────────────────────────────────────────
    // Sentinels: -1.0 = fill parent (.infinity), -2.0 = fit content (.fixedSize)
    public var width: Float?
    public var height: Float?
    public var minWidth: Float?
    public var maxWidth: Float?
    public var minHeight: Float?
    public var maxHeight: Float?
    public var widthFraction: Float?   // 0.0–1.0 relative to parent
    public var heightFraction: Float?
    public var aspectRatio: Float?

    // ── Typography ─────────────────────────────────────────
    public var fontSize: Float?
    public var fontWeight: UInt16?     // CSS 100–900
    public var fontFamily: String?     // "mono" | "serif" | "sans" (default)
    public var textAlign: String?      // "left" | "center" | "right"
    public var lineHeight: Float?      // multiplier, e.g. 1.5
    public var letterSpacing: Float?   // points
    public var textTransform: String?  // "uppercase" | "lowercase" | "capitalize"
    public var foregroundColor: String?
    public var backgroundColor: String?
    public var cornerRadius: Float?
    public var italic: Bool?
    public var underline: Bool?
    public var strikethrough: Bool?

    // ── Border ─────────────────────────────────────────────
    public var borderWidth: Float?
    public var borderColor: String?

    // ── Visibility & Opacity ───────────────────────────────
    public var opacity: Float?         // 0.0–1.0
    public var hidden: Bool?
    public var overflowHidden: Bool?

    // ── Flex / Layout ──────────────────────────────────────
    public var flexDirection: String?  // "row" | "column"
    public var flexGrow: Float?        // ≥ 1.0 → expand along primary axis
    public var flexShrink: Float?
    public var flexWrap: String?       // "wrap" | "nowrap"
    public var alignSelf: String?      // "start" | "end" | "center" | "stretch"

    // ── Position & Layering ───────────────────────────────
    public var position: String?       // "absolute" | "relative" | "fixed"
    public var zIndex: Int?
    public var top: Float?
    public var right: Float?
    public var bottom: Float?
    public var left: Float?

    // ── Transform ─────────────────────────────────────────
    public var translateX: Float?
    public var translateY: Float?
    public var scaleX: Float?
    public var scaleY: Float?
    public var rotate: Float?          // degrees

    // ── Shadow ────────────────────────────────────────────
    public var shadowColor: String?
    public var shadowRadius: Float?
    public var shadowOffsetX: Float?
    public var shadowOffsetY: Float?

    // ── Text Layout ───────────────────────────────────────
    public var textOverflow: String?   // "clip" | "ellipsis" | "truncate"
    public var whiteSpace: String?     // "normal" | "nowrap" | "pre" | "pre-wrap"
    public var lineClamp: Int?
    public var cursor: String?         // macOS only; ignored on iOS
    public var userSelect: String?     // not supported in pure SwiftUI; ignored
}

// MARK: - Hot-reload protocol

public struct HotReloadEnvelope: Codable {
    public let sequence: UInt64
    public let message: HotReloadMessage
}

public struct HotReloadMessage: Codable {
    public enum Kind: String, Codable {
        case noop, patch, fullReload, error
    }
    public let kind: Kind
    public var mutations: [IrMutation]?
    public var ir: ViewIr?
    public var reason: String?
    public var message: String?
}

// MARK: - IrMutation

public struct IrMutation: Codable {
    public enum Op: String, Codable {
        case replaceRoot, replaceNode, insertNode, removeNode, updateText, updateStyle
    }
    public let op: Op
    public var root: [ViewNode]?
    public var path: [Int]?
    public var node: ViewNode?
    public var parentPath: [Int]?
    public var index: Int?
    public var content: String?
    public var style: ViewStyle?  // nil = remove style
}

// MARK: - Mutation application

extension ViewIr {
    /// Apply a patch sequence in-place via the Rust mutation engine.
    /// Falls back to `self` on encoding errors (should not happen in practice).
    public func applying(_ mutations: [IrMutation]) -> ViewIr {
        guard !mutations.isEmpty else { return self }
        guard
            let irData  = try? JSONEncoder().encode(self),
            let irJson  = String(data: irData, encoding: .utf8),
            let mutData = try? JSONEncoder().encode(mutations),
            let mutJson = String(data: mutData, encoding: .utf8),
            let result  = try? applyMutations(irJson: irJson, mutationsJson: mutJson),
            let newIr   = try? JSONDecoder().decode(ViewIr.self, from: Data(result.utf8))
        else { return self }
        return newIr
    }
}
