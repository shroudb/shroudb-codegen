//! shroudb-codegen — unified SDK generator for all ShrouDB protocols.
//!
//! Auto-detects the spec format:
//!   - `[protocol]` → wire protocol (RESP3) client
//!   - `[api]`      → HTTP API client
//!
//! Usage:
//!   shroudb-codegen --spec protocol.toml --lang python --output generated/python
//!   shroudb-codegen --spec protocol.toml --lang all --output generated/

use clap::Parser;
use shroudb_codegen::cli::{CodegenCli, run};

#[derive(Parser)]
#[command(
    name = "shroudb-codegen",
    about = "Generate typed client libraries from a ShrouDB protocol spec",
    long_about = "Reads a protocol.toml and produces ready-to-publish client \
                  packages. Supports both wire protocol (RESP3) and HTTP API specs."
)]
struct Cli {
    #[command(flatten)]
    inner: CodegenCli,
}

fn main() {
    let cli = Cli::parse();
    run(&cli.inner, |spec_text, lang| {
        if spec_text.contains("\n[protocol]") || spec_text.starts_with("[protocol]") {
            shroudb_codegen::wire::generate(spec_text, lang)
        } else if spec_text.contains("\n[api]") || spec_text.starts_with("[api]") {
            shroudb_codegen::http::generate(spec_text, lang)
        } else {
            Err("Unknown spec format. Expected [protocol] (wire) or [api] (HTTP) section.".into())
        }
    });
}
