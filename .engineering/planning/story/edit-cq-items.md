---
format: aep.planning-md/3
id: story:edit-cq-items
kind: story
status: proposed
title: cq edit items, and the source_name and extent relations
summary: edit <op>(args) :- body. in cq, rows per edit item, and two caller-given relations
relations:
- decomposes: epic:code-edit-language
scope:
- confidence: inferred
  path: crates/cli/src/query.rs
- confidence: inferred
  path: crates/cli/tests/query.rs
- confidence: inferred
  path: crates/query/src/engine.rs
- confidence: inferred
  path: crates/query/src/facts.rs
- confidence: inferred
  path: crates/query/src/lib.rs
- confidence: inferred
  path: crates/query/src/syntax.rs
- confidence: inferred
  path: crates/query/tests/query.rs
- confidence: inferred
  path: docs/query-language.md
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T00:42:49Z", actor: "human:timo", revision: 3}
---
# Story: cq edit items, and the source_name and extent relations

## Outcome

cq's grammar gains `edit = "edit" op "(" term ("," term)* ")" ":-" body "."`
(`docs/code-edit-language.md` § 3.1). The operation names and arities are a fixed table in the
crate (`rename/2`, `rename_field/3`, `retarget/2`, `delete/1` in wave 1); an unknown operation, a
wrong arity, an argument bound in no row, and a `?-` query beside an `edit` item are refused with a
position. A library call `edit_rows(program, fact base)` returns, per `edit` item in program order,
its rows sorted, each with its bound arguments and its certainty (`definite`, or `possible` with
the sites). `--explain` prints `edit` items. `codespec query` refuses a program holding an `edit`
item, naming it and `codespec edit`.

The fact base takes optional per-node records given by the caller: `source_name(node, text)` and
`extent(node, file, line, end_line)`, built by the caller from span documents (a plain Rust input
type of `crates/query`; this story does not read `codespec-spans/1`). Without them both relations
are empty, and the cache digest covers them when given.

## Done When

- `crates/query/tests/query.rs` (the existing target): the five wave-1 examples of
  `docs/code-edit-language.md` § 3.2 parse and give the expected rows over a fixture set; one test
  per refusal above; a possible row comes back marked possible with its site.
- `source_name` and `extent` given for two nodes are queryable; the cache file of a fact base with
  them differs from one without.
- `crates/cli/tests/query.rs`: `codespec query` on a program with an `edit` item exits 2 naming it.
- `docs/query-language.md` gains the grammar line and a section pointing to
  `docs/code-edit-language.md`; the crate doc of `crates/query` states the item. No test over 5 s.

## Scope

- Own: `crates/query/src/syntax.rs`, `crates/query/src/engine.rs`, `crates/query/src/facts.rs`,
  `crates/query/src/lib.rs`, `crates/query/tests/query.rs`, `crates/cli/src/query.rs`,
  `crates/cli/tests/query.rs`, `docs/query-language.md`.
- Independent of the span and engine stories; `story:edit-cli-verified-apply` uses it. Touches no
  file another wave-1 story owns (`crates/cli/src/query.rs` is not `lib.rs` or `main.rs`).
