# ShrouDB Codegen

Unified SDK generator for ShrouDB wire protocol and HTTP API specs.

## Pre-push checklist (mandatory — no exceptions)

Every check below **must** pass locally before pushing to any branch. Do not rely on GitHub Actions to catch these — CI is a safety net, not the first line of defense.

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### Rules

1. **Run all checks before every push.** No shortcuts, no "I'll fix it in the next commit."
2. **Pre-existing issues must be fixed.** If any check reveals warnings, formatting drift, or any other issue — even if you didn't introduce it — fix it in the same changeset. Do not skip it as "not in scope", "pre-existing", or "unrelated." If the tool flags it, it gets fixed.
3. **Never suppress or bypass checks.** Do not add `#[allow(...)]` to silence clippy, do not push with known failures. Do not use `--no-verify` on git push.
4. **Warnings are errors.** `RUSTFLAGS="-D warnings"` is set in CI. Clippy runs with `-D warnings`. Both compiler warnings and clippy warnings fail the build.
5. **Documentation must stay in sync.** Any change that affects CLI commands, config keys, public API, or user-facing behavior **must** include corresponding updates to `README.md`, `DOCS.md`, and `ABOUT.md` in the same changeset. Do not merge code changes with stale docs.
6. **Cross-repo impact must be addressed.** If a change affects shared types, protocols, or APIs consumed by other ShrouDB repos, update those downstream repos in the same effort. Do not leave other repos broken or out of sync.

## Dependencies

Codegen has no Cargo crate dependencies on other ShrouDB repos, but it reads `protocol.toml` specs from nearly every repo and generates SDK clients (Python, TypeScript, Go, Ruby, Protobuf) from them. Changes flow in both directions.

- **Spec inputs (upstream):** shroudb, shroudb-auth, shroudb-transit, shroudb-veil, shroudb-sentry, shroudb-mint, shroudb-keep, shroudb-courier, shroudb-pulse, shroudb-moat — each provides a `protocol.toml` that codegen reads.
- **Generated clients (downstream):** Changes to codegen templates, naming conventions, or output structure break all generated SDKs. After any codegen change, regenerate and verify clients for all affected specs.
- **Reverse direction:** When a repo changes its `protocol.toml` (adds/removes/renames commands, fields, or error codes), the generated clients must be regenerated to stay in sync.
