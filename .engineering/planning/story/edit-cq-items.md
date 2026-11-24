---
format: aep.planning-md/3
id: story:edit-cq-items
kind: story
status: active
title: cq edit items, and the source_name relation
summary: edit <op>(args) :- body. in cq, rows per edit item, and a caller-given source_name relation
relations:
- decomposes: epic:code-edit-language
- depends_on: story:edit-spans-format
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
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T00:42:49Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-24T01:16:45Z", actor: "human:timo", revision: 7}
---
# Story: cq edit items, and the source_name relation

## Outcome

cq's grammar gains `edit = "edit" op "(" term ("," term)* ")" ":-" body "."`
(`docs/code-edit-language.md` § 3.1). The operation names and arities are a fixed table in the
crate (`rename/2`, `rename_field/3`, `retarget/2`, `delete/1` in wave 1); an unknown operation, a
wrong arity, an argument bound in no row, and a `?-` query beside an `edit` item are refused with a
position. A library call `edit_rows(program, fact base)` returns, per `edit` item in program order,
its rows sorted, each with its bound arguments and its certainty (`definite`, or `possible` with
the sites). `--explain` prints `edit` items. `codespec query` refuses a program holding an `edit`
item, naming it and `codespec edit`.

The fact base takes an optional list of `source_name(node, text)` records given by the caller (a
plain Rust input type of `crates/query`; this story does not read `codespec-spans/1`). The input
is additive: a new method `FactBase::build_with_names` beside `FactBase::build` and
`FactBase::build_with` (`crates/query/src/facts.rs:2383-2389`), and a new free function
`load_with_names` beside the free functions `load`, `load_with` and `load_in`
(`crates/query/src/lib.rs:303-322`). None of the existing five changes its signature, so no
existing caller or test changes. Without names the relation is empty; the cache digest covers the names when given.

## Done When

- `crates/query/tests/query.rs` (the existing target): the five wave-1 examples of
  `docs/code-edit-language.md` § 3.2 parse and give the expected rows over a fixture set; one test
  per refusal above; a possible row comes back marked possible with its site.
- Same target: `--explain` of a program with a rule and two `edit` items equals a checked-in text,
  each `edit` item printed under its stratum.
- `source_name` given for two nodes is queryable; the cache file of a fact base with names differs
  from one without, and one without names equals the file `load_with` writes today.
- `crates/cli/tests/query.rs`: `codespec query` on a program with an `edit` item exits 2 naming it.
- `docs/query-language.md` gains the grammar line and a section pointing to
  `docs/code-edit-language.md`; the crate doc of `crates/query` states the item. No test over 5 s.

## Critic findings answered

- `--explain` untested (acceptance): the checked-in explain case above.
- The `extent` relation serves only the unplanned assessor (scope): dropped.
- Caller-given records change the builder signatures used by nine test files (parallel safety):
  additive functions; existing signatures unchanged.
- Round 2, `load`, `load_with` and `load_in` named as `FactBase` methods: they are free functions of
  `crates/query/src/lib.rs`; the outcome says so.

## Scope

- Own: `crates/query/src/syntax.rs`, `crates/query/src/engine.rs`, `crates/query/src/facts.rs`,
  `crates/query/src/lib.rs`, `crates/query/tests/query.rs`, `crates/cli/src/query.rs`,
  `crates/cli/tests/query.rs`, `docs/query-language.md`.
- Runs after `story:edit-spans-format`, which adds the `mod edit;` line to
  `crates/cli/tests/query.rs`; that file is the only reason for the order.
