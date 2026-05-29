//! Build SwiftUI project and bundle into native macOS .app.
//!
//! Reads project metadata from `crepus.toml` (or falls back to `.brisk.toml`
//! or `Package.swift`), compiles with `swift build`, wraps the binary in a
//! minimal `.app` bundle, and launches it with `open`.

use std::path::{Path, PathBuf};
use std::process::{Child, Command};

use anyhow::Result;

pub struct ProjectConfig {
    pub name: String,
    pub bundle_id: String,
    pub resources: Vec<String>,
}

pub fn read_config(project_root: &Path) -> Result<ProjectConfig> {
    // 1. Try crepus.toml
    if let Ok(cfg) = try_read_toml(&project_root.join("crepus.toml")) {
        return Ok(cfg);
    }
    // 2. Fallback: .brisk.toml
    if let Ok(cfg) = try_read_brisk(&project_root.join(".brisk.toml")) {
        return Ok(cfg);
    }
    // 3. Fallback: infer from Package.swift
    infer_from_package(project_root)
}

fn try_read_toml(path: &Path) -> Result<ProjectConfig> {
    let content = std::fs::read_to_string(path)?;
    let mut name = String::new();
    let mut bundle_id = String::new();
    let mut resources: Vec<String> = vec![];

    let mut section = "";
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            section = trimmed;
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            let k = k.trim();
            let v = v.trim().trim_matches('"');
            match (section, k) {
                ("[package]", "name") => name = v.to_string(),
                ("[app]", "bundle_id") => bundle_id = v.to_string(),
                _ => {}
            }
        }
        if section == "[app]" && trimmed.starts_with("resources") {
            if let Some(arr) = trimmed.split_once('=').map(|x| x.1.trim()) {
                resources = arr
                    .trim_matches(|c: char| c == '[' || c == ']' || c == '"')
                    .split(',')
                    .map(|s| s.trim().trim_matches('"').to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }
    }

    if name.is_empty() {
        anyhow::bail!("missing [package].name in {}", path.display());
    }
    if bundle_id.is_empty() {
        bundle_id = format!("dev.aurorality.{}", name.to_lowercase());
    }

    Ok(ProjectConfig {
        name,
        bundle_id,
        resources,
    })
}

fn try_read_brisk(path: &Path) -> Result<ProjectConfig> {
    try_read_toml(path)
}

fn infer_from_package(project_root: &Path) -> Result<ProjectConfig> {
    let pkg_swift = project_root.join("Package.swift");
    let content = std::fs::read_to_string(&pkg_swift)?;
    let mut name = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("name:") {
            if let Some(start) = trimmed.find('"') {
                if let Some(end) = trimmed[start + 1..].find('"') {
                    name = trimmed[start + 1..start + 1 + end].to_string();
                    break;
                }
            }
        }
    }

    if name.is_empty() {
        anyhow::bail!("could not infer project name from {}", pkg_swift.display());
    }

    Ok(ProjectConfig {
        bundle_id: format!("dev.aurorality.{}", name.to_lowercase()),
        resources: vec![],
        name,
    })
}

/// Build and launch on iOS Simulator.
pub fn build_and_launch_ios(project_root: &Path) -> Result<()> {
    let cfg = read_config(project_root)?;
    let swift = find_swift();

    let sdk_path = String::from_utf8(
        Command::new("xcrun")
            .args(["--sdk", "iphonesimulator", "--show-sdk-path"])
            .output()
            .map_err(|e| anyhow::anyhow!("xcrun: {e}"))?
            .stdout,
    )?
    .trim()
    .to_string();

    println!("building {} for iOS Simulator...", cfg.name);
    let status = Command::new(&swift)
        .args([
            "build",
            "--triple",
            "arm64-apple-ios-simulator",
            "-Xswiftc",
            "-sdk",
            "-Xswiftc",
            &sdk_path,
        ])
        .current_dir(project_root)
        .status()
        .map_err(|e| anyhow::anyhow!("swift build ios: {e}"))?;
    if !status.success() {
        anyhow::bail!("ios build failed");
    }

    let bin_name = package_name(project_root).unwrap_or_else(|| cfg.name.clone());
    let bin =
        find_binary(project_root, &bin_name).or_else(|_| find_binary(project_root, &cfg.name))?;

    println!("launching in iOS Simulator...");
    let booted = Command::new("xcrun")
        .args(["simctl", "list", "booted", "devices"])
        .output()
        .map_err(|e| anyhow::anyhow!("simctl list: {e}"))?;
    if booted.stdout.is_empty() || !String::from_utf8_lossy(&booted.stdout).contains("iPhone") {
        eprintln!("  no booted simulator — booting iPhone 16...");
        Command::new("xcrun")
            .args(["simctl", "boot", "iPhone 16"])
            .status()
            .ok();
        std::thread::sleep(std::time::Duration::from_secs(5));
    }

    Command::new("xcrun")
        .args(["simctl", "install", "booted", &bin.display().to_string()])
        .status()
        .map_err(|e| anyhow::anyhow!("simctl install: {e}"))?;

    Command::new("xcrun")
        .args(["simctl", "launch", "booted", &cfg.bundle_id])
        .status()
        .map_err(|e| anyhow::anyhow!("simctl launch: {e}"))?;

    println!("app running in Simulator");
    Ok(())
}

/// Build project, wrap in .app bundle, launch with `open` — returns Child.
pub fn build_and_launch_spawn(project_root: &Path) -> Result<Child> {
    let cfg = read_config(project_root)?;
    let bin_name = package_name(project_root).unwrap_or_else(|| cfg.name.clone());
    let bin =
        find_binary(project_root, &bin_name).or_else(|_| find_binary(project_root, &cfg.name))?;

    let app_dir = create_app_bundle(
        project_root,
        &cfg.name,
        &cfg.bundle_id,
        &bin,
        &cfg.resources,
    )?;

    Command::new("open")
        .arg(&app_dir)
        .spawn()
        .map_err(|e| anyhow::anyhow!("open: {e}"))
}

/// Build project, wrap in .app bundle, launch with `open`.
pub fn build_and_launch(project_root: &Path, dev_port: Option<u16>) -> Result<()> {
    let cfg = read_config(project_root)?;
    let swift = find_swift();

    // 1. Build via swift
    println!("building {}...", cfg.name);
    let status = Command::new(&swift)
        .arg("build")
        .current_dir(project_root)
        .status()
        .map_err(|e| anyhow::anyhow!("swift build: {e}"))?;
    if !status.success() {
        anyhow::bail!("swift build failed");
    }

    // 2. Find built binary — try Package.swift name first, then config name
    let bin_name = package_name(project_root).unwrap_or_else(|| cfg.name.clone());
    let bin =
        find_binary(project_root, &bin_name).or_else(|_| find_binary(project_root, &cfg.name))?;

    // 3. Create .app bundle and launch
    let app_dir = create_app_bundle(
        project_root,
        &cfg.name,
        &cfg.bundle_id,
        &bin,
        &cfg.resources,
    )?;

    println!("launching {}...", cfg.name);
    let mut cmd = Command::new("open");
    cmd.arg(&app_dir);
    if let Some(p) = dev_port {
        cmd.env("AURORALITY_DEV", "1")
            .env("AURORALITY_DEV_PORT", p.to_string());
    }
    cmd.spawn().map_err(|e| anyhow::anyhow!("open: {e}"))?;

    println!("app launched  →  use `aurorality dev` in another terminal for hot reload");
    Ok(())
}

fn create_app_bundle(
    project_root: &Path,
    name: &str,
    bundle_id: &str,
    bin: &Path,
    resources: &[String],
) -> Result<PathBuf> {
    let app_name = format!("{}.app", name);
    let app_dir = project_root.join(".build").join(&app_name);
    let macos_dir = app_dir.join("Contents/MacOS");
    let res_dir = app_dir.join("Contents/Resources");

    std::fs::create_dir_all(&macos_dir)?;
    std::fs::create_dir_all(&res_dir)?;

    let dest_bin = macos_dir.join(name);
    std::fs::copy(bin, &dest_bin)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dest_bin)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dest_bin, perms)?;
    }

    for res in resources {
        let src = project_root.join(res);
        if src.exists() {
            let dest = res_dir.join(res);
            if src.is_dir() {
                copy_dir(&src, &dest)?;
            } else {
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(&src, &dest)?;
            }
        }
    }

    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>{name}</string>
    <key>CFBundleIdentifier</key>
    <string>{bundle_id}</string>
    <key>CFBundleName</key>
    <string>{name}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSMinimumSystemVersion</key>
    <string>14.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>"#,
        name = name,
        bundle_id = bundle_id,
    );
    std::fs::write(app_dir.join("Contents/Info.plist"), plist)?;

    Ok(app_dir)
}

fn package_name(project_root: &Path) -> Option<String> {
    let pkg = project_root.join("Package.swift");
    let content = std::fs::read_to_string(&pkg).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("name:") {
            if let Some(start) = trimmed.find('"') {
                if let Some(end) = trimmed[start + 1..].find('"') {
                    return Some(trimmed[start + 1..start + 1 + end].to_string());
                }
            }
        }
    }
    None
}

fn find_binary(project_root: &Path, name: &str) -> Result<PathBuf> {
    // Try debug build
    let debug = project_root.join(".build").join("debug").join(name);
    if debug.exists() {
        return Ok(debug);
    }

    // Search in .build/arm64-apple-macosx/debug/
    for entry in std::fs::read_dir(project_root.join(".build"))?.flatten() {
        let p = entry.path();
        if p.is_dir() {
            let candidate = p.join("debug").join(name);
            if candidate.exists() {
                return Ok(candidate);
            }
            let candidate2 = p.join(name);
            if candidate2.exists() {
                return Ok(candidate2);
            }
        }
    }

    anyhow::bail!("built binary not found for {name} in .build/")
}

fn find_swift() -> String {
    if let Ok(path) = std::env::var("SWIFT_PATH") {
        return path;
    }
    if let Ok(out) = Command::new("xcrun").args(["-f", "swift"]).output() {
        if out.status.success() {
            let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !p.is_empty() {
                return p;
            }
        }
    }
    "swift".to_string()
}

fn copy_dir(src: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = dest.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_dir(&path, &dest_path)?;
        } else {
            std::fs::copy(&path, &dest_path)?;
        }
    }
    Ok(())
}
