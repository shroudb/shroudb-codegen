//! Wire protocol (RESP3) codegen support.
//!
//! Used by `shroudb-codegen` and `shroudb-transit-codegen` to generate
//! typed client libraries from a `protocol.toml` spec.

pub mod generators;
pub mod spec;

use crate::generator::GenerateResult;
use spec::ProtocolSpec;

/// Generate client SDK files for a wire protocol spec.
///
/// This is the entry point used by thin codegen binaries.
pub fn generate(spec_text: &str, lang: &str) -> GenerateResult {
    let spec = ProtocolSpec::from_toml(spec_text)?;
    let gens = generators::generators_for_lang(lang)?;
    Ok(gens
        .iter()
        .map(|g| (g.language().to_string(), g.generate(&spec)))
        .collect())
}
