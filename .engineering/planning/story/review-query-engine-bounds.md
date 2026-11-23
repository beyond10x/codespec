---
format: aep.planning-md/3
id: story:review-query-engine-bounds
kind: story
status: implemented
title: 'Query engine: bounded call paths, finite recursion, graph membership'
summary: A bound reaches_path builds only the witnesses asked for, a value-making recursion is refused, a path of no edge needs its node in the graph
relations:
- decomposes: epic:maintainer-review-2026-11-23
scope:
- confidence: cited
  path: crates/query/src/engine.rs
- confidence: cited
  path: crates/query/tests/paths.rs
- confidence: cited
  path: crates/query/tests/query.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T03:58:00Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-23T03:58:01Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-23T04:20:59Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

The query engine of `b10x-codespec-query` builds only the call-path witnesses a query asks for,
refuses a recursion that could derive values without end, and gives a path of no edge only to a
node of the graph.

## Findings

- Q2: a `reaches_path` scan or negation with an end bound walks from the bound start (or from the
  callers of the bound end) and builds witnesses for the pairs asked for only.
  Test: `crates/query/src/engine.rs::tests::a_bound_reaches_path_builds_only_the_witnesses_it_asks_for`
  (counts the functions held by the witnesses built; the same on chains of 40 and 120 functions).
- Q10: a recursive rule that puts into its head a value made by arithmetic or a builtin from a
  value of the recursion is refused when the program is compiled.
  Test: `crates/query/tests/query.rs::a_recursion_that_makes_new_values_is_refused`.
- Q11: a path of no edge needs its node in the graph (a node of the control flow, or a function
  declared or linked by a call); a declared isolated function keeps its path of one node.
  Test: `crates/query/tests/paths.rs::a_path_of_no_edge_needs_its_node_in_the_graph`.

## Done When

- The three tests fail on the code of 0.10.0 and pass now.
- `cargo test -p b10x-codespec-query --locked` and clippy with `-D warnings` are green.
