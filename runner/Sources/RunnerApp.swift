import SwiftUI
import Aurorality

@main
struct AurorRunnerApp: App {
    @State private var state = AurorState()
    @State private var bridge = AurorBridge()
    @State private var client = HotReloadClient()

    var body: some Scene {
        WindowGroup {
            RunnerRootView(state: state, client: client)
                .environment(bridge)
                .environment(state)
        }
        .aurorBridge(bridge)
    }
}

struct RunnerRootView: View {
    let state: AurorState
    let client: HotReloadClient

    @State private var showConnect = false

    var body: some View {
        ZStack(alignment: .topTrailing) {
            AurorRootView(state: state)

            Button {
                showConnect.toggle()
            } label: {
                Image(systemName: statusIcon)
                    .padding(8)
                    .background(.ultraThinMaterial)
                    .clipShape(Circle())
            }
            .padding(12)
        }
        .sheet(isPresented: $showConnect) {
            ConnectView(client: client, state: state, isPresented: $showConnect)
        }
        .task { autoConnect() }
    }

    private func autoConnect() {
        if let portStr = ProcessInfo.processInfo.environment["AURORALITY_DEV_PORT"],
           let port = UInt16(portStr) {
            let host = ProcessInfo.processInfo.environment["AURORALITY_DEV_HOST"] ?? "127.0.0.1"
            client.connect(to: host, port: port, state: state)
            showConnect = false
        } else if case .disconnected = client.status {
            showConnect = true
        }
    }

    private var statusIcon: String {
        switch client.status {
        case .connected:   return "antenna.radiowaves.left.and.right"
        case .connecting:  return "antenna.radiowaves.left.and.right.slash"
        case .error:       return "exclamationmark.triangle"
        default:           return "antenna.radiowaves.left.and.right.slash"
        }
    }
}
