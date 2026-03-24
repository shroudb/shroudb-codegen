//! Protobuf generator.
//!
//! Produces a `.proto` service definition with:
//! - `{snake}/v1/{snake}.proto` — gRPC service + request/response messages
//! - `buf.yaml`                — buf.build module configuration
//! - `README.md`               — usage instructions

use super::super::spec::ProtocolSpec;
use super::Generator;
use crate::generator::{GeneratedFile, Naming};
use heck::{ToSnakeCase, ToUpperCamelCase};
use std::fmt::Write;

pub struct ProtoGenerator;

impl Generator for ProtoGenerator {
    fn language(&self) -> &'static str {
        "Protobuf"
    }

    fn generate(&self, spec: &ProtocolSpec) -> Vec<GeneratedFile> {
        let n = super::naming_from_spec(spec);
        vec![
            gen_proto(spec, &n),
            gen_buf_yaml(spec, &n),
            gen_readme(spec, &n),
        ]
    }
}

/// Map a wire-spec type name to a proto3 type, using the spec's TypeDef when available.
fn proto_type(spec: &ProtocolSpec, type_name: &str) -> &'static str {
    if let Some(t) = spec.types.get(type_name) {
        match t.rust_type.as_str() {
            "String" | "&str" => "string",
            "i64" | "u64" => "int64",
            "i32" | "u32" => "int32",
            "bool" => "bool",
            "f64" => "double",
            "Vec<u8>" => "bytes",
            "Vec<String>" => "string", // repeated handled at field level
            "serde_json::Value" | "HashMap<String, serde_json::Value>" => "google.protobuf.Struct",
            _ => "string",
        }
    } else {
        "string"
    }
}

/// Check if any command uses a type that requires the struct.proto import.
fn needs_struct_import(spec: &ProtocolSpec) -> bool {
    spec.commands.values().any(|cmd| {
        cmd.params
            .iter()
            .any(|p| proto_type(spec, &p.param_type) == "google.protobuf.Struct")
            || cmd
                .response
                .iter()
                .any(|f| proto_type(spec, &f.field_type) == "google.protobuf.Struct")
    })
}

/// Derive the RPC method name from a command definition key.
///
/// For commands with a subcommand (e.g. verb="CONFIG" subcommand="GET"),
/// we combine them: "ConfigGet". Otherwise we just upper-camel the key.
fn rpc_method_name(cmd_key: &str, verb: &str, subcommand: &Option<String>) -> String {
    if let Some(sub) = subcommand {
        let combined = format!("{}_{}", verb.to_lowercase(), sub.to_lowercase());
        combined.to_upper_camel_case()
    } else {
        cmd_key.to_upper_camel_case()
    }
}

// ─── {snake}/v1/{snake}.proto ─────────────────────────────────────────────────

fn gen_proto(spec: &ProtocolSpec, n: &Naming) -> GeneratedFile {
    let mut out = String::with_capacity(4096);

    // Header
    writeln!(
        out,
        "// Auto-generated from {} protocol spec. Do not edit.",
        n.raw
    )
    .unwrap();
    writeln!(out).unwrap();
    writeln!(out, "syntax = \"proto3\";").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "package {}.v1;", n.snake).unwrap();
    writeln!(out).unwrap();

    // Language-specific package options
    writeln!(
        out,
        "option go_package = \"{}/v1;{}v1\";",
        n.go_module, n.snake
    )
    .unwrap();
    writeln!(out, "option java_package = \"com.shroudb.{}.v1\";", n.snake).unwrap();
    writeln!(out, "option java_multiple_files = true;").unwrap();
    writeln!(out).unwrap();

    // Conditional import
    if needs_struct_import(spec) {
        writeln!(out, "import \"google/protobuf/struct.proto\";").unwrap();
        writeln!(out).unwrap();
    }

    // Error codes enum (if any defined)
    if !spec.error_codes.is_empty() {
        writeln!(out, "// Error codes returned by the {} protocol.", n.pascal).unwrap();
        writeln!(out, "enum ErrorCode {{").unwrap();
        writeln!(out, "  ERROR_CODE_UNSPECIFIED = 0;").unwrap();
        for (i, (code, def)) in spec.error_codes.iter().enumerate() {
            let enum_name = code.to_uppercase();
            writeln!(
                out,
                "  ERROR_CODE_{enum_name} = {}; // {} (HTTP equiv: {})",
                i + 1,
                def.description,
                def.http_equiv
            )
            .unwrap();
        }
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();
    }

    // Service definition
    writeln!(out, "// {} — {}", n.pascal, spec.protocol.description).unwrap();
    writeln!(out, "service {}Service {{", n.pascal).unwrap();

    // Collect commands for service RPCs
    let commands: Vec<_> = spec.commands.iter().collect();
    for (i, (cmd_key, cmd)) in commands.iter().enumerate() {
        let method = rpc_method_name(cmd_key, &cmd.verb, &cmd.subcommand);
        let req = format!("{method}Request");
        let resp = format!("{method}Response");

        writeln!(out, "  // {}", cmd.description).unwrap();
        if cmd.streaming {
            writeln!(out, "  rpc {method}({req}) returns (stream {resp});").unwrap();
        } else {
            writeln!(out, "  rpc {method}({req}) returns ({resp});").unwrap();
        }

        if i < commands.len() - 1 {
            writeln!(out).unwrap();
        }
    }
    writeln!(out, "}}").unwrap();

    // Message definitions for each command
    for (cmd_key, cmd) in &spec.commands {
        let method = rpc_method_name(cmd_key, &cmd.verb, &cmd.subcommand);

        // Request message
        writeln!(out).unwrap();
        writeln!(out, "message {method}Request {{").unwrap();
        for (field_num, param) in cmd.params.iter().enumerate() {
            let field_name = param.name.to_snake_case();
            let ptype = proto_type(spec, &param.param_type);
            let field_idx = field_num + 1;

            writeln!(out, "  // {}", param.description).unwrap();
            if param.variadic {
                writeln!(out, "  repeated {ptype} {field_name} = {field_idx};").unwrap();
            } else if param.required {
                writeln!(out, "  {ptype} {field_name} = {field_idx};").unwrap();
            } else {
                writeln!(out, "  optional {ptype} {field_name} = {field_idx};").unwrap();
            }
        }
        writeln!(out, "}}").unwrap();

        // Response message
        writeln!(out).unwrap();
        writeln!(out, "message {method}Response {{").unwrap();
        for (field_num, field) in cmd.response.iter().enumerate() {
            let field_name = field.name.to_snake_case();
            let ftype = proto_type(spec, &field.field_type);
            let field_idx = field_num + 1;

            writeln!(out, "  // {}", field.description).unwrap();
            if field.optional {
                writeln!(out, "  optional {ftype} {field_name} = {field_idx};").unwrap();
            } else {
                writeln!(out, "  {ftype} {field_name} = {field_idx};").unwrap();
            }
        }
        writeln!(out, "}}").unwrap();
    }

    GeneratedFile {
        path: format!("{}/v1/{}.proto", n.snake, n.snake),
        content: out,
    }
}

// ─── buf.yaml ─────────────────────────────────────────────────────────────────

fn gen_buf_yaml(spec: &ProtocolSpec, n: &Naming) -> GeneratedFile {
    let mut content = format!(
        r#"version: v2
modules:
  - path: {snake}/v1
"#,
        snake = n.snake,
    );

    // Only add googleapis dep if we import struct.proto
    if needs_struct_import(spec) {
        content.push_str("deps:\n  - buf.build/googleapis/googleapis\n");
    }

    content.push_str(
        r#"lint:
  use:
    - STANDARD
breaking:
  use:
    - FILE
"#,
    );

    GeneratedFile {
        path: "buf.yaml".into(),
        content,
    }
}

// ─── README.md ────────────────────────────────────────────────────────────────

fn gen_readme(spec: &ProtocolSpec, n: &Naming) -> GeneratedFile {
    let mut out = String::with_capacity(2048);

    writeln!(out, "# {pascal} Protobuf Definitions", pascal = n.pascal).unwrap();
    writeln!(out).unwrap();
    writeln!(
        out,
        "gRPC service definition for the {} protocol.",
        n.pascal
    )
    .unwrap();
    writeln!(out).unwrap();

    // RPC listing
    writeln!(out, "## RPCs").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "| Method | Description | Streaming |").unwrap();
    writeln!(out, "|--------|-------------|-----------|").unwrap();
    for (cmd_key, cmd) in &spec.commands {
        let method = rpc_method_name(cmd_key, &cmd.verb, &cmd.subcommand);
        let streaming = if cmd.streaming { "Yes" } else { "No" };
        writeln!(out, "| `{method}` | {} | {streaming} |", cmd.description).unwrap();
    }
    writeln!(out).unwrap();

    write!(
        out,
        r#"## Prerequisites

Install [buf](https://buf.build/docs/installation) or `protoc` with the gRPC
plugin for your target language.

## Generate code with `buf`

```bash
# Initialize (first time only)
buf mod update

# Generate client/server stubs
buf generate
```

Create a `buf.gen.yaml` to configure your target language. For example, to
generate Go stubs:

```yaml
version: v2
plugins:
  - remote: buf.build/protocolbuffers/go
    out: gen/go
    opt: paths=source_relative
  - remote: buf.build/grpc/go
    out: gen/go
    opt: paths=source_relative
```

## Generate code with `protoc`

```bash
protoc \
  -I . \
  -I $(buf config ls-modules 2>/dev/null || echo .) \
  --go_out=gen/go --go_opt=paths=source_relative \
  --go-grpc_out=gen/go --go-grpc_opt=paths=source_relative \
  {snake}/v1/{snake}.proto
```

## Package layout

- `{snake}/v1/{snake}.proto` — service and message definitions
- `buf.yaml` — buf module configuration
"#,
        snake = n.snake,
    )
    .unwrap();

    GeneratedFile {
        path: "README.md".into(),
        content: out,
    }
}
