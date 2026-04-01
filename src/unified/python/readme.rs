//! README.md generation for the Python SDK.

use crate::generator::GeneratedFile;
use crate::unified::ir::{CommandIR, UnifiedIR};
use heck::ToSnakeCase;
use std::fmt::Write;

pub(super) fn generate(ir: &UnifiedIR) -> GeneratedFile {
    let mut out = String::new();
    let pkg = &ir.packages.python;

    writeln!(out, "# {pkg}").unwrap();
    out.push('\n');
    out.push_str("Unified Python SDK for all ShrouDB engines. Provides namespaced, type-safe\n");
    out.push_str("access to every engine with built-in serialization, connection pooling, and\n");
    out.push_str("dual transport support (RESP3 for direct connections, HTTP for Moat gateway).\n");
    out.push('\n');

    // Installation.
    out.push_str("## Installation\n\n");
    writeln!(out, "```bash\npip install {pkg}\n```\n").unwrap();

    // Quick start.
    out.push_str("## Quick Start\n\n");
    out.push_str("```python\n");
    out.push_str("import asyncio\n");
    writeln!(out, "from {pkg} import ShrouDB\n").unwrap();

    out.push_str("async def main():\n");
    out.push_str("    # Connect via Moat gateway (routes all engines through one endpoint)\n");
    out.push_str("    async with ShrouDB(\n");
    out.push_str("        moat=\"https://moat.example.com\",\n");
    out.push_str("        token=\"my-token\",\n");
    out.push_str("    ) as db:\n");

    // Show a few example calls.
    if let Some(cipher) = ir.engines.iter().find(|e| e.name == "cipher")
        && cipher.commands.iter().any(|c| c.name == "encrypt")
    {
        out.push_str("        # Encrypt data\n");
        out.push_str("        result = await db.cipher.encrypt(\"my-keyring\", \"SGVsbG8=\")\n");
        out.push_str("        print(result.ciphertext)\n\n");
    }

    if let Some(sigil) = ir.engines.iter().find(|e| e.name == "sigil")
        && sigil.commands.iter().any(|c| c.name == "USER_CREATE")
    {
        out.push_str("        # Create a user\n");
        out.push_str("        user = await db.sigil.user_create(\"myapp\", \"alice\",\n");
        out.push_str("            password=\"s3cret\", email=\"alice@example.com\")\n\n");
    }

    out.push_str("\nasyncio.run(main())\n");
    out.push_str("```\n\n");

    // Direct connections.
    out.push_str("```python\n");
    out.push_str("# Or connect to individual engines directly\n");
    out.push_str("db = ShrouDB(\n");
    for engine in &ir.engines {
        if !engine.uri_schemes.is_empty() {
            let scheme = &engine.uri_schemes[0];
            writeln!(
                out,
                "    {name}=\"{scheme}token@localhost:{port}\",",
                name = engine.name,
                port = engine.default_port
            )
            .unwrap();
        }
    }
    out.push_str(")\n");
    out.push_str("```\n\n");

    // Connection modes.
    out.push_str("## Connection Modes\n\n");
    out.push_str("### Moat Gateway (HTTP)\n\n");
    out.push_str("Routes all engine commands through a single Moat endpoint via HTTP POST.\n\n");
    out.push_str("```python\n");
    out.push_str("db = ShrouDB(moat=\"https://moat.example.com\", token=\"my-token\")\n");
    out.push_str("```\n\n");
    out.push_str("### Moat Gateway (RESP3)\n\n");
    out.push_str("Direct RESP3 connection to Moat with engine-prefixed commands.\n\n");
    out.push_str("```python\n");
    out.push_str("db = ShrouDB(moat=\"shroudb-moat://my-token@moat.example.com:8201\")\n");
    out.push_str("```\n\n");
    out.push_str("### Direct Engine Connections\n\n");
    out.push_str(
        "Connect to individual engines via RESP3. Only configure the engines you need.\n\n",
    );
    out.push_str("```python\n");
    out.push_str("db = ShrouDB(\n");
    out.push_str("    cipher=\"shroudb-cipher://token@cipher-host:6599\",\n");
    out.push_str("    sigil=\"shroudb-sigil://token@sigil-host:6499\",\n");
    out.push_str(")\n");
    out.push_str("```\n\n");
    out.push_str("### Mixed Mode\n\n");
    out.push_str("Route most engines through Moat, but connect directly to specific engines.\n\n");
    out.push_str("```python\n");
    out.push_str("db = ShrouDB(\n");
    out.push_str("    moat=\"https://moat.example.com\",\n");
    out.push_str("    cipher=\"shroudb-cipher://token@dedicated-cipher:6599\",  # direct\n");
    out.push_str("    token=\"moat-token\",\n");
    out.push_str(")\n");
    out.push_str("```\n\n");

    // Engine reference.
    out.push_str("## Engines\n\n");
    for engine in &ir.engines {
        writeln!(out, "### `db.{}`\n", engine.name).unwrap();
        writeln!(out, "{}\n", engine.description).unwrap();

        out.push_str("| Method | Description |\n");
        out.push_str("|--------|-------------|\n");
        for cmd in &engine.commands {
            let method = cmd.name.to_snake_case();
            let params = brief_params(cmd);
            writeln!(out, "| `{method}({params})` | {} |", cmd.description).unwrap();
        }
        out.push('\n');
    }

    // Error handling.
    out.push_str("## Error Handling\n\n");
    out.push_str("```python\n");
    writeln!(
        out,
        "from {pkg} import ShrouDBError\nfrom {pkg}.errors import ErrorCode\n"
    )
    .unwrap();
    out.push_str("try:\n");
    out.push_str("    await db.cipher.encrypt(\"missing-keyring\", data)\n");
    out.push_str("except ShrouDBError as err:\n");
    out.push_str("    if err.code == ErrorCode.NOTFOUND:\n");
    out.push_str("        print(\"Keyring not found\")\n");
    out.push_str("```\n");

    GeneratedFile {
        path: "README.md".into(),
        content: out,
    }
}

fn brief_params(cmd: &CommandIR) -> String {
    let mut parts = Vec::new();
    for p in &cmd.positional_params {
        parts.push(p.name.to_snake_case());
    }
    if !cmd.named_params.is_empty() {
        parts.push("**kwargs".into());
    }
    parts.join(", ")
}
