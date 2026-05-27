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
        // Load aurorality-lite first, then backend, in same JSC context.
        let lite = resourceURL("aurorality-lite", "js")
            .flatMap { try? String(contentsOf: $0) } ?? ""
        let backend = resourceURL("backend", "js")
            .flatMap { try? String(contentsOf: $0) } ?? ""
        try? loadJsPlugin(id: "counter", code: lite + "\n" + backend)
        render()
    }

    private func render() {
        let url = resourceURL("main", "crepus")
        let template = url.flatMap { try? String(contentsOf: $0) } ?? "No template found"
        let timestamp = try? bridge.invokeData(pluginId: "core", method: "timestamp", as: TimestampResponse.self)
        let ts = timestamp?.unixMs ?? 0
        let data = (try? bridge.invokeData(pluginId: "counter", method: "state", as: CounterData.self))
            ?? CounterData(count: "0", mood: "neutral", next: "Tap a button")
        try? state.load(
            template: template,
            context: [
                "count":     .string(data.count),
                "mood":      .string(data.mood),
                "next":      .string(data.next),
                "timestamp": .string("\(ts)ms"),
            ]
        )
    }

    private struct TimestampResponse: Decodable {
        let unixMs: UInt64
        enum CodingKeys: String, CodingKey { case unixMs }
    }
    private struct CounterData: Decodable { let count: String; let mood: String; let next: String }
}

struct CounterView: View {
    let state: AurorState
    let bridge: AurorBridge

    var body: some View {
        AurorRootView(state: state)
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
        let data = try? bridge.invokeData(pluginId: "counter", method: method, as: CounterData.self)
        guard let d = data else { return }
        let url = resourceURL("main", "crepus")
        let template = url.flatMap { try? String(contentsOf: $0) } ?? "No template found"
        let timestamp = try? bridge.invokeData(pluginId: "core", method: "timestamp", as: TimestampResponse.self)
        let ts = timestamp?.unixMs ?? 0
        try? state.load(
            template: template,
            context: [
                "count":     .string(d.count),
                "mood":      .string(d.mood),
                "next":      .string(d.next),
                "timestamp": .string("\(ts)ms"),
            ]
        )
    }

    private struct CounterData: Decodable { let count: String; let mood: String; let next: String }
    private struct TimestampResponse: Decodable {
        let unixMs: UInt64
        enum CodingKeys: String, CodingKey { case unixMs }
    }
}
