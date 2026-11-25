---
format: aep.planning-md/3
id: story:asm-riscv64-backend
kind: story
status: draft
title: The RISC-V 64 backend
summary: adapters/asm writes asm_ir/1 as RV64GC assembly, run under qemu-riscv64.
relations:
- decomposes: epic:asm-ir-native-backends
- depends_on: story:asm-aarch64-backend
scope:
- confidence: cited
  path: adapters/asm-run
- confidence: cited
  path: adapters/asm/src/riscv64.rs
- confidence: cited
  path: crates/asm-ir/tests/ops
- confidence: cited
  path: scripts/ci-tools.sh
revision: 3
---
# Story: The RISC-V 64 backend

## Outcome

`adapters/asm` gains `riscv64::emit`: RV64GC, LP64D-shaped frames under the private calling
convention of `docs/asm-ir.md` A4, assembled with `clang --target=riscv64-linux-gnu -c`, linked
with `ld.lld -static -e _start` without `--relax-gp`, run by `codespec-run-asm --module <file>
--arch riscv64` under `qemu-riscv64`. The mapping is `docs/asm-ir/arm-rv-mapping.md`. CI installs
`qemu-riscv64` in the `check / tools` job.

## Done When

- A failing test first: `runs_ops_add_i64_riscv64`, the corpus module `add_i64` through the
  backend and `codespec-run-asm --arch riscv64`, printing `5\n` with exit 0.
- Every module of `crates/asm-ir/tests/ops/index.txt` prints its expected output on RISC-V 64, and
  the corpus gains the RISC-V edges, each a module listed in `index.txt`: a conditional branch to
  a block more than 4 KiB away (the backend always writes an inverted branch over a `j`); a frame
  offset beyond ±2047; a NaN compare under each `cc`. A test checks that `sym_addr` is written
  with `lla`, never `la`.
- On a host without `qemu-riscv64` the run tests fail, naming the missing program; they never
  pass by skipping.
- `scripts/ci-tools.sh` installs `qemu-riscv64`, and the part passes in CI.
- Every test under 60 s; the step under 120 s.

## Scope

- Own: `adapters/asm/src/riscv64.rs` (new), `adapters/asm-run` (the `riscv64` target),
  `crates/asm-ir/tests/ops/` (the RISC-V modules), `scripts/ci-tools.sh` (the install).
- After `story:asm-aarch64-backend` (shared files).
