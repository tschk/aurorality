import SwiftUI
import Aurorality

#if SWIFT_PACKAGE
private func resourceURL(_ name: String, _ ext: String) -> URL? {
    Bundle.module.url(forResource: name, withExtension: ext)
}
#else
private func resourceURL(_ name: String, _ ext: String) -> URL? {
    Bundle.main.url(forResource: name, withExtension: ext)
}
#endif

@main
struct CounterApp: App {
    @State private var bridge = AurorBridge()
    @State private var state  = AurorState()

    var body: some Scene {
        WindowGroup {
            CounterView(state: state, bridge: bridge)
                .environment(bridge)
                .task { load() }
        }
    }

    private func load() {
        guard let url = resourceURL("backend", "js"),
              let code = try? String(contentsOf: url, encoding: .utf8) else { return }
        // JS plugin lives in Rust bridge. Register a RustPlugin proxy in Swift bridge.
        try? loadJsPlugin(id: "counter", code: code)
        bridge.register(RustPlugin(id: "counter"))
        render()
    }

    private func render() {
        let url = resourceURL("main", "crepus")
        let template = url.flatMap { try? String(contentsOf: $0) } ?? "No template found"
        let data = (try? bridge.invokeData(pluginId: "counter", method: "state", as: CounterData.self))
            ?? CounterData(count: "0", mood: "neutral", next: "Tap a button")
        try? state.load(
            template: template,
            context: [
                "count":     .string(data.count),
                "mood":      .string(data.mood),
                "next":      .string(data.next),
            ]
        )
    }

    private struct CounterData: Decodable { let count: String; let mood: String; let next: String }
}

struct CounterView: View {
    let state: AurorState
    let bridge: AurorBridge
    @State private var errorMsg: String?

    var body: some View {
        AurorRootView(state: state)
            .alert("Error", isPresented: .constant(errorMsg != nil)) {
                Button("OK") { errorMsg = nil }
            } message: {
                Text(errorMsg ?? "")
            }
            .onReceive(NotificationCenter.default.publisher(for: .init("auror.event"))) { note in
                guard let event = note.object as? String else { return }
                switch event {
                case "Increment": tap("increment")
                case "Decrement": tap("decrement")
                default: break
                }
            }
    }

    private func tap(_ method: String) {
        do {
            let d: CounterData = try bridge.invokeData(pluginId: "counter", method: method)
            let url = resourceURL("main", "crepus")
            let template = url.flatMap { try? String(contentsOf: $0) } ?? ""
            try state.load(template: template, context: [
                "count": .string(d.count),
                "mood": .string(d.mood),
                "next": .string(d.next),
            ])
        } catch {
            errorMsg = error.localizedDescription
        }
    }

    private struct CounterData: Decodable { let count: String; let mood: String; let next: String }
}
