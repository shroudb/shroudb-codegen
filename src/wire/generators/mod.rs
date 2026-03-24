pub mod go;
pub mod proto;
pub mod python;
pub mod ruby;
pub mod typescript;

use crate::generator::{GeneratedFile, Naming};
use super::spec::ProtocolSpec;

/// Trait implemented by each wire protocol language generator.
pub trait Generator {
    /// Human-readable name of the target language (e.g. "Python", "TypeScript").
    fn language(&self) -> &'static str;

    /// Generate all output files from the protocol spec.
    fn generate(&self, spec: &ProtocolSpec) -> Vec<GeneratedFile>;
}

/// Construct [`Naming`] from a wire protocol spec.
pub(crate) fn naming_from_spec(spec: &ProtocolSpec) -> Naming {
    Naming::new(
        &spec.protocol.name,
        &spec.protocol.description,
        spec.protocol.default_port,
        &spec.protocol.uri_schemes,
    )
}

/// Build the set of generators for a language string.
pub fn generators_for_lang(
    lang: &str,
) -> Result<Vec<Box<dyn Generator>>, Box<dyn std::error::Error>> {
    match lang {
        "python" | "py" => Ok(vec![Box::new(python::PythonGenerator)]),
        "typescript" | "ts" => Ok(vec![Box::new(typescript::TypeScriptGenerator)]),
        "go" | "golang" => Ok(vec![Box::new(go::GoGenerator)]),
        "ruby" | "rb" => Ok(vec![Box::new(ruby::RubyGenerator)]),
        "proto" | "protobuf" | "grpc" => Ok(vec![Box::new(proto::ProtoGenerator)]),
        "all" => Ok(vec![
            Box::new(python::PythonGenerator),
            Box::new(typescript::TypeScriptGenerator),
            Box::new(go::GoGenerator),
            Box::new(ruby::RubyGenerator),
            Box::new(proto::ProtoGenerator),
        ]),
        other => Err(
            format!("Unknown language: {other}\nSupported: python, typescript, go, ruby, proto, all")
                .into(),
        ),
    }
}
