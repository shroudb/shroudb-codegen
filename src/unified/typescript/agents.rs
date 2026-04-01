//! AGENTS.md generation for the TypeScript SDK.

use crate::generator::GeneratedFile;
use crate::unified::ir::{CommandIR, EngineIR, UnifiedIR};
use heck::ToLowerCamelCase;
use std::fmt::Write;

pub(super) fn generate(ir: &UnifiedIR) -> GeneratedFile {
    let mut out = String::new();
    let pkg = &ir.packages.typescript;

    out.push_str("# ShrouDB SDK — Agent Instructions\n\n");
    out.push_str("> Unified TypeScript SDK for all ShrouDB security engines. ");
    out.push_str("Provides namespaced, type-safe access with built-in serialization.\n\n");

    // Quick context.
    out.push_str("## Quick Context\n\n");
    writeln!(out, "- **Package**: `{pkg}`").unwrap();
    out.push_str("- **Transport**: RESP3 (direct engine connections) or HTTP (Moat gateway)\n");
    out.push_str("- **Pattern**: `db.<engine>.<command>(params)` — all methods async, return typed responses\n");
    out.push_str("- **Serialization**: Handled internally — pass native JS types, get typed objects back\n\n");

    // Connection.
    out.push_str("## Connection\n\n");
    out.push_str("```typescript\n");
    writeln!(out, "import {{ ShrouDB }} from '{pkg}';\n").unwrap();
    out.push_str("// Moat gateway (HTTP) — all engines through one endpoint\n");
    out.push_str(
        "const db = new ShrouDB({ moat: 'https://moat.example.com', token: 'my-token' });\n\n",
    );
    out.push_str("// Direct — only the engines you need\n");
    out.push_str("const db = new ShrouDB({ cipher: 'shroudb-cipher://token@host:6599' });\n\n");
    out.push_str("// Mixed — Moat default + direct overrides\n");
    out.push_str("const db = new ShrouDB({\n");
    out.push_str("  moat: 'https://moat.example.com',\n");
    out.push_str("  cipher: 'shroudb-cipher://token@dedicated:6599',\n");
    out.push_str("  token: 'moat-token',\n");
    out.push_str("});\n\n");
    out.push_str("// Always close when done\n");
    out.push_str("await db.close();\n");
    out.push_str("```\n\n");

    // Per-engine sections.
    for engine in &ir.engines {
        gen_engine_section(&mut out, engine);
    }

    // Error handling.
    out.push_str("## Error Handling\n\n");
    out.push_str("All methods throw `ShrouDBError` on failure. The `code` property matches ");
    out.push_str("the server error code (e.g., `NOTFOUND`, `DENIED`, `BADARG`).\n\n");
    out.push_str("```typescript\n");
    writeln!(out, "import {{ ShrouDBError, ErrorCode }} from '{pkg}';\n").unwrap();
    out.push_str("try {\n");
    out.push_str("  await db.cipher.encrypt('kr', data);\n");
    out.push_str("} catch (err) {\n");
    out.push_str("  if (err instanceof ShrouDBError) {\n");
    out.push_str("    console.error(err.code, err.message);\n");
    out.push_str("  }\n");
    out.push_str("}\n");
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
    out.push_str("- Always `await db.close()` to release connection pool resources\n");
    out.push_str("- Engine methods handle serialization — pass JS objects for JSON params, not `JSON.stringify()`\n");
    out.push_str("- Accessing an engine without a configured URI throws immediately — check your `ShrouDBOptions`\n");
    out.push_str("- Boolean keyword params (like `convergent`, `force`) are flags — `true` sends the keyword, `false`/`undefined` omits it\n");

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
        let method = cmd.name.to_lower_camel_case();
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
        out.push_str("```typescript\n");
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
            parts.push(p.name.clone());
        } else {
            parts.push(format!("{}?", p.name));
        }
    }
    if !cmd.named_params.is_empty() {
        parts.push("options?".into());
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
    let method = cmd.name.to_lower_camel_case();
    let mut args = Vec::new();
    for p in &cmd.positional_params {
        args.push(match p.type_key.as_str() {
            "string" | "keyring" | "plaintext" | "ciphertext" | "signature" => {
                format!("'{}'", example_value(&p.name))
            }
            "json" => "{ /* fields */ }".into(),
            "integer" | "key_version" => "1".into(),
            "boolean" => "true".into(),
            _ => format!("'{}'", p.name),
        });
    }
    let args_str = args.join(", ");

    let returns: Vec<_> = cmd
        .response_fields
        .iter()
        .map(|f| f.name.as_str())
        .collect();
    let destructure = if returns.is_empty() {
        String::new()
    } else {
        format!("const {{ {} }} = ", returns.join(", "))
    };

    writeln!(
        out,
        "{destructure}await db.{}.{method}({args_str});",
        engine.name
    )
    .unwrap();
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
