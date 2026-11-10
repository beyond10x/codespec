---
format: aep.planning-md/3
id: review-result:reach-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic on the reach epic, round 1
relations:
- reviews: epic:sound-reach
- reviews: story:reach-three-valued
- reviews: story:reader-indirect-call-sites
- reviews: story:reach-entry-file
- reviews: story:reach-at-positions
- reviews: story:reach-registered-candidates
- reviews: story:reach-framework-entries
revision: 1
---
needs-revision

story:reader-indirect-call-sites — shares `crates/model/src/lib.rs` (the `Stmt::Foreign` field, line 1246) and `schemas/cir-1.schema.json` with story:cir1-raise-out-of-entry, story:cir1-fields-of-imported-types and the active story:cir1-defer-receiver-root, and the body never mentions any of them; the `cir/1`-versus-`cir/2` decision in its first step also decides whether those three stay on `cir/1` (cited: reader-indirect-call-sites.md:46, cir1-raise-out-of-entry.md:45, cir1-defer-receiver-root.md scope line `crates/model/src/lib.rs`; cir1-fields-of-imported-types names no scope line but its acceptance edits `crates/model` and `adapters/java`, so that part is inferred). The fix is either `depends_on` edges that record the shared model and schema files as the reason, or splitting the surface so the `indirect` field lands apart from the other three — .engineering/planning/story/reader-indirect-call-sites.md:29,46

story:reader-indirect-call-sites — shares `adapters/c-read`, `adapters/rust-read`, `adapters/go/cmd/codespec-read-go`, `docs/design.md` and `CHANGELOG.md` with story:reader-function-spans, and neither body mentions the other. This story waits only on readers-bound-untrusted-input (wave 4), function-spans sits in wave 6, so both can run in the next wave (cited, both Own lines). Either order them with an edge that records the shared adapters, or split the surface — .engineering/planning/story/reader-indirect-call-sites.md:46-50, .engineering/planning/story/reader-function-spans.md:62

story:reader-indirect-call-sites — the surface is named so widely that it forbids concurrency: the model crate, `schemas/`, five adapters, `crates/effects`, `docs/design.md` and `CHANGELOG.md` in one item. It collides with every reader story in the store and its own parenthetical "one reader at a time may be split off" leaves that split undone. It is also ordered only by the prose line "the reader stories of `epic:reader-coverage`", and the only recorded edge is `readers-bound-untrusted-input`; story:reader-go-kit (`adapters/go/cmd/codespec-read-go`) and story:reader-check-externs have no edge, so the prose order is not enforced — .engineering/planning/story/reader-indirect-call-sites.md:46-51

story:reach-three-valued — it lands on `crates/cli/src/lib.rs` and `crates/cli/src/main.rs`, which is where `codespec reach` lives (`Cmd` enum and the dispatch at main.rs:312), with story:mcp-server-read-tools (the `mcp` subcommand), story:reader-function-spans (`read --spans`) and its own successors reach-entry-file, reach-at-positions and reach-framework-entries. Only the in-set order is recorded; the body says nothing of mcp-server-read-tools, and that story's body says nothing of reach. The fix is an ordering edge naming `crates/cli/src/lib.rs` and `main.rs`, or splitting the subcommand registration out (cited: reach-three-valued.md:47, mcp-server-read-tools.md:56, `git grep reach crates/cli/src`) — .engineering/planning/story/reach-three-valued.md:47

story:reach-entry-file — it shares `crates/effects/src/lib.rs` (where `reach` is defined, line 681) with story:reader-indirect-call-sites, story:reach-at-positions and story:reach-registered-candidates. The only edge is to reach-three-valued, so entry-file and indirect-call-sites are unordered and the body says "same files" only of three-valued. The epic's numbered list gives an order, but it is prose and not an edge, so nothing in the store says so. The fix is an edge, or splitting the file (cited, Own lines) — .engineering/planning/story/reach-entry-file.md:35-37

story:reach-at-positions — it shares `crates/effects/src/lib.rs` with story:reader-indirect-call-sites and story:reach-registered-candidates, and `crates/cli/src/lib.rs` with story:reach-framework-entries (no edge between at-positions and either). It also shares `crates/cli` and `CHANGELOG.md` with story:reader-function-spans, which it names only as a data source ("where that is implemented") and not as a collision. The body names none of these pairs. The fix is an edge, or splitting the file (cited) — .engineering/planning/story/reach-at-positions.md:32-34

story:reach-registered-candidates — its only edge is to indirect-call-sites, so it is unordered against story:reach-entry-file and story:reach-at-positions, all three of which edit the reach pass in `crates/effects/src/lib.rs`. It also touches `crates/effects` with story:cir1-raise-out-of-entry ("Shared: crates/effects") and story:reach-three-valued. The body says none of this. The fix is an edge, or splitting the file (cited, Own lines; the `lib.rs` file is from `git grep "pub fn reach" crates`) — .engineering/planning/story/reach-registered-candidates.md:33

story:reach-three-valued — all six new stories carry an empty store `scope`, and `aep plan artifact waves` prints each as `unassessed` and never places it. The surfaces exist only as body prose marked "Inferred", so waves cannot detect or separate any collision above, including the in-set `crates/effects` and `crates/cli` pairs. Entering each story's Own lines as `scope` (the other stories in the store do this) would let waves keep these pairs apart — `aep plan artifact waves` lines 1549-1556 (unassessed: all six reach stories), .engineering/planning/story/reach-three-valued.md:45-49

```findings
- file: .engineering/planning/story/reader-indirect-call-sites.md
  line: 29
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "shares crates/model/src/lib.rs (Stmt::Foreign) and schemas/cir-1.schema.json with story:cir1-raise-out-of-entry, story:cir1-fields-of-imported-types and the active story:cir1-defer-receiver-root, names none of them, and its cir/1-or-cir/2 decision binds the other three; the model-side overlap with cir1-fields-of-imported-types rests on an inferred surface (that story names no scope line); add depends_on edges recording the shared file, or split the surface"
- file: .engineering/planning/story/reader-indirect-call-sites.md
  line: 46
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "shares adapters/c-read, adapters/rust-read, adapters/go/cmd/codespec-read-go, docs/design.md and CHANGELOG.md with story:reader-function-spans and neither body mentions the other, so both can be scheduled in one wave; add an edge recording the shared adapters or split the surface"
- file: .engineering/planning/story/reader-indirect-call-sites.md
  line: 46
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the surface is model, schemas, five adapters, effects and docs in one item, so it collides with every reader story; the reader-coverage ordering is prose only and story:reader-go-kit and story:reader-check-externs have no depends_on edge; narrow it by splitting one reader per unit, as its own body offers, or add the edges"
- file: .engineering/planning/story/reach-three-valued.md
  line: 47
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "lands on crates/cli/src/lib.rs and main.rs with story:mcp-server-read-tools and story:reader-function-spans and neither body mentions the other; add an ordering edge naming those files as the reason, or split the subcommand registration out"
- file: .engineering/planning/story/reach-entry-file.md
  line: 35
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "shares crates/effects/src/lib.rs with story:reader-indirect-call-sites, story:reach-at-positions and story:reach-registered-candidates with no depends_on edge between entry-file and indirect-call-sites; the epic's numbered order is prose only; add the edge or split the file"
- file: .engineering/planning/story/reach-at-positions.md
  line: 32
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "shares crates/effects/src/lib.rs with story:reader-indirect-call-sites and story:reach-registered-candidates, crates/cli/src/lib.rs with story:reach-framework-entries, and crates/cli and CHANGELOG.md with story:reader-function-spans, with no edge to any and no body that names the pairs; add edges or split the files"
- file: .engineering/planning/story/reach-registered-candidates.md
  line: 33
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "unordered against story:reach-entry-file and story:reach-at-positions on the reach pass in crates/effects/src/lib.rs and shares crates/effects with story:cir1-raise-out-of-entry, none named in the body; add edges or split the file"
- file: .engineering/planning/story/reach-three-valued.md
  line: 45
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "all six new stories have an empty store scope field, so `aep plan artifact waves` lists each as unassessed and never places it and cannot separate any collision; enter each Own list as scope"
```

**What you read:** 14 artifacts shown in full: epic:sound-reach, the six stories, readers-bound-untrusted-input, reader-function-spans, mcp-server-read-tools, mcp-semantic-tools, cir1-raise-out-of-entry, cir1-fields-of-imported-types and reachability. I also read the Scope sections of 15 other open stories. I ran `aep plan artifact show`, `list`, `waves` and `graph`, plus `git grep` on `crates/cli/src`, `crates/effects` and `crates/model/src/lib.rs`.
Surfaces: 6 cited (every body names its paths, though each labels them "Inferred"), 0 inferred, 0 unplaceable.
Acceptance reads: 9 traced to a producer in the set. 8 have a `depends_on` edge, 1 directly and 1 transitively. The ninth is the at-positions read of story:reader-function-spans, a conditional read ("where that is implemented") with no edge.

**What I could not establish:**
- story:c-interfaces-generics-closures and story:reader-check-externs are "Not scoped", so I cannot say whether they collide with indirect-call-sites (C `expr.lambda` and function pointers).
- story:mcp-semantic-tools shares no file with the set. It rebuilds against `crates/effects` and its `calls` acceptance lists unresolved sites, which `reader-indirect-call-sites` changes in `crates/effects` (matching). I treat that as a library-interface read and not a collision.
- Every story edits `CHANGELOG.md`. The reader stories in the store already treat it as a coordinator-resolved shared file, so I did not file it separately.
- Out of my lane: whether the reach-at-positions acceptance needs function spans to be checkable (acceptance critic), and whether the six-way split is right (design critic).
