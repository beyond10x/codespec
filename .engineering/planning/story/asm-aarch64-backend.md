---
format: aep.planning-md/3
id: story:asm-aarch64-backend
kind: story
status: draft
title: The AArch64 backend
summary: adapters/asm writes asm_ir/1 as AArch64 assembly, run under qemu-aarch64.
relations:
- decomposes: epic:asm-ir-native-backends
- depends_on: story:asm-x86-64-writer
scope:
- confidence: cited
  path: adapters/asm-run
- confidence: cited
  path: adapters/asm/src/aarch64.rs
- confidence: cited
  path: crates/asm-ir/tests/ops
- confidence: cited
  path: scripts/ci-tools.sh
revision: 3
---
# Story: The AArch64 backend

## Outcome

`adapters/asm` gains `aarch64::emit`: AAPCS64-shaped frames under the private calling convention
of `docs/asm-ir.md` A4, assembled with `clang --target=aarch64-linux-gnu -c`, linked with
`ld.lld -static -e _start`, run by `codespec-run-asm --module <file> --arch aarch64` under
`qemu-aarch64`. The mapping is `docs/asm-ir/arm-rv-mapping.md`. CI installs `qemu-aarch64` in
the `check / tools` job.

## Done When

- A failing test first: `runs_ops_add_i64_aarch64`, the corpus module `add_i64` through the
  backend and `codespec-run-asm --arch aarch64`, printing `5\n` with exit 0.
- Every module of `crates/asm-ir/tests/ops/index.txt` prints its expected output on AArch64, and
  the corpus gains the AArch64 edges, each a module listed in `index.txt`: a constant needing
  `movz` and three `movk`; a load and a store at a frame offset beyond the scaled 12-bit range; a
  float compare with NaN under each `cc` (the conditions `mi` and `ls`, never `lt`/`le`).
- On a host without `qemu-aarch64` the run tests fail, naming the missing program; they never
  pass by skipping.
- `scripts/ci-tools.sh` installs `qemu-aarch64`, and the part passes in CI.
- Every test under 60 s; the step under 120 s.

## Scope

- Own: `adapters/asm/src/aarch64.rs` (new), `adapters/asm-run` (the `aarch64` target),
  `crates/asm-ir/tests/ops/` (the AArch64 modules), `scripts/ci-tools.sh` (the install).
- After `story:asm-x86-64-writer` (shared `adapters/asm` and `adapters/asm-run`); before
  `story:asm-riscv64-backend` (the same files and `scripts/ci-tools.sh`).
