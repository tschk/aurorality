import Foundation
#if canImport(Aurorality)
import Aurorality
#endif

struct ChatMessage: Codable, Identifiable, Equatable {
    let id: String
    let text: String
    let transport: String
    let status: String
}

final class ChatStorePlugin: AurorPlugin {
    let id = "chatStore"
    private var messages: [ChatMessage] = []

    func invoke(method: String, payload: String) throws -> String {
        let args = (try? JSONDecoder().decode([String: AnyCodable].self, from: Data(payload.utf8))) ?? [:]

        switch method {
        case "send":
            let text = args["text"]?.stringValue ?? ""
            let transport = args["transport"]?.stringValue ?? "matrix"
            guard !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
                return encode(["accepted": false, "count": messages.count])
            }
            let message = ChatMessage(
                id: "msg-\(Date().timeIntervalSince1970)",
                text: text,
                transport: transport,
                status: status(for: transport)
            )
            messages.insert(message, at: 0)
            if messages.count > 24 { messages.removeLast() }
            return encode(["accepted": true, "count": messages.count])

        case "list":
            return encode(["items": messages.map(messageJson)])

        case "clear":
            messages.removeAll()
            return encode(["items": []])

        default:
            throw AurorPluginError("unknown method: \(method)")
        }
    }

    private func status(for transport: String) -> String {
        switch transport {
        case "matrix": return "federating"
        case "stalwart": return "archived"
        default: return "routed"
        }
    }

    private func messageJson(_ message: ChatMessage) -> [String: Any] {
        [
            "id": message.id,
            "text": message.text,
            "transport": message.transport,
            "status": message.status,
        ]
    }

    private func encode(_ value: Any) -> String {
        guard let data = try? JSONSerialization.data(withJSONObject: value),
              let json = String(data: data, encoding: .utf8)
        else { return "{}" }
        return json
    }
}

struct AnyCodable: Codable {
    let value: Any

    var stringValue: String? { value as? String }

    init(from decoder: Decoder) throws {
        let c = try decoder.singleValueContainer()
        if let s = try? c.decode(String.self) { value = s; return }
        if let i = try? c.decode(Int.self) { value = i; return }
        if let d = try? c.decode(Double.self) { value = d; return }
        if let b = try? c.decode(Bool.self) { value = b; return }
        value = NSNull()
    }

    func encode(to encoder: Encoder) throws {
        var c = encoder.singleValueContainer()
        switch value {
        case let s as String: try c.encode(s)
        case let i as Int: try c.encode(i)
        case let d as Double: try c.encode(d)
        case let b as Bool: try c.encode(b)
        default: try c.encodeNil()
        }
    }
}
