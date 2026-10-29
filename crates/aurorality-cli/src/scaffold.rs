//! Project scaffolding for `aurorality new <name>`.
//!
//! The generated project is a SwiftPM package that depends on an aurorality
//! checkout by absolute path (the Swift sources and the Rust core are not
//! published as a binary package), mirroring `examples/basic`.

use std::path::{Path, PathBuf};

use anyhow::Result;

pub fn new_project(name: &str, aurorality: Option<&Path>) -> Result<()> {
    let root = Path::new(name);
    if root.exists() {
        anyhow::bail!("directory {name:?} already exists");
    }
    let aur = aurorality_checkout(aurorality)?;
    let aur_disp = aur.display().to_string();
    if aur_disp.contains('"') || aur_disp.contains('\\') {
        anyhow::bail!("aurorality path {aur_disp:?} contains characters that cannot be embedded");
    }

    let name_pascal = to_pascal_case(name);
    if name_pascal.is_empty() || name_pascal.starts_with(|c: char| c.is_ascii_digit()) {
        anyhow::bail!("{name:?} does not yield a valid Swift target name");
    }
    let name_lower = name_pascal.to_ascii_lowercase();

    std::fs::create_dir_all(root.join("views"))?;
    std::fs::create_dir_all(root.join("Sources"))?;

    std::fs::write(
        root.join("views/main.crepus"),
        "div flex flex-col gap-4 p-8\n  span text-2xl font-bold\n    \"{title}\"\n  span text-base\n    \"{subtitle}\"\n",
    )?;

    std::fs::write(
        root.join("Package.swift"),
        format!(
            r#"// swift-tools-version:5.9
import Foundation
import PackageDescription

let aurorality = "{aur_disp}"
// `aurorality run --ios` points this at the iOS Simulator build of aurorality-core.
let rustLibPath = ProcessInfo.processInfo.environment["AURORALITY_RUST_LIB_DIR"] ?? aurorality + "/target/debug"

let package = Package(
    name: "{name_pascal}",
    platforms: [.macOS(.v14), .iOS(.v17)],
    dependencies: [
        .package(name: "Aurorality", path: "{aur_disp}"),
    ],
    targets: [
        .executableTarget(
            name: "{name_pascal}",
            dependencies: [
                .product(name: "Aurorality", package: "Aurorality"),
            ],
            path: ".",
            exclude: ["Package.swift", ".brisk.toml", ".gitignore"],
            sources: ["Sources"],
            resources: [
                .process("views/main.crepus"),
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
"#
        ),
    )?;

    // Swift string literal: Rust's Debug escaping (`"`, `\\`, control chars) is valid Swift.
    let title = format!("{:?}", format!("Hello from {name}"));
    std::fs::write(
        root.join("Sources/App.swift"),
        format!(
            r#"import SwiftUI
import Aurorality

@main
struct {name_pascal}App: App {{
    @State private var state = AurorState()
    @State private var bridge = AurorBridge()

    var body: some Scene {{
        WindowGroup {{
            AurorRootView(state: state)
                .environment(bridge)
                .task {{ load() }}
        }}
    }}

    private func load() {{
        do {{
            // Under `aurorality dev` this reads views/main.crepus itself; otherwise the bundled copy.
            try state.load(
                template: AurorTemplates.source("main", bundle: .module),
                context: [
                    "title": .string({title}),
                    "subtitle": .string("Edit views/main.crepus to get started"),
                ])
        }} catch {{
            state.error = "\(error)"
        }}
        // `aurorality dev`: re-render on every save, with the context above.
        state.watchLiveTemplate("main")
    }}
}}
"#
        ),
    )?;

    let manifest = sh_quote(&format!("{aur_disp}/Cargo.toml"));
    let dylib = sh_quote(&format!("{aur_disp}/target/debug/libaurorality_core.dylib"));
    let generated = format!("{aur_disp}/generated");
    let commands = [
        format!("cargo build --manifest-path {manifest} -p aurorality-core --features js"),
        format!(
            "cargo run --manifest-path {manifest} -p aurorality-core --features js --bin uniffi-bindgen generate --library {dylib} --language swift --out-dir {}",
            sh_quote(&generated)
        ),
        format!(
            "cp {} {}",
            sh_quote(&format!("{generated}/aurorality_coreFFI.modulemap")),
            sh_quote(&format!("{generated}/module.modulemap"))
        ),
    ];
    let commands_toml = commands
        .iter()
        .map(|c| format!("    {},", toml::Value::String(c.clone())))
        .collect::<Vec<_>>()
        .join("\n");

    let aur_toml = toml::Value::String(aur_disp.clone());
    std::fs::write(
        root.join(".brisk.toml"),
        format!(
            r#"[package]
name = "{name_pascal}"
version = "0.1.0"

[app]
bundle_id = "com.example.{name_lower}"
deployment_target = "17.0"
sources = "Sources"
resources = ["views"]

[aurorality]
path = {aur_toml}

[pre_build]
commands = [
{commands_toml}
]
"#
        ),
    )?;
    std::fs::write(root.join(".gitignore"), ".build/\n*.resolved\n")?;

    println!("created project: {name}");
    println!("  cd {name} && aurorality run");
    Ok(())
}

/// The aurorality checkout to depend on: `--aurorality`, else the checkout this
/// CLI was built from (path / git installs keep their sources on disk).
fn aurorality_checkout(explicit: Option<&Path>) -> Result<PathBuf> {
    let candidate = match explicit {
        Some(p) => p.to_path_buf(),
        None => Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
    };
    let is_checkout = candidate.join("Package.swift").is_file()
        && candidate
            .join("crates/aurorality-core/Cargo.toml")
            .is_file();
    if !is_checkout {
        anyhow::bail!(
            "{} is not an aurorality checkout (expected Package.swift and crates/aurorality-core); \
             pass --aurorality <path to a clone of https://github.com/tschk/aurorality>",
            candidate.display()
        );
    }
    Ok(candidate.canonicalize()?)
}

fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

fn to_pascal_case(s: &str) -> String {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut c = p.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect()
}
