//! Moat composite spec: references multiple engine specs and adds Moat-specific
//! meta-commands and SDK configuration.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use super::wire::ProtocolSpec;

/// Top-level Moat composite spec.
#[derive(Debug, Deserialize)]
pub struct MoatSpec {
    pub protocol: MoatProtocolMeta,
    #[serde(default)]
    pub engines: Vec<EngineRef>,
    #[serde(default)]
    pub meta: Option<MetaSection>,
    pub sdk: Option<SdkConfig>,
}

#[derive(Debug, Deserialize)]
pub struct MoatProtocolMeta {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default = "default_http_port")]
    pub default_http_port: u16,
    #[serde(default = "default_resp3_port")]
    pub default_resp3_port: u16,
}

#[derive(Debug, Deserialize)]
pub struct EngineRef {
    pub name: String,
    pub spec: String,
    #[serde(default)]
    pub transport: Vec<String>,
    pub http_prefix: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MetaSection {
    pub description: Option<String>,
    #[serde(default)]
    pub transport: Vec<String>,
    #[serde(default)]
    pub commands: Vec<MetaCommandDef>,
}

#[derive(Debug, Deserialize)]
pub struct MetaCommandDef {
    pub name: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct SdkConfig {
    #[serde(default)]
    pub namespace_by: Option<String>,
    pub default_transport: Option<String>,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub typescript: Option<SdkLangConfig>,
    #[serde(default)]
    pub python: Option<SdkLangConfig>,
    #[serde(default)]
    pub go: Option<SdkLangConfig>,
    #[serde(default)]
    pub ruby: Option<SdkLangConfig>,
}

#[derive(Debug, Deserialize)]
pub struct SdkLangConfig {
    #[serde(alias = "package", alias = "gem")]
    pub module: Option<String>,
}

fn default_http_port() -> u16 {
    8200
}
fn default_resp3_port() -> u16 {
    8201
}

/// Resolved Moat spec with loaded engine sub-specs.
pub struct ResolvedMoatSpec {
    pub moat: MoatSpec,
    pub engine_specs: BTreeMap<String, ProtocolSpec>,
}

impl MoatSpec {
    pub fn from_toml(text: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(toml::from_str(text)?)
    }

    /// Resolve engine spec references by loading their protocol.toml files.
    pub fn resolve(self, base_dir: &Path) -> Result<ResolvedMoatSpec, Box<dyn std::error::Error>> {
        let mut engine_specs = BTreeMap::new();

        for engine in &self.engines {
            if engine.spec.is_empty() {
                continue;
            }
            let spec_path = base_dir.join(&engine.spec);
            let spec_text = std::fs::read_to_string(&spec_path).map_err(|e| {
                format!(
                    "failed to read engine spec for '{}' at {}: {e}",
                    engine.name,
                    spec_path.display()
                )
            })?;

            if spec_text.contains("\n[protocol]") || spec_text.starts_with("[protocol]") {
                let spec = ProtocolSpec::from_toml(&spec_text).map_err(|e| {
                    format!("failed to parse engine spec for '{}': {e}", engine.name)
                })?;
                engine_specs.insert(engine.name.clone(), spec);
            }
        }

        Ok(ResolvedMoatSpec {
            moat: self,
            engine_specs,
        })
    }
}
