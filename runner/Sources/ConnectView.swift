import SwiftUI
import Aurorality

struct ConnectView: View {
    let client: HotReloadClient
    let state: AurorState
    @Binding var isPresented: Bool

    @State private var host = "127.0.0.1"
    @State private var portString = "47832"

    var body: some View {
        NavigationStack {
            Form {
                Section("Dev server") {
                    TextField("Host", text: $host)
                        .autocorrectionDisabled()
                        #if os(iOS)
                        .keyboardType(.URL)
                        #endif

                    TextField("Port", text: $portString)
                        #if os(iOS)
                        .keyboardType(.numberPad)
                        #endif
                }

                Section {
                    statusRow
                }

                Section {
                    Button("Connect") {
                        let port = UInt16(portString) ?? 47832
                        client.connect(to: host, port: port, state: state)
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
                    Text("Run `aurorality dev` in your project, then enter the server address above.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
            .navigationTitle("Aurorality Runner")
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { isPresented = false }
                }
            }
        }
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
            case .error(let msg):
                Image(systemName: "xmark.circle.fill").foregroundStyle(.red)
                Text(msg).foregroundStyle(.red).lineLimit(1)
            }
        }
    }
}
