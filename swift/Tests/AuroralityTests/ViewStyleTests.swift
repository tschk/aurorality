import XCTest
@testable import Aurorality

final class ViewStyleTests: XCTestCase {

    // MARK: - flexDirection
    func testFlexDirectionRow() {
        var s = ViewStyle()
        s.flexDirection = "row"
        XCTAssertEqual(s.flexDirection, "row")
    }

    func testFlexDirectionColumn() {
        var s = ViewStyle()
        s.flexDirection = "column"
        XCTAssertEqual(s.flexDirection, "column")
    }

    // MARK: - Position & zIndex
    func testPositionAbsolute() {
        var s = ViewStyle()
        s.position = "absolute"
        s.zIndex = 10
        s.top = 20
        s.left = 10
        XCTAssertEqual(s.position, "absolute")
        XCTAssertEqual(s.zIndex, 10)
        XCTAssertEqual(s.top, 20)
        XCTAssertEqual(s.left, 10)
    }

    func testPositionRelative() {
        var s = ViewStyle()
        s.position = "relative"
        XCTAssertEqual(s.position, "relative")
    }

    // MARK: - Transform
    func testTranslate() {
        var s = ViewStyle()
        s.translateX = 50
        s.translateY = -20
        XCTAssertEqual(s.translateX, 50)
        XCTAssertEqual(s.translateY, -20)
    }

    func testScaleAndRotate() {
        var s = ViewStyle()
        s.scaleX = 1.5
        s.scaleY = 0.8
        s.rotate = 45
        XCTAssertEqual(s.scaleX, 1.5)
        XCTAssertEqual(s.scaleY, 0.8)
        XCTAssertEqual(s.rotate, 45)
    }

    // MARK: - Shadow
    func testShadowFields() {
        var s = ViewStyle()
        s.shadowColor = "#ff0000"
        s.shadowRadius = 8
        s.shadowOffsetX = 0
        s.shadowOffsetY = 4
        XCTAssertEqual(s.shadowColor, "#ff0000")
        XCTAssertEqual(s.shadowRadius, 8)
        XCTAssertEqual(s.shadowOffsetY, 4)
    }

    // MARK: - Text Layout
    func testTextOverflow() {
        var s = ViewStyle()
        s.textOverflow = "truncate"
        s.lineClamp = 3
        XCTAssertEqual(s.textOverflow, "truncate")
        XCTAssertEqual(s.lineClamp, 3)
    }

    func testWhiteSpace() {
        var s = ViewStyle()
        s.whiteSpace = "nowrap"
        XCTAssertEqual(s.whiteSpace, "nowrap")
    }

    func testCursorAndUserSelect() {
        var s = ViewStyle()
        s.cursor = "pointer"
        s.userSelect = "none"
        XCTAssertEqual(s.cursor, "pointer")
        XCTAssertEqual(s.userSelect, "none")
    }

    // MARK: - Codable round-trip
    func testCodableRoundTrip() throws {
        var original = ViewStyle()
        original.flexDirection = "row"
        original.position = "absolute"
        original.zIndex = 5
        original.translateX = 10
        original.rotate = 90
        original.shadowRadius = 4
        original.textOverflow = "ellipsis"
        original.lineClamp = 2

        let encoded = try JSONEncoder().encode(original)
        let decoded = try JSONDecoder().decode(ViewStyle.self, from: encoded)

        XCTAssertEqual(decoded.flexDirection, original.flexDirection)
        XCTAssertEqual(decoded.position, original.position)
        XCTAssertEqual(decoded.zIndex, original.zIndex)
        XCTAssertEqual(decoded.translateX, original.translateX)
        XCTAssertEqual(decoded.rotate, original.rotate)
        XCTAssertEqual(decoded.shadowRadius, original.shadowRadius)
        XCTAssertEqual(decoded.textOverflow, original.textOverflow)
        XCTAssertEqual(decoded.lineClamp, original.lineClamp)
    }
}
