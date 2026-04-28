// swift-tools-version:5.9
import PackageDescription

// NOTE: Run `cargo build -p aurorality-core` and the uniffi-bindgen step before
// opening this package. The generated/ directory must contain:
//   aurorality_core.swift, aurorality_coreFFI.h, aurorality_coreFFI.modulemap

let package = Package(
    name: "Aurorality",
    platforms: [.macOS(.v14), .iOS(.v17)],
    products: [
        .library(name: "Aurorality", targets: ["Aurorality"]),
    ],
    targets: [
        // C module wrapping the UniFFI-generated header.
        // The modulemap lives in generated/ and defines the 'aurorality_coreFFI' module.
        .systemLibrary(
            name: "aurorality_coreFFI",
            path: "generated"
        ),
        // Main library: handwritten Swift + generated UniFFI wrapper.
        .target(
            name: "Aurorality",
            dependencies: ["aurorality_coreFFI"],
            path: ".",
            exclude: [
                "Cargo.toml",
                "crates",
                "examples",
                "runner",
                ".brisk.toml",
                "generated/aurorality_coreFFI.h",
                "generated/aurorality_coreFFI.modulemap",
            ],
            sources: [
                "generated/aurorality_core.swift",
                "swift/Sources/Aurorality",
            ]
        ),
    ]
)
