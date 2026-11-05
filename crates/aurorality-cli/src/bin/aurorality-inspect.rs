//! Inspect rendered IR without a SwiftUI runtime.

use std::io::{self, Read};

use anyhow::{Context, Result};
use clap::Parser;

#[derive(Parser)]
#[command(about = "Render a template from stdin to JSON IR (not a native preview)")]
struct Args {
    /// JSON object containing template variables.
    #[arg(long, default_value = "{}")]
    context: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut template = String::new();
    io::stdin()
        .read_to_string(&mut template)
        .context("reading template from stdin")?;
    let ir = aurorality_core::render::render(&template, &args.context)?;
    let value: serde_json::Value = serde_json::from_str(&ir)?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}
