import Foundation

/// Read-only snapshot passed into `HyperChatGeneratedView` (`swiftgen` maps `{field}` → `context.field`).
public struct HyperChatContext: Sendable {
    public var conversations: [ConversationItem]
    public var selectedConversationId: String?
    public var recipientTitle: String
    public var protocolSummary: String
    public var headerBadge: String
    public var fallbackLabel: String
    public var messages: [MessageItem]
    public var draft: String
    public var selectedProtocol: String
    public var matrixStatusLine: String
    public var stalwartStatusLine: String
    public var bitchatStatusLine: String
    public var canSend: Bool
    public var newConversationPrompt: Bool
    /// `true` when Bitchat is selected — sends are blocked in this demo.
    public var bitchatSendBlocked: Bool
    /// Tooltip / accessibility hint when Send is disabled.
    public var sendDisabledHint: String?

    public init(
        conversations: [ConversationItem] = [],
        selectedConversationId: String? = nil,
        recipientTitle: String = "",
        protocolSummary: String = "",
        headerBadge: String = "",
        fallbackLabel: String = "",
        messages: [MessageItem] = [],
        draft: String = "",
        selectedProtocol: String = "",
        matrixStatusLine: String = "",
        stalwartStatusLine: String = "",
        bitchatStatusLine: String = "",
        canSend: Bool = false,
        newConversationPrompt: Bool = false,
        bitchatSendBlocked: Bool = false,
        sendDisabledHint: String? = nil
    ) {
        self.conversations = conversations
        self.selectedConversationId = selectedConversationId
        self.recipientTitle = recipientTitle
        self.protocolSummary = protocolSummary
        self.headerBadge = headerBadge
        self.fallbackLabel = fallbackLabel
        self.messages = messages
        self.draft = draft
        self.selectedProtocol = selectedProtocol
        self.matrixStatusLine = matrixStatusLine
        self.stalwartStatusLine = stalwartStatusLine
        self.bitchatStatusLine = bitchatStatusLine
        self.canSend = canSend
        self.newConversationPrompt = newConversationPrompt
        self.bitchatSendBlocked = bitchatSendBlocked
        self.sendDisabledHint = sendDisabledHint
    }
}

public struct ConversationItem: Identifiable, Hashable, Sendable {
    public let id: String
    public var title: String
    public var subtitle: String
    public var protocolLabel: String

    public init(id: String, title: String, subtitle: String, protocolLabel: String) {
        self.id = id
        self.title = title
        self.subtitle = subtitle
        self.protocolLabel = protocolLabel
    }
}

public struct MessageItem: Identifiable, Hashable, Sendable {
    public let id: String
    public var body: String
    public var metaLine: String
    public var isOutgoing: Bool

    public init(id: String, body: String, metaLine: String, isOutgoing: Bool) {
        self.id = id
        self.body = body
        self.metaLine = metaLine
        self.isOutgoing = isOutgoing
    }
}
