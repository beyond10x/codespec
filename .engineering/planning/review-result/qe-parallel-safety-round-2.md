---
format: aep.planning-md/3
id: review-result:qe-parallel-safety-round-2
kind: review-result
status: active
title: Parallel-safety critic, round 2 (query and evaluator)
relations:
- reviews: story:query-core
- reviews: story:query-paths-properties
- reviews: story:cir1-defer-receiver-root
- reviews: story:eval-core
- reviews: story:eval-conformance-runner
- reviews: story:eval-handlers-traces
- reviews: story:eval-fault-plans
revision: 1
---
needs-revision
story:query-core — this story and story:eval-core each add a new crate, so each must change the committed root `Cargo.lock` (every task runs `--locked`) and the `[dependencies]` block of `crates/cli/Cargo.toml`, yet the Shared list at line 60 (identical in all six sibling bodies) names `lib.rs`, `main.rs`, the `members` line, `CHANGELOG.md` and `AGENTS.md` but not `Cargo.lock` or `crates/cli/Cargo.toml`; the Scope should name both and say who resolves them (ordering edge or a coordinator rule, not chosen here). `Cargo.lock` is cited; the `crates/cli/Cargo.toml` dependency lines are inferred from the CLI calling the new crate — .engineering/planning/story/query-core.md:60, .engineering/planning/story/eval-core.md:34, Cargo.lock:95

What I read: 8 artifacts. I ran `aep plan artifact show` on the 7 stories and on `review-result:qe-parallel-safety-round-1`, plus `aep plan artifact graph`. I also ran `git ls-files`, `grep` and `sed` on `Cargo.toml`, `crates/cli/Cargo.toml`, `Cargo.lock`, `AGENTS.md`, `Taskfile.yml`, `.github/workflows/release-build.yml`, `crates/docs/src/*.rs` and `docs/{evaluator,query-language}.md`. Surfaces: 7 placed and 0 unplaceable. The Own surfaces of every story are tagged Inferred by the bodies themselves, and the shared files and the generated-file list are cited. I traced 7 acceptance reads to a producer in the set, and a `depends_on` edge records all 7. That count includes query-paths-properties on query-core and eval-fault-plans on eval-handlers-traces. It also includes cir1-defer-receiver-root on query-core (via query-paths-properties), eval-core, eval-conformance-runner and eval-fault-plans. The slot ordering and the "runs alone" ordering are carried by edges.

Round 1 findings that are fixed:
- The shared files are now named in all six bodies.
- cir1 now has edges into every other story.
- cir1 now lists its generated files.
- The three-way conformance collision is resolved by the edges.

What I could not establish:
- Every Own surface is inferred from the drafter's notes, not recorded scope.
- Whether `[workspace.dependencies]` in the root `Cargo.toml` gains a new third-party entry from either crate. I did not guess.
- Out of my lane: both slots reuse the `<module>.<funckey>#<index path>` node ids of § 4.1. `docs/evaluator.md` says the evaluator reuses them, and this set does not say whether `crates/eval` and `crates/query` share code for them or compute them separately. That is a design question.

```findings
- file: .engineering/planning/story/query-core.md
  line: 60
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "this story and story:eval-core each add a new crate, so both change the committed root Cargo.lock (tasks run --locked) and the dependencies of crates/cli/Cargo.toml, and the Shared list names neither file nor says who resolves them; name both and an ordering edge or a coordinator rule (Cargo.lock cited; crates/cli/Cargo.toml dependency lines inferred)"
```
