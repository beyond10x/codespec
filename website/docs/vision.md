---
title: Vision
sidebar_position: 2
description: Why codespec exists, the rules it keeps, what it deliberately does not use, and its milestones.
lede: Code becomes typed data with one definition, and every translation carries the run that shows it behaves like the original.
source: docs/design.md, crates/model/src/lib.rs, the planning store's epics
---

# Vision

## Why

A system specification describes a system from the outside: types, entities, lifecycles,
commands, events, views. What a function body computes is left to the implementation. Translating
code between languages, or rewriting it within one, needs a representation of that inside.
`cir/1` is that representation: one typed, canonical document per module, with a single
definition in Rust.

Its uses:

- Round trip with rewrites: Go → `cir/1` → Go, with normalisation or idiom changes between.
- Translation: Go → `cir/1` → Rust, and Rust → `cir/1` → Go with the Rust reader.
- One analysis for many languages: sharing, fallibility, purity and iteration order, computed on
  `cir/1` once and read by every writer.

## The pipeline

```text
source ──reader──▶ cir/1 ──passes──▶ cir/1 ──writer──▶ source
```

| Step | Contract |
|---|---|
| reader | source language → `cir/1`; a program that prints one module on stdout |
| pass | `cir/1` → `cir/1`; deterministic, total, named |
| facts | whole-module results written by passes and read by writers, so writers decide nothing |
| writer | `cir/1` → source language; a program that reads one module on stdin |
| refusal | a construct a reader or writer does not handle, with its position and reason |
| harness | runs every implementation of one program on seeded traces and compares outputs byte for byte |

The details are on [Readers, passes and writers](/docs/concepts/pipeline) and
[The harness](/docs/concepts/harness).

## Rules

| rule | what it means |
|---|---|
| deterministic, no model | No step is a language model or a heuristic. The same source read twice gives the same bytes, and a pass run on its own output gives the same bytes. |
| refuse what cannot be expressed | What `cir/1` cannot express becomes a `foreign` statement holding the source text and a reason, plus a refusal. What a writer cannot write is left out and listed. |
| one definition | The Rust model is the format. The schema, every adapter and every document follow it. A change in meaning is a new format version, `cir/2`, not an edit to `cir/1`. |
| evidence by running | A translation is checked by running the original and the output on the same inputs. |

## Measured adapters

Each adapter is written in the language whose tooling handles its target best:

| adapter | tooling |
|---|---|
| Go reader | `go/types` |
| Go writer | builds `go/ast` and prints it with `go/printer` |
| Rust writer | builds `syn` trees and prints them with `prettyplease` |

The Rust reader parses with `syn`, which the Rust
writer already depends on, and resolves names and types itself for the subset of Rust that `cir/1`
covers.

Two questions decide each adapter, and the harness answers both:

1. Does the reader capture enough detail to produce `cir/1`?
2. Does the writer produce code that behaves like the original?

## Milestones

| | Content | Done when | Status |
|---|---|---|---|
| M0 | model, schema, CLI (`validate`, `schema`, `read`, `write`), Go reader, `orders` | `task check` green; `orders` read twice gives the same bytes | shipped in 0.1.0 |
| M1 | passes, Go writer, Rust writer, harness | `orders`: original Go, Go output and Rust output byte-equal on 200 × 300 traces | shipped in 0.1.0 |
| M2 | a tree-sitter Go reader | — | dropped ([Not used](#not-used)) |
| M3 | a second, algorithmic example: loops, `switch`, slices, arithmetic | the M1 criteria on both examples | done in 0.2.0 (`series`) |
| M4 | Rust reader on `syn`; Rust → `cir/1` → Go | harness green on a Rust example | shipped in 0.1.0 (`inventory`) |

Release 0.1.0 is M0, M1 and M4: Go reader, Rust reader, Go writer, Rust writer, passes, harness.

## Not used

Decided on 2026-11-06.

- tree-sitter: considered as the reader for languages without typed tooling, and not used. It
  gives syntax only, and `cir/1` needs a type on every expression. A tree-sitter reader would have
  to bring its own type inference for each language, and codespec keeps its dependencies minimal.
  No tree-sitter-based tool infers types:
  - Semgrep's generic AST covers 31 languages but stays close to the source text.
  - GitHub's stack-graphs resolves names but not types, and was archived on 2025-09-09.
  - tree-sitter tags queries only label definitions and references.
- rust-analyzer's `ra_ap_*` crates: not used for the Rust reader, for the same dependency
  reason. The reader resolves names and types itself, on `syn`, for the `cir/1` subset.

## Later

- [ESS](https://beyond10x.github.io/ess/) ([GitHub](https://github.com/beyond10x/ess)): a
  specification could supply a service's skeleton, and `cir/1` bodies fill what it leaves to the
  implementation. The ESS conformance suite would be an equivalence oracle beside the harness.
  This item is a direction, not a plan: codespec does not use ESS today.
- More target languages through the same adapter boundary.
