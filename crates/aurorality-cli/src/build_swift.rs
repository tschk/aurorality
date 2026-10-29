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
    pub sources: Option<String>,
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
    let mut sources = None;
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
                ("[app]", "sources") => sources = Some(v.to_string()),
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
        sources,
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
        sources: None,
        name,
    })
}

/// Swift triple for the arm64 iOS Simulator.
const IOS_SIM_TRIPLE: &str = "arm64-apple-ios-simulator";

/// Build and launch on iOS Simulator.
///
/// `rust_lib_dir` must hold `libaurorality_core.a` built for the simulator
/// (see [`crate::pre_build_ios`]); it is passed to SwiftPM through
/// `AURORALITY_RUST_LIB_DIR`. Every step's exit status is checked.
pub fn build_and_launch_ios(project_root: &Path, rust_lib_dir: &Path) -> Result<()> {
    let cfg = read_config(project_root)?;
    let swift = find_swift();

    let sdk_path = checked_stdout(
        Command::new("xcrun").args(["--sdk", "iphonesimulator", "--show-sdk-path"]),
        "xcrun --show-sdk-path",
    )?;
    if sdk_path.is_empty() {
        anyhow::bail!("xcrun returned no iphonesimulator SDK path");
    }

    println!("building {} for iOS Simulator...", cfg.name);
    let status = Command::new(&swift)
        .args([
            "build",
            "--triple",
            IOS_SIM_TRIPLE,
            "-Xswiftc",
            "-sdk",
            "-Xswiftc",
            &sdk_path,
        ])
        .env("AURORALITY_RUST_LIB_DIR", rust_lib_dir)
        .current_dir(project_root)
        .status()
        .map_err(|e| anyhow::anyhow!("swift build ios: {e}"))?;
    if !status.success() {
        anyhow::bail!("ios build failed ({})", status.code().unwrap_or(1));
    }

    // Only accept the simulator-triple binary; `.build/debug` may be a macOS build.
    let bin_name = package_name(project_root).unwrap_or_else(|| cfg.name.clone());
    let triple_dir = project_root
        .join(".build")
        .join(IOS_SIM_TRIPLE)
        .join("debug");
    let bin = [&bin_name, &cfg.name]
        .into_iter()
        .map(|n| triple_dir.join(n))
        .find(|p| p.is_file())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "iOS Simulator binary {bin_name} not found in {}",
                triple_dir.display()
            )
        })?;

    let app_dir = create_ios_app_bundle(project_root, &cfg, &bin)?;

    // An explicit AURORALITY_SIMULATOR wins over whatever is booted; else use a
    // booted simulator; else boot a compatible one.
    let device = match std::env::var("AURORALITY_SIMULATOR") {
        Ok(dev) => Some(dev),
        Err(_) => {
            let booted = checked_stdout(
                Command::new("xcrun").args(["simctl", "list", "devices", "booted"]),
                "simctl list",
            )?;
            if booted.contains("Booted") {
                None
            } else {
                Some(pick_simulator()?)
            }
        }
    };
    if let Some(dev) = &device {
        println!("  booting simulator {dev}...");
        // Boots the device if needed and waits until it is ready.
        run_checked(
            Command::new("xcrun").args(["simctl", "bootstatus", dev, "-b"]),
            "simctl bootstatus",
        )?;
    }
    let target = device.as_deref().unwrap_or("booted");

    println!("installing in iOS Simulator...");
    run_checked(
        Command::new("xcrun")
            .args(["simctl", "install", target])
            .arg(&app_dir),
        "simctl install",
    )?;
    run_checked(
        Command::new("xcrun").args(["simctl", "launch", target, &cfg.bundle_id]),
        "simctl launch",
    )?;

    println!("app running in Simulator");
    Ok(())
}

/// Oldest iOS runtime the generated app supports (`MinimumOSVersion`).
const IOS_MIN_MAJOR: u32 = 17;

/// Simulator to boot when none is booted: the first available iPhone on a
/// runtime the app supports, else any such device.
fn pick_simulator() -> Result<String> {
    let json = checked_stdout(
        Command::new("xcrun").args(["simctl", "list", "devices", "available", "-j"]),
        "simctl list devices available",
    )?;
    let parsed: serde_json::Value = serde_json::from_str(&json)
        .map_err(|e| anyhow::anyhow!("simctl list devices: unexpected output: {e}"))?;
    let devices: Vec<&serde_json::Value> = parsed["devices"]
        .as_object()
        .into_iter()
        .flat_map(|runtimes| runtimes.iter())
        // Runtime keys look like `com.apple.CoreSimulator.SimRuntime.iOS-27-0`.
        .filter(|(runtime, _)| {
            runtime
                .rsplit_once("iOS-")
                .and_then(|(_, v)| v.split('-').next()?.parse::<u32>().ok())
                .is_some_and(|major| major >= IOS_MIN_MAJOR)
        })
        .flat_map(|(_, list)| list.as_array().into_iter().flatten())
        .collect();
    devices
        .iter()
        .find(|d| d["name"].as_str().is_some_and(|n| n.starts_with("iPhone")))
        .or(devices.first())
        .and_then(|d| d["udid"].as_str())
        .map(str::to_string)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no available iOS {IOS_MIN_MAJOR}+ simulator; create one in Xcode or set AURORALITY_SIMULATOR"
            )
        })
}

/// Copy SwiftPM resource bundles (`<Package>_<Target>.bundle`) built next to
/// `bin` into `dest`, the app's resource directory. `Bundle.module` looks in
/// `Bundle.main.resourceURL`, and current toolchains have no build-path fallback,
/// so without this an app that loads resources traps on launch.
fn copy_swiftpm_bundles(bin: &Path, dest: &Path) -> Result<()> {
    let Some(build_dir) = bin.parent() else {
        return Ok(());
    };
    for entry in std::fs::read_dir(build_dir)?.flatten() {
        let path = entry.path();
        if path.is_dir() && path.extension().is_some_and(|e| e == "bundle") {
            let target = dest.join(entry.file_name());
            if target.exists() {
                std::fs::remove_dir_all(&target)?;
            }
            copy_dir(&path, &target)?;
        }
    }
    Ok(())
}

fn run_checked(cmd: &mut Command, what: &str) -> Result<()> {
    let status = cmd.status().map_err(|e| anyhow::anyhow!("{what}: {e}"))?;
    if !status.success() {
        anyhow::bail!("{what} failed ({})", status.code().unwrap_or(1));
    }
    Ok(())
}

fn checked_stdout(cmd: &mut Command, what: &str) -> Result<String> {
    let out = cmd.output().map_err(|e| anyhow::anyhow!("{what}: {e}"))?;
    if !out.status.success() {
        anyhow::bail!(
            "{what} failed ({}): {}",
            out.status.code().unwrap_or(1),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// iOS bundles are flat: executable + Info.plist at the bundle root.
fn create_ios_app_bundle(project_root: &Path, cfg: &ProjectConfig, bin: &Path) -> Result<PathBuf> {
    let app_dir = project_root
        .join(".build")
        .join(IOS_SIM_TRIPLE)
        .join(format!("{}.app", cfg.name));
    if app_dir.exists() {
        std::fs::remove_dir_all(&app_dir)?;
    }
    std::fs::create_dir_all(&app_dir)?;
    std::fs::copy(bin, app_dir.join(&cfg.name))?;
    // On iOS the resource URL is the app bundle root.
    copy_swiftpm_bundles(bin, &app_dir)?;
    for res in &cfg.resources {
        let src = project_root.join(res);
        let dest = app_dir.join(res);
        if src.is_dir() {
            copy_dir(&src, &dest)?;
        } else if src.is_file() {
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&src, &dest)?;
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
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundleSupportedPlatforms</key>
    <array><string>iPhoneSimulator</string></array>
    <key>MinimumOSVersion</key>
    <string>17.0</string>
    <key>UIDeviceFamily</key>
    <array><integer>1</integer><integer>2</integer></array>
    <key>UILaunchScreen</key>
    <dict/>
</dict>
</plist>"#,
        name = cfg.name,
        bundle_id = cfg.bundle_id,
    );
    std::fs::write(app_dir.join("Info.plist"), plist)?;
    Ok(app_dir)
}

/// Build project, wrap in .app bundle, launch with `open` — returns Child.
pub fn build_and_launch_spawn(project_root: &Path, env: &[(&str, String)]) -> Result<Child> {
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

    let mut cmd = Command::new(app_dir.join("Contents").join("MacOS").join(&bin_name));
    // AURORALITY_DEV turns on the dev overlay, which connects to the WebSocket
    // server; only `dev --serve` (which passes a port) runs one.
    if env.iter().any(|(k, _)| *k == "AURORALITY_DEV_PORT") {
        cmd.env("AURORALITY_DEV", "1");
    }
    cmd.envs(env.iter().map(|(k, v)| (*k, v.as_str())))
        .spawn()
        .map_err(|e| anyhow::anyhow!("launch {}: {e}", cfg.name))
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
    copy_swiftpm_bundles(bin, &res_dir)?;

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
