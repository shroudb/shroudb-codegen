//! Protobuf generator with `google.api.http` annotations for gRPC-JSON transcoding.
//!
//! Produces:
//! - `{snake}/v1/{snake}.proto` — service definition with HTTP annotations
//! - `buf.yaml`                — buf module configuration
//! - `README.md`               — usage docs

use super::super::spec::{ApiSpec, EndpointDef};
use crate::generator::{GeneratedFile, Naming};
use heck::{ToSnakeCase, ToUpperCamelCase};
use std::fmt::Write;

use super::HttpGenerator;

pub struct ProtoGenerator;

impl HttpGenerator for ProtoGenerator {
    fn language(&self) -> &'static str {
        "Proto"
    }

    fn generate(&self, spec: &ApiSpec, n: &Naming) -> Vec<GeneratedFile> {
        vec![
            gen_proto(spec, n),
            gen_buf_yaml(n),
            gen_readme(spec, n),
        ]
    }
}

// ─── Type mapping ────────────────────────────────────────────────────────────

fn proto_type(field_type: &str) -> &'static str {
    match field_type {
        "string" => "string",
        "integer" => "int64",
        "json" => "google.protobuf.Struct",
        "json_array" => "google.protobuf.ListValue",
        _ => "string",
    }
}

/// Whether the proto file needs `google/protobuf/struct.proto` imported.
fn needs_struct_import(spec: &ApiSpec) -> bool {
    spec.endpoints.values().any(|ep| {
        ep.body
            .values()
            .chain(ep.response.values())
            .any(|f| matches!(f.field_type.as_str(), "json" | "json_array"))
    })
}

// ─── {snake}/v1/{snake}.proto ────────────────────────────────────────────────

fn gen_proto(spec: &ApiSpec, n: &Naming) -> GeneratedFile {
    let mut out = String::new();

    // Header
    writeln!(out, "syntax = \"proto3\";").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "package {}.v1;", n.snake).unwrap();
    writeln!(out).unwrap();
    writeln!(out, "import \"google/api/annotations.proto\";").unwrap();
    if needs_struct_import(spec) {
        writeln!(out, "import \"google/protobuf/struct.proto\";").unwrap();
    }
    writeln!(out).unwrap();

    // Service
    writeln!(
        out,
        "// {} \u{2014} {}",
        n.pascal, n.description
    )
    .unwrap();
    writeln!(out, "service {}Service {{", n.pascal).unwrap();

    for (ep_name, ep) in &spec.endpoints {
        let rpc_name = ep_name.to_upper_camel_case();
        let req_msg = format!("{rpc_name}Request");
        let resp_msg = format!("{rpc_name}Response");

        writeln!(out, "  // {}", ep.description).unwrap();
        writeln!(
            out,
            "  rpc {rpc_name}({req_msg}) returns ({resp_msg}) {{"
        )
        .unwrap();
        writeln!(out, "    option (google.api.http) = {{").unwrap();

        let method_lower = ep.method.to_lowercase();
        writeln!(out, "      {method_lower}: \"{}\"", ep.path).unwrap();

        if method_lower == "post" {
            writeln!(out, "      body: \"*\"").unwrap();
        }

        writeln!(out, "    }};").unwrap();
        writeln!(out, "  }}").unwrap();
        writeln!(out).unwrap();
    }

    writeln!(out, "}}").unwrap();

    // Messages
    for (ep_name, ep) in &spec.endpoints {
        writeln!(out).unwrap();
        gen_request_message(&mut out, ep_name, ep);
        writeln!(out).unwrap();
        gen_response_message(&mut out, ep_name, ep);
    }

    GeneratedFile {
        path: format!("{}/v1/{}.proto", n.snake, n.snake),
        content: out,
    }
}

fn gen_request_message(out: &mut String, ep_name: &str, ep: &EndpointDef) {
    let msg_name = format!("{}Request", ep_name.to_upper_camel_case());
    writeln!(out, "message {msg_name} {{").unwrap();

    let mut field_num: u32 = 1;

    // Keyspace field if endpoint has keyspace in path
    if ep.has_keyspace() {
        writeln!(out, "  // Auth keyspace name").unwrap();
        writeln!(out, "  string keyspace = {field_num};").unwrap();
        field_num += 1;
    }

    // Body fields
    for (name, field) in &ep.body {
        let proto_field_name = name.to_snake_case();
        let proto_t = proto_type(&field.field_type);
        let is_optional = field.optional || !field.required;

        writeln!(out, "  // {}", field.description).unwrap();
        if is_optional {
            writeln!(
                out,
                "  optional {proto_t} {proto_field_name} = {field_num};"
            )
            .unwrap();
        } else {
            writeln!(out, "  {proto_t} {proto_field_name} = {field_num};").unwrap();
        }
        field_num += 1;
    }

    writeln!(out, "}}").unwrap();
}

fn gen_response_message(out: &mut String, ep_name: &str, ep: &EndpointDef) {
    let msg_name = format!("{}Response", ep_name.to_upper_camel_case());
    writeln!(out, "message {msg_name} {{").unwrap();

    let mut field_num: u32 = 1;

    for (name, field) in &ep.response {
        let proto_field_name = name.to_snake_case();
        let proto_t = proto_type(&field.field_type);

        writeln!(out, "  // {}", field.description).unwrap();
        if field.optional {
            writeln!(
                out,
                "  optional {proto_t} {proto_field_name} = {field_num};"
            )
            .unwrap();
        } else {
            writeln!(out, "  {proto_t} {proto_field_name} = {field_num};").unwrap();
        }
        field_num += 1;
    }

    writeln!(out, "}}").unwrap();
}

// ─── buf.yaml ────────────────────────────────────────────────────────────────

fn gen_buf_yaml(n: &Naming) -> GeneratedFile {
    GeneratedFile {
        path: "buf.yaml".into(),
        content: format!(
            r#"version: v2
modules:
  - path: {snake}/v1
deps:
  - buf.build/googleapis/googleapis
"#,
            snake = n.snake,
        ),
    }
}

// ─── README.md ───────────────────────────────────────────────────────────────

fn gen_readme(spec: &ApiSpec, n: &Naming) -> GeneratedFile {
    let mut rpcs = String::new();
    for (ep_name, ep) in &spec.endpoints {
        let rpc_name = ep_name.to_upper_camel_case();
        writeln!(rpcs, "- `{rpc_name}` \u{2014} {}", ep.description).unwrap();
    }

    GeneratedFile {
        path: "README.md".into(),
        content: format!(
            r#"# {pascal} Protobuf / gRPC

Protobuf service definition for the [{pascal}](https://github.com/shroudb/{kebab}) {description}, with `google.api.http` annotations for [Envoy gRPC-JSON transcoding](https://www.envoyproxy.io/docs/envoy/latest/configuration/http/http_filters/grpc_json_transcoder_filter).

## Files

| File | Description |
|------|-------------|
| `{snake}/v1/{snake}.proto` | Service, request, and response messages |
| `buf.yaml` | [Buf](https://buf.build) module configuration |

## Prerequisites

Install [buf](https://buf.build/docs/installation) or `protoc` with the `googleapis` include path.

## Generate code with buf

```bash
buf generate
```

Add a `buf.gen.yaml` to configure your target language plugins. For example, to generate Go stubs:

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

## Generate code with protoc

```bash
protoc \
  -I . \
  -I $(buf export buf.build/googleapis/googleapis --output /tmp/googleapis) \
  --go_out=gen/go --go_opt=paths=source_relative \
  --go-grpc_out=gen/go --go-grpc_opt=paths=source_relative \
  {snake}/v1/{snake}.proto
```

## Envoy gRPC-JSON transcoding

The `.proto` file includes `google.api.http` annotations on every RPC. This enables
[Envoy's gRPC-JSON transcoder filter](https://www.envoyproxy.io/docs/envoy/latest/configuration/http/http_filters/grpc_json_transcoder_filter)
to expose the gRPC service as a RESTful JSON API without any extra code.

Add the transcoder filter to your Envoy configuration and point it at the compiled
descriptor set:

```bash
buf build -o {snake}.binpb
```

Then reference `{snake}.binpb` in the Envoy `grpc_json_transcoder` filter config.

## RPCs

{rpcs}
## Auto-generated

This proto definition was generated by `shroudb-codegen` from `protocol.toml`.
"#,
            pascal = n.pascal,
            kebab = n.kebab,
            snake = n.snake,
            description = n.description,
            rpcs = rpcs,
        ),
    }
}
