#if canImport(Aurorality)
import Aurorality
#endif
#if canImport(AppKit)
import AppKit
#endif
import Foundation
import Observation
#if canImport(UserNotifications)
import UserNotifications
#endif

/// Orchestrates HyperChat state, transport health, and `.crepus` → `eventSink` actions.
@Observable
public final class HyperChatModel {
    public private(set) var conversations: [ConversationItem] = []
    public var selectedConversationId: String?
    public var draft: String = ""
    /// `matrix` | `stalwart` | `bitchat` | ``
    public var selectedProtocol: String = ""

    /// Bound from SwiftUI for Settings sheet (`menubar` → `openSettings`).
    public var showSettingsSheet = false

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
    @ObservationIgnored private var pollTimer: Timer?
    @ObservationIgnored private var typingTimer: Timer?
    @ObservationIgnored private var seenMatrixEvents = Set<String>()
    @ObservationIgnored private var typingUserIdsByRoom: [String: [String]] = [:]
    #if canImport(AppKit)
        @ObservationIgnored private var appActive = true
    #endif

    private let bundleId =
        Bundle.main.bundleIdentifier ?? "dev.aurorality.example.hyperchat"

    /// Typing hint shown under the transcript (`typingindicator` tag).
    public private(set) var typingLine: String = ""

    public init(bridge: AurorBridge) {
        self.bridge = bridge
        applyStoredTransportConfig()
        seedIfNeeded()
        loadPersistedSnapshot()
        reschedulePollingTimer()
        requestNotificationsIfNeeded()
        updateDockBadge()
    }

    deinit {
        pollTimer?.invalidate()
        typingTimer?.invalidate()
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
            sendDisabledHint: sendDisabledHint,
            totalUnread: conversations.reduce(0) { $0 + max(0, $1.unread) },
            typingLine: typingLine
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
        persistSnapshot()
    }

    public func deleteConversations(at offsets: IndexSet) {
        let removedIds = offsets.compactMap { i -> String? in
            guard conversations.indices.contains(i) else { return nil }
            return conversations[i].id
        }
        conversations.remove(atOffsets: offsets)
        var localThreadMessages = threadMessages
        for id in removedIds {
            localThreadMessages.removeValue(forKey: id)
        }
        threadMessages = localThreadMessages
        if let sel = selectedConversationId, removedIds.contains(sel) {
            selectedConversationId = conversations.first?.id
        }
        persistSnapshot()
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
            scheduleTypingPing()
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
                clearUnread(for: id)
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
        case "openSettings":
            showSettingsSheet = true
        case "archiveSelected":
            archiveSelectedConversation()
        case "openInfo":
            break
        case "pickAttachment":
            pickAttachment()
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

    #if canImport(AppKit)
        /// Hook `NSApplication.didBecomeActiveNotification`.
        public func applicationDidBecomeActive() {
            appActive = true
            reschedulePollingTimer()
            refreshTransportHealth()
        }

        /// Hook `NSApplication.didResignActiveNotification`.
        public func applicationDidResignActive() {
            appActive = false
            reschedulePollingTimer()
        }
    #endif

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
        reloadTransports()
        refreshTransportHealth()
        persistSnapshot()
    }

    private func applyStoredTransportConfig() {
        setMatrixConfig(
            homeserver: matrixHomeserver,
            userId: matrixUserId,
            accessToken: matrixAccessToken,
            roomId: matrixRoomId
        )
        setStalwartConfig(
            baseUrl: stalwartBaseUrl,
            username: stalwartUsername,
            password: stalwartPassword
        )
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
            case "matrix":
                if let room = effectiveMatrixRoomId(), !room.isEmpty {
                    return matrixSendRoomJson(roomId: room, text: text)
                }
                return matrixSendJson(text: text)
            case "stalwart": return stalwartSendJson(text: text)
            default: return "{}"
            }
        }()
        if json.contains("\"error\"") {
            markLastFailed(conv: conv, detail: extractError(json) ?? json)
            persistSnapshot()
            return
        }
        if json.contains("\"accepted\":false") {
            markLastFailed(conv: conv, detail: extractError(json) ?? "rejected")
        } else {
            markLastSent(conv: conv)
        }
        persistSnapshot()
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

    // MARK: - Persistence (`aurorStore`)

    private enum SnapKeys {
        static let root = "hyperchat.snapshot.v1"
    }

    private struct PersistedSnapshot: Codable {
        var conversations: [ConversationItem]
        var selectedConversationId: String?
        var threadMessages: [String: [MessageItem]]
        var draft: String
        var selectedProtocol: String
    }

    private func loadPersistedSnapshot() {
        guard let raw = try? aurorStoreGet(bundleId: bundleId, key: SnapKeys.root),
            let data = raw.data(using: .utf8),
            let snap = try? JSONDecoder().decode(PersistedSnapshot.self, from: data)
        else { return }
        if !snap.conversations.isEmpty { conversations = snap.conversations }
        selectedConversationId = snap.selectedConversationId
        threadMessages = snap.threadMessages
        draft = snap.draft
        selectedProtocol = snap.selectedProtocol
    }

    private func persistSnapshot() {
        let snap = PersistedSnapshot(
            conversations: conversations,
            selectedConversationId: selectedConversationId,
            threadMessages: threadMessages,
            draft: draft,
            selectedProtocol: selectedProtocol
        )
        guard let data = try? JSONEncoder().encode(snap),
            let json = String(data: data, encoding: .utf8)
        else { return }
        try? aurorStoreSet(bundleId: bundleId, key: SnapKeys.root, json: json)
        updateDockBadge()
    }

    #if canImport(AppKit)
        private func updateDockBadge() {
            let n = conversations.reduce(0) { $0 + max(0, $1.unread) }
            NSApp.dockTile.badgeLabel = n > 0 ? "\(n)" : nil
        }
    #else
        private func updateDockBadge() {}
    #endif

    // MARK: - Matrix polling

    private var matrixCredentialsPresent: Bool {
        let hs = matrixHomeserver.trimmingCharacters(in: .whitespacesAndNewlines)
        let tok = matrixAccessToken.trimmingCharacters(in: .whitespacesAndNewlines)
        return !hs.isEmpty && !tok.isEmpty
    }

    private func effectiveMatrixRoomId() -> String? {
        if let s = selectedConversationId, s.contains(":"), s.hasPrefix("!") { return s }
        let envRoom = matrixRoomId.trimmingCharacters(in: .whitespacesAndNewlines)
        return envRoom.isEmpty ? nil : envRoom
    }

    private func reschedulePollingTimer() {
        pollTimer?.invalidate()
        let interval = pollIntervalSeconds
        pollTimer = Timer.scheduledTimer(withTimeInterval: interval, repeats: true) { [weak self] _ in
            self?.pollTransportTick()
        }
        if let t = pollTimer {
            RunLoop.main.add(t, forMode: .common)
        }
        pollTransportTick()
    }

    private var pollIntervalSeconds: TimeInterval {
        #if canImport(AppKit)
            return appActive ? 4 : 30
        #else
            return 6
        #endif
    }

    private func pollTransportTick() {
        guard matrixCredentialsPresent else { return }
        ingestMatrixJoinedRooms()
        ingestMatrixSync()
        persistSnapshot()
    }

    private func ingestMatrixJoinedRooms() {
        let json = matrixJoinedRoomsJson()
        guard let data = json.data(using: .utf8),
            let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
            let joined = obj["joined_rooms"] as? [String]
        else { return }

        var ids = Set(conversations.map(\.id))
        for roomId in joined where !ids.contains(roomId) {
            ids.insert(roomId)
            conversations.append(
                ConversationItem(
                    id: roomId,
                    title: shortRoomTitle(roomId),
                    subtitle: "Matrix",
                    protocolLabel: "Matrix",
                    preview: "",
                    unread: 0
                )
            )
        }
    }

    private func shortRoomTitle(_ id: String) -> String {
        if let colon = id.firstIndex(of: ":") {
            return String(id[..<colon])
        }
        return id
    }

    private func ingestMatrixSync() {
        let json = matrixSyncDeltaJson()
        if json.contains("matrix not configured") || json.contains("\"error\"") { return }
        guard let data = json.data(using: .utf8),
            let root = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
            let rooms = root["rooms"] as? [String: Any],
            let join = rooms["join"] as? [String: Any]
        else { return }

        for (roomId, roomVal) in join {
            guard let roomObj = roomVal as? [String: Any] else { continue }
            if let timeline = roomObj["timeline"] as? [String: Any],
                let events = timeline["events"] as? [[String: Any]]
            {
                for ev in events {
                    ingestTimelineEvent(roomId: roomId, ev: ev)
                }
            }
            if let ephemeral = roomObj["ephemeral"] as? [String: Any],
                let events = ephemeral["events"] as? [[String: Any]]
            {
                for ev in events {
                    ingestEphemeralEvent(roomId: roomId, ev: ev)
                }
            }
        }
        refreshTypingLine()
    }

    private func ingestTimelineEvent(roomId: String, ev: [String: Any]) {
        guard ev["type"] as? String == "m.room.message" else { return }
        let eventId = ev["event_id"] as? String ?? ""
        if !eventId.isEmpty {
            let dedupeKey = "\(roomId)|\(eventId)"
            if seenMatrixEvents.contains(dedupeKey) { return }
            seenMatrixEvents.insert(dedupeKey)
        }
        let content = ev["content"] as? [String: Any]
        let body = content?["body"] as? String ?? ""
        let sender = ev["sender"] as? String ?? ""
        let outgoing = normalizeUser(sender) == normalizeUser(matrixUserId)
        let msg = MessageItem(
            id: eventId.isEmpty ? UUID().uuidString : eventId,
            body: body,
            metaLine: formatMeta(sender: sender),
            isOutgoing: outgoing
        )
        appendMatrixMessage(roomId: roomId, msg: msg)
    }

    private func ingestEphemeralEvent(roomId: String, ev: [String: Any]) {
        guard ev["type"] as? String == "m.typing" else { return }
        let content = ev["content"] as? [String: Any]
        let ids = content?["user_ids"] as? [String] ?? []
        typingUserIdsByRoom[roomId] = ids
    }

    private func normalizeUser(_ s: String) -> String {
        s.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
    }

    private func formatMeta(sender: String) -> String {
        let trimmed = sender.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty { return "Matrix" }
        if let colon = trimmed.firstIndex(of: ":") {
            return String(trimmed[..<colon])
        }
        return trimmed
    }

    private func appendMatrixMessage(roomId: String, msg: MessageItem) {
        var list = threadMessages[roomId] ?? []
        if list.contains(where: { $0.id == msg.id }) { return }
        list.append(msg)
        threadMessages[roomId] = list

        updateConversationPreview(roomId: roomId, body: msg.body)

        if selectedConversationId != roomId, !msg.isOutgoing {
            bumpUnread(roomId: roomId)
            notifyIncoming(title: "HyperChat", body: msg.body, roomId: roomId)
        }
    }

    private func updateConversationPreview(roomId: String, body: String) {
        guard let idx = conversations.firstIndex(where: { $0.id == roomId }) else { return }
        var c = conversations[idx]
        c.preview = body
        c.timeAgo = "now"
        conversations[idx] = c
    }

    private func bumpUnread(roomId: String) {
        guard let idx = conversations.firstIndex(where: { $0.id == roomId }) else { return }
        var c = conversations[idx]
        c.unread += 1
        conversations[idx] = c
    }

    private func clearUnread(for roomId: String) {
        guard let idx = conversations.firstIndex(where: { $0.id == roomId }) else { return }
        var c = conversations[idx]
        c.unread = 0
        conversations[idx] = c
        persistSnapshot()
    }

    private func refreshTypingLine() {
        guard let rid = selectedConversationId,
            let ids = typingUserIdsByRoom[rid], !ids.isEmpty
        else {
            typingLine = ""
            return
        }
        let names = ids.map { formatMeta(sender: $0) }.joined(separator: ", ")
        typingLine = "\(names) typing…"
    }

    private func scheduleTypingPing() {
        typingTimer?.invalidate()
        guard sanitizeProtocol(selectedProtocol) == "matrix",
            let room = effectiveMatrixRoomId(), !room.isEmpty
        else { return }
        _ = matrixTypingJson(roomId: room, typing: true)
        typingTimer = Timer.scheduledTimer(withTimeInterval: 3.0, repeats: false) { [weak self] _ in
            guard let self, let room = self.effectiveMatrixRoomId(), !room.isEmpty else { return }
            _ = matrixTypingJson(roomId: room, typing: false)
        }
        if let t = typingTimer {
            RunLoop.main.add(t, forMode: .common)
        }
    }

    private func requestNotificationsIfNeeded() {
        #if canImport(UserNotifications)
            UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound]) { _, _ in }
        #endif
    }

    private func notifyIncoming(title: String, body: String, roomId: String) {
        #if canImport(UserNotifications) && canImport(AppKit)
            guard !appActive else { return }
            let content = UNMutableNotificationContent()
            content.title = title
            content.body = body
            content.userInfo = ["roomId": roomId]
            let req = UNNotificationRequest(identifier: UUID().uuidString, content: content, trigger: nil)
            UNUserNotificationCenter.current().add(req)
        #endif
    }

    private func archiveSelectedConversation() {
        guard let idx = conversations.firstIndex(where: { $0.id == selectedConversationId }) else { return }
        conversations.remove(at: idx)
        if let sel = selectedConversationId {
            threadMessages.removeValue(forKey: sel)
        }
        selectedConversationId = conversations.first?.id
        persistSnapshot()
    }

    private func pickAttachment() {
        #if canImport(AppKit)
            guard sanitizeProtocol(selectedProtocol) == "matrix",
                let room = effectiveMatrixRoomId(), !room.isEmpty
            else { return }
            let panel = NSOpenPanel()
            panel.allowsMultipleSelection = false
            panel.canChooseDirectories = false
            guard panel.runModal() == .OK, let url = panel.url else { return }
            guard let data = try? Data(contentsOf: url) else { return }
            let b64 = data.base64EncodedString()
            let mime = mimeForPathExtension(url.pathExtension)
            let filename = url.lastPathComponent
            let json = matrixUploadMediaJson(
                roomId: room,
                filename: filename,
                mime: mime,
                dataBase64: b64
            )
            if json.contains("\"error\"") {
                if let conv = selectedConversationId {
                    appendFailed(
                        conv: conv,
                        text: filename,
                        meta: "Upload failed — \(extractError(json) ?? json)"
                    )
                }
                persistSnapshot()
                return
            }
            guard let conv = selectedConversationId else { return }
            let optimistic = MessageItem(
                id: UUID().uuidString,
                body: "📎 \(filename)",
                metaLine: "Matrix · attachment sent",
                isOutgoing: true
            )
            threadMessages[conv, default: []].append(optimistic)
            persistSnapshot()
        #endif
    }

    private func mimeForPathExtension(_ ext: String) -> String {
        switch ext.lowercased() {
        case "jpg", "jpeg": return "image/jpeg"
        case "png": return "image/png"
        case "gif": return "image/gif"
        case "webp": return "image/webp"
        case "pdf": return "application/pdf"
        case "txt": return "text/plain"
        default: return "application/octet-stream"
        }
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
