/// Shared dev-runtime state for `aurorality dev` WebSocket messages that are not IR patches.
/// IR updates still flow through [`AurorState`]; this bus holds swiftgen / session metadata for the HUD.

import Foundation
import Observation

@Observable
public final class HotReloadBus {
    public static let shared = HotReloadBus()

    public private(set) var lastDevHelloSessionId: String?
    public private(set) var watchDir: String?
    public private(set) var swiftgenViewPath: String?
    public private(set) var swiftgenOutPath: String?
    public private(set) var irEnabledFromServer: Bool = true

    public private(set) var lastSwiftgenOk: Bool?
    public private(set) var lastSwiftgenErrors: [String] = []
    public private(set) var lastSwiftgenOutputPath: String?
    public private(set) var lastSwiftgenTsMs: UInt64?

    /// When true, [`AurorDevOverlay`] swaps the hosted view for live IR ([`AurorRootView`]).
    public var liveIRMode: Bool = false

    private init() {}

    func ingest(_ message: HotReloadMessage) {
        switch message.kind {
        case .devHello:
            lastDevHelloSessionId = message.sessionId
            watchDir = message.watchDir
            swiftgenViewPath = message.swiftgenView
            swiftgenOutPath = message.swiftgenOut
            irEnabledFromServer = message.irEnabled ?? true
        case .swiftgenStatus:
            lastSwiftgenOk = message.ok
            lastSwiftgenErrors = message.errors ?? []
            lastSwiftgenOutputPath = message.outputPath
            lastSwiftgenTsMs = message.tsMs
        default:
            break
        }
    }
}
