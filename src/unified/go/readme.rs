//! README.md generation for the Go SDK.

use crate::generator::GeneratedFile;
use crate::unified::ir::{CommandIR, UnifiedIR};
use heck::ToPascalCase;
use std::fmt::Write;

pub(super) fn generate(ir: &UnifiedIR) -> GeneratedFile {
    let mut out = String::new();
    let module = &ir.packages.go_module;

    writeln!(out, "# {module}").unwrap();
    out.push('\n');
    out.push_str("Unified Go SDK for all ShrouDB engines. Provides namespaced, type-safe\n");
    out.push_str("access to every engine with built-in serialization, connection pooling, and\n");
    out.push_str("dual transport support (RESP3 for direct connections, HTTP for Moat gateway).\n");
    out.push('\n');

    // Installation.
    out.push_str("## Installation\n\n");
    writeln!(out, "```bash\ngo get {module}\n```\n").unwrap();

    // Quick start.
    out.push_str("## Quick Start\n\n");
    out.push_str("```go\n");
    out.push_str("package main\n\n");
    out.push_str("import (\n");
    out.push_str("\t\"context\"\n");
    out.push_str("\t\"fmt\"\n");
    out.push_str("\t\"log\"\n\n");
    writeln!(out, "\tshroudb \"{module}\"").unwrap();
    out.push_str(")\n\n");
    out.push_str("func main() {\n");
    out.push_str("\tctx := context.Background()\n\n");

    out.push_str("\t// Connect via Moat gateway (routes all engines through one endpoint)\n");
    out.push_str("\tdb, err := shroudb.New(shroudb.Options{\n");
    out.push_str("\t\tMoat:  \"https://moat.example.com\",\n");
    out.push_str("\t\tToken: \"my-token\",\n");
    out.push_str("\t})\n");
    out.push_str("\tif err != nil {\n");
    out.push_str("\t\tlog.Fatal(err)\n");
    out.push_str("\t}\n");
    out.push_str("\tdefer db.Close()\n\n");

    // Show a few example calls.
    if let Some(cipher) = ir.engines.iter().find(|e| e.name == "cipher")
        && cipher.commands.iter().any(|c| c.name == "encrypt")
    {
        out.push_str("\t// Encrypt data\n");
        out.push_str("\tresult, err := db.Cipher.Encrypt(ctx, \"my-keyring\", \"SGVsbG8=\")\n");
        out.push_str("\tif err != nil {\n");
        out.push_str("\t\tlog.Fatal(err)\n");
        out.push_str("\t}\n");
        out.push_str("\tfmt.Println(result.Ciphertext)\n");
    }

    out.push_str("}\n");
    out.push_str("```\n\n");

    // Direct connection example.
    out.push_str("### Direct Engine Connections\n\n");
    out.push_str("```go\n");
    out.push_str("db, err := shroudb.New(shroudb.Options{\n");
    for engine in &ir.engines {
        let pascal = engine.name.to_pascal_case();
        if !engine.uri_schemes.is_empty() {
            let scheme = &engine.uri_schemes[0];
            writeln!(
                out,
                "\t{pascal}: \"{scheme}token@localhost:{}\",",
                engine.default_port
            )
            .unwrap();
        }
    }
    out.push_str("})\n");
    out.push_str("```\n\n");

    // Connection modes.
    out.push_str("## Connection Modes\n\n");
    out.push_str("### Moat Gateway (HTTP)\n\n");
    out.push_str("Routes all engine commands through a single Moat endpoint via HTTP POST.\n\n");
    out.push_str("```go\n");
    out.push_str("db, _ := shroudb.New(shroudb.Options{Moat: \"https://moat.example.com\", Token: \"my-token\"})\n");
    out.push_str("```\n\n");
    out.push_str("### Moat Gateway (RESP3)\n\n");
    out.push_str("Direct RESP3 connection to Moat with engine-prefixed commands.\n\n");
    out.push_str("```go\n");
    out.push_str("db, _ := shroudb.New(shroudb.Options{Moat: \"shroudb-moat://my-token@moat.example.com:8201\"})\n");
    out.push_str("```\n\n");
    out.push_str("### Direct Engine Connections\n\n");
    out.push_str(
        "Connect to individual engines via RESP3. Only configure the engines you need.\n\n",
    );
    out.push_str("```go\n");
    out.push_str("db, _ := shroudb.New(shroudb.Options{\n");
    out.push_str("\tCipher: \"shroudb-cipher://token@cipher-host:6599\",\n");
    out.push_str("\tSigil:  \"shroudb-sigil://token@sigil-host:6499\",\n");
    out.push_str("})\n");
    out.push_str("```\n\n");
    out.push_str("### Mixed Mode\n\n");
    out.push_str("Route most engines through Moat, but connect directly to specific engines.\n\n");
    out.push_str("```go\n");
    out.push_str("db, _ := shroudb.New(shroudb.Options{\n");
    out.push_str("\tMoat:   \"https://moat.example.com\",\n");
    out.push_str("\tCipher: \"shroudb-cipher://token@dedicated-cipher:6599\", // direct\n");
    out.push_str("\tToken:  \"moat-token\",\n");
    out.push_str("})\n");
    out.push_str("```\n\n");

    // Engine reference.
    out.push_str("## Engines\n\n");
    for engine in &ir.engines {
        let pascal = engine.name.to_pascal_case();
        writeln!(out, "### `db.{pascal}`\n").unwrap();
        writeln!(out, "{}\n", engine.description).unwrap();

        out.push_str("| Method | Description |\n");
        out.push_str("|--------|-------------|\n");
        for cmd in &engine.commands {
            let method = cmd.name.to_pascal_case();
            let params = brief_params(cmd);
            writeln!(out, "| `{method}({params})` | {} |", cmd.description).unwrap();
        }
        out.push('\n');
    }

    // Error handling.
    out.push_str("## Error Handling\n\n");
    out.push_str("```go\n");
    out.push_str("result, err := db.Cipher.Encrypt(ctx, \"missing-keyring\", data)\n");
    out.push_str("if err != nil {\n");
    out.push_str("\tif shroudb.IsCode(err, shroudb.ErrNOTFOUND) {\n");
    out.push_str("\t\tfmt.Println(\"Keyring not found\")\n");
    out.push_str("\t}\n");
    out.push_str("}\n");
    out.push_str("```\n");

    GeneratedFile {
        path: "README.md".into(),
        content: out,
    }
}

fn brief_params(cmd: &CommandIR) -> String {
    let mut parts = vec!["ctx".to_string()];
    for p in &cmd.positional_params {
        parts.push(p.name.clone());
    }
    if !cmd.named_params.is_empty() {
        parts.push("opts".into());
    }
    parts.join(", ")
}
