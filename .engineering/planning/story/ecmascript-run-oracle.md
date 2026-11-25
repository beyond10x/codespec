---
format: aep.planning-md/3
id: story:ecmascript-run-oracle
kind: story
status: draft
title: An ECMAScript runner that says what a script prints
summary: codespec-run-ecmascript runs a reader corpus case under a pinned node in a container through a generated driver and prints the kit's encoding.
relations:
- decomposes: epic:ecmascript-to-native
- depends_on: story:native-run-harness
- depends_on: story:ecmascript-reader
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: Cargo.toml
- confidence: inferred
  path: adapters/ecmascript-run
- confidence: inferred
  path: conformance/reader/ecmascript
- confidence: inferred
  path: crates/cli
- confidence: inferred
  path: scripts/check-parts.txt
revision: 2
---
# Story: An ECMAScript runner that says what a script prints

## Outcome

`codespec-run-ecmascript`, a Rust crate `adapters/ecmascript-run`, runs a corpus case of the
ECMAScript reader (`case.js` and `calls.json`) under a pinned `node` in a container
(`story:native-run-harness`) through a generated driver and prints the kit's encoding, one line per
call, so `codespec conform` can compare it with the case's `expected.txt`.

## Why

A reader conformance case is only worth something if its source means what the Go reference means.
The C reader does this with `codespec-run-c` (the compiler); JavaScript needs an engine. The oracle
is the reference for the meaning of a script, so the reader's cases and the speed comparison's
inputs are checked against a real engine, not against the reader's own reading.

## Done When

- A failing test first: `oracle_prints_expected_for_fib` runs the case `case.js` of a recursive `fib`
  and compares with `expected.txt`; it fails until the driver prints the encoding.
- The driver is generated per case (as `codespec-run-c` generates `driver.c`), written to the work
  directory and never committed: it calls each function of `calls.json` with the typed arguments
  (`check_argument` of `codespec_conformance`), prints `ok <v>` per result with floats in the
  encoding of `story:ecmascript-number-semantics` (the hex bits, NaN canonical), and a thrown
  exception as `err <message>`. The driver runs in `node` of the pinned image, which the
  harness runs with no network.
- A case whose `case.js` prints a different result from `expected.txt` is a named failure with
  both lines; a test holds it with a case that differs by one float bit (`0.1 + 0.2`).
- Every case of `conformance/reader/ecmascript` passes the oracle; the oracle's run is one
  container for all cases of a run, not one per case (the budget rule).
- The engine of the oracle is `node` only. A second engine (`qjs`) is not a second oracle here: a
  difference between engines on a case is a finding of that case, filed, not averaged.
- Floats and `int` 32 results are checked: a case returning `-0`, `NaN`, `2147483647 + 1 | 0` and
  `4294967295 >>> 0` is in the corpus and passes.
- `--describe` prints a manifest `task adapters` reads strictly. `task check` exits 0, each test
  under 60 s and the step under 120 s.

## Scope

- Own: `adapters/ecmascript-run` (new), `conformance/reader/ecmascript` (the cases it runs, shared
  with `story:ecmascript-reader`), `crates/cli` (`conform` role registration), root `Cargo.toml`
  `members`, `Cargo.lock`, `scripts/check-parts.txt`.
- Shares `conformance/reader/ecmascript`, `crates/cli` and `Cargo.toml` with
  `story:ecmascript-reader`: after it, or in its last step.
