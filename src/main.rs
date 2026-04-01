//! shroudb-codegen — unified SDK generator for all ShrouDB engines.
//!
//! Reads the Moat composite protocol.toml (which references all engine specs)
//! and generates a single SDK per language with engine-namespaced methods.
//!
//! Usage:
//!   shroudb-codegen --spec ../shroudb-moat/protocol.toml --lang typescript --output generated/typescript
//!   shroudb-codegen --spec ../shroudb-moat/protocol.toml --lang all --output generated/

use clap::Parser;
use shroudb_codegen::cli::{CodegenCli, run};

#[derive(Parser)]
#[command(
    name = "shroudb-codegen",
    about = "Generate unified ShrouDB SDK from the Moat composite spec",
    long_about = "Reads the Moat protocol.toml (which references all engine specs) \
                  and produces a single SDK per language with engine-namespaced methods, \
                  dual RESP3/HTTP transport, and full documentation."
)]
struct Cli {
    #[command(flatten)]
    inner: CodegenCli,
}

fn main() {
    let cli = Cli::parse();
    let spec_path = cli.inner.spec.clone();
    run(&cli.inner, |spec_text, lang| {
        let base_dir = spec_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        shroudb_codegen::unified::generate(spec_text, lang, base_dir)
    });
}
