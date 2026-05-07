import SwiftUI
#if canImport(Aurorality)
import Aurorality
#endif

/// HyperChat — crepus template + JS/Swift backends.
///
/// Architecture:
/// - Content area driven by AurorRootView (.crepus template)
/// - Service adapters: Matrix and Stalwart
/// - JavaScript backend: routeMessage, digest (keyword routing)
/// - Swift backend: ChatStorePlugin (message persistence)

@main
struct HyperChatApp: App {
    @State private var bridge = {
        let b = AurorBridge()
        b.register(ChatStorePlugin())
        return b
    }()
    @State private var state = AurorState()

    var body: some Scene {
        WindowGroup {
            ChatShell(bridge: bridge, state: state)
                .environment(bridge)
        }
        .windowStyle(.hiddenTitleBar)
        .windowToolbarStyle(.unified)
    }
}

// MARK: - Shell

struct ChatShell: View {
    let bridge: AurorBridge
    let state: AurorState

    @State private var draft = ""
    @State private var preferred = "auto"
    @State private var lastRoute = RouteDecision(selected: "matrix", confidence: 80, reason: "federated room route")
    @State private var transportHealth: [String: TransportHealth] = [:]
    @State private var messages: [ChatMessage] = []

    private let transportChoices = [
        ServiceOption(id: "auto", name: "Automatic", symbol: "sparkles", tint: .blue),
        ServiceOption(id: "matrix", name: "Matrix", symbol: "number", tint: .purple),
        ServiceOption(id: "stalwart", name: "Stalwart", symbol: "archivebox.fill", tint: .cyan),
    ]

    var body: some View {
        AurorRootView(state: state)
        .frame(minWidth: 900, minHeight: 680)
        .background(Color(nsColor: .windowBackgroundColor))
        .task {
            try? state.loadBundled(name: "main")
        }
    }

    // MARK: - Compose bar

    private var composeBar: some View {
        VStack(alignment: .leading, spacing: 10) {
            Picker("Service", selection: $preferred) {
                ForEach(transportChoices) { service in
                    Label(service.name, systemImage: service.symbol).tag(service.id)
                }
            }
            .pickerStyle(.segmented)
            HStack(alignment: .bottom, spacing: 10) {
                TextField("Message payload", text: $draft, axis: .vertical)
                    .lineLimit(2...6)
                    .textFieldStyle(.roundedBorder)
                    .onSubmit { send() }
                Button("Send", action: send)
                    .buttonStyle(.borderedProminent)
                    .disabled(!canSend)
            }
            Text(lastRoute.reason)
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .padding(.horizontal)
        .padding(.vertical, 12)
        .glassEffect()
    }

    // MARK: - Actions

    private func send() {
        let text = draft.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty else { return }
        let route = routeForDraft()
        lastRoute = route
        _ = try? bridge.invoke(pluginId: "chatStore", method: "send", payload: json([
            "text": text, "transport": route.selected,
        ]))
        if route.selected != "auto" {
            _ = try? bridge.invoke(pluginId: route.selected, method: "send", payload: json(["text": text]))
        }
        draft = ""
        refreshAll(route: route)
    }

    private func refreshAll(route: RouteDecision? = nil) {
        let route = route ?? routeForDraft()
        lastRoute = route
        for tid in ["matrix", "stalwart"] { refreshHealth(tid) }

        let jsRoute = (try? bridge.invokeData(
            pluginId: "chatJs", method: "routeMessage",
            payload: json(["text": draft, "preferred": preferred]),
            as: RouteDecision.self
        )) ?? route

        let messagesJson = try? bridge.invoke(pluginId: "chatStore", method: "list")
        messages = messagesJson.flatMap { decode(MessageList.self, from: $0) }?.items ?? []
        let jsDigest = (try? bridge.invokeData(
            pluginId: "chatJs", method: "digest",
            payload: json(["messages": messages.map { ["text": $0.text] }]),
            as: DigestResult.self
        )) ?? DigestResult(count: messages.count, lastPreview: "No messages yet")

        let stHealth = (try? bridge.invokeData(
            pluginId: "stalwart", method: "health", payload: "{}", as: TransportHealth.self
        ))

        let url = Bundle.main.url(forResource: "main", withExtension: "crepus", subdirectory: "views")
        let template = url.flatMap { try? String(contentsOf: $0, encoding: .utf8) } ?? "span \"HyperChat\""

        var ctx: [String: ContextValue] = [:]
        ctx["selectedTransport"] = .string(jsRoute.selected)
        ctx["confidence"] = .int(jsRoute.confidence)
        ctx["routeReason"] = .string(jsRoute.reason)
        ctx["messageCount"] = .int(jsDigest.count)
        ctx["lastPreview"] = .string(jsDigest.lastPreview)
        ctx["stalwartStatus"] = .string(stHealth?.connected == true ? "online" : "offline")
        ctx["services"] = .list(transportInfos().map { t in
            ["name": .string(t.name), "mode": .string(t.role), "status": .string(t.trust)]
        })
        ctx["transports"] = .list(transportInfos().map { t in
            ["name": .string(t.name), "role": .string(t.role), "trust": .string(t.trust)]
        })
        ctx["routes"] = .list(routePlan(jsRoute.selected).map { r in
            ["rank": .int(r.rank), "id": .string(r.id), "mode": .string(r.mode)]
        })
        ctx["messages"] = .list(messages.map { m in
            ["text": .string(m.text), "transport": .string(m.transport), "status": .string(m.status)]
        })
        try? state.load(template: template, context: ctx)
    }

    private func refreshHealth(_ tid: String) {
        guard let result = try? bridge.invoke(pluginId: tid, method: "health", payload: "{}"),
              let data = result.data(using: .utf8),
              let envelope = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              envelope["ok"] as? Bool == true,
              let healthJson = envelope["data"] as? [String: Any],
              let healthData = try? JSONSerialization.data(withJSONObject: healthJson),
              let health = try? JSONDecoder().decode(TransportHealth.self, from: healthData)
        else { return }
        transportHealth[tid] = health
    }

    private func routeForDraft() -> RouteDecision {
        if preferred != "auto" {
            return RouteDecision(selected: preferred, confidence: 100, reason: "\(service(for: preferred).name) selected")
        }
        let lower = draft.lowercased()
        if lower.contains("archive") || lower.contains("email") {
            return RouteDecision(selected: "stalwart", confidence: 82, reason: "Durable archive route")
        }
        return RouteDecision(selected: "matrix", confidence: 78, reason: "Federated room route")
    }

    // MARK: - Helpers

    private func transportInfos() -> [TransportInfo] {
        [
            TransportInfo(id: "matrix", name: "Matrix", role: "federation", trust: serviceStatus(service(for: "matrix")), latency: 15),
            TransportInfo(id: "stalwart", name: "Stalwart Archive", role: "archive", trust: serviceStatus(service(for: "stalwart")), latency: 10),
        ]
    }

    private func routePlan(_ selected: String) -> [RouteStep] {
        switch selected {
        case "matrix":
            return [RouteStep(rank: 1, id: "matrix", mode: "federated room"), RouteStep(rank: 2, id: "stalwart", mode: "archive relay")]
        case "stalwart":
            return [RouteStep(rank: 1, id: "stalwart", mode: "durable archive"), RouteStep(rank: 2, id: "matrix", mode: "federation mirror")]
        default:
            return [RouteStep(rank: 1, id: "matrix", mode: "federation"), RouteStep(rank: 2, id: "stalwart", mode: "archive")]
        }
    }

    private var canSend: Bool {
        !draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    private func service(for id: String) -> ServiceOption {
        transportChoices.first { $0.id == id } ?? transportChoices[1]
    }

    private func serviceStatus(_ service: ServiceOption) -> String {
        if service.id == "auto" {
            return "Selects Matrix or Stalwart"
        }
        if transportHealth[service.id]?.connected == true {
            return "Available"
        }
        return transportHealth[service.id]?.lastError ?? "Unavailable"
    }
}

// MARK: - Types

struct RouteDecision: Decodable, Sendable { let selected: String; let confidence: Int; let reason: String }
struct MessageList: Decodable { let items: [ChatMessage] }
struct DigestResult: Decodable { let count: Int; let lastPreview: String }
struct RouteStep { let rank: Int; let id: String; let mode: String }

struct ServiceOption: Identifiable, Hashable {
    let id: String
    let name: String
    let symbol: String
    let tint: Color
}

// MARK: - Top-level helpers

private func loadScriptPlugin(id: String, script: String) throws {
    guard let url = Bundle.main.url(forResource: script, withExtension: "js", subdirectory: "scripts") else {
        throw AurorPluginError("missing scripts/\(script).js")
    }
    try loadJsPlugin(id: id, code: String(contentsOf: url, encoding: .utf8))
}

private func json(_ value: Any) -> String {
    guard let data = try? JSONSerialization.data(withJSONObject: value),
          let json = String(data: data, encoding: .utf8)
    else { return "{}" }
    return json
}

private func decode<T: Decodable>(_ type: T.Type, from json: String) -> T? {
    try? JSONDecoder().decode(type, from: Data(json.utf8))
}
