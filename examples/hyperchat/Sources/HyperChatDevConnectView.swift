#if canImport(Aurorality)
import Aurorality
#endif
import SwiftUI

/// Minimal dev-server connection sheet (matches the standalone Runner UX).
struct HyperChatDevConnectView: View {
    let client: HotReloadClient
    let aurorState: AurorState
    @Binding var isPresented: Bool

    @State private var host = "127.0.0.1"
    @State private var portString = "47832"

    var body: some View {
        NavigationStack {
            Form {
                Section("WebSocket") {
                    TextField("Host", text: $host)
                        .autocorrectionDisabled()
                    TextField("Port", text: $portString)
                }
                Section {
                    statusRow
                }
                Section {
                    Button("Connect") {
                        let port = UInt16(portString) ?? 47832
                        client.connect(to: host, port: port, state: aurorState)
                        isPresented = false
                    }
                    .disabled(host.isEmpty)

                    if case .connected = client.status {
                        Button("Disconnect", role: .destructive) {
                            client.disconnect()
                        }
                    }
                }
                Section {
                    Text(
                        """
                        From the repo root run:

                        cargo run -p aurorality-cli -- dev --watch examples/hyperchat/views

                        Then connect here. The first `.crepus` file is sent as a full IR snapshot; edits hot-reload into the Template (live) tab.
                        """
                    )
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
                }
            }
            .navigationTitle("Template dev server")
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { isPresented = false }
                }
            }
        }
        .frame(minWidth: 420, idealWidth: 480, minHeight: 360)
    }

    @ViewBuilder
    private var statusRow: some View {
        HStack {
            Text("Status")
            Spacer()
            switch client.status {
            case .disconnected:
                Text("Disconnected").foregroundStyle(.secondary)
            case .connecting:
                ProgressView().scaleEffect(0.8)
                Text("Connecting…").foregroundStyle(.secondary)
            case .connected:
                Image(systemName: "checkmark.circle.fill").foregroundStyle(.green)
                Text("Connected").foregroundStyle(.green)
            case let .error(msg):
                Image(systemName: "xmark.circle.fill").foregroundStyle(.red)
                Text(msg).foregroundStyle(.red).lineLimit(2)
            }
        }
    }
}
