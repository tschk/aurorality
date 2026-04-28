/// Plugin protocol — implement in Swift or wrap a Rust plugin via `RustPlugin`.

import Foundation

// MARK: - Protocol

public protocol AurorPlugin: AnyObject {
    var id: String { get }
    func invoke(method: String, payload: String) throws -> String
}

// MARK: - Errors

public struct AurorPluginError: Error, LocalizedError {
    public let message: String
    public var errorDescription: String? { message }

    public init(_ message: String) {
        self.message = message
    }
}

// MARK: - RustPlugin adapter

/// Wraps a Rust built-in plugin (CorePlugin, AppPlugin, etc.) by delegating
/// to the UniFFI `pluginInvoke` function.
public final class RustPlugin: AurorPlugin {
    public let id: String

    public init(id: String) {
        self.id = id
    }

    public func invoke(method: String, payload: String) throws -> String {
        do {
            return try pluginInvoke(pluginId: id, method: method, payloadJson: payload)
        } catch let e as AurorError {
            throw AurorPluginError(e.localizedDescription)
        }
    }
}

// MARK: - ContextValue

/// Typed values for building template context from Swift code.
public enum ContextValue {
    case string(String)
    case int(Int)
    case float(Double)
    case bool(Bool)
    case list([[String: ContextValue]])
    case null
}

extension ContextValue {
    func toJson() -> Any {
        switch self {
        case .string(let s): return s
        case .int(let i): return i
        case .float(let f): return f
        case .bool(let b): return b
        case .null: return NSNull()
        case .list(let items):
            return items.map { dict in
                dict.mapValues { $0.toJson() }
            }
        }
    }
}
