//! `aurorality run --ios` must build an iOS-target Rust core and must fail
//! loudly when any step fails. Uses fake `cargo` / `swift` / `xcrun` so the
//! test never touches a real toolchain or simulator.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const FAKE_CARGO: &str = r#"#!/bin/sh
echo "cargo $*" >> "$FAKE_LOG"
case "$*" in
  *--target*)
    [ "$FAKE_CARGO_IOS_FAIL" = "1" ] && exit 101
    out="${CARGO_TARGET_DIR:-target}/aarch64-apple-ios-sim/debug"
    mkdir -p "$out"
    : > "$out/libaurorality_core.a"
    ;;
esac
exit 0
"#;

const FAKE_SWIFT: &str = r#"#!/bin/sh
echo "swift $* RUST_LIB_DIR=${AURORALITY_RUST_LIB_DIR:-unset}" >> "$FAKE_LOG"
case "$*" in
  *arm64-apple-ios-simulator*)
    mkdir -p .build/arm64-apple-ios-simulator/debug/Demo_Demo.bundle
    : > .build/arm64-apple-ios-simulator/debug/Demo_Demo.bundle/main.crepus
    : > .build/arm64-apple-ios-simulator/debug/Demo
    ;;
  *)
    mkdir -p .build/debug
    : > .build/debug/Demo
    ;;
esac
exit 0
"#;

const FAKE_XCRUN: &str = r#"#!/bin/sh
echo "xcrun $*" >> "$FAKE_LOG"
case "$*" in
  "-f swift") echo "$SWIFT_PATH" ;;
  *show-sdk-path*) echo "/fake/iPhoneSimulator.sdk" ;;
  "simctl list devices booted") [ "$FAKE_NONE_BOOTED" = "1" ] || echo "    iPhone 16 (AAAA) (Booted)" ;;
  "simctl list devices available -j") echo '{"devices":{"com.apple.CoreSimulator.SimRuntime.iOS-16-4":[{"name":"iPhone 14","udid":"OLD-0"}],"com.apple.CoreSimulator.SimRuntime.iOS-27-0":[{"name":"iPad Air","udid":"PAD-1"},{"name":"iPhone 17 Pro","udid":"PHONE-2"}]}}' ;;
  "simctl install"*) exit "${FAKE_INSTALL_STATUS:-0}" ;;
  "simctl launch"*) exit "${FAKE_LAUNCH_STATUS:-0}" ;;
esac
exit 0
"#;

struct Fixture {
    _dir: tempfile::TempDir,
    project: PathBuf,
    bin: PathBuf,
    log: PathBuf,
}

fn write_exe(path: &Path, body: &str) {
    std::fs::write(path, body).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("bin");
    let project = dir.path().join("proj");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::create_dir_all(&project).unwrap();
    write_exe(&bin.join("cargo"), FAKE_CARGO);
    write_exe(&bin.join("swift"), FAKE_SWIFT);
    write_exe(&bin.join("xcrun"), FAKE_XCRUN);
    std::fs::create_dir_all(project.join("assets/nested")).unwrap();
    std::fs::write(project.join("assets/nested/logo.txt"), "x").unwrap();
    std::fs::write(
        project.join(".brisk.toml"),
        "[package]\nname = \"Demo\"\n\n[aurorality]\npath = \".\"\n\n[app]\nbundle_id = \"dev.test.demo\"\nresources = [\"assets/nested/logo.txt\"]\n",
    )
    .unwrap();
    std::fs::write(
        project.join("Package.swift"),
        "// swift-tools-version:5.9\nlet package = Package(\n    name: \"Demo\",\n)\n",
    )
    .unwrap();
    let log = dir.path().join("calls.log");
    Fixture {
        _dir: dir,
        project,
        bin,
        log,
    }
}

fn run_ios(fx: &Fixture, envs: &[(&str, &str)]) -> (Output, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_aurorality"));
    cmd.args(["run", "--ios", "."])
        .current_dir(&fx.project)
        .env("PATH", format!("{}:/usr/bin:/bin", fx.bin.display()))
        .env("CARGO", fx.bin.join("cargo"))
        .env("SWIFT_PATH", fx.bin.join("swift"))
        .env("FAKE_LOG", &fx.log)
        // Tests opt in explicitly; an inherited selection would override the fixture.
        .env_remove("AURORALITY_SIMULATOR")
        .env_remove("CARGO_TARGET_DIR");
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let out = cmd.output().unwrap();
    let log = std::fs::read_to_string(&fx.log).unwrap_or_default();
    (out, log)
}

#[test]
fn ios_run_builds_rust_for_the_simulator_target() {
    let fx = fixture();
    let (out, log) = run_ios(&fx, &[]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        log.lines()
            .any(|l| l.starts_with("cargo build") && l.contains("--target aarch64-apple-ios-sim")),
        "no iOS-target cargo build:\n{log}"
    );
    let ios_swift = log
        .lines()
        .find(|l| l.starts_with("swift build") && l.contains("arm64-apple-ios-simulator"))
        .unwrap_or_else(|| panic!("no iOS swift build:\n{log}"));
    assert!(
        ios_swift.contains("aarch64-apple-ios-sim"),
        "swift build not pointed at the iOS Rust library: {ios_swift}"
    );
    assert!(
        log.lines()
            .any(|l| l.starts_with("xcrun simctl install") && l.trim_end().ends_with(".app")),
        "simctl install was not given an .app bundle:\n{log}"
    );
    let app = fx.project.join(".build/arm64-apple-ios-simulator/Demo.app");
    assert!(
        app.join("Demo_Demo.bundle/main.crepus").is_file(),
        "SwiftPM resource bundle missing from the iOS app (Bundle.module would trap)"
    );
    assert!(
        app.join("assets/nested/logo.txt").is_file(),
        "nested file resource not copied"
    );
}

#[test]
fn ios_run_finds_the_rust_library_in_cargo_target_dir() {
    let fx = fixture();
    let target = fx.project.parent().unwrap().join("custom-target");
    let (out, log) = run_ios(&fx, &[("CARGO_TARGET_DIR", target.to_str().unwrap())]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let ios_swift = log
        .lines()
        .find(|l| l.starts_with("swift build") && l.contains("arm64-apple-ios-simulator"))
        .unwrap_or_else(|| panic!("no iOS swift build:\n{log}"));
    assert!(
        ios_swift.contains(target.to_str().unwrap()),
        "swift build not linked against CARGO_TARGET_DIR: {ios_swift}"
    );
}

#[test]
fn ios_run_boots_an_available_simulator_when_none_is_booted() {
    let fx = fixture();
    let (out, log) = run_ios(&fx, &[("FAKE_NONE_BOOTED", "1")]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        log.lines()
            .any(|l| l == "xcrun simctl bootstatus PHONE-2 -b"),
        "should boot the first iPhone on a supported (iOS 17+) runtime:\n{log}"
    );
    assert!(
        log.lines()
            .any(|l| l.starts_with("xcrun simctl install PHONE-2 ")),
        "{log}"
    );
}

#[test]
fn ios_run_honours_an_explicit_simulator_even_when_another_is_booted() {
    let fx = fixture();
    let (out, log) = run_ios(&fx, &[("AURORALITY_SIMULATOR", "CHOSEN-9")]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        log.lines()
            .any(|l| l == "xcrun simctl bootstatus CHOSEN-9 -b"),
        "{log}"
    );
    assert!(
        log.lines()
            .any(|l| l.starts_with("xcrun simctl install CHOSEN-9 ")),
        "{log}"
    );
    assert!(
        log.lines()
            .any(|l| l == "xcrun simctl launch CHOSEN-9 dev.test.demo"),
        "{log}"
    );
    assert!(!log.contains(" booted "), "used the booted device:\n{log}");
}

#[test]
fn ios_run_fails_when_simulator_install_fails() {
    let fx = fixture();
    let (out, _) = run_ios(&fx, &[("FAKE_INSTALL_STATUS", "1")]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!out.status.success(), "reported success: {stdout}");
    assert!(!stdout.contains("app running in Simulator"), "{stdout}");
}

#[test]
fn ios_run_fails_when_simulator_launch_fails() {
    let fx = fixture();
    let (out, _) = run_ios(&fx, &[("FAKE_LAUNCH_STATUS", "3")]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!out.status.success(), "reported success: {stdout}");
    assert!(!stdout.contains("app running in Simulator"), "{stdout}");
}

#[test]
fn ios_run_stops_when_ios_rust_build_fails() {
    let fx = fixture();
    let (out, log) = run_ios(&fx, &[("FAKE_CARGO_IOS_FAIL", "1")]);
    assert!(!out.status.success());
    assert!(
        !log.lines()
            .any(|l| l.starts_with("swift build") && l.contains("arm64-apple-ios-simulator")),
        "swift build ran after the iOS Rust build failed:\n{log}"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("aarch64-apple-ios-sim"), "{stderr}");
}
