---
format: aep.planning-md/3
id: story:asm-ir-lower-slices-strings-errors
kind: story
status: draft
title: Lower slices, strings and errors
summary: 'Groups 2 to 5: slices, string literals, run-time strings and sentinel errors; 111 kit cases in total.'
relations:
- decomposes: epic:asm-ir-native-backends
- depends_on: story:asm-x86-64-writer
scope:
- confidence: cited
  path: adapters/asm/src/claim.rs
- confidence: cited
  path: crates/lower
revision: 5
---
# Story: Lower slices, strings and errors

## Outcome

`crates/lower` lowers groups 2 to 5 of `docs/asm-ir.md` § 5: slices of numbers (`make`, `append`
with doubling, `len`, index with a bounds check, `for_range`), string literals as read-only data,
string `==` and `len`, strings built at run time by `sprintf` and `errorf` with the verbs the kit
uses, sentinel errors (`errvar`, `nil`, `to_error`) and the driver's `ok`/`err` lines with Go's
`strconv.Quote` for a string result. The runtime gains `rt.append`, `rt.str_eq`, `rt.write_quoted`
and a string builder over `rt.alloc`. The claim list `adapters/asm/src/claim.rs`, which every writer of `adapters/asm` reads, gains the features of these groups; where `story:asm-kit-conformance` is implemented, its parts hold them on AArch64 and RISC-V 64 as well.

## Done When

- A failing test first: the kit case `type-slice` passes on x86-64 through `codespec-run-asm`.
- `codespec conform --lang x86-64 --role writer` passes the 33 cases of groups 2 to 5 (7 + 11 + 2
  + 13, listed in `crates/lower/tests/group2-5.txt`), 111 with group 1.
- A string holding a byte outside printable ASCII is refused when printed (exit 2), as the other
  runners refuse it.
- Every test under 60 s; the step under 120 s.

## Scope

- Own: `crates/lower`, `adapters/asm/src/claim.rs`.
- `story:asm-ir-lower-floats` and `story:asm-ir-lower-maps` write `crates/lower` after this story.
