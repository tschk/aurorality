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
                .environment(
                    \.aurorDevEnabled,
                    ProcessInfo.processInfo.environment["AURORALITY_DEV"] == "1"
                )
                .aurorDevOverlay(templatePath: "views/main.crepus")
                .task { loadTemplate() }
        }
    }

    private func loadTemplate() {
        try? loadScriptPlugin(id: "counterJs", script: "backend")
        render()
    }

    private func render() {
        let url = Bundle.main.url(forResource: "main", withExtension: "crepus")
        let template = url.flatMap { try? String(contentsOf: $0) } ?? "No template found"
        let timestamp = try? bridge.invokeData(pluginId: "core", method: "timestamp", as: TimestampResponse.self)
        let ts = timestamp?.unixMs ?? 0
        let formatted = (try? bridge.invokeData(
            pluginId: "counterJs",
            method: "formatCounter",
            payload: encodePayload(["count": count]),
            as: CounterCopy.self
        )) ?? CounterCopy(display: "\(count)", mood: "neutral", next: "Tap a button")
        try? state.load(
            template: template,
            context: [
                "count":     .string(formatted.display),
                "mood":      .string(formatted.mood),
                "next":      .string(formatted.next),
                "timestamp": .string("\(ts)ms"),
            ]
        )
    }

    private struct TimestampResponse: Decodable {
        let unixMs: UInt64
        enum CodingKeys: String, CodingKey { case unixMs }
    }
    private struct CounterCopy: Decodable { let display: String; let mood: String; let next: String }

    private func loadScriptPlugin(id: String, script: String) throws {
        guard let url = Bundle.main.url(forResource: script, withExtension: "js", subdirectory: "scripts") else {
            throw AurorPluginError("missing scripts/\(script).js")
        }
        try loadJsPlugin(id: id, code: String(contentsOf: url, encoding: .utf8))
    }

    private func encodePayload(_ dict: [String: Any]) -> String {
        guard let data = try? JSONSerialization.data(withJSONObject: dict),
              let json = String(data: data, encoding: .utf8)
        else { return "{}" }
        return json
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
        let url = Bundle.main.url(forResource: "main", withExtension: "crepus")
        let template = url.flatMap { try? String(contentsOf: $0) } ?? "No template found"
        let timestamp = try? bridge.invokeData(pluginId: "core", method: "timestamp", as: TimestampResponse.self)
        let ts = timestamp?.unixMs ?? 0
        let formatted = (try? bridge.invokeData(
            pluginId: "counterJs",
            method: "formatCounter",
            payload: encodePayload(["count": count]),
            as: CounterCopy.self
        )) ?? CounterCopy(display: "\(count)", mood: "neutral", next: "Tap a button")
        try? state.load(
            template: template,
            context: [
                "count":     .string(formatted.display),
                "mood":      .string(formatted.mood),
                "next":      .string(formatted.next),
                "timestamp": .string("\(ts)ms"),
            ]
        )
    }

    private struct CountResult: Decodable { let count: Int }
    private struct CounterCopy: Decodable { let display: String; let mood: String; let next: String }
    private struct TimestampResponse: Decodable {
        let unixMs: UInt64
        enum CodingKeys: String, CodingKey { case unixMs }
    }

    private func encodePayload(_ dict: [String: Any]) -> String {
        guard let data = try? JSONSerialization.data(withJSONObject: dict),
              let json = String(data: data, encoding: .utf8)
        else { return "{}" }
        return json
    }
}
