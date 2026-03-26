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
const client = new ShrouDBAuth({ baseUrl: 'https://auth.example.com', token: 'sk-...' });
const session = await client.login(email, password);
```

### Moat Unified SDK

When using a composite spec, Codegen produces a single SDK where each engine is accessed as a namespace:

```typescript
const client = new ShrouDB({ endpoint: 'https://moat.example.com', token: 'sk-...' });
await client.vault.verify('auth-tokens', userId, password);
await client.transit.encrypt('payments', plaintext);
await client.control.createTenant({ id: 'acme', name: 'Acme Corp' });
```

Currently generates TypeScript for composite specs. Python, Go, and Ruby support is planned.

## Supported Engines

| Engine | Spec type | Description |
|--------|-----------|-------------|
| ShrouDB | Wire protocol | Credential management |
| ShrouDB Transit | Wire protocol | Encryption as a service |
| ShrouDB Auth | HTTP API | Authentication service |
| ShrouDB Moat | Composite | Unified hub that imports engine specs into a single SDK |
