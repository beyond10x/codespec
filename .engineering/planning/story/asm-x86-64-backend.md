---
format: aep.planning-md/3
id: story:asm-x86-64-backend
kind: story
status: draft
title: The x86-64 backend, the module runner and the op corpus
summary: adapters/asm writes asm_ir/1 as AT&T assembly; codespec-run-asm assembles, links and runs it; the op corpus pins each operation.
relations:
- decomposes: epic:asm-ir-native-backends
- depends_on: story:asm-ir-model
scope:
- confidence: cited
  path: adapters/asm-run/src
- confidence: cited
  path: adapters/asm/src
- confidence: cited
  path: crates/asm-ir/tests/ops
revision: 5
---
# Story: The x86-64 backend, the module runner and the op corpus

## Outcome

`adapters/asm` (library `codespec_asm`) holds the x86-64 backend
`x86_64::emit(&asm_ir::Module) -> Result<String, Refusal>`: GNU `as`, AT&T syntax, ELF, Linux,
the private calling convention of `docs/asm-ir.md` A4 (every argument and result an 8-byte cell,
every vreg a frame cell), `_start` calling the module's `entry`, and the four syscalls. All 41
operations of `docs/asm-ir.md` § 4 are written, floats included (SSE2 scalar). The first line of
the output is `# codespec: arch x86-64 asm_ir/1`. A module whose `features` name one the backend
does not claim is refused whole.

`adapters/asm-run` (`codespec-run-asm`, library `codespec_run_asm`), mode `--module <file>
--arch <target>`, takes an `asm_ir/1` module with an `entry`, writes it with the backend of
`--arch`, assembles with `as`, links with `ld.lld -static -e _start`, runs the ELF (natively on
x86-64; under `qemu-<arch>` otherwise), and prints its stdout and stderr and exits with its
status. Tool lookup is one function, so the containers of `story:native-run-harness` can replace
the host tools later.

`crates/asm-ir/tests/ops/` holds the per-instruction corpus of `docs/asm-ir.md` § 8.3: one
`asm_ir/1` module per operation kind, made with the builder of `story:asm-ir-model`, with
boundary operands and an expected output written by hand; `index.txt` lists every module with the
operation it covers. The modules print through `syscall write` of bytes they compute, so the
corpus needs nothing from the lowering.

## Done When

- A failing test first: `runs_ops_add_i64` writes the corpus module `add_i64` (prints `2 + 3` as
  one decimal line through a hand-built digit loop) as x86-64, runs it and gets `5\n`, exit 0. It
  fails until the backend exists.
- `index.txt` names each of the 41 operation kinds at least once; a test fails when an operation
  of `asm_ir::Op` has no module, so a corpus that omits one cannot pass.
- Every module of `index.txt` prints its expected output on x86-64. The boundary modules include:
  `MinInt / -1` and `MinInt % -1` at 32 and 64 bits (the `-1` fix-up of A7); shifts by 0, W-1 and
  W through the masked form; `ext` from 8, 16 and 32 bits signed and unsigned; every `cmp`
  condition at the signed and unsigned boundaries; `fcmp` under each `cc` with NaN, `-0.0` and
  equal operands; `fneg` of `0.0`; `ftoi` and `itof` of in-range values signed and unsigned, an
  `i64` above 2^63 to `f64` included; `fdemote` rounding; a frame larger than 4 KiB; a call with
  10 arguments and 3 results; recursion 1,000 deep; `trap 3` (exit 3, nothing printed); `syscall
  mmap` of 1 MiB written at both ends.
- The first line of every written file is `# codespec: arch x86-64 asm_ir/1`, checked by a test;
  a module listing a feature the backend does not claim is refused with exit 1 and no file, checked
  by a test.
- The same module written twice gives the same bytes; labels carry function and block indices
  only.
- Every test under 60 s; the step under 120 s; the corpus runs as one binary per architecture
  (one `entry` calling each module's function in turn), and the step's time is stated in the pull
  request.

## Scope

- Own: `adapters/asm` (its skeleton made by `story:asm-ir-model`: the shared frame and cell logic,
  `src/x86_64.rs`), `adapters/asm-run` (its skeleton: the `--module` mode), `crates/asm-ir/tests/ops/`
  (new).
- `Cargo.lock` stays as `story:asm-ir-model` left it (the skeletons' dependency lists are final).
- Reads `crates/asm-ir`. Does not read or write `crates/lower`, so it runs beside
  `story:asm-ir-lower-core`; the two share no file.
- `story:asm-aarch64-backend` and `story:asm-riscv64-backend` add modules to `adapters/asm` and
  targets to `adapters/asm-run` after this story.
