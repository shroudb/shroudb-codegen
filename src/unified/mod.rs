//! Unified SDK code generation.
//!
//! Generates a single SDK per language from the Moat composite spec,
//! with engine-namespaced methods and dual RESP3/HTTP transport.

pub mod go;
pub mod ir;
pub mod python;
pub mod ruby;
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

/// Generate unified SDK files from a Moat composite spec.
pub fn generate(spec_text: &str, lang: &str, base_dir: &Path) -> GenerateResult {
    let moat_spec = MoatSpec::from_toml(spec_text)?;
    let resolved = moat_spec.resolve(base_dir)?;
    let ir = UnifiedIR::from_resolved(&resolved)?;

    let generators = generators_for_lang(lang)?;
    Ok(generators
        .iter()
        .map(|g| (g.language().to_string(), g.generate(&ir)))
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
