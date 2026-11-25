---
format: aep.planning-md/3
id: story:wasm-writer
kind: story
status: draft
title: 'A WebAssembly writer: cir/1 to WAT with WasmGC, run by wasmtime and node'
summary: codespec-write-wasm writes WAT with the GC proposal, codespec-run-wasm assembles it with wasm-tools and runs it on wasmtime and node in a container.
relations:
- decomposes: epic:ecmascript-to-native
- depends_on: story:native-run-harness
- depends_on: story:ecmascript-reader
- depends_on: story:ecmascript-number-semantics
- depends_on: story:ecmascript-run-oracle
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: Cargo.toml
- confidence: inferred
  path: Taskfile.yml
- confidence: inferred
  path: adapters/wasm
- confidence: inferred
  path: adapters/wasm-run
- confidence: inferred
  path: containers/wasm
- confidence: inferred
  path: crates/cli
- confidence: inferred
  path: crates/conformance
- confidence: inferred
  path: crates/docs
- confidence: inferred
  path: scripts/check-parts.txt
revision: 4
---
# Story: A WebAssembly writer: cir/1 to WAT with WasmGC, run by wasmtime and node

## Outcome

`codespec-write-wasm`, a Rust crate `adapters/wasm`, writes a `cir/1` module as WebAssembly text
(WAT) that uses the GC proposal, and `codespec-run-wasm`, `adapters/wasm-run`, assembles it with
`wasm-tools parse`, validates it, and runs the calls of a case on `wasmtime` and on `node` (V8),
each in the container of `story:native-run-harness`, printing the kit's encoding. The first
cut writes what the ECMAScript reader produces and what the kit's numeric, control-flow and
struct cases use, and refuses the rest at a position with a reason.

## Why

Ordered first by the owner (2026-11-25, 13:19 CET): a Wasm writer before native assembly. WasmGC
gives structs, arrays and function references without an allocator or a collector of our own, and two
independent engines (wasmtime, V8) run the same file, so a difference between them is a finding. The
x86-64 writer (`story:x86-64-writer`) then needs its own allocator; this one does not.

## The mapping (inferred, decision D3: not proven until the tests below pass)

| cir/1 | WasmGC |
|---|---|
| `int` 8, 16, 32 / 64 / platform | `i32` (wrapped by masking or sign extension after each operation, `sem.int.wrap`) / `i64` / `i64` |
| `float` 32, 64; `bool` | `f32`, `f64`; `i32` |
| `string` | `(array (mut i8))` of UTF-8 bytes; literals in data segments |
| `struct` | `(struct (field …))`; a `ptr` to a struct is the reference to it |
| `slice` | `(struct (field $arr (ref (array T))) (field $off i32) (field $len i32) (field $cap i32))`; `append` grows by copy |
| `array {len, elem}` | `(array T)` of fixed length |
| `func` value, `lambda` | `(struct (field $fn funcref) (field $env (ref null struct)))`, call by `call_ref`; captured variables live in a struct, assigned captures in boxes |
| `&local`, `&v[i]`, `&s.f` | refused in the first cut; boxing, or a `{base, index}` pair, is a named later step. The ECMAScript reader produces none |
| `map` | refused in the first cut. A small runtime in WAT, `sem.map.order` (insertion order, as the evaluator iterates) reproduced, is the next step |
| `error`, `any`, interfaces, channels, `raise`, `foreign` | refused, each with its reason |

Whether a struct field needs a `mut`, how a slice of slices nests, and `i31ref` for small values are
decided by the tests, not here.

## Done When

- A failing test first: `writes_add_i32` writes a module with `func Add(a, b int32) int32` and the
  runner prints `ok 5` for `Add(2, 3)`; it fails until the writer exists.
- `wasm-tools parse` and `wasm-tools validate` (with the GC proposal) accept every output the
  writer produces for the cases it claims; a test writes the claimed kit cases and validates all in
  one container run (the budget rule: one run, not one per case).
- The runner runs each claimed case on `wasmtime` and on `node` and prints `expected.txt` exactly,
  both engines; where the two differ the case fails and names both lines. The versions are pinned
  in the image (`story:native-run-harness`), and the flags each needs for the GC proposal are named in the
  runner's crate doc; a node whose V8 needs a flag for WasmGC is not used.
- The writer's manifest claims: the integer, float (`story:ecmascript-number-semantics`), bool,
  string, struct, slice, array, `if`, `for`, `switch`, call, `lambda` and `call_value` features and
  `sem.int.wrap`, `sem.rem.sign`, `sem.switch.break`, `sem.continue.post`, `sem.multi_assign.order`,
  and the number features `sem.float.ieee`, `sem.float.to_int`, `sem.int.to_float` and
  `intrinsic.math`, mapped as: float operations to the `f32`/`f64` instructions, `math.Sqrt` to
  `f64.sqrt`, `math.Abs` to `f64.abs`, `math.Floor` to `f64.floor`, `math.Trunc` to `f64.trunc`,
  `math.Mod` to a runtime function in WAT (Wasm has no `fmod`), a float to `int` conversion in the
  defined range to `i64.trunc_f64_s`, and `int` to float to `f64.convert_i64_s` (and the `i32`
  forms); each has the kit case of that story passing on both engines, printed in its float
  encoding;
  every other kit case is a refusal with a reason, and the support matrix lists the writer with
  `claimed` and `refused_unclaimed` counts that `codespec conform` computes.
- Checked against the oracle: for every module of `examples/series` and `examples/inventory`
  that the writer claims, and for every case of the ECMAScript reader's corpus, the evaluator
  (`codespec eval --calls`) and both Wasm engines print the same bytes on the same
  `calls.json`. No `codespec harness` driver for Wasm exists under `examples/*/drivers` and this
  story adds none: the harness is not claimed. The refused ones are listed in the report.
- Near misses held by tests: an `int` 8 that overflows (`127 + 1`), a `uint` 32 division, a negative
  `%`, a shift by 32 or more, `-0.0` written and printed, a slice whose `append` reallocates while
  another slice still points at the old array, a struct value assigned to a second variable and
  then changed through one of them (a `cir/1` struct is a value, a GC struct a reference: the writer
  copies), a closure whose captured variable is assigned after
  the closure is made, a `for` whose `continue` must run the post statement, a refused module that
  writes no file.
- Stopping: the conformance run gives wasmtime a fuel limit (decision D4), so a loop that does not
  end stops with a named verdict like the evaluator's step budget; a test holds it with `for {}`.
  Nothing is emitted for a budget into the written code, so the artifacts swell's bench measures
  run the code and not a counter.
- The writer is deterministic (the same module gives the same bytes); its output is read back by
  `wasm-tools print` and the printed text parses to the same binary (a round trip held by a test).
  No reader of Wasm exists and none is planned here.
- `task check` exits 0; each test under 60 s, the step under 120 s; the tests that need the
  container run in the shared fixture. The time of the new test binary and of the conformance step
  is stated before and after.

## Scope

- Own: `adapters/wasm` (new), `adapters/wasm-run` (new), `containers/wasm` (new: the image with
  `wasm-tools`, `wasmtime` and `node`, built by the mechanism of `story:native-run-harness`),
  `crates/cli` (the `--lang` value),
  `crates/conformance` (the manifest and runner registration), `crates/docs` (support matrix),
  root `Cargo.toml` `members`, `Cargo.lock`, `Taskfile.yml`, `scripts/check-parts.txt`,
  `AGENTS.md`, `CHANGELOG.md`, one site page.
- Shares `crates/cli`, `crates/conformance`, `crates/docs` and `Taskfile.yml` with
  `story:x86-64-writer` (after this story), `story:ecmascript-run-oracle` (before this story, which
  depends on it for that reason and for the corpus it checks against) and the reader stories.
- Inferred by the planner: WasmGC as the target (given by the owner), the first-cut refusals, the
  engine-side fuel.
