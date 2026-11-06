---
format: aep.planning-md/3
id: review-result:m1-design-round-1
kind: review-result
status: active
title: Design critic, round 1 (M1 decomposition)
relations:
- reviews: story:passes-and-facts
- reviews: story:go-writer
- reviews: story:rust-writer
- reviews: story:harness
revision: 1
---
needs-revision
story:go-writer — its outcome and acceptance read `examples/orders/orders.cir.json`, which only story:passes-and-facts creates, and no `depends_on` edge runs from story:go-writer to story:passes-and-facts; add that edge — `.engineering/planning/story/go-writer.md:16`, `.engineering/planning/story/passes-and-facts.md:16`
story:rust-writer — the orders Rust output is required but `orders.go:132` stays `foreign` and the writer refuses `foreign`, and the body leaves "a Rust replacement" to an undefined "story's own decision" that no item in the set owns, so the Rust output the harness depends on cannot exist; name which item supplies the Rust form (a pass in story:passes-and-facts, or an explicit decision here) — `.engineering/planning/story/rust-writer.md:27`, `.engineering/planning/epic/go-and-rust-writers-with-harness.md:40`

**What you read**: 5 artifacts, using `aep plan artifact show` on each, plus `relations`, `graph` and `validate` (valid). I also read `docs/design.md` §§ 1–5, `Taskfile.yml`, `crates/cli/src/main.rs` and `examples/orders/source/orders.go`.

**What you could not establish**:
- I walked 7 `depends_on` edges across the whole graph, including the three sibling epics outside the set, and found no cycle. The chain is not serialising: passes → rust-writer → harness, with go-writer a parallel branch.
- I traced 5 acceptance reads to a producer in the set. 4 are recorded by an edge: harness reads both writers' output, rust-writer reads the facts, and the harness's driver read of the Rust output is covered by the same edge. The 1 unrecorded read is the go-writer one in the first finding.
- `codespec write` already exists in `crates/cli/src/main.rs`, so neither writer story has a missing CLI half.
- Whether the harness's "original" implementation has a driver is an acceptance question, out of my lane. Both writers editing `Taskfile.yml` `examples-check` is a parallel-safety question, out of my lane.

```findings
- file: .engineering/planning/story/go-writer.md
  line: 16
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its outcome and acceptance read examples/orders/orders.cir.json, which only story:passes-and-facts creates, and no depends_on edge runs from story:go-writer to story:passes-and-facts; add that edge"
- file: .engineering/planning/story/rust-writer.md
  line: 27
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the orders Rust output is required but orders.go:132 stays foreign and the writer refuses foreign, and the body leaves 'a Rust replacement' to an undefined 'story's own decision' that no item in the set owns, so the Rust output the harness depends on cannot exist; name which item supplies the Rust form"
```
