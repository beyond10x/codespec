---
format: aep.planning-md/3
id: epic:reader-coverage
kind: epic
status: active
title: Reader coverage of the conformance kit
summary: Go, Rust, Java and C readers read close to all of the 120 kit features a reader can claim; the Quarkus quickstart reads with at most 10 refusals.
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T02:03:37Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":5}}}
- {from: "proposed", to: "active", at: "2026-11-10T02:03:38Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":5}}}
---
# Epic: Reader coverage of the conformance kit

## Outcome

The Go, Rust, Java and C readers each read close to all of the 120 kit features a reader can
claim, and the Java reader reads the pinned Quarkus quickstart with fewer than half of today's
refusals. One command prints the number per reader, and a floor in `task check` holds it.

## Why Now

Requested by the operator on 2026-11-10: "readers for quarkus+c+golang+rust close to 100%".
Measured that day from the `read` columns of `website/data/support-matrix.json` (verified of the
120 applicable features; `fact.*` is a writer feature only, `docs/conformance.md` § 5): Go 114,
Rust 89, Java 77, C 59. `story:go-reader-full-coverage` (implemented) used the same measure. The
Quarkus quickstart of `adapters/java/README.md` reads with 25 refusals in 15 functions.

## Done When

- The eight stories that decompose this epic are implemented.
- Verified of 120, each reader report with `fail` 0 and `conforms: true`: Go 115, Rust at least
  105 (ceiling 109), Java at least 100 (ceiling 107), C at least 100 (ceiling 108). Every feature still unclaimed is listed with its reason
  in the reader's documentation.
- The Quarkus quickstart reads with at most 10 refusals.

## Known gaps this epic leaves, each a draft story

- `story:reader-check-externs`: extern calls of a non-Go reader cannot run at step 4 of the reader
  check (`expr.extern`, `expr.extern_method`, `type.extern`: 3 features for Rust, Java and C).
- `story:c-interfaces-generics-closures`: `codespec-write-c` refuses interfaces, type parameters
  and closures, so C has no form for 8 features.
- No source form in the language, so not reachable by its reader: Rust `unop.plus`,
  `decl.variadic`, `expr.spread`, `decl.embedded`; Java `assign.and_not`, `op.and_not`,
  `decl.newtype`, `sem.multi_assign.order`, `stmt.send`, `type.chan`, `unop.recv`, `stmt.var`,
  `unop.deref` (`adapters/java/README.md`). `story:reader-coverage-floor` prints the count against
  the reachable features beside the count of 120.
- `stmt.foreign` for Rust, Java and C: the Go writer refuses a foreign statement that is not Go,
  and the evaluator stops at one.
- Go: `decl.union`, `expr.zero`, `intrinsic.sorted_keys`, `type.unit` have no Go form and
  `stmt.lookup_else` is written by a pass (`crates/conformance/src/lib.rs` § Cases written by
  hand); 115 is every feature Go source reaches.
- Quarkus: 8 of the refusals are throws of an exception from a jar; `cir/1` has no statement that
  leaves a function by an exception.

## Out of Scope

- The Groovy reader's own coverage, the writers and the runners, except what a story names.
- A change of `cir/1`.
