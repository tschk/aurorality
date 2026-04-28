/// Pure Swift plugin — manages counter state.
/// Demonstrates registering a Swift-native plugin alongside built-in Rust plugins.

import Foundation
import Aurorality

final class CounterPlugin: AurorPlugin {
    let id = "counter"
    private var count = 0

    func invoke(method: String, payload: String) throws -> String {
        switch method {
        case "increment": count += 1
        case "decrement": count -= 1
        case "reset":     count  = 0
        default:
            throw AurorPluginError("unknown method: \(method)")
        }
        let result: [String: Any] = ["count": count]
        let data = try JSONSerialization.data(withJSONObject: result)
        return String(data: data, encoding: .utf8)!
    }
}
