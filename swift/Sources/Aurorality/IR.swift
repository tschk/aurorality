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
    public var fontSize: Float?
    public var fontWeight: UInt16?
    public var textAlign: String?
    public var foregroundColor: String?
    public var backgroundColor: String?
    public var cornerRadius: Float?
    public var italic: Bool?
    public var underline: Bool?
    public var strikethrough: Bool?
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
    /// Apply a patch in-place. Returns new IR or self on unknown ops.
    public func applying(_ mutations: [IrMutation]) -> ViewIr {
        var copy = self
        for m in mutations {
            copy = copy.applying(m)
        }
        return copy
    }

    private func applying(_ m: IrMutation) -> ViewIr {
        switch m.op {
        case .replaceRoot:
            return ViewIr(version: version, root: m.root ?? [])
        case .replaceNode:
            guard let path = m.path, let node = m.node else { return self }
            var newRoot = root
            replaceNode(in: &newRoot, at: path, with: node)
            return ViewIr(version: version, root: newRoot)
        case .insertNode:
            guard let parentPath = m.parentPath, let idx = m.index, let node = m.node else { return self }
            var newRoot = root
            insertNode(in: &newRoot, parentPath: parentPath, at: idx, node: node)
            return ViewIr(version: version, root: newRoot)
        case .removeNode:
            guard let path = m.path, !path.isEmpty else { return self }
            var newRoot = root
            removeNode(in: &newRoot, at: path)
            return ViewIr(version: version, root: newRoot)
        case .updateText:
            guard let path = m.path, let content = m.content else { return self }
            var newRoot = root
            updateText(in: &newRoot, at: path, content: content)
            return ViewIr(version: version, root: newRoot)
        case .updateStyle:
            guard let path = m.path else { return self }
            var newRoot = root
            updateStyle(in: &newRoot, at: path, style: m.style)
            return ViewIr(version: version, root: newRoot)
        }
    }
}

// MARK: - Path-based tree helpers

private func nodeAt(_ nodes: [ViewNode], path: [Int]) -> ViewNode? {
    guard let first = path.first else { return nil }
    guard first < nodes.count else { return nil }
    let node = nodes[first]
    if path.count == 1 { return node }
    return nodeAt(node.children ?? [], path: Array(path.dropFirst()))
}

private func replaceNode(in nodes: inout [ViewNode], at path: [Int], with replacement: ViewNode) {
    guard let first = path.first, first < nodes.count else { return }
    if path.count == 1 {
        nodes[first] = replacement
    } else {
        var child = nodes[first]
        var children = child.children ?? []
        replaceNode(in: &children, at: Array(path.dropFirst()), with: replacement)
        child.children = children
        nodes[first] = child
    }
}

private func insertNode(in nodes: inout [ViewNode], parentPath: [Int], at idx: Int, node: ViewNode) {
    if parentPath.isEmpty {
        let safeIdx = min(idx, nodes.count)
        nodes.insert(node, at: safeIdx)
        return
    }
    guard let first = parentPath.first, first < nodes.count else { return }
    var parent = nodes[first]
    var children = parent.children ?? []
    insertNode(in: &children, parentPath: Array(parentPath.dropFirst()), at: idx, node: node)
    parent.children = children
    nodes[first] = parent
}

private func removeNode(in nodes: inout [ViewNode], at path: [Int]) {
    guard let first = path.first, first < nodes.count else { return }
    if path.count == 1 {
        nodes.remove(at: first)
        return
    }
    var parent = nodes[first]
    var children = parent.children ?? []
    removeNode(in: &children, at: Array(path.dropFirst()))
    parent.children = children
    nodes[first] = parent
}

private func updateText(in nodes: inout [ViewNode], at path: [Int], content: String) {
    guard let first = path.first, first < nodes.count else { return }
    if path.count == 1 {
        nodes[first].content = content
        return
    }
    var parent = nodes[first]
    var children = parent.children ?? []
    updateText(in: &children, at: Array(path.dropFirst()), content: content)
    parent.children = children
    nodes[first] = parent
}

private func updateStyle(in nodes: inout [ViewNode], at path: [Int], style: ViewStyle?) {
    guard let first = path.first, first < nodes.count else { return }
    if path.count == 1 {
        nodes[first].style = style
        return
    }
    var parent = nodes[first]
    var children = parent.children ?? []
    updateStyle(in: &children, at: Array(path.dropFirst()), style: style)
    parent.children = children
    nodes[first] = parent
}
