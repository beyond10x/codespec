---
format: aep.planning-md/3
id: story:asm-ir-interpreter
kind: story
status: draft
title: An asm_ir/1 interpreter
summary: Runs asm_ir/1 with the four syscalls simulated, to tell a lowering bug from a backend bug.
relations:
- decomposes: epic:asm-ir-native-backends
- depends_on: story:asm-x86-64-writer
- depends_on: story:asm-riscv64-backend
scope:
- confidence: cited
  path: crates/asm-ir/src/interp.rs
- confidence: cited
  path: crates/asm-ir/tests/interp.rs
revision: 5
---
# Story: An `asm_ir/1` interpreter

## Outcome

`crates/asm-ir` gains `interp::run(&Module) -> Outcome`: a vreg array per frame, flat byte memory
for slots, data and the heap, and the four syscalls simulated (`write` to captured stdout and
stderr, `read` from a given stdin, `exit` with its code, `mmap` from the flat memory). It tells a
lowering bug (the interpreter disagrees with the evaluator) from a backend bug (a target disagrees
with the interpreter) when all three targets fail alike (`docs/asm-ir.md` § 8.4).

## Done When

- A failing test first: the corpus module `add_i64` runs in the interpreter and prints `5\n`.
- Every module listed in `crates/asm-ir/tests/ops/index.txt` prints its expected output in the
  interpreter, with the exit status of its expected run.
- Each of the 78 cases of `crates/lower/tests/group1.txt`, lowered and run with the driver of
  `story:asm-x86-64-writer` in the interpreter, prints `expected.txt`.
- Every test under 60 s; the step under 120 s.

## Scope

- Own: `crates/asm-ir/src/interp.rs` (new), `crates/asm-ir/tests/interp.rs` (new).
- Reads, never writes: `crates/asm-ir/tests/ops/` and its `index.txt`, `crates/lower`, the driver
  of `adapters/asm-run`.
- After `story:asm-x86-64-writer` (the lowering and the driver it runs) and
  `story:asm-riscv64-backend` (the last story that adds corpus modules, so `index.txt` is complete
  when this story runs it).
