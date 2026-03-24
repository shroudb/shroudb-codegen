# shroudb-codegen

Unified SDK generator for all ShrouDB protocols. Reads a `protocol.toml` spec and produces ready-to-publish client libraries in Python, TypeScript, Go, and Ruby.

## Supported specs

| Spec | Type | Description |
|------|------|-------------|
| `shroudb` | Wire (RESP3) | Credential management server |
| `shroudb-transit` | Wire (RESP3) | Encryption-as-a-service |
| `shroudb-auth` | HTTP API | Authentication service |

Spec format is auto-detected: `[protocol]` for wire, `[api]` for HTTP.

## Usage

```bash
# Generate Python client from a wire protocol spec
shroudb-codegen --spec protocol.toml --lang python --output generated/python

# Generate all languages
shroudb-codegen --spec protocol.toml --lang all --output generated/

# Dry run (list files without writing)
shroudb-codegen --spec protocol.toml --lang all --dry-run
```

### Options

| Flag | Default | Description |
|------|---------|-------------|
| `--spec` | `protocol.toml` | Path to the protocol spec file |
| `--lang` | (required) | `python`, `typescript`, `go`, `ruby`, `proto`, or `all` |
| `--output` | `generated` | Output directory |
| `--dry-run` | | Print file list without writing |

## Generated output

### Wire protocol clients (shroudb, shroudb-transit)

Each language gets a complete, publishable package:

- **Connection codec** — RESP3 frame parser and serializer
- **Connection pool** — idle connection reuse with configurable limits
- **Typed client** — URI-based connect, one method per spec command
- **Pipeline** — batch multiple commands in a single round-trip
- **Streaming subscribe** — async iterator for real-time event notifications
- **Response types** — typed structs/dataclasses for every command response
- **Error hierarchy** — one error class per spec error code
- **Package metadata** — pyproject.toml, package.json, go.mod, gemspec

### HTTP API clients (shroudb-auth)

- **Typed client** — base URL + keyspace, one method per endpoint
- **Auth handling** — automatic Bearer token headers for access/refresh auth
- **Response types** — typed structs for every endpoint response
- **Error hierarchy** — mapped from HTTP status codes
- **Package metadata** — same per-language packaging as wire clients

## Testing

The `test-sandbox/` directory validates all generated clients against live servers.

```bash
cd test-sandbox
make test-clients
```

This will:
1. Generate clients for all 3 specs (shroudb, transit, auth)
2. Start all 3 servers on random ports
3. Run integration tests in all 4 languages
4. Report pass/fail per suite

Requires the server binaries (built automatically from sibling repos) and language runtimes (`python3`, `node`, `go`, `ruby`).

### Current test coverage

| Spec | Python | TypeScript | Go | Ruby |
|------|--------|------------|-----|------|
| shroudb (22 checks) | PASS | PASS | PASS | PASS |
| shroudb-transit (13 checks) | PASS | PASS | PASS | PASS |
| shroudb-auth (15 checks) | PASS | PASS | PASS | PASS |

## Building

```bash
cargo build --release
```

## License

MIT OR Apache-2.0
