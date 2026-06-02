import XCTest
@testable import Aurorality

final class AurorStateTests: XCTestCase {

    func testLoadTemplate() throws {
        let state = AurorState()

        let template = """
        span
          "Hello {name}"
        """

        try state.load(template: template, context: ["name": .string("World")])

        XCTAssertEqual(state.ir.root.count, 1)
        XCTAssertNil(state.error)

        if let node = state.ir.root.first, node.kind == .text {
            XCTAssertEqual(node.content, "Hello World")
        } else {
            XCTFail("Expected a text node")
        }
    }

    func testLoadTemplateEmptyContext() throws {
        let state = AurorState()

        let template = """
        span
          "Static Text"
        """

        try state.load(template: template)

        XCTAssertEqual(state.ir.root.count, 1)
        XCTAssertNil(state.error)

        if let node = state.ir.root.first, node.kind == .text {
            XCTAssertEqual(node.content, "Static Text")
        } else {
            XCTFail("Expected a text node")
        }
    }

    func testReload() throws {
        let state = AurorState()

        let template = """
        span
          "Count: {count}"
        """

        try state.load(template: template, context: ["count": .int(1)])

        if let node = state.ir.root.first, node.kind == .text {
            XCTAssertEqual(node.content, "Count: 1")
        } else {
            XCTFail("Expected a text node")
        }

        try state.reload(context: ["count": .int(2)])

        if let node = state.ir.root.first, node.kind == .text {
            XCTAssertEqual(node.content, "Count: 2")
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
        }
    }
}
