#if canImport(Aurorality)
import Aurorality
#endif
import SwiftUI

@main
struct HyperChatApp: App {
    private let bridge: AurorBridge
    private let model: HyperChatModel

    init() {
        let b = AurorBridge()
        b.register(RustPlugin(id: "matrix"))
        b.register(RustPlugin(id: "stalwart"))
        bridge = b
        model = HyperChatModel(bridge: b)
    }

    var body: some Scene {
        WindowGroup {
            HyperChatGeneratedView(
                context: model.viewContext,
                eventSink: { model.handleEvent($0) }
            )
            .frame(minWidth: 900, minHeight: 620)
            .background(Color(nsColor: .windowBackgroundColor))
            .aurorBridge(bridge)
            .task {
                model.refreshTransportHealth()
            }
        }
        .defaultSize(width: 960, height: 700)
        .commands {
            CommandGroup(replacing: .newItem) {
                Button("New Conversation", action: addConversation)
                .keyboardShortcut("n", modifiers: .command)
            }
            CommandMenu("Transport") {
                Button("Refresh Status", action: refreshTransportHealth)
                .keyboardShortcut("r", modifiers: .command)
                Divider()
                Button("Use Matrix") { selectProtocol("matrix") }
                Button("Use Stalwart") { selectProtocol("stalwart") }
                Button("Use Bitchat (view-only)") { selectProtocol("bitchat") }
            }
        }

        Settings {
            HyperChatSettingsView(model: model)
        }
    }

    private func addConversation() {
        model.addConversation()
    }

    private func refreshTransportHealth() {
        model.refreshTransportHealth()
    }

    private func selectProtocol(_ id: String) {
        model.selectedProtocol = id
        model.syncProtocolLabelForSelection()
    }
}

// MARK: - Settings (⌘,)

private struct HyperChatSettingsView: View {
    let model: HyperChatModel

    @State private var matrixHomeserver = ""
    @State private var matrixUserId = ""
    @State private var matrixAccessToken = ""
    @State private var matrixRoomId = ""
    @State private var stalwartBaseUrl = "http://localhost:8080"
    @State private var stalwartUsername = ""
    @State private var stalwartPassword = ""

    var body: some View {
        Form {
            Section {
                Text("Configure Matrix and Stalwart here; values are saved and exported to runtime env for the Rust transport adapters.")
                    .font(.body)
            } header: {
                Text("Transports")
            }
            Section("Matrix") {
                TextField("Homeserver", text: $matrixHomeserver)
                TextField("User ID", text: $matrixUserId)
                SecureField("Access token", text: $matrixAccessToken)
                TextField("Room ID", text: $matrixRoomId)
            }
            Section("Stalwart") {
                TextField("Base URL", text: $stalwartBaseUrl)
                TextField("Username", text: $stalwartUsername)
                SecureField("Password", text: $stalwartPassword)
            }
            Section {
                LabeledContent("Hot reload (`.crepus`)") {
                    Text(
                        """
                        brisk run
                        """
                    )
                    .font(.callout.monospaced())
                    .textSelection(.enabled)
                }
                Text("HyperChat is driven by `views/main.crepus` through `aurorality swiftgen`, which Brisk runs in `[pre_build]` before compiling native SwiftUI.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            } header: {
                Text("Templates")
            }
            Section {
                Text("Use `cargo run -p aurorality-cli -- swiftgen --view views/main.crepus --out Generated --view-name HyperChatGeneratedView --context-type HyperChatContext` for manual regeneration.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            } header: {
                Text("Manual swiftgen")
            }
            Section {
                Button("Save transport configuration") {
                    model.saveTransportConfig(
                        matrixHomeserver: matrixHomeserver,
                        matrixUserId: matrixUserId,
                        matrixAccessToken: matrixAccessToken,
                        matrixRoomId: matrixRoomId,
                        stalwartBaseUrl: stalwartBaseUrl,
                        stalwartUsername: stalwartUsername,
                        stalwartPassword: stalwartPassword
                    )
                }
                Button("Reload status now") {
                    model.refreshTransportHealth()
                }
            }
        }
        .formStyle(.grouped)
        .frame(width: 520, height: 620)
        .onAppear {
            matrixHomeserver = model.matrixHomeserver
            matrixUserId = model.matrixUserId
            matrixAccessToken = model.matrixAccessToken
            matrixRoomId = model.matrixRoomId
            stalwartBaseUrl = model.stalwartBaseUrl
            stalwartUsername = model.stalwartUsername
            stalwartPassword = model.stalwartPassword
        }
    }
}
