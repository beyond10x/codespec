---
format: aep.planning-md/3
id: epic:rust-reader
kind: epic
status: draft
title: A Rust reader and Rust to Go
summary: A reader for Rust into cir/1, unions for enums with data, and a Rust example translated to Go with the harness green.
revision: 2
---
# Epic: A Rust reader and Rust to Go

## Outcome

`codespec read --lang rust <crate>` lifts a Rust crate into `cir/1`, and a Rust example goes
Rust → `cir/1` → Go with the harness byte-equal on its traces.

## Decision (2026-11-06)

Each language is read with the tooling that knows its types; `cir/1` needs a type on every
expression, and syntax trees alone (tree-sitter, `syn`) do not carry them.

| option | types | decided |
|---|---|---|
| A: rust-analyzer's library crates (`ra_ap_*`): name resolution and inferred types | yes | **Rust uses this**, or whatever typed Rust tooling is most efficient when the epic starts |
| B: `syn` plus our own name resolution and inference | only what we write | no |
| C: a shared tree-sitter layer | syntax only | for languages with no typed tooling (JavaScript, Java, …); see `epic:tree-sitter-go-reader` |

Operator, 2026-11-06: "it's clear that we can't use tree sitter for the actual code content, the
body. … A for Rust, C for languages with no such tooling … use whatever is most efficient".

## Scope

- `adapters/rust`: a `codespec-read-rust` binary on the chosen typed tooling.
- `cir/1` `union` read from Rust enums with data; `Result`/`Option` read back to the Go-shaped
  forms the passes expect, or new facts where they do not fit.
- A Rust example with a driver in the harness.

## Out of Scope

- Readers for other languages (`epic:tree-sitter-go-reader` measures the tree-sitter route first).

## Done When

The harness is byte-equal on a Rust example read by `codespec-read-rust` and written as Go.
