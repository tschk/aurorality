import Aurorality
@testable import HyperChat
import XCTest

final class HyperChatModelTests: XCTestCase {
    func testProtocolRequiredBeforeSend() {
        let bridge = AurorBridge()
        let m = HyperChatModel(bridge: bridge)
        m.draft = "hello"
        m.selectedProtocol = ""
        XCTAssertFalse(m.viewContext.canSend)
        m.selectedProtocol = "matrix"
        XCTAssertTrue(m.viewContext.canSend)
    }

    func testDraftClearAfterSendAttemptClearsWhenBitchatSelected() {
        let bridge = AurorBridge()
        let m = HyperChatModel(bridge: bridge)
        m.selectedProtocol = "bitchat"
        m.draft = "x"
        m.handleEvent("send")
        XCTAssertTrue(m.draft.isEmpty)
    }
}
