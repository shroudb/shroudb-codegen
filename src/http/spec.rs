//! HTTP API specification parser.
//!
//! Deserializes `protocol.toml` into typed Rust structs that generators consume.

use serde::Deserialize;
use std::collections::BTreeMap;

/// Root of the HTTP API specification.
#[derive(Debug, Deserialize)]
pub struct ApiSpec {
    pub api: ApiMeta,
    #[serde(default)]
    pub error_codes: BTreeMap<String, ErrorCodeDef>,
    pub endpoints: BTreeMap<String, EndpointDef>,
}

#[derive(Debug, Deserialize)]
pub struct ApiMeta {
    pub name: String,
    pub version: String,
    pub description: String,
    pub default_port: u16,
}

#[derive(Debug, Deserialize)]
pub struct ErrorCodeDef {
    pub description: String,
    pub http_status: u16,
}

#[derive(Debug, Deserialize)]
pub struct EndpointDef {
    pub method: String,
    pub path: String,
    pub description: String,
    #[serde(default = "default_auth")]
    pub auth: String,
    #[serde(default = "default_success_status")]
    pub success_status: u16,
    #[serde(default)]
    pub keyspace_in_path: Option<bool>,
    #[serde(default)]
    pub body: BTreeMap<String, FieldDef>,
    #[serde(default)]
    pub response: BTreeMap<String, FieldDef>,
}

#[derive(Debug, Deserialize)]
pub struct FieldDef {
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub required: bool,
    pub description: String,
    #[serde(default)]
    pub optional: bool,
}

fn default_auth() -> String {
    "none".into()
}

fn default_success_status() -> u16 {
    200
}

impl ApiSpec {
    pub fn from_toml(text: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(text)
    }
}

impl EndpointDef {
    /// Whether this endpoint has `{keyspace}` in the path.
    pub fn has_keyspace(&self) -> bool {
        self.keyspace_in_path.unwrap_or_else(|| self.path.contains("{keyspace}"))
    }

    /// Required body fields, sorted by name.
    pub fn required_body(&self) -> Vec<(&str, &FieldDef)> {
        self.body
            .iter()
            .filter(|(_, f)| f.required)
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }

    /// Optional body fields, sorted by name.
    pub fn optional_body(&self) -> Vec<(&str, &FieldDef)> {
        self.body
            .iter()
            .filter(|(_, f)| !f.required)
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }

    /// Required response fields.
    pub fn required_response(&self) -> Vec<(&str, &FieldDef)> {
        self.response
            .iter()
            .filter(|(_, f)| !f.optional)
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }

    /// Optional response fields.
    pub fn optional_response(&self) -> Vec<(&str, &FieldDef)> {
        self.response
            .iter()
            .filter(|(_, f)| f.optional)
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }
}
