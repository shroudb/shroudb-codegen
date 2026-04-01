//! AGENTS.md generation for the Ruby SDK.

use crate::generator::GeneratedFile;
use crate::unified::ir::{CommandIR, EngineIR, UnifiedIR};
use heck::ToSnakeCase;
use std::fmt::Write;

pub(super) fn generate(ir: &UnifiedIR) -> GeneratedFile {
    let mut out = String::new();
    let gem = &ir.packages.ruby;

    out.push_str("# ShrouDB SDK — Agent Instructions\n\n");
    out.push_str("> Unified Ruby SDK for all ShrouDB security engines. ");
    out.push_str("Provides namespaced access with built-in serialization.\n\n");

    // Quick context.
    out.push_str("## Quick Context\n\n");
    writeln!(out, "- **Gem**: `{gem}`").unwrap();
    out.push_str("- **Transport**: RESP3 (direct engine connections) or HTTP (Moat gateway)\n");
    out.push_str("- **Pattern**: `db.<engine>.<command>(params)` — all methods return typed Struct responses\n");
    out.push_str("- **Serialization**: Handled internally — pass native Ruby types, get Struct objects back\n\n");

    // Connection.
    out.push_str("## Connection\n\n");
    out.push_str("```ruby\n");
    writeln!(out, "require \"{gem}\"\n").unwrap();
    out.push_str("# Moat gateway (HTTP) — all engines through one endpoint\n");
    out.push_str(
        "db = ShrouDB::Client.new(moat: \"https://moat.example.com\", token: \"my-token\")\n\n",
    );
    out.push_str("# Direct — only the engines you need\n");
    out.push_str("db = ShrouDB::Client.new(cipher: \"shroudb-cipher://token@host:6599\")\n\n");
    out.push_str("# Mixed — Moat default + direct overrides\n");
    out.push_str("db = ShrouDB::Client.new(\n");
    out.push_str("  moat: \"https://moat.example.com\",\n");
    out.push_str("  cipher: \"shroudb-cipher://token@dedicated:6599\",\n");
    out.push_str("  token: \"moat-token\"\n");
    out.push_str(")\n\n");
    out.push_str("# Always close when done\n");
    out.push_str("db.close\n");
    out.push_str("```\n\n");

    // Per-engine sections.
    for engine in &ir.engines {
        gen_engine_section(&mut out, engine);
    }

    // Error handling.
    out.push_str("## Error Handling\n\n");
    out.push_str("All methods raise `ShrouDB::Error` on failure. The `code` attribute matches ");
    out.push_str("the server error code (e.g., `NOTFOUND`, `DENIED`, `BADARG`).\n\n");
    out.push_str("```ruby\n");
    out.push_str("begin\n");
    out.push_str("  db.cipher.encrypt(\"kr\", data)\n");
    out.push_str("rescue ShrouDB::Error => e\n");
    out.push_str("  puts \"#{e.code}: #{e.message}\"\n");
    out.push_str("end\n");
    out.push_str("```\n\n");

    // Error codes.
    out.push_str("## Error Codes\n\n");
    out.push_str("| Code | Description |\n");
    out.push_str("|------|-------------|\n");
    let mut seen = std::collections::BTreeSet::new();
    for engine in &ir.engines {
        for (code, def) in &engine.error_codes {
            if seen.insert(code.clone()) {
                writeln!(out, "| `{code}` | {} |", def.description).unwrap();
            }
        }
    }
    out.push('\n');

    // Common mistakes.
    out.push_str("## Common Mistakes\n\n");
    out.push_str("- Always call `db.close` to release connection pool resources\n");
    out.push_str("- Engine methods handle serialization — pass Ruby Hashes for JSON params, not `JSON.generate()`\n");
    out.push_str("- Accessing an engine without a configured URI returns `nil` — check your constructor arguments\n");
    out.push_str("- Boolean keyword params (like `convergent`, `force`) are flags — `true` sends the keyword, `false`/`nil` omits it\n");

    GeneratedFile {
        path: "AGENTS.md".into(),
        content: out,
    }
}

fn gen_engine_section(out: &mut String, engine: &EngineIR) {
    writeln!(
        out,
        "## `db.{}` — {}\n",
        engine.name.to_snake_case(),
        engine.description
    )
    .unwrap();

    out.push_str("| Method | Args | Returns | Description |\n");
    out.push_str("|--------|------|---------|-------------|\n");

    for cmd in &engine.commands {
        let method = cmd.name.to_snake_case();
        let args = cmd_args_brief(cmd);
        let returns = cmd_returns_brief(cmd);
        writeln!(
            out,
            "| `{method}` | `{args}` | `{returns}` | {} |",
            cmd.description
        )
        .unwrap();
    }

    out.push('\n');

    // Show usage examples for the first 2-3 non-trivial commands.
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
        out.push_str("```ruby\n");
        for cmd in interesting {
            write_example(out, engine, cmd);
        }
        out.push_str("```\n\n");
    }
}

fn cmd_args_brief(cmd: &CommandIR) -> String {
    let mut parts = Vec::new();
    for p in &cmd.positional_params {
        if p.required {
            parts.push(p.name.to_snake_case());
        } else {
            parts.push(format!("{}?", p.name.to_snake_case()));
        }
    }
    if !cmd.named_params.is_empty() {
        parts.push("**opts".into());
    }
    parts.join(", ")
}

fn cmd_returns_brief(cmd: &CommandIR) -> String {
    if cmd.response_fields.is_empty() {
        return "{}".into();
    }
    let fields: Vec<_> = cmd
        .response_fields
        .iter()
        .map(|f| f.name.to_snake_case())
        .collect();
    format!("{{ {} }}", fields.join(", "))
}

fn write_example(out: &mut String, engine: &EngineIR, cmd: &CommandIR) {
    let method = cmd.name.to_snake_case();
    let mut args = Vec::new();
    for p in &cmd.positional_params {
        args.push(match p.type_key.as_str() {
            "string" | "keyring" | "plaintext" | "ciphertext" | "signature" => {
                format!("\"{}\"", example_value(&p.name))
            }
            "json" => "{ }".into(),
            "integer" | "key_version" => "1".into(),
            "boolean" => "true".into(),
            _ => format!("\"{}\"", p.name),
        });
    }
    let args_str = args.join(", ");

    let returns: Vec<_> = cmd
        .response_fields
        .iter()
        .map(|f| f.name.to_snake_case())
        .collect();

    if returns.is_empty() {
        writeln!(
            out,
            "db.{}.{method}({args_str})",
            engine.name.to_snake_case()
        )
        .unwrap();
    } else {
        writeln!(
            out,
            "result = db.{}.{method}({args_str})",
            engine.name.to_snake_case()
        )
        .unwrap();
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
