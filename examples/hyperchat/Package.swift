// swift-tools-version:5.9
import PackageDescription

let aurorRust = "../../target/debug"

let package = Package(
    name: "HyperChat",
    platforms: [.macOS("15.0")],
    dependencies: [
        .package(name: "Aurorality", path: "../../"),
    ],
    targets: [
        .systemLibrary(name: "hyperchat_backendFFI", path: "FFI"),
        .executableTarget(
            name: "HyperChat",
            dependencies: [
                .product(name: "Aurorality", package: "Aurorality"),
                "hyperchat_backendFFI",
            ],
            path: ".",
            exclude: ["rust-backend", "Package.swift", ".brisk.toml", "README.md", "views", "Tests", "run.sh"],
            sources: [
                "Sources",
                "Generated/HyperChatGeneratedView.swift",
                "Generated/hyperchat_backend.swift",
            ],
            resources: [],
            linkerSettings: [
                .unsafeFlags([
                    "-L", aurorRust, "-laurorality_core", "-lhyperchat_backend",
                    "-framework", "JavaScriptCore",
                ]),
            ]
        ),
        .testTarget(
            name: "HyperChatTests",
            dependencies: [
                "HyperChat",
                .product(name: "Aurorality", package: "Aurorality"),
                "hyperchat_backendFFI",
            ],
            path: "Tests/HyperChatTests",
            linkerSettings: [
                .unsafeFlags([
                    "-L", aurorRust, "-laurorality_core", "-lhyperchat_backend",
                    "-framework", "JavaScriptCore",
                ]),
            ]
        ),
    ]
)
