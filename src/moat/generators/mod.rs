pub mod typescript;

use super::spec::ResolvedMoatSpec;
use crate::generator::GeneratedFile;

/// Trait for Moat unified SDK generators.
pub trait MoatGenerator {
    fn language(&self) -> &'static str;
    fn generate(&self, spec: &ResolvedMoatSpec) -> Vec<GeneratedFile>;
}

pub fn generators_for_lang(
    lang: &str,
) -> Result<Vec<Box<dyn MoatGenerator>>, Box<dyn std::error::Error>> {
    match lang {
        "typescript" | "ts" => Ok(vec![Box::new(typescript::TypeScriptMoatGenerator)]),
        "all" => Ok(vec![Box::new(typescript::TypeScriptMoatGenerator)]),
        other => Err(format!(
            "Moat SDK generation not yet supported for: {other}\nCurrently supported: typescript, all"
        )
        .into()),
    }
}
