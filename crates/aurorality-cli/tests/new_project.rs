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
    assert!(app.contains("Bundle.module.url(forResource: \"main\", withExtension: \"crepus\")"));
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
