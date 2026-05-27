//! `aurorality` CLI — dev server, build, and project scaffolding.

mod bindgen;
mod build_swift;
mod bundle;
mod dev_server;
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
    /// Start hot-reload dev server and launch the app in a native window.
    ///
    /// Pre-builds Rust core + bindings, starts the WebSocket hot-reload
    /// server, then builds and launches the SwiftUI app.
    /// Changes to .crepus files hot-reload the UI without restarting.
    Dev {
        /// Directory of `.crepus` files to watch.
        #[arg(short, long, default_value = "views")]
        watch: PathBuf,

        /// WebSocket port for hot-reload clients.
        #[arg(short, long, default_value_t = 47832)]
        port: u16,

        /// Path to the `.crepus` file for `swiftgen` (hybrid reload).
        #[arg(long)]
        swiftgen_view: Option<PathBuf>,
        #[arg(long)]
        swiftgen_out: Option<PathBuf>,
        #[arg(long)]
        swiftgen_name: Option<String>,
        #[arg(long)]
        swiftgen_context_type: Option<String>,

        /// Skip IR diff/render; only swiftgen events.
        #[arg(long, default_value_t = false)]
        no_ir: bool,
    },

    /// Build and launch the SwiftUI app (no hot-reload server).
    Run {
        /// Project directory.
        #[arg(default_value = ".")]
        project: PathBuf,
    },

    /// Render all .crepus files in a directory to JSON IR for bundling.
    Build {
        #[arg(short, long, default_value = "views")]
        watch: PathBuf,
        #[arg(short, long, default_value = "generated/ir")]
        out: PathBuf,
    },

    /// Scaffold a new aurorality project.
    New {
        name: String,
    },

    /// Bundle JS (via bun/esbuild) + compile .crepus templates to IR JSON.
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

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Dev { watch, port, swiftgen_view, swiftgen_out, swiftgen_name, swiftgen_context_type, no_ir } => {
            let ir_enabled = !no_ir;
            let swiftgen = match (swiftgen_view, swiftgen_out, swiftgen_name, swiftgen_context_type) {
                (Some(view), Some(out), Some(view_name), Some(context_type)) => {
                    Some(dev_server::SwiftgenDevConfig {
                        view: std::fs::canonicalize(&view).unwrap_or(view),
                        out, view_name, context_type,
                    })
                }
                (None, None, None, None) => None,
                _ => anyhow::bail!("swiftgen requires --swiftgen-view, --swiftgen-out, --swiftgen-name, and --swiftgen-context-type"),
            };
            if no_ir && swiftgen.is_none() {
                anyhow::bail!("--no-ir requires --swiftgen-view (and related swiftgen flags)");
            }

            pre_build()?;

            let dev_port = port;
            let project = std::env::current_dir()?;
            tokio::task::spawn_blocking(move || {
                if let Err(e) = build_swift::build_and_launch(&project, Some(dev_port)) {
                    eprintln!("app launch failed: {e}");
                }
            });

            dev_server::run(dev_server::DevServerConfig {
                watch_dir: watch,
                port,
                swiftgen,
                ir_enabled,
            }).await?;
        }
        Commands::Run { project } => {
            pre_build()?;
            build_swift::build_and_launch(&project, None)?;
        }
        Commands::Build { watch, out } => {
            build_all(&watch, &out)?;
        }
        Commands::New { name } => {
            scaffold::new_project(&name)?;
        }
        Commands::Bundle { views, out, js_entry, js_out, bundler } => {
            bundle::run(bundle::BundleConfig {
                views_dir: views, ir_out: out, js_entry, js_out, bundler,
            })?;
        }
        Commands::Bindgen { input, output } => {
            bindgen::run(&input, &output)?;
        }
        Commands::SwiftGen { view, out, view_name, context_type } => {
            swiftgen::run(&view, &out, &view_name, &context_type)?;
        }
    }

    Ok(())
}

fn find_workspace_root() -> PathBuf {
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

fn pre_build() -> Result<()> {
    let ws = find_workspace_root();
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    println!("building aurorality-core...");
    run_cmd(
        Command::new(&cargo).args(["build", "-p", "aurorality-core", "--features", "js"])
            .current_dir(&ws),
        "cargo build",
    )?;

    println!("generating Swift bindings...");
    let dylib = ws.join("target/debug/libaurorality_core.dylib");
    let generated = ws.join("generated");
    run_cmd(
        Command::new(&cargo)
            .args(["run", "-p", "aurorality-core", "--features", "js", "--bin", "uniffi-bindgen",
                   "generate", "--library"])
            .arg(&dylib)
            .args(["--language", "swift", "--out-dir"])
            .arg(&generated)
            .current_dir(&ws),
        "uniffi-bindgen",
    )?;
    let mm = generated.join("aurorality_coreFFI.modulemap");
    if mm.exists() {
        std::fs::copy(&mm, generated.join("module.modulemap"))?;
    }

    Ok(())
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

fn run_cmd(cmd: &mut Command, label: &str) -> Result<()> {
    let status = cmd.status().map_err(|e| anyhow::anyhow!("{label}: {e}"))?;
    if !status.success() {
        anyhow::bail!("{label} failed ({})", status.code().unwrap_or(1));
    }
    Ok(())
}
