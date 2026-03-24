//! Shared generator types and output utilities.

use heck::{ToLowerCamelCase, ToPascalCase, ToSnakeCase};
use std::path::Path;

/// A named output file produced by a generator.
pub struct GeneratedFile {
    /// Relative path within the output directory.
    pub path: String,
    /// File contents.
    pub content: String,
}

/// Naming conventions derived from a protocol/API name.
/// Generators use this instead of hardcoding names.
pub struct Naming {
    /// Raw name (e.g., "shroudb" or "shroudb-transit" or "shroudb-auth")
    pub raw: String,
    /// Snake case (e.g., "shroudb" or "shroudb_transit")
    pub snake: String,
    /// PascalCase (e.g., "Shroudb" or "ShroudbTransit")
    pub pascal: String,
    /// camelCase (e.g., "shroudb" or "shroudbTransit")
    pub camel: String,
    /// Hyphenated (e.g., "shroudb" or "shroudb-transit") — for package names
    pub kebab: String,
    /// npm package name (e.g., "shroudb-client" or "shroudb-transit-client")
    pub npm_name: String,
    /// Go module path
    pub go_module: String,
    /// Description from spec
    pub description: String,
    /// Default port
    pub default_port: u16,
    /// URI schemes (wire protocols) or base URL schemes (HTTP APIs)
    pub uri_schemes: Vec<String>,
}

impl Naming {
    pub fn new(name: &str, description: &str, default_port: u16, uri_schemes: &[String]) -> Self {
        let raw = name.to_string();
        let snake = raw.to_snake_case();
        let pascal = raw.to_pascal_case();
        let camel = raw.to_lower_camel_case();
        let kebab = raw.clone();
        let npm_name = format!("{kebab}-client");
        let go_module = format!("github.com/shroudb/{kebab}-go");

        Self {
            raw,
            snake,
            pascal,
            camel,
            kebab,
            npm_name,
            go_module,
            description: description.to_string(),
            default_port,
            uri_schemes: uri_schemes.to_vec(),
        }
    }
}

/// Write all generated files to the output directory.
pub fn write_output(files: &[GeneratedFile], output_dir: &Path) -> std::io::Result<()> {
    for file in files {
        let path = output_dir.join(&file.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, &file.content)?;
    }
    Ok(())
}
