# ShrouDB Codegen

## What ShrouDB Codegen Does

ShrouDB Codegen reads a TOML protocol specification and generates typed, publish-ready client SDKs in Python, TypeScript, Go, and Ruby. One spec file defines every command, response shape, and error code for a ShrouDB engine. Codegen turns that spec into complete client libraries -- connection management, typed methods, response models, error handling, and package metadata -- so SDK authors never hand-write boilerplate.

It supports three spec shapes:

- **Wire protocol specs** for engines like Cipher, Forge, Keep, Chronicle, Veil, Courier, and Sentry.
- **HTTP API specs** for engines like Sigil.
- **Composite specs** for Moat, which imports multiple engine specs and produces a single unified SDK with engine-namespaced methods.

## Why It Matters

Hand-maintained SDKs drift from the server. A field gets added on the server side, the Python client picks it up, the Go client doesn't, and users hit runtime errors that a type system should have caught. Codegen eliminates that class of bug: the spec is the single source of truth, and every generated client stays in sync with the protocol it targets.

For ShrouDB engine developers, this means adding a new command is a one-line spec change followed by a regeneration step -- no manual edits across four language repositories. For SDK consumers, it means every client library has the same coverage, the same method signatures, and the same error types, regardless of language.
