# shroudb-codegen

Generates a single unified SDK per language from the Moat composite spec. Each SDK provides engine-namespaced methods with built-in serialization, dual RESP3/HTTP transport, and full documentation.

## Usage

```bash
# Generate all language SDKs from the Moat composite spec
shroudb-codegen --spec ../shroudb-moat/protocol.toml --lang all --output generated/

# Single language
shroudb-codegen --spec ../shroudb-moat/protocol.toml --lang typescript --output sdk-ts/

# Dry run (list files without writing)
shroudb-codegen --spec ../shroudb-moat/protocol.toml --lang all --dry-run
```

### Options

| Flag | Default | Description |
|------|---------|-------------|
| `--spec` | `protocol.toml` | Path to the Moat composite spec |
| `--lang` | (required) | `typescript`, `python`, `go`, `ruby`, or `all` |
| `--output` | `generated` | Output directory |
| `--dry-run` | | Print file list without writing |

## Generated SDKs

| Language | Package | Files |
|----------|---------|-------|
| TypeScript | `@shroudb/sdk` | 29 |
| Python | `shroudb` | 38 |
| Go | `github.com/shroudb/shroudb-go` | 18 |
| Ruby | `shroudb` gem | 27 |

Each SDK includes:

- **Engine namespaces** — `db.cipher.encrypt(...)`, `db.sigil.envelopeCreate(...)`, etc.
- **Dual transport** — RESP3 for direct engine connections, HTTP for Moat gateway
- **Per-engine URIs** — configure only the engines you use; Moat routes the rest
- **Mixed mode** — Moat default with per-engine direct overrides
- **Internal serialization** — methods accept native types, build RESP3 frames internally
- **Typed responses** — language-idiomatic response types per command
- **Error hierarchy** — unified error class with server error codes
- **Connection pooling** — idle connection reuse with configurable limits
- **README.md** — installation, quick start, connection modes, engine reference
- **AGENTS.md** — AI coding assistant instructions with commands, types, examples
- **postinstall** (TypeScript) — injects AGENTS.md pointer into project root

### Example (TypeScript)

```typescript
import { ShrouDB } from '@shroudb/sdk';

// Moat gateway — all engines through one endpoint
const db = new ShrouDB({ moat: 'https://moat.example.com', token: 'my-token' });

// Or direct connections — only the engines you need
const db = new ShrouDB({
  cipher: 'shroudb-cipher://token@cipher-host:6599',
  keep: 'shroudb-keep://token@keep-host:6899',
});

// Namespaced, serialization handled internally
const enc = await db.cipher.encrypt('payments', btoa('hello'));
const dec = await db.cipher.decrypt('payments', enc.ciphertext);
await db.keep.put('db/api-key', btoa('sk-secret'));
await db.close();
```

### Example (Python)

```python
from shroudb import ShrouDB

async with ShrouDB(moat="https://moat.example.com", token="my-token") as db:
    enc = await db.cipher.encrypt("payments", plaintext_b64)
    dec = await db.cipher.decrypt("payments", enc.ciphertext)
    await db.keep.put("db/api-key", secret_b64)
```

## Architecture

```
src/
  spec/          — Protocol spec parsers (handles all TOML format variants)
  unified/
    ir.rs        — Intermediate representation (normalizes specs into common types)
    typescript/  — TypeScript SDK generator
    python/      — Python SDK generator
    go/          — Go SDK generator
    ruby/        — Ruby SDK generator
```

The Moat composite `protocol.toml` references all 9 engine specs. Codegen loads them, builds a unified IR, then each language generator produces a complete SDK from that IR.

## Testing

```bash
cd test-sandbox
make test-clients          # all languages
./run-tests.sh --lang python   # single language
./run-tests.sh --keep          # keep servers running after tests
```

Starts all 9 engine servers on random ports, generates unified SDKs, and runs per-engine integration tests in each language.

### Test matrix

| Engine | Python | TypeScript | Go | Ruby |
|--------|--------|-----------|-----|------|
| shroudb (7 checks) | PASS | PASS | PASS | PASS |
| cipher (12 checks) | PASS | PASS | PASS | PASS |
| sigil (11 checks) | PASS | PASS | PASS | PASS |
| forge (8 checks) | PASS | PASS | PASS | PASS |
| sentry (8 checks) | PASS | PASS | PASS | PASS |
| keep (11 checks) | PASS | PASS | PASS | PASS |
| courier (5 checks) | PASS | PASS | PASS | PASS |
| chronicle (8 checks) | PASS | PASS | PASS | PASS |
| veil (6 checks) | PASS | PASS | PASS | PASS |

**339 checks across 9 engines, 4 languages, 36 test suites.**

Requires engine binaries (from sibling repos) and language runtimes (`python3`, `node`, `go`, `ruby`).

## Building

```bash
cargo build --release
```

## License

MIT OR Apache-2.0
