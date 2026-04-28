import SwiftUI
import Aurorality

@main
struct CounterApp: App {
    @State private var bridge = AurorBridge()
    @State private var state  = AurorState()
    @State private var count  = 0

    init() {
        // Register the Swift plugin alongside the built-in Rust ones.
        _bridge.wrappedValue.register(CounterPlugin())
    }

    var body: some Scene {
        WindowGroup {
            CounterView(state: state, bridge: bridge, count: $count)
                .environment(bridge)
                .task { loadTemplate() }
        }
    }

    private func loadTemplate() {
        let url = Bundle.main.url(forResource: "main", withExtension: "crepus")!
        let template = try! String(contentsOf: url)
        // Fetch a timestamp from the Rust CorePlugin.
        let tsJson   = try! bridge.invoke(pluginId: "core", method: "timestamp", payload: "{}")
        let ts       = (try? JSONDecoder().decode(TimestampResponse.self, from: Data(tsJson.utf8)))?.unixMs ?? 0
        try! state.load(
            template: template,
            context: [
                "count":     .int(count),
                "timestamp": .string("\(ts)ms"),
            ]
        )
    }

    private struct TimestampResponse: Decodable {
        let unixMs: UInt64
        enum CodingKeys: String, CodingKey { case unixMs }
    }
}

struct CounterView: View {
    let state: AurorState
    let bridge: AurorBridge
    @Binding var count: Int

    var body: some View {
        AurorRootView(state: state)
            // Wire buttons to the counter plugin.
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
        guard let json = try? bridge.invoke(pluginId: "counter", method: method),
              let data = json.data(using: .utf8),
              let obj  = try? JSONDecoder().decode(CountResult.self, from: data)
        else { return }
        count = obj.count
        let url = Bundle.main.url(forResource: "main", withExtension: "crepus")!
        let template = try! String(contentsOf: url)
        let tsJson   = try! bridge.invoke(pluginId: "core", method: "timestamp", payload: "{}")
        let ts       = (try? JSONDecoder().decode(TimestampResponse.self, from: Data(tsJson.utf8)))?.unixMs ?? 0
        try? state.load(
            template: template,
            context: [
                "count":     .int(count),
                "timestamp": .string("\(ts)ms"),
            ]
        )
    }

    private struct CountResult: Decodable { let count: Int }
    private struct TimestampResponse: Decodable {
        let unixMs: UInt64
        enum CodingKeys: String, CodingKey { case unixMs }
    }
}
