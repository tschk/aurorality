#if canImport(Aurorality)
import Aurorality
#endif
import Foundation
import Observation

/// Orchestrates HyperChat state, transport health, and `.crepus` → `eventSink` actions.
@Observable
public final class HyperChatModel {
    public private(set) var conversations: [ConversationItem] = []
    public var selectedConversationId: String?
    public var draft: String = ""
    /// `matrix` | `stalwart` | `bitchat` | ``
    public var selectedProtocol: String = ""

    /// Sidebar + native UI; updated by `refreshTransportHealth()`.
    public private(set) var matrixTransport: TransportSidebarStatus = .placeholder
    public private(set) var stalwartTransport: TransportSidebarStatus = .placeholder
    public private(set) var bitchatTransport: TransportSidebarStatus = TransportSidebarStatus(
        state: .unavailable,
        headline: "Not in this build",
        detail: "Upstream is CLI-only."
    )

    /// One-line strings for `HyperChatGeneratedView` / tooling (no duplicated section titles).
    public var matrixStatusLine: String { legacyStatusLine(title: "Matrix", status: matrixTransport) }
    public var stalwartStatusLine: String { legacyStatusLine(title: "Stalwart", status: stalwartTransport) }
    public var bitchatStatusLine: String { legacyStatusLine(title: "Bitchat", status: bitchatTransport) }

    @ObservationIgnored private let bridge: AurorBridge
    @ObservationIgnored private let defaults = UserDefaults.standard
    @ObservationIgnored private let keyPrefix = "hyperchat.transport."

    public init(bridge: AurorBridge) {
        self.bridge = bridge
        applyStoredTransportConfig()
        seedIfNeeded()
    }

    /// Snapshot for `HyperChatGeneratedView` / native UI.
    public var viewContext: HyperChatContext {
        let sel = selectedConversationId.flatMap { id in conversations.first { $0.id == id } }
        let recipient = sel?.title ?? "New Conversation"
        let proto = sel?.protocolLabel ?? (selectedProtocol.isEmpty ? "(pick protocol)" : protocolTitle(selectedProtocol))
        return HyperChatContext(
            conversations: conversations,
            selectedConversationId: selectedConversationId,
            recipientTitle: recipient,
            protocolSummary: proto,
            headerBadge: protocolTitle(selectedProtocol),
            fallbackLabel: "Offline / local mesh",
            messages: currentMessages,
            draft: draft,
            selectedProtocol: selectedProtocol,
            matrixStatusLine: matrixStatusLine,
            stalwartStatusLine: stalwartStatusLine,
            bitchatStatusLine: bitchatStatusLine,
            canSend: canSend,
            newConversationPrompt: selectedProtocol.isEmpty,
            bitchatSendBlocked: bitchatSendBlocked,
            sendDisabledHint: sendDisabledHint
        )
    }

    public private(set) var threadMessages: [String: [MessageItem]] = [:]

    private var currentMessages: [MessageItem] {
        guard let id = selectedConversationId else { return [] }
        return threadMessages[id] ?? []
    }

    private var canSend: Bool {
        let t = draft.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !t.isEmpty, !selectedProtocol.isEmpty else { return false }
        if sanitizeProtocol(selectedProtocol) == "bitchat" { return false }
        return true
    }

    private var bitchatSendBlocked: Bool {
        sanitizeProtocol(selectedProtocol) == "bitchat"
    }

    private var sendDisabledHint: String? {
        let t = draft.trimmingCharacters(in: .whitespacesAndNewlines)
        if t.isEmpty { return "Enter a message" }
        if selectedProtocol.isEmpty { return "Choose Matrix, Stalwart, or Bitchat" }
        if bitchatSendBlocked { return "Bitchat can’t send in this demo" }
        return nil
    }

    private func seedIfNeeded() {
        guard conversations.isEmpty else { return }
        let main = "local-main"
        conversations = [
            ConversationItem(
                id: main,
                title: "Notes",
                subtitle: "Local",
                protocolLabel: "—"
            ),
        ]
        selectedConversationId = main
        threadMessages[main] = []
    }

    public func addConversation() {
        let n = conversations.count + 1
        let id = UUID().uuidString
        let proto = selectedProtocol.isEmpty ? "—" : protocolTitle(selectedProtocol)
        conversations.append(
            ConversationItem(
                id: id,
                title: "Conversation \(n)",
                subtitle: "Local",
                protocolLabel: proto
            )
        )
        selectedConversationId = id
        threadMessages[id] = []
    }

    public func deleteConversations(at offsets: IndexSet) {
        let removedIds = offsets.compactMap { i -> String? in
            guard conversations.indices.contains(i) else { return nil }
            return conversations[i].id
        }
        conversations.remove(atOffsets: offsets)
        for id in removedIds {
            threadMessages.removeValue(forKey: id)
        }
        if let sel = selectedConversationId, removedIds.contains(sel) {
            selectedConversationId = conversations.first?.id
        }
    }

    public func syncProtocolLabelForSelection() {
        guard let idx = conversations.firstIndex(where: { $0.id == selectedConversationId }) else { return }
        var c = conversations[idx]
        c.protocolLabel = selectedProtocol.isEmpty ? "—" : protocolTitle(selectedProtocol)
        conversations[idx] = c
    }

    /// Call from `HyperChatGeneratedView(context:eventSink:)`.
    public func handleEvent(_ raw: String) {
        if raw.hasPrefix("bind:draft:") {
            let rest = String(raw.dropFirst("bind:draft:".count))
            draft = rest
            return
        }
        if raw.hasPrefix("bind:selectedProtocol:") {
            let rest = String(raw.dropFirst("bind:selectedProtocol:".count))
            selectedProtocol = sanitizeProtocol(rest)
            syncProtocolLabelForSelection()
            return
        }
        if raw.hasPrefix("selectConversation:") {
            let id = String(raw.dropFirst("selectConversation:".count))
            if conversations.contains(where: { $0.id == id }) {
                selectedConversationId = id
                if let selected = conversations.first(where: { $0.id == id }) {
                    selectedProtocol = protocolId(fromTitle: selected.protocolLabel)
                }
            }
            return
        }
        switch raw {
        case "send":
            sendFromDraft()
        case "newConversation":
            addConversation()
        case "refreshStatus":
            refreshTransportHealth()
        case "pickMatrix":
            selectedProtocol = "matrix"
            syncProtocolLabelForSelection()
        case "pickStalwart":
            selectedProtocol = "stalwart"
            syncProtocolLabelForSelection()
        case "pickBitchat":
            selectedProtocol = "bitchat"
            syncProtocolLabelForSelection()
        default:
            break
        }
    }

    /// Send action for native UI (same as the generated view’s `"send"` event).
    public func commitSend() {
        sendFromDraft()
    }

    public func refreshTransportHealth() {
        applyStoredTransportConfig()
        matrixTransport = parseTransportHealth(json: matrixHealthJson())
        stalwartTransport = parseTransportHealth(json: stalwartHealthJson())
        bitchatTransport = parseBitchatTransport(json: bitchatStatusJson())
    }

    // MARK: - Transport config (stored + exported to process env)

    public var matrixHomeserver: String {
        defaults.string(forKey: keyPrefix + "matrix.homeserver") ?? ""
    }

    public var matrixUserId: String {
        defaults.string(forKey: keyPrefix + "matrix.userId") ?? ""
    }

    public var matrixAccessToken: String {
        defaults.string(forKey: keyPrefix + "matrix.accessToken") ?? ""
    }

    public var matrixRoomId: String {
        defaults.string(forKey: keyPrefix + "matrix.roomId") ?? ""
    }

    public var stalwartBaseUrl: String {
        defaults.string(forKey: keyPrefix + "stalwart.baseUrl") ?? "http://localhost:8080"
    }

    public var stalwartUsername: String {
        defaults.string(forKey: keyPrefix + "stalwart.username") ?? ""
    }

    public var stalwartPassword: String {
        defaults.string(forKey: keyPrefix + "stalwart.password") ?? ""
    }

    public func saveTransportConfig(
        matrixHomeserver: String,
        matrixUserId: String,
        matrixAccessToken: String,
        matrixRoomId: String,
        stalwartBaseUrl: String,
        stalwartUsername: String,
        stalwartPassword: String
    ) {
        defaults.set(matrixHomeserver, forKey: keyPrefix + "matrix.homeserver")
        defaults.set(matrixUserId, forKey: keyPrefix + "matrix.userId")
        defaults.set(matrixAccessToken, forKey: keyPrefix + "matrix.accessToken")
        defaults.set(matrixRoomId, forKey: keyPrefix + "matrix.roomId")
        defaults.set(stalwartBaseUrl, forKey: keyPrefix + "stalwart.baseUrl")
        defaults.set(stalwartUsername, forKey: keyPrefix + "stalwart.username")
        defaults.set(stalwartPassword, forKey: keyPrefix + "stalwart.password")
        applyStoredTransportConfig()
        refreshTransportHealth()
    }

    private func applyStoredTransportConfig() {
        applyEnv("MATRIX_HOMESERVER", matrixHomeserver)
        applyEnv("MATRIX_USER_ID", matrixUserId)
        applyEnv("MATRIX_ACCESS_TOKEN", matrixAccessToken)
        applyEnv("MATRIX_ROOM_ID", matrixRoomId)
        applyEnv("STALWART_BASE_URL", stalwartBaseUrl)
        applyEnv("STALWART_USERNAME", stalwartUsername)
        applyEnv("STALWART_PASSWORD", stalwartPassword)
    }

    private func applyEnv(_ name: String, _ value: String) {
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty {
            unsetenv(name)
        } else {
            setenv(name, trimmed, 1)
        }
    }

    private func legacyStatusLine(title: String, status: TransportSidebarStatus) -> String {
        if let d = status.detail?.trimmingCharacters(in: .whitespacesAndNewlines), !d.isEmpty {
            return "\(title): \(status.headline) — \(d)"
        }
        return "\(title): \(status.headline)"
    }

    /// Parses Matrix/Stalwart `TransportHealth` JSON from `hyperchat-backend`.
    private func parseTransportHealth(json: String) -> TransportSidebarStatus {
        guard let data = json.data(using: .utf8) else {
            return TransportSidebarStatus(state: .disconnected, headline: "Unknown", detail: nil)
        }
        if let env = try? JSONDecoder().decode(JsonEnvelope.self, from: data),
            let e = env.error?.trimmingCharacters(in: .whitespacesAndNewlines), !e.isEmpty
        {
            return TransportSidebarStatus(
                state: .misconfigured,
                headline: "Can’t load",
                detail: shortReason(e)
            )
        }
        guard let d = try? JSONDecoder().decode(TransportHealthRow.self, from: data) else {
            return TransportSidebarStatus(state: .disconnected, headline: "Unreadable", detail: nil)
        }

        if d.connected == true {
            let ms = d.latencyMs.map { "\($0) ms" }
            return TransportSidebarStatus(state: .connected, headline: "Connected", detail: ms)
        }

        let err = d.lastError?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if err.localizedCaseInsensitiveContains("not configured")
            || err.localizedCaseInsensitiveContains("credentials not configured")
        {
            return TransportSidebarStatus(state: .misconfigured, headline: "Not configured", detail: nil)
        }
        if err.isEmpty {
            return TransportSidebarStatus(state: .disconnected, headline: "Offline", detail: nil)
        }
        return TransportSidebarStatus(
            state: .disconnected,
            headline: "Offline",
            detail: shortReason(err)
        )
    }

    private func parseBitchatTransport(json: String) -> TransportSidebarStatus {
        guard let data = json.data(using: .utf8) else {
            return TransportSidebarStatus(state: .unavailable, headline: "Unavailable", detail: nil)
        }
        if (try? JSONDecoder().decode(BitchatHealthRow.self, from: data)) != nil {
            return TransportSidebarStatus(
                state: .unavailable,
                headline: "Not in app bundle",
                detail: "Mesh tools are CLI-only in upstream."
            )
        }
        if let env = try? JSONDecoder().decode(JsonEnvelope.self, from: data),
            let e = env.error, !e.isEmpty
        {
            return TransportSidebarStatus(state: .unavailable, headline: "Unavailable", detail: shortReason(e))
        }
        return TransportSidebarStatus(state: .unavailable, headline: "Unavailable", detail: nil)
    }

    /// Sidebar-safe: no URLs, capped length.
    private func shortReason(_ raw: String, maxLen: Int = 90) -> String {
        let s = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        if s.contains("http://") || s.contains("https://") {
            return "Check documentation for setup."
        }
        if s.count > maxLen {
            return String(s.prefix(maxLen - 1)) + "…"
        }
        return s
    }

    private func sendFromDraft() {
        let text = draft.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty else { return }
        guard !selectedProtocol.isEmpty else { return }
        guard let conv = selectedConversationId else { return }

        if sanitizeProtocol(selectedProtocol) == "bitchat" {
            appendFailed(conv: conv, text: text, meta: "Bitchat: no linkable library — cannot send.")
            draft = ""
            return
        }

        let optimistic = MessageItem(
            id: UUID().uuidString,
            body: text,
            metaLine: "\(protocolTitle(selectedProtocol)) · sending",
            isOutgoing: true
        )
        threadMessages[conv, default: []].append(optimistic)
        draft = ""

        let json: String = {
            switch sanitizeProtocol(selectedProtocol) {
            case "matrix": return matrixSendJson(text: text)
            case "stalwart": return stalwartSendJson(text: text)
            default: return "{}"
            }
        }()
        if json.contains("\"error\"") {
            markLastFailed(conv: conv, detail: extractError(json) ?? json)
            return
        }
        if json.contains("\"accepted\":false") {
            markLastFailed(conv: conv, detail: extractError(json) ?? "rejected")
        } else {
            markLastSent(conv: conv)
        }
    }

    private func markLastSent(conv: String) {
        guard var list = threadMessages[conv], !list.isEmpty else { return }
        let i = list.count - 1
        var m = list[i]
        m.metaLine = "\(protocolTitle(selectedProtocol)) · sent"
        list[i] = m
        threadMessages[conv] = list
    }

    private func markLastFailed(conv: String, detail: String) {
        guard var list = threadMessages[conv], !list.isEmpty else { return }
        let i = list.count - 1
        var m = list[i]
        m.metaLine = "\(protocolTitle(selectedProtocol)) · failed — \(detail)"
        list[i] = m
        threadMessages[conv] = list
    }

    private func appendFailed(conv: String, text: String, meta: String) {
        let m = MessageItem(id: UUID().uuidString, body: text, metaLine: meta, isOutgoing: true)
        threadMessages[conv, default: []].append(m)
    }

    private func sanitizeProtocol(_ s: String) -> String {
        let t = s.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        if ["matrix", "stalwart", "bitchat"].contains(t) { return t }
        return t
    }

    private func protocolTitle(_ p: String) -> String {
        switch sanitizeProtocol(p) {
        case "matrix": return "Matrix"
        case "stalwart": return "Stalwart"
        case "bitchat": return "Bitchat"
        default: return p.isEmpty ? "—" : p
        }
    }

    private func protocolId(fromTitle title: String) -> String {
        switch title.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
        case "matrix": return "matrix"
        case "stalwart": return "stalwart"
        case "bitchat": return "bitchat"
        default: return selectedProtocol
        }
    }

    private func extractError(_ json: String) -> String? {
        guard let data = json.data(using: .utf8),
            let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else { return nil }
        if let e = obj["error"] as? String { return e }
        if let dataObj = obj["data"] as? [String: Any],
            let r = dataObj["reason"] as? String { return r }
        return nil
    }
}

// MARK: - JSON helpers

private struct TransportHealthRow: Decodable {
    let connected: Bool?
    let latencyMs: UInt64?
    let lastError: String?

    enum CodingKeys: String, CodingKey {
        case connected
        case latencyMs = "latency_ms"
        case lastError = "last_error"
    }
}

private struct BitchatHealthRow: Decodable {
    let lastError: String?

    enum CodingKeys: String, CodingKey {
        case lastError = "last_error"
    }
}

private struct JsonEnvelope: Decodable {
    let error: String?
}

/// Reserved for non-Rust copy; Swift UI uses structured `bitchatTransport` after refresh.
public enum BitchatAdapter {
    public static let statusLine: String = "Bitchat: see status in sidebar after refresh"
}
