// swift-tools-version:5.9
import PackageDescription
let rustLibDir = "../../target/debug"
let package = Package(
    name: "HyperChat", platforms: [.macOS("15.0")],
    dependencies: [.package(name: "Aurorality", path: "../../")],
    targets: [.executableTarget(name: "HyperChat",
        dependencies: [.product(name: "Aurorality", package: "Aurorality")],
        path: ".", exclude: ["rust-backend", "Package.swift", ".brisk.toml", "README.md"],
        sources: ["Sources"],
        resources: [.copy("views"), .copy("scripts")],
        linkerSettings: [.unsafeFlags([
            "-L\(rustLibDir)", "-laurorality_core",
            "-framework", "JavaScriptCore"
        ])])])
