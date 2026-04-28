/// Pure Swift plugin — keeps an in-memory history of analyzed texts.
///
/// This is the Swift side of the Rust+Swift split:
/// - StatsPlugin (Rust) does the computation
/// - HistoryPlugin (Swift) manages app-level state
///
/// Methods:
///   push  { "preview": "...", "words": 42 }  → { "count": N }
///   list  {}                                  → { "items": [...] }
///   clear {}                                  → { "count": 0 }

import Foundation
import Aurorality

struct HistoryEntry: Codable {
    let preview: String
    let words: Int
}

final class HistoryPlugin: AurorPlugin {
    let id = "history"
    private var entries: [HistoryEntry] = []

    func invoke(method: String, payload: String) throws -> String {
        let args = try JSONDecoder().decode([String: AnyCodable].self, from: Data(payload.utf8))

        switch method {
        case "push":
            let preview = args["preview"]?.stringValue ?? ""
            let words   = args["words"]?.intValue ?? 0
            entries.insert(HistoryEntry(preview: preview, words: words), at: 0)
            if entries.count > 10 { entries.removeLast() }
            return encode(["count": entries.count])

        case "list":
            let items = entries.map { ["preview": $0.preview, "words": $0.words] as [String: Any] }
            return encode(["items": items])

        case "clear":
            entries.removeAll()
            return encode(["count": 0])

        default:
            throw AurorPluginError("unknown method: \(method)")
        }
    }

    private func encode(_ value: Any) -> String {
        let data = try! JSONSerialization.data(withJSONObject: value)
        return String(data: data, encoding: .utf8)!
    }
}

// Minimal Codable wrapper for heterogeneous JSON values.
struct AnyCodable: Codable {
    let value: Any

    var stringValue: String? { value as? String }
    var intValue: Int? {
        if let i = value as? Int { return i }
        if let d = value as? Double { return Int(d) }
        return nil
    }

    init(_ value: Any) { self.value = value }

    init(from decoder: Decoder) throws {
        let c = try decoder.singleValueContainer()
        if let s = try? c.decode(String.self)  { value = s; return }
        if let i = try? c.decode(Int.self)     { value = i; return }
        if let d = try? c.decode(Double.self)  { value = d; return }
        if let b = try? c.decode(Bool.self)    { value = b; return }
        value = NSNull()
    }

    func encode(to encoder: Encoder) throws {
        var c = encoder.singleValueContainer()
        switch value {
        case let s as String: try c.encode(s)
        case let i as Int:    try c.encode(i)
        case let d as Double: try c.encode(d)
        case let b as Bool:   try c.encode(b)
        default:              try c.encodeNil()
        }
    }
}
