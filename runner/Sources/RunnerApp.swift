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

    @State private var showConnect = true

    var body: some View {
        ZStack(alignment: .topTrailing) {
            AurorRootView(state: state)

            // Dev toolbar
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
        .onAppear {
            if case .disconnected = client.status { showConnect = true }
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
