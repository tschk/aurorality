/// Central plugin registry. Holds both Swift and Rust plugins.
/// Accessible from SwiftUI via @Environment.

import Foundation

@Observable
public final class AurorBridge {
    private var plugins: [String: any AurorPlugin] = [:]

    /// Default bridge: registers the Rust CorePlugin and AppPlugin.
    public init() {
        register(RustPlugin(id: "core"))
        register(RustPlugin(id: "app"))
    }

    public func register(_ plugin: any AurorPlugin) {
        plugins[plugin.id] = plugin
    }

    /// Dispatch a call to a registered plugin.
    /// Returns the raw JSON string from the plugin.
    @discardableResult
    public func invoke(pluginId: String, method: String, payload: String = "{}") throws -> String {
        guard let plugin = plugins[pluginId] else {
            throw AurorPluginError("no plugin registered as \"\(pluginId)\"")
        }
        return try plugin.invoke(method: method, payload: payload)
    }

    /// Convenience: dispatch and decode the result as `T`.
    public func invoke<T: Decodable>(
        pluginId: String,
        method: String,
        payload: String = "{}",
        as _: T.Type = T.self
    ) throws -> T {
        let json = try invoke(pluginId: pluginId, method: method, payload: payload)
        return try JSONDecoder().decode(T.self, from: Data(json.utf8))
    }
}
