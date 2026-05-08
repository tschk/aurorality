/// textanalyzer — combined Rust + Swift plugin example.
///
/// Rust side  (aurorality-core, StatsPlugin):
///   bridge.invoke("stats", "analyze", payload) → word/char counts, top word
///
/// Swift side (HistoryPlugin, this file):
///   bridge.invoke("history", "push", payload)  → persists entry in memory
///
/// Both plugins are registered in AurorBridge and called from the same
/// Swift view. The .crepus template renders whatever context we pass in.

import SwiftUI
import Aurorality

// MARK: - Response types

struct StatsResult: Decodable {
    let wordCount: Int
    let charCount: Int
    let lineCount: Int
    let topWord: String
    let topWordCount: Int
}

struct HistoryItem: Decodable {
    let preview: String
    let words: Int
}

struct HistoryList: Decodable {
    let items: [HistoryItem]
}

// MARK: - App

@main
struct TextAnalyzerApp: App {
    @State private var bridge = {
        let b = AurorBridge()
        // Register our Swift plugin. The Rust StatsPlugin is already
        // built into AurorBridge's default initialiser (CorePlugin, AppPlugin, StatsPlugin).
        b.register(HistoryPlugin())
        return b
    }()
    @State private var state = AurorState()

    var body: some Scene {
        WindowGroup {
            AnalyzerView(bridge: bridge, state: state)
                .environment(bridge)
        }
    }
}

// MARK: - View

struct AnalyzerView: View {
    let bridge: AurorBridge
    let state: AurorState

    @State private var inputText = "The quick brown fox jumps over the lazy dog. The fox was very quick."
    @State private var stats: StatsResult?
    @State private var history: [HistoryItem] = []

    var body: some View {
        VStack(spacing: 0) {
            // Input area (native SwiftUI — not driven by .crepus)
            TextEditor(text: $inputText)
                .frame(height: 120)
                .padding(12)
                .background(Color(.secondarySystemBackground))
                .cornerRadius(8)
                .padding()

            Button("Analyze") { analyze() }
                .buttonStyle(.borderedProminent)
                .padding(.bottom)

            // Results rendered from the .crepus template
            AurorRootView(state: state)
                .padding(.horizontal)

            Spacer()
        }
        .task {
            try? loadScriptPlugin(id: "textJs", script: "backend")
            loadTemplate()
        }
    }

    // MARK: - Actions

    private func analyze() {
        guard !inputText.trimmingCharacters(in: .whitespaces).isEmpty else { return }

        // 1. Call the Rust StatsPlugin to compute statistics.
        let payload = encodePayload(["text": inputText])
        guard let result = try? bridge.invokeData(pluginId: "stats", method: "analyze", payload: payload, as: StatsResult.self)
        else { return }
        stats = result

        // 2. Call the Swift HistoryPlugin to record this analysis.
        let preview = String(inputText.prefix(40)).replacingOccurrences(of: "\n", with: " ")
        let histPayload = encodePayload(["preview": preview, "words": result.wordCount])
        _ = try? bridge.invoke(pluginId: "history", method: "push", payload: histPayload)

        // 3. Fetch updated history from the Swift plugin.
        if let listJson = try? bridge.invoke(pluginId: "history", method: "list", payload: "{}"),
           let list = decode(HistoryList.self, from: listJson) {
            history = list.items
        }

        // 4. Re-render the .crepus template with fresh context.
        rerender(stats: result)
    }

    private func rerender(stats: StatsResult) {
let url = Bundle.main.url(forResource: "main", withExtension: "crepus")
let template = url.flatMap { try? String(contentsOf: $0) } ?? "No template found"
        let js = (try? bridge.invokeData(
            pluginId: "textJs",
            method: "score",
            payload: encodePayload(["wordCount": stats.wordCount, "charCount": stats.charCount]),
            as: TextScore.self
        )) ?? TextScore(density: 0, readability: "unknown", summary: "waiting for JavaScript scoring")

        try? state.load(template: template, context: [
            "wordCount": .int(stats.wordCount),
            "charCount": .int(stats.charCount),
            "topWord":   .string(stats.topWord.isEmpty ? "—" : stats.topWord),
            "density":   .int(js.density),
            "readability": .string(js.readability),
            "summary":   .string(js.summary),
            "history":   .list(history.map { ["preview": .string($0.preview), "words": .int($0.words)] }),
        ])
    }

    private func loadTemplate() {
let url = Bundle.main.url(forResource: "main", withExtension: "crepus")
let template = url.flatMap { try? String(contentsOf: $0) } ?? "No template found"
        try? state.load(template: template, context: [
            "wordCount": .int(0),
            "charCount": .int(0),
            "topWord":   .string("—"),
            "density":   .int(0),
            "readability": .string("waiting"),
            "summary":   .string("JavaScript scoring appears after the first analysis"),
            "history":   .list([]),
        ])
    }

    // MARK: - Helpers

    private func encodePayload(_ dict: [String: Any]) -> String {
        guard let data = try? JSONSerialization.data(withJSONObject: dict),
              let json = String(data: data, encoding: .utf8)
        else { return "{}" }
        return json
    }

    private func decode<T: Decodable>(_ type: T.Type, from json: String) -> T? {
        try? JSONDecoder().decode(type, from: Data(json.utf8))
    }

    private func loadScriptPlugin(id: String, script: String) throws {
        guard let url = Bundle.main.url(forResource: script, withExtension: "js", subdirectory: "scripts") else {
            throw AurorPluginError("missing scripts/\(script).js")
        }
        try loadJsPlugin(id: id, code: String(contentsOf: url, encoding: .utf8))
    }
}

struct TextScore: Decodable {
    let density: Int
    let readability: String
    let summary: String
}
