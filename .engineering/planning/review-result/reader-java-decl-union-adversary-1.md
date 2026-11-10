---
format: aep.planning-md/3
id: review-result:reader-java-decl-union-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on the Java reader's decl.union unit
relations:
- reviews: story:reader-java-decl-union
revision: 1
---
# Adversary pass 1: the Java reader's decl.union unit

Unit: `story:reader-java-decl-union`, commit `23f841ac` on `impl/reader-java-decl-union`.

Verdict: NEEDS-CHANGE. Four red tests, two findings, both introduced by the unit.

## What was run

- `java codespec.read.AdversaryUnionTest`: 0 passed, 2 failed.
- `cargo test -p b10x-codespec-eval --test runner --test adversary_go_spelling --locked --no-fail-fast`:
  `adversary_go_spelling` 0 passed, 2 failed; `runner` 7 passed.
- The base reader (`23f841ac~1`) refuses the F1 source with one refusal, so F1 is introduced.

## Findings

F1. `unionShape` decided that a record declares nothing from its source text ending in `{}`. A
record whose last body line is a line comment ending in `{` passed the check, so a method or a
compact constructor was dropped with no refusal.

F2. `go_spelled` mapped a called `Echo` to an exported `echo` without checking that no other
package-level name holds `Echo`. The Go writer exports `echo` as `Echo_` then. No kit case reaches
it.

## Attacked and not broken

`permits` contents that are not plain package records; component types that are a union, a struct
or `char`; records in other files; uses of the interface or a record as a parameter, in `new`,
`instanceof`, concatenation, `equals`, a generic argument or a field; annotations on records and
components; modules read by the Go and Rust readers; a module that exports both `echo` and `Echo`.

## Outcome

Both fixed in the unit's fix pass. `declaresMembers` walks the record's `ClassTree` members;
`go_spelled` gives up when a package-level name holds the called name. `UnionMemberTest` (8
variants) and `crates/eval/tests/go_spelling.rs` (2 tests) hold them.

```findings
- file: "adapters/java/src/codespec/read/Reader.java"
  line: 1483
  category: "boundary"
  severity: "warning"
  origin: "introduced"
  verdict: "NEEDS-CHANGE"
  message: "unionShape's source-text endsWith(\"{}\") check is fooled by a trailing line comment ending in '{', so a record with a method or validating compact constructor is read as a union variant with the member dropped and no refusal."
- file: "crates/eval/src/runner.rs"
  line: 226
  category: "contract-drift"
  severity: "warning"
  origin: "introduced"
  verdict: "INFEASIBLE"
  message: "go_spelled maps Echo to an exported echo even when a type or unexported function already holds Echo, where the Go writer exports echo as Echo_, so the runner runs a function the Go path would not; no kit case reaches it."
```
