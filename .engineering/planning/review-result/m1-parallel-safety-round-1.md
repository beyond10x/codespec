---
format: aep.planning-md/3
id: review-result:m1-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic, round 1 (M1 decomposition)
relations:
- reviews: story:passes-and-facts
- reviews: story:go-writer
- reviews: story:rust-writer
- reviews: story:harness
revision: 1
---
needs-revision
story:go-writer — its outcome reads `examples/orders/orders.cir.json`, which `story:passes-and-facts` creates, and no `depends_on` edge runs from go-writer to passes-and-facts, so one wave would hold both; add the edge or merge the two (cited, and `examples/orders/` holds only `orders.read.cir.json` today) — .engineering/planning/story/go-writer.md:16, .engineering/planning/story/passes-and-facts.md:16
story:go-writer — its acceptance edits `task examples-check` in `Taskfile.yml`, but its Scope omits that file, and `story:passes-and-facts` (and `story:rust-writer`) edit the same task without saying so (cited); either order them on this shared file or split the task so each story owns its own check — .engineering/planning/story/go-writer.md:21, .engineering/planning/story/passes-and-facts.md:29
story:rust-writer — its acceptance edits `task examples-check` and its outcome needs `codespec-write-rust` built into `target/adapters` by the `adapters` task, yet its Scope names no `Taskfile.yml`, so it collides unnamed with `story:go-writer`, which can run beside it because only passes-and-facts is a `depends_on` edge (cited); either order the two on the shared file or split the Taskfile tasks — .engineering/planning/story/rust-writer.md:24, .engineering/planning/story/go-writer.md:21

What I read: 5 artifacts (the epic and 4 stories) via `aep plan artifact show`, plus `aep plan artifact graph` and `relations`. In the tree I read `Taskfile.yml`, `Cargo.toml`, `crates/cli/src/main.rs`, `adapters/go/` and `examples/orders/`. Surfaces: 4 cited, 0 inferred, 0 unplaced. Acceptance reads traced to a producer in the set: 4, of which 3 have a `depends_on` edge (rust-writer to passes, harness to go-writer, harness to rust-writer). The untraced one is go-writer to passes, the first finding.

What I could not establish, out of my lane or unclear:
- `story:rust-writer` says a Rust replacement for the `foreign` statement may be recorded "in `examples/orders`" without naming a file. If it edits `orders.cir.json`, it would also collide with passes-and-facts. The surface is unclear, so it is not a finding.
- `story:go-writer` says "shared decode of `cir/1`" in `adapters/go/`. That may move files out of M0's `cmd/codespec-read-go/ir.go`, which is outside this set, so I did not assess it.
- `story:passes-and-facts` and `story:harness` both change `crates/cli/src/main.rs` and `Cargo.toml` members. `story:rust-writer` also touches the workspace through `adapters/rust` and `Cargo.lock`. These are ordered by the chain harness → rust-writer → passes, so they do not collide.

```findings
- file: .engineering/planning/story/go-writer.md
  line: 16
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its outcome reads examples/orders/orders.cir.json, which story:passes-and-facts creates (passes-and-facts.md:16), and no depends_on edge runs from go-writer to passes-and-facts, so one wave would hold both; add the edge or merge the two"
- file: .engineering/planning/story/go-writer.md
  line: 21
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance edits task examples-check in Taskfile.yml but its Scope omits that file, and story:passes-and-facts (passes-and-facts.md:29) and story:rust-writer edit the same task without saying so; order them on the shared file or split the task so each story owns its own check"
- file: .engineering/planning/story/rust-writer.md
  line: 24
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance edits task examples-check and needs the adapters task to build codespec-write-rust, but its Scope names no Taskfile.yml, so it collides unnamed with story:go-writer (go-writer.md:21), which can run beside it; order the two on the shared file or split the Taskfile tasks"
```
