//! Moat composite spec codegen.
//!
//! Generates unified SDKs from a Moat composite spec that references
//! multiple engine protocol.toml files. The produced SDK has engine-namespaced
//! methods: `client.vault.verify()`, `client.transit.encrypt()`, etc.

pub mod generators;
pub mod spec;

use std::path::Path;

use crate::generator::GenerateResult;

/// Generate unified Moat SDK files from a composite spec.
///
/// `spec_text` is the content of the Moat `protocol.toml`.
/// `base_dir` is the directory containing the spec (for resolving relative paths).
pub fn generate(spec_text: &str, lang: &str, base_dir: &Path) -> GenerateResult {
    let moat_spec = spec::MoatSpec::from_toml(spec_text)?;
    let resolved = moat_spec.resolve(base_dir)?;
    let gens = generators::generators_for_lang(lang)?;
    Ok(gens
        .iter()
        .map(|g| (g.language().to_string(), g.generate(&resolved)))
        .collect())
}
