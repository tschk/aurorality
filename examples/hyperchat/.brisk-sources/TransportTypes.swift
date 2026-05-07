import Foundation

/// Health status reported by a transport adapter.
public struct TransportHealth: Decodable, Sendable {
    public let id: String
    public let name: String
    public let role: String
    public let connected: Bool
    public let latencyMs: UInt64
    public let lastError: String?

    enum CodingKeys: String, CodingKey {
        case id, name, role, connected
        case latencyMs = "latency_ms"
        case lastError = "last_error"
    }
}

/// Metadata for transport discovery / dashboard display.
public struct TransportInfo: Decodable, Sendable {
    public let id: String
    public let name: String
    public let role: String
    public let trust: String
    public let latency: UInt64

    public init(id: String, name: String, role: String, trust: String, latency: UInt64) {
        self.id = id
        self.name = name
        self.role = role
        self.trust = trust
        self.latency = latency
    }
}

/// Message exchanged between transports.
public struct TransportMessage: Decodable, Identifiable, Sendable {
    public let id: String
    public let text: String
    public let transport: String
    public let status: String
}

/// Result of sending a message through a transport.
public struct SendResult: Decodable, Sendable {
    public let accepted: Bool
    public let messageId: String?
    public let transportMessage: String?

    enum CodingKeys: String, CodingKey {
        case accepted
        case messageId = "message_id"
        case transportMessage = "transport_message"
    }
}
