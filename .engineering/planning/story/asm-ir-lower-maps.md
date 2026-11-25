---
format: aep.planning-md/3
id: story:asm-ir-lower-maps
kind: story
status: draft
title: Lower maps
summary: 'Group 7: insertion-ordered maps; 122 kit cases in total.'
relations:
- decomposes: epic:asm-ir-native-backends
- depends_on: story:asm-ir-lower-floats
scope:
- confidence: cited
  path: adapters/asm/src/claim.rs
- confidence: cited
  path: crates/lower
revision: 5
---
# Story: Lower maps

## Outcome

`crates/lower` lowers group 7 of `docs/asm-ir.md` § 5: maps over integer and string keys,
insertion-ordered as the evaluator's `MapStore` and the C runtime's `cs_map` are, with `make`,
index, assignment, `len`, `lookup_else`, comma-ok, `for_range` and `sorted_keys`, as `asm_ir`
runtime functions. The claim list `adapters/asm/src/claim.rs`, which every writer of `adapters/asm` reads, gains the map features; where `story:asm-kit-conformance` is implemented, its parts hold them on AArch64 and RISC-V 64 as well.

## Done When

- A failing test first: the kit case `type-map` passes on x86-64.
- `codespec conform --lang x86-64 --role writer` passes the 7 map cases, 122 with groups 1 to 6.
- Every test under 60 s; the step under 120 s.

## Scope

- Own: `crates/lower`, `adapters/asm/src/claim.rs`.
- After `story:asm-ir-lower-floats` (shared `crates/lower`).
