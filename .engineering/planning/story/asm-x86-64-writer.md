---
format: aep.planning-md/3
id: story:asm-x86-64-writer
kind: story
status: draft
title: 'The x86-64 writer: cir/1 to a running binary, held to the kit'
summary: codespec-write-x86-64 composes lowering and the x86-64 backend; codespec-run-asm runs kit cases; the gate runs conform for x86-64.
relations:
- decomposes: epic:asm-ir-native-backends
- depends_on: story:asm-ir-lower-core
- depends_on: story:asm-x86-64-backend
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: adapters/asm-run
- confidence: cited
  path: adapters/asm/src/bin/codespec-write-x86-64.rs
- confidence: cited
  path: adapters/asm/src/claim.rs
- confidence: cited
  path: crates/cli
- confidence: cited
  path: scripts/check-parts.txt
revision: 4
---
# Story: The x86-64 writer: cir/1 to a running binary, held to the kit

## Outcome

The first whole path, `cir/1 -> asm_ir/1 -> x86-64 -> ELF`, on the host's architecture.
`adapters/asm` gains the writer program `codespec-write-x86-64`: it reads `cir/1` on stdin, lowers
it with `crates/lower`, emits it with the x86-64 backend and writes `<out>/<module>.s`,
`<module>.asm-ir.json` and `refusals.json`; `--asm-ir` reads an `asm_ir/1` module instead of
`cir/1`; `--describe` prints the manifest. `codespec write --lang x86-64` finds it by name.

`codespec-run-asm <dir> <calls.json>` (the writer contract of `docs/conformance.md` § 4) reads the
target from the `.s` file's first line, reads `<module>.asm-ir.json`, generates the driver as
`asm_ir` (an `entry` making each call of `calls.json` with literal arguments and printing the
kit's encoding through the runtime of `crates/lower`, as `codespec-run-c` generates `driver.c`),
and runs it by the `--module` mode of `story:asm-x86-64-backend`. Exit 0 printed, 2 refused,
1 failed.

The gate runs `codespec conform --lang x86-64 --role writer` as a part of
`scripts/check-parts.txt`.

## Done When

- A failing test first: `the_x86_64_writer_runs_op_add`, the kit case `op-add` through
  `codespec-write-x86-64` and `codespec-run-asm`, equal to `expected.txt`.
- `codespec conform --lang x86-64 --role writer` reports `conforms: true` with the 78 cases of
  group 1 (`crates/lower/tests/group1.txt`) passing; the manifest claims group 1 and lists every
  other feature in `not`.
- The runner's other exits, each a test: a `calls.json` calling a function the module does not
  export exits 2 naming it; a directory without a `.s` file exits 1; an argument that does not fit
  its parameter exits 2.
- `--asm-ir`: the `<module>.asm-ir.json` of `op-add`, fed back with `--asm-ir`, writes the same
  `.s` bytes.
- `codespec write --lang x86-64` on `op-add` writes the same three files as the program run
  directly, checked by a test in `crates/cli`.
- `scripts/check-parts.txt` holds the part `conform x86-64`, and `task check-parts` fails when it
  is removed (shown by the part list check of the `lint` part).
- The part's time stated in the pull request, under 120 s; the 78 drivers are built in one batch.

## Scope

- Own: `adapters/asm/src/bin/codespec-write-x86-64.rs`, `adapters/asm/src/claim.rs` (new: the
  claim list every writer of `adapters/asm` reads, group 1), `adapters/asm-run`
  (the writer-contract mode, the driver), `crates/cli` (the `x86-64` `--lang` value),
  `scripts/check-parts.txt` (the conform part), `Taskfile.yml`, `CHANGELOG.md`.
- After `story:asm-ir-lower-core` and `story:asm-x86-64-backend`; before the AArch64 and RISC-V 64
  backends only by the shared files of `adapters/asm` and `adapters/asm-run`.
