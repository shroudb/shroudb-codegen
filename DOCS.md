# ShrouDB Codegen Documentation

## What It Generates

ShrouDB Codegen produces client SDKs in four languages:

| Language | Package format |
|----------|---------------|
| Python | pyproject.toml, typed dataclasses |
| TypeScript | package.json, TypeScript interfaces |
| Go | go.mod, typed structs |
| Ruby | gemspec, typed classes |

Each generated SDK includes:

- **Typed client** -- one method per spec command or endpoint, with full type annotations.
- **Response types** -- a struct, dataclass, or interface for every command response.
- **Error hierarchy** -- one error class per spec error code, so callers can catch specific failures.
- **Connection pool** -- idle connection reuse with configurable limits (wire protocol specs).
- **Pipeline support** -- batch multiple commands in a single round-trip (wire protocol specs).
- **Streaming subscribe** -- async iterator for real-time event notifications (wire protocol specs).
- **Auth handling** -- automatic Bearer token headers (HTTP API specs).
- **Package metadata** -- ready-to-publish packaging for each language ecosystem.

## How to Use It

### Commands

```bash
# Generate a Python client from a protocol spec
shroudb-codegen --spec protocol.toml --lang python --output generated/python

# Generate clients for all supported languages
shroudb-codegen --spec protocol.toml --lang all --output generated/

# Preview which files would be generated without writing them
shroudb-codegen --spec protocol.toml --lang all --dry-run
```

### CLI Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--spec` | `protocol.toml` | Path to the TOML spec file |
| `--lang` | (required) | `python`, `typescript`, `go`, `ruby`, or `all` |
| `--output` | `generated` | Output directory |
| `--http` | | Generate HTTP REST SDK instead of RESP3 (for engines with HTTP APIs) |
| `--dry-run` | | List files that would be generated, without writing |

### TOML Spec Format

The spec format is auto-detected based on top-level keys.

**Wire protocol spec** -- uses a `[protocol]` section to define commands, arguments, response fields, and error codes for a binary wire protocol engine.

**HTTP API spec** -- uses an `[api]` section to define endpoints, request/response schemas, and auth requirements for an HTTP-based engine.

**Composite spec** -- uses `[[engines]]` to reference multiple engine specs by relative path and produce a single unified SDK with engine-namespaced methods.

## Generated SDK Features

### Wire Protocol Clients

Generated clients connect via URI, manage a connection pool, and expose one method per command:

```python
from shroudb import ShrouDB

client = ShrouDB("shroudb://localhost:6400")
result = client.verify("auth-tokens", user_id, password)
```

Pipelines batch multiple commands into a single round-trip:

```python
pipe = client.pipeline()
pipe.verify("auth-tokens", user_id, password)
pipe.rotate("encryption-keys", key_id)
results = pipe.execute()
```

### HTTP API Clients

Generated HTTP clients handle base URL configuration and automatic auth headers:

```typescript
const client = new ShrouDBSigil({ baseUrl: 'https://sigil.example.com', token: 'sk-...' });
const session = await client.login(email, password);
```

### Moat Unified SDK

When using a composite spec, Codegen produces a single SDK where each engine is accessed as a namespace:

```typescript
const client = new ShrouDB({ moat: 'https://moat.example.com', token: 'sk-...' });
await client.cipher.encrypt('payments', plaintext);
await client.keep.put('db/api-key', secret);
await client.sigil.userCreate({ email, password });
```

Generates TypeScript, Python, Go, and Ruby SDKs from composite specs.

## E2EE Blind Mode

Generated SDKs include E2EE support for Veil and Sigil:

- **Veil `put` and `search`** accept a `blind` boolean option. When `true`, the client provides pre-computed blind tokens (from `shroudb-veil-blind`) instead of plaintext. The server stores and searches without seeing plaintext.
- **Sigil envelope create/update** payloads accept per-field blind wrappers (`{"blind": true, "value": "...", "tokens": "..."}`). Fields wrapped this way skip server-side Cipher encryption and Veil tokenization.

The `flag` parameter type in `protocol.toml` generates as a boolean option (no value on the wire — the keyword is present or absent).

Required parameters that appear with a `KEYWORD <value>` prefix in a command's
`syntax` string (e.g., Forge's `CA CREATE <name> <algorithm> SUBJECT <subject>`)
are emitted as positional arguments in the public SDK method signature, but the
keyword is prepended on the wire. Codegen derives this from the syntax string;
no extra field in `protocol.toml` is required.

## Supported Engines

| Engine | Spec type | Description |
|--------|-----------|-------------|
| ShrouDB | Wire protocol | Core encrypted KV database |
| Cipher | Wire protocol | Encryption as a service |
| Sigil | HTTP API | Authentication and identity |
| Forge | Wire protocol | Key and token generation |
| Keep | Wire protocol | Secret storage |
| Chronicle | Wire protocol | Audit logging |
| Veil | Wire protocol | Searchable encryption |
| Courier | Wire protocol | Secure messaging |
| Sentry | Wire protocol | Access control |
| Moat | Composite | Unified gateway that imports all engine specs into a single SDK |
