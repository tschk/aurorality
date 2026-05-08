#if canImport(Aurorality)
import Aurorality
#endif
#if canImport(AppKit)
import AppKit
#endif
import SwiftUI

@main
struct HyperChatApp: App {
    private let bridge: AurorBridge
    @State private var model: HyperChatModel

    init() {
        let b = AurorBridge()
        b.register(RustPlugin(id: "matrix"))
        b.register(RustPlugin(id: "stalwart"))
        bridge = b
        _model = State(wrappedValue: HyperChatModel(bridge: b))
    }

    var body: some Scene {
        WindowGroup {
            HyperChatRoot(
                bridge: bridge,
                model: model
            )
        }
        .defaultSize(width: 960, height: 700)
        .commands {
            HyperChatGeneratedViewCommands(
                context: model.viewContext,
                eventSink: { model.handleEvent($0) }
            )
        }
    }
}

// MARK: - Root chrome

private struct HyperChatRoot: View {
    let bridge: AurorBridge
    @Bindable var model: HyperChatModel

    var body: some View {
        HyperChatGeneratedView(
            context: model.viewContext,
            eventSink: { model.handleEvent($0) }
        )
        .frame(minWidth: 900, minHeight: 620)
        .background(Color(nsColor: .windowBackgroundColor))
        .aurorBridge(bridge)
        .environment(
            \.aurorDevEnabled,
            ProcessInfo.processInfo.environment["AURORALITY_DEV"] == "1"
        )
        .aurorDevOverlay(templatePath: "views/main.crepus")
        .sheet(isPresented: $model.showSettingsSheet) {
            HyperChatSettingsView(model: model)
        }
        .task {
            model.refreshTransportHealth()
        }
        #if canImport(AppKit)
            .onReceive(
                NotificationCenter.default.publisher(for: NSApplication.didBecomeActiveNotification)
            ) { _ in
                model.applicationDidBecomeActive()
            }
            .onReceive(
                NotificationCenter.default.publisher(for: NSApplication.didResignActiveNotification)
            ) { _ in
                model.applicationDidResignActive()
            }
        #endif
    }
}

// MARK: - Settings (⌘, via template menubar or sheet)

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
                Text(
                    "Configure Matrix and Stalwart here; values are saved and exported to runtime env for the Rust transport adapters."
                )
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
                    Text("brisk run")
                        .font(.callout.monospaced())
                        .textSelection(.enabled)
                }
                Text(
                    "HyperChat is driven by `views/main.crepus` through `aurorality swiftgen`, which Brisk runs in `[pre_build]` before compiling native SwiftUI."
                )
                .font(.caption)
                .foregroundStyle(.secondary)
            } header: {
                Text("Templates")
            }
            Section {
                Text(
                    "Use `cargo run -p aurorality-cli -- swiftgen --view views/main.crepus --out Generated --view-name HyperChatGeneratedView --context-type HyperChatContext` for manual regeneration."
                )
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
