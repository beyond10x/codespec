---
format: aep.planning-md/3
id: story:asm-ir-lower-core
kind: story
status: draft
title: 'Lower cir/1 to asm_ir/1: integers, structs, control flow and the runtime'
summary: crates/lower and codespec lower take group 1 (78 kit cases) and build the runtime in asm_ir.
relations:
- decomposes: epic:asm-ir-native-backends
- depends_on: story:asm-ir-model
scope:
- confidence: cited
  path: crates/cli
- confidence: cited
  path: crates/lower
revision: 5
---
# Story: Lower `cir/1` to `asm_ir/1`: integers, structs, control flow and the runtime

## Outcome

`crates/lower` (`b10x-codespec-lower`) and the command `codespec lower` read one `cir/1` module and
print one `asm_ir/1` module, for group 1 of `docs/asm-ir.md` § 5: integers of every width and
signedness, `bool`, calls, `if`, `for`, `switch`, `break`, `continue`, compound assignment,
`incr`/`decr`, constants, newtypes, enums, module variables, structs, pointers, methods, fixed
arrays, `zero` and several results. The runtime of § 6 needed by that group (`rt.alloc`,
`rt.panic` and its wrappers, `rt.write_all`, `rt.fmt_i64`, `rt.memmove`) is built in `asm_ir` by
the crate's builder and emitted only when reachable. A function outside the group is left out and
listed with its key, position and reason, as the C writer lists refusals.

## Done When

- A failing test first: `lowers_add_i64` lowers the kit case `op-add` and checks the lowered
  function's instructions (`bin add i64`), reading the result back with `asm_ir::Module::from_json`;
  it fails until the crate exists.
- The semantics are those of the evaluator, held by tests that lower a module and check the
  instructions emitted: every `+ - *` and unary `-` of an 8-, 16- or 32-bit integer is followed by
  an `ext` of its width (A11); every `/` and `%` whose divisor is not a nonzero constant is preceded
  by a zero check that calls `rt.panic_divzero`; a shift by a constant at or above the width is a
  constant (0, or `sar` by width-1), by a variable count is the masked sequence of A8, and a
  negative signed count calls `rt.panic_shift`; an array index is bounds-checked and calls
  `rt.panic_index`; a dereference of a pointer is nil-checked and calls `rt.panic_nil`.
- A struct argument is passed as a pointer to a caller-made copy, a struct result through a hidden
  destination pointer (A3); a local whose address is taken lives in a slot.
- Refused with a reason, each with a test: `foreign`, `raise`, an extern, a construct of a later
  group (named: "slice: story:asm-ir-lower-slices-strings-errors").
- `codespec lower` on each of the 78 kit cases of group 1 (the list is `crates/lower/tests/group1.txt`,
  generated from `docs/asm-ir/cir-semantics.md` § 6 and checked against the cases' `features.json`)
  prints a module strict reading accepts, with no refusal; lowering twice gives the same bytes.
- Every test under 60 s; the crate's test step under 120 s.

## Scope

- Own: `crates/lower` (its skeleton made by `story:asm-ir-model`), `crates/cli` (`codespec lower`).
- `Cargo.lock` stays as `story:asm-ir-model` left it: the skeleton's dependency list is final, and a
  dependency this story finds it needs goes to the coordinator, not into this story's diff.
- Reads `crates/asm-ir`, `crates/model`; changes neither.
