//! Shared CLI argument parsing and codegen runner.

use crate::generator::{GenerateResult, write_output};
use clap::Args;
use std::path::PathBuf;

/// CLI arguments shared by all codegen binaries.
///
/// Embed in your binary's own `#[derive(Parser)]` struct via `#[command(flatten)]`.
#[derive(Args)]
pub struct CodegenCli {
    /// Path to the protocol/API spec file
    #[arg(short, long, default_value = "protocol.toml")]
    pub spec: PathBuf,

    /// Target language: python, typescript, go, ruby, or all
    #[arg(short, long)]
    pub lang: String,

    /// Output directory for generated code
    #[arg(short, long, default_value = "generated")]
    pub output: PathBuf,

    /// Print what would be generated without writing files
    #[arg(long)]
    pub dry_run: bool,
}

/// Run the codegen pipeline.
///
/// `generate` takes `(spec_text, lang)` and returns `Vec<(language_name, files)>`.
pub fn run(cli: &CodegenCli, generate: impl Fn(&str, &str) -> GenerateResult) {
    let spec_text = std::fs::read_to_string(&cli.spec).unwrap_or_else(|e| {
        eprintln!("Error reading spec file {:?}: {e}", cli.spec);
        std::process::exit(1);
    });

    let results = generate(&spec_text, &cli.lang).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        std::process::exit(1);
    });

    for (lang_name, files) in &results {
        let lang_dir = if results.len() > 1 {
            cli.output.join(lang_name.to_lowercase())
        } else {
            cli.output.clone()
        };

        if cli.dry_run {
            println!("\n=== {} ({} files) ===", lang_name, files.len());
            for f in files {
                println!("  {}", lang_dir.join(&f.path).display());
            }
        } else {
            write_output(files, &lang_dir).unwrap_or_else(|e| {
                eprintln!("Error writing {} output: {e}", lang_name);
                std::process::exit(1);
            });
            println!(
                "Generated {} {} files in {}",
                files.len(),
                lang_name,
                lang_dir.display()
            );
        }
    }
}
