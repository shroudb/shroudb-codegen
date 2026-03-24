//! HTTP API codegen support.
//!
//! Used by shroudb-auth (and any future HTTP API projects) to generate
//! typed client libraries from a `protocol.toml` spec with `[api]` root.

pub mod generators;
pub mod spec;

use crate::generator::GeneratedFile;

/// Generate HTTP client SDK files from an API spec.
pub fn generate(
    spec_text: &str,
    lang: &str,
) -> Result<Vec<(String, Vec<GeneratedFile>)>, Box<dyn std::error::Error>> {
    generators::generate(spec_text, lang)
}
