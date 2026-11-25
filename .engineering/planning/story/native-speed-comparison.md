---
format: aep.planning-md/3
id: story:native-speed-comparison
kind: story
status: draft
title: A speed comparison with the fastest ECMAScript engines, measured on the build node
summary: codespec-bench runs public numeric benchmarks under V8, JavaScriptCore, SpiderMonkey, QuickJS, swell where it runs, and the written Wasm and x86-64 code, on the build node only, into a committed report.
relations:
- decomposes: epic:ecmascript-to-native
- depends_on: story:wasm-writer
- depends_on: story:ecmascript-run-oracle
- informed_by: story:x86-64-writer
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
  path: benchmarks/ecmascript-native
- confidence: inferred
  path: containers
- confidence: inferred
  path: crates/bench
- confidence: inferred
  path: scripts/check-parts.txt
- confidence: inferred
  path: website
revision: 2
---
# Story: A speed comparison with the fastest ECMAScript engines, measured on the build node

## Outcome

`codespec-bench`, a Rust crate `crates/bench`, runs a set of public benchmark programs, each written
in the ECMAScript subset, under every engine and every written target, on the build node only, and
writes one committed report (`benchmarks/ecmascript-native/report.json` and the page generated from
it) with medians, spread and the versions of everything that took part. The report answers one
question: how far is code written through `cir/1` from the fastest engines on typed numeric
JavaScript.

## Why

Requested by the owner with the path itself. A number without its method is not a comparison, and a
benchmark that runs on a busy workstation or a shared node measures the neighbours. The method is
fixed here before the first result exists, so a result cannot choose it.

## The set

- Programs, public and licensed, each file keeping its licence header and source URL: the numeric
  kernels of the Computer Language Benchmarks Game (`nbody`, `spectral-norm`, `mandelbrot`,
  `fannkuch-redux`; 3-clause BSD as that site states, to be verified per file before it is
  committed), adapted to the subset (typed arrays, tagged entry functions, no output formatting),
  plus kernels written for this repository (`fib`, `sieve`, `matmul`, `collatz`) under the
  repository's licence. Each adaptation is a diff against the original, committed beside it. A
  program the reader refuses is listed with its refusal and is not rewritten to pass.
- Engines, each with a public source and a version pinned in the container image: V8 through `node`,
  JavaScriptCore through `bun`, SpiderMonkey through its `js` shell, QuickJS (the quickjs-ng fork or
  Bellard's, one named), and swell where it has a runnable binary at its pinned tag (a library with
  no binary gives a row of `not measured: no binary at <tag>`, never a driver written here).
- Written targets: the `wasm` output on `wasmtime` and on `node`, the `x86-64` output linked and run
  natively. Where the x86-64 writer does not yet exist the column is absent and says so.

## Method

- Only on the build node, through `story:native-run-harness` (a `slot` job, the container
  with `--cpus 1` and a pinned cpu set); `codespec-bench` refuses to run where the node's CPU model
  is not the recorded one and refuses to run on the workstation at all, by a check of the host
  name against the recorded node.
- Cold: a fresh process per run, wall time from `exec` to exit, including start-up and
  compile; 11 runs, the median, the minimum and the maximum are reported. An empty program gives
  the start-up floor of each engine, printed beside every row.
- Warm: the kernel's entry is called `K` times in one process after `W` warm-up calls, timed inside the
  process by the engine's monotonic clock; 11 processes, the median of the per-process median.
  `K` and `W` are chosen so the slowest engine's run is at least one second and written beside
  each result. Wasm and native code have no warm-up, so for them warm equals cold minus start-up,
  reported once and marked.
- Every run's output is checked against the oracle's (`story:ecmascript-run-oracle`) before its
  time is kept; a result that differs is a failure of that cell, shown as such, never timed.
- A run is discarded and repeated when another slot job overlapped it (read from the node's job
  log); a cell with three discards is reported as `noisy`.
- Results are in time and as a ratio to V8 (`node`), both columns.

## Done When

- A failing check first: `report_cell_without_version_fails` loads a report fixture whose engine
  row has no version and expects the schema check to name the cell; it fails until the check exists.
- The report schema (`codespec-bench-report/1`) is checked in the gate, fast and with no run:
  every cell has the engine version, the image digest, the commit, the node's CPU model, the raw
  samples, the median, the minimum and the maximum; a median that is not the median of the samples
  fails; the page is regenerated from the JSON and compared byte for byte.
- A run (`task bench`, not in the gate, a slot job on the node of at most 30 minutes, split per
  program when it would be longer) writes the report; the report is committed with the run's
  commit and the log of the node's jobs that ran during it.
- The gate never runs a benchmark: a test that did would fail the budget rule; a test holds that
  `task check` and every part of `scripts/check-parts.txt` name no `bench` command.
- Near misses: an engine that prints the right answer but starts a second thread (counted, the
  report states `--cpus 1`), a kernel whose result is optimised away in one engine (each kernel's
  result feeds a printed checksum), a warm-up that is too short (`W` is shown against the point
  where the median stops falling), an engine whose version differs between two runs (the report
  refuses to merge them).
- Not a ratchet in this story. A KPI over the ratio to V8, held by a `ratchet/1` document in the way
  `epic:reader-self-improvement-loop` (draft, open pull request 66) holds the reader KPIs, is named
  as the next step and is the owner's decision.

## Scope

- Own: `crates/bench` (new), `benchmarks/ecmascript-native/` (new: programs, adaptations,
  report), `containers/` (the engine image, a file of `story:native-run-harness`'s directory),
  root `Cargo.toml` `members`, `Cargo.lock`, `Taskfile.yml`, `scripts/check-parts.txt`,
  `website/` (one page), `AGENTS.md`, `CHANGELOG.md`.
- Shares `Taskfile.yml`, `scripts/check-parts.txt` and `containers/` with every other story of the
  epic; this story last.
