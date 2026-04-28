import SwiftUI
import Aurorality

@main
struct BasicApp: App {
    @State private var state = AurorState()
    @State private var bridge = AurorBridge()

    var body: some Scene {
        WindowGroup {
            AurorRootView(state: state)
                .environment(bridge)
                .task { try? state.loadBundled(name: "main") }
        }
    }
}
