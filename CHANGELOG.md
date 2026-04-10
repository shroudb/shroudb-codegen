# Changelog

All notable changes to ShrouDB Codegen are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

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

