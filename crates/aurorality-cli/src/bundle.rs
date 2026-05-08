//! `aurorality bundle` — bundle JS via bun/esbuild and compile .crepus templates to IR JSON.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Context, Result};

use crepuscularity_core::context::TemplateContext;
use crepuscularity_native::{render_template_to_ir, to_json_pretty};

pub struct BundleConfig {
    pub views_dir: PathBuf,
    pub ir_out: PathBuf,
    pub js_entry: Option<PathBuf>,
    pub js_out: PathBuf,
    pub bundler: String,
}

pub fn run(config: BundleConfig) -> Result<()> {
    // 1. Bundle JS if an entry is given
    if let Some(ref entry) = config.js_entry {
        println!(
            "bundling JS: {} → {}",
            entry.display(),
            config.js_out.display()
        );
        bundle_js(entry, &config.js_out, &config.bundler)
            .with_context(|| format!("JS bundle failed (bundler={})", config.bundler))?;
        println!("  JS bundle ok");
    }

    // 2. Compile .crepus → IR JSON
    compile_templates(&config.views_dir, &config.ir_out)?;

    Ok(())
}

fn bundle_js(entry: &Path, out: &Path, bundler: &str) -> Result<()> {
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let status = match bundler {
        "bun" => Command::new("bun")
            .args([
                "build",
                entry.to_str().unwrap_or(""),
                "--outfile",
                out.to_str().unwrap_or(""),
            ])
            .status()
            .context("failed to spawn `bun`")?,
        _ => {
            // default: esbuild
            Command::new("esbuild")
                .args([
                    entry.to_str().unwrap_or(""),
                    "--bundle",
                    &format!("--outfile={}", out.display()),
                ])
                .status()
                .context("failed to spawn `esbuild`")?
        }
    };

    if !status.success() {
        return Err(anyhow!("bundler exited with status {status}"));
    }
    Ok(())
}

fn compile_templates(views_dir: &Path, out_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(out_dir)?;
    let ctx = TemplateContext::new();
    let mut count = 0;

    for entry in std::fs::read_dir(views_dir)
        .with_context(|| format!("cannot read views dir: {}", views_dir.display()))?
        .flatten()
    {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "crepus") {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("read {}", path.display()))?;
            let ir = render_template_to_ir(&content, &ctx)
                .map_err(|e| anyhow!("render {}: {e}", path.display()))?;
            let json = to_json_pretty(&ir)?;
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            let out_path = out_dir.join(format!("{stem}.json"));
            std::fs::write(&out_path, json)
                .with_context(|| format!("write {}", out_path.display()))?;
            println!("  {}  →  {}", path.display(), out_path.display());
            count += 1;
        }
    }

    println!("compiled {count} template(s)");
    Ok(())
}
