---
format: aep.planning-md/3
id: epic:ecmascript-to-native
kind: epic
status: draft
title: ECMAScript to native code through cir/1, run in a container, compared in speed
summary: A typed ECMAScript subset read into cir/1, written as WebAssembly and x86-64 assembly, run in containers on the build node, and compared in speed with the fastest ECMAScript engines.
relations:
- informed_by: epic:low-level-readers
revision: 2
---
# Epic: ECMAScript to native code through cir/1, run in a container, compared in speed

## Outcome

A typed subset of ECMAScript goes the whole way and the result is measured:

```
file.js --codespec-read-ecmascript--> cir/1 --codespec-write-wasm--> WAT --wasm-tools--> .wasm
                                        |                                              (wasmtime, node)
                                        +--codespec-write-x86-64--> GNU as --as, ld--> ELF
                                                                       (each run in a container)
```

- `codespec read --lang ecmascript <file.js>` prints one `cir/1` module for a script in a typed,
  refusal-first subset, on the parser of swell (`beyond10x/swell`, crate `swell-syntax`, pinned to a
  release tag). A construct that needs a JavaScript runtime is refused with a reason and
  `file:line`.
- `codespec write --lang wasm` writes that module as WebAssembly text (WAT, WasmGC), and
  `codespec write --lang x86-64` writes it as x86-64 GNU assembly (System V ABI, Linux). Each is
  assembled and run in a container on the build node, through `slot`, and its output is compared
  byte for byte with the evaluator's on the same inputs.
- `codespec-bench` builds a runnable artifact per benchmark script and writes its ahead-of-time
  rows in swell's bench schema; swell's `bench/` measures them beside the fastest ECMAScript
  engines on the build node and holds the data file and the comparison page. codespec measures
  nothing for the comparison.

## Why Now

Requested by the owner on 2026-11-25: a path from JavaScript through `cir/1` to assembly for one
target, run in a container (a microVM later), with its speed compared with the fastest ECMAScript
engines. Agreed with the maintainers of swell (option A): the reader is codespec's, built on swell's
`syntax` crate; swell promises a versioned syntax surface with spans; the writers are codespec's.
Order set by the owner at 13:19 CET the same day: the WebAssembly writer first, x86-64 second.

`cir/1` is statically typed and Go-shaped. The path is a typed subset, not a JavaScript engine: no
tagged value, no property lookup, no coercion, no `try`. That is the point of the measurement: how
close does typed code written as JavaScript get to a JIT, when nothing is left to the run time.

## What exists and is not duplicated

- `epic:low-level-readers` (`story:reader-asm`, `story:reader-llvm-ir`, both draft) reads
  assembly and LLVM IR into `cir/1`. This epic writes assembly. `story:x86-64-writer` reads its own
  output back with `story:reader-asm` once that exists, and does not read anything itself.
- The Go writer is the only writer with floats, function values and interfaces
  (`website/data/support-matrix.json`); the C and Rust writers refuse floats and lambdas, and the C
  runtime never frees. The evaluator (`crates/eval`) is the oracle of every run here.
- `adapters/rust-read` (on `syn`) and `adapters/c-read` are the models for the reader's layout,
  conformance corpus and runner.

## Decisions taken in this plan (reversible; each is a story's first step)

- D1. Types come from JSDoc tags on entry functions plus inference inside the file (not TypeScript,
  not inference alone). `story:ecmascript-reader`.
- D2. Every `number` is `float` 64; `int` 32 only where a `|0` or `>>>0` form fixes it.
  `story:ecmascript-reader`, `story:ecmascript-number-semantics`.
- D3. The WasmGC mapping of `story:wasm-writer` is inferred, not proven, until that story's tests
  pass on both engines.
- D4. No step budget is emitted into written code. The conformance run takes one from the engine
  (wasmtime fuel); the artifacts swell's bench measures have none and end at the bench's own time limit.

## Done When

- `story:ecmascript-number-semantics`, `story:ecmascript-swell-surface`, `story:ecmascript-reader`,
  `story:native-run-harness`, `story:ecmascript-run-oracle`, `story:wasm-writer`,
  `story:x86-64-writer` and `story:native-speed-comparison` are implemented.
- One JavaScript benchmark file `k.js` goes from source to a result line on the build node with
  four commands, each named by a story: `codespec read --lang ecmascript k.js` (the module),
  `codespec write --lang wasm` on it (the WAT), `codespec-bench build k.js --target wasm --out d`
  (assembly with `wasm-tools`, the `.wasm` and `artifact.json`) and `codespec-bench run
  d/artifact.json` (one run in the container, the line `codespec-bench-result/1 <name> <target>
  exit=<n> out=<sha256>` and `match` or `differs` against `node`). A script the reader refuses
  gives a row with the reason and `file:line` instead of an artifact.
- Every gate step of the stories ends within 2 minutes, no test over 60 s. Benchmarks are not tests
  and never run in the gate.

## Out of Scope

- Full ECMAScript, or a JavaScript runtime in the written code: no `eval`, `with`, `Function`,
  prototype mutation, getters, setters, proxies, `try`, generators, `async`, modules, classes.
- Any change to `cir/1` beyond the additive float semantics of `story:ecmascript-number-semantics`;
  no new node (`story:cir1-type-test` stays its own story).
- A garbage collector for x86-64, other architectures (AArch64) and other operating systems.
- A microVM step (Firecracker needs `/dev/kvm`, which the build node lacks); `story:native-run-harness`
  names where it would go.
- An LLVM IR writer, the other route to native code.
- Work inside swell: its syntax surface, its release and its corpus, and its `bench/` (the pinned
  engines, the method, the measuring, the data file and the comparison page) are swell's stories;
  this epic names what it needs from them (`story:ecmascript-swell-surface`,
  `story:native-speed-comparison`).
