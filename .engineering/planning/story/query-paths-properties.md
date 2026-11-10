---
format: aep.planning-md/3
id: story:query-paths-properties
kind: story
status: implemented
title: cq path patterns and typestate properties
relations:
- decomposes: epic:codebase-transformations
- depends_on: story:query-core
scope:
- confidence: inferred
  path: crates/query
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T01:24:37Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-11-10T01:24:37Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-11-10T17:52:00Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
---
# Story: cq path patterns and typestate properties

## Outcome

cq has its two forms of sugar, compiled into rules: `path(A -> B, avoid: P)` over control flow or
the call graph with a witness, and properties, small automata over the calls of a body or a call
path (`docs/query-language.md` § 5.2 and § 5.3).

## Acceptance

- A path query over control flow and one over the call graph, each on a checked-in fixture, return
  one witness path per match, and none when every path passes through the `avoid` node; the
  output equals checked-in expected files.
- A property over the calls of a body, "Lock, then Unlock on every path to exit", reports the one
  function of a fixture that returns early without unlocking and none of the others; a property
  over a call path reports the one entry function of a fixture whose callee chain breaks the
  order. Both equal checked-in expected files.
- Each example of `docs/query-language.md` § 6 that uses only § 5.1 to § 5.3 runs on a checked-in
  fixture and prints a checked-in expected file with at least one row, byte-equal across two runs.

## Scope

- Own: `crates/query` (after `story:query-core`). Inferred.
- Shared with the unit running beside it in the other slot (`crates/eval` stories run beside
  `crates/query` stories): `crates/cli/src/lib.rs` and `main.rs` (each story adds only its own
  subcommand variant and its own module file), the `members` line of `Cargo.toml`, `crates/cli/Cargo.toml` (its own dependency line), `Cargo.lock` (never merged by hand: the coordinator lets `cargo build` rewrite it on the merged head), `CHANGELOG.md`
  (each adds its own entry), `AGENTS.md` (its own table row). The coordinator resolves these at
  merge and regenerates `website/docs/reference/{cli,crates}.md` and `website/data/status.json`
  with `task docs-generate` on the merged head; a unit does not commit generated site files.
