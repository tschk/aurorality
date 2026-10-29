/// Observable state holding the current rendered ViewIr.

import Foundation

@Observable
public final class AurorState {
    public var ir: ViewIr = .empty
    public var error: String?

    private var rawTemplate: String = ""
    /// Context of the last `load`, reused when a live template is re-rendered.
    private(set) var lastContext: [String: ContextValue] = [:]
    /// Live-template watchers started by `watchLiveTemplate` (kept alive with the state).
    @ObservationIgnored var liveWatchers: [String: AurorTemplateWatcher] = [:]

    public init() {}

    // MARK: - Load from template string

    /// Render a `.crepus` template with an optional context dict and update `ir`.
    public func load(template: String, context: [String: ContextValue] = [:]) throws {
        rawTemplate = template
        lastContext = context
        let contextJson = try encodeContext(context)
        let irJson = try renderTemplate(template: template, contextJson: contextJson)
        ir = try JSONDecoder().decode(ViewIr.self, from: Data(irJson.utf8))
        error = nil
    }

    /// Reload with the last template and a new context.
    public func reload(context: [String: ContextValue] = [:]) throws {
        guard !rawTemplate.isEmpty else { return }
        try load(template: rawTemplate, context: context)
    }

    // MARK: - Load from bundled JSON IR (production)

    /// Load a pre-compiled `ViewIr` JSON file from the app bundle.
    public func loadBundled(name: String, bundle: Bundle = .main) throws {
        guard let url = bundle.url(forResource: name, withExtension: "json") else {
            throw AurorPluginError("bundled IR file not found: \(name).json")
        }
        let data = try Data(contentsOf: url)
        ir = try JSONDecoder().decode(ViewIr.self, from: data)
        error = nil
    }

    // MARK: - Mutation application (called by HotReloadClient)

    func apply(_ message: HotReloadMessage) {
        switch message.kind {
        case .noop:
            break
        case .patch:
            if let mutations = message.mutations {
                ir = ir.applying(mutations)
            }
        case .fullReload:
            if let newIr = message.ir {
                ir = newIr
            }
        case .error:
            error = message.message
        case .devHello, .swiftgenStatus:
            break
        }
    }
}

// MARK: - Context encoding

private func encodeContext(_ context: [String: ContextValue]) throws -> String {
    let raw = context.mapValues { $0.toJson() }
    let data = try JSONSerialization.data(withJSONObject: raw)
    return String(data: data, encoding: .utf8) ?? "{}"
}
