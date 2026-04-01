//! HTTP REST SDK generator for engines with HTTP API endpoints.
//!
//! Currently used for Sigil's REST API. Generates a standalone HTTP client
//! package per language with proper REST semantics (GET/POST/PATCH/DELETE,
//! path parameters, JSON request bodies).

mod go;
mod python;
mod ruby;
mod typescript;

use super::ir::{EngineIR, UnifiedIR};
use crate::generator::GeneratedFile;

/// Trait for HTTP SDK language generators.
pub trait HttpSdkGenerator {
    fn language(&self) -> &'static str;
    fn generate(&self, engine: &EngineIR, ir: &UnifiedIR) -> Vec<GeneratedFile>;
}

pub fn generators_for_lang(
    lang: &str,
) -> Result<Vec<Box<dyn HttpSdkGenerator>>, Box<dyn std::error::Error>> {
    match lang {
        "typescript" | "ts" => Ok(vec![Box::new(typescript::TsHttpGenerator)]),
        "python" | "py" => Ok(vec![Box::new(python::PyHttpGenerator)]),
        "go" | "golang" => Ok(vec![Box::new(go::GoHttpGenerator)]),
        "ruby" | "rb" => Ok(vec![Box::new(ruby::RbHttpGenerator)]),
        "all" => Ok(vec![
            Box::new(typescript::TsHttpGenerator),
            Box::new(python::PyHttpGenerator),
            Box::new(go::GoHttpGenerator),
            Box::new(ruby::RbHttpGenerator),
        ]),
        other => Err(format!(
            "Unknown language: {other}\nSupported: typescript, python, go, ruby, all"
        )
        .into()),
    }
}
