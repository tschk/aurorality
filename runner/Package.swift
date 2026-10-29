// swift-tools-version:5.9
import Foundation
import PackageDescription

// `aurorality run --ios` points this at the iOS Simulator build of aurorality-core.
let rustLibPath = ProcessInfo.processInfo.environment["AURORALITY_RUST_LIB_DIR"] ?? "../target/debug"

let package = Package(
    name: "AurorRunner",
    platforms: [.macOS(.v14), .iOS(.v17)],
    dependencies: [
        .package(name: "Aurorality", path: ".."),
    ],
    targets: [
        .executableTarget(
            name: "AurorRunner",
            dependencies: [
                .product(name: "Aurorality", package: "Aurorality"),
            ],
            path: ".",
            exclude: ["Package.swift", ".brisk.toml"],
            sources: ["Sources"],
            linkerSettings: [
                .unsafeFlags([
                    "-L", rustLibPath, "-laurorality_core",
                    "-framework", "JavaScriptCore",
                ]),
            ]
        ),
    ]
)
