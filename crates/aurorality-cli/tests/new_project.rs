//! `aurorality new` must scaffold a SwiftPM project wired to an aurorality checkout.

use std::path::Path;
use std::process::Command;

fn workspace_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

#[test]
fn new_project_is_a_swift_package_wired_to_aurorality() {
    let dir = tempfile::tempdir().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_aurorality"))
        .args(["new", "demo-app"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let root = dir.path().join("demo-app");
    let aur = workspace_root();

    let package = std::fs::read_to_string(root.join("Package.swift"))
        .expect("scaffold must write Package.swift so `swift build` can run");
    assert!(
        package.contains(&format!(
            ".package(name: \"Aurorality\", path: \"{}\")",
            aur.display()
        )),
        "{package}"
    );
    assert!(
        package.contains(".executableTarget(\n            name: \"DemoApp\""),
        "{package}"
    );
    assert!(
        package.contains(".process(\"views/main.crepus\")"),
        "{package}"
    );
    assert!(package.contains("-laurorality_core"), "{package}");

    let app = std::fs::read_to_string(root.join("Sources/App.swift")).unwrap();
    assert!(
        app.contains("AurorTemplates.source(\"main\", bundle: .module)"),
        "{app}"
    );
    assert!(
        app.contains("state.watchLiveTemplate(\"main\")"),
        "scaffold must live-reload under dev:\n{app}"
    );
    assert!(
        app.contains(".environment(bridge)"),
        "AurorRootView requires an AurorBridge:\n{app}"
    );
    assert!(
        !app.contains("!)"),
        "scaffold must not force-unwrap resource loading:\n{app}"
    );

    // Pre-build commands must target the checkout, not a sibling `../aurorality`
    // or the project directory (which has no Cargo workspace).
    let brisk: toml::Value =
        toml::from_str(&std::fs::read_to_string(root.join(".brisk.toml")).unwrap()).unwrap();
    assert_eq!(brisk["package"]["name"].as_str(), Some("DemoApp"));
    let cmds: Vec<&str> = brisk["pre_build"]["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    let manifest = format!("--manifest-path '{}/Cargo.toml'", aur.display());
    assert!(
        cmds.iter()
            .all(|c| !c.starts_with("cargo") || c.contains(&manifest)),
        "{cmds:?}"
    );
    assert!(
        cmds.iter().any(|c| c.contains("module.modulemap")),
        "{cmds:?}"
    );
    assert!(
        cmds.iter().all(|c| !c.contains("../aurorality")),
        "{cmds:?}"
    );
    assert_eq!(
        brisk["aurorality"]["path"].as_str(),
        Some(aur.to_str().unwrap())
    );
}

#[test]
fn new_project_rejects_a_path_that_is_not_an_aurorality_checkout() {
    let dir = tempfile::tempdir().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_aurorality"))
        .args(["new", "demo", "--aurorality"])
        .arg(dir.path())
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(!dir.path().join("demo").exists());
}

#[test]
fn new_project_uses_an_explicit_checkout_and_escapes_the_name() {
    let dir = tempfile::tempdir().unwrap();
    let checkout = dir.path().join("my aurorality");
    std::fs::create_dir_all(checkout.join("crates/aurorality-core")).unwrap();
    std::fs::write(checkout.join("Package.swift"), "").unwrap();
    std::fs::write(checkout.join("crates/aurorality-core/Cargo.toml"), "").unwrap();
    let checkout = checkout.canonicalize().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_aurorality"))
        .args(["new", "say\"hi", "--aurorality"])
        .arg(&checkout)
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let root = dir.path().join("say\"hi");
    let package = std::fs::read_to_string(root.join("Package.swift")).unwrap();
    assert!(
        package.contains(&format!(
            ".package(name: \"Aurorality\", path: \"{}\")",
            checkout.display()
        )),
        "{package}"
    );
    let brisk: toml::Value =
        toml::from_str(&std::fs::read_to_string(root.join(".brisk.toml")).unwrap()).unwrap();
    assert_eq!(
        brisk["aurorality"]["path"].as_str(),
        Some(checkout.to_str().unwrap())
    );
    let app = std::fs::read_to_string(root.join("Sources/App.swift")).unwrap();
    assert!(app.contains(r#".string("Hello from say\"hi")"#), "{app}");

    // The printed next-step command must work in a shell for this name.
    let stdout = String::from_utf8_lossy(&out.stdout);
    let hint = stdout
        .lines()
        .find_map(|l| l.trim().strip_suffix(" && aurorality run"))
        .unwrap_or_else(|| panic!("no cd hint in: {stdout}"));
    let pwd = Command::new("sh")
        .arg("-c")
        .arg(format!("{hint} && pwd -P"))
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(pwd.status.success(), "hint `{hint}` failed in sh");
    assert_eq!(
        String::from_utf8_lossy(&pwd.stdout).trim(),
        root.canonicalize().unwrap().to_str().unwrap()
    );
}
