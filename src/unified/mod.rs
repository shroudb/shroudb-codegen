//! Unified SDK code generation.
//!
//! Generates a single SDK per language from the Moat composite spec,
//! with engine-namespaced methods and dual RESP3/HTTP transport.
//!
//! Also supports HTTP-only SDK generation via `generate_http` for engines
//! with REST API endpoints (e.g., Sigil).

pub mod compat;
pub mod go;
pub mod ir;
pub mod python;
pub mod ruby;
pub mod sigil_http;
pub mod typescript;

use std::path::Path;

use crate::generator::{GenerateResult, GeneratedFile};
use crate::spec::moat::MoatSpec;
use ir::UnifiedIR;

/// Trait implemented by each unified language generator.
pub trait UnifiedGenerator {
    fn language(&self) -> &'static str;
    fn generate(&self, ir: &UnifiedIR) -> Vec<GeneratedFile>;
}

/// Generate unified RESP3 SDK files from a Moat composite spec.
///
/// `sdk_version` is the ShrouDB client SDK's own version, independent of
/// any individual engine's protocol version. Sourced from the `VERSION`
/// file at the codegen repo root and baked into the binary at compile time.
pub fn generate(spec_text: &str, lang: &str, base_dir: &Path, sdk_version: &str) -> GenerateResult {
    let moat_spec = MoatSpec::from_toml(spec_text)?;
    let resolved = moat_spec.resolve(base_dir)?;
    let ir = UnifiedIR::from_resolved(&resolved, sdk_version)?;

    let generators = generators_for_lang(lang)?;
    let compat = compat::generate(&ir);
    Ok(generators
        .iter()
        .map(|g| {
            let mut files = g.generate(&ir);
            files.push(GeneratedFile {
                path: compat.path.clone(),
                content: compat.content.clone(),
            });
            (g.language().to_string(), files)
        })
        .collect())
}

/// Generate HTTP REST SDK from an engine spec with `http` annotations.
///
/// Reads a single engine's protocol.toml (not the Moat composite), finds
/// commands with HTTP endpoints, and generates a standalone HTTP client.
/// If `sdk_version` is supplied it overrides the engine's own protocol
/// version; otherwise the engine's protocol.version is used.
pub fn generate_http(
    spec_text: &str,
    lang: &str,
    _base_dir: &Path,
    sdk_version: Option<&str>,
) -> GenerateResult {
    // Wrap the single spec in a synthetic Moat envelope so we can reuse the IR.
    let spec = crate::spec::wire::ProtocolSpec::from_toml(spec_text)?;
    let engine_name = spec
        .protocol
        .name
        .replace("shroudb-", "")
        .replace("shroudb", "core");

    let mut ir = ir::UnifiedIR::from_single_engine(&engine_name, &spec)?;
    if let Some(v) = sdk_version {
        ir.version = v.to_string();
    }

    // Find the engine with HTTP endpoints.
    let engine = ir
        .engines
        .iter()
        .find(|e| e.has_http_api)
        .ok_or_else(|| format!("No HTTP API found in spec for '{}'", engine_name))?;

    let generators = sigil_http::generators_for_lang(lang)?;
    Ok(generators
        .iter()
        .map(|g| (g.language().to_string(), g.generate(engine, &ir)))
        .collect())
}

fn generators_for_lang(
    lang: &str,
) -> Result<Vec<Box<dyn UnifiedGenerator>>, Box<dyn std::error::Error>> {
    match lang {
        "typescript" | "ts" => Ok(vec![Box::new(typescript::TypeScriptUnifiedGenerator)]),
        "python" | "py" => Ok(vec![Box::new(python::PythonUnifiedGenerator)]),
        "go" | "golang" => Ok(vec![Box::new(go::GoUnifiedGenerator)]),
        "ruby" | "rb" => Ok(vec![Box::new(ruby::RubyUnifiedGenerator)]),
        "all" => Ok(vec![
            Box::new(typescript::TypeScriptUnifiedGenerator),
            Box::new(python::PythonUnifiedGenerator),
            Box::new(go::GoUnifiedGenerator),
            Box::new(ruby::RubyUnifiedGenerator),
        ]),
        other => Err(format!(
            "Unknown language: {other}\nSupported: typescript, python, go, ruby, all"
        )
        .into()),
    }
}
