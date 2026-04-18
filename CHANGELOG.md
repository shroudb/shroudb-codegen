# Changelog

All notable changes to ShrouDB Codegen are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## Unreleased

## [v0.2.0] - 2026-04-17

### Added

- Scroll engine support end-to-end. `.github/workflows/ci.yml` and
  `.github/workflows/generate.yml` now clone `shroudb-scroll` so the Moat
  composite spec can resolve the new `[[engines]].name = "scroll"` entry.
  Sandbox gains `test-sandbox/test-scroll-config.toml` plus four integration
  tests (`test_scroll_python.py`, `_typescript.ts`, `_go.go`, `_ruby.rb`)
  covering hello → append → read → create_group → read_group → ack →
  log_info → group_info → delete_group → delete_log. `run-tests.sh` adds
  `scroll` to the engine list and wires `{{CIPHER_PORT}}` substitution so
  scroll-server can reach the sandbox's remote Cipher.

### Fixed

- Emit required keyword-prefixed params on the wire in all 4 SDKs. Required
  params with a `KEYWORD <value>` form in `syntax` (e.g., Forge's
  `CA CREATE <name> <algorithm> SUBJECT <subject>`) were previously serialized
  as bare positionals, which the engine rejects with `requires SUBJECT <subject>`.
  Codegen now parses the syntax string for keyword prefixes and sets `wire_key`
  on positional params so each generator prepends the keyword at emit time.
  Public SDK method signatures are unchanged.
- Emit a correct `PIPELINE` method across all 4 SDKs (TypeScript, Python, Ruby,
  Go). Previously, the generator naively read PIPELINE's declared `count`
  positional parameter from `shroudb/protocol.toml` and emitted a
  `pipeline(count)` helper that only serialized the count prefix with no way
  to feed sub-commands — making the generated method unusable. The emitter now
  special-cases commands with `verb = "PIPELINE"` and generates
  `pipeline(commands, requestId?)` that delegates to a new transport method.
  Transport interfaces gain `executePipeline` (RESP3 transports implement the
  nested-array frame with optional `REQUEST_ID` keyword for idempotent retry;
  HTTP transports raise a not-supported error since PIPELINE is RESP3-only).
  Matches the hand-written Rust client in `shroudb-client/src/lib.rs`. READMEs
  also updated so the per-method parameter list reflects the real signature.

## [v0.1.0] - 2026-03-26

### Other

- Add security posture requirements to CLAUDE.md
- Add package publishing to SDK generation workflow
- Add SDK generation workflow for automated codegen pipeline
- Switch CI to reusable workflow on self-hosted runners
- Fix 19 failing integration tests to match generated client APIs
- Update README: add Moat composite spec and unified SDK section
- Add integration test support for Mint, Sentry, Keep, Courier, Pulse
- Add Moat composite spec codegen for unified SDK generation
- Add README.md
- Complete test coverage: all 3 specs × 4 languages = 12 suites, all passing
- Expand test sandbox to cover all 3 specs: shroudb, transit, and auth
- Fix all codegen response parsing bugs, all 4 languages pass 22/22
- Remove accidentally committed server data, add to gitignore
- Fix sandbox test issues, document remaining codegen bugs
- Fix Python None-value int parsing and TypeScript ESM module type
- Fix codegen bugs found by client sandbox testing
- Fix test scripts to use correct spec field names
- Fix URI scheme doubling in all wire generators
- Fix run-tests.sh for macOS bash 3.2 (no associative arrays)
- Add Docker fallback for test sandbox server startup
- Fix cargo fmt formatting in wire generators
- Add client test sandbox for validating generated SDKs
- Remove dead code: fix reverted type mappings, replace custom to_pascal with heck
- Wire generators: implement streaming SUBSCRIBE for Go, TypeScript, Ruby
- Wire generators: implement real streaming SUBSCRIBE support for all languages
- HTTP generators: complete language-specific fixes with re-applied cross-cutting changes
- HTTP generators: language-specific fixes across Go, Python, TypeScript, Ruby, Proto
- HTTP generators: generic path params, auth validation, success status checks
- Wire proto: add error codes enum, package options, conditional imports, RPC table
- Wire generators: fix hardcoded values, add safety limits, spec-driven READMEs
- Wire generators: consistent streaming command stubs across all languages
- Wire generators: use spec-defined type mappings instead of hardcoded matches
- Fix broken code: remove crashing Python subscribe stub, harden Ruby key unwrap
- Fix core infrastructure: add GenerateResult type alias, proper kebab-casing, clippy fixes
- Add .gitignore, remove target/ from tracking
- Initial commit: unified SDK codegen for all ShrouDB protocols

