//! `aurorality` CLI — dev server, build, and project scaffolding.

mod bindgen;
mod build_swift;
mod bundle;
mod dev_loop;
mod hot_reload;
mod scaffold;
mod swiftgen;
mod swiftgen_style;

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "aurorality",
    about = "SwiftUI + Rust shell for .crepus templates",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build, launch, and live-reload the app; rebuild on code changes.
    ///
    /// By default there is no server and no port: the app is launched with
    /// `AURORALITY_VIEWS_DIR` and re-renders its own templates on save
    /// (`AurorState.watchLiveTemplate`). Swift changes, and template saves for
    /// apps that do not watch, kill the app, rebuild, and relaunch it.
    ///
    /// `--serve` also runs the WebSocket server (ws://127.0.0.1:<port>) for
    /// `HotReloadClient`: the Runner, `aurorDevOverlay`, and real devices.
    Dev {
        /// Directory of `.crepus` files to watch.
        #[arg(short, long, default_value = "views")]
        watch: PathBuf,
        /// Also serve hot reload over WebSocket for `HotReloadClient`.
        #[arg(long)]
        serve: bool,
        /// WebSocket port for `--serve` (default 47832).
        #[arg(long, requires = "serve")]
        port: Option<u16>,
        /// With `--serve`: do not send IR patches (swiftgen status only).
        #[arg(long, requires = "serve")]
        no_ir: bool,
        /// With `--serve`: re-run swiftgen on this `.crepus` file after each save.
        #[arg(long, requires = "swiftgen_out", requires = "serve")]
        swiftgen_view: Option<PathBuf>,
        /// Output directory for swiftgen.
        #[arg(long, requires = "swiftgen_view")]
        swiftgen_out: Option<PathBuf>,
        #[arg(long, default_value = "GeneratedView")]
        swiftgen_view_name: String,
        #[arg(long, default_value = "HyperChatContext")]
        swiftgen_context_type: String,
    },

    /// Build and launch the SwiftUI app.
    Run {
        /// Project directory.
        #[arg(default_value = ".")]
        project: PathBuf,

        /// Build and launch on iOS Simulator instead of macOS.
        #[arg(long)]
        ios: bool,
    },

    /// Render all .crepus files in a directory to JSON IR for bundling.
    Build {
        #[arg(short, long, default_value = "views")]
        watch: PathBuf,
        #[arg(short, long, default_value = "generated/ir")]
        out: PathBuf,
    },

    /// Scaffold a new aurorality project (a SwiftPM package).
    New {
        name: String,
        /// Path to the aurorality checkout the project builds against
        /// (defaults to the checkout this CLI was built from).
        #[arg(long)]
        aurorality: Option<PathBuf>,
    },

    /// Bundle JS + compile .crepus templates to IR JSON.
    Bundle {
        #[arg(long, default_value = "views")]
        views: PathBuf,
        #[arg(long, default_value = "generated/ir")]
        out: PathBuf,
        #[arg(long)]
        js_entry: Option<PathBuf>,
        #[arg(long, default_value = "bundle/main.js")]
        js_out: PathBuf,
        #[arg(long, default_value = "bun")]
        bundler: String,
    },

    /// Generate SwiftUI source from a `.crepus` template.
    #[command(name = "swiftgen")]
    SwiftGen {
        #[arg(long)]
        view: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        view_name: String,
        #[arg(long, default_value = "HyperChatContext")]
        context_type: String,
    },

    /// Generate typed Swift wrappers from JS plugin exports.
    Bindgen {
        #[arg(short, long, default_value = "plugins")]
        input: PathBuf,
        #[arg(short, long, default_value = "generated")]
        output: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Dev {
            watch,
            serve,
            port,
            no_ir,
            swiftgen_view,
            swiftgen_out,
            swiftgen_view_name,
            swiftgen_context_type,
        } => {
            let swiftgen =
                swiftgen_view
                    .zip(swiftgen_out)
                    .map(|(view, out)| hot_reload::SwiftgenConfig {
                        view,
                        out,
                        view_name: swiftgen_view_name,
                        context_type: swiftgen_context_type,
                    });
            let serve = serve.then(|| dev_loop::Serve {
                port: port.unwrap_or(hot_reload::DEFAULT_PORT),
                ir_enabled: !no_ir,
                swiftgen,
            });
            dev_loop::run(watch, serve)?;
        }
        Commands::Run { project, ios } => {
            pre_build()?;
            if ios {
                let rust_lib_dir = pre_build_ios(&project)?;
                build_swift::build_and_launch_ios(&project, &rust_lib_dir)?;
            } else {
                build_swift::build_and_launch(&project, None)?;
            }
        }
        Commands::Build { watch, out } => {
            build_all(&watch, &out)?;
        }
        Commands::New { name, aurorality } => {
            scaffold::new_project(&name, aurorality.as_deref())?;
        }
        Commands::Bundle {
            views,
            out,
            js_entry,
            js_out,
            bundler,
        } => {
            bundle::run(bundle::BundleConfig {
                views_dir: views,
                ir_out: out,
                js_entry,
                js_out,
                bundler,
            })?;
        }
        Commands::Bindgen { input, output } => {
            bindgen::run(&input, &output)?;
        }
        Commands::SwiftGen {
            view,
            out,
            view_name,
            context_type,
        } => {
            swiftgen::run(&view, &out, &view_name, &context_type)?;
        }
    }

    Ok(())
}

pub fn find_workspace_root() -> PathBuf {
    let mut dir = std::env::current_dir().unwrap_or_default();
    loop {
        if dir.join("Cargo.toml").exists() && dir.join("Cargo.lock").exists() {
            return dir;
        }
        if !dir.pop() {
            return PathBuf::from(".");
        }
    }
}

pub fn pre_build() -> Result<()> {
    if let Some(commands) = configured_pre_build_commands()? {
        for command in commands {
            let status = Command::new("sh")
                .arg("-c")
                .arg(&command)
                .status()
                .map_err(|e| anyhow::anyhow!("pre-build command `{command}`: {e}"))?;
            if !status.success() {
                anyhow::bail!(
                    "pre-build command `{}` failed ({})",
                    command,
                    status.code().unwrap_or(1)
                );
            }
        }
        return Ok(());
    }

    let ws = find_workspace_root();
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    let status = Command::new(&cargo)
        .args(["build", "-p", "aurorality-core", "--features", "js"])
        .current_dir(&ws)
        .status()
        .map_err(|e| anyhow::anyhow!("cargo build: {e}"))?;
    if !status.success() {
        anyhow::bail!("cargo build failed ({})", status.code().unwrap_or(1));
    }

    let dylib = ws.join("target/debug/libaurorality_core.dylib");
    let generated = ws.join("generated");
    let status = Command::new(&cargo)
        .args([
            "run",
            "-p",
            "aurorality-core",
            "--features",
            "js",
            "--bin",
            "uniffi-bindgen",
            "generate",
            "--library",
        ])
        .arg(&dylib)
        .args(["--language", "swift", "--out-dir"])
        .arg(&generated)
        .current_dir(&ws)
        .status()
        .map_err(|e| anyhow::anyhow!("uniffi-bindgen: {e}"))?;
    if !status.success() {
        anyhow::bail!("uniffi-bindgen failed ({})", status.code().unwrap_or(1));
    }

    let mm = generated.join("aurorality_coreFFI.modulemap");
    if mm.exists() {
        std::fs::copy(&mm, generated.join("module.modulemap"))?;
    }

    Ok(())
}

/// Rust target for the arm64 iOS Simulator (matches `arm64-apple-ios-simulator`).
pub const IOS_SIM_RUST_TARGET: &str = "aarch64-apple-ios-sim";

/// Build `aurorality-core` for the iOS Simulator and return a directory that
/// contains only the static library, so the Swift link step cannot fall back
/// to the host (macOS) build or to a dylib the app bundle does not embed.
pub fn pre_build_ios(project: &Path) -> Result<PathBuf> {
    let ws = aurorality_checkout(project)?;
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    println!("building aurorality-core for {IOS_SIM_RUST_TARGET}...");
    let status = Command::new(&cargo)
        .args([
            "build",
            "-p",
            "aurorality-core",
            "--features",
            "js",
            "--target",
            IOS_SIM_RUST_TARGET,
        ])
        .current_dir(&ws)
        .status()
        .map_err(|e| anyhow::anyhow!("cargo build --target {IOS_SIM_RUST_TARGET}: {e}"))?;
    if !status.success() {
        anyhow::bail!(
            "cargo build for {IOS_SIM_RUST_TARGET} failed ({}); is the target installed? \
             (`rustup target add {IOS_SIM_RUST_TARGET}`)",
            status.code().unwrap_or(1)
        );
    }

    let staticlib = ws
        .join("target")
        .join(IOS_SIM_RUST_TARGET)
        .join("debug/libaurorality_core.a");
    if !staticlib.is_file() {
        anyhow::bail!(
            "cargo build for {IOS_SIM_RUST_TARGET} did not produce {}",
            staticlib.display()
        );
    }
    let link_dir = ws
        .join("target")
        .join(IOS_SIM_RUST_TARGET)
        .join("aurorality-link");
    std::fs::create_dir_all(&link_dir)?;
    std::fs::copy(&staticlib, link_dir.join("libaurorality_core.a"))?;
    Ok(std::path::absolute(&link_dir)?)
}

/// The aurorality checkout whose Rust core the project links: `[aurorality].path`
/// in the project's `crepus.toml` / `.brisk.toml` (written by `aurorality new`),
/// else the enclosing Cargo workspace if it contains `aurorality-core`.
fn aurorality_checkout(project: &Path) -> Result<PathBuf> {
    for file in ["crepus.toml", ".brisk.toml"] {
        let Ok(raw) = std::fs::read_to_string(project.join(file)) else {
            continue;
        };
        let value: toml::Value = toml::from_str(&raw)?;
        if let Some(path) = value
            .get("aurorality")
            .and_then(|a| a.get("path"))
            .and_then(|p| p.as_str())
        {
            return Ok(project.join(path));
        }
    }
    let ws = find_workspace_root();
    if ws.join("crates/aurorality-core/Cargo.toml").is_file() {
        return Ok(ws);
    }
    anyhow::bail!(
        "cannot find the aurorality checkout for {}: set [aurorality] path in .brisk.toml",
        project.display()
    )
}

fn configured_pre_build_commands() -> Result<Option<Vec<String>>> {
    for file in ["crepus.toml", ".brisk.toml"] {
        let path = PathBuf::from(file);
        if path.is_file() {
            let commands = read_pre_build_commands(&path)?;
            if !commands.is_empty() {
                return Ok(Some(commands));
            }
        }
    }
    Ok(None)
}

fn read_pre_build_commands(path: &Path) -> Result<Vec<String>> {
    let raw = std::fs::read_to_string(path)?;
    let value: toml::Value = toml::from_str(&raw)?;
    let Some(commands) = value
        .get("pre_build")
        .and_then(|pre_build| pre_build.get("commands"))
        .and_then(|commands| commands.as_array())
    else {
        return Ok(Vec::new());
    };

    Ok(commands
        .iter()
        .filter_map(|command| command.as_str().map(ToString::to_string))
        .collect())
}

pub fn find_swift() -> String {
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

fn build_all(views_dir: &Path, out_dir: &Path) -> Result<()> {
    use crepuscularity_core::context::TemplateContext;
    use crepuscularity_native::{render_template_to_ir, to_json_pretty};

    std::fs::create_dir_all(out_dir)?;
    let ctx = TemplateContext::new();
    let mut count = 0;

    for entry in std::fs::read_dir(views_dir)?.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "crepus") {
            let content = std::fs::read_to_string(&path)?;
            let ir = render_template_to_ir(&content, &ctx)
                .map_err(|e| anyhow::anyhow!("failed to render {}: {e}", path.display()))?;
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            let out_path = out_dir.join(format!("{stem}.json"));
            std::fs::write(&out_path, to_json_pretty(&ir)?)?;
            println!("  {}  →  {}", path.display(), out_path.display());
            count += 1;
        }
    }
    println!("built {count} template(s)");
    Ok(())
}
