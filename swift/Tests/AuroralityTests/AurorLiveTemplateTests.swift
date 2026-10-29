import Foundation
import XCTest
@testable import Aurorality

/// In-process hot reload: the app re-renders its own template on save, with its
/// own context, and no server.
@MainActor
final class AurorLiveTemplateTests: XCTestCase {
    private var dir: URL!
    private var lastState: AurorState?

    override func setUp() async throws {
        dir = FileManager.default.temporaryDirectory.appendingPathComponent("aur-live-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        setenv("AURORALITY_VIEWS_DIR", dir.path, 1)
        setenv("AURORALITY_LIVE_MARKER", dir.appendingPathComponent("live-marker").path, 1)
    }

    override func tearDown() async throws {
        unsetenv("AURORALITY_VIEWS_DIR")
        unsetenv("AURORALITY_LIVE_MARKER")
        try? FileManager.default.removeItem(at: dir)
    }

    func testSavesReRenderWithTheAppsOwnContext() async throws {
        let file = dir.appendingPathComponent("main.crepus")
        try "span text-base\n  \"Hello {title}\"\n".write(to: file, atomically: false, encoding: .utf8)

        let state = AurorState()
        lastState = state
        try state.load(template: AurorTemplates.source("main"), context: ["title": .string("Ctx")])
        XCTAssertTrue(state.watchLiveTemplate("main"))
        XCTAssertEqual(text(state), "Hello Ctx")
        XCTAssertTrue(FileManager.default.fileExists(atPath: dir.appendingPathComponent("live-marker").path))

        // In-place write.
        try "span text-base\n  \"Edited {title}\"\n".write(to: file, atomically: false, encoding: .utf8)
        try await waitFor { self.text(state) == "Edited Ctx" }

        // Atomic save (rename over the file), as most editors do; then a later
        // context change from the app is kept for the next save.
        try "span text-base\n  \"Renamed {title}\"\n".write(to: file, atomically: true, encoding: .utf8)
        try await waitFor { self.text(state) == "Renamed Ctx" }
        try state.load(template: AurorTemplates.source("main"), context: ["title": .string("New")])
        try "span text-base\n  \"Again {title}\"\n".write(to: file, atomically: true, encoding: .utf8)
        try await waitFor { self.text(state) == "Again New" }

        // A broken save reports an error instead of crashing; fixing it recovers.
        try "<span>\n".write(to: file, atomically: true, encoding: .utf8)
        try await waitFor { state.error != nil }
        try "span text-base\n  \"Fixed {title}\"\n".write(to: file, atomically: true, encoding: .utf8)
        try await waitFor { self.text(state) == "Fixed New" && state.error == nil }
    }

    func testWithoutDevEnvironmentTheBundleIsUsedAndNothingIsWatched() throws {
        unsetenv("AURORALITY_VIEWS_DIR")
        XCTAssertNil(AurorTemplates.liveURL("main"))
        XCTAssertFalse(AurorState().watchLiveTemplate("main"))
        XCTAssertThrowsError(try AurorTemplates.source("definitely-missing"))
    }

    private func text(_ state: AurorState) -> String? {
        state.ir.root.first?.content
    }

    private func waitFor(_ what: String = #function, line: UInt = #line, _ condition: () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(5)
        while Date() < deadline {
            if condition() { return }
            try await Task.sleep(nanoseconds: 50_000_000)
        }
        XCTFail("timed out; ir=\(String(describing: lastState?.ir.root.first?.content)) error=\(String(describing: lastState?.error))", line: line)
    }
}
