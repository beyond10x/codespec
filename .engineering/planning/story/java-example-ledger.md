---
format: aep.planning-md/3
id: story:java-example-ledger
kind: story
status: active
title: A Java example under the harness
relations:
- decomposes: epic:java-reader
- depends_on: story:java-reader-javac
scope:
- confidence: cited
  path: adapters/go/cmd/codespec-write-go
- confidence: cited
  path: adapters/rust/src
- confidence: cited
  path: crates/harness/src/lib.rs
- confidence: cited
  path: crates/harness/tests/harness.rs
- confidence: cited
  path: examples/ledger
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T14:54:26Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-06T14:54:26Z", actor: "human:timo", revision: 4}
---
# Story: A Java example under the harness

## Outcome

`examples/ledger` (Java source, documents, Go and Rust outputs, drivers, `harness.toml`) runs the
original Java, the Go output and the Rust output byte-equal on 200 traces of 300 commands.

## Acceptance

- `task examples-ledger` reads `examples/ledger/source` twice byte-equal with 0 refusals, passes,
  writes Go and Rust with 0 refusals, compares every output with its checked-in copy, builds both
  and runs `codespec harness examples/ledger --seeds 200 --commands 300`, which prints
  `3 implementations (original, go, rust)` and `byte-equal`; it is in `task check`.
- The harness trace `ledger` (in `crates/harness`) pushes `int` and `long` past their maximum and
  minimum and reaches each error path the source declares: unknown account, duplicate account,
  non-positive amount, insufficient funds. A test over seeds 0–199 counts each error at least 50
  times, and a test pins seed 0.
- The Go and Rust writers render what the Java reader emits for the ledger: `sorted_keys` in the order cir/1 defines (integers numerically, strings by UTF-16 code unit), `conv` to `int32` (Rust handles `conv` only to a declared newtype today), and a field and a method with the same name on one type (Java getters such as `id()` beside field `id`, which Go rejects), each with a writer test.
- A Java driver (`drivers/java/`, built with `javac`) runs the original.

## Scope

- `examples/ledger/` (new).
- `adapters/go/cmd/codespec-write-go/`, `adapters/rust/src/`: the three writer forms above.
- `crates/harness/src/lib.rs`, `crates/harness/tests/harness.rs`: the `ledger` trace.
- `Taskfile.yml`, `CHANGELOG.md`: the coordinator's.
