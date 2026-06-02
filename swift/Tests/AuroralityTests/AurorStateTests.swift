import XCTest
@testable import Aurorality

final class AurorStateTests: XCTestCase {

    func testLoadTemplate() throws {
        let state = AurorState()

        let template = "<text>Hello {name}</text>"

        try state.load(template: template, context: ["name": .string("World")])

        XCTAssertEqual(state.ir.root.count, 1)
        XCTAssertNil(state.error)

        if case let .text(textNode) = state.ir.root.first?.kind {
            XCTAssertEqual(textNode.text, "Hello World")
        } else {
            XCTFail("Expected a text node")
        }
    }

    func testLoadTemplateEmptyContext() throws {
        let state = AurorState()

        let template = "<text>Static Text</text>"

        try state.load(template: template)

        XCTAssertEqual(state.ir.root.count, 1)
        XCTAssertNil(state.error)

        if case let .text(textNode) = state.ir.root.first?.kind {
            XCTAssertEqual(textNode.text, "Static Text")
        } else {
            XCTFail("Expected a text node")
        }
    }

    func testReload() throws {
        let state = AurorState()

        let template = "<text>Count: {count}</text>"

        try state.load(template: template, context: ["count": .int(1)])

        if case let .text(textNode) = state.ir.root.first?.kind {
            XCTAssertEqual(textNode.text, "Count: 1")
        } else {
            XCTFail("Expected a text node")
        }

        try state.reload(context: ["count": .int(2)])

        if case let .text(textNode) = state.ir.root.first?.kind {
            XCTAssertEqual(textNode.text, "Count: 2")
        } else {
            XCTFail("Expected a text node")
        }
    }

    func testMalformedTemplate() throws {
        let state = AurorState()

        let template = "<text>Unclosed"

        do {
            try state.load(template: template)
            XCTFail("Expected an error to be thrown for malformed template")
        } catch {
            // Success: an error was thrown
        }
    }
}
