---
format: aep.planning-md/3
id: story:nullable-reference-fields
kind: story
status: implemented
title: A JVM reference field with no initializer reads as a nil pointer
relations:
- decomposes: epic:groovy-grails-reader
- depends_on: story:jvm-type-mapping-shared
- depends_on: story:rust-writer-nil-pointer
scope:
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: adapters/go/cmd/codespec-write-go
- confidence: cited
  path: adapters/java/README.md
- confidence: cited
  path: adapters/java/src/codespec/read/Reader.java
- confidence: inferred
  path: adapters/java/test/codespec/read/ReaderTest.java
- confidence: inferred
  path: adapters/java/testdata/nullable
- confidence: inferred
  path: adapters/java/testdata/nullable/calls.json
- confidence: inferred
  path: adapters/java/testdata/nullable/driver/Main.java
- confidence: cited
  path: adapters/jvm
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T17:43:51Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-11-06T17:43:51Z", actor: "human:timo", revision: 7}
- {from: "active", to: "implemented", at: "2026-11-06T21:54:56Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1}}}
---
# Story: A JVM reference field with no initializer reads as a nil pointer

## Outcome

A JVM reference field that some constructor leaves unassigned, the exact set `nullableFields`
flags today (`adapters/java/src/codespec/read/Reader.java:668-696`: an instance field of a
reference type, no initializer, not assigned first by every constructor), reads as `ptr(T)` with
zero value `nil` instead of being refused. Every other field reads as it does today. It uses the
existing `Type::Ptr` and `Expr::Nil` of `crates/model/src/lib.rs` (lines 105-107 and 389-391); a
comparison of such a field with `null` reads as a comparison with `nil`. The Go writer writes `*T`
and nil checks; the Rust writer writes them as `story:rust-writer-nil-pointer` decides.

This story covers the Java reader and the shared mapping in `adapters/jvm/` only. The Groovy side
is not here: `story:grails-artefacts` wires Grails domain properties to the shared rule, and until
it lands `story:groovy-reader-core` refuses an uninitialised reference property of a plain class
with `type: …`.

## Context

- Today the Java reader refuses each such field: "field <name> holds null until assigned, which no
  cir/1 zero value is" (`Reader.java:496-510` and the class comment at `:110-112`). Grails domain
  classes mostly have reference properties without initializers, so `story:grails-artefacts`
  refuses most of them unless this changes; the design critic asked for this decision
  (`review-result:groovy-design-round-1`).
- Decision taken: `ptr(T)` with `nil`, not a reader assumption and not a new nullable `cir/1` type.
  No `cir/1` change: `Type::Ptr` and `Expr::Nil` exist.
- The rule (a reference type left unassigned maps to `ptr(T)`) goes in the shared class of
  `story:jvm-type-mapping-shared`, so a Groovy reader can call it without a second copy; which
  fields are unassigned stays each reader's analysis.
- A use of the field that would print or compute with Java's `null` (string concatenation,
  `String.format`, arithmetic on an unboxed value) is refused at the use, since the written
  program would not print `null` as Java does.
- The Go writer already has `ptr` and `nil` cases (`adapters/go/cmd/codespec-write-go/write.go:274`,
  `:402`, `:915`); this story adds what a `*T` of a primitive-mapped `T` (`*string`, `*int32`) needs
  there, with tests.

## Domain relations

- JVM reference field left unassigned by some constructor -> `cir/1` field of type `ptr(T)`,
  one-to-one, zero value `nil`: inferable (stated by the operator decisions of 2026-11-06 recorded
  in this story; inferred from `Reader.java:668-696` and `crates/model/src/lib.rs:105-107`,
  `:389-391`; no `ess/1` document, the repository opted out of ESS, `AGENTS.md` section ESS).

## Acceptance

- `adapters/java/testdata/nullable/` (new) holds a Java class with an uninitialised `String` field
  and a null check of it; a `ReaderTest` case reads it with 0 refusals, the field typed
  `ptr(string)` and the check a comparison with `nil`.
- The fixture runs equal in Java, Go and Rust, as steps of `task adapters-check`:
  `codespec read --lang java adapters/java/testdata/nullable --out target/nullable/read.json`;
  `codespec pass target/nullable/read.json --out target/nullable/pass.json`;
  `codespec write --lang go` and `codespec write --lang rust` of it into `target/nullable/go` and
  `target/nullable/rust`; `codespec-run-go` and `codespec-run-rust` (`story:runners-and-manifests`)
  on each with the fixed call list `adapters/java/testdata/nullable/calls.json`, checked in beside
  the fixture (at least: the check on a fresh object, the check after assignment, a read after
  assignment); the Java original run on the same list by
  `adapters/java/testdata/nullable/driver/Main.java`; then `cmp` of each output with the Java run.
- `codespec.read.ReaderTest`'s case
  `adversary_a_reference_field_left_at_its_null_default_is_not_read_as_a_zero_value` passes
  unedited: no field of `adapters/java/testdata/adversary/null-field/Label.java` reads with a zero
  value, because each is now `ptr`, and the concatenation `"text=" + text` is refused at its use.
- `examples/ledger/ledger.read.cir.json` and every other example's checked-in outputs stay
  byte-identical: `task examples-check` and `task examples-ledger` pass with no change under
  `examples/`.
- `adapters/java/README.md`'s Assumptions text is updated: a reference field some constructor
  leaves unassigned is a nil pointer, not a refusal, and a use that would print `null` is refused.

## Scope

- `adapters/jvm/`: the `ptr(T)` rule for an unassigned reference field, with a test.
  `story:groovy-reader-core` may also add a mapped type here; whichever lands second rebases.
- `adapters/java/src/codespec/read/Reader.java`: `nullableFields`' set becomes `ptr` fields instead
  of refusals; null comparisons on them read as `nil` comparisons.
- `adapters/java/README.md`: the Assumptions text.
- `adapters/java/testdata/nullable/` (new: the fixture, `calls.json` and `driver/Main.java`) and
  `adapters/java/test/codespec/read/ReaderTest.java` (the new case).
- `adapters/go/cmd/codespec-write-go/`: `write.go` and `write_test.go` for `*T` and nil checks;
  not `main.go`, whose manifest is `story:runners-and-manifests`'.
- `Taskfile.yml`: the coordinator, the unit writes a patch.
