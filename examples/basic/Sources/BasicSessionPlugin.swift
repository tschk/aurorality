import Foundation
import Aurorality

final class BasicSessionPlugin: AurorPlugin {
    let id = "session"

    func invoke(method: String, payload: String) throws -> String {
        switch method {
        case "describe":
            return encode([
                "owner": "Swift",
                "mode": "app-local state",
            ])
        default:
            throw AurorPluginError("unknown method: \(method)")
        }
    }

    private func encode(_ value: Any) -> String {
        guard let data = try? JSONSerialization.data(withJSONObject: value),
              let json = String(data: data, encoding: .utf8)
        else { return "{}" }
        return json
    }
}
