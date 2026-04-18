# ShrouDB Codegen

Generates a single unified SDK per language from the Moat composite spec, with engine-namespaced methods (`db.cipher.encrypt(...)`, `db.sigil.userCreate(...)`), dual RESP3/HTTP transport, and full documentation (README.md + AGENTS.md per SDK).

## Security posture

ShrouDB is security infrastructure. Every change must be evaluated through a security lens:

- **Fail closed, not open.** When in doubt, deny access, reject the request, or return an error. Never default to permissive behavior for convenience.
- **No plaintext at rest.** Secrets, keys, and sensitive data must be encrypted before touching disk. If a value could be sensitive, treat it as sensitive.
- **Minimize exposure windows.** Plaintext in memory must be zeroized after use. Connections holding decrypted data must be short-lived. Audit every code path where sensitive data is held in the clear.
- **Cryptographic choices are not negotiable.** Do not downgrade algorithms, skip integrity checks, weaken key derivation, or reduce key sizes to simplify implementation. If the secure path is harder, take the harder path.
- **Every shortcut is a vulnerability.** Skipping validation, hardcoding credentials, disabling TLS for testing, using `unsafe` without justification, suppressing security-relevant warnings — these are not acceptable trade-offs regardless of time pressure. The correct implementation is the only implementation.
- **Audit surface changes require scrutiny.** Any change that modifies authentication, authorization, key management, WAL encryption, or network-facing code must be reviewed with the assumption that an attacker will examine it.

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

Codegen has no Cargo crate dependencies on other ShrouDB repos, but it reads `protocol.toml` specs via the Moat composite spec and generates unified SDK clients (TypeScript, Python, Go, Ruby) from them.

- **Entry point:** `shroudb-moat/protocol.toml` — the composite spec that references every engine spec
- **Spec inputs (upstream):** shroudb, shroudb-cipher, shroudb-sigil, shroudb-veil, shroudb-sentry, shroudb-forge, shroudb-keep, shroudb-courier, shroudb-chronicle, shroudb-stash, shroudb-scroll — each provides a `protocol.toml` that Moat references
- **Generated SDKs (downstream):** One unified SDK per language with all engines namespaced. Changes to codegen templates or output structure affect all generated SDKs. After any codegen change, regenerate and verify.
- **Reverse direction:** When any engine changes its `protocol.toml`, regenerate all SDKs.
