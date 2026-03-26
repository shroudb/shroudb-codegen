//! Moat composite spec: references multiple engine specs and adds Moat-specific
//! meta-commands and control plane endpoints.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::wire::spec::ProtocolSpec;

/// Top-level Moat composite spec.
#[derive(Debug, Deserialize)]
pub struct MoatSpec {
    pub protocol: MoatProtocolMeta,
    #[serde(default)]
    pub engines: Vec<EngineRef>,
    #[serde(default)]
    pub meta_commands: BTreeMap<String, MetaCommandDef>,
    #[serde(default)]
    pub control_plane: BTreeMap<String, ControlEndpointDef>,
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
    /// Path to the engine's protocol.toml (relative to the Moat spec).
    pub spec: String,
    /// Transports the engine supports in Moat.
    #[serde(default)]
    pub transport: Vec<String>,
    /// HTTP path prefix.
    pub http_prefix: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MetaCommandDef {
    pub verb: String,
    pub description: String,
    #[serde(default)]
    pub params: Vec<ParamDef>,
    #[serde(default)]
    pub response: Vec<FieldDef>,
}

#[derive(Debug, Deserialize)]
pub struct ControlEndpointDef {
    pub method: String,
    pub path: String,
    pub description: String,
    #[serde(default)]
    pub auth: Option<String>,
    #[serde(default)]
    pub body: Vec<FieldDef>,
    #[serde(default)]
    pub response: Vec<FieldDef>,
}

#[derive(Debug, Deserialize)]
pub struct ParamDef {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: String,
    #[serde(default)]
    pub required: bool,
    pub position: Option<usize>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FieldDef {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Debug, Deserialize)]
pub struct SdkConfig {
    #[serde(default)]
    pub namespace_by_engine: bool,
    pub default_transport: Option<String>,
    #[serde(default)]
    pub resp3_engine_selection: bool,
    #[serde(default)]
    pub languages: Vec<String>,
    pub packages: Option<SdkPackages>,
}

#[derive(Debug, Deserialize)]
pub struct SdkPackages {
    pub typescript: Option<String>,
    pub python: Option<String>,
    pub go: Option<String>,
    pub ruby: Option<String>,
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
    /// `base_dir` is the directory containing the Moat spec file.
    ///
    /// Only wire protocol specs (`[protocol]` section) are loaded. HTTP API specs
    /// (`[api]` section) are skipped — the auth engine's methods are hand-written
    /// in the Moat SDK generator since they compose vault commands.
    pub fn resolve(self, base_dir: &Path) -> Result<ResolvedMoatSpec, Box<dyn std::error::Error>> {
        let mut engine_specs = BTreeMap::new();

        for engine in &self.engines {
            if engine.spec.is_empty() {
                continue; // Spec not yet available (pending).
            }
            let spec_path = base_dir.join(&engine.spec);
            let spec_text = std::fs::read_to_string(&spec_path).map_err(|e| {
                format!(
                    "failed to read engine spec for '{}' at {}: {e}",
                    engine.name,
                    spec_path.display()
                )
            })?;

            // Only load wire protocol specs. Skip HTTP API specs.
            if spec_text.contains("\n[protocol]") || spec_text.starts_with("[protocol]") {
                let spec = ProtocolSpec::from_toml(&spec_text)?;
                engine_specs.insert(engine.name.clone(), spec);
            }
        }

        Ok(ResolvedMoatSpec {
            moat: self,
            engine_specs,
        })
    }
}
