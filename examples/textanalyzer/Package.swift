// swift-tools-version:5.9
import PackageDescription

let rustLibPath = "../../target/debug"

let package = Package(
    name: "TextAnalyzer",
    platforms: [.macOS(.v14), .iOS(.v17)],
    dependencies: [
        .package(name: "Aurorality", path: "../.."),
    ],
    targets: [
        .executableTarget(
            name: "TextAnalyzer",
            dependencies: [
                .product(name: "Aurorality", package: "Aurorality"),
            ],
            path: ".",
            exclude: ["Package.swift", ".brisk.toml", "run.sh"],
            sources: ["Sources"],
            resources: [
                .process("views/main.crepus"),
                .process("scripts/backend.js"),
            ],
            linkerSettings: [
                .unsafeFlags([
                    "-L", rustLibPath, "-laurorality_core",
                    "-framework", "JavaScriptCore",
                ]),
            ]
        ),
    ]
)
