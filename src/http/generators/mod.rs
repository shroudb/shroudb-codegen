pub mod go;
pub mod proto;
pub mod python;
pub mod ruby;
pub mod typescript;

use super::spec::ApiSpec;
use crate::generator::{GeneratedFile, Naming};

/// Trait implemented by each HTTP API language generator.
pub trait HttpGenerator {
    fn language(&self) -> &'static str;
    fn generate(&self, spec: &ApiSpec, naming: &Naming) -> Vec<GeneratedFile>;
}

/// Construct [`Naming`] from an HTTP API spec.
fn naming_from_spec(spec: &ApiSpec) -> Naming {
    Naming::new(
        &spec.api.name,
        &spec.api.description,
        spec.api.default_port,
        &[], // HTTP APIs don't have URI schemes
    )
}

/// Entry point: generate HTTP client SDK files from a spec.
pub fn generate(
    spec_text: &str,
    lang: &str,
) -> Result<Vec<(String, Vec<GeneratedFile>)>, Box<dyn std::error::Error>> {
    let spec = ApiSpec::from_toml(spec_text)?;
    let naming = naming_from_spec(&spec);
    let generators: Vec<Box<dyn HttpGenerator>> = match lang {
        "python" | "py" => vec![Box::new(python::PythonGenerator)],
        "typescript" | "ts" => vec![Box::new(typescript::TypeScriptGenerator)],
        "go" | "golang" => vec![Box::new(go::GoGenerator)],
        "ruby" | "rb" => vec![Box::new(ruby::RubyGenerator)],
        "proto" | "grpc" | "protobuf" => vec![Box::new(proto::ProtoGenerator)],
        "all" => vec![
            Box::new(python::PythonGenerator),
            Box::new(typescript::TypeScriptGenerator),
            Box::new(go::GoGenerator),
            Box::new(ruby::RubyGenerator),
            Box::new(proto::ProtoGenerator),
        ],
        other => {
            return Err(
                format!("Unknown language: {other}\nSupported: python, typescript, go, ruby, all")
                    .into(),
            )
        }
    };
    Ok(generators
        .iter()
        .map(|g| (g.language().to_string(), g.generate(&spec, &naming)))
        .collect())
}
