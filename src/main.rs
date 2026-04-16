//! shroudb-codegen — unified SDK generator for all ShrouDB engines.
//!
//! Usage:
//!   # Unified RESP3 SDK (all engines)
//!   shroudb-codegen --spec ../shroudb-moat/protocol.toml --lang all --output generated/
//!
//!   # HTTP REST SDK (Sigil)
//!   shroudb-codegen --spec ../shroudb-sigil/protocol.toml --lang all --output generated-http/ --http

use clap::Parser;
use shroudb_codegen::cli::{CodegenCli, run};

/// SDK version baked in at compile time from `VERSION` at the repo root.
/// This is the ShrouDB client SDK's own version — explicitly independent
/// of any single engine's protocol version. Bump this file and rebuild to
/// cut a new SDK release.
const SDK_VERSION: &str = include_str!("../VERSION");

#[derive(Parser)]
#[command(
    name = "shroudb-codegen",
    about = "Generate ShrouDB SDKs from protocol specs",
    long_about = "Generates unified RESP3 SDKs from the Moat composite spec, \
                  or HTTP REST SDKs from individual engine specs with --http."
)]
struct Cli {
    #[command(flatten)]
    inner: CodegenCli,
}

fn main() {
    let cli = Cli::parse();
    let spec_path = cli.inner.spec.clone();
    let is_http = cli.inner.http;
    let sdk_version = SDK_VERSION.trim();

    run(&cli.inner, |spec_text, lang| {
        let base_dir = spec_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));

        if is_http {
            shroudb_codegen::unified::generate_http(spec_text, lang, base_dir)
        } else {
            shroudb_codegen::unified::generate(spec_text, lang, base_dir, sdk_version)
        }
    });
}
