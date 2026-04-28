//! `aurorality` CLI — dev server, build, and project scaffolding.

mod dev_server;
mod scaffold;

use std::path::PathBuf;

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
    /// Start the hot-reload dev server and file watcher.
    ///
    /// Launch the Aurorality Runner app on your simulator/device, then connect
    /// it to the address printed by this command.
    Dev {
        /// Directory of .crepus files to watch.
        #[arg(short, long, default_value = "views")]
        watch: PathBuf,

        /// WebSocket port for the Runner app to connect to.
        #[arg(short, long, default_value_t = 47832)]
        port: u16,
    },

    /// Render all .crepus files in a directory to JSON IR for bundling.
    Build {
        /// Directory of .crepus files to render.
        #[arg(short, long, default_value = "views")]
        watch: PathBuf,

        /// Output directory for the JSON IR files.
        #[arg(short, long, default_value = "generated/ir")]
        out: PathBuf,
    },

    /// Scaffold a new aurorality project.
    New {
        /// Name of the new project.
        name: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Dev { watch, port } => {
            dev_server::run(watch, port).await?;
        }
        Commands::Build { watch, out } => {
            build_all(&watch, &out)?;
        }
        Commands::New { name } => {
            scaffold::new_project(&name)?;
        }
    }

    Ok(())
}

fn build_all(views_dir: &PathBuf, out_dir: &PathBuf) -> Result<()> {
    use crepuscularity_core::context::TemplateContext;
    use crepuscularity_native::{render_template_to_ir, to_json_pretty};

    std::fs::create_dir_all(out_dir)?;
    let ctx = TemplateContext::new();

    let mut count = 0;
    for entry in std::fs::read_dir(views_dir)?.flatten() {
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "crepus") {
            let content = std::fs::read_to_string(&path)?;
            let ir = render_template_to_ir(&content, &ctx).map_err(|e| {
                anyhow::anyhow!("failed to render {}: {e}", path.display())
            })?;
            let json = to_json_pretty(&ir)?;
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            let out_path = out_dir.join(format!("{stem}.json"));
            std::fs::write(&out_path, json)?;
            println!("  {}  →  {}", path.display(), out_path.display());
            count += 1;
        }
    }

    println!("built {count} template(s)");
    Ok(())
}
