//! Compatibility manifest generation.
//!
//! Emits a `compatibility.json` file into each generated SDK declaring:
//! - the SDK's own version (already in the language-native manifest, but
//!   restated here for cross-language consumers)
//! - the wire protocol it speaks
//! - per-engine **minimum** versions — the versions of each engine's
//!   `protocol.toml` the SDK was generated against. An engine at a lower
//!   version may be missing commands the SDK tries to call.
//!
//! This replaces the previous "SDK version == Moat version" mental model
//! with an explicit matrix. Clients can read it at runtime, or use it to
//! compare against a HELLO response from a standalone engine before
//! issuing commands.

use crate::generator::GeneratedFile;
use crate::unified::ir::UnifiedIR;
use std::fmt::Write;

/// Wire protocol identifier stamped into the compatibility manifest.
/// Must stay in sync with `shroudb_protocol_wire::WIRE_PROTOCOL`; kept as
/// a literal here to avoid pulling commons into the codegen binary for a
/// single constant.
const WIRE_PROTOCOL: &str = "RESP3/1";

/// Build the `compatibility.json` file content for an SDK built from `ir`.
pub fn generate(ir: &UnifiedIR) -> GeneratedFile {
    let mut out = String::new();
    out.push_str("{\n");
    writeln!(out, "  \"sdk_version\": \"{}\",", escape(&ir.version)).unwrap();
    writeln!(out, "  \"wire_protocol\": \"{}\",", WIRE_PROTOCOL).unwrap();
    out.push_str("  \"engines\": {\n");

    let mut first = true;
    for engine in &ir.engines {
        if !first {
            out.push_str(",\n");
        }
        first = false;
        write!(
            out,
            "    \"{}\": {{ \"min_version\": \"{}\" }}",
            escape(&engine.name),
            escape(&engine.version)
        )
        .unwrap();
    }
    out.push_str("\n  }\n");
    out.push_str("}\n");

    GeneratedFile {
        path: "compatibility.json".into(),
        content: out,
    }
}

fn escape(s: &str) -> String {
    // Engine names and versions are constrained to `[A-Za-z0-9_.-]`, so
    // we only need to guard against double-quote / backslash in principle.
    // Still, do it defensively so unexpected characters don't corrupt the
    // emitted JSON.
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => write!(out, "\\u{:04x}", c as u32).unwrap(),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unified::ir::{EngineIR, SdkPackages, UnifiedIR};
    use std::collections::BTreeMap;

    fn fake_ir() -> UnifiedIR {
        UnifiedIR {
            version: "2.2.0".into(),
            packages: SdkPackages {
                typescript: "@shroudb/sdk".into(),
                python: "shroudb".into(),
                go_module: "github.com/shroudb/shroudb-go".into(),
                ruby: "shroudb".into(),
            },
            engines: vec![
                EngineIR {
                    name: "cipher".into(),
                    description: String::new(),
                    version: "1.4.15".into(),
                    default_port: 6599,
                    uri_schemes: vec![],
                    http_prefix: "/v1/cipher".into(),
                    commands: vec![],
                    types: BTreeMap::new(),
                    error_codes: BTreeMap::new(),
                    has_http_api: false,
                    http_port: None,
                    http_base_path: None,
                },
                EngineIR {
                    name: "sigil".into(),
                    description: String::new(),
                    version: "2.1.0".into(),
                    default_port: 6600,
                    uri_schemes: vec![],
                    http_prefix: "/v1/sigil".into(),
                    commands: vec![],
                    types: BTreeMap::new(),
                    error_codes: BTreeMap::new(),
                    has_http_api: true,
                    http_port: None,
                    http_base_path: None,
                },
            ],
            moat_http_port: 0,
            moat_resp3_port: 0,
        }
    }

    #[test]
    fn manifest_shape_is_stable() {
        let file = generate(&fake_ir());
        assert_eq!(file.path, "compatibility.json");
        assert!(file.content.contains("\"sdk_version\": \"2.2.0\""));
        assert!(
            file.content
                .contains("\"cipher\": { \"min_version\": \"1.4.15\" }")
        );
        assert!(
            file.content
                .contains("\"sigil\": { \"min_version\": \"2.1.0\" }")
        );
        assert!(file.content.contains("\"wire_protocol\": \"RESP3/1\""));
    }

    #[test]
    fn every_engine_has_a_min_version() {
        let file = generate(&fake_ir());
        for engine in &fake_ir().engines {
            let needle = format!("\"{}\": {{ \"min_version\":", engine.name);
            assert!(
                file.content.contains(&needle),
                "compatibility.json missing entry for {}: {}",
                engine.name,
                file.content
            );
        }
    }

    #[test]
    fn escapes_quotes_in_names() {
        // Engine names should never contain quotes, but if they did we
        // must not produce syntactically invalid JSON.
        let mut ir = fake_ir();
        ir.engines[0].name = r#"bad"name"#.into();
        let file = generate(&ir);
        assert!(file.content.contains(r#"bad\"name"#));
    }
}
