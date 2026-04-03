//! Intermediate representation consumed by all language generators.
//!
//! Built from a [`ResolvedMoatSpec`] in one pass. Normalizes the varied
//! command formats across engine specs into a common [`CommandIR`] type.

use std::collections::BTreeMap;

use crate::spec::moat::ResolvedMoatSpec;

/// Top-level unified intermediate representation.
pub struct UnifiedIR {
    pub version: String,
    pub packages: SdkPackages,
    pub engines: Vec<EngineIR>,
    pub moat_http_port: u16,
    pub moat_resp3_port: u16,
}

/// Per-language package names.
pub struct SdkPackages {
    pub typescript: String,
    pub python: String,
    pub go_module: String,
    pub ruby: String,
}

/// A single engine with all its commands, types, and error codes.
pub struct EngineIR {
    pub name: String,
    pub description: String,
    pub default_port: u16,
    pub uri_schemes: Vec<String>,
    pub http_prefix: String,
    pub commands: Vec<CommandIR>,
    pub types: BTreeMap<String, TypeIR>,
    pub error_codes: BTreeMap<String, ErrorCodeIR>,

    /// Whether this engine exposes an HTTP REST API.
    pub has_http_api: bool,
    /// HTTP port (if the engine has an HTTP API).
    pub http_port: Option<u16>,
    /// HTTP base path (e.g., "/sigil").
    pub http_base_path: Option<String>,
}

impl EngineIR {
    /// Check if a type key refers to a base64-encoded wire type.
    pub fn is_base64_type(&self, type_key: &str) -> bool {
        self.types.get(type_key).is_some_and(|t| t.base64)
    }
}

/// A normalized command.
pub struct CommandIR {
    pub name: String,
    pub verb: String,
    pub subcommand: Option<String>,
    pub description: String,
    pub is_read: bool,
    pub is_streaming: bool,
    pub positional_params: Vec<ParamIR>,
    pub named_params: Vec<ParamIR>,
    pub response_fields: Vec<FieldIR>,
    pub error_refs: Vec<String>,
    /// HTTP endpoint metadata (if the command has an HTTP API mapping).
    pub http: Option<HttpEndpointIR>,
}

/// HTTP REST endpoint metadata parsed from `http = { method, path, request_body }`.
pub struct HttpEndpointIR {
    pub method: String,
    pub path: String,
    pub body_type: Option<String>,
}

pub struct ParamIR {
    pub name: String,
    pub type_key: String,
    pub required: bool,
    pub wire_key: Option<String>,
    pub variadic: bool,
    pub description: String,
}

pub struct FieldIR {
    pub name: String,
    pub type_key: String,
    pub optional: bool,
    pub description: String,
}

pub struct TypeIR {
    pub description: String,
    pub wire_type: String,
    pub python_type: String,
    pub typescript_type: String,
    pub go_type: String,
    pub ruby_type: String,
    /// Whether this type is base64-encoded on the wire.
    pub base64: bool,
}

pub struct ErrorCodeIR {
    pub code: String,
    pub description: String,
    pub http_equiv: u16,
}

// ── Builder ──────────────────────────────────────────────────────────────────

impl UnifiedIR {
    /// Build the IR from a resolved Moat spec.
    pub fn from_resolved(resolved: &ResolvedMoatSpec) -> Result<Self, Box<dyn std::error::Error>> {
        let moat = &resolved.moat;

        let sdk = moat.sdk.as_ref();
        let packages = SdkPackages {
            typescript: sdk
                .and_then(|s| s.typescript.as_ref())
                .and_then(|c| c.module.as_deref())
                .unwrap_or("@shroudb/sdk")
                .to_string(),
            python: sdk
                .and_then(|s| s.python.as_ref())
                .and_then(|c| c.module.as_deref())
                .unwrap_or("shroudb")
                .to_string(),
            go_module: sdk
                .and_then(|s| s.go.as_ref())
                .and_then(|c| c.module.as_deref())
                .unwrap_or("github.com/shroudb/shroudb-go")
                .to_string(),
            ruby: sdk
                .and_then(|s| s.ruby.as_ref())
                .and_then(|c| c.module.as_deref())
                .unwrap_or("shroudb")
                .to_string(),
        };

        let mut engines = Vec::new();

        for engine_ref in &moat.engines {
            let Some(spec) = resolved.engine_specs.get(&engine_ref.name) else {
                continue;
            };

            let default_prefix = format!("/v1/{}", engine_ref.name);
            let http_prefix = engine_ref
                .http_prefix
                .as_deref()
                .unwrap_or(&default_prefix)
                .to_string();

            // Parse types.
            let mut types = BTreeMap::new();
            for (type_name, type_val) in &spec.types {
                if let Some(t) = parse_type_def(type_val) {
                    types.insert(type_name.clone(), t);
                }
            }

            // Error codes.
            let mut error_codes = BTreeMap::new();
            for (code, def) in &spec.error_codes {
                error_codes.insert(
                    code.clone(),
                    ErrorCodeIR {
                        code: code.clone(),
                        description: def.description.clone(),
                        http_equiv: def.http_equiv,
                    },
                );
            }

            // Parse commands — detect format per command.
            let mut commands = Vec::new();
            for (cmd_name, cmd_val) in &spec.commands {
                if let Some(cmd) = parse_command(cmd_name, cmd_val) {
                    commands.push(cmd);
                }
            }

            let has_http_api = commands.iter().any(|c| c.http.is_some());
            let http_port = spec.protocol.default_http_port;
            let http_base_path = if has_http_api {
                // Extract base path from the first HTTP endpoint (e.g., "/sigil" from "/sigil/schemas")
                commands
                    .iter()
                    .filter_map(|c| c.http.as_ref())
                    .map(|h| {
                        let path = &h.path;
                        // Take the first path segment as base: "/sigil/..." → "/sigil"
                        match path[1..].find('/') {
                            Some(i) => path[..i + 1].to_string(),
                            None => path.clone(),
                        }
                    })
                    .next()
            } else {
                None
            };

            engines.push(EngineIR {
                name: engine_ref.name.clone(),
                description: spec.protocol.description.clone(),
                default_port: spec.protocol.default_tcp_port,
                uri_schemes: spec.protocol.uri_schemes.clone(),
                http_prefix,
                commands,
                types,
                error_codes,
                has_http_api,
                http_port,
                http_base_path,
            });
        }

        Ok(Self {
            version: moat.protocol.version.clone(),
            packages,
            engines,
            moat_http_port: moat.protocol.default_http_port,
            moat_resp3_port: moat.protocol.default_resp3_port,
        })
    }

    /// Build the IR from a single engine spec (for `--http` mode).
    pub fn from_single_engine(
        engine_name: &str,
        spec: &crate::spec::wire::ProtocolSpec,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut types = BTreeMap::new();
        for (type_name, type_val) in &spec.types {
            if let Some(t) = parse_type_def(type_val) {
                types.insert(type_name.clone(), t);
            }
        }

        let mut error_codes = BTreeMap::new();
        for (code, def) in &spec.error_codes {
            error_codes.insert(
                code.clone(),
                ErrorCodeIR {
                    code: code.clone(),
                    description: def.description.clone(),
                    http_equiv: def.http_equiv,
                },
            );
        }

        let mut commands = Vec::new();
        for (cmd_name, cmd_val) in &spec.commands {
            if let Some(cmd) = parse_command(cmd_name, cmd_val) {
                commands.push(cmd);
            }
        }

        let has_http_api = commands.iter().any(|c| c.http.is_some());
        let http_port = spec.protocol.default_http_port;

        let http_base_path = commands
            .iter()
            .filter_map(|c| c.http.as_ref())
            .map(|h| match h.path[1..].find('/') {
                Some(i) => h.path[..i + 1].to_string(),
                None => h.path.clone(),
            })
            .next();

        let engine = EngineIR {
            name: engine_name.to_string(),
            description: spec.protocol.description.clone(),
            default_port: spec.protocol.default_tcp_port,
            uri_schemes: spec.protocol.uri_schemes.clone(),
            http_prefix: format!("/v1/{engine_name}"),
            commands,
            types,
            error_codes,
            has_http_api,
            http_port,
            http_base_path,
        };

        Ok(Self {
            version: spec.protocol.version.clone(),
            packages: SdkPackages {
                typescript: format!("@shroudb/{engine_name}-http"),
                python: format!("shroudb-{engine_name}-http"),
                go_module: format!("github.com/shroudb/{engine_name}-http-go"),
                ruby: format!("shroudb-{engine_name}-http"),
            },
            engines: vec![engine],
            moat_http_port: http_port.unwrap_or(0),
            moat_resp3_port: 0,
        })
    }
}

// ── Command parsing ──────────────────────────────────────────────────────────

fn parse_command(name: &str, val: &toml::Value) -> Option<CommandIR> {
    let table = val.as_table()?;

    let description = table
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Detect format:
    // - "verb" field → standard (Cipher, etc.)
    // - "syntax" field → Sigil/Veil style
    // - neither → implicit verb from command name (ShrouDB core)
    if let Some(verb) = table.get("verb").and_then(|v| v.as_str()) {
        parse_standard_command(name, table, verb, &description)
    } else if let Some(syntax) = table.get("syntax").and_then(|v| v.as_str()) {
        parse_syntax_command(name, table, syntax, &description)
    } else {
        // Implicit format: command name IS the verb (e.g., "PUT", "GET", "AUTH").
        parse_implicit_command(name, table, &description)
    }
}

fn parse_standard_command(
    name: &str,
    table: &toml::map::Map<String, toml::Value>,
    verb: &str,
    description: &str,
) -> Option<CommandIR> {
    let subcommand = table
        .get("variant")
        .and_then(|v| v.as_str())
        .map(String::from);

    let is_read = table
        .get("replica_behavior")
        .and_then(|v| v.as_str())
        .is_some_and(|r| r == "PureRead");

    let is_streaming = table
        .get("streaming")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Parse params array.
    let mut positional_params = Vec::new();
    let mut named_params = Vec::new();

    if let Some(params) = table
        .get("params")
        .or_else(|| table.get("parameters"))
        .and_then(|v| v.as_array())
    {
        let mut positional_items: Vec<(u32, ParamIR)> = Vec::new();

        for p in params {
            let pt = p.as_table()?;
            let param_name = pt.get("name")?.as_str()?.to_string();
            let type_key = pt
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("string")
                .to_string();
            let required = pt
                .get("required")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let desc = pt
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let variadic = pt
                .get("variadic")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if let Some(pos) = pt.get("position").and_then(|v| v.as_integer()) {
                positional_items.push((
                    pos as u32,
                    ParamIR {
                        name: param_name,
                        type_key,
                        required,
                        wire_key: None,
                        variadic,
                        description: desc,
                    },
                ));
            } else if let Some(key) = pt.get("key").and_then(|v| v.as_str()) {
                named_params.push(ParamIR {
                    name: param_name,
                    type_key,
                    required,
                    wire_key: Some(key.to_string()),
                    variadic,
                    description: desc,
                });
            }
        }

        positional_items.sort_by_key(|(pos, _)| *pos);
        positional_params = positional_items.into_iter().map(|(_, p)| p).collect();
    }

    // Parse response array.
    let response_fields = parse_response_array(table);

    // Error refs.
    let error_refs = table
        .get("errors")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    v.as_table()
                        .and_then(|t| t.get("code"))
                        .and_then(|c| c.as_str())
                        .map(String::from)
                        .or_else(|| v.as_str().map(String::from))
                })
                .collect()
        })
        .unwrap_or_default();

    Some(CommandIR {
        name: name.to_string(),
        verb: verb.to_string(),
        subcommand,
        description: description.to_string(),
        is_read,
        is_streaming,
        positional_params,
        named_params,
        response_fields,
        error_refs,
        http: parse_http_annotation(table),
    })
}

fn parse_syntax_command(
    name: &str,
    table: &toml::map::Map<String, toml::Value>,
    syntax: &str,
    description: &str,
) -> Option<CommandIR> {
    // Parse verb and subcommand from syntax: "VERB SUBCOMMAND <param> [KEYWORD <param>]"
    let parts: Vec<&str> = syntax.split_whitespace().collect();
    let mut verb = String::new();
    let mut subcommand: Option<String> = None;

    for (i, part) in parts.iter().enumerate() {
        if part.starts_with('<') || part.starts_with('[') {
            break;
        }
        if i == 0 {
            verb = part.to_string();
        } else {
            // Accumulate multi-word subcommands (e.g., "REVOKE ALL").
            subcommand = Some(match subcommand {
                Some(existing) => format!("{existing} {part}"),
                None => part.to_string(),
            });
        }
    }

    // Parse parameters.
    let mut positional_params = Vec::new();
    let mut named_params = Vec::new();

    let params_val = table.get("parameters").or_else(|| table.get("params"));

    if let Some(params_arr) = params_val.and_then(|v| v.as_array()) {
        // Array format: [{ name, type, required, ... }]
        for p in params_arr {
            let pt = p.as_table()?;
            let param_name = pt.get("name")?.as_str()?.to_string();
            let type_key =
                normalize_type(pt.get("type").and_then(|v| v.as_str()).unwrap_or("string"));
            let required = pt.get("required").and_then(|v| v.as_bool()).unwrap_or(true);
            let desc = pt
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if required {
                positional_params.push(ParamIR {
                    name: param_name,
                    type_key,
                    required: true,
                    wire_key: None,
                    variadic: false,
                    description: desc,
                });
            } else {
                named_params.push(ParamIR {
                    name: param_name.clone(),
                    type_key,
                    required: false,
                    wire_key: Some(param_name.to_uppercase()),
                    variadic: false,
                    description: desc,
                });
            }
        }
    } else if let Some(params_table) = params_val.and_then(|v| v.as_table()) {
        // Table format: { param_name = { type, required, ... } } (Stash style)
        // Extract positional order from the syntax string: <param1> <param2>
        let syntax_order: Vec<String> = parts
            .iter()
            .filter(|p| p.starts_with('<') && p.ends_with('>'))
            .map(|p| p.trim_matches(|c| c == '<' || c == '>').to_string())
            .collect();

        // Process in syntax order for positional params, then remaining for keywords
        let mut processed = std::collections::HashSet::new();
        for syntax_name in &syntax_order {
            if let Some(param_val) = params_table.get(syntax_name) {
                processed.insert(syntax_name.clone());
                let pt = param_val.as_table()?;
                let type_str = pt.get("type").and_then(|v| v.as_str()).unwrap_or("string");
                let desc = pt
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                positional_params.push(ParamIR {
                    name: syntax_name.clone(),
                    type_key: normalize_type(type_str),
                    required: true,
                    wire_key: None,
                    variadic: false,
                    description: desc,
                });
            }
        }
        for (param_name, param_val) in params_table {
            if processed.contains(param_name) {
                continue;
            }
            let pt = param_val.as_table()?;
            let type_str = pt.get("type").and_then(|v| v.as_str()).unwrap_or("string");
            let type_key = normalize_type(type_str);
            let desc = pt
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // Remaining params (not in syntax order) are keyword params.
            // "flag" type = boolean keyword with no value.
            if type_str == "flag" {
                named_params.push(ParamIR {
                    name: param_name.clone(),
                    type_key: "boolean".into(),
                    required: false,
                    wire_key: Some(param_name.to_uppercase()),
                    variadic: false,
                    description: desc,
                });
            } else {
                named_params.push(ParamIR {
                    name: param_name.clone(),
                    type_key,
                    required: false,
                    wire_key: Some(param_name.to_uppercase()),
                    variadic: false,
                    description: desc,
                });
            }
        }
    }

    // Parse response — handles multiple formats.
    let response_fields = parse_response(table);

    // Is this a read command? Check for HTTP GET hint.
    let is_read = table
        .get("http")
        .and_then(|v| v.as_table())
        .and_then(|t| t.get("method"))
        .and_then(|v| v.as_str())
        .is_some_and(|m| m.eq_ignore_ascii_case("GET"));

    // Error refs.
    let error_refs = table
        .get("errors")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    Some(CommandIR {
        name: name.to_string(),
        verb,
        subcommand,
        description: description.to_string(),
        is_read,
        is_streaming: false,
        positional_params,
        named_params,
        response_fields,
        error_refs,
        http: parse_http_annotation(table),
    })
}

/// Parse a command where the command name IS the verb (ShrouDB core format).
/// Params come as `[[commands.CMD.params]]` arrays-of-tables.
/// Response is an inline table `[commands.CMD.response]`.
fn parse_implicit_command(
    name: &str,
    table: &toml::map::Map<String, toml::Value>,
    description: &str,
) -> Option<CommandIR> {
    // Split multi-word commands: "NAMESPACE CREATE" → verb="NAMESPACE", subcommand="CREATE"
    let (verb, subcommand) = if let Some(idx) = name.find(' ') {
        (name[..idx].to_string(), Some(name[idx + 1..].to_string()))
    } else {
        (name.to_string(), None)
    };

    let acl = table.get("acl").and_then(|v| v.as_str()).unwrap_or("");
    let is_read = acl == "read";

    // Parse params — may be TOML array-of-tables or inline array.
    let mut positional_params = Vec::new();
    let mut named_params = Vec::new();

    if let Some(params) = table
        .get("params")
        .or_else(|| table.get("parameters"))
        .and_then(|v| v.as_array())
    {
        for p in params {
            let pt = p.as_table()?;
            let param_name = pt.get("name")?.as_str()?.to_string();
            let type_str = pt.get("type").and_then(|v| v.as_str()).unwrap_or("string");
            let required = pt
                .get("required")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let desc = pt
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // Types starting with "keyword_" are named/keyword params.
            // Skip keyword params that duplicate a positional param name.
            if type_str.starts_with("keyword_") {
                let base_type = type_str.strip_prefix("keyword_").unwrap_or("string");
                let kw_name = param_name.to_lowercase();
                let is_dup = positional_params
                    .iter()
                    .any(|pp: &ParamIR| pp.name.to_lowercase() == kw_name);
                if !is_dup {
                    named_params.push(ParamIR {
                        name: kw_name,
                        type_key: normalize_type(base_type),
                        required,
                        wire_key: Some(param_name.clone()),
                        variadic: false,
                        description: desc,
                    });
                }
            } else {
                positional_params.push(ParamIR {
                    name: param_name.clone(),
                    type_key: normalize_type(type_str),
                    required,
                    wire_key: None,
                    variadic: false,
                    description: desc,
                });
            }
        }
    }

    // Parse response.
    let response_fields = parse_response(table);

    Some(CommandIR {
        name: name.to_string(),
        verb,
        subcommand,
        description: description.to_string(),
        is_read,
        is_streaming: false,
        positional_params,
        named_params,
        response_fields,
        error_refs: Vec::new(),
        http: parse_http_annotation(table),
    })
}

// ── Response parsing ─────────────────────────────────────────────────────────

/// Parse response from standard format: `response = [{ name, type, description, optional }]`
fn parse_response_array(table: &toml::map::Map<String, toml::Value>) -> Vec<FieldIR> {
    if let Some(arr) = table.get("response").and_then(|v| v.as_array()) {
        arr.iter()
            .filter_map(|v| {
                let t = v.as_table()?;
                Some(FieldIR {
                    name: t.get("name")?.as_str()?.to_string(),
                    type_key: normalize_type(
                        t.get("type").and_then(|v| v.as_str()).unwrap_or("string"),
                    ),
                    optional: t.get("optional").and_then(|v| v.as_bool()).unwrap_or(false),
                    description: t
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                })
            })
            .collect()
    } else {
        Vec::new()
    }
}

/// Parse response from various formats:
/// - Sigil: `response = { version = "integer", status = "string" }`
/// - Veil: `response = { fields = ["status", "index"] }`
/// - Standard: `response = [{ name, type, ... }]`
fn parse_response(table: &toml::map::Map<String, toml::Value>) -> Vec<FieldIR> {
    let Some(resp) = table.get("response") else {
        return Vec::new();
    };

    // Standard array format.
    if let Some(arr) = resp.as_array() {
        return arr
            .iter()
            .filter_map(|v| {
                let t = v.as_table()?;
                Some(FieldIR {
                    name: t.get("name")?.as_str()?.to_string(),
                    type_key: normalize_type(
                        t.get("type").and_then(|v| v.as_str()).unwrap_or("string"),
                    ),
                    optional: t.get("optional").and_then(|v| v.as_bool()).unwrap_or(false),
                    description: t
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                })
            })
            .collect();
    }

    // Inline table format.
    if let Some(t) = resp.as_table() {
        // Veil condensed: { fields = ["status", "index"] }
        // Uses "any" type since the condensed format doesn't carry type info.
        if let Some(fields_arr) = t.get("fields").and_then(|v| v.as_array()) {
            return fields_arr
                .iter()
                .filter_map(|v| {
                    Some(FieldIR {
                        name: v.as_str()?.to_string(),
                        type_key: "any".to_string(),
                        optional: false,
                        description: String::new(),
                    })
                })
                .collect();
        }

        // Sigil inline: { version = "integer", status = "string" }
        return t
            .iter()
            .map(|(name, type_val)| FieldIR {
                name: name.clone(),
                type_key: normalize_type(type_val.as_str().unwrap_or("string")),
                optional: false,
                description: String::new(),
            })
            .collect();
    }

    Vec::new()
}

// ── Type parsing ─────────────────────────────────────────────────────────────

fn parse_type_def(val: &toml::Value) -> Option<TypeIR> {
    let t = val.as_table()?;

    // Skip Sigil-style types that have "fields" (composite types, not simple type mappings).
    if t.contains_key("fields") {
        return None;
    }

    let desc = t
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let base64 = desc.to_lowercase().starts_with("base64-encoded");

    Some(TypeIR {
        description: desc,
        wire_type: t
            .get("wire_type")
            .and_then(|v| v.as_str())
            .unwrap_or("bulk_string")
            .to_string(),
        python_type: t
            .get("python_type")
            .and_then(|v| v.as_str())
            .unwrap_or("str")
            .to_string(),
        typescript_type: t
            .get("typescript_type")
            .and_then(|v| v.as_str())
            .unwrap_or("string")
            .to_string(),
        go_type: t
            .get("go_type")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| infer_go_type(t)),
        ruby_type: t
            .get("ruby_type")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| infer_ruby_type(t)),
        base64,
    })
}

/// Infer Go type from other type fields (wire_type, rust_type).
fn infer_go_type(t: &toml::map::Map<String, toml::Value>) -> String {
    let wire = t.get("wire_type").and_then(|v| v.as_str()).unwrap_or("");
    let rust = t.get("rust_type").and_then(|v| v.as_str()).unwrap_or("");
    if wire == "integer" || rust.starts_with("u") || rust.starts_with("i") {
        return "int".into();
    }
    if wire == "boolean" || rust == "bool" {
        return "bool".into();
    }
    "string".into()
}

/// Infer Ruby type from other type fields.
fn infer_ruby_type(t: &toml::map::Map<String, toml::Value>) -> String {
    let wire = t.get("wire_type").and_then(|v| v.as_str()).unwrap_or("");
    let rust = t.get("rust_type").and_then(|v| v.as_str()).unwrap_or("");
    if wire == "integer" || rust.starts_with("u") || rust.starts_with("i") {
        return "Integer".into();
    }
    if wire == "boolean" || rust == "bool" {
        return "TrueClass".into();
    }
    "String".into()
}

/// Parse `http = { method, path, request_body }` annotation from a command table.
fn parse_http_annotation(table: &toml::map::Map<String, toml::Value>) -> Option<HttpEndpointIR> {
    let http = table.get("http")?.as_table()?;
    let method = http.get("method")?.as_str()?.to_string();
    let path = http.get("path")?.as_str()?.to_string();
    let body_type = http
        .get("request_body")
        .and_then(|v| v.as_str())
        .map(String::from);
    Some(HttpEndpointIR {
        method,
        path,
        body_type,
    })
}

/// Normalize type names from various specs into consistent keys.
fn normalize_type(t: &str) -> String {
    match t {
        "str" | "String" => "string".into(),
        "int" | "u32" | "u64" | "i64" => "integer".into(),
        "bool" | "flag" => "boolean".into(),
        s if s.starts_with("array<") || s.starts_with("Array<") => "array".into(),
        s if s.starts_with("map") || s.starts_with("Map") || s.starts_with("dict") => "json".into(),
        other => other.to_string(),
    }
}
