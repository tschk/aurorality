import Foundation
import XCTest
@testable import Aurorality

/// End to end: a real `aurorality dev` process serves a temp project; the real
/// `HotReloadClient` drives an `AurorState`; saving the template must update it
/// through the patch path.
///
/// Needs `AURORALITY_CLI` (path to the built `aurorality` binary); skipped otherwise.
@MainActor
final class HotReloadEndToEndTests: XCTestCase {
    private var dev: Process?
    private var observer: URLSessionWebSocketTask?
    private var tempDir: URL?

    override func tearDown() async throws {
        observer?.cancel(with: .normalClosure, reason: nil)
        dev?.terminate()
        dev?.waitUntilExit()
        if let tempDir { try? FileManager.default.removeItem(at: tempDir) }
    }

    func testSavingATemplatePatchesTheClientState() async throws {
        guard let cli = ProcessInfo.processInfo.environment["AURORALITY_CLI"] else {
            throw XCTSkip("set AURORALITY_CLI to the aurorality binary")
        }
        let fm = FileManager.default
        let root = fm.temporaryDirectory.appendingPathComponent("aur-e2e-\(UUID().uuidString)")
        tempDir = root
        let bin = root.appendingPathComponent("bin")
        let project = root.appendingPathComponent("proj")
        try fm.createDirectory(at: bin, withIntermediateDirectories: true)
        try fm.createDirectory(at: project.appendingPathComponent("views"), withIntermediateDirectories: true)
        // Fake toolchain: the Rust pre-build succeeds and the Swift build fails, so
        // `dev` serves hot reload without launching an app.
        for (name, body) in [("cargo", "#!/bin/sh\nexit 0\n"), ("swift", "#!/bin/sh\nexit 1\n")] {
            let url = bin.appendingPathComponent(name)
            try body.write(to: url, atomically: true, encoding: .utf8)
            try fm.setAttributes([.posixPermissions: 0o755], ofItemAtPath: url.path)
        }
        try "let package = Package(\n    name: \"AurorE2EFixture\",\n)\n"
            .write(to: project.appendingPathComponent("Package.swift"), atomically: true, encoding: .utf8)
        let template = project.appendingPathComponent("views/main.crepus")
        try "div flex flex-col\n  span text-base\n    \"first\"\n"
            .write(to: template, atomically: true, encoding: .utf8)

        let port = UInt16.random(in: 40000...59000)
        let process = Process()
        process.executableURL = URL(fileURLWithPath: cli)
        process.arguments = ["dev", "--port", String(port)]
        process.currentDirectoryURL = project
        var env = ProcessInfo.processInfo.environment
        env["CARGO"] = bin.appendingPathComponent("cargo").path
        env["SWIFT_PATH"] = bin.appendingPathComponent("swift").path
        process.environment = env
        process.standardOutput = FileHandle.nullDevice
        process.standardError = FileHandle.nullDevice
        try process.run()
        dev = process

        // Independent raw listener records which message kinds the server sends.
        let kinds = KindLog()
        let url = URL(string: "ws://127.0.0.1:\(port)")!
        try await waitFor("server listening", timeout: 15) {
            await Self.canConnect(url)
        }
        let task = URLSession.shared.webSocketTask(with: url)
        observer = task
        task.resume()
        Self.record(task, into: kinds)

        let state = AurorState()
        let client = HotReloadClient()
        client.connect(port: port, state: state)

        try await waitFor("devHello + initial IR", timeout: 10) {
            HotReloadBus.shared.lastDevHelloSessionId != nil && Self.text(of: state.ir).contains("first")
        }
        XCTAssertEqual(HotReloadBus.shared.irEnabledFromServer, true)

        try "div flex flex-col\n  span text-base\n    \"second\"\n"
            .write(to: template, atomically: true, encoding: .utf8)

        try await waitFor("patched IR", timeout: 10) {
            Self.text(of: state.ir).contains("second")
        }
        XCTAssertFalse(Self.text(of: state.ir).contains("first"))
        XCTAssertNil(state.error)
        let seen = await kinds.all
        XCTAssertEqual(Array(seen.prefix(2)), ["devHello", "fullReload"], "\(seen)")
        XCTAssertTrue(seen.contains("patch"), "text-only edit should be a patch: \(seen)")
        client.disconnect()
    }

    // MARK: - Helpers

    private actor KindLog {
        var all: [String] = []
        func add(_ kind: String) { all.append(kind) }
    }

    nonisolated private static func record(_ task: URLSessionWebSocketTask, into log: KindLog) {
        task.receive { result in
            guard case .success(.string(let text)) = result else { return }
            if let data = text.data(using: .utf8),
               let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
               let msg = obj["message"] as? [String: Any],
               let kind = msg["kind"] as? String {
                Task { await log.add(kind) }
            }
            record(task, into: log)
        }
    }

    nonisolated private static func canConnect(_ url: URL) async -> Bool {
        let probe = URLSession.shared.webSocketTask(with: url)
        probe.resume()
        defer { probe.cancel(with: .normalClosure, reason: nil) }
        return (try? await probe.receive()) != nil
    }

    private static func text(of ir: ViewIr) -> String {
        (try? String(data: JSONEncoder().encode(ir), encoding: .utf8)) ?? nil ?? ""
    }

    private func waitFor(
        _ what: String, timeout: TimeInterval, _ condition: () async -> Bool
    ) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if await condition() { return }
            try await Task.sleep(nanoseconds: 100_000_000)
        }
        XCTFail("timed out waiting for \(what)")
        throw XCTSkip("aborting after timeout")
    }
}
