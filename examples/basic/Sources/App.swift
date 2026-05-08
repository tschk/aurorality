import SwiftUI
import Aurorality

@main
struct BasicApp: App {
    @State private var state = AurorState()
    @State private var bridge = {
        let bridge = AurorBridge()
        bridge.register(BasicSessionPlugin())
        return bridge
    }()

    var body: some Scene {
        WindowGroup {
            AurorRootView(state: state)
                .environment(bridge)
                .task { load() }
        }
    }

    private func load() {
        try? loadScriptPlugin(id: "basicJs", script: "backend")

        let platform = (try? bridge.invokeData(pluginId: "app", method: "platform", as: PlatformInfo.self))
            ?? PlatformInfo(os: "unknown", arch: "unknown")
        let version = (try? bridge.invokeData(pluginId: "app", method: "version", as: VersionInfo.self))
            ?? VersionInfo(aurorality: "dev", plugin: "app")
        let sessionJson = try? bridge.invoke(pluginId: "session", method: "describe")
        let session = sessionJson.flatMap { decode(SessionInfo.self, from: $0) }
            ?? SessionInfo(owner: "Swift", mode: "local state")
        let jsPayload = json([
            "platform": ["os": platform.os, "arch": platform.arch],
            "version": ["aurorality": version.aurorality],
        ])
        let copy = (try? bridge.invokeData(pluginId: "basicJs", method: "describe", payload: jsPayload, as: BasicCopy.self))
            ?? BasicCopy(headline: "Hello from aurorality", detail: "SwiftUI + Rust + JavaScript", badge: "three backends")

        let url = Bundle.main.url(forResource: "main", withExtension: "crepus")
        let template = url.flatMap { try? String(contentsOf: $0) } ?? "No template found"
        try? state.load(template: template, context: [
            "headline": .string(copy.headline),
            "detail": .string(copy.detail),
            "badge": .string(copy.badge),
            "sessionOwner": .string(session.owner),
            "sessionMode": .string(session.mode),
        ])
    }
}

struct PlatformInfo: Decodable { let os: String; let arch: String }
struct VersionInfo: Decodable { let aurorality: String; let plugin: String }
struct BasicCopy: Decodable { let headline: String; let detail: String; let badge: String }
struct SessionInfo: Decodable { let owner: String; let mode: String }

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
