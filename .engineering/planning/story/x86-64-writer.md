---
format: aep.planning-md/3
id: story:x86-64-writer
kind: story
status: draft
title: 'x86-64 for the ECMAScript path: the bench target, the container and the math intrinsics'
summary: codespec-bench builds and runs ECMAScript kernels with codespec-write-x86-64 (asm_ir/1) in the pinned container, with math intrinsics added to asm_ir/1.
relations:
- decomposes: epic:ecmascript-to-native
- depends_on: story:wasm-writer
- depends_on: story:native-run-harness
- informed_by: story:reader-asm
- depends_on: story:native-speed-comparison
- depends_on: story:asm-kit-conformance
- depends_on: story:asm-ir-lower-maps
- depends_on: story:asm-ir-interpreter
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: Taskfile.yml
- confidence: cited
  path: adapters/asm-run
- confidence: cited
  path: adapters/asm/src/x86_64.rs
- confidence: inferred
  path: containers/x86-64
- confidence: cited
  path: crates/asm-ir
- confidence: cited
  path: crates/asm-ir/tests/ops
- confidence: inferred
  path: crates/bench
- confidence: cited
  path: crates/lower
- confidence: cited
  path: schemas/asm-ir-1.schema.json
revision: 13
---
# Story: x86-64 for the ECMAScript path: the bench target, the container and the math intrinsics

## Outcome

The x86-64 writer itself is `codespec-write-x86-64` of `epic:asm-ir-native-backends`
(`docs/asm-ir.md`), written through `asm_ir/1` in `adapters/asm`: its mapping, runtime, driver
and kit conformance are that epic's (`story:asm-x86-64-writer`, `story:asm-ir-lower-floats`). This
story takes it the rest of the way for `epic:ecmascript-to-native`: the x86-64 target of
`codespec-bench`, assembling and running in the container of `story:native-run-harness`, the
`intrinsic.math` functions every kernel of `story:native-speed-comparison` needs, and the round
trip through `story:reader-asm`.

Decided in the design and not redone here: no libc and no C driver (`docs/asm-ir.md` A4, A5), so
the ABI check by a C call of the first draft is replaced by the kit run through `codespec-run-asm`;
the IR's calling convention is private, so a System V entry point is a later, additive
`abi: "c"`; an allocation past the arena panics "out of memory" with exit 3 (`docs/asm-ir.md`
§ 6), not exit 4.

## Why

The second target the owner named for the ECMAScript path (2026-11-25, 13:19 CET: Wasm first,
x86-64 second). The owner's direction of the same day (`[any_lang] -> [codespec] -> [asm_ir/1] ->
[any_arch]`) moved the writer into one IR shared by three architectures; what is specific to the
benchmark path stays here.

## Done When

- A failing test first: `fib_kernel_through_the_bench`, the `fib` kernel through
  `codespec-bench build <file.js> --target x86-64 --out <dir>`, `rows` and `run`, printing the
  result line and `match` against `node`; it fails until the target exists.
- `codespec-bench build --target x86-64` reads the script, writes it with `codespec-write-x86-64`,
  assembles and links it with `as` and `ld.lld` of the pinned image of
  `story:native-run-harness` (refusing another version, naming both), and writes the ELF and an
  `artifact.json` with the runner command (the ELF run directly in the container); `rows` writes
  the row in `swell-bench/1` like the Wasm rows. A script that uses a closure, a map or a run-time
  string is a row `refused` with the writer's refusal and its `file:line`, no artifact; a test
  holds one (a closure).
- `intrinsic.math` through `asm_ir/1`: the operations `fsqrt`, `fabs` (a mask of the sign bit),
  `ffloor` and `ftrunc` are added to `asm_ir/1` (additive, guarded by the feature `float.math`,
  `docs/asm-ir.md` § 4), lowered from `math.Sqrt`, `math.Abs`, `math.Floor`, `math.Trunc`, and
  `math.Mod` as an `asm_ir` runtime function; the x86-64 backend writes `sqrtsd`, `andpd`, and
  `roundsd` (baseline x86-64-v2, SSE4.2, stated in `adapters/asm`'s crate doc); each has the kit
  case of `story:ecmascript-number-semantics` passing on x86-64. The AArch64 and RISC-V 64
  backends refuse `float.math` until a story adds it. The four operations get a corpus module each
  in `crates/asm-ir/tests/ops/` (listed in `index.txt` with the feature `float.math`, which the
  AArch64 and RISC-V 64 corpus runs skip by that feature, not by name), an arm each in the
  interpreter that prints the same bytes, and `schemas/asm-ir-1.schema.json` and the printer
  fixture regenerated.
- Round trip, as far as it goes: the written assembly is read back by `story:reader-asm`
  (`codespec read --lang asm`) and the number of functions it reads against the number written is
  reported. That reader targets `cc -S -O0` output, so this writer's output may not be inside its
  table: the story records the count and each refusal, and does not fail on a refusal. The check is
  skipped, with the reason printed, until `story:reader-asm` is implemented; it is not counted as
  passed.
- No step budget is emitted (decision D4): a loop that does not end ends at the container's time
  limit with the harness's `timeout` verdict; a test holds it with `for {}` and a one-second limit.
- `task check` exits 0; each test under 60 s, the step under 120 s; assembling, linking and running
  happen in one container call per step, in the shared fixture.

## Scope

- Own: `crates/bench` (the `x86-64` target of `build`, `rows` and `run`), `containers/x86-64`
  (new: the `as`, `ld.lld` image, built by the mechanism of `story:native-run-harness`),
  `crates/asm-ir` (the four math operations, their corpus modules, their interpreter arms),
  `schemas/asm-ir-1.schema.json`, `crates/lower` (their lowering and `rt.fmod`),
  `adapters/asm/src/x86_64.rs` (their templates), `adapters/asm-run` (the container as its tool
  lookup), `Taskfile.yml`, `CHANGELOG.md`.
- After `story:asm-ir-lower-maps` (the last writer of `crates/lower`), `story:asm-ir-interpreter`
  (`crates/asm-ir/src/interp.rs`), `story:asm-kit-conformance` (`adapters/asm`),
  `story:native-speed-comparison` (`crates/bench`) and `story:native-run-harness` (the container).
