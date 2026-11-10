---
format: aep.planning-md/3
id: story:query-core
kind: story
status: implemented
title: 'cq core: a fact base, an interpreter and codespec query'
relations:
- decomposes: epic:codebase-transformations
- supersedes: story:cir-patterns
scope:
- confidence: inferred
  path: crates/query
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T01:24:36Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":7}}}
- {from: "proposed", to: "active", at: "2026-11-10T01:24:37Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":7}}}
- {from: "active", to: "implemented", at: "2026-11-10T17:51:59Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":9}}}
---
# Story: cq core: a fact base, an interpreter and `codespec query`

## Outcome

`codespec query` answers a question over a set of `cir/1` documents written in cq, the typed
Datalog with named-field record patterns of `docs/query-language.md` § 3 to § 5.1 and § 7: a new
crate `crates/query` builds the fact base of § 4 from the documents and codespec's own graphs and
evaluates rules with recursion, stratified negation and aggregates.

## Context

- Design: `docs/query-language.md` (status proposed, 2026-11-09), plan item 1 of its § 9.
- It supersedes `story:cir-patterns` (`codespec match`), whose pattern-by-example form the
  design rejects as the core (§ 3).

## Acceptance

- `codespec query --schema` prints every relation of the fact base with its fields, equal to a
  checked-in expected file.
- On a checked-in fixture, `codespec query -f <file> <documents…>` and the same query given
  inline print the same rows, equal to a checked-in expected file; `--explain` on that query
  prints the rules in the order of their strata, equal to a checked-in expected file.
- Each built-in check of `story:builtin-defect-checks` (the 8 named in
  `docs/query-language.md` § 9 item 1) is either written in cq, checked in, and gives the same
  findings as `codespec check` on that check's hit and miss fixtures under
  `crates/check/tests/fixtures`, or is named in a checked-in list with the design section it
  waits for (§ 5.2, § 5.3 or § 8); a test in `task check` asserts one or the other for all 8.
  `lock-without-unlock` is in the list (it needs § 5.3 and § 8); a check that needs none of the
  three sections may not be in the list.
- Every node has the id `<module path>.<function key>#<index path>` of § 4.1; two runs over the
  same documents print byte-equal output.
- The fact base is cached as `<dir>/.cq/<digest>.facts` (§ 4): a second run reads the cache and
  prints byte-equal output; a changed document gives another digest. Both asserted by a test.
- On a checked-in fixture where a negated query ("no call to X reachable") depends on a function
  with no body, the output row carries the certainty mark `possible` and names the unknown site
  (§ 4.3); the same query on the fixture with that body present prints the row without the mark.
  Both equal checked-in expected files.
- A path pattern or a property (§ 5.2, § 5.3) in a query is refused with an error naming the
  construct.

## Out of Scope

- Path patterns and properties (`story:query-paths-properties`), the `defer` statement
  (`story:cir1-defer-receiver-root`), taint, source-language patterns, rewrites.
- The generated schema page on the documentation site that § 4 names: left out of this story.
- The run on google/capslock that § 9 item 1 also names: it needs a checkout that is not in this
  repository; record it as not run.

## Scope

- Own: `crates/query` (new, `b10x-codespec-query`), `crates/cli/src/query.rs` (new). Inferred.
- Shared with the unit running beside it in the other slot (`crates/eval` stories run beside
  `crates/query` stories): `crates/cli/src/lib.rs` and `main.rs` (each story adds only its own
  subcommand variant and its own module file), the `members` line of `Cargo.toml`, `crates/cli/Cargo.toml` (its own dependency line), `Cargo.lock` (never merged by hand: the coordinator lets `cargo build` rewrite it on the merged head), `CHANGELOG.md`
  (each adds its own entry), `AGENTS.md` (its own table row). The coordinator resolves these at
  merge and regenerates `website/docs/reference/{cli,crates}.md` and `website/data/status.json`
  with `task docs-generate` on the merged head; a unit does not commit generated site files.
