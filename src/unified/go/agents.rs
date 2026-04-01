//! AGENTS.md generation for the Go SDK.

use crate::generator::GeneratedFile;
use crate::unified::ir::{CommandIR, EngineIR, UnifiedIR};
use heck::ToPascalCase;
use std::fmt::Write;

pub(super) fn generate(ir: &UnifiedIR) -> GeneratedFile {
    let mut out = String::new();
    let module = &ir.packages.go_module;

    out.push_str("# ShrouDB SDK — Agent Instructions\n\n");
    out.push_str("> Unified Go SDK for all ShrouDB security engines. ");
    out.push_str("Provides namespaced, type-safe access with built-in serialization.\n\n");

    // Quick context.
    out.push_str("## Quick Context\n\n");
    writeln!(out, "- **Module**: `{module}`").unwrap();
    out.push_str("- **Transport**: RESP3 (direct engine connections) or HTTP (Moat gateway)\n");
    out.push_str("- **Pattern**: `db.Engine.Method(ctx, params)` — all methods take `context.Context`, return `(*Response, error)` or `error`\n");
    out.push_str(
        "- **Serialization**: Handled internally — pass Go types, get typed structs back\n\n",
    );

    // Connection.
    out.push_str("## Connection\n\n");
    out.push_str("```go\n");
    writeln!(out, "import shroudb \"{module}\"\n").unwrap();
    out.push_str("// Moat gateway (HTTP) — all engines through one endpoint\n");
    out.push_str("db, err := shroudb.New(shroudb.Options{Moat: \"https://moat.example.com\", Token: \"my-token\"})\n\n");
    out.push_str("// Direct — only the engines you need\n");
    out.push_str(
        "db, err := shroudb.New(shroudb.Options{Cipher: \"shroudb-cipher://token@host:6599\"})\n\n",
    );
    out.push_str("// Mixed — Moat default + direct overrides\n");
    out.push_str("db, err := shroudb.New(shroudb.Options{\n");
    out.push_str("\tMoat:   \"https://moat.example.com\",\n");
    out.push_str("\tCipher: \"shroudb-cipher://token@dedicated:6599\",\n");
    out.push_str("\tToken:  \"moat-token\",\n");
    out.push_str("})\n\n");
    out.push_str("// Always close when done\n");
    out.push_str("defer db.Close()\n");
    out.push_str("```\n\n");

    // Per-engine sections.
    for engine in &ir.engines {
        gen_engine_section(&mut out, engine);
    }

    // Error handling.
    out.push_str("## Error Handling\n\n");
    out.push_str("All methods return `error` (or `(*Response, error)`). Errors from the server ");
    out.push_str("are `*ShrouDBError` with a `Code` field matching the server error code (e.g., `NOTFOUND`, `DENIED`, `BADARG`).\n\n");
    out.push_str("```go\n");
    out.push_str("result, err := db.Cipher.Encrypt(ctx, \"kr\", data)\n");
    out.push_str("if err != nil {\n");
    out.push_str("\tif shroudb.IsCode(err, shroudb.ErrNOTFOUND) {\n");
    out.push_str("\t\t// handle not found\n");
    out.push_str("\t}\n");
    out.push_str("}\n");
    out.push_str("```\n\n");

    // Error codes.
    out.push_str("## Error Codes\n\n");
    out.push_str("| Code | Constant | Description |\n");
    out.push_str("|------|----------|-------------|\n");
    let mut seen = std::collections::BTreeSet::new();
    for engine in &ir.engines {
        for (code, def) in &engine.error_codes {
            if seen.insert(code.clone()) {
                writeln!(out, "| `{code}` | `Err{code}` | {} |", def.description).unwrap();
            }
        }
    }
    out.push('\n');

    // Common mistakes.
    out.push_str("## Common Mistakes\n\n");
    out.push_str("- Always `defer db.Close()` to release connection pool resources\n");
    out.push_str("- Every method requires a `context.Context` as the first argument\n");
    out.push_str("- Engine methods handle serialization — pass Go maps for JSON params, not `json.Marshal()`\n");
    out.push_str(
        "- Accessing a nil engine namespace panics — check your `Options` configuration\n",
    );
    out.push_str("- Optional keyword params use pointer fields in the Options struct — use `&value` to set them\n");

    GeneratedFile {
        path: "AGENTS.md".into(),
        content: out,
    }
}

fn gen_engine_section(out: &mut String, engine: &EngineIR) {
    let pascal = engine.name.to_pascal_case();
    writeln!(out, "## `db.{pascal}` — {}\n", engine.description).unwrap();

    out.push_str("| Method | Args | Returns | Description |\n");
    out.push_str("|--------|------|---------|-------------|\n");

    for cmd in &engine.commands {
        let method = cmd.name.to_pascal_case();
        let args = cmd_args_brief(cmd);
        let returns = cmd_returns_brief(engine, cmd);
        writeln!(
            out,
            "| `{method}` | `{args}` | `{returns}` | {} |",
            cmd.description
        )
        .unwrap();
    }

    out.push('\n');

    // Show a few usage examples for the first 2-3 non-trivial commands.
    let interesting: Vec<_> = engine
        .commands
        .iter()
        .filter(|c| {
            !c.positional_params.is_empty()
                && !matches!(c.verb.as_str(), "AUTH" | "PING" | "HEALTH" | "COMMAND")
        })
        .take(3)
        .collect();

    if !interesting.is_empty() {
        writeln!(out, "### Examples\n").unwrap();
        out.push_str("```go\n");
        out.push_str("ctx := context.Background()\n");
        for cmd in interesting {
            write_example(out, engine, cmd);
        }
        out.push_str("```\n\n");
    }
}

fn cmd_args_brief(cmd: &CommandIR) -> String {
    let mut parts = vec!["ctx".to_string()];
    for p in &cmd.positional_params {
        if p.required {
            parts.push(p.name.clone());
        } else {
            parts.push(format!("{}?", p.name));
        }
    }
    if !cmd.named_params.is_empty() {
        parts.push("opts".into());
    }
    parts.join(", ")
}

fn cmd_returns_brief(engine: &EngineIR, cmd: &CommandIR) -> String {
    let pascal = engine.name.to_pascal_case();
    if cmd.response_fields.is_empty() {
        return "error".into();
    }
    let type_name = format!("{}{}Response", pascal, cmd.name.to_pascal_case());
    format!("*{type_name}, error")
}

fn write_example(out: &mut String, engine: &EngineIR, cmd: &CommandIR) {
    let pascal = engine.name.to_pascal_case();
    let method = cmd.name.to_pascal_case();
    let mut args = vec!["ctx".to_string()];
    for p in &cmd.positional_params {
        args.push(match p.type_key.as_str() {
            "string" | "keyring" | "plaintext" | "ciphertext" | "signature" => {
                format!("\"{}\"", example_value(&p.name))
            }
            "json" | "map" => "map[string]any{}".into(),
            "integer" | "key_version" => "1".into(),
            "boolean" => "true".into(),
            _ => format!("\"{}\"", p.name),
        });
    }
    let args_str = args.join(", ");

    if cmd.response_fields.is_empty() {
        writeln!(out, "err := db.{pascal}.{method}({args_str})",).unwrap();
    } else {
        let fields: Vec<_> = cmd
            .response_fields
            .iter()
            .map(|f| f.name.to_pascal_case())
            .collect();
        writeln!(out, "resp, err := db.{pascal}.{method}({args_str})",).unwrap();
        if let Some(first) = fields.first() {
            writeln!(out, "// resp.{first}").unwrap();
        }
    }
}

fn example_value(name: &str) -> String {
    match name {
        "name" | "keyring" => "my-keyring".into(),
        "schema" => "myapp".into(),
        "id" => "alice".into(),
        "plaintext" | "data" => "SGVsbG8=".into(),
        "ciphertext" => "k3Xm:encrypted...".into(),
        "signature" => "abc123...".into(),
        "token" => "my-token".into(),
        "password" => "s3cret".into(),
        "field" => "email".into(),
        "value" => "alice@example.com".into(),
        "path" => "db/password".into(),
        _ => name.into(),
    }
}
