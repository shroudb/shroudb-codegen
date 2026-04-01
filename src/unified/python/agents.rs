//! AGENTS.md generation for the Python SDK.

use crate::generator::GeneratedFile;
use crate::unified::ir::{CommandIR, EngineIR, UnifiedIR};
use heck::ToSnakeCase;
use std::fmt::Write;

pub(super) fn generate(ir: &UnifiedIR) -> GeneratedFile {
    let mut out = String::new();
    let pkg = &ir.packages.python;

    out.push_str("# ShrouDB SDK — Agent Instructions\n\n");
    out.push_str("> Unified Python SDK for all ShrouDB security engines. ");
    out.push_str("Provides namespaced, type-safe access with built-in serialization.\n\n");

    // Quick context.
    out.push_str("## Quick Context\n\n");
    writeln!(out, "- **Package**: `{pkg}`").unwrap();
    out.push_str("- **Transport**: RESP3 (direct engine connections) or HTTP (Moat gateway)\n");
    out.push_str("- **Pattern**: `await db.<engine>.<command>(params)` — all methods async, return typed dataclasses\n");
    out.push_str("- **Serialization**: Handled internally — pass native Python types, get typed objects back\n\n");

    // Connection.
    out.push_str("## Connection\n\n");
    out.push_str("```python\n");
    writeln!(out, "from {pkg} import ShrouDB\n").unwrap();
    out.push_str("# Moat gateway (HTTP) — all engines through one endpoint\n");
    out.push_str(
        "async with ShrouDB(moat=\"https://moat.example.com\", token=\"my-token\") as db:\n",
    );
    out.push_str("    ...\n\n");
    out.push_str("# Direct — only the engines you need\n");
    out.push_str("db = ShrouDB(cipher=\"shroudb-cipher://token@host:6599\")\n\n");
    out.push_str("# Mixed — Moat default + direct overrides\n");
    out.push_str("db = ShrouDB(\n");
    out.push_str("    moat=\"https://moat.example.com\",\n");
    out.push_str("    cipher=\"shroudb-cipher://token@dedicated:6599\",\n");
    out.push_str("    token=\"moat-token\",\n");
    out.push_str(")\n\n");
    out.push_str("# Always close when done (or use async with)\n");
    out.push_str("await db.close()\n");
    out.push_str("```\n\n");

    // Per-engine sections.
    for engine in &ir.engines {
        gen_engine_section(&mut out, engine);
    }

    // Error handling.
    out.push_str("## Error Handling\n\n");
    out.push_str("All methods raise ``ShrouDBError`` on failure. The ``code`` attribute matches ");
    out.push_str("the server error code (e.g., ``NOTFOUND``, ``DENIED``, ``BADARG``).\n\n");
    out.push_str("```python\n");
    writeln!(out, "from {pkg} import ShrouDBError").unwrap();
    writeln!(out, "from {pkg}.errors import ErrorCode\n").unwrap();
    out.push_str("try:\n");
    out.push_str("    await db.cipher.encrypt(\"kr\", data)\n");
    out.push_str("except ShrouDBError as err:\n");
    out.push_str("    print(err.code, err.message)\n");
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
    out.push_str("- Always ``await db.close()`` or use ``async with`` to release connection pool resources\n");
    out.push_str("- Engine methods handle serialization — pass Python dicts for JSON params, not ``json.dumps()``\n");
    out.push_str("- Accessing an engine without a configured URI raises immediately — check your constructor kwargs\n");
    out.push_str("- Boolean keyword params (like ``convergent``, ``force``) are flags — ``True`` sends the keyword, ``False`` omits it\n");

    GeneratedFile {
        path: "AGENTS.md".into(),
        content: out,
    }
}

fn gen_engine_section(out: &mut String, engine: &EngineIR) {
    writeln!(out, "## `db.{}` — {}\n", engine.name, engine.description).unwrap();

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
        out.push_str("```python\n");
        for cmd in interesting {
            write_example(out, engine, cmd);
        }
        out.push_str("```\n\n");
    }
}

fn cmd_args_brief(cmd: &CommandIR) -> String {
    let mut parts = Vec::new();
    for p in &cmd.positional_params {
        let snake = p.name.to_snake_case();
        if p.required {
            parts.push(snake);
        } else {
            parts.push(format!("{snake}=None"));
        }
    }
    if !cmd.named_params.is_empty() {
        parts.push("**kwargs".into());
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
        .map(|f| f.name.as_str())
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
            "json" => "{}".into(),
            "integer" | "key_version" => "1".into(),
            "boolean" => "True".into(),
            _ => format!("\"{}\"", p.name),
        });
    }
    let args_str = args.join(", ");

    let returns: Vec<_> = cmd
        .response_fields
        .iter()
        .map(|f| f.name.as_str())
        .collect();
    let lhs = if returns.is_empty() {
        String::new()
    } else {
        "result = ".to_string()
    };

    writeln!(out, "{lhs}await db.{}.{method}({args_str})", engine.name).unwrap();

    if !returns.is_empty() {
        let first = returns[0];
        writeln!(out, "print(result.{first})").unwrap();
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
