---
format: aep.planning-md/3
id: epic:rust-reader
kind: epic
status: active
title: A Rust reader and Rust to Go
summary: A reader for Rust into cir/1, unions for enums with data, and a Rust example translated to Go with the harness green.
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T01:44:49Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-06T01:44:49Z", actor: "human:timo", revision: 4}
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
| A: rust-analyzer's library crates (`ra_ap_*`): name resolution and inferred types | yes | no: a large, fast-moving dependency tree |
| B: `syn` plus our own name resolution and inference | only what we write | **Rust uses this** (`story:rust-reader-on-syn`) |
| C: a shared tree-sitter layer | syntax only | no: dropped, see `docs/design.md` § Not used |

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

Revised the same day: "So to me, it sounds like it's not really feasible to do tree sitter. Also, I'm really not a fan of dependencies in general. So let's maybe forget about tree sitter entirely." Rust is read with `syn`, already a dependency of the Rust writer.
