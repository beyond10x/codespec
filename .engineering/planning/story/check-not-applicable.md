---
format: aep.planning-md/3
id: story:check-not-applicable
kind: story
status: draft
title: A check with no name for the set's language says it is not applicable
summary: 'Defect: a check whose lists name nothing of the set''s language gives 0 rows silently; it reports not applicable instead.'
relations:
- decomposes: epic:codebase-transformations
- depends_on: story:reach-entry-file
scope:
- confidence: inferred
  path: crates/check/src/checks.rs
- confidence: inferred
  path: crates/check/src/lib.rs
- confidence: inferred
  path: crates/check/tests/checks.rs
- confidence: inferred
  path: crates/cli/src/main.rs
- confidence: inferred
  path: crates/cli/tests/check.rs
revision: 3
---
# Story: a check with no name for the set's language says it is not applicable

## Outcome

`codespec check <check>` over documents of a language that the check's name lists name nothing
of reports the check as not applicable instead of 0 rows: it prints
`<check>: not applicable to <language>` on standard error, and its normal output says so too (the
text report has the line `not applicable to <language>` in place of rows, the JSON report a field
`not_applicable` naming the language), so a caller reading only the rows or only the summary sees
it. Exit 0. The decision is per check and per language of the set, from the lists themselves and
`--names`: a list given with `--names` that names something of the language makes the check
applicable. It holds for every language and every check whose name lists are language-bound;
a check with no name list (one on shapes alone) is always applicable.

## Why

Defect (silent false negative), split out of `story:check-java-name-lists` because it is
language-independent and has its own test. Reported by the consumer of a Java service: 17 of 18
built-in checks give 0 rows on its Java documents (codespec 0.12.0), indistinguishable from a
clean result. The same happens for any language a check's lists do not cover.

## Reproducer

The Java reproducer of `story:check-java-name-lists`, read with `codespec-read-java`, and
`codespec check insecure-random <documents>` on main `5f82fff97`: exit 0, no row, no line on
standard error.

## Done When

- A failing test in `crates/check/tests/checks.rs` first: a check whose lists name only Go names,
  run over a `cir/1` set of Java documents, returns a report marked not applicable to `java` with
  no rows; the same check with `--names` naming a Java function is applicable and gives its row.
- `crates/cli/tests/check.rs`: `codespec check <check>` over that set prints
  `<check>: not applicable to java` on standard error, exit 0; the text output has the
  `not applicable to java` line and `--format json` has `not_applicable: "java"`; neither has
  0 rows as its only content.
- Over a Go set the same check is applicable and its output has no such line (a test).
- The rule is not Java's: the same check over a set of Rust documents read by
  `codespec-read-rust` reports not applicable to `rust` (a test).
- A check with no name list is always applicable: `unsigned-below-zero` (the one built-in check
  with `lists: &[]` on main `5f82fff97`) over the Java set prints no not-applicable line on
  standard error, its text output has none, and its JSON report has no `not_applicable` field
  (a test).
- The crate doc of `b10x-codespec-check` states the result; `task check` green.

## Scope

- Own: `crates/check/src/lib.rs` (the report and the applicability test),
  `crates/check/src/checks.rs`, `crates/check/tests/checks.rs`, `crates/cli/src/main.rs` (the
  standard-error line), `crates/cli/tests/check.rs`. Inferred.
- After `story:reach-entry-file` (same `checks.rs`, `main.rs`, `tests/check.rs`), before
  `story:check-java-name-lists`.
- `CHANGELOG.md` gets one line; conflicts there are resolved at merge.
