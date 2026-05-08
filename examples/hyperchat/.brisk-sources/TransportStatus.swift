import Foundation

/// Compact UI model for the sidebar — no duplicated “Matrix:” prefixes or walls of URL text.
public enum TransportConnectionState: Sendable, Equatable {
    case connected
    case disconnected
    case misconfigured
    case unavailable
}

public struct TransportSidebarStatus: Sendable, Equatable {
    public var state: TransportConnectionState
    /// Short primary label, e.g. “Connected”, “Offline”, “Not configured”.
    public var headline: String
    /// Optional second line (latency, short reason). Keep to one line in the sidebar when possible.
    public var detail: String?

    public static let placeholder = TransportSidebarStatus(state: .disconnected, headline: "…", detail: nil)
}

#if canImport(SwiftUI)
import SwiftUI

extension TransportConnectionState {
    /// Traffic-light style indicator in the sidebar.
    public var dotColor: Color {
        switch self {
        case .connected:
            return Color(nsColor: .systemGreen)
        case .disconnected:
            return Color(nsColor: .secondaryLabelColor)
        case .misconfigured:
            return Color(nsColor: .systemOrange)
        case .unavailable:
            return Color(nsColor: .tertiaryLabelColor)
        }
    }
}
#endif
