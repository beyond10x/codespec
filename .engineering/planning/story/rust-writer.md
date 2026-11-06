---
format: aep.planning-md/3
id: story:rust-writer
kind: story
status: proposed
title: Rust writer
summary: codespec-write-rust emits a building Rust crate from cir/1 and its facts; a function holding foreign code is listed as a refusal
relations:
- decomposes: epic:go-and-rust-writers-with-harness
- depends_on: story:passes-and-facts
scope:
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: adapters/rust
- confidence: cited
  path: examples/orders/out/rust
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T00:09:10Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":4}}}
---
# Story: Rust writer

## Outcome

`codespec write --lang rust examples/orders/orders.cir.json --out <dir>` produces a Rust crate that
builds with `cargo build`, using the facts (`story:passes-and-facts`) for ownership
(`Rc<RefCell<T>>`), `Result`, `Option`, `&self`, `Display` and `enum Message` / `enum Error`.

A function whose body holds a `foreign` statement in another language is left out of the crate and
listed in `<dir>/refusals.json` with its `pos`; the writer exits 0. On `orders` that is `Audit`
(`examples/orders/source/orders.go:132`), and nothing else.

## Acceptance

- Task `examples-rust` writes `examples/orders/out/rust/`, builds it, and `cmp`s it with the
  checked-in copy.
- `examples/orders/out/rust/refusals.json` lists exactly one entry, `Audit`, at `orders.go:132`.
- A module without `facts` is refused, exit 1, naming the missing pass.

## Scope

- new crate `adapters/rust` (`codespec-write-rust`, `syn` + `prettyplease`); `Cargo.toml` workspace
  members.
- `examples/orders/out/rust/`.
- `Taskfile.yml`: new task `examples-rust`, the `adapters` task builds `codespec-write-rust`, and one
  entry in `check`'s list.

## Depends on

`story:passes-and-facts`. Its entry in `check`'s list conflicts by one line with
`story:go-writer`'s when both are open at once; resolved at merge.
