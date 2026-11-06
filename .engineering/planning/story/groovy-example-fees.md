---
format: aep.planning-md/3
id: story:groovy-example-fees
kind: story
status: implemented
title: A Grails service example under the harness
relations:
- decomposes: epic:groovy-grails-reader
- depends_on: story:grails-artefacts
- depends_on: story:measure-report
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: adapters/go/cmd/codespec-write-go/write.go
- confidence: inferred
  path: adapters/go/cmd/codespec-write-go/write_test.go
- confidence: cited
  path: adapters/rust/src/write.rs
- confidence: inferred
  path: adapters/rust/tests/writer.rs
- confidence: cited
  path: crates/harness/src/lib.rs
- confidence: cited
  path: crates/harness/tests/harness.rs
- confidence: inferred
  path: examples/fees/drivers/go/go.mod
- confidence: inferred
  path: examples/fees/drivers/go/main.go
- confidence: inferred
  path: examples/fees/drivers/go/overlay-go.json
- confidence: inferred
  path: examples/fees/drivers/groovy/driver/Main.groovy
- confidence: inferred
  path: examples/fees/drivers/rust/Cargo.lock
- confidence: inferred
  path: examples/fees/drivers/rust/Cargo.toml
- confidence: inferred
  path: examples/fees/drivers/rust/src/main.rs
- confidence: inferred
  path: examples/fees/fees.cir.json
- confidence: inferred
  path: examples/fees/fees.read.cir.json
- confidence: inferred
  path: examples/fees/harness.toml
- confidence: inferred
  path: examples/fees/out/go/fees.go
- confidence: inferred
  path: examples/fees/out/rust/Cargo.toml
- confidence: inferred
  path: examples/fees/out/rust/refusals.json
- confidence: inferred
  path: examples/fees/out/rust/src/lib.rs
- confidence: inferred
  path: examples/fees/source/application.properties
- confidence: inferred
  path: examples/fees/source/grails-app/domain/fees/Rate.groovy
- confidence: inferred
  path: examples/fees/source/grails-app/services/fees/LateFeeService.groovy
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T17:43:51Z", actor: "human:timo", revision: 9}
- {from: "proposed", to: "active", at: "2026-11-06T17:43:51Z", actor: "human:timo", revision: 10}
- {from: "active", to: "implemented", at: "2026-11-06T21:54:55Z", actor: "human:timo", revision: 11, decided_on: {"recorded":{"test_result":1}}}
---
# Story: A Grails service example under the harness

## Outcome

`examples/fees` (a Grails 2.5.6 layout with one service and one domain class, its documents, Go and
Rust outputs, drivers, `harness.toml`) runs the original Groovy, the Go output and the Rust output
byte-equal on 200 traces of 300 commands, so a Groovy service read by the Grails path is shown to
translate, not only to read.

## Context

- Feasible: Groovy 2.4.10 output runs on JDK 21 here (probe 2026-11-06), so the original runs under
  the harness like the Java `ledger` does, compiled by `groovyc` and run with `java` with the Groovy
  jar on the classpath. No Grails runtime is needed, because the example holds no GORM call and no
  injected dependency.
- Source, written from scratch, neutral (no organisation, customer or product name):
  `examples/fees/source/application.properties`, `grails-app/domain/fees/Rate.groovy` (every
  property initialised, no metadata blocks, so it reads as a struct with 0 refusals and needs no
  nil pointer) and `grails-app/services/fees/LateFeeService.groovy`: per-day rates in cents
  (`long`), a grace period (`int`), a `TreeMap` of item kinds to rates, a cap, a waiver count,
  `switch` over the enum `ItemKind`, a `while` with `break`. The enum is declared in
  `LateFeeService.groovy`, because `src/groovy` is resolve-only (`story:grails-artefacts`).
- No error propagation: results are values and no method throws (the core reader keeps `throw` as
  `unsupported`), so no fallible function calls another. The example does not need
  `story:rust-writer-error-propagation` and carries no edge to it; nor does it compare a pointer
  with nil, so it does not need `story:rust-writer-nil-pointer`.
- Every implementation is reached through the same trace: a new `fees` trace in `crates/harness`
  whose generator pushes `long` and `int` past their extremes and reaches each branch.
- The harness substitutes only `{build}` in commands (`crates/harness/src/lib.rs:16`, `:550`), so
  `harness.toml` calls `groovyc` and `groovy` from `PATH`, as it calls `javac`, `go` and `cargo`;
  CI puts `$GROOVY_HOME/bin` on `PATH` (the Groovy install of `story:grails-lending-fixture`). No
  harness change beyond the trace.

## Domain relations

- None beyond those `story:grails-artefacts` records; the example uses its mapping unchanged.

## Acceptance

One acceptance statement: `task examples-fees` reads `examples/fees/source` twice byte-equal with 0
refusals, passes, writes Go and Rust with 0 refusals equal to the checked-in copies, builds all three
and `codespec harness examples/fees --seeds 200 --commands 300` prints
`3 implementations (original, go, rust)` and `byte-equal`; the task is in `task check`.

- The `fees` trace: a test over seeds 0-199 counts each branch of `LateFeeService` (grace, capped,
  waived, each `ItemKind`) at least 50 times.
- A test pins seed 0: the first 5 lines of the `fees` trace for seed 0 (300 commands) are checked
  in as a string constant in `crates/harness/tests/harness.rs`, and the test fails when the
  generator's output differs from it.
- Any writer form the Groovy reader emits that the Go or Rust writer refuses today is added with a
  writer test each, and the other examples outputs stay unchanged.
- `codespec measure examples/fees/fees.read.cir.json` shows every function read.

## Scope

- `examples/fees/` (new), these files: `harness.toml`; `source/application.properties`,
  `source/grails-app/domain/fees/Rate.groovy`, `source/grails-app/services/fees/LateFeeService.groovy`;
  `fees.read.cir.json`, `fees.cir.json`; `out/go/fees.go`; `out/rust/Cargo.toml`,
  `out/rust/src/lib.rs`, `out/rust/refusals.json`; `drivers/groovy/driver/Main.groovy`;
  `drivers/go/go.mod`, `drivers/go/main.go`, `drivers/go/overlay-go.json`;
  `drivers/rust/Cargo.toml`, `drivers/rust/Cargo.lock`, `drivers/rust/src/main.rs`.
- `crates/harness/src/lib.rs`: the `fees` trace generator and its `trace = "fees"` kind.
- `crates/harness/tests/harness.rs`: the branch-count test and the seed-0 pin.
- Writer forms, only those the example needs and none of the forms below:
  `adapters/go/cmd/codespec-write-go/write.go` with a case in
  `adapters/go/cmd/codespec-write-go/write_test.go`; `adapters/rust/src/write.rs` with a case in
  `adapters/rust/tests/writer.rs`. Excluded, owned by other stories: the nil comparison and nil of
  a pointer type (`adapters/rust/src/write.rs:2586-2589`, `story:rust-writer-nil-pointer`), a
  fallible call inside a fallible function (`adapters/rust/src/write.rs:2807-2817`,
  `story:rust-writer-error-propagation`), and every `--describe` manifest
  (`adapters/rust/src/main.rs`, `adapters/go/cmd/codespec-write-go/main.go`,
  `story:runners-and-manifests`).
- `Taskfile.yml` `examples-fees` and `check`, `CHANGELOG.md`: the coordinator.
