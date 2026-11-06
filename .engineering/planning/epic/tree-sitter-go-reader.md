---
format: aep.planning-md/3
id: epic:tree-sitter-go-reader
kind: epic
status: archived
title: A tree-sitter Go reader, measured against the go/types reader
summary: A Go reader on tree-sitter with its own name resolution and type inference; it must produce byte-identical cir/1 to codespec-read-go on every example, or the gap is recorded by construct.
revision: 4
transitions:
- {from: "draft", to: "archived", at: "2026-11-06T01:44:49Z", actor: "human:timo", revision: 4}
---
# Epic: A tree-sitter Go reader, measured against the go/types reader

## Outcome

A Go reader on tree-sitter, with its own name resolution and type inference, run on every example
and compared byte for byte with `codespec-read-go`. The result is either byte-identical `cir/1` or
a table of the gap by construct. It decides how far route C reaches for languages with no typed
tooling.

## Decision (2026-11-06)

Route C (a shared tree-sitter layer) is for languages that have no typed tooling of their own
(JavaScript, Java, …); Rust uses typed tooling (`epic:rust-reader`). Tree-sitter gives syntax only.
Operator, 2026-11-06: "it's clear that we can't use tree sitter for the actual code content, the
body." So the expected outcome is: declarations and signatures read from tree-sitter, and function
bodies kept as `foreign` statements with a refusal wherever an expression's type cannot be derived.
This epic measures that on Go, where the `go/types` reader is the reference.

## Scope

- `adapters/tree-sitter` (Rust, `tree-sitter` + `tree-sitter-go`): `codespec-read-go-ts`.
- A comparison over every example: identical, or the differing constructs counted.
- The tree-sitter layer is written so that adding a grammar (Rust, Java, JavaScript, …) reuses the
  declaration extraction and only adds that language's node mapping.

## Out of Scope

- Readers for JavaScript or Java: they follow only if the measurement shows the route is worth it.

## Done When

The comparison runs in `task check` and its result is recorded in `docs/design.md`.
