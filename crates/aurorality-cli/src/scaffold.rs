//! Project scaffolding for `aurorality new <name>`.

use std::path::Path;

use anyhow::Result;

pub fn new_project(name: &str) -> Result<()> {
    let root = Path::new(name);
    if root.exists() {
        anyhow::bail!("directory {name:?} already exists");
    }

    std::fs::create_dir_all(root.join("views"))?;
    std::fs::create_dir_all(root.join("Sources"))?;

    let name_pascal = to_pascal_case(name);
    let name_lower = name.to_ascii_lowercase();

    std::fs::write(
        root.join("views/main.crepus"),
        "div flex flex-col gap-4 p-8\n  span text-2xl font-bold\n    \"{title}\"\n  span text-base\n    \"{subtitle}\"\n",
    )?;

    std::fs::write(
        root.join("Sources/App.swift"),
        format!(
            r#"import SwiftUI
import Aurorality

@main
struct {name_pascal}App: App {{
    @State private var state = AurorState()

    var body: some Scene {{
        WindowGroup {{
            AurorRootView(state: state)
                .task {{
                    let url = Bundle.main.url(forResource: "main", withExtension: "crepus")!
                    let template = try! String(contentsOf: url)
                    try! state.load(
                        template: template,
                        context: [
                            "title": .string("Hello from {name}"),
                            "subtitle": .string("Edit views/main.crepus to get started"),
                        ])
                }}
        }}
    }}
}}
"#
        ),
    )?;

    std::fs::write(
        root.join(".brisk.toml"),
        format!(
            r#"[package]
name = "{name}"
version = "0.1.0"

[app]
bundle_id = "com.example.{name_lower}"
deployment_target = "17.0"
sources = "Sources"
resources = ["views"]

[pre_build]
commands = [
    "cargo build -p aurorality-core",
    "cargo run --manifest-path ../aurorality/Cargo.toml -p aurorality-core --bin uniffi-bindgen generate --library ../aurorality/target/debug/libaurorality_core.dylib --language swift --out-dir ../aurorality/generated"
]

[build]
linker_flags = ["-L../aurorality/target/debug", "-laurorality_core"]
"#
        ),
    )?;

    println!("created project: {name}");
    println!("  cd {name} && aurorality dev");
    Ok(())
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
