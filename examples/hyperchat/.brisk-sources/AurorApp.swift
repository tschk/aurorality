/// Convenience helpers for bootstrapping an aurorality app.

import SwiftUI

// MARK: - Environment keys

private struct BridgeKey: EnvironmentKey {
    static let defaultValue = AurorBridge()
}

extension EnvironmentValues {
    public var aurorBridge: AurorBridge {
        get { self[BridgeKey.self] }
        set { self[BridgeKey.self] = newValue }
    }
}

// MARK: - Scene modifier

extension Scene {
    /// Inject a shared AurorBridge into the environment.
    public func aurorBridge(_ bridge: AurorBridge) -> some Scene {
        self.environment(bridge)
    }
}

// MARK: - View modifier

extension View {
    /// Inject a shared AurorBridge into the environment.
    public func aurorBridge(_ bridge: AurorBridge) -> some View {
        self.environment(bridge)
    }
}
