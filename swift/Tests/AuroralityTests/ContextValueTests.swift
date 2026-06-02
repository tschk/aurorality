import XCTest
@testable import Aurorality

final class ContextValueTests: XCTestCase {

    func testStringToJson() {
        let val = ContextValue.string("hello")
        let json = val.toJson()
        XCTAssertEqual(json as? String, "hello")
    }

    func testIntToJson() {
        let val = ContextValue.int(42)
        let json = val.toJson()
        XCTAssertEqual(json as? Int, 42)
    }

    func testFloatToJson() {
        let val = ContextValue.float(3.14)
        let json = val.toJson()
        XCTAssertEqual(json as? Double, 3.14)
    }

    func testBoolToJson() {
        let val = ContextValue.bool(true)
        let json = val.toJson()
        XCTAssertEqual(json as? Bool, true)
    }

    func testNullToJson() {
        let val = ContextValue.null
        let json = val.toJson()
        XCTAssertTrue(json is NSNull)
    }

    func testListToJson() {
        let val = ContextValue.list([
            ["name": .string("Alice"), "age": .int(30)],
            ["name": .string("Bob"), "active": .bool(false)]
        ])

        let json = val.toJson()
        guard let list = json as? [[String: Any]] else {
            XCTFail("Expected [[String: Any]]")
            return
        }

        XCTAssertEqual(list.count, 2)

        let first = list[0]
        XCTAssertEqual(first["name"] as? String, "Alice")
        XCTAssertEqual(first["age"] as? Int, 30)

        let second = list[1]
        XCTAssertEqual(second["name"] as? String, "Bob")
        XCTAssertEqual(second["active"] as? Bool, false)
    }

    func testListWithNestedNull() {
        let val = ContextValue.list([
            ["value": .null]
        ])

        let json = val.toJson()
        guard let list = json as? [[String: Any]] else {
            XCTFail("Expected [[String: Any]]")
            return
        }

        XCTAssertEqual(list.count, 1)
        XCTAssertTrue(list[0]["value"] is NSNull)
    }
}
