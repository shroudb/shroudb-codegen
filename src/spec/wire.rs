//! Wire protocol specification parser.
//!
//! Deserializes `protocol.toml` into a flexible representation that handles
//! the varied formats across engines (standard verb/params, Sigil syntax,
//! Veil condensed, etc.).

use std::collections::BTreeMap;

/// Parsed engine protocol spec — stores raw TOML values for flexible access.
/// Different engines use slightly different field names and structures.
pub struct ProtocolSpec {
    pub protocol: ProtocolMeta,
    pub error_codes: BTreeMap<String, ErrorCodeDef>,
    pub commands: BTreeMap<String, toml::Value>,
    pub types: BTreeMap<String, toml::Value>,
}

pub struct ProtocolMeta {
    pub name: String,
    pub version: String,
    pub description: String,
    pub default_tcp_port: u16,
    pub default_http_port: Option<u16>,
    pub uri_schemes: Vec<String>,
}

pub struct ErrorCodeDef {
    pub description: String,
    pub http_equiv: u16,
}

impl ProtocolSpec {
    /// Parse a protocol spec from TOML text, handling format variations.
    pub fn from_toml(text: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let root: toml::Value = toml::from_str(text)?;
        let root = root.as_table().ok_or("spec root must be a table")?;

        // Protocol metadata.
        let proto_table = root
            .get("protocol")
            .and_then(|v| v.as_table())
            .ok_or("missing [protocol] section")?;

        let name = proto_table
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let version = proto_table
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("1.0.0")
            .to_string();
        let description = proto_table
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or(&name)
            .to_string();

        let default_tcp_port = proto_table
            .get("default_tcp_port")
            .or_else(|| proto_table.get("default_port"))
            .and_then(|v| v.as_integer())
            .unwrap_or(6399) as u16;

        let default_http_port = proto_table
            .get("default_http_port")
            .and_then(|v| v.as_integer())
            .map(|v| v as u16);

        // URI schemes — handle both `uri_schemes` (array) and `uri_scheme` (string).
        let uri_schemes =
            if let Some(arr) = proto_table.get("uri_schemes").and_then(|v| v.as_array()) {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            } else if let Some(s) = proto_table.get("uri_scheme").and_then(|v| v.as_str()) {
                vec![format!("{s}://"), format!("{s}+tls://")]
            } else {
                vec![format!("{name}://"), format!("{name}+tls://")]
            };

        // Error codes — handle both `[error_codes]` and `[errors]` sections.
        let mut error_codes = BTreeMap::new();
        let errors_table = root
            .get("error_codes")
            .or_else(|| root.get("errors"))
            .and_then(|v| v.as_table());

        if let Some(table) = errors_table {
            for (code, val) in table {
                let (desc, http) = if let Some(t) = val.as_table() {
                    let desc = t
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let http = t
                        .get("http_equiv")
                        .or_else(|| t.get("http"))
                        .and_then(|v| v.as_integer())
                        .unwrap_or(500) as u16;
                    (desc, http)
                } else {
                    (String::new(), 500)
                };
                error_codes.insert(
                    code.clone(),
                    ErrorCodeDef {
                        description: desc,
                        http_equiv: http,
                    },
                );
            }
        }

        // Commands.
        let commands = root
            .get("commands")
            .and_then(|v| v.as_table())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();

        // Types.
        let types = root
            .get("types")
            .and_then(|v| v.as_table())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();

        Ok(ProtocolSpec {
            protocol: ProtocolMeta {
                name,
                version,
                description,
                default_tcp_port,
                default_http_port,
                uri_schemes,
            },
            error_codes,
            commands,
            types,
        })
    }
}
