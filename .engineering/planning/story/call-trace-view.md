---
format: aep.planning-md/3
id: story:call-trace-view
kind: story
status: implemented
title: A call tree from one function, with branches and syscalls, on the documentation site
relations:
- decomposes: epic:whole-repo-to-syscalls
- depends_on: story:syscall-barrier
- depends_on: story:dispatch-over-set
- depends_on: story:reachability
- depends_on: story:library-syscall-map
scope:
- confidence: cited
  path: crates/cli/src/lib.rs
- confidence: cited
  path: crates/cli/src/main.rs
- confidence: cited
  path: crates/cli/tests/trace.rs
- confidence: cited
  path: crates/docs/src/
- confidence: cited
  path: crates/effects/
- confidence: cited
  path: website/docs/
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T02:00:59Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "proposed", to: "active", at: "2026-11-08T02:00:59Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "active", to: "implemented", at: "2026-11-08T06:43:46Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":4}}}
---
# Story: A call tree from one function, with branches and syscalls, on the documentation site

## Outcome

`codespec trace <documents…> --from <path.Func>` writes `codespec-trace/1`: the call tree from one
function, each `if`, `switch` and loop a branch node with its condition, each call expanded once
(a repeat is a reference), dispatch nodes listing candidates, syscalls at the leaves. The
documentation site draws it for `examples/envcheck`.

## Context

- `docs/static-analysis.md` § 5c. `crates/docs/src/diagram.rs` already draws one function's
  body; this draws across functions.
- Depth is bounded by `--depth` (default 8); a cut is a node saying so.

## Acceptance

- `crates/effects/tests/trace.rs`: a fixture function with `if x > 0`, a `switch` with two cases and
  a `for` loop gives one branch node of each kind, each carrying its condition text (`x > 0`, the
  case values, the loop condition); the `crates/cli/tests/fixtures/readfile/` function of
  `story:syscall-barrier` gives a tree with an `openat` syscall leaf.
- Same file: envcheck `Announce` gives a
  tree with its `if` as a branch node and `os.Getenv` and `println` as leaves.
- `crates/cli/tests/trace.rs`: on a fixture with an interface call, a function called twice and a
  chain deeper than `--depth 2`, the `codespec-trace/1` output has one dispatch node listing both
  candidates, one reference node for the repeat, and one cut node.
- `crates/docs` test: the generated page holds the tree for envcheck; `task docs-check` passes.
- `task check` passes.
