---
format: aep.planning-md/3
id: story:native-speed-comparison
kind: story
status: draft
title: 'AOT rows for swell''s speed comparison: a runnable artifact per benchmark script'
summary: codespec-bench builds a .wasm artifact and a runner command per benchmark script from a pinned codespec release and writes the ahead-of-time rows in swell's bench schema; swell's bench measures.
relations:
- decomposes: epic:ecmascript-to-native
- depends_on: story:wasm-writer
- depends_on: story:ecmascript-run-oracle
scope:
- confidence: inferred
  path: .github/workflows/release-build.yml
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
  path: benchmarks/ecmascript-native/kernels
- confidence: inferred
  path: benchmarks/ecmascript-native/schema
- confidence: inferred
  path: crates/bench
- confidence: inferred
  path: scripts/check-parts.txt
revision: 7
---
# Story: AOT rows for swell's speed comparison: a runnable artifact per benchmark script

## Outcome

`codespec-bench`, a Rust crate `crates/bench`, turns a benchmark script into a runnable artifact and
into rows of swell's speed comparison, and measures nothing. swell's `bench/` measures every engine
(pinned engines in a container image, bench-v8 v7 and are-we-fast-yet, through `slot` on the build
node) and holds the data file and the comparison page; codespec supplies the ahead-of-time rows.

- `codespec-bench build <file.js> --target wasm --out <dir>` reads the script, writes it as
  WebAssembly, assembles it with `wasm-tools`, and writes `<name>.wasm` and `artifact.json`
  (`codespec-bench-artifact/1`): the codespec version, the sha256 of the source and of the `.wasm`,
  the entry function, the encoding of its arguments, the tool versions, and the runner command as an
  argument list (`wasmtime run --invoke <entry> <dir>/<name>.wasm <args>`, with the flags the pinned
  `wasmtime` needs for the GC proposal). A second target, `x86-64`, an ELF with its own runner
  command, is added by `story:x86-64-writer`, which depends on this story and owns that target in
  its scope and Done When; it is not part of this story's outcome.
- `codespec-bench rows <dir>...` writes the AOT rows in swell's bench schema `swell-bench/1`
  (swell's `story:bench-runner-and-data-file`, beyond10x/swell#9): one row per script and target,
  carrying the artifact, its runner command and its digests. A script the reader refuses is a row
  with `status: refused`, the refusal's reason and `file:line`; it is not rewritten to pass.
- `codespec-bench run <dir>/artifact.json` runs the artifact once, in the container of
  `story:native-run-harness`, and prints one result line
  `codespec-bench-result/1 <name> <target> exit=<n> out=<sha256 of stdout>`, then compares the output
  with what `story:ecmascript-run-oracle` gets from `node` for the same script: `match` or
  `differs`. It is a correctness run: it prints no time.

## Why

Requested by the owner with the path itself: compare speed with the fastest ECMAScript engines. The
division is agreed with swell (option a): one place holds the method, the pinned engines and the data
file, so a comparison has one method and codespec does not pin a second set of engines. codespec's
part is what only it can make: the artifact, and an honest row for what it cannot read.

The set is swell's. Most of bench-v8 v7 and are-we-fast-yet use classes, `try` and prototypes, which
the reader refuses; the rows for them say so, and the share of the set that is read is itself a
result. The kernels below exist so that some rows are artifacts.

## What codespec needs from swell

Not a story here: swell's work is swell's. `story:native-speed-comparison` cannot be implemented
before swell's `story:bench-runner-and-data-file` (beyond10x/swell#9) is delivered: the schema
`swell-bench/1` as a file at a swell release tag (compared here by digest) and the entry convention
for a script (how the script names the function to call and its arguments). It pins a swell release tag for the schema as `story:ecmascript-swell-surface` does for
the syntax crate.

## Done When

- A failing test first: `refused_script_is_a_row_with_reason` builds the rows of a directory holding
  `eval.js` (a call to `eval` on line 3) and expects one row, `status: refused`, `at: eval.js:3`,
  naming `eval`; it fails until `rows` exists.
- `build` is deterministic: the same script and the same tool versions give a byte-equal `.wasm` and
  `artifact.json`. It refuses to run when `wasm-tools` is not the version the codespec release pins,
  naming both versions; a test holds it with a stub that prints another version.
- The rows validate against the schema file of the pinned swell release (compared by digest, no
  network); a row that does not is refused naming the field. Rows hold no time, no count of runs and
  no engine other than the artifact's own runner: a test greps the rows for such fields, since the
  measuring is swell's.
- The kernels `benchmarks/ecmascript-native/kernels/` (`fib`, `sieve`, `matmul`, `collatz`, `nbody`),
  written for the subset under this repository's licence and offered to swell as an input set: at
  least four of the five build, and each of the five has a row, built or refused with its reason.
  Every script swell's set holds at the pinned tag gets a row the same way; the report of
  `rows` prints `read N of M`.
- `run` prints the result line for every built kernel, `match` against `node`; a kernel changed by one
  float bit (`0.1 + 0.2` against `0.3`) prints `differs` and exits 1; a test holds it.
- The release workflow ships `codespec-bench` in the release archive beside the readers and writers it
  calls, so swell's bench builds artifacts from a pinned codespec release, never from a checkout; a
  check of the release workflow's program list names it.
- The gate never builds a benchmark artifact beyond the kernels' own build test, runs no engine
  but the container's `wasmtime` and `node` on tiny inputs, and `task check` and every part of
  `scripts/check-parts.txt` name no timing command. Each test under 60 s, the step under 120 s; the
  time of the new test binary is stated.
- Near misses: a script whose entry the schema's convention does not name (a row `refused`, reason
  `no entry`), a script with two top-level functions of the same name (a row `refused`, reason `duplicate
  function <name>`, at the file and line of the second declaration, no artifact), an artifact directory with a
  `.wasm` that does not match `artifact.json`'s digest (`run` refuses), a refused script that still
  has a source digest in its row.

## Scope

- Own: `crates/bench` (new), `benchmarks/ecmascript-native/kernels` (new),
  `benchmarks/ecmascript-native/schema` (new: the digest of swell's schema file),
  `.github/workflows/release-build.yml` (the program list), root `Cargo.toml` `members`,
  `Cargo.lock`, `Taskfile.yml`, `scripts/check-parts.txt`, `AGENTS.md`, `CHANGELOG.md`.
- Shares root `Cargo.toml`, `Cargo.lock`, `Taskfile.yml`, `scripts/check-parts.txt`, `AGENTS.md` and
  `CHANGELOG.md` with every other story of the epic; it comes after `story:wasm-writer` and
  `story:ecmascript-run-oracle`, which it depends on, and before `story:x86-64-writer`, which
  depends on it and adds the x86-64 target to `crates/bench`.
