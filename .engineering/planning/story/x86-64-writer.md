---
format: aep.planning-md/3
id: story:x86-64-writer
kind: story
status: draft
title: 'An x86-64 assembly writer: cir/1 to GNU assembly, System V, Linux'
summary: codespec-write-x86-64 writes AT&T assembly for the System V ABI with a bump allocator, codespec-run-x86-64 links it with a C driver and runs it in a container.
relations:
- decomposes: epic:ecmascript-to-native
- depends_on: story:wasm-writer
- depends_on: story:native-run-harness
- informed_by: story:reader-asm
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
  path: adapters/x86-64
- confidence: inferred
  path: adapters/x86-64-run
- confidence: inferred
  path: containers/x86-64
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
# Story: An x86-64 assembly writer: cir/1 to GNU assembly, System V, Linux

## Outcome

`codespec-write-x86-64`, a Rust crate `adapters/x86-64`, writes a `cir/1` module as one x86-64
assembly file (GNU `as`, AT&T syntax, System V AMD64 ABI, ELF, Linux), and `codespec-run-x86-64`,
`adapters/x86-64-run`, assembles and links it in the container of `story:native-run-harness`
with a generated C driver and prints the kit's encoding. First cut: integers of every width,
`bool`, `float` 32 and 64 (SSE2 scalar), calls, control flow, `struct`, fixed arrays, slices of
numbers and string literals, with a small runtime in assembly: a bump allocator over
`mmap`, an out-of-range trap and a panic exit. Everything else is refused with a reason.

## Why

The second target the owner named. Unlike Wasm there is no collector under it: the written
program owns its memory. Written code that never frees is what the C runtime does today
(`conformance/c/codespec_rt.h`), and short-lived benchmark programs run under it, so the first
cut states the limit instead of hiding it: an allocation past the arena stops the program with
exit 4 and a message. The C and Rust writers refuse floats and function values; this one takes
floats from the start, because every kernel of `story:native-speed-comparison` needs them.

## Done When

- A failing test first: `writes_add_i64` writes `func Add(a, b int64) int64` and the runner, which
  links a C driver that calls `Add` by the System V convention, prints `ok 5` for `Add(2, 3)`; it
  fails until the writer exists. Calling it from C is the ABI check.
- Mapping, decided by the tests: integers in the registers of their width with sign or zero
  extension after each operation (`sem.int.wrap`); `idiv` and `div` by sign, with the divide-by-zero
  and `MinInt / -1` cases a named trap; floats in `xmm`, `float` 32 rounded after each operation,
  `ucomisd` with the NaN result of each comparison written out; a struct by pointer to the stack or
  the arena, aggregates of one or two eightbytes passed as the ABI says; locals in stack slots
  (`-O0` shape: no register allocation in the first cut, stated); callee-saved registers kept.
- Refused with a reason and position: `map`, `chan`, `any`, `error`, interfaces, generics,
  `lambda` and `call_value` (a function value needs an environment: the Wasm writer has it), string
  `+` and any string built at run time, `raise`, `foreign`.
- Checked by running both: for every case the writer claims, and for every module of
  `examples/series` and `examples/inventory` it claims, the evaluator (`codespec eval --calls`)
  and the linked binary print the same bytes on the same `calls.json`. No `codespec harness` driver
  for x86-64 exists under `examples/*/drivers` and this story adds none: the harness is not
  claimed. `codespec conform --lang x86-64 --role writer` has `conforms: true`; refused kit cases
  are listed.
- The float and number features are claimed and mapped: `sem.float.ieee` (`addsd`, `subsd`, `mulsd`,
  `divsd` and their single forms, comparisons through `ucomisd` with NaN false), `sem.float.to_int`
  and `sem.int.to_float` (`cvttsd2si`, `cvtsi2sd`, in the range
  `story:ecmascript-number-semantics` defines), and `intrinsic.math`: `math.Sqrt` as `sqrtsd`,
  `math.Abs` as a mask of the sign bit, `math.Floor` and `math.Trunc` as `roundsd` (SSE4.1; the
  story names the baseline it assumes), `math.Mod` as a loop of `fprem` or a runtime function; each
  has the kit case of that story passing, printed in its float encoding.
- Near misses held by tests: `MinInt64 / -1`, a shift by 64 or more, `uint8` 255 + 1, a negative `%`,
  `-0.0` and NaN comparisons, a struct of three eightbytes returned by value, a call with seven
  integer arguments and nine float arguments (stack-passed), an unaligned stack at a call (the
  16-byte rule, checked by a libc call that needs it), an array index at `len`, `len - 1` and
  `-1` (trap, with the same exit code as the evaluator's stop mapped by the runner), an arena
  exhausted by a loop (exit 4), a refused module that writes no file.
- Round trip, as far as it goes: the written assembly is read back by `story:reader-asm`
  (`codespec read --lang asm`) and the number of functions it reads against the number written is
  reported. That reader targets `cc -S -O0` output, so this writer's output may not be inside its
  table: the story records the count and each refusal, and does not fail on a refusal. The check is
  skipped, with the reason printed, until `story:reader-asm` is implemented; it is not counted as
  passed.
- Output is deterministic: the same module gives the same file; labels are numbered in order of
  appearance and carry no address or time.
- No step budget is emitted (decision D4): a loop that does not end ends at the container's
  time limit with the harness's `timeout` verdict; a test holds it with `for {}` and a one-second
  limit.
- `task check` exits 0; each test under 60 s, the step under 120 s; assembling, linking and running
  happen in one container call per step, in the shared fixture. The time of the new test binary is
  stated.

## Scope

- Own: `adapters/x86-64` (new), `adapters/x86-64-run` (new), `containers/x86-64` (new: the `as`,
  `ld`, `cc` image, built by the mechanism of `story:native-run-harness`), `crates/cli` (the `--lang` value),
  `crates/conformance`, `crates/docs`, root `Cargo.toml` `members`, `Cargo.lock`, `Taskfile.yml`,
  `scripts/check-parts.txt`, `AGENTS.md`, `CHANGELOG.md`, one site page.
- Shares `crates/cli`, `crates/conformance`, `crates/docs` and `Taskfile.yml` with
  `story:wasm-writer` (first) and `story:reader-asm`.
- Inferred by the planner: AT&T syntax (the dialect of `story:reader-asm`), `-O0` shape, no
  collector.
