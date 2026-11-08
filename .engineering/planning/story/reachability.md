---
format: aep.planning-md/3
id: story:reachability
kind: story
status: implemented
title: codespec reach lists the functions no entry point reaches
relations:
- decomposes: epic:whole-repo-to-syscalls
- depends_on: story:link-modules
- depends_on: story:dispatch-over-set
scope:
- confidence: cited
  path: crates/cli/src/lib.rs
- confidence: cited
  path: crates/cli/src/main.rs
- confidence: cited
  path: crates/cli/tests/reach.rs
- confidence: cited
  path: crates/effects/
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T02:00:58Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-11-08T02:00:58Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-11-08T05:32:46Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
# Story: codespec reach lists the functions no entry point reaches

## Outcome

`codespec reach <documents…> --entry <path.Func>…` prints `codespec-reach-report/1`: every function
of the set reached from the entry points (calls, dispatch candidates, function values) and every
function not reached, per module. That list is the tree-shaking result and the dead-code report.

## Context

- `docs/static-analysis.md` § 5c. Default entries for Go: `main.main`, every `init`, and with
  `--exported` every exported function of the named modules.
- Shares the linked call graph with `crates/effects` (`story:link-modules`,
  `story:dispatch-over-set`); the graph is moved to one function both use.

## Acceptance

- `crates/effects/tests/reach.rs`: a set with an unreached private function lists it unreached;
  a function only referenced as a value is reached.
- Same file: a function that is a candidate of an interface call from an entry is reached; the
  report groups functions by module `path`.
- `crates/cli/tests/reach.rs`: on a two-module fixture with one unreached function, the output
  names exactly that function as unreached, and is byte-identical across two runs.
- `task check` passes.
