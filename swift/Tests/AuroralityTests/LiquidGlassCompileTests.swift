import XCTest
import SwiftUI
@testable import Aurorality

/// Compile-time validation for Liquid Glass availability guards.

final class LiquidGlassCompileTests: XCTestCase {

    func testGlassEffectCompilesOnAllPlatforms() {
        struct TestView: View {
            var body: some View {
                Text("Hello")
                    .glassEffect()
            }
        }
        _ = TestView()
    }

    @available(macOS 26, *)
    func testNativeLiquidGlassAvailable() {
        struct NativeGlassView: View {
            var body: some View {
                Text("Native")
                    .glassEffect(in: RoundedRectangle(cornerRadius: 12))
            }
        }
        _ = NativeGlassView()
    }
}
